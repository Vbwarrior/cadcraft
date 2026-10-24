//! Phase 6, group 4: joinery (Mortise, Tenon, Biscuit, Joint type) and End treatments.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A session whose settings file is its own temporary file for the duration of the closure, so
/// the seeded End treatment list is isolated from other tests.
fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-joints-test-{}-{n}", std::process::id()));
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
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Rail", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_joinery() {
    let (mut s, insert) = session_with_part();
    let r = s.execute("woodjoinery", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["joinery"], json!({}));
}

#[test]
fn adding_a_mortise_round_trips() {
    let (mut s, insert) = session_with_part();
    s.execute("woodjoineryadd", &json!({ "handles": [insert.clone()], "kind": "Mortise", "faceEdge": "Left", "at": [0, 2], "length": 1.5, "width": 0.375, "depth": 0.75, "tolerance": 0.005 })).unwrap();
    let r = s.execute("woodjoinery", &json!({ "handles": [insert] })).unwrap();
    let entries = r["joinery"]["Mortise"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["depth"], json!(0.75));
    assert_eq!(entries[0]["tolerance"], json!(0.005));
}

#[test]
fn biscuit_refuses_depth_and_tolerance() {
    let (mut s, insert) = session_with_part();
    assert!(
        s.execute(
            "woodjoineryadd",
            &json!({ "handles": [insert.clone()], "kind": "Biscuit", "faceEdge": "Left", "at": [0, 2], "length": 2.0, "width": 0.25, "depth": 0.5 })
        )
        .is_err()
    );
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Biscuit", "faceEdge": "Left", "at": [0, 2], "length": 2.0, "width": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodjoinery", &json!({ "handles": [insert] })).unwrap();
    let entries = r["joinery"]["Biscuit"].as_array().unwrap();
    assert!(entries[0]["depth"].is_null());
    assert!(entries[0]["tolerance"].is_null());
}

#[test]
fn two_matching_tenons_share_one_entry() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Tenon", "faceEdge": "Left", "at": [0, 1], "length": 1.5, "width": 0.375, "depth": 0.75 }),
    )
    .unwrap();
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Tenon", "faceEdge": "Left", "at": [0, 3], "length": 1.5, "width": 0.375, "depth": 0.75 }),
    )
    .unwrap();
    let r = s.execute("woodjoinery", &json!({ "handles": [insert] })).unwrap();
    let entries = r["joinery"]["Tenon"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["at"], json!([[0.0, 1.0], [0.0, 3.0]]));
}

#[test]
fn deleting_a_joinery_position_leaves_the_other() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Tenon", "faceEdge": "Left", "at": [0, 1], "length": 1.5, "width": 0.375, "depth": 0.75 }),
    )
    .unwrap();
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Tenon", "faceEdge": "Left", "at": [0, 3], "length": 1.5, "width": 0.375, "depth": 0.75 }),
    )
    .unwrap();
    s.execute("woodjoinerydelete", &json!({ "handles": [insert.clone()], "kind": "Tenon", "at": [0, 1] })).unwrap();
    let r = s.execute("woodjoinery", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["joinery"]["Tenon"].as_array().unwrap()[0]["at"], json!([[0.0, 3.0]]));
}

#[test]
fn the_joinery_mark_is_never_mistaken_for_the_outline() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodjoineryadd",
        &json!({ "handles": [insert.clone()], "kind": "Mortise", "faceEdge": "Left", "at": [0, 2], "length": 1.5, "width": 0.375, "depth": 0.75 }),
    )
    .unwrap();
    let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["edges"].as_array().unwrap().len(), 4);
}

#[test]
fn joint_type_set_and_clear_round_trips() {
    let (mut s, insert) = session_with_part();
    let info = s.execute("woodjointinfo", &json!({ "handles": [insert.clone()] })).unwrap();
    assert!(info["jointType"].is_null());

    s.execute("woodjointtypeset", &json!({ "handles": [insert.clone()], "jointCode": "Tenon", "matingPartId": "leg-guid", "tolerance": 0.01 }))
        .unwrap();
    let info = s.execute("woodjointinfo", &json!({ "handles": [insert.clone()] })).unwrap();
    assert_eq!(info["jointType"], json!({ "jointCode": "Tenon", "matingPartId": "leg-guid", "tolerance": 0.01 }));

    // Setting again replaces, never adds a second entry.
    s.execute("woodjointtypeset", &json!({ "handles": [insert.clone()], "jointCode": "Dado", "matingPartId": "shelf-guid" })).unwrap();
    let info = s.execute("woodjointinfo", &json!({ "handles": [insert.clone()] })).unwrap();
    assert_eq!(info["jointType"]["jointCode"], json!("Dado"));

    s.execute("woodjointtypeclear", &json!({ "handles": [insert.clone()] })).unwrap();
    let info = s.execute("woodjointinfo", &json!({ "handles": [insert] })).unwrap();
    assert!(info["jointType"].is_null());
}

#[test]
fn end_treatment_must_be_in_the_settings_list() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(
            s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End A", "profile": "Not a real treatment" })).is_err()
        );
        s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End A", "profile": "Tenon" })).unwrap();
        let info = s.execute("woodjointinfo", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(info["endTreatments"], json!([{ "edgeEnd": "End A", "profile": "Tenon" }]));
    });
}

#[test]
fn setting_an_end_treatment_twice_for_the_same_edge_replaces_it() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End A", "profile": "Tenon" })).unwrap();
        s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End A", "profile": "Dovetail" })).unwrap();
        let info = s.execute("woodjointinfo", &json!({ "handles": [insert] })).unwrap();
        let treatments = info["endTreatments"].as_array().unwrap();
        assert_eq!(treatments.len(), 1);
        assert_eq!(treatments[0]["profile"], json!("Dovetail"));
    });
}

#[test]
fn end_a_and_end_b_are_independent() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End A", "profile": "Tenon" })).unwrap();
        s.execute("woodendtreatmentset", &json!({ "handles": [insert.clone()], "edgeEnd": "End B", "profile": "Mitered" })).unwrap();
        let info = s.execute("woodjointinfo", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(info["endTreatments"].as_array().unwrap().len(), 2);
    });
}

#[test]
fn clearing_an_end_treatment_that_is_not_set_is_refused() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(s.execute("woodendtreatmentclear", &json!({ "handles": [insert], "edgeEnd": "End A" })).is_err());
    });
}

#[test]
fn joinery_and_joint_type_are_copied_with_the_part() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodjoineryadd", &json!({ "handles": [insert.clone()], "kind": "Mortise", "faceEdge": "Left", "at": [0, 2], "length": 1.5, "width": 0.375, "depth": 0.75 })).unwrap();
        s.execute("woodjointtypeset", &json!({ "handles": [insert.clone()], "jointCode": "Tenon", "matingPartId": "leg-guid" })).unwrap();
        let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
        let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
        assert_eq!(data["Mortise"].as_array().unwrap().len(), 1);
        assert_eq!(data["Joint type"].as_array().unwrap().len(), 1);
    });
}
