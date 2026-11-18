use rune::{
    Color, Cursor, Cx, El, Icon, LineCap, Phase as DragPhase, ScrollSpec, Shape, TextStyle, Tx,
    canvas, col, el, icon, row, text,
};
use std::f32::consts::TAU;

use crate::preferences::Preferences;
use crate::theme::Theme;
use crate::ui_shortcuts::ShortcutAction;
use crate::{
    Page, PreferenceChange, State, TITLEBAR_HEIGHT, TimerAction,
    timer::{Phase, Status},
};

const BODY: TextStyle = TextStyle::new(14.0, 400.0).tracking(0.35);
const NUMBER: TextStyle = TextStyle::new(13.0, 300.0).mono();
const TITLE: TextStyle = TextStyle::new(16.0, 400.0).tracking(0.7);
const DIAL: TextStyle = TextStyle::new(46.0, 300.0).mono().lh(56.0);

const MENU: Icon = Icon {
    name: "settings-menu",
    view: 24.0,
    body: r#"<path d="M2 8h20M2 16h10" stroke="currentColor" stroke-width="2"/>"#,
};
const BACK: Icon = Icon {
    name: "settings-back",
    view: 24.0,
    body: r#"<path d="m16 4-8 8 8 8" fill="none" stroke="currentColor" stroke-width="2"/>"#,
};
const PLAY: Icon = Icon {
    name: "timer-play",
    view: 24.0,
    body: r#"<path d="M8 4v16l9-8z" fill="currentColor"/>"#,
};
const PAUSE: Icon = Icon {
    name: "timer-pause",
    view: 24.0,
    body: r#"<path d="M8 5v14M16 5v14" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>"#,
};
const SKIP: Icon = Icon {
    name: "timer-skip",
    view: 24.0,
    body: r#"<path d="M6 4v16l10-8zM17 4h2v16h-2z" fill="currentColor"/>"#,
};
const SOUND: Icon = Icon {
    name: "sound",
    view: 24.0,
    body: r#"<path d="M3 9h4l5-5v16l-5-5H3z" fill="currentColor"/><path d="M15 8a5 5 0 0 1 0 8m2-11a8 8 0 0 1 0 14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>"#,
};
const MUTE: Icon = Icon {
    name: "sound-muted",
    view: 24.0,
    body: r#"<path d="M3 9h4l5-5v16l-5-5H3z" fill="currentColor"/><path d="m16 9 6 6m0-6-6 6" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>"#,
};
const CLOCK: Icon = Icon {
    name: "timer-settings",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 6v6l4 2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#,
};
const GEAR: Icon = Icon {
    name: "options",
    view: 20.0,
    body: r#"<path fill="currentColor" fill-rule="evenodd" d="M17.2 11c.1-.7.1-1.3 0-2l2.1-1.6c.2-.1.3-.4.1-.6l-2-3.5c-.1-.2-.4-.3-.6-.2l-2.5 1c-.5-.4-1.1-.7-1.7-1l-.4-2.7a.5.5 0 0 0-.5-.4h-4a.5.5 0 0 0-.5.4l-.3 2.7c-.6.3-1.2.6-1.7 1l-2.5-1a.5.5 0 0 0-.6.2l-2 3.5c-.2.2-.1.5.1.6L2.3 9c-.1.7-.1 1.3 0 2L.2 12.6c-.2.1-.3.4-.1.6l2 3.5c.1.2.4.3.6.2l2.5-1c.5.4 1.1.7 1.7 1l.4 2.7c0 .2.2.4.5.4h4c.3 0 .5-.2.5-.4l.4-2.7c.6-.3 1.2-.6 1.7-1l2.5 1c.2.1.5 0 .6-.2l2-3.5c.2-.2.1-.5-.1-.6z M9.7 13.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7z"/>"#,
};
const PALETTE: Icon = Icon {
    name: "themes",
    view: 24.0,
    body: r#"<path d="M21 11.8c0-5.2-4.3-9.3-9.7-9.3A9.4 9.4 0 0 0 2 12c0 5.3 4.3 9.5 9.2 9.5 2.7 0 3.4-1.8 2.5-3.1-.5-.9.1-2 1.3-2h2c2.6 0 4-1.8 4-4.6ZM6 10a1.3 1.3 0 1 1 2.6 0A1.3 1.3 0 0 1 6 10Zm4-3.5a1.3 1.3 0 1 1 2.6 0 1.3 1.3 0 0 1-2.6 0Zm4.6 1.5a1.3 1.3 0 1 1 2.6 0 1.3 1.3 0 0 1-2.6 0ZM17 12a1.3 1.3 0 1 1 2.6 0 1.3 1.3 0 0 1-2.6 0Z" fill="currentColor" fill-rule="evenodd"/>"#,
};
const INFO: Icon = Icon {
    name: "about",
    view: 24.0,
    body: r#"<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="12" cy="8" r="1.15" fill="currentColor"/><path d="M12 11v6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#,
};
const CHECK: Icon = Icon {
    name: "theme-selected",
    view: 24.0,
    body: r#"<path d="m4 12 5 5L20 6" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

#[derive(Clone, Copy)]
struct Ink {
    bg: Color,
    panel: Color,
    line: Color,
    fg: Color,
    muted: Color,
    accent: Color,
    focus: Color,
    short: Color,
    long: Color,
}

impl From<&Theme> for Ink {
    fn from(t: &Theme) -> Self {
        fn rgba(value: u32) -> Color {
            let [r, g, b, a] = value
                .to_be_bytes()
                .map(|channel| f32::from(channel) / 255.0);
            let linear = rune::color::srgb_decode;
            Color {
                r: linear(r),
                g: linear(g),
                b: linear(b),
                a,
            }
        }
        Self {
            bg: rgba(t.background),
            panel: rgba(t.background_light),
            line: rgba(t.background_lightest),
            fg: rgba(t.foreground),
            muted: rgba(t.foreground_darker),
            accent: rgba(t.accent),
            focus: rgba(t.focus_round),
            short: rgba(t.short_round),
            long: rgba(t.long_round),
        }
    }
}

pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Work => "Focus",
        Phase::ShortBreak => "Short Break",
        Phase::LongBreak => "Long Break",
    }
}

fn label(value: impl Into<String>, color: Color) -> El {
    text(value).font(BODY).color(color)
}

fn button(key: impl Into<String>, name: &str, ink: Ink) -> El {
    el().key(key)
        .role("button")
        .label(name)
        .center()
        .radius(5.0)
        .focusable(true)
        .focus_ring(1.5, 1.0, ink.accent)
        .cursor(Cursor::Pointer)
        .hover(ink.line.alpha(0.12))
        .press(ink.line.alpha(0.20))
}

fn glyph(symbol: &'static Icon, size: f32, color: Color) -> El {
    icon(symbol, size).color(color).a11y_hidden()
}

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let ink = Ink::from(state.theme());
    let inset = cx.chrome_insets();
    cx.caption_color(ink.line);
    let title = el()
        .w_full()
        .h(TITLEBAR_HEIGHT)
        .window_drag()
        .center()
        .child(
            text("Pomotroid")
                .font(TITLE)
                .color(ink.short)
                .role("heading")
                .level(1),
        )
        .child(
            button(
                "settings",
                if state.drawer.is_some() {
                    "Back to timer"
                } else {
                    "Settings"
                },
                ink,
            )
            .size(42.0, 40.0)
            .absolute(inset.leading + 7.0, 7.0)
            .no_window_drag()
            .expanded(state.drawer.is_some())
            .on_press_tx(|state: &mut State, tx| {
                state.select_page(state.drawer.is_none().then_some(state.last_page), tx);
            })
            .child(glyph(
                if state.drawer.is_some() { &BACK } else { &MENU },
                22.0,
                ink.line,
            )),
        );
    let content = state
        .drawer
        .map_or_else(|| timer_view(state, ink), |page| drawer(state, page, ink));
    col()
        .key("pomotroid")
        .w_full()
        .h_full()
        .fill(ink.bg)
        .child(title)
        .child(content)
}

fn dial_shape(fraction: f32, accent: Color, line: Color) -> Shape {
    const CENTER: f32 = 110.0;
    const RADIUS: f32 = 105.0;
    let points = |count: u32, turn: f32| {
        (0..=count)
            .map(|i| {
                let angle = TAU * turn * i as f32 / count as f32;
                let (s, c) = angle.sin_cos();
                [CENTER + RADIUS * s, CENTER - RADIUS * c]
            })
            .collect::<Vec<_>>()
    };
    let mut shape = Shape::new().stroke_with(points(120, 1.0), true, 2.0, LineCap::Butt, line);
    if fraction >= 1.0 {
        return shape.stroke_with(points(120, 1.0), true, 9.5, LineCap::Round, accent);
    }
    if fraction > 0.0 {
        shape = shape.stroke_with(
            points((fraction * 120.0).ceil() as u32, fraction),
            false,
            9.5,
            LineCap::Round,
            accent,
        );
    }
    shape
}

fn timer_view(state: &State, ink: Ink) -> El {
    let phase = state.timer.phase();
    let remaining = state.timer.remaining_seconds();
    let fraction = remaining as f32 / state.timer.total_seconds() as f32;
    let time = format!("{}:{:02}", remaining / 60, remaining % 60);
    let accent = match phase {
        Phase::Work => ink.focus,
        Phase::ShortBreak => ink.short,
        Phase::LongBreak => ink.long,
    };
    let (action, symbol) = match state.timer.status() {
        Status::Running => ("Pause timer", &PAUSE),
        Status::Paused => ("Resume timer", &PLAY),
        _ => ("Start timer", &PLAY),
    };

    let dial = el()
        .key("dial")
        .size(220.0, 220.0)
        .child(
            canvas(dial_shape(fraction, accent, ink.line))
                .size(220.0, 220.0)
                .absolute(0.0, 0.0)
                .a11y_hidden(),
        )
        .child(
            row()
                .size(220.0, 56.0)
                .absolute(0.0, 74.0)
                .center()
                .child(text(time).key("time-left").font(DIAL).color(ink.fg)),
        )
        .child(
            row().size(220.0, 24.0).absolute(0.0, 143.0).center().child(
                text(phase_name(phase).to_uppercase())
                    .key("phase")
                    .font(TITLE.tracking(1.5))
                    .color(ink.fg),
            ),
        );

    let mut rounds = row().gap(4.0).center_y().child(
        label(
            format!("{}/{}", state.timer.round(), state.prefs.work_rounds),
            ink.fg,
        )
        .key("round"),
    );
    if state.timer.completed_work() > 0 {
        rounds = rounds.child(
            text(format!("({})", state.timer.completed_work()))
                .key("completed-work")
                .font(TextStyle::new(11.0, 400.0))
                .color(ink.muted)
                .description_static("Focus rounds completed"),
        );
    }

    let footer = row()
        .key("timer-footer")
        .w_full()
        .h(62.0)
        .px(14.0)
        .justify_between()
        .center_y()
        .child(
            col().items_start().gap(2.0).child(rounds).child(
                button("reset", "Reset timer", ink)
                    .size(54.0, 27.0)
                    .on_press_tx(|state: &mut State, tx| state.play_timer(TimerAction::Reset, tx))
                    .child(label("Reset", ink.muted)),
            ),
        )
        .child(
            row()
                .gap(1.0)
                .child(
                    button("skip", "Skip round", ink)
                        .size(36.0, 36.0)
                        .help("Skip round")
                        .on_press_tx(|state: &mut State, tx| {
                            state.play_timer(TimerAction::Skip, tx)
                        })
                        .child(glyph(&SKIP, 20.0, ink.line)),
                )
                .child(
                    button(
                        "mute",
                        if state.prefs.volume == 0 {
                            "Unmute"
                        } else {
                            "Mute"
                        },
                        ink,
                    )
                    .size(38.0, 36.0)
                    .help("Mute or adjust volume")
                    .on_hover_change(State::hover_volume)
                    .on_press_tx(|state: &mut State, tx| {
                        let volume = if state.prefs.volume == 0 {
                            state.last_volume
                        } else {
                            0
                        };
                        RangeSetting::Volume.set(state, volume, tx);
                    })
                    .child(glyph(
                        if state.prefs.volume == 0 {
                            &MUTE
                        } else {
                            &SOUND
                        },
                        21.0,
                        ink.line,
                    )),
                ),
        );

    let timer = col()
        .key("timer")
        .w_full()
        .grow(1.0)
        .items_center()
        .child(el().h(53.0))
        .child(dial)
        .child(el().h(20.0))
        .child(
            button("start-pause", action, ink)
                .size(54.0, 54.0)
                .radius(27.0)
                .border(2.0, ink.line)
                .help(action)
                .on_press_tx(|state: &mut State, tx| state.play_timer(TimerAction::Toggle, tx))
                .child(glyph(symbol, 22.0, ink.fg)),
        )
        .child(el().h(3.0))
        .child(footer);
    if state.volume_popup.is_some() {
        timer.child(
            col()
                .key("volume-popover")
                .w(116.0)
                .h(62.0)
                .padding(10.0)
                .items_center()
                .pin(None, None, Some(12.0), Some(63.0))
                .radius(6.0)
                .fill(ink.panel)
                .on_hover_change(State::hover_volume)
                .child(label(
                    format!("Volume {}%", RangeSetting::Volume.get(state)),
                    ink.fg,
                ))
                .child(slider(state, RangeSetting::Volume, ink.accent, ink, 96.0)),
        )
    } else {
        timer
    }
}

pub fn tab_key(page: Page) -> &'static str {
    match page {
        Page::Timer => "tab-timer",
        Page::Options => "tab-options",
        Page::Themes => "tab-themes",
        Page::About => "tab-about",
    }
}

fn page_name(page: Page) -> &'static str {
    match page {
        Page::Timer => "Timer",
        Page::Options => "Options",
        Page::Themes => "Themes",
        Page::About => "About",
    }
}

fn drawer(state: &State, page: Page, ink: Ink) -> El {
    let contents = match page {
        Page::Timer => timer_settings(state, ink),
        Page::Options => options(state, ink),
        Page::Themes => themes(state, ink),
        Page::About => about(ink),
    };
    let nav = row()
        .key("settings-tabs")
        .role("group")
        .label("Settings pages")
        .w_full()
        .h(36.0)
        .fill(ink.bg)
        .children(
            [
                (Page::Timer, &CLOCK),
                (Page::Options, &GEAR),
                (Page::Themes, &PALETTE),
                (Page::About, &INFO),
            ]
            .map(|(tab, picture)| {
                let selected = page == tab;
                button(tab_key(tab), page_name(tab), ink)
                    .checked(selected)
                    .grow(1.0)
                    .h_full()
                    .help(page_name(tab))
                    .on_press_tx(move |state: &mut State, tx| state.select_page(Some(tab), tx))
                    .child(glyph(
                        picture,
                        20.0,
                        if selected { ink.fg } else { ink.line },
                    ))
                    .children(selected.then(|| {
                        el().size(30.0, 2.0)
                            .fill(ink.accent)
                            .pin(Some(30.0), None, None, Some(0.0))
                            .a11y_hidden()
                    }))
            }),
        );

    col()
        .key("settings-drawer")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(el().h(18.0))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .fill(ink.panel)
                .child(
                    row().w_full().h(34.0).center().child(
                        label(page_name(page), ink.fg)
                            .key("page-title")
                            .role("heading")
                            .level(2),
                    ),
                )
                .children(state.save_error.as_ref().map(|error| {
                    row()
                        .w_full()
                        .px(18.0)
                        .py(4.0)
                        .child(label(error.clone(), ink.fg).role("alert").wrap())
                }))
                .child(contents)
                .child(nav),
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RangeSetting {
    Focus,
    ShortBreak,
    LongBreak,
    Rounds,
    Volume,
}

impl RangeSetting {
    const TIMER_SETTINGS: [Self; 4] =
        [Self::Focus, Self::ShortBreak, Self::LongBreak, Self::Rounds];

    fn name(self) -> &'static str {
        match self {
            Self::Focus => "Focus",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
            Self::Rounds => "Rounds",
            Self::Volume => "Volume",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Focus => "duration-focus",
            Self::ShortBreak => "duration-short-break",
            Self::LongBreak => "duration-long-break",
            Self::Rounds => "round-count",
            Self::Volume => "volume-level",
        }
    }

    fn limits(self) -> (u32, u32) {
        match self {
            Self::Rounds => (1, 12),
            Self::Volume => (0, 100),
            _ => (1, 90),
        }
    }

    fn display(self, value: u32) -> String {
        match self {
            Self::Rounds => value.to_string(),
            Self::Volume => format!("{value}%"),
            _ => format!("{value}:00"),
        }
    }

    fn change(self, prefs: &mut Preferences) -> &mut u32 {
        match self {
            Self::Focus => &mut prefs.time_work,
            Self::ShortBreak => &mut prefs.time_short_break,
            Self::LongBreak => &mut prefs.time_long_break,
            Self::Rounds => &mut prefs.work_rounds,
            Self::Volume => &mut prefs.volume,
        }
    }

    fn get(self, state: &State) -> u32 {
        if let Some((setting, draft)) = state.range_draft
            && setting == self
        {
            return draft;
        }
        match self {
            Self::Focus => state.prefs.time_work,
            Self::ShortBreak => state.prefs.time_short_break,
            Self::LongBreak => state.prefs.time_long_break,
            Self::Rounds => state.prefs.work_rounds,
            Self::Volume => state.prefs.volume,
        }
    }

    fn set(self, state: &mut State, value: u32, tx: &mut Tx) {
        let slot = self.change(&mut state.prefs);
        if *slot == value {
            return;
        }
        *slot = value;
        if self == Self::Volume && value > 0 {
            state.last_volume = value;
        }
        state.apply_preferences(PreferenceChange::Update, tx);
    }
}

fn slider(state: &State, setting: RangeSetting, accent: Color, ink: Ink, width: f32) -> El {
    const HANDLE: f32 = 16.0;
    let value = setting.get(state);
    let (min, max) = setting.limits();
    let position = ((value - min) as f32 / (max - min) as f32) * (width - HANDLE);
    el().key(setting.key())
        .role("slider")
        .label(setting.name())
        .a11y_value(setting.display(value))
        .range_value(f64::from(value), f64::from(min), f64::from(max))
        .range_step(1.0)
        .focusable(true)
        .focus_ring(1.5, 1.0, ink.accent)
        .size(width, 24.0)
        .cursor(Cursor::Pointer)
        .child(el().size(width, 3.0).absolute(0.0, 10.5).fill(ink.bg))
        .child(
            el().size(position + HANDLE / 2.0, 3.0)
                .absolute(0.0, 10.5)
                .fill(accent),
        )
        .child(
            el().size(HANDLE, HANDLE)
                .absolute(position, 4.0)
                .radius(HANDLE / 2.0)
                .fill(accent),
        )
        .on_drag(move |state: &mut State, drag, tx| {
            let fraction = ((drag.x - HANDLE / 2.0) / (drag.w - HANDLE)).clamp(0.0, 1.0);
            let value = min + (fraction * (max - min) as f32).round() as u32;
            if drag.phase == DragPhase::Up {
                state.range_draft = None;
                if !drag.cancelled() {
                    setting.set(state, value, tx);
                }
                if state.volume_popup.is_some_and(|due| due <= tx.clock()) {
                    state.volume_popup = None;
                }
            } else {
                state.range_draft = Some((setting, value));
            }
        })
        .on_step(move |state: &mut State, steps, tx| {
            let value = (setting.get(state) as i32 + steps).clamp(min as i32, max as i32) as u32;
            setting.set(state, value, tx);
        })
        .on_step_limit(move |state: &mut State, end, tx| {
            setting.set(state, if end { max } else { min }, tx)
        })
}

fn timer_settings(state: &State, ink: Ink) -> El {
    col()
        .key("timer-settings")
        .w_full()
        .grow(1.0)
        .items_center()
        .children(RangeSetting::TIMER_SETTINGS.map(|setting| {
            let accent = match setting {
                RangeSetting::Focus => ink.focus,
                RangeSetting::ShortBreak => ink.short,
                RangeSetting::LongBreak => ink.long,
                _ => ink.line,
            };
            col()
                .h(74.0)
                .items_center()
                .gap(4.0)
                .child(label(setting.name(), ink.muted))
                .child(
                    row()
                        .h(20.0)
                        .px(6.0)
                        .center()
                        .radius(4.0)
                        .fill(ink.bg)
                        .child(
                            text(setting.display(setting.get(state)))
                                .font(NUMBER)
                                .color(ink.fg),
                        ),
                )
                .child(slider(state, setting, accent, ink, 324.0))
        }))
        .child(
            button("reset-defaults", "Reset timer defaults", ink)
                .size(130.0, 30.0)
                .on_press_tx(|state: &mut State, tx| {
                    state.apply_preferences(PreferenceChange::ResetTimerDefaults, tx);
                })
                .child(label("Reset Defaults", ink.muted)),
        )
}

type Flag = fn(&mut Preferences) -> &mut bool;

fn toggle(key: &'static str, name: &'static str, enabled: bool, flag: Flag, ink: Ink) -> El {
    row()
        .w_full()
        .h(34.0)
        .justify_between()
        .center_y()
        .gap(8.0)
        .child(label(name, ink.fg).grow(1.0).wrap())
        .child(
            el().key(key)
                .role("switch")
                .label(name)
                .checked(enabled)
                .focusable(true)
                .focus_ring(1.5, 2.0, ink.accent)
                .cursor(Cursor::Pointer)
                .size(36.0, 20.0)
                .radius(10.0)
                .fill(if enabled { ink.accent } else { ink.bg })
                .on_press_tx(move |state: &mut State, tx| {
                    let flag = flag(&mut state.prefs);
                    *flag = !*flag;
                    state.apply_preferences(PreferenceChange::Update, tx);
                })
                .child(
                    el().size(14.0, 14.0)
                        .radius(7.0)
                        .absolute(if enabled { 19.0 } else { 3.0 }, 3.0)
                        .fill(if enabled { ink.bg } else { ink.line }),
                ),
        )
}

fn options(state: &State, ink: Ink) -> El {
    let p = &state.prefs;
    let entries: [(&str, &str, bool, Flag); 9] = [
        ("always-on-top", "Always On Top", p.always_on_top, |p| {
            &mut p.always_on_top
        }),
        (
            "top-on-breaks",
            "Deactivate Always On Top on Breaks",
            p.break_always_on_top,
            |p| &mut p.break_always_on_top,
        ),
        (
            "auto-work",
            "Auto-start Work Timer",
            p.auto_start_work_timer,
            |p| &mut p.auto_start_work_timer,
        ),
        (
            "auto-break",
            "Auto-start Break Timer",
            p.auto_start_break_timer,
            |p| &mut p.auto_start_break_timer,
        ),
        ("tick-work", "Tick Sounds - Work", p.tick_sounds, |p| {
            &mut p.tick_sounds
        }),
        (
            "tick-break",
            "Tick Sounds - Break",
            p.tick_sounds_during_break,
            |p| &mut p.tick_sounds_during_break,
        ),
        (
            "notifications",
            "Desktop Notifications",
            p.notifications,
            |p| &mut p.notifications,
        ),
        ("minimize-to-tray", "Minimize to Tray", p.min_to_tray, |p| {
            &mut p.min_to_tray
        }),
        (
            "close-to-tray",
            "Minimize to Tray on Close",
            p.min_to_tray_on_close,
            |p| &mut p.min_to_tray_on_close,
        ),
    ];
    col()
        .key("options")
        .w_full()
        .grow(1.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .px(18.0)
                .py(2.0)
                .children(
                    entries
                        .into_iter()
                        .filter(|(key, ..)| *key != "top-on-breaks" || p.always_on_top)
                        .map(|(key, name, enabled, flag)| toggle(key, name, enabled, flag, ink)),
                )
                .child(
                    row()
                        .w_full()
                        .h(38.0)
                        .center_y()
                        .justify_between()
                        .child(label(
                            format!(
                                "Volume {}",
                                RangeSetting::Volume.display(RangeSetting::Volume.get(state))
                            ),
                            ink.fg,
                        ))
                        .child(slider(state, RangeSetting::Volume, ink.accent, ink, 124.0)),
                )
                .child(
                    row()
                        .w_full()
                        .h(36.0)
                        .center_y()
                        .child(label("Global Shortcuts", ink.fg).role("heading").level(3)),
                )
                .children(ShortcutAction::ALL.map(|action| shortcut_row(state, action, ink))),
        )
}

fn shortcut_row(state: &State, action: ShortcutAction, ink: Ink) -> El {
    let capture = state.shortcuts.capture == Some(action);
    let working = state.shortcuts.pending(action);
    let value = if capture {
        "Press shortcut"
    } else if working {
        "Registering…"
    } else if action.accelerator(&state.prefs).is_empty() {
        "Unassigned"
    } else {
        action.accelerator(&state.prefs)
    };
    col()
        .w_full()
        .py(3.0)
        .child(
            row()
                .w_full()
                .h(32.0)
                .justify_between()
                .center_y()
                .child(label(action.name(), ink.fg))
                .child(
                    button(action.key(), action.name(), ink)
                        .help("Press a shortcut. Backspace clears it and Escape cancels.")
                        .w(128.0)
                        .h(28.0)
                        .fill(if capture { ink.bg } else { ink.panel })
                        .a11y_value(value)
                        .border(
                            1.0,
                            if capture {
                                ink.accent
                            } else {
                                ink.line.alpha(0.38)
                            },
                        )
                        .on_press_tx(move |state: &mut State, tx| {
                            if !state.shortcuts.pending(action) {
                                state.shortcuts.capture = Some(action);
                                tx.focus(action.key(), true);
                            }
                        })
                        .on_key(move |state: &mut State, key, tx| {
                            state.capture_shortcut(action, key, tx)
                        })
                        .child(text(value).font(NUMBER).color(ink.accent).truncate()),
                ),
        )
        .children(state.shortcuts.error(action).map(|error| {
            label(error.to_owned(), ink.muted)
                .w_full()
                .wrap()
                .role("status")
        }))
}

fn themes(state: &State, ink: Ink) -> El {
    col()
        .key("theme-picker")
        .w_full()
        .grow(1.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .px(18.0)
                .py(3.0)
                .gap(10.0)
                .children(state.themes.iter().map(|theme| {
                    let preview = Ink::from(theme);
                    let name = theme.name.clone();
                    let selected = state.theme().name == name;
                    let key = format!("theme-{}", name.to_lowercase().replace(' ', "-"));
                    button(key, &name, ink)
                        .checked(selected)
                        .w_full()
                        .h(46.0)
                        .fill(preview.bg)
                        .on_press_tx(move |state: &mut State, tx| {
                            state.prefs.theme = Some(name.clone());
                            state.apply_preferences(PreferenceChange::Update, tx);
                        })
                        .child(el().size(3.0, 46.0).absolute(0.0, 0.0).fill(preview.accent))
                        .child(
                            row()
                                .h_full()
                                .absolute(13.0, 0.0)
                                .center_y()
                                .child(label(theme.name.clone(), preview.fg)),
                        )
                        .children(selected.then(|| {
                            row()
                                .size(32.0, 46.0)
                                .center()
                                .pin(None, Some(0.0), Some(5.0), None)
                                .child(glyph(&CHECK, 20.0, preview.accent))
                        }))
                })),
        )
}

fn about(ink: Ink) -> El {
    let version = env!("CARGO_PKG_VERSION");
    let link = |key, name, url: String| {
        button(key, name, ink)
            .w(220.0)
            .h(32.0)
            .href(url)
            .child(label(name, ink.muted))
    };
    col()
        .key("about")
        .w_full()
        .grow(1.0)
        .items_center()
        .gap(8.0)
        .child(el().h(22.0))
        .child(
            el().size(72.0, 72.0)
                .radius(36.0)
                .fill(ink.bg)
                .center()
                .child(el().size(52.0, 52.0).radius(26.0).border(8.0, ink.focus)),
        )
        .child(
            text("Pomotroid")
                .font(TextStyle::new(24.0, 400.0))
                .color(ink.short),
        )
        .child(label(format!("Version {version}"), ink.muted))
        .child(link(
            "release-notes",
            "Release notes",
            "https://github.com/gtakodev/pomotroid/releases".to_owned(),
        ))
        .child(link(
            "project-docs",
            "License and Documentation",
            "https://github.com/gtakodev/pomotroid".to_owned(),
        ))
        .child(link(
            "original-pomotroid",
            "Original Pomotroid by Splode",
            "https://github.com/Splode/pomotroid".to_owned(),
        ))
}
