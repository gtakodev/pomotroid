use rune::{Key, KeyEvent, Mods, Shortcut, ShortcutError, ShortcutId, Tx};

use crate::{Preferences, State, TimerAction};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutAction {
    Toggle,
    Reset,
    Skip,
}

impl ShortcutAction {
    pub const ALL: [Self; 3] = [Self::Toggle, Self::Reset, Self::Skip];

    pub fn key(self) -> &'static str {
        match self {
            Self::Toggle => "shortcut-toggle",
            Self::Reset => "shortcut-reset",
            Self::Skip => "shortcut-skip",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Toggle => "Toggle Timer",
            Self::Reset => "Reset Timer",
            Self::Skip => "Skip Round",
        }
    }

    pub fn accelerator(self, prefs: &Preferences) -> &str {
        match self {
            Self::Toggle => &prefs.global_shortcuts.toggle,
            Self::Reset => &prefs.global_shortcuts.reset,
            Self::Skip => &prefs.global_shortcuts.skip,
        }
    }

    fn set(self, prefs: &mut Preferences, chord: String) {
        let target = match self {
            Self::Toggle => &mut prefs.global_shortcuts.toggle,
            Self::Reset => &mut prefs.global_shortcuts.reset,
            Self::Skip => &mut prefs.global_shortcuts.skip,
        };
        *target = chord;
    }

    fn command(self) -> TimerAction {
        match self {
            Self::Toggle => TimerAction::Toggle,
            Self::Reset => TimerAction::Reset,
            Self::Skip => TimerAction::Skip,
        }
    }
}

#[derive(Default)]
struct Binding {
    id: Option<ShortcutId>,
    pending: bool,
    error: Option<String>,
}

#[derive(Default)]
pub struct Shortcuts {
    pub capture: Option<ShortcutAction>,
    initialized: bool,
    bindings: [Binding; 3],
}

impl Shortcuts {
    pub fn pending(&self, action: ShortcutAction) -> bool {
        self.bindings[action as usize].pending
    }

    pub fn error(&self, action: ShortcutAction) -> Option<&str> {
        self.bindings[action as usize].error.as_deref()
    }
}

fn logical_shortcut(key: &KeyEvent) -> Result<Shortcut, ShortcutError> {
    let mut chord = String::new();
    for (held, prefix) in [
        (key.mods.ctrl, "Control+"),
        (key.mods.logo, "Super+"),
        (key.mods.shift, "Shift+"),
        (key.mods.alt, "Alt+"),
    ] {
        if held {
            chord.push_str(prefix);
        }
    }
    let last = match &key.key {
        Key::Function(number) => format!("F{number}"),
        Key::Command(' ') => "Space".to_owned(),
        Key::Command('+') => "Plus".to_owned(),
        Key::Command(c) => c.to_string(),
        Key::Text(value) if value == " " => "Space".to_owned(),
        Key::Text(value) if value == "+" => "Plus".to_owned(),
        Key::Text(value) => value.to_owned(),
        Key::Enter => "Enter".to_owned(),
        Key::Delete => "Delete".to_owned(),
        Key::Left => "Left".to_owned(),
        Key::Right => "Right".to_owned(),
        Key::Up => "Up".to_owned(),
        Key::Down => "Down".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Insert => "Insert".to_owned(),
        _ => return Err(ShortcutError::InvalidAccelerator(chord)),
    };
    chord.push_str(&last);
    chord.parse()
}

impl State {
    pub fn initialize_shortcuts(&mut self, tx: &mut Tx) {
        if self.shortcuts.initialized {
            return;
        }
        self.shortcuts.initialized = true;
        for action in ShortcutAction::ALL {
            let accelerator = action.accelerator(&self.prefs);
            if !accelerator.is_empty() {
                self.replace_shortcut(action, accelerator.parse(), false, tx);
            }
        }
    }

    pub fn capture_shortcut(
        &mut self,
        action: ShortcutAction,
        event: &KeyEvent,
        tx: &mut Tx,
    ) -> bool {
        if self.shortcuts.capture != Some(action) {
            return false;
        }
        match event.key {
            _ if tx.key_repeat() => return true,
            Key::Tab
                if event.mods
                    == (Mods {
                        shift: event.mods.shift,
                        ..Mods::default()
                    }) =>
            {
                self.shortcuts.capture = None;
                return false;
            }
            Key::Modifier { .. } => return true,
            Key::Escape if event.mods == Mods::default() => {}
            Key::Backspace if event.mods == Mods::default() => self.clear_shortcut(action, tx),
            _ => {
                let shortcut = tx.physical_key().map_or_else(
                    || logical_shortcut(event),
                    |key| Shortcut::from_physical_key(key, event.mods),
                );
                self.replace_shortcut(action, shortcut, true, tx);
            }
        }
        self.shortcuts.capture = None;
        true
    }

    fn clear_shortcut(&mut self, action: ShortcutAction, tx: &mut Tx) {
        if let Some(id) = self.shortcuts.bindings[action as usize].id
            && let Some(app) = &self.app
            && let Err(error) = app.unregister_shortcut(id)
        {
            self.shortcut_result(action, Err(error), None, tx);
            return;
        }
        self.shortcuts.bindings[action as usize] = Binding::default();
        action.set(&mut self.prefs, String::new());
        self.save_preferences();
    }

    fn replace_shortcut(
        &mut self,
        action: ShortcutAction,
        shortcut: Result<Shortcut, ShortcutError>,
        persist: bool,
        tx: &mut Tx,
    ) {
        let shortcut = match shortcut.and_then(|key| key.require_modifier().map(|()| key)) {
            Ok(key) => key,
            Err(error) => {
                self.shortcut_result(action, Err(error), None, tx);
                return;
            }
        };
        let current = &mut self.shortcuts.bindings[action as usize];
        if current.id.is_some()
            && action
                .accelerator(&self.prefs)
                .parse::<Shortcut>()
                .ok()
                .as_ref()
                == Some(&shortcut)
        {
            current.error = None;
            return;
        }
        let Some(app) = self.app.clone() else {
            return;
        };
        let old = current.id;
        let accelerator = persist.then(|| shortcut.to_string());
        let register = move |app: &rune::AppHandle<State>| {
            let new = app.register_shortcut(shortcut, move |state, tx| {
                if let Some(capture) = state.shortcuts.capture.take() {
                    let existing = action.accelerator(&state.prefs).parse();
                    state.replace_shortcut(capture, existing, true, tx);
                } else {
                    state.play_timer(action.command(), tx);
                }
            })?;
            if let Some(old) = old
                && let Err(error) = app.unregister_shortcut(old)
            {
                let _ = app.unregister_shortcut(new);
                return Err(error);
            }
            Ok(new)
        };
        if cfg!(target_os = "linux") && !rune::is_headless() {
            current.pending = true;
            current.error = None;
            std::thread::spawn(move || {
                let result = register(&app);
                app.update(move |state, tx| state.shortcut_result(action, result, accelerator, tx));
            });
        } else {
            self.shortcut_result(action, register(&app), accelerator, tx);
        }
    }

    fn shortcut_result(
        &mut self,
        action: ShortcutAction,
        outcome: Result<ShortcutId, ShortcutError>,
        accelerator: Option<String>,
        tx: &mut Tx,
    ) {
        let binding = &mut self.shortcuts.bindings[action as usize];
        binding.pending = false;
        match outcome {
            Ok(id) => {
                binding.id = Some(id);
                binding.error = None;
                if let Some(accelerator) = accelerator {
                    action.set(&mut self.prefs, accelerator);
                    self.save_preferences();
                }
            }
            Err(error) => {
                let message = match error {
                    ShortcutError::InvalidAccelerator(_) => {
                        "Use Ctrl, Alt or Super with a key, or use F1 through F24.".to_owned()
                    }
                    _ => format!("{error}"),
                };
                binding.error = Some(message.clone());
                tx.announce(message, rune::LiveRegion::Polite);
            }
        }
    }
}
