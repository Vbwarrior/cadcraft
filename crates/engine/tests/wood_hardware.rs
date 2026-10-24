//! Phase 6, group 6 (part 2): hardware markers.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-hardware-test-{}-{n}", std::process::id()));
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
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Door", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_hardware() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        let r = s.execute("woodhardware", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["hardware"], json!({}));
    });
}

#[test]
fn hardware_type_must_be_in_the_settings_list() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(
            s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Not real", "at": [1, 1] }))
                .is_err()
        );
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [1, 1] })).unwrap();
        let r = s.execute("woodhardware", &json!({ "handles": [insert] })).unwrap();
        let entries = r["hardware"]["Hinge"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0]["size"].is_null());
    });
}

#[test]
fn drawer_slide_requires_a_size_hinge_refuses_one() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(
            s.execute(
                "woodhardwareadd",
                &json!({ "handles": [insert.clone()], "kind": "Drawer slide", "face": "A", "type": "Drawer slide", "at": [1, 1] })
            )
            .is_err()
        );
        s.execute(
            "woodhardwareadd",
            &json!({ "handles": [insert.clone()], "kind": "Drawer slide", "face": "A", "type": "Drawer slide", "at": [1, 1], "size": 18.0 }),
        )
        .unwrap();
        assert!(
            s.execute(
                "woodhardwareadd",
                &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [2, 2], "size": 1.0 })
            )
            .is_err()
        );
        let r = s.execute("woodhardware", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["hardware"]["Drawer slide"].as_array().unwrap()[0]["size"], json!(18.0));
    });
}

#[test]
fn two_matching_hinges_share_one_entry() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [0, 1] })).unwrap();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [0, 3] })).unwrap();
        let r = s.execute("woodhardware", &json!({ "handles": [insert] })).unwrap();
        let entries = r["hardware"]["Hinge"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["at"], json!([[0.0, 1.0], [0.0, 3.0]]));
    });
}

#[test]
fn deleting_a_hardware_position_leaves_the_other() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [0, 1] })).unwrap();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Hinge", "face": "A", "type": "Hinge", "at": [0, 3] })).unwrap();
        s.execute("woodhardwaredelete", &json!({ "handles": [insert.clone()], "kind": "Hinge", "at": [0, 1] })).unwrap();
        let r = s.execute("woodhardware", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["hardware"]["Hinge"].as_array().unwrap()[0]["at"], json!([[0.0, 3.0]]));
    });
}

#[test]
fn the_hardware_mark_is_never_mistaken_for_the_outline() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Drawer Pull", "face": "A", "type": "Knob", "at": [5, 2] }))
            .unwrap();
        let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["edges"].as_array().unwrap().len(), 4);
    });
}

#[test]
fn an_unknown_kind_is_refused() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(s.execute("woodhardwareadd", &json!({ "handles": [insert], "kind": "Latch", "face": "A", "type": "Latch", "at": [1, 1] })).is_err());
    });
}

#[test]
fn hardware_is_copied_with_the_part() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodhardwareadd", &json!({ "handles": [insert.clone()], "kind": "Drawer Pull", "face": "A", "type": "Knob", "at": [5, 2] }))
            .unwrap();
        let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
        let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
        assert_eq!(data["Drawer Pull"].as_array().unwrap().len(), 1);
    });
}
