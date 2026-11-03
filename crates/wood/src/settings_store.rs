//! Reads and writes the WoodCraft settings file (`%APPDATA%\WoodCraft\settings.json`, or the
//! platform equivalent). Unlike [`crate::store`] (the part JSON kept inside a drawing), this is
//! a plain file on disk, shared by every drawing: straightness tolerance, units, joint spacing,
//! build-size limits, the Common size presets, and the managed lists.
//!
//! Follows the same shape as CADCraft's own native workspace file (atomic write through a temp
//! file plus rename, a byte cap, a `version` field, and a corrupt or newer file left untouched
//! rather than overwritten).

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::settings_data::WoodSettings;

const MAX_BYTES: u64 = 64 * 1024;
const FILE_VERSION: u64 = 1;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error("{0}: the settings file is {1} bytes, over the 64 KiB limit")]
    TooLarge(String, u64),
    #[error("{0}: invalid settings JSON: {1}")]
    Json(String, serde_json::Error),
    #[error("{0}: unsupported settings file version")]
    Version(String),
    #[error("no settings folder could be found for this platform")]
    NoConfigDir,
}

/// `%APPDATA%\WoodCraft` on Windows, `~/Library/Application Support/WoodCraft` on macOS,
/// `$XDG_CONFIG_HOME/woodcraft` (or `~/.config/woodcraft`) elsewhere. A `WOODCRAFT_CONFIG_DIR`
/// override, checked first on every platform, lets tests (and advanced users) redirect it.
fn config_dir_from(platform: &str, get: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if let Some(root) = get("WOODCRAFT_CONFIG_DIR") {
        return (!root.is_empty()).then(|| PathBuf::from(root));
    }
    let nonempty = |key| get(key).filter(|value| !value.is_empty()).map(PathBuf::from);
    match platform {
        "macos" => nonempty("HOME").map(|home| home.join("Library/Application Support/WoodCraft")),
        "windows" => nonempty("APPDATA").map(|root| root.join("WoodCraft")),
        _ => nonempty("XDG_CONFIG_HOME").map(|root| root.join("woodcraft")).or_else(|| nonempty("HOME").map(|home| home.join(".config/woodcraft"))),
    }
}

thread_local! {
    /// A path override for tests, so each test thread can use its own settings file without
    /// touching the real per-platform path or any process-wide environment variable (`set_var`
    /// is `unsafe` under this workspace's `unsafe_code = "forbid"` lint, and a shared env var
    /// would race across threads in any case).
    static TEST_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Test-only: use `path` for every [`get`]/[`update`] call on this thread, instead of the real
/// per-platform settings path. `None` restores the normal resolution.
#[doc(hidden)]
pub fn set_test_path(path: Option<PathBuf>) {
    TEST_PATH.with(|cell| *cell.borrow_mut() = path);
}

fn settings_path() -> Result<PathBuf, SettingsError> {
    if let Some(path) = TEST_PATH.with(|cell| cell.borrow().clone()) {
        return Ok(path);
    }
    config_dir_from(std::env::consts::OS, |key| std::env::var_os(key)).map(|dir| dir.join("settings.json")).ok_or(SettingsError::NoConfigDir)
}

fn read(path: &Path) -> Result<WoodSettings, SettingsError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(WoodSettings::default()),
        Err(error) => return Err(SettingsError::Io(path.display().to_string(), error)),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|error| SettingsError::Io(path.display().to_string(), error))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(SettingsError::TooLarge(path.display().to_string(), bytes.len() as u64));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| SettingsError::Json(path.display().to_string(), error))?;
    if value.get("version").and_then(serde_json::Value::as_u64) != Some(FILE_VERSION) {
        return Err(SettingsError::Version(path.display().to_string()));
    }
    let settings = value.get("settings").cloned().unwrap_or(serde_json::Value::Null);
    serde_json::from_value(settings).map_err(|error| SettingsError::Json(path.display().to_string(), error))
}

fn write(path: &Path, settings: &WoodSettings) -> Result<(), SettingsError> {
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({ "version": FILE_VERSION, "settings": settings }))
        .map_err(|error| SettingsError::Json(path.display().to_string(), error))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(SettingsError::TooLarge(path.display().to_string(), bytes.len() as u64));
    }
    let parent = path.parent().ok_or(SettingsError::NoConfigDir)?;
    fs::create_dir_all(parent).map_err(|error| SettingsError::Io(parent.display().to_string(), error))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".settings-{}-{sequence}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| SettingsError::Io(temporary.display().to_string(), error))?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(SettingsError::Io(path.display().to_string(), error));
    }
    Ok(())
}

/// The settings as they stand on disk now (the defaults, seeded with the Common size presets
/// and the managed lists, if no file exists yet).
pub fn get() -> Result<WoodSettings, SettingsError> {
    read(&settings_path()?)
}

/// The Model Warehouse folder: the `warehouseFolder` setting if one is set, otherwise the
/// per-platform default (a `warehouse` folder next to the settings file). `None` only when no
/// config folder could be found for this platform and no setting override is in place either.
pub fn warehouse_dir() -> Result<PathBuf, SettingsError> {
    let settings = get()?;
    if let Some(folder) = settings.warehouse_folder.filter(|f| !f.trim().is_empty()) {
        return Ok(PathBuf::from(folder));
    }
    Ok(settings_path()?.parent().ok_or(SettingsError::NoConfigDir)?.join("warehouse"))
}

/// Read, change and write back.
pub fn update<R>(change: impl FnOnce(&mut WoodSettings) -> R) -> Result<R, SettingsError> {
    let path = settings_path()?;
    let mut settings = read(&path)?;
    let r = change(&mut settings);
    write(&path, &settings)?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_data::CommonSizePreset;

    fn temp_dir() -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("woodcraft-settings-test-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&path).unwrap_or(());
        path
    }

    #[test]
    fn config_dir_resolution_matches_each_platform_and_the_override() {
        let get = |key: &str| match key {
            "WOODCRAFT_CONFIG_DIR" => Some(OsString::from("/private/test-state")),
            "HOME" | "APPDATA" | "XDG_CONFIG_HOME" => Some(OsString::from("/unrelated-user-profile")),
            _ => None,
        };
        for platform in ["macos", "windows", "linux", "freebsd"] {
            assert_eq!(config_dir_from(platform, get), Some(PathBuf::from("/private/test-state")));
            assert_eq!(config_dir_from(platform, |key| if key == "WOODCRAFT_CONFIG_DIR" { Some(OsString::new()) } else { get(key) }), None);
        }
        assert_eq!(
            config_dir_from("windows", |key| (key == "APPDATA").then(|| OsString::from("/Users/x/AppData/Roaming"))),
            Some(PathBuf::from("/Users/x/AppData/Roaming/WoodCraft"))
        );
        assert_eq!(
            config_dir_from("macos", |key| (key == "HOME").then(|| OsString::from("/Users/x"))),
            Some(PathBuf::from("/Users/x/Library/Application Support/WoodCraft"))
        );
        assert_eq!(
            config_dir_from("linux", |key| (key == "XDG_CONFIG_HOME").then(|| OsString::from("/config"))),
            Some(PathBuf::from("/config/woodcraft"))
        );
        assert_eq!(config_dir_from("macos", |_| None), None);
    }

    #[test]
    fn missing_file_reads_as_defaults_without_creating_anything() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let settings = read(&path).unwrap();
        assert_eq!(settings, WoodSettings::default());
        assert!(!path.exists());
        fs::remove_dir_all(dir).unwrap_or(());
    }

    #[test]
    fn round_trip_through_write_and_read() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let mut settings = WoodSettings { units: "mm".into(), ..WoodSettings::default() };
        settings.common_size_presets.push(CommonSizePreset { name: "Scrap".into(), width: 2.0, thickness: 0.5 });
        write(&path, &settings).unwrap();
        let reloaded = read(&path).unwrap();
        assert_eq!(reloaded, settings);
        fs::remove_dir_all(dir).unwrap_or(());
    }

    #[test]
    fn malformed_oversized_and_newer_version_files_are_reported_not_overwritten() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        for bytes in [b"{not json".to_vec(), vec![b' '; MAX_BYTES as usize + 1], br#"{"version":2,"settings":{}}"#.to_vec()] {
            fs::write(&path, &bytes).unwrap();
            assert!(read(&path).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::remove_dir_all(dir).unwrap_or(());
    }

    #[test]
    fn update_seeds_defaults_on_first_use_then_changes_only_what_was_asked() {
        let dir = temp_dir();
        set_test_path(Some(dir.join("settings.json")));
        let before = update(|w| w.units.clone()).unwrap();
        assert_eq!(before, "in");
        update(|w| w.units = "mm".into()).unwrap();
        let after = get().unwrap();
        assert_eq!(after.units, "mm");
        assert_eq!(after.common_size_presets.len(), 10, "seeded presets must survive an unrelated change");
        set_test_path(None);
        fs::remove_dir_all(dir).unwrap_or(());
    }
}
