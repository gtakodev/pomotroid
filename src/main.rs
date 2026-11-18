#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod desktop;
mod preferences;
mod theme;
mod timer;
mod ui;
mod ui_shortcuts;

use std::time::Duration;

use desktop::{Desktop, DesktopAction};
use preferences::{ActivityLog, Preferences, PreferencesStore};
use rune::{
    App, AppHandle, AppSpec, Key, KeyEvent, LiveRegion, Mods, Tx, WindowHandle, WindowOptions,
};
use theme::{Theme, bundled_themes, load_themes, selected_theme};
use timer::{Phase, Status, Timer, TimerConfig, TimerEvent};
use ui_shortcuts::Shortcuts;

const SIZE: (f32, f32) = (360.0, 478.0);
const TITLEBAR_HEIGHT: f32 = 50.0;
const FONTS: &[&[u8]] = &[
    include_bytes!("../static/fonts/Lato-Regular.ttf"),
    include_bytes!("../static/fonts/RobotoMono-Light.ttf"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Timer,
    Options,
    Themes,
    About,
}

#[derive(Clone, Copy)]
enum TimerAction {
    Toggle,
    Reset,
    Skip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreferenceChange {
    Update,
    ResetTimerDefaults,
}

struct State {
    prefs: Preferences,
    store: Option<PreferencesStore>,
    activity: Option<ActivityLog>,
    themes: Vec<Theme>,
    timer: Timer,
    desktop: Desktop,
    window: Option<WindowHandle>,
    always_on_top: Option<bool>,
    app: Option<AppHandle<Self>>,
    shortcuts: Shortcuts,
    drawer: Option<Page>,
    last_page: Page,
    range_draft: Option<(ui::RangeSetting, u32)>,
    volume_popup: Option<f64>,
    last_volume: u32,
    save_error: Option<String>,
    last_clock: f64,
}

fn timer_config(prefs: &Preferences) -> TimerConfig {
    TimerConfig::from_minutes(
        prefs.time_work,
        prefs.time_short_break,
        prefs.time_long_break,
        prefs.work_rounds,
        prefs.auto_start_work_timer,
        prefs.auto_start_break_timer,
    )
}

impl State {
    fn new(headless: bool, app: Option<AppHandle<Self>>) -> Self {
        let mut save_error = None;
        let (store, prefs) = if headless {
            (None, Preferences::default())
        } else {
            match PreferencesStore::discover() {
                Ok(store) => {
                    let prefs = store.load().unwrap_or_else(|error| {
                        save_error = Some(format!("Could not load preferences. {error}"));
                        Preferences::default()
                    });
                    (Some(store), prefs)
                }
                Err(error) => {
                    save_error = Some(format!("Could not locate preferences. {error}"));
                    (None, Preferences::default())
                }
            }
        };
        let activity = store
            .as_ref()
            .map(|store| ActivityLog::new(store.directory()));
        let themes = store
            .as_ref()
            .map_or_else(bundled_themes, |store| load_themes(store.directory()));
        let timer = Timer::new(timer_config(&prefs));
        let callback = app.clone();
        let mut desktop = Desktop::new(headless, move |action| {
            let Some(app) = &callback else { return };
            match action {
                DesktopAction::Show(token) => {
                    let controls = app.clone();
                    app.update_if(move |state, _| {
                        if controls.is_visible() && !state.window_minimized() {
                            controls.hide();
                        } else if let Some(token) = token {
                            controls.activate(token);
                        } else {
                            controls.open();
                        }
                        false
                    });
                }
                DesktopAction::Quit => app.quit(),
            }
        });
        desktop.configure(&prefs);
        desktop.progress(
            timer.phase().as_str(),
            false,
            timer.remaining_seconds(),
            timer.total_seconds(),
        );
        let last_volume = if prefs.volume > 0 { prefs.volume } else { 100 };
        Self {
            prefs,
            store,
            activity,
            themes,
            timer,
            desktop,
            window: None,
            always_on_top: None,
            app,
            shortcuts: Shortcuts::default(),
            drawer: None,
            last_page: Page::Timer,
            range_draft: None,
            volume_popup: None,
            last_volume,
            save_error,
            last_clock: 0.0,
        }
    }

    fn theme(&self) -> &Theme {
        selected_theme(&self.themes, self.prefs.theme.as_deref())
    }

    fn select_page(&mut self, page: Option<Page>, tx: &mut Tx) {
        if self.drawer == page {
            return;
        }
        self.shortcuts.capture = None;
        self.volume_popup = None;
        self.range_draft = None;
        self.drawer = page;
        if let Some(page) = page {
            self.last_page = page;
            tx.focus_when_shown(ui::tab_key(page), tx.focus_visible());
        } else {
            tx.focus_when_shown("settings", tx.focus_visible());
        }
    }

    fn play_timer(&mut self, action: TimerAction, tx: &mut Tx) {
        self.advance_to(tx.clock(), tx);
        let event = match action {
            TimerAction::Toggle => self.timer.toggle(),
            TimerAction::Reset => self.timer.reset(),
            TimerAction::Skip => self.timer.skip(),
        };
        if let Some(event) = event {
            self.handle_timer_event(event, tx);
        }
        self.schedule(tx);
    }

    fn advance_to(&mut self, clock: f64, tx: &mut Tx) -> bool {
        if clock <= self.last_clock {
            return false;
        }
        let elapsed = clock - self.last_clock;
        self.last_clock = clock;
        let event = self.timer.advance(Duration::from_secs_f64(elapsed));
        if let Some(event) = event {
            self.handle_timer_event(event, tx);
        }
        event.is_some()
    }

    fn handle_timer_event(&mut self, event: TimerEvent, tx: &mut Tx) {
        let log = match event {
            TimerEvent::Started(phase) => Some((phase, "started")),
            TimerEvent::Resumed(phase) => Some((phase, "resumed")),
            TimerEvent::Paused(phase) => Some((phase, "paused")),
            TimerEvent::Reset(phase) => Some((phase, "reset")),
            TimerEvent::Completed { phase, .. } => Some((phase, "completed")),
            TimerEvent::Tick(_) => None,
        };
        if let (Some(activity), Some((phase, action))) = (&self.activity, log) {
            let phase = match phase {
                Phase::Work => "focus round",
                Phase::ShortBreak => "short break",
                Phase::LongBreak => "long break",
            };
            if let Err(error) = activity.record(&format!("{phase} {action}")) {
                eprintln!("Could not write Pomotroid's activity log: {error}");
            }
        }
        match event {
            TimerEvent::Started(phase) | TimerEvent::Resumed(phase) => {
                self.desktop.publish(phase.as_str())
            }
            TimerEvent::Paused(_) => self.desktop.publish("paused"),
            TimerEvent::Reset(_) => self.desktop.publish("idle"),
            TimerEvent::Tick(phase) => {
                if (phase == Phase::Work && self.prefs.tick_sounds)
                    || (phase != Phase::Work && self.prefs.tick_sounds_during_break)
                {
                    self.desktop.tick(self.prefs.volume);
                }
            }
            TimerEvent::Completed { next, .. } => {
                self.desktop.publish("idle");
                self.desktop.round_changed(
                    next.as_str(),
                    self.prefs.volume,
                    self.prefs.notifications,
                );
                self.configure_desktop();
                tx.announce(
                    format!("{} ready", ui::phase_name(next)),
                    LiveRegion::Polite,
                );
            }
        }
        self.update_tray();
    }

    fn schedule(&self, tx: &mut Tx) {
        if let Some(wake) = self.timer.next_wake() {
            tx.wake_after(wake);
        }
        if self.prefs.min_to_tray
            && self.tray_available()
            && self.app.as_ref().is_some_and(AppHandle::is_visible)
        {
            tx.wake_after(Duration::from_millis(250));
        }
    }

    fn hover_volume(&mut self, over: bool, tx: &mut Tx) {
        if over {
            self.volume_popup = Some(f64::INFINITY);
        } else if self.volume_popup.is_some() {
            let delay = Duration::from_millis(450);
            self.volume_popup = Some(tx.clock() + delay.as_secs_f64());
            tx.wake_after(delay);
        }
    }

    fn apply_preferences(&mut self, change: PreferenceChange, tx: &mut Tx) {
        self.advance_to(tx.clock(), tx);
        let reset_timer = change == PreferenceChange::ResetTimerDefaults;
        if reset_timer {
            self.prefs.reset_timer_defaults();
        }
        let event = self.timer.configure(timer_config(&self.prefs));
        let event = event.or_else(|| {
            if reset_timer {
                self.timer.reset()
            } else {
                None
            }
        });
        if let Some(event) = event {
            self.handle_timer_event(event, tx);
        }
        self.configure_desktop();
        self.save_preferences();
        self.schedule(tx);
    }

    fn save_preferences(&mut self) {
        if let Some(store) = &self.store {
            self.save_error = store
                .save(&self.prefs)
                .err()
                .map(|error| format!("Could not save preferences. {error}"));
        }
    }

    fn configure_desktop(&mut self) {
        let on_break = self.timer.phase() != Phase::Work;
        self.desktop.configure(&self.prefs);
        self.update_tray();
        let top = self.prefs.always_on_top && !(on_break && self.prefs.break_always_on_top);
        if self.always_on_top != Some(top)
            && let Some(window) = &self.window
        {
            window.set_always_on_top(top);
            self.always_on_top = Some(top);
        }
    }

    fn tray_available(&self) -> bool {
        self.desktop.tray_available()
    }

    fn update_tray(&mut self) {
        self.desktop.progress(
            self.timer.phase().as_str(),
            self.timer.status() == Status::Running,
            self.timer.remaining_seconds(),
            self.timer.total_seconds(),
        );
    }

    fn hide_minimized_window(&self) {
        if self.prefs.min_to_tray
            && self.tray_available()
            && self.window_minimized()
            && let Some(app) = &self.app
            && app.is_visible()
        {
            app.hide();
        }
    }

    fn window_minimized(&self) -> bool {
        self.window
            .as_ref()
            .and_then(WindowHandle::placement)
            .is_some_and(|placement| placement.minimized)
    }
}

fn poll(state: &mut State, clock: f64, tx: &mut Tx) -> bool {
    state.hide_minimized_window();
    let mut changed = state.advance_to(clock, tx);
    if state.volume_popup.is_some_and(|due| due <= clock) && state.range_draft.is_none() {
        state.volume_popup = None;
        changed = true;
    }
    if state
        .shortcuts
        .capture
        .is_some_and(|action| tx.focused() != Some(action.key()))
    {
        state.shortcuts.capture = None;
        changed = true;
    }
    state.schedule(tx);
    changed
}

fn on_close(state: &mut State, tx: &mut Tx) {
    if !(state.prefs.min_to_tray_on_close && state.tray_available()) {
        tx.quit();
    }
}

fn on_key(state: &mut State, key: &KeyEvent, tx: &mut Tx) -> bool {
    if key.key == Key::Escape && key.mods == Mods::default() {
        if state.shortcuts.capture.take().is_some() {
            return true;
        }
        if state.volume_popup.take().is_some() {
            return true;
        }
        if state.drawer.is_some() {
            state.select_page(None, tx);
            return true;
        }
    }
    if state.drawer.is_none()
        && key.mods == Mods::default()
        && matches!(&key.key, Key::Text(value) if value == " ")
    {
        if !tx.key_repeat() {
            state.play_timer(TimerAction::Toggle, tx);
        }
        return true;
    }
    if key.is_command('q') && !key.mods.shift && !key.mods.alt {
        tx.quit();
        return true;
    }
    false
}

fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some(SIZE),
        fonts: FONTS,
        families: ("Lato", "Roboto Mono"),
        poll: Some(poll),
        on_key: Some(on_key),
        scenes: &[
            ("timer", |_| {}),
            ("timer-settings", |state| state.drawer = Some(Page::Timer)),
            ("options", |state| state.drawer = Some(Page::Options)),
            ("themes", |state| state.drawer = Some(Page::Themes)),
            ("about", |state| state.drawer = Some(Page::About)),
            ("short-break", |state| {
                state.timer.skip();
            }),
            ("long-break", |state| {
                while state.timer.phase() != Phase::LongBreak {
                    state.timer.skip();
                }
            }),
        ],
        ..AppSpec::new("Pomotroid", SIZE, || State::new(true, None), ui::view)
    }
}

fn main() {
    let headless = rune::is_headless();
    rune::set_app_id("com.splode.pomotroid");
    if !headless {
        #[cfg(target_os = "windows")]
        rune::set_app_icon_resource(1);
        #[cfg(not(target_os = "windows"))]
        if let Err(error) = rune::set_app_icon(include_bytes!("../static/icon.png").to_vec()) {
            eprintln!("Could not set the Pomotroid app icon: {error}");
        }
    }
    let app = App::new(spec())
        .window_options(WindowOptions {
            resizable: false,
            ..WindowOptions::default()
        })
        .titlebar(rune::Titlebar::new(TITLEBAR_HEIGHT))
        .keep_running()
        .on_focus(|state, focused, _| {
            if !focused {
                state.shortcuts.capture = None;
            }
        })
        .on_close(on_close);
    let handle = app.handle();
    let opened = handle.clone();
    handle.update(|state, tx| state.initialize_shortcuts(tx));
    app.init(move || State::new(headless, Some(handle)))
        .on_window(move |window| {
            let window = window.clone();
            opened.update(move |state, tx| {
                state.window = Some(window);
                state.always_on_top = None;
                state.configure_desktop();
                state.schedule(tx);
            });
        })
        .run();
}
