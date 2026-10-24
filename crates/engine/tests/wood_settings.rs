//! Phase 4: the WoodCraft settings file, Common size presets, and the managed lists. Each test
//! points `cadcraft_wood::settings_store` at its own temporary file (a thread-local override;
//! `cargo test` gives every test its own thread, so this never crosses tests).

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A session with its own, empty settings file for the duration of the closure.
fn with_settings<R>(f: impl FnOnce(&mut Session) -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-settings-test-{}-{n}", std::process::id()));
    set_test_path(Some(dir.join("settings.json")));
    let mut s = Session::empty();
    let r = f(&mut s);
    set_test_path(None);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

#[test]
fn woodsettingsget_returns_the_seeded_defaults_on_first_use() {
    with_settings(|s| {
        let r = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(r["straightnessTolerance"], json!(0.0));
        assert_eq!(r["units"], json!("in"));
        assert_eq!(r["blockExportOnFastenerWarnings"], json!(false));
        assert_eq!(r["gitTimerMinutes"], json!(1.0));
        assert_eq!(r["buildSizeLimits"], json!({ "smallest": null, "middle": null, "largest": null }));
        assert_eq!(r["commonSizePresets"].as_array().unwrap().len(), 10);
        assert_eq!(r["commonSizePresets"][0], json!({ "name": "1x2", "width": 1.5, "thickness": 0.75 }));
        assert_eq!(r["lists"]["Joint type"].as_array().unwrap().len(), 10);
        assert_eq!(r["lists"]["Edge profile"].as_array().unwrap()[0], json!("Round-over"));
    });
}

#[test]
fn woodsettingsset_changes_only_the_fields_given() {
    with_settings(|s| {
        s.execute("woodsettingsset", &json!({ "units": "mm" })).unwrap();
        let r = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(r["units"], json!("mm"));
        assert_eq!(r["straightnessTolerance"], json!(0.0), "an untouched field must keep its default");
        assert_eq!(r["commonSizePresets"].as_array().unwrap().len(), 10, "an unrelated change must not disturb the seeded presets");
    });
}

#[test]
fn woodsettingsset_refuses_a_bad_units_value_and_changes_nothing() {
    with_settings(|s| {
        let before = s.execute("woodsettingsget", &json!({})).unwrap();
        assert!(s.execute("woodsettingsset", &json!({ "units": "cm" })).is_err());
        let after = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn woodsettingsset_fills_build_size_limits_one_at_a_time() {
    with_settings(|s| {
        s.execute("woodsettingsset", &json!({ "buildSizeLimits": { "smallest": 1.0 } })).unwrap();
        s.execute("woodsettingsset", &json!({ "buildSizeLimits": { "middle": 2.0 } })).unwrap();
        let mid = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(mid["buildSizeLimits"], json!({ "smallest": 1.0, "middle": 2.0, "largest": null }));
        s.execute("woodsettingsset", &json!({ "buildSizeLimits": { "largest": 3.0 } })).unwrap();
        let full = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(full["buildSizeLimits"], json!({ "smallest": 1.0, "middle": 2.0, "largest": 3.0 }));
    });
}

#[test]
fn woodsizeadd_rename_delete_round_trip() {
    with_settings(|s| {
        let added = s.execute("woodsizeadd", &json!({ "name": "Scrap", "width": 2.0, "thickness": 0.5 })).unwrap();
        let presets = added["commonSizePresets"].as_array().unwrap();
        assert_eq!(presets.len(), 11);
        assert!(presets.iter().any(|p| p["name"] == json!("Scrap")));

        assert!(s.execute("woodsizeadd", &json!({ "name": "Scrap", "width": 1.0, "thickness": 1.0 })).is_err(), "a duplicate name must be refused");

        let renamed = s.execute("woodsizerename", &json!({ "name": "Scrap", "newName": "Offcut" })).unwrap();
        let presets = renamed["commonSizePresets"].as_array().unwrap();
        assert!(presets.iter().any(|p| p["name"] == json!("Offcut")));
        assert!(!presets.iter().any(|p| p["name"] == json!("Scrap")));

        let deleted = s.execute("woodsizedelete", &json!({ "name": "Offcut" })).unwrap();
        assert_eq!(deleted["commonSizePresets"].as_array().unwrap().len(), 10);
        assert!(s.execute("woodsizedelete", &json!({ "name": "Offcut" })).is_err(), "deleting a missing preset must be refused");
    });
}

#[test]
fn woodsizerename_refuses_a_name_already_in_use_and_changes_nothing() {
    with_settings(|s| {
        let before = s.execute("woodsettingsget", &json!({})).unwrap();
        assert!(s.execute("woodsizerename", &json!({ "name": "1x2", "newName": "1x4" })).is_err());
        let after = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn woodlistadd_rename_delete_round_trip() {
    with_settings(|s| {
        let added = s.execute("woodlistadd", &json!({ "list": "Glue type", "entry": "Hide glue" })).unwrap();
        let glue = added["lists"]["Glue type"].as_array().unwrap();
        assert_eq!(glue.len(), 4);
        assert!(glue.contains(&json!("Hide glue")));

        assert!(s.execute("woodlistadd", &json!({ "list": "Glue type", "entry": "Hide glue" })).is_err(), "a duplicate entry must be refused");

        let renamed = s.execute("woodlistrename", &json!({ "list": "Glue type", "entry": "Hide glue", "newEntry": "Hot hide glue" })).unwrap();
        let glue = renamed["lists"]["Glue type"].as_array().unwrap();
        assert!(glue.contains(&json!("Hot hide glue")));
        assert!(!glue.contains(&json!("Hide glue")));

        let deleted = s.execute("woodlistdelete", &json!({ "list": "Glue type", "entry": "Hot hide glue" })).unwrap();
        assert_eq!(deleted["lists"]["Glue type"].as_array().unwrap().len(), 3);
    });
}

#[test]
fn woodlistadd_refuses_an_unknown_list_name() {
    with_settings(|s| {
        assert!(s.execute("woodlistadd", &json!({ "list": "Wood type", "entry": "Oak" })).is_err());
    });
}

#[test]
fn woodlistdelete_refuses_a_missing_entry_and_changes_nothing() {
    with_settings(|s| {
        let before = s.execute("woodsettingsget", &json!({})).unwrap();
        assert!(s.execute("woodlistdelete", &json!({ "list": "Joint type", "entry": "Nonexistent" })).is_err());
        let after = s.execute("woodsettingsget", &json!({})).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn settings_commands_need_no_open_drawing() {
    with_settings(|_| {
        let mut s = Session::empty();
        assert!(s.execute("woodsettingsget", &json!({})).is_ok());
        assert!(s.execute("woodsizeadd", &json!({ "name": "Scrap", "width": 1.0, "thickness": 1.0 })).is_ok());
    });
}
