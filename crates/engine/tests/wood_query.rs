//! Phase 2: reading a part's properties, listing parts, the display-name substitution, and the
//! commands that change Thickness, Grain direction and a part's display name.

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::store;
use serde_json::json;

fn session_with_part(name: &str, thickness: f64) -> (Session, String, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [3.5, 38] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": name, "thickness": thickness })).unwrap();
    let guid = r["part"].as_str().unwrap().to_string();
    let insert = r["insert"].as_str().unwrap().to_string();
    (s, guid, insert)
}

#[test]
fn woodparts_lists_every_part() {
    let (mut s, guid, _) = session_with_part("Leg", 1.5);
    let r = s.execute("woodparts", &json!({})).unwrap();
    let parts = r["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["guid"], json!(guid));
    assert_eq!(parts[0]["name"], json!("Leg"));
    assert_eq!(parts[0]["thickness"], json!(1.5));
    assert_eq!(parts[0]["revision"], json!(1));
}

#[test]
fn woodparts_is_empty_for_a_plain_drawing() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let r = s.execute("woodparts", &json!({})).unwrap();
    assert_eq!(r["parts"].as_array().unwrap().len(), 0);
}

#[test]
fn woodpartinfo_returns_the_selected_parts_full_properties() {
    let (mut s, _guid, insert) = session_with_part("Leg", 1.5);
    let r = s.execute("woodpartinfo", &json!({ "handle": insert })).unwrap();
    assert_eq!(r["name"], json!("Leg"));
    assert_eq!(r["Thickness"], json!(1.5));
}

#[test]
fn woodpartinfo_uses_the_current_selection_when_no_handle_is_given() {
    let (mut s, _guid, insert) = session_with_part("Leg", 1.5);
    s.set_selection(vec![cadcraft_doc::Handle::parse_hex(&insert).unwrap()]);
    let r = s.execute("woodpartinfo", &json!({})).unwrap();
    assert_eq!(r["name"], json!("Leg"));
}

#[test]
fn woodpartinfo_refuses_a_non_wood_block_and_no_selection() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    assert!(s.execute("woodpartinfo", &json!({})).is_err(), "nothing selected");

    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [1, 1] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let err = s.execute("woodpartinfo", &json!({ "handle": h })).unwrap_err().to_string();
    assert!(err.contains("block reference"), "{err}");
}

#[test]
fn wooddisplayname_substitutes_for_a_part_and_passes_through_otherwise() {
    let (mut s, guid, _) = session_with_part("Leg", 1.5);
    let r = s.execute("wooddisplayname", &json!({ "block": guid })).unwrap();
    assert_eq!(r["name"], json!("Leg"));

    let r = s.execute("wooddisplayname", &json!({ "block": "SomeOtherBlock" })).unwrap();
    assert_eq!(r["name"], json!("SomeOtherBlock"), "a non-wood block name passes through unchanged");
}

#[test]
fn woodthickness_changes_thickness_and_is_undoable() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodthickness", &json!({ "handle": insert, "thickness": 0.75 })).unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].thickness, 0.75);
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].revision, 2);

    s.undo().unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].thickness, 1.5);
    s.redo().unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].thickness, 0.75);
}

#[test]
fn woodthickness_refuses_bad_values_and_changes_nothing() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    assert!(s.execute("woodthickness", &json!({ "handle": insert.clone(), "thickness": -1.0 })).is_err());
    assert!(s.execute("woodthickness", &json!({ "handle": insert.clone(), "thickness": 0 })).is_err());
    assert!(s.execute("woodthickness", &json!({ "handle": insert })).is_err(), "missing thickness");
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].thickness, 1.5);
}

#[test]
fn woodgrain_sets_and_clears_grain_direction() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodgrain", &json!({ "handle": insert.clone(), "grainDirection": 90.0 })).unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].grain_direction, Some(90.0));

    s.execute("woodgrain", &json!({ "handle": insert, "grainDirection": null })).unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].grain_direction, None);
}

#[test]
fn woodname_renames_the_display_name_without_touching_the_block_name() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodname", &json!({ "handle": insert, "name": "Leg – Front Left" })).unwrap();
    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts[&guid].name, "Leg – Front Left");
    assert!(s.doc().unwrap().block(&guid).is_some(), "the block (GUID) name itself is unchanged");
}

#[test]
fn woodname_refuses_an_empty_name_and_changes_nothing() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    assert!(s.execute("woodname", &json!({ "handle": insert, "name": "   " })).is_err());
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts[&guid].name, "Leg");
}
