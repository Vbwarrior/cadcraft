//! Phase 6, group 3: cuts.

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use serde_json::json;

fn session_with_part() -> (Session, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [10, 4] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Shelf", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_cuts() {
    let (mut s, insert) = session_with_part();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["cuts"], json!({}));
}

#[test]
fn adding_a_dado_round_trips() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 2], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Dado"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["faceEdge"], json!("A"));
    assert_eq!(entries[0]["depth"], json!(0.25));
    assert!(entries[0]["angle"].is_null());
    assert_eq!(entries[0]["at"], json!([[0.0, 2.0]]));
}

#[test]
fn dado_without_depth_is_refused() {
    let (mut s, insert) = session_with_part();
    assert!(
        s.execute("woodcutadd", &json!({ "handles": [insert], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 2], "length": 3.5, "width": 0.75 }))
            .is_err()
    );
}

#[test]
fn slot_requires_angle_not_depth() {
    let (mut s, insert) = session_with_part();
    assert!(
        s.execute(
            "woodcutadd",
            &json!({ "handles": [insert.clone()], "kind": "Cut-Slot", "faceEdge": "A", "at": [1, 1], "length": 2.0, "width": 0.25 })
        )
        .is_err()
    );
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Slot", "faceEdge": "A", "at": [1, 1], "length": 2.0, "width": 0.25, "angle": 45.0 }),
    )
    .unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Slot"].as_array().unwrap();
    assert_eq!(entries[0]["angle"], json!(45.0));
    assert!(entries[0]["depth"].is_null());
}

#[test]
fn notch_needs_neither_depth_nor_angle() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Notch", "faceEdge": "Top", "at": [0, 4], "length": 1.0, "width": 0.5 }),
    )
    .unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Notch"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0]["depth"].is_null());
    assert!(entries[0]["angle"].is_null());
}

#[test]
fn rabbet_takes_no_position() {
    let (mut s, insert) = session_with_part();
    assert!(s.execute("woodcutadd", &json!({ "handles": [insert.clone()], "kind": "Cut-Rabbet", "faceEdge": "Left", "at": [0, 0], "length": 10.0, "width": 0.25, "depth": 0.25 })).is_err());
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Rabbet", "faceEdge": "Left", "length": 10.0, "width": 0.25, "depth": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Rabbet"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["at"], json!([]));
}

#[test]
fn two_matching_dados_share_one_entry() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 1], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 3], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Dado"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["at"], json!([[0.0, 1.0], [0.0, 3.0]]));
}

#[test]
fn deleting_a_dado_position_leaves_the_other() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 1], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 3], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    s.execute("woodcutdelete", &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "at": [0, 1] })).unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    let entries = r["cuts"]["Cut-Dado"].as_array().unwrap();
    assert_eq!(entries[0]["at"], json!([[0.0, 3.0]]));
}

#[test]
fn deleting_the_rabbet_entry_needs_no_position() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Rabbet", "faceEdge": "Left", "length": 10.0, "width": 0.25, "depth": 0.25 }),
    )
    .unwrap();
    s.execute("woodcutdelete", &json!({ "handles": [insert.clone()], "kind": "Cut-Rabbet" })).unwrap();
    let r = s.execute("woodcuts", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["cuts"], json!({}));
}

#[test]
fn an_unknown_kind_is_refused() {
    let (mut s, insert) = session_with_part();
    assert!(s.execute("woodcutadd", &json!({ "handles": [insert], "kind": "Cut-Wrong", "faceEdge": "A", "length": 1.0, "width": 1.0 })).is_err());
}

#[test]
fn the_drawn_mark_is_never_mistaken_for_the_outline() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 2], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["edges"].as_array().unwrap().len(), 4);
}

#[test]
fn cuts_are_copied_with_the_part() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodcutadd",
        &json!({ "handles": [insert.clone()], "kind": "Cut-Dado", "faceEdge": "A", "at": [0, 2], "length": 3.5, "width": 0.75, "depth": 0.25 }),
    )
    .unwrap();
    let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
    let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
    assert_eq!(data["Cut-Dado"].as_array().unwrap().len(), 1);
}
