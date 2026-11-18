use rune::{
    Color, Cursor, Cx, El, Icon, Phase as DragPhase, ScrollSpec, TextStyle, Tx, col, el, icon, row,
    text,
};

use crate::avatar::Avatar;
use crate::preferences::Preferences;
use crate::theme::Theme;
use crate::ui_shortcuts::ShortcutAction;
use crate::{
    Page, PreferenceChange, State, TITLEBAR_HEIGHT, TimerAction,
    timer::{Phase, Status},
};

const BODY: TextStyle = TextStyle::new(14.0, 480.0);
const NUMBER: TextStyle = TextStyle::new(13.0, 560.0).mono();
const BRAND: TextStyle = TextStyle::new(16.0, 800.0).tracking(2.6);
const LEGEND: TextStyle = TextStyle::new(11.0, 680.0).tracking(0.8);
const DIAL: TextStyle = TextStyle::new(88.0, 630.0).mono().tracking(-3.8).lh(96.0);
const FACE_WIDTH: f32 = 384.0;
pub const CHOOSE_AVATAR_KEY: &str = "choose-avatar";

const TOMATO: Icon = Icon {
    name: "pomotroid-logo",
    view: 24.0,
    body: r#"<path d="M12 8C8.4 5.5 3 8.2 3 13.5 3 18.1 6.8 21 12 21s9-2.9 9-7.5C21 8.2 15.6 5.5 12 8Z" fill="currentColor"/><path d="M12 3v5m-5-4 5 4 5-4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M7 13.5c0-1.1.7-2 1.7-2.5" fill="none" stroke="white" stroke-opacity=".65" stroke-linecap="round" stroke-width="1.15"/>"#,
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
const RESET: Icon = Icon {
    name: "timer-reset",
    view: 24.0,
    body: r#"<path d="M4 11a8 8 0 1 1 2.2 6.6M4 6v5h5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
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
const AVATAR: Icon = Icon {
    name: "timer-avatar",
    view: 24.0,
    body: r#"<circle cx="12" cy="8" r="3.5" fill="none" stroke="currentColor" stroke-width="2"/><path d="M5 19v-1a7 7 0 0 1 14 0v1z" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>"#,
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
        if t.name == "Pomotroid" {
            return Self {
                bg: rgba(0x14101bff),
                panel: rgba(0x282134ff),
                line: rgba(0x827290ff),
                fg: rgba(0xfff4e5ff),
                muted: rgba(0xbaaecaff),
                accent: rgba(0xd6fb65ff),
                focus: rgba(0xff5f69ff),
                short: rgba(0x78f4c7ff),
                long: rgba(0xe4a6ffff),
            };
        }
        let bg = rgba(t.background);
        let panel = rgba(t.background_light);
        let line = rgba(t.background_lightest);
        let fg = rgba(t.foreground);
        let muted = rgba(t.foreground_darker);
        let muted_is_readable = [bg, panel.mix(line, 0.13)].into_iter().all(|surface| {
            let text = muted.luminance() + 0.05;
            let fill = surface.luminance() + 0.05;
            text.max(fill) / text.min(fill) >= 4.5
        });
        Self {
            bg,
            panel,
            line,
            fg,
            muted: if muted_is_readable { muted } else { fg },
            accent: rgba(t.accent),
            focus: rgba(t.focus_round),
            short: rgba(t.short_round),
            long: rgba(t.long_round),
        }
    }
}

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

impl Ink {
    fn metal(self) -> Color {
        self.panel.mix(self.line, 0.13)
    }

    fn well(self) -> Color {
        self.bg.mix(self.fg, 0.005)
    }

    fn on_accent(self) -> Color {
        if self.accent.luminance() > 0.18 {
            Color::BLACK
        } else {
            Color::WHITE
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
        .radius(7.0)
        .focusable(true)
        .focus_ring(2.0, 2.0, ink.accent)
        .cursor(Cursor::Pointer)
        .hover(ink.line.alpha(0.22))
        .press(Color::BLACK.alpha(0.24))
}

fn glyph(symbol: &'static Icon, size: f32, color: Color) -> El {
    icon(symbol, size).color(color).a11y_hidden()
}

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let ink = Ink::from(state.theme());
    let inset = cx.chrome_insets();
    cx.caption_color(ink.muted);
    let title = el()
        .w_full()
        .h(TITLEBAR_HEIGHT)
        .window_drag()
        .gradient(ink.metal(), ink.panel)
        .border_sides([1.0, 0.0, 1.0, 0.0], ink.line.alpha(0.34))
        .child(
            row()
                .h(TITLEBAR_HEIGHT)
                .absolute(inset.leading + 23.0, 0.0)
                .gap(9.0)
                .center_y()
                .child(glyph(&TOMATO, 23.0, ink.focus))
                .child(
                    text("POMOTROID")
                        .font(BRAND)
                        .color(ink.fg)
                        .role("heading")
                        .level(1),
                ),
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
            .size(36.0, 32.0)
            .pin(
                None,
                Some((TITLEBAR_HEIGHT - 32.0) / 2.0),
                Some(inset.trailing + 16.0),
                None,
            )
            .fill(ink.bg)
            .border_gradient(1.0, ink.line.alpha(0.64), ink.line.alpha(0.22))
            .no_window_drag()
            .expanded(state.drawer.is_some())
            .on_press_tx(|state: &mut State, tx| {
                state.interface_click();
                state.select_page(state.drawer.is_none().then_some(state.last_page), tx);
            })
            .child(glyph(
                if state.drawer.is_some() { &BACK } else { &GEAR },
                16.0,
                ink.accent,
            )),
        );
    let content = state
        .drawer
        .map_or_else(|| timer_view(state, ink), |page| drawer(state, page, ink));
    col()
        .key("pomotroid")
        .w_full()
        .h_full()
        .gradient(ink.panel, ink.bg)
        .border_sides([0.0, 1.0, 1.0, 1.0], ink.line.alpha(0.46))
        .child(title)
        .child(content)
}

fn timer_view(state: &State, ink: Ink) -> El {
    let phase = state.timer.phase();
    let status = state.timer.status();
    let avatar = state.prefs.avatar;
    let boop = if avatar == Avatar::Tomato {
        "Boop the tomato".to_owned()
    } else {
        format!("Boop {}", avatar.name())
    };
    let celebration = state.celebration_progress();
    let accent = match phase {
        Phase::Work => ink.focus,
        Phase::ShortBreak => ink.short,
        Phase::LongBreak => ink.long,
    };

    let timer = col()
        .key("timer")
        .w_full()
        .grow(1.0)
        .items_center()
        .child(el().grow(1.0).min_h(14.0))
        .child(
            el().size(FACE_WIDTH, 228.0)
                .child(crate::reactor::view(
                    avatar,
                    phase,
                    status,
                    state.visual_time,
                    state.pulse(),
                    celebration.unwrap_or(-1.0),
                    accent,
                ))
                .child(
                    button("boop", &boop, ink)
                        .size(164.0, 160.0)
                        .absolute(110.0, 22.0)
                        .radius(66.0)
                        .hover(Color::TRANSPARENT)
                        .press(Color::TRANSPARENT)
                        .help(boop)
                        .on_press_tx(|state: &mut State, tx| state.boop(tx)),
                )
                .children(celebration.map(|_| {
                    row()
                        .size(130.0, 32.0)
                        .pin(Some(12.0), None, None, Some(12.0))
                        .radius(7.0)
                        .center()
                        .gap(6.0)
                        .fill(ink.accent)
                        .child(glyph(&CHECK, 15.0, ink.on_accent()))
                        .child(
                            text("Focus complete!")
                                .key("focus-complete")
                                .font(TextStyle::new(13.0, 750.0))
                                .color(ink.on_accent()),
                        )
                }))
                .child(
                    button(CHOOSE_AVATAR_KEY, "Change avatar", ink)
                        .size(124.0, 32.0)
                        .pin(None, None, Some(12.0), Some(12.0))
                        .fill(ink.well())
                        .border_gradient(1.0, ink.line.alpha(0.80), ink.line.alpha(0.40))
                        .on_press_tx(|state: &mut State, tx| {
                            state.interface_click();
                            state.open_avatar_gallery(tx);
                        })
                        .child(
                            row()
                                .center()
                                .gap(6.0)
                                .a11y_hidden()
                                .child(glyph(&AVATAR, 15.0, ink.accent))
                                .child(
                                    text("Change avatar")
                                        .font(LEGEND.tracking(0.1))
                                        .color(ink.fg),
                                ),
                        ),
                ),
        )
        .child(el().h(13.0))
        .child(timer_display(state, phase, status, accent, ink))
        .child(el().h(16.0))
        .child(timer_controls(state, status, ink))
        .child(
            row()
                .h(35.0)
                .gap(5.0)
                .center()
                .a11y_hidden()
                .children((0..11).map(|_| {
                    el().size(8.0, 2.0)
                        .radius(1.0)
                        .gradient(ink.well(), ink.line.alpha(0.48))
                })),
        )
        .child(el().grow(1.0).min_h(10.0));
    timer.children(state.volume_popup.map(|_| {
        col()
            .key("volume-popover")
            .size(154.0, 65.0)
            .padding(11.0)
            .items_center()
            .pin(None, None, Some(24.0), Some(129.0))
            .raise()
            .radius(8.0)
            .gradient(ink.metal(), ink.panel)
            .border_gradient(1.0, ink.line, ink.line.alpha(0.30))
            .shadow(5.0, 12.0, Color::BLACK.alpha(0.58))
            .on_hover_change(State::hover_volume)
            .child(label(
                format!("Volume {}%", RangeSetting::Volume.get(state)),
                ink.fg,
            ))
            .child(slider(state, RangeSetting::Volume, ink.accent, ink, 126.0))
    }))
}

fn timer_display(state: &State, phase: Phase, status: Status, accent: Color, ink: Ink) -> El {
    let remaining = state.timer.remaining_seconds();
    let fraction = remaining as f32 / state.timer.total_seconds() as f32;
    let time = format!("{}:{:02}", remaining / 60, remaining % 60);
    let status_name = match status {
        Status::Running => "Running",
        Status::Paused => "Paused",
        Status::Waiting => "Starting",
        Status::Ready => "Ready",
    };
    let dot = el()
        .size(8.0, 8.0)
        .radius(4.0)
        .fill(accent)
        .shadow(0.0, 12.0, accent.alpha(0.74))
        .a11y_hidden();
    let progress = el()
        .w_full()
        .h(5.0)
        .radius(2.0)
        .fill(ink.line.alpha(0.22))
        .a11y_hidden()
        .child(
            el().w(340.0 * fraction)
                .h(5.0)
                .radius(2.0)
                .gradient(accent.mix(Color::WHITE, 0.23), accent)
                .shadow(0.0, 8.0, accent.alpha(0.36)),
        );

    col()
        .key("dial")
        .size(FACE_WIDTH, 166.0)
        .px(22.0)
        .pt(13.0)
        .radius(11.0)
        .fill(ink.well())
        .radial_gradient(
            [0.48, 0.30],
            [230.0, 145.0],
            [(0.0, accent.alpha(0.048)), (1.0, accent.alpha(0.0))],
        )
        .border_gradient(1.0, Color::BLACK.alpha(0.74), ink.line.alpha(0.49))
        .child(
            row()
                .w_full()
                .h(21.0)
                .center_y()
                .justify_between()
                .child(
                    row().gap(8.0).center_y().child(dot).child(
                        text(phase_name(phase).to_uppercase())
                            .key("phase")
                            .font(LEGEND.tracking(1.35))
                            .color(ink.fg),
                    ),
                )
                .child(text(status_name).font(LEGEND).color(ink.muted)),
        )
        .child(
            row()
                .w_full()
                .h(89.0)
                .center()
                .child(text(time).key("time-left").font(DIAL).color(ink.fg)),
        )
        .child(progress)
        .child(round_meter(state, accent, ink))
}

fn round_meter(state: &State, accent: Color, ink: Ink) -> El {
    let current = state.timer.round();
    let total = state.prefs.work_rounds;
    let completed = state.timer.completed_work();
    row()
        .w_full()
        .h(29.0)
        .gap(9.0)
        .center_y()
        .child(text("Round").font(LEGEND).color(ink.muted).a11y_hidden())
        .child(
            text(format!("{current}/{total}"))
                .key("round")
                .font(NUMBER)
                .color(ink.fg)
                .description_static("Current round / total rounds"),
        )
        .child(el().grow(1.0))
        .child(
            row()
                .gap(4.0)
                .center_y()
                .a11y_hidden()
                .children((1..=total).map(|round| {
                    let pip = el().size(12.0, 6.0).radius(2.0);
                    match round.cmp(&current) {
                        std::cmp::Ordering::Less => pip.fill(ink.focus.alpha(0.64)),
                        std::cmp::Ordering::Equal => {
                            pip.fill(accent).shadow(0.0, 7.0, accent.alpha(0.50))
                        }
                        std::cmp::Ordering::Greater => pip.fill(ink.line.alpha(0.20)),
                    }
                })),
        )
        .children((completed > 0).then(|| {
            text(format!("({completed})"))
                .key("completed-work")
                .font(LEGEND)
                .color(ink.muted)
                .description_static("Focus rounds completed")
        }))
}

fn timer_controls(state: &State, status: Status, ink: Ink) -> El {
    let (verb, action, symbol) = match status {
        Status::Running => ("Pause", "Pause timer", &PAUSE),
        Status::Paused => ("Resume", "Resume timer", &PLAY),
        _ => ("Start", "Start timer", &PLAY),
    };
    let muted = state.prefs.volume == 0;
    let secondary = |key, action, name, symbol| {
        button(key, action, ink)
            .size(53.0, 55.0)
            .gradient(ink.metal(), ink.panel)
            .border_gradient(1.0, ink.line.alpha(0.58), Color::BLACK.alpha(0.60))
            .shadow(2.0, 4.0, Color::BLACK.alpha(0.50))
            .press_scale(0.94)
            .child(
                col()
                    .center()
                    .gap(2.0)
                    .a11y_hidden()
                    .child(glyph(symbol, 19.0, ink.accent))
                    .child(text(name).font(LEGEND.tracking(0.15)).color(ink.fg)),
            )
    };
    row()
        .key("timer-footer")
        .size(FACE_WIDTH, 76.0)
        .px(10.0)
        .gap(7.0)
        .center_y()
        .radius(10.0)
        .gradient(ink.well(), ink.bg)
        .border_gradient(1.0, Color::BLACK.alpha(0.55), ink.line.alpha(0.44))
        .child(
            button("start-pause", action, ink)
                .grow(1.0)
                .h(55.0)
                .gradient(
                    if ink.on_accent() == Color::BLACK {
                        ink.accent.mix(Color::WHITE, 0.24)
                    } else {
                        ink.accent.mix(Color::BLACK, 0.10)
                    },
                    ink.accent,
                )
                .border_gradient(1.0, Color::WHITE.alpha(0.60), ink.on_accent().alpha(0.48))
                .shadow(3.0, 0.0, ink.accent.mix(Color::BLACK, 0.60))
                .shadow(4.0, 10.0, Color::BLACK.alpha(0.46))
                .hover(Color::WHITE.alpha(0.16))
                .press_scale(0.96)
                .help(action)
                .on_press_tx(|state: &mut State, tx| state.play_timer(TimerAction::Toggle, tx))
                .child(
                    row()
                        .center()
                        .gap(7.0)
                        .a11y_hidden()
                        .child(glyph(symbol, 24.0, ink.on_accent()))
                        .child(
                            text(verb)
                                .font(TextStyle::new(18.0, 760.0))
                                .color(ink.on_accent()),
                        ),
                ),
        )
        .child(
            secondary("reset", "Reset timer", "Reset", &RESET)
                .help("Reset timer")
                .on_press_tx(|state: &mut State, tx| state.play_timer(TimerAction::Reset, tx)),
        )
        .child(
            secondary("skip", "Skip round", "Skip", &SKIP)
                .help("Skip round")
                .on_press_tx(|state: &mut State, tx| state.play_timer(TimerAction::Skip, tx)),
        )
        .child(
            secondary(
                "mute",
                if muted { "Unmute" } else { "Mute" },
                "Sound",
                if muted { &MUTE } else { &SOUND },
            )
            .help("Mute or adjust volume")
            .on_hover_change(State::hover_volume)
            .on_press_tx(|state: &mut State, tx| {
                let volume = if state.prefs.volume == 0 {
                    state.last_volume
                } else {
                    0
                };
                RangeSetting::Volume.set(state, volume, tx);
                state.interface_click();
            }),
        )
}

pub fn tab_key(page: Page) -> &'static str {
    match page {
        Page::Timer => "tab-timer",
        Page::Options => "tab-options",
        Page::Themes => "tab-themes",
        Page::Avatars => "tab-avatars",
        Page::About => "tab-about",
    }
}

fn page_name(page: Page) -> &'static str {
    match page {
        Page::Timer => "Timer",
        Page::Options => "Options",
        Page::Themes => "Themes",
        Page::Avatars => "Avatars",
        Page::About => "About",
    }
}

fn drawer(state: &State, page: Page, ink: Ink) -> El {
    let (title, contents) = match page {
        Page::Timer => ("Round lengths", timer_settings(state, ink)),
        Page::Options => ("Behavior and sound", options(state, ink)),
        Page::Themes => ("Console themes", themes(state, ink)),
        Page::Avatars => ("Choose an avatar", avatars(state.prefs.avatar, ink)),
        Page::About => ("About Pomotroid", about(ink)),
    };
    let nav = row()
        .key("settings-tabs")
        .role("group")
        .label("Settings pages")
        .w_full()
        .h(52.0)
        .gap(5.0)
        .padding(5.0)
        .radius(10.0)
        .fill(ink.well())
        .border_gradient(1.0, Color::BLACK.alpha(0.55), ink.line.alpha(0.43))
        .children(
            [
                (Page::Timer, &CLOCK),
                (Page::Options, &GEAR),
                (Page::Themes, &PALETTE),
                (Page::Avatars, &AVATAR),
                (Page::About, &INFO),
            ]
            .map(|(tab, picture)| {
                let selected = page == tab;
                button(tab_key(tab), page_name(tab), ink)
                    .checked(selected)
                    .grow(1.0)
                    .h_full()
                    .gradient(
                        if selected {
                            ink.metal().mix(ink.accent, 0.10)
                        } else {
                            ink.metal()
                        },
                        ink.panel,
                    )
                    .border_gradient(
                        1.0,
                        if selected {
                            ink.accent.alpha(0.68)
                        } else {
                            ink.line.alpha(0.38)
                        },
                        if selected {
                            ink.accent.alpha(0.40)
                        } else {
                            Color::BLACK.alpha(0.48)
                        },
                    )
                    .press_scale(0.96)
                    .on_press_tx(move |state: &mut State, tx| {
                        state.interface_click();
                        state.select_page(Some(tab), tx);
                    })
                    .child(
                        row()
                            .center()
                            .gap(5.0)
                            .a11y_hidden()
                            .child(glyph(
                                picture,
                                14.0,
                                if selected { ink.accent } else { ink.muted },
                            ))
                            .child(
                                text(page_name(tab))
                                    .font(LEGEND.tracking(0.15))
                                    .color(ink.fg),
                            ),
                    )
            }),
        );

    col()
        .key("settings-drawer")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .items_center()
        .py(20.0)
        .child(
            col()
                .w(FACE_WIDTH)
                .grow(1.0)
                .min_h(0.0)
                .child(nav)
                .child(el().h(12.0))
                .child(
                    col()
                        .w_full()
                        .grow(1.0)
                        .min_h(0.0)
                        .radius(10.0)
                        .gradient(ink.panel, ink.well())
                        .border_gradient(1.0, ink.line.alpha(0.58), Color::BLACK.alpha(0.60))
                        .child(
                            row()
                                .w_full()
                                .h(56.0)
                                .px(20.0)
                                .center_y()
                                .border_sides([0.0, 0.0, 1.0, 0.0], ink.line.alpha(0.30))
                                .child(
                                    text(title)
                                        .font(TextStyle::new(21.0, 720.0))
                                        .color(ink.fg)
                                        .key("page-title")
                                        .role("heading")
                                        .level(2),
                                ),
                        )
                        .children(state.save_error.as_ref().map(|error| {
                            row()
                                .w_full()
                                .px(20.0)
                                .py(5.0)
                                .child(label(error.clone(), ink.fg).role("alert").wrap())
                        }))
                        .child(contents),
                ),
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
        .child(
            el().size(width, 5.0)
                .absolute(0.0, 9.5)
                .radius(2.5)
                .fill(ink.well())
                .border(1.0, ink.line.alpha(0.28)),
        )
        .child(
            el().size(position + HANDLE / 2.0, 3.0)
                .absolute(0.0, 10.5)
                .radius(1.5)
                .fill(accent),
        )
        .child(
            el().size(HANDLE, HANDLE)
                .absolute(position, 4.0)
                .radius(4.0)
                .gradient(accent.mix(Color::WHITE, 0.26), accent)
                .border_gradient(1.0, Color::WHITE.alpha(0.41), Color::BLACK.alpha(0.41))
                .shadow(2.0, 4.0, Color::BLACK.alpha(0.65)),
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
        .pt(8.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .items_center()
        .children(RangeSetting::TIMER_SETTINGS.map(|setting| {
            let accent = match setting {
                RangeSetting::Focus => ink.focus,
                RangeSetting::ShortBreak => ink.short,
                RangeSetting::LongBreak => ink.long,
                _ => ink.accent,
            };
            col()
                .w(342.0)
                .h(64.0)
                .pt(5.0)
                .gap(4.0)
                .border_sides([0.0, 0.0, 1.0, 0.0], ink.line.alpha(0.22))
                .child(
                    row()
                        .w_full()
                        .h(23.0)
                        .center_y()
                        .justify_between()
                        .child(label(setting.name(), ink.fg))
                        .child(
                            row()
                                .h(25.0)
                                .min_w(50.0)
                                .px(8.0)
                                .center()
                                .radius(4.0)
                                .border(1.0, ink.line.alpha(0.32))
                                .fill(ink.well())
                                .child(
                                    text(setting.display(setting.get(state)))
                                        .font(NUMBER)
                                        .color(ink.fg),
                                ),
                        ),
                )
                .child(slider(state, setting, accent, ink, 342.0))
        }))
        .child(el().h(13.0))
        .child(
            button("reset-defaults", "Reset timer defaults", ink)
                .size(145.0, 33.0)
                .gradient(ink.metal(), ink.panel)
                .border_gradient(1.0, ink.line.alpha(0.54), Color::BLACK.alpha(0.52))
                .on_press_tx(|state: &mut State, tx| {
                    state.apply_preferences(PreferenceChange::ResetTimerDefaults, tx);
                    state.interface_click();
                })
                .child(
                    text("Reset defaults")
                        .font(LEGEND.tracking(0.2))
                        .color(ink.fg),
                ),
        )
}

type Flag = fn(&mut Preferences) -> &mut bool;

fn toggle(key: &'static str, name: &'static str, enabled: bool, flag: Flag, ink: Ink) -> El {
    row()
        .w_full()
        .h(36.0)
        .justify_between()
        .center_y()
        .gap(8.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], ink.line.alpha(0.18))
        .child(label(name, ink.fg).grow(1.0).wrap())
        .child(
            el().key(key)
                .role("switch")
                .label(name)
                .checked(enabled)
                .focusable(true)
                .focus_ring(1.5, 2.0, ink.accent)
                .cursor(Cursor::Pointer)
                .size(40.0, 22.0)
                .radius(6.0)
                .fill(if enabled {
                    ink.accent.mix(ink.well(), 0.82)
                } else {
                    ink.well()
                })
                .border(
                    1.0,
                    if enabled {
                        ink.accent.alpha(0.58)
                    } else {
                        ink.line.alpha(0.45)
                    },
                )
                .hover(ink.line.alpha(0.19))
                .press_scale(0.95)
                .on_press_tx(move |state: &mut State, tx| {
                    let flag = flag(&mut state.prefs);
                    *flag = !*flag;
                    state.apply_preferences(PreferenceChange::Update, tx);
                    state.interface_click();
                })
                .child(
                    el().size(14.0, 14.0)
                        .radius(4.0)
                        .absolute(4.0, 4.0)
                        .offset(if enabled { 18.0 } else { 0.0 }, 0.0)
                        .gradient(
                            if enabled { ink.accent } else { ink.muted },
                            if enabled {
                                ink.accent.mix(ink.well(), 0.22)
                            } else {
                                ink.line
                            },
                        )
                        .a11y_hidden(),
                ),
        )
}

fn options(state: &State, ink: Ink) -> El {
    let p = &state.prefs;
    let entries: [(&str, &str, bool, Flag); 11] = [
        ("visual-motion", "Animation", p.visual_motion, |p| {
            &mut p.visual_motion
        }),
        (
            "interface-sounds",
            "Interface sounds",
            p.interface_sounds,
            |p| &mut p.interface_sounds,
        ),
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
        .min_h(0.0)
        .clip_rounded(10.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .px(20.0)
                .py(6.0)
                .children(
                    entries
                        .into_iter()
                        .filter(|(key, ..)| *key != "top-on-breaks" || p.always_on_top)
                        .map(|(key, name, enabled, flag)| toggle(key, name, enabled, flag, ink)),
                )
                .child(
                    row()
                        .w_full()
                        .h(44.0)
                        .center_y()
                        .justify_between()
                        .border_sides([0.0, 0.0, 1.0, 0.0], ink.line.alpha(0.18))
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
                    row().w_full().h(39.0).pt(8.0).center_y().child(
                        text("Global shortcuts")
                            .font(TextStyle::new(16.0, 700.0))
                            .color(ink.fg)
                            .role("heading")
                            .level(3),
                    ),
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
                                state.interface_click();
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
    let current = &state.theme().name;
    col()
        .key("theme-picker")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(10.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col().w_full().px(14.0).py(11.0).gap(7.0).children(
                state
                    .themes
                    .iter()
                    .filter(|theme| theme.name == "Pomotroid")
                    .chain(
                        state
                            .themes
                            .iter()
                            .filter(|theme| theme.name != "Pomotroid"),
                    )
                    .map(|theme| theme_sample(theme, current, ink)),
            ),
        )
}

fn theme_sample(theme: &Theme, current: &str, ink: Ink) -> El {
    let preview = Ink::from(theme);
    let name = theme.name.clone();
    let selected = current == name;
    let key = format!("theme-{}", name.to_lowercase().replace(' ', "-"));
    button(key, &name, ink)
        .checked(selected)
        .w_full()
        .h(45.0)
        .gradient(preview.metal(), preview.bg)
        .border_gradient(
            1.0,
            if selected {
                preview.accent
            } else {
                preview.line.alpha(0.60)
            },
            if selected {
                preview.accent.alpha(0.55)
            } else {
                preview.line.alpha(0.22)
            },
        )
        .on_press_tx(move |state: &mut State, tx| {
            state.prefs.theme = Some(name.clone());
            state.apply_preferences(PreferenceChange::Update, tx);
            state.interface_click();
        })
        .child(
            row()
                .w_full()
                .h_full()
                .px(12.0)
                .gap(10.0)
                .center_y()
                .a11y_hidden()
                .child(row().w(49.0).gap(3.0).center_y().children(
                    [preview.focus, preview.short, preview.long].map(|color| {
                        el().size(13.0, 14.0)
                            .radius(3.0)
                            .gradient(color.mix(Color::WHITE, 0.21), color)
                            .border(1.0, color.mix(Color::BLACK, 0.17))
                    }),
                ))
                .child(label(theme.name.clone(), preview.fg).grow(1.0).truncate())
                .child(
                    el().size(19.0, 20.0)
                        .center()
                        .children(selected.then(|| glyph(&CHECK, 16.0, preview.accent))),
                ),
        )
}

fn avatars(current: Avatar, ink: Ink) -> El {
    col()
        .key("avatar-picker")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(10.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .px(16.0)
                .py(16.0)
                .gap(12.0)
                .children(Avatar::ALL.map(|avatar| avatar_option(avatar, current, ink))),
        )
}

pub fn avatar_key(avatar: Avatar) -> String {
    format!("avatar-{}", avatar.key())
}

fn avatar_option(avatar: Avatar, current: Avatar, ink: Ink) -> El {
    let selected = avatar == current;
    button(avatar_key(avatar), avatar.name(), ink)
        .checked(selected)
        .description_static(avatar.description())
        .w_full()
        .h(96.0)
        .radius(9.0)
        .gradient(ink.metal(), ink.panel)
        .border_gradient(
            1.0,
            if selected {
                ink.accent
            } else {
                ink.line.alpha(0.56)
            },
            if selected {
                ink.accent.alpha(0.55)
            } else {
                ink.line.alpha(0.28)
            },
        )
        .on_press_tx(move |state: &mut State, tx| state.choose_avatar(avatar, tx))
        .child(
            row()
                .w_full()
                .h_full()
                .px(12.0)
                .gap(14.0)
                .center_y()
                .a11y_hidden()
                .child(
                    el().size(84.0, 68.0)
                        .radius(6.0)
                        .center()
                        .fill(ink.well())
                        .border(1.0, ink.line.alpha(0.30))
                        .child(crate::reactor::portrait(avatar)),
                )
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(3.0)
                        .child(
                            text(avatar.name())
                                .font(TextStyle::new(17.0, 720.0))
                                .color(ink.fg),
                        )
                        .child(label(avatar.description(), ink.muted).w_full().wrap()),
                )
                .child(
                    el().size(24.0, 24.0)
                        .center()
                        .radius(12.0)
                        .border(1.0, ink.line.alpha(0.72))
                        .children(selected.then(|| {
                            el().size(24.0, 24.0)
                                .center()
                                .radius(12.0)
                                .fill(ink.accent)
                                .child(glyph(&CHECK, 15.0, ink.on_accent()))
                        })),
                ),
        )
}

fn about(ink: Ink) -> El {
    let version = env!("CARGO_PKG_VERSION");
    let link = |key, name, url: String| {
        button(key, name, ink)
            .w(220.0)
            .h(35.0)
            .gradient(ink.metal(), ink.panel)
            .border_gradient(1.0, ink.line.alpha(0.48), Color::BLACK.alpha(0.52))
            .href(url)
            .child(label(name, ink.fg))
    };
    col()
        .key("about")
        .w_full()
        .grow(1.0)
        .items_center()
        .gap(10.0)
        .child(el().h(22.0))
        .child(
            el().size(80.0, 80.0)
                .radius(15.0)
                .gradient(ink.metal(), ink.well())
                .border_gradient(1.0, ink.line.alpha(0.75), ink.line.alpha(0.32))
                .center()
                .child(glyph(&TOMATO, 50.0, ink.focus)),
        )
        .child(
            text(format!("Version {version}"))
                .font(NUMBER)
                .color(ink.muted),
        )
        .child(el().h(6.0))
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
