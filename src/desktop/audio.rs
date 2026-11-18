use rodio::{Decoder, OutputStream, Sink, buffer::SamplesBuffer};
use std::{
    f32::consts::TAU,
    io::Cursor,
    sync::mpsc::{SyncSender, sync_channel},
    thread,
};

const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy)]
pub(super) enum Sound {
    Work,
    ShortBreak,
    LongBreak,
    Tick,
}

impl Sound {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Work => include_bytes!("../../static/audio/alert-work.mp3"),
            Self::ShortBreak => include_bytes!("../../static/audio/alert-short-break.mp3"),
            Self::LongBreak => include_bytes!("../../static/audio/alert-long-break.mp3"),
            Self::Tick => include_bytes!("../../static/audio/tick.mp3"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Cue {
    Start,
    Pause,
    Reset,
    Click,
    Boop,
}

impl Cue {
    fn samples(self) -> Vec<f32> {
        use Voice::{Mechanism, Motor, Spring};

        let notes: &[Note] = match self {
            Self::Start => &[
                Note::new(0, 48, (280.0, 122.0), 0.09, Mechanism),
                Note::new(19, 185, (225.0, 603.0), 0.16, Spring),
                Note::new(150, 124, (680.0, 856.0), 0.08, Spring),
            ],
            Self::Pause => &[
                Note::new(0, 195, (488.0, 106.0), 0.14, Motor),
                Note::new(129, 63, (221.0, 102.0), 0.075, Mechanism),
            ],
            Self::Reset => &[
                Note::new(0, 85, (315.0, 105.0), 0.15, Mechanism),
                Note::new(66, 76, (452.0, 252.0), 0.085, Mechanism),
            ],
            Self::Click => &[Note::new(0, 53, (695.0, 270.0), 0.09, Mechanism)],
            Self::Boop => &[
                Note::new(0, 152, (690.0, 335.0), 0.155, Spring),
                Note::new(93, 112, (396.0, 511.0), 0.08, Spring),
            ],
        };
        let length = notes
            .iter()
            .map(|note| note.offset_ms + note.duration_ms)
            .max()
            .unwrap_or(0);
        let mut samples = vec![0.0; frames(length)];
        for note in notes {
            note.mix_into(&mut samples);
        }
        samples
    }
}

#[derive(Clone, Copy)]
enum Voice {
    Spring,
    Motor,
    Mechanism,
}

struct Note {
    offset_ms: u32,
    duration_ms: u32,
    pitch_hz: (f32, f32),
    gain: f32,
    voice: Voice,
}

impl Note {
    const fn new(
        offset_ms: u32,
        duration_ms: u32,
        pitch_hz: (f32, f32),
        gain: f32,
        voice: Voice,
    ) -> Self {
        Self {
            offset_ms,
            duration_ms,
            pitch_hz,
            gain,
            voice,
        }
    }

    fn mix_into(&self, samples: &mut [f32]) {
        let offset = frames(self.offset_ms);
        let length = frames(self.duration_ms);
        let (start_hz, end_hz) = self.pitch_hz;
        let mut noise_state = 1_u32;
        let mut noise = 0.0;
        for (index, sample) in samples[offset..offset + length].iter_mut().enumerate() {
            let position = index as f32 / (length - 1) as f32;
            let time = index as f32 / SAMPLE_RATE as f32;
            let angle = TAU * time * (start_hz + (end_hz - start_hz) * position * 0.5);
            let attack = (position / 0.075).min(1.0);
            let decay = (1.0 - position).powi(2);
            noise_state = noise_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let white_noise = (noise_state >> 16) as f32 / 32_768.0 - 1.0;
            noise += 0.24 * (white_noise - noise);

            let signal = match self.voice {
                Voice::Spring => {
                    let wobble = 0.58 * (TAU * 28.0 * time).sin() * (1.0 - position);
                    0.76 * (angle + wobble).sin() + 0.24 * (2.0 * angle).sin()
                }
                Voice::Motor => 0.72 * angle.sin() + 0.18 * (3.0 * angle).sin() + 0.10 * noise,
                Voice::Mechanism => 0.57 * angle.sin() + 0.17 * (2.7 * angle).sin() + 0.26 * noise,
            };
            *sample += signal * attack * decay * self.gain;
        }
    }
}

fn frames(milliseconds: u32) -> usize {
    (SAMPLE_RATE * milliseconds / 1_000) as usize
}

enum Playback {
    Alert(Sound),
    Interface(Cue),
}

pub(super) struct Audio(SyncSender<(Playback, u32)>);

impl Audio {
    pub(super) fn new() -> Self {
        let (sender, receiver) = sync_channel::<(Playback, u32)>(8);
        thread::spawn(move || {
            let mut device = None;
            let mut interface: Option<Sink> = None;
            for (sound, volume) in receiver {
                if device.is_none() {
                    device = OutputStream::try_default().ok();
                }
                let Some((_, handle)) = &device else { continue };
                let volume = volume as f32 / 100.0;
                match sound {
                    Playback::Alert(alert) => {
                        if let (Ok(sink), Ok(source)) = (
                            Sink::try_new(handle),
                            Decoder::new(Cursor::new(alert.bytes())),
                        ) {
                            sink.set_volume(volume);
                            sink.append(source);
                            sink.detach();
                        }
                    }
                    Playback::Interface(cue) => {
                        if matches!(cue, Cue::Click)
                            && interface.as_ref().is_some_and(|sink| !sink.empty())
                        {
                            continue;
                        }
                        drop(interface.take());
                        if let Ok(sink) = Sink::try_new(handle) {
                            sink.set_volume(volume);
                            sink.append(SamplesBuffer::new(1, SAMPLE_RATE, cue.samples()));
                            interface = Some(sink);
                        }
                    }
                }
            }
        });
        Self(sender)
    }

    pub(super) fn play(&self, sound: Sound, volume: u32) {
        self.send(Playback::Alert(sound), volume);
    }

    pub(super) fn cue(&self, cue: Cue, volume: u32) {
        self.send(Playback::Interface(cue), volume);
    }

    fn send(&self, sound: Playback, volume: u32) {
        if volume > 0 {
            let _ = self.0.try_send((sound, volume.min(100)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_original_alert_decodes_into_audible_samples() {
        for sound in [
            Sound::Work,
            Sound::ShortBreak,
            Sound::LongBreak,
            Sound::Tick,
        ] {
            assert!(
                Decoder::new(Cursor::new(sound.bytes()))
                    .unwrap()
                    .any(|s| s != 0)
            );
        }
    }

    #[test]
    fn reactor_cues_are_short_audible_deterministic_and_fade_to_silence() {
        for (cue, duration) in [
            (Cue::Start, 274),
            (Cue::Pause, 195),
            (Cue::Reset, 142),
            (Cue::Click, 53),
            (Cue::Boop, 205),
        ] {
            let samples = cue.samples();
            assert_eq!(samples, cue.samples());
            assert_eq!(samples.len(), frames(duration));
            assert_eq!(samples.first(), Some(&0.0));
            assert_eq!(samples.last(), Some(&0.0));
            assert!(samples.iter().all(|sample| sample.abs() < 0.25));
            assert!(samples.iter().any(|sample| sample.abs() > 0.03));
            assert!(
                samples
                    .windows(2)
                    .all(|pair| (pair[1] - pair[0]).abs() < 0.04)
            );
        }
    }

    #[test]
    fn reactor_cues_respect_mute_and_clamp_to_master_volume() {
        let (sender, receiver) = sync_channel(1);
        let audio = Audio(sender);
        audio.cue(Cue::Start, 0);
        assert!(receiver.try_recv().is_err());
        audio.cue(Cue::Start, 140);
        assert!(matches!(
            receiver.try_recv(),
            Ok((Playback::Interface(Cue::Start), 100))
        ));
    }
}
