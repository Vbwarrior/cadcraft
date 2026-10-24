//! Phase 7: the warnings engine.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-warnings-test-{}-{n}", std::process::id()));
    set_test_path(Some(dir.join("settings.json")));
    let r = f();
    set_test_path(None);
    let _ = std::fs::remove_dir_all(&dir);
    r
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
fn a_new_part_has_no_warnings() {
    with_settings(|| {
        let (mut s, ..) = session_with_part(10.0, 4.0, 0.75);
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        assert_eq!(r["warnings"], json!([]));
    });
}

#[test]
fn a_deep_hole_produces_a_fastener_warning() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        let warnings = r["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0]["kind"], json!("Fastener"));
        assert_eq!(warnings[0]["acknowledged"], json!(false));
    });
}

#[test]
fn a_shallow_hole_has_no_warning() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 0.25 }))
            .unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        assert_eq!(r["warnings"], json!([]));
    });
}

#[test]
fn acknowledging_a_warning_marks_it_without_removing_it() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();
        let before = s.execute("woodwarnings", &json!({})).unwrap();
        let id = before["warnings"][0]["id"].as_str().unwrap().to_string();

        s.execute("woodwarningacknowledge", &json!({ "id": id })).unwrap();
        let after = s.execute("woodwarnings", &json!({})).unwrap();
        let warnings = after["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1, "the warning must still be listed, only marked acknowledged");
        assert_eq!(warnings[0]["acknowledged"], json!(true));

        s.execute("woodwarningunacknowledge", &json!({ "id": id })).unwrap();
        let reverted = s.execute("woodwarnings", &json!({})).unwrap();
        assert_eq!(reverted["warnings"][0]["acknowledged"], json!(false));
    });
}

#[test]
fn fixing_the_underlying_problem_removes_the_warning() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 }))
            .unwrap();
        assert_eq!(s.execute("woodwarnings", &json!({})).unwrap()["warnings"].as_array().unwrap().len(), 1);

        s.execute("woodholedelete", &json!({ "handles": [insert], "kind": "Hole-Blind", "at": [1, 1] })).unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        assert_eq!(r["warnings"], json!([]), "a warning must disappear once its cause is fixed");
    });
}

#[test]
fn a_through_hole_never_warns_no_matter_how_thin_the_part_is() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.1);
        s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Through", "face": "A", "at": [1, 1], "dia": 0.25 })).unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        assert_eq!(r["warnings"], json!([]));
    });
}

#[test]
fn a_tight_joint_tolerance_produces_a_joint_fit_warning() {
    with_settings(|| {
        let (mut s, _guid, insert) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodsettingsset", &json!({ "minimumJointSpacing": 0.01 })).unwrap();
        s.execute(
            "woodjoineryadd",
            &json!({ "handles": [insert], "kind": "Mortise", "faceEdge": "Left", "at": [0, 2], "length": 1.5, "width": 0.375, "depth": 0.5, "tolerance": 0.001 }),
        )
        .unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        let warnings = r["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0]["kind"], json!("JointFit"));
    });
}

#[test]
fn build_size_warning_appears_only_once_all_three_limits_are_set() {
    with_settings(|| {
        let (mut s, ..) = session_with_part(100.0, 4.0, 0.75);
        assert_eq!(s.execute("woodwarnings", &json!({})).unwrap()["warnings"], json!([]));

        s.execute("woodsettingsset", &json!({ "buildSizeLimits": { "smallest": 1.0, "middle": 2.0, "largest": 3.0 } })).unwrap();
        let r = s.execute("woodwarnings", &json!({})).unwrap();
        let warnings = r["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w["kind"] == json!("BuildSize")), "the oversized part should warn once all three limits are set");
    });
}

#[test]
fn warnings_from_different_parts_are_reported_separately() {
    with_settings(|| {
        let (mut s, _g1, i1) = session_with_part(10.0, 4.0, 0.75);
        s.execute("woodholeadd", &json!({ "handles": [i1], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 })).unwrap();
        s.execute("rectang", &json!({ "p1": [20, 0], "p2": [30, 4] })).unwrap();
        let h2 = s.doc().unwrap().model.iter().find(|e| e.handle.hex() != i1).map(|e| e.handle).unwrap().hex();
        let r2 = s.execute("woodpart", &json!({ "handles": [h2], "name": "Other", "thickness": 0.75 })).unwrap();
        let i2 = r2["insert"].as_str().unwrap().to_string();
        s.execute("woodholeadd", &json!({ "handles": [i2], "kind": "Hole-Blind", "face": "A", "at": [1, 1], "dia": 0.25, "depth": 1.0 })).unwrap();

        let r = s.execute("woodwarnings", &json!({})).unwrap();
        let warnings = r["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 2);
        let guids: std::collections::HashSet<_> = warnings.iter().map(|w| w["partGuid"].as_str().unwrap()).collect();
        assert_eq!(guids.len(), 2, "each part's warning must carry its own part GUID");
    });
}
