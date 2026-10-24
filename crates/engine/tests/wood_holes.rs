//! Phase 6, group 2: holes.

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use serde_json::json;

fn session_with_part() -> (Session, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [10, 4] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Leg", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_holes() {
    let (mut s, insert) = session_with_part();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["holes"], json!({}));
}

#[test]
fn adding_a_through_hole_round_trips() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    let entries = r["holes"]["Hole-Through"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["face"], json!("A"));
    assert_eq!(entries[0]["dia"], json!(0.25));
    assert!(entries[0]["depth"].is_null());
    assert_eq!(entries[0]["at"], json!([[1.5, 2.0]]));
}

#[test]
fn two_holes_with_the_same_values_share_one_entry_with_two_positions() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 4.5], "dia": 0.25 })).unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    let entries = r["holes"]["Hole-Through"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "same face/dia/depth must collapse into one entry");
    assert_eq!(entries[0]["at"], json!([[1.5, 2.0], [1.5, 4.5]]));
}

#[test]
fn a_different_diameter_makes_a_separate_entry() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 4.5], "dia": 0.375 })).unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    let entries = r["holes"]["Hole-Through"].as_array().unwrap();
    assert_eq!(entries.len(), 2, "a different dia must not collapse");
}

#[test]
fn counterbore_keeps_its_extra_fields() {
    let (mut s, insert) = session_with_part();
    s.execute(
        "woodholeadd",
        &json!({ "handles": [insert.clone()], "kind": "Hole-Counterbore", "face": "A", "at": [2.0, 2.0], "dia": 0.25, "depth": 1.0, "extra1": 0.5, "extra2": 0.25 }),
    )
    .unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert], "kind": "Hole-Counterbore" })).unwrap();
    let entries = r["holes"]["Hole-Counterbore"].as_array().unwrap();
    assert_eq!(entries[0]["depth"], json!(1.0));
    assert_eq!(entries[0]["extra1"], json!(0.5));
    assert_eq!(entries[0]["extra2"], json!(0.25));
}

#[test]
fn deleting_one_of_two_positions_leaves_the_entry_with_one_left() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 4.5], "dia": 0.25 })).unwrap();
    s.execute("woodholedelete", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "at": [1.5, 2.0] })).unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    let entries = r["holes"]["Hole-Through"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["at"], json!([[1.5, 4.5]]));
}

#[test]
fn deleting_the_last_position_removes_the_whole_entry() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    s.execute("woodholedelete", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "at": [1.5, 2.0] })).unwrap();
    let r = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["holes"], json!({}));
}

#[test]
fn deleting_a_position_that_is_not_there_is_refused() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    assert!(s.execute("woodholedelete", &json!({ "handles": [insert], "kind": "Hole-Through", "at": [9.0, 9.0] })).is_err());
}

#[test]
fn an_unknown_kind_is_refused() {
    let (mut s, insert) = session_with_part();
    assert!(s.execute("woodholeadd", &json!({ "handles": [insert], "kind": "Hole-Wrong", "face": "A", "at": [1, 1], "dia": 0.25 })).is_err());
}

#[test]
fn the_drawn_mark_is_never_mistaken_for_the_outline() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["edges"].as_array().unwrap().len(), 4, "the hole mark must not be read as the outline");
}

#[test]
fn holes_are_copied_with_the_part() {
    let (mut s, insert) = session_with_part();
    s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Through", "face": "A", "at": [1.5, 2.0], "dia": 0.25 })).unwrap();
    let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
    let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
    let entries = data["Hole-Through"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
}
