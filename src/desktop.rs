use crate::preferences::Preferences;
use std::{net::TcpListener, sync::Arc};

mod audio;
mod socket;
mod tray;

pub enum DesktopAction {
    Show(Option<String>),
    Quit,
}

pub struct Desktop {
    headless: bool,
    handler: Arc<dyn Fn(DesktopAction) + Send + Sync>,
    audio: Option<audio::Audio>,
    socket: Option<socket::StateServer>,
    tray: Option<tray::Tray>,
    durations: [u32; 3],
}

impl Desktop {
    pub fn new(headless: bool, handler: impl Fn(DesktopAction) + Send + Sync + 'static) -> Self {
        let handler = Arc::new(handler);
        let mut desktop = Self {
            headless,
            handler,
            audio: None,
            socket: None,
            tray: None,
            durations: [25, 5, 15],
        };
        if !headless {
            tray::install_handlers(desktop.handler.clone());
            desktop.audio = Some(audio::Audio::new());
            match TcpListener::bind(("127.0.0.1", 1314)).and_then(socket::StateServer::start) {
                Ok(server) => desktop.socket = Some(server),
                Err(error) => eprintln!("Local timer events unavailable on port 1314: {error}"),
            }
        }
        desktop
    }

    pub fn configure(&mut self, prefs: &Preferences) {
        self.durations = [
            prefs.time_work,
            prefs.time_short_break,
            prefs.time_long_break,
        ];
        if self.headless {
            return;
        }
        if prefs.min_to_tray || prefs.min_to_tray_on_close {
            if self.tray.is_none() {
                match tray::Tray::new(self.handler.clone()) {
                    Ok(tray) => self.tray = Some(tray),
                    Err(error) => eprintln!("Tray unavailable: {error}"),
                }
            }
        } else {
            self.tray = None;
        }
    }

    pub fn tray_available(&self) -> bool {
        self.tray.is_some()
    }

    pub fn progress(&mut self, phase: &str, running: bool, remaining: u32, total: u32) {
        if let Some(tray) = &mut self.tray {
            tray.update(phase, running, remaining, total);
        }
    }

    pub fn publish(&self, state: &'static str) {
        if let Some(socket) = &self.socket {
            socket.publish(state);
        }
    }

    pub fn tick(&self, volume: u32) {
        if let Some(audio) = &self.audio {
            audio.play(audio::Sound::Tick, volume);
        }
    }

    pub fn round_changed(&self, phase: &str, volume: u32, notify: bool) {
        if self.headless {
            return;
        }
        let (sound, title, body) = match phase {
            "short-break" => (
                audio::Sound::ShortBreak,
                "Focus round complete",
                format!("Take a {} minute short break.", self.durations[1]),
            ),
            "long-break" => (
                audio::Sound::LongBreak,
                "Focus round complete",
                format!("Take a {} minute long break.", self.durations[2]),
            ),
            _ => (
                audio::Sound::Work,
                "Break finished",
                format!("Focus for {} minutes.", self.durations[0]),
            ),
        };
        if let Some(audio) = &self.audio {
            audio.play(sound, volume);
        }
        if notify {
            notify_round(title, body);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn notify_round(title: &str, body: String) {
    static REGISTERED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    if let Err(error) = REGISTERED.get_or_init(|| {
        rune_notify::register(&rune_notify::Identity::new(
            "com.splode.pomotroid",
            "Pomotroid",
        ))
    }) {
        eprintln!("Desktop notifications unavailable: {error}");
        return;
    }
    rune_notify::post(&rune_notify::Notification::new("pomotroid-round", title).body(body));
}

#[cfg(target_os = "linux")]
fn notify_round(title: &'static str, body: String) {
    std::thread::spawn(move || {
        if let Err(error) = std::process::Command::new("notify-send")
            .args(["--app-name=Pomotroid", title, &body])
            .status()
        {
            eprintln!("Desktop notification failed: {error}");
        }
    });
}
