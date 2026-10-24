//! Phase 6, group 5: edge profile, bevel and miter.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-edgeprofile-test-{}-{n}", std::process::id()));
    set_test_path(Some(dir.join("settings.json")));
    let r = f();
    set_test_path(None);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

fn session_with_part() -> (Session, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [10, 4] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Top", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_edge_properties() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["edgeProfiles"], json!([]));
        assert_eq!(r["bevels"], json!([]));
        assert_eq!(r["miters"], json!([]));
    });
}

#[test]
fn edge_profile_must_be_in_the_settings_list() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(
            s.execute("woodedgeprofileset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "profile": "Not a real profile", "size": 0.75 }))
                .is_err()
        );
        s.execute("woodedgeprofileset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "profile": "Round-over", "size": 0.75 })).unwrap();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["edgeProfiles"], json!([{ "edgeEnd": "Top", "profile": "Round-over", "size": 0.75 }]));
    });
}

#[test]
fn setting_an_edge_profile_twice_for_the_same_edge_replaces_it() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodedgeprofileset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "profile": "Round-over", "size": 0.75 })).unwrap();
        s.execute("woodedgeprofileset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "profile": "Chamfer", "size": 0.5 })).unwrap();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        let profiles = r["edgeProfiles"].as_array().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0]["profile"], json!("Chamfer"));
    });
}

#[test]
fn clearing_an_edge_profile_that_is_not_set_is_refused() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(s.execute("woodedgeprofileclear", &json!({ "handles": [insert], "edgeEnd": "Top" })).is_err());
    });
}

#[test]
fn bevel_and_miter_round_trip_independently() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodbevelset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "angle": 15.0 })).unwrap();
        s.execute("woodmiterset", &json!({ "handles": [insert.clone()], "edgeEnd": "Right", "angle": 45.0 })).unwrap();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["bevels"], json!([{ "edgeEnd": "Top", "angle": 15.0 }]));
        assert_eq!(r["miters"], json!([{ "edgeEnd": "Right", "angle": 45.0 }]));
    });
}

#[test]
fn setting_a_bevel_twice_replaces_it() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodbevelset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "angle": 15.0 })).unwrap();
        s.execute("woodbevelset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "angle": 30.0 })).unwrap();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        let bevels = r["bevels"].as_array().unwrap();
        assert_eq!(bevels.len(), 1);
        assert_eq!(bevels[0]["angle"], json!(30.0));
    });
}

#[test]
fn clearing_a_bevel_that_is_not_set_is_refused() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(s.execute("woodbevelclear", &json!({ "handles": [insert], "edgeEnd": "Top" })).is_err());
    });
}

#[test]
fn clearing_a_miter_removes_only_that_edge() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodmiterset", &json!({ "handles": [insert.clone()], "edgeEnd": "Right", "angle": 45.0 })).unwrap();
        s.execute("woodmiterset", &json!({ "handles": [insert.clone()], "edgeEnd": "Left", "angle": 45.0 })).unwrap();
        s.execute("woodmiterclear", &json!({ "handles": [insert.clone()], "edgeEnd": "Right" })).unwrap();
        let r = s.execute("woodedgeinfo", &json!({ "handles": [insert] })).unwrap();
        let miters = r["miters"].as_array().unwrap();
        assert_eq!(miters.len(), 1);
        assert_eq!(miters[0]["edgeEnd"], json!("Left"));
    });
}

#[test]
fn edge_properties_are_copied_with_the_part() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodedgeprofileset", &json!({ "handles": [insert.clone()], "edgeEnd": "Top", "profile": "Round-over", "size": 0.75 })).unwrap();
        s.execute("woodbevelset", &json!({ "handles": [insert.clone()], "edgeEnd": "Left", "angle": 10.0 })).unwrap();
        let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
        let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
        assert_eq!(data["Edge profile"].as_array().unwrap().len(), 1);
        assert_eq!(data["Bevel"].as_array().unwrap().len(), 1);
    });
}
