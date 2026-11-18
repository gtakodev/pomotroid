use crate::avatar::Avatar;
use chrono::{Local, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::{Builder, NamedTempFile};

const PREFERENCES_FILE: &str = "user-preferences.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub always_on_top: bool,
    pub break_always_on_top: bool,
    pub auto_start_work_timer: bool,
    pub auto_start_break_timer: bool,
    pub min_to_tray: bool,
    pub min_to_tray_on_close: bool,
    pub notifications: bool,
    #[serde(deserialize_with = "read_number")]
    pub work_rounds: u32,
    pub theme: Option<String>,
    pub avatar: Avatar,
    pub visual_motion: bool,
    pub interface_sounds: bool,
    pub tick_sounds: bool,
    pub tick_sounds_during_break: bool,
    #[serde(deserialize_with = "read_number")]
    pub time_long_break: u32,
    #[serde(deserialize_with = "read_number")]
    pub time_short_break: u32,
    #[serde(deserialize_with = "read_number")]
    pub time_work: u32,
    #[serde(deserialize_with = "read_number")]
    pub volume: u32,
    pub global_shortcuts: GlobalShortcuts,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            always_on_top: false,
            break_always_on_top: false,
            auto_start_work_timer: true,
            auto_start_break_timer: true,
            min_to_tray: false,
            min_to_tray_on_close: false,
            notifications: true,
            work_rounds: 4,
            theme: None,
            avatar: Avatar::default(),
            visual_motion: true,
            interface_sounds: true,
            tick_sounds: false,
            tick_sounds_during_break: true,
            time_long_break: 15,
            time_short_break: 5,
            time_work: 25,
            volume: 100,
            global_shortcuts: GlobalShortcuts::default(),
            extra: Map::new(),
        }
    }
}

impl Preferences {
    pub fn reset_timer_defaults(&mut self) {
        let defaults = Self::default();
        self.time_work = defaults.time_work;
        self.time_short_break = defaults.time_short_break;
        self.time_long_break = defaults.time_long_break;
        self.work_rounds = defaults.work_rounds;
    }
}

fn read_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SettingNumber {
        Number(u32),
        Text(String),
    }

    match SettingNumber::deserialize(deserializer)? {
        SettingNumber::Number(value) => Ok(value),
        SettingNumber::Text(value) => value.trim().parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalShortcuts {
    #[serde(rename = "call-timer-toggle")]
    pub toggle: String,
    #[serde(rename = "call-timer-reset")]
    pub reset: String,
    #[serde(rename = "call-timer-skip")]
    pub skip: String,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl Default for GlobalShortcuts {
    fn default() -> Self {
        Self {
            toggle: "Control+F1".into(),
            reset: "Control+F2".into(),
            skip: "Control+F3".into(),
            extra: Map::new(),
        }
    }
}

pub struct PreferencesStore {
    directory: PathBuf,
}

impl PreferencesStore {
    pub fn discover() -> io::Result<Self> {
        if let Some(directory) = std::env::var_os("POMOTROID_HOME").filter(|path| !path.is_empty())
        {
            return Ok(Self::new(PathBuf::from(directory)));
        }
        let root = dirs::config_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "user configuration folder unavailable",
            )
        })?;
        Ok(Self::for_config_root(&root))
    }

    fn for_config_root(root: &Path) -> Self {
        let packaged = root.join("Pomotroid");
        let documented = root.join("pomotroid");
        let profile = |directory: &Path| {
            (
                directory.join(PREFERENCES_FILE).is_file(),
                directory.join("themes").is_dir() || directory.join("logs").is_dir(),
            )
        };
        Self::new(if profile(&packaged) > profile(&documented) {
            packaged
        } else {
            documented
        })
    }

    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn load(&self) -> io::Result<Preferences> {
        let contents = match fs::read(self.directory.join(PREFERENCES_FILE)) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Preferences::default());
            }
            Err(error) => return Err(error),
        };
        let mut preferences = parse_preferences(&contents)?;
        preferences.time_work = preferences.time_work.clamp(1, 90);
        preferences.time_short_break = preferences.time_short_break.clamp(1, 90);
        preferences.time_long_break = preferences.time_long_break.clamp(1, 90);
        preferences.work_rounds = preferences.work_rounds.clamp(1, 12);
        preferences.volume = preferences.volume.min(100);
        Ok(preferences)
    }

    pub fn save(&self, preferences: &Preferences) -> io::Result<()> {
        fs::create_dir_all(&self.directory)?;
        let path = self.directory.join(PREFERENCES_FILE);
        let mut pending = NamedTempFile::new_in(&self.directory)?;
        serde_json::to_writer_pretty(&mut pending, preferences)?;
        pending.write_all(b"\n")?;
        pending.as_file().sync_all()?;

        match fs::read(&path) {
            Ok(contents) if parse_preferences(&contents).is_err() => {
                let mut backup = Builder::new()
                    .prefix("user-preferences.invalid-")
                    .suffix(".json")
                    .tempfile_in(&self.directory)?;
                backup.write_all(&contents)?;
                backup.as_file().sync_all()?;
                backup.keep().map_err(|error| error.error)?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        pending.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

fn parse_preferences(bytes: &[u8]) -> io::Result<Preferences> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let preferences = serde_json::from_slice::<Map<String, Value>>(bytes)
        .and_then(|object| serde_json::from_value(Value::Object(object)));
    preferences.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub struct ActivityLog {
    directory: PathBuf,
    hostname: Option<String>,
}

impl ActivityLog {
    pub fn new(data_directory: &Path) -> Self {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok()
            .or_else(|| fs::read_to_string("/etc/hostname").ok())
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty());
        Self {
            directory: data_directory.join("logs"),
            hostname,
        }
    }

    pub fn record(&self, message: &str) -> io::Result<()> {
        self.record_at(Utc::now(), message)
    }

    fn record_at(&self, now: chrono::DateTime<Utc>, message: &str) -> io::Result<()> {
        let date = now.with_timezone(&Local).format("%Y-%m-%d");
        let mut entry = serde_json::to_vec(&serde_json::json!({
            "hostname": self.hostname,
            "level": "info",
            "message": message,
            "timestamp": now.to_rfc3339_opts(SecondsFormat::Millis, true),
        }))?;
        entry.push(b'\n');

        fs::create_dir_all(&self.directory)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join(format!("pomotroid-{date}.log")))?
            .write_all(&entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_preferences_round_trip_with_missing_and_unknown_options() {
        let folder = tempfile::tempdir().unwrap();
        let store = PreferencesStore::new(folder.path().to_owned());
        assert_eq!(store.load().unwrap(), Preferences::default());
        let path = folder.path().join(PREFERENCES_FILE);
        fs::write(
            &path,
            br#"{
                "alwaysOnTop": true,
                "autoStartBreakTimer": false,
                "timeWork": "42",
                "volume": 0,
                "theme": "Dracula",
                "globalShortcuts": {"call-timer-toggle": "Shift+F4", "extra": "F11"},
                "futureOption": [1, 2, 3]
            }"#,
        )
        .unwrap();

        let mut preferences = store.load().unwrap();
        assert!(preferences.always_on_top);
        assert!(preferences.auto_start_work_timer);
        assert!(!preferences.auto_start_break_timer);
        assert_eq!(preferences.time_work, 42);
        assert_eq!(preferences.volume, 0);
        assert_eq!(preferences.theme.as_deref(), Some("Dracula"));
        assert!(preferences.visual_motion);
        assert_eq!(preferences.avatar, Avatar::Tomato);
        assert!(preferences.interface_sounds);
        assert_eq!(preferences.global_shortcuts.toggle, "Shift+F4");
        assert_eq!(preferences.global_shortcuts.reset, "Control+F2");

        preferences.visual_motion = false;
        preferences.avatar = Avatar::Clack;
        preferences.interface_sounds = false;
        preferences.reset_timer_defaults();
        store.save(&preferences).unwrap();
        assert_eq!(store.load().unwrap(), preferences);
        let raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["timeWork"], 25);
        assert_eq!(raw["visualMotion"], false);
        assert_eq!(raw["avatar"], "clack");
        assert_eq!(raw["interfaceSounds"], false);
        assert_eq!(raw["futureOption"], serde_json::json!([1, 2, 3]));
        assert_eq!(raw["globalShortcuts"]["extra"], "F11");
    }

    #[test]
    fn unknown_avatar_keeps_the_rest_of_the_profile() {
        let prefs = parse_preferences(br#"{"avatar":"visitor","timeWork":37}"#).unwrap();
        assert_eq!(prefs.avatar, Avatar::Tomato);
        assert_eq!(prefs.time_work, 37);
    }

    #[test]
    fn saving_after_corruption_keeps_the_original_bytes() {
        let folder = tempfile::tempdir().unwrap();
        let store = PreferencesStore::new(folder.path().to_owned());
        let corrupted = b"{ broken settings";
        fs::write(folder.path().join(PREFERENCES_FILE), corrupted).unwrap();
        assert_eq!(store.load().unwrap_err().kind(), io::ErrorKind::InvalidData);

        store.save(&Preferences::default()).unwrap();
        assert_eq!(store.load().unwrap(), Preferences::default());
        let backup = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(".invalid-")
            })
            .unwrap();
        assert_eq!(fs::read(backup).unwrap(), corrupted);
    }

    #[test]
    fn finds_the_packaged_electron_app_directory() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("Pomotroid");
        let store = PreferencesStore::new(directory.clone());
        store
            .save(&Preferences {
                time_work: 55,
                ..Preferences::default()
            })
            .unwrap();
        let discovered = PreferencesStore::for_config_root(root.path());
        assert_eq!(discovered.load().unwrap().time_work, 55);
    }

    #[test]
    fn appends_legacy_json_lines_to_daily_log() {
        let folder = tempfile::tempdir().unwrap();
        let log = ActivityLog::new(folder.path());
        let now = Utc::now();
        let path = log.directory.join(format!(
            "pomotroid-{}.log",
            now.with_timezone(&Local).format("%Y-%m-%d")
        ));
        fs::create_dir_all(&log.directory).unwrap();
        fs::write(&path, "{\"message\":\"existing\"}\n").unwrap();
        log.record_at(now, "focus round completed").unwrap();

        let contents = fs::read_to_string(path).unwrap();
        let entries: Vec<Value> = contents
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(entries[0]["message"], "existing");
        assert_eq!(entries[1]["message"], "focus round completed");
        assert_eq!(entries[1]["level"], "info");
        assert!(
            chrono::DateTime::parse_from_rfc3339(entries[1]["timestamp"].as_str().unwrap()).is_ok()
        );
    }
}
