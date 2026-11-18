use super::DesktopAction;
use std::sync::Arc;
type Handler = Arc<dyn Fn(DesktopAction) + Send + Sync>;

fn pixels(phase: &str, running: bool, fraction: f32) -> Vec<u8> {
    let accent = match phase {
        "long-break" => [11, 188, 218, 255],
        "short-break" => [5, 235, 139, 255],
        _ => [255, 78, 77, 255],
    };
    let mut bytes = Vec::with_capacity(32 * 32 * 4);
    for row in 0..32 {
        for col in 0..32 {
            let x = (col as f32 + 0.5) - 16.0;
            let y = (row as f32 + 0.5) - 16.0;
            let radius = x.hypot(y);
            let turn = x.atan2(-y).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
            let lit = if running {
                (9.0..13.0).contains(&radius) && turn < fraction
            } else {
                (2.0..5.5).contains(&x.abs()) && y.abs() <= 8.0
            };
            let color = if lit {
                accent
            } else if radius < 15.5 {
                [47, 56, 75, 255]
            } else {
                [0, 0, 0, 0]
            };
            bytes.extend_from_slice(&color);
        }
    }
    bytes
}

fn tooltip(phase: &str, remaining: u32) -> String {
    let label = match phase {
        "short-break" => "Short break",
        "long-break" => "Long break",
        _ => "Focus",
    };
    format!(
        "Pomotroid · {label} {:02}:{:02}",
        remaining / 60,
        remaining % 60
    )
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native {
    use super::*;
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem},
    };

    pub(crate) fn install_handlers(handler: Handler) {
        let menu_handler = handler.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| match e.id.as_ref() {
            "pomotroid-view" => menu_handler(DesktopAction::Show(None)),
            "pomotroid-quit" => menu_handler(DesktopAction::Quit),
            _ => (),
        }));
        TrayIconEvent::set_event_handler(Some(move |e| {
            if matches!(
                e,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                handler(DesktopAction::Show(None));
            }
        }));
    }

    pub(crate) struct Tray {
        icon: TrayIcon,
        last: Option<(String, bool, u32)>,
    }
    impl Tray {
        pub(crate) fn new(_: Handler) -> Result<Self, Box<dyn std::error::Error>> {
            let menu = Menu::new();
            menu.append(&MenuItem::with_id("pomotroid-view", "View", true, None))?;
            menu.append(&MenuItem::with_id("pomotroid-quit", "Quit", true, None))?;
            let icon = TrayIconBuilder::new()
                .with_id("pomotroid")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_tooltip("Pomotroid")
                .with_icon(Icon::from_rgba(pixels("work", false, 1.0), 32, 32)?)
                .build()?;
            Ok(Self { icon, last: None })
        }
        pub(crate) fn update(&mut self, phase: &str, running: bool, remaining: u32, total: u32) {
            let percent = remaining * 100 / total.max(1);
            let key = (phase.to_owned(), running, percent);
            if self.last.as_ref() != Some(&key) {
                self.last = Some(key);
                if let Ok(icon) =
                    Icon::from_rgba(pixels(phase, running, percent as f32 / 100.0), 32, 32)
                {
                    let _ = self.icon.set_icon(Some(icon));
                }
            }
            let _ = self.icon.set_tooltip(Some(tooltip(phase, remaining)));
        }
    }
}

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use ksni::{Icon, MenuItem, ToolTip, blocking, blocking::TrayMethods, menu::StandardItem};

    pub(crate) fn install_handlers(_: Handler) {}

    pub(crate) struct Tray(blocking::Handle<StatusItem>);

    impl Tray {
        pub(crate) fn new(handler: Handler) -> Result<Self, ksni::Error> {
            StatusItem {
                handler,
                icon: status_icon("work", false, 100),
                icon_state: ("work".into(), false, 100),
                tooltip: "Pomotroid".into(),
            }
            .spawn()
            .map(Self)
        }

        pub(crate) fn update(&mut self, phase: &str, running: bool, remaining: u32, total: u32) {
            let percent = u64::from(remaining.min(total)) * 100 / u64::from(total.max(1));
            let state = (phase.to_owned(), running, percent);
            let _ = self.0.update(|item| {
                if item.icon_state != state {
                    item.icon = status_icon(phase, running, percent);
                    item.icon_state = state;
                }
                item.tooltip = tooltip(phase, remaining);
            });
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            self.0.shutdown();
        }
    }

    struct StatusItem {
        handler: Handler,
        icon: Icon,
        icon_state: (String, bool, u64),
        tooltip: String,
    }

    impl StatusItem {
        fn show(&mut self) {
            (self.handler)(DesktopAction::Show(ksni::current_xdg_activation_token()));
        }
    }

    impl ksni::Tray for StatusItem {
        fn id(&self) -> String {
            "pomotroid".into()
        }

        fn title(&self) -> String {
            "Pomotroid".into()
        }

        fn icon_pixmap(&self) -> Vec<Icon> {
            vec![self.icon.clone()]
        }

        fn tool_tip(&self) -> ToolTip {
            ToolTip {
                title: self.tooltip.clone(),
                ..ToolTip::default()
            }
        }

        fn activate(&mut self, _: i32, _: i32) {
            self.show();
        }

        fn menu(&self) -> Vec<MenuItem<Self>> {
            vec![
                StandardItem {
                    label: "View".into(),
                    activate: Box::new(Self::show),
                    ..StandardItem::default()
                }
                .into(),
                StandardItem {
                    label: "Quit".into(),
                    activate: Box::new(|item: &mut Self| (item.handler)(DesktopAction::Quit)),
                    ..StandardItem::default()
                }
                .into(),
            ]
        }
    }

    fn status_icon(phase: &str, running: bool, percent: u64) -> Icon {
        let mut data = pixels(phase, running, percent as f32 / 100.0);
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        Icon {
            width: 32,
            height: 32,
            data,
        }
    }
}

pub(super) use native::{Tray, install_handlers};
