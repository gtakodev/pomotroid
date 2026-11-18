use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const DEFAULT_JSON: &str = include_str!("../static/themes/pomotroid.json");
const BUNDLED_THEMES: &[&str] = &[
    include_str!("../static/themes/andromeda.json"),
    include_str!("../static/themes/ayu.json"),
    include_str!("../static/themes/city-lights.json"),
    include_str!("../static/themes/dracula.json"),
    include_str!("../static/themes/dva.json"),
    include_str!("../static/themes/github.json"),
    include_str!("../static/themes/graphite.json"),
    include_str!("../static/themes/gruvbox.json"),
    include_str!("../static/themes/monokai.json"),
    include_str!("../static/themes/nord.json"),
    include_str!("../static/themes/one-dark.json"),
    DEFAULT_JSON,
    include_str!("../static/themes/popping-and-locking.json"),
    include_str!("../static/themes/solarized-light.json"),
    include_str!("../static/themes/spandex.json"),
    include_str!("../static/themes/synthwave.json"),
    include_str!("../static/themes/tokyo-night.json"),
];

static DEFAULT_THEME: LazyLock<Theme> =
    LazyLock::new(|| Theme::parse(DEFAULT_JSON, None).expect("invalid bundled Pomotroid theme"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub long_round: u32,
    pub short_round: u32,
    pub focus_round: u32,
    pub background: u32,
    pub background_light: u32,
    pub background_lightest: u32,
    pub foreground: u32,
    pub foreground_darker: u32,
    pub foreground_darkest: u32,
    pub accent: u32,
}

impl Theme {
    fn parse(json: &str, fallback: Option<&Self>) -> Option<Self> {
        #[derive(Deserialize)]
        struct ThemeFile {
            name: String,
            colors: BTreeMap<String, String>,
        }

        let json = json.strip_prefix('\u{feff}').unwrap_or(json);
        let ThemeFile { name, colors } = serde_json::from_str(json).ok()?;
        if name.trim().is_empty() {
            return None;
        }
        let color = |name, default: fn(&Self) -> u32| {
            colors
                .get(name)
                .and_then(|value| csscolorparser::parse(value).ok())
                .map(|value| u32::from_be_bytes(value.to_rgba8()))
                .or_else(|| fallback.map(default))
        };
        Some(Self {
            name,
            long_round: color("--color-long-round", |theme| theme.long_round)?,
            short_round: color("--color-short-round", |theme| theme.short_round)?,
            focus_round: color("--color-focus-round", |theme| theme.focus_round)?,
            background: color("--color-background", |theme| theme.background)?,
            background_light: color("--color-background-light", |theme| theme.background_light)?,
            background_lightest: color("--color-background-lightest", |theme| {
                theme.background_lightest
            })?,
            foreground: color("--color-foreground", |theme| theme.foreground)?,
            foreground_darker: color("--color-foreground-darker", |theme| theme.foreground_darker)?,
            foreground_darkest: color("--color-foreground-darkest", |theme| {
                theme.foreground_darkest
            })?,
            accent: color("--color-accent", |theme| theme.accent)?,
        })
    }
}

pub fn load_themes(data_directory: &Path) -> Vec<Theme> {
    let mut themes = bundled_themes();
    let directory = data_directory.join("themes");
    if let Err(error) = fs::create_dir_all(&directory) {
        eprintln!(
            "Cannot create theme directory {}: {error}",
            directory.display()
        );
        return themes;
    }
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!(
                "Cannot read theme directory {}: {error}",
                directory.display()
            );
            return themes;
        }
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                && path.is_file()
        })
        .collect();
    paths.sort();

    let mut names: HashSet<_> = themes.iter().map(|theme| theme.name.clone()).collect();
    for path in paths {
        let theme = fs::read_to_string(&path)
            .ok()
            .and_then(|json| Theme::parse(&json, Some(&DEFAULT_THEME)));
        if let Some(theme) = theme {
            if names.insert(theme.name.clone()) {
                themes.push(theme);
            }
        } else {
            eprintln!("Cannot load theme {}", path.display());
        }
    }
    themes
}

pub fn bundled_themes() -> Vec<Theme> {
    BUNDLED_THEMES
        .iter()
        .map(|json| Theme::parse(json, None).expect("invalid bundled theme"))
        .collect()
}

pub fn selected_theme<'a>(themes: &'a [Theme], name: Option<&str>) -> &'a Theme {
    name.and_then(|name| {
        themes.iter().find(|theme| theme.name == name).or_else(|| {
            themes
                .iter()
                .find(|theme| theme.name.eq_ignore_ascii_case(name))
        })
    })
    .unwrap_or(&DEFAULT_THEME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_bundled_and_custom_themes_with_css_colors() {
        let data_directory = tempfile::tempdir().unwrap();
        let directory = data_directory.path().join("themes");
        fs::create_dir(&directory).unwrap();
        fs::write(
            directory.join("my-theme.json"),
            r##"{"name":"My Theme","colors":{
                "--color-background":"rgba(32, 128, 64, 0.5)",
                "--color-foreground":"RebeccaPurple",
                "--color-short-round":"hsl(120 100% 50%)",
                "--color-accent":"#abcf"
            }}"##,
        )
        .unwrap();
        fs::write(directory.join("broken.json"), "not a theme").unwrap();
        fs::write(
            directory.join("duplicate.json"),
            r#"{"name":"Pomotroid","colors":{"--color-background":"black"}}"#,
        )
        .unwrap();

        let themes = load_themes(data_directory.path());
        assert_eq!(themes.len(), 18);
        let theme = selected_theme(&themes, Some("My Theme"));
        assert_eq!(theme.background, 0x20804080);
        assert_eq!(theme.foreground, 0x663399ff);
        assert_eq!(theme.short_round, 0x00ff00ff);
        assert_eq!(theme.accent, 0xaabbccff);
        assert_eq!(theme.focus_round, DEFAULT_THEME.focus_round);
        assert_eq!(
            selected_theme(&themes, Some("Pomotroid")).background,
            0x2f384bff
        );
        assert_eq!(selected_theme(&themes, Some("Missing")), &*DEFAULT_THEME);
    }
}
