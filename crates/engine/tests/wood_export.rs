//! Phase 8: Export. `project.json` plus one SCAD file per part under `models/`.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-export-test-{}-{n}", std::process::id()));
    set_test_path(Some(dir.join("settings.json")));
    let r = f();
    set_test_path(None);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

fn export_dir() -> std::path::PathBuf {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("woodcraft-export-test-{}-{n}", std::process::id()))
}

fn session_with_part(w: f64, h: f64, thickness: f64) -> (Session, String, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [w, h] })).unwrap();
    let handle = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [handle], "name": "Shelf", "thickness": thickness })).unwrap();
    let guid = r["part"].as_str().unwrap().to_string();
    let insert = r["insert"].as_str().unwrap().to_string();
    (s, guid, insert)
}

#[test]
fn exporting_writes_project_json_and_one_scad_per_part() {
    with_settings(|| {
        let (mut s, guid, _insert) = session_with_part(10.0, 4.0, 0.75);
        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "Hall table" })).unwrap();
        assert_eq!(r["parts"], json!(1));
        assert!(r.get("blocked").is_none());

        let project_text = std::fs::read_to_string(dir.join("project.json")).unwrap();
        let project: serde_json::Value = serde_json::from_str(&project_text).unwrap();
        assert_eq!(project["project"]["name"], "Hall table");
        assert_eq!(project["parts"].as_array().unwrap().len(), 1);
        assert_eq!(project["parts"][0]["id"], guid);
        assert_eq!(project["parts"][0]["name"], "Shelf");
        assert_eq!(project["parts"][0]["instances"], 1);
        assert_eq!(project["parts"][0]["model"], format!("models/{guid}.scad"));
        assert!(project["parts"][0]["properties"].get("name").is_none(), "name belongs alongside properties, not inside them");

        let scad = std::fs::read_to_string(dir.join("models").join(format!("{guid}.scad"))).unwrap();
        assert!(scad.contains("difference()"));
        assert!(scad.contains("polygon"));

        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn the_outline_is_the_part_s_rectangle_in_order() {
    with_settings(|| {
        let (mut s, _guid, _insert) = session_with_part(10.0, 4.0, 0.75);
        let dir = export_dir();
        s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        let project: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).unwrap()).unwrap();
        let outline = project["parts"][0]["outline"].as_array().unwrap();
        assert_eq!(outline.len(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn a_copy_array_is_exported_once_with_the_right_instance_count() {
    with_settings(|| {
        let (mut s, guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodcopyarray", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
        s.execute("woodcopyarray", &json!({ "handles": [insert], "to": [40, 0] })).unwrap();

        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r["parts"], json!(1), "three instances of one GUID is still one part entry");

        let project: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(project["parts"][0]["id"], guid);
        assert_eq!(project["parts"][0]["instances"], 3);
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn export_is_blocked_while_an_unacknowledged_fastener_warning_exists_and_the_setting_is_on() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodsettingsset", &json!({ "blockExportOnFastenerWarnings": true })).unwrap();
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();

        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r["parts"], json!(0));
        let blocked = r["blocked"].as_array().unwrap();
        assert_eq!(blocked.len(), 1);
        assert!(!dir.join("project.json").exists(), "nothing should be written while export is blocked");
    });
}

#[test]
fn acknowledging_the_warning_lets_a_blocked_export_through_once() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodsettingsset", &json!({ "blockExportOnFastenerWarnings": true })).unwrap();
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();
        let id = s.execute("woodwarnings", &json!({})).unwrap()["warnings"][0]["id"].as_str().unwrap().to_string();
        s.execute("woodwarningacknowledge", &json!({ "id": id })).unwrap();

        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r["parts"], json!(1), "an acknowledged warning must not block export");
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn the_override_lasts_for_one_export_only() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodsettingsset", &json!({ "blockExportOnFastenerWarnings": true })).unwrap();
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();
        let id = s.execute("woodwarnings", &json!({})).unwrap()["warnings"][0]["id"].as_str().unwrap().to_string();
        s.execute("woodwarningacknowledge", &json!({ "id": id })).unwrap();

        let dir1 = export_dir();
        let r1 = s.execute("woodexport", &json!({ "path": dir1.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r1["parts"], json!(1));
        let _ = std::fs::remove_dir_all(&dir1);

        let dir2 = export_dir();
        let r2 = s.execute("woodexport", &json!({ "path": dir2.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r2["parts"], json!(0), "the acknowledge must not carry over to the next export");
        assert_eq!(r2["blocked"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn the_setting_being_off_never_blocks_export() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();

        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r["parts"], json!(1));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn joint_fit_and_build_size_warnings_never_block_export() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodsettingsset", &json!({ "blockExportOnFastenerWarnings": true, "minimumJointSpacing": 0.01 })).unwrap();
        s.execute(
            "woodjoineryadd",
            &json!({ "handles": [insert], "kind": "Mortise", "faceEdge": "Left", "at": [0, 2], "length": 1.5, "width": 0.375, "depth": 0.5, "tolerance": 0.001 }),
        )
        .unwrap();

        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        assert_eq!(r["parts"], json!(1), "only fastener warnings are gated by the setting");
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn a_hole_and_cut_are_present_in_the_exported_scad() {
    with_settings(|| {
        let (mut s, guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1, 1], "dia": 0.25 })).unwrap();
        s.execute(
            "woodcutadd",
            &json!({ "handles": [insert], "kind": "Cut-Dado", "faceEdge": "Left", "at": [0, 2], "length": 3.5, "width": 0.75, "depth": 0.25 }),
        )
        .unwrap();

        let dir = export_dir();
        s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "P" })).unwrap();
        let scad = std::fs::read_to_string(dir.join("models").join(format!("{guid}.scad"))).unwrap();
        assert!(scad.contains("circle(d = 0.250000)"));
        assert!(scad.contains("cube([3.500000, 0.750000, 0.250000])"));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn exporting_an_empty_project_writes_an_empty_parts_array() {
    with_settings(|| {
        let mut s = Session::empty();
        s.open_drawing(Drawing::new_imperial(), "wood", None);
        let dir = export_dir();
        let r = s.execute("woodexport", &json!({ "path": dir.to_string_lossy(), "projectName": "Empty" })).unwrap();
        assert_eq!(r["parts"], json!(0));
        let project: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(project["parts"], json!([]));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn missing_path_and_project_name_are_refused() {
    with_settings(|| {
        let (mut s, ..) = session_with_part(10.0, 4.0, 0.75);
        assert!(s.execute("woodexport", &json!({ "projectName": "P" })).is_err());
        assert!(s.execute("woodexport", &json!({ "path": "/tmp/whatever" })).is_err());
    });
}
