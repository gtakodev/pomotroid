use rodio::{Decoder, OutputStream, Sink};
use std::{
    io::Cursor,
    sync::mpsc::{SyncSender, sync_channel},
    thread,
};

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

pub(super) struct Audio(SyncSender<(Sound, u32)>);

impl Audio {
    pub(super) fn new() -> Self {
        let (sender, receiver) = sync_channel::<(Sound, u32)>(8);
        thread::spawn(move || {
            let mut device = None;
            for (sound, volume) in receiver {
                if device.is_none() {
                    device = OutputStream::try_default().ok();
                }
                let Some((_, handle)) = &device else { continue };
                if let (Ok(sink), Ok(source)) = (
                    Sink::try_new(handle),
                    Decoder::new(Cursor::new(sound.bytes())),
                ) {
                    sink.set_volume(volume as f32 / 100.0);
                    sink.append(source);
                    sink.detach();
                }
            }
        });
        Self(sender)
    }

    pub(super) fn play(&self, sound: Sound, volume: u32) {
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
}
