//! Phase 6, group 1: the orientation mark (V) and reference marks.

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
fn a_new_part_has_no_marks() {
    let (mut s, insert) = session_with_part();
    let r = s.execute("woodmarks", &json!({ "handles": [insert] })).unwrap();
    assert!(r["orientationMark"].is_null());
    assert_eq!(r["referenceMarks"].as_array().unwrap().len(), 0);
}

#[test]
fn setting_the_orientation_mark_round_trips_and_draws_one_entity() {
    let (mut s, insert) = session_with_part();
    s.execute("woodorientationmark", &json!({ "handles": [insert.clone()], "face": "A", "edge": "Top", "at": [1.0, 4.0] })).unwrap();
    let r = s.execute("woodmarks", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["orientationMark"], json!({ "face": "A", "edge": "Top" }));
}

#[test]
fn setting_the_orientation_mark_twice_replaces_the_drawn_mark_not_doubles_it() {
    let (mut s, insert) = session_with_part();
    s.execute("woodorientationmark", &json!({ "handles": [insert.clone()], "face": "A", "edge": "Top", "at": [1.0, 4.0] })).unwrap();
    s.execute("woodorientationmark", &json!({ "handles": [insert.clone()], "face": "B", "edge": "Left", "at": [0.0, 1.0] })).unwrap();
    let r = s.execute("woodmarks", &json!({ "handles": [insert.clone()] })).unwrap();
    assert_eq!(r["orientationMark"], json!({ "face": "B", "edge": "Left" }));

    // Exactly one small open 3-vertex polyline (the V) lives in the part's block, never two.
    let guid = part_guid(&mut s, &insert);
    let block = s.doc().unwrap().block(&guid).unwrap();
    let v_marks = block
        .entities
        .iter()
        .filter(|e| matches!(&e.kind, cadcraft_engine::doc::EntityKind::LwPolyline(pl) if !pl.closed && pl.vertices.len() == 3))
        .count();
    assert_eq!(v_marks, 1);
}

#[test]
fn clearing_the_orientation_mark_removes_its_drawn_mark() {
    let (mut s, insert) = session_with_part();
    s.execute("woodorientationmark", &json!({ "handles": [insert.clone()], "face": "A", "edge": "Top", "at": [1.0, 4.0] })).unwrap();
    s.execute("woodorientationmarkclear", &json!({ "handles": [insert.clone()] })).unwrap();
    let r = s.execute("woodmarks", &json!({ "handles": [insert.clone()] })).unwrap();
    assert!(r["orientationMark"].is_null());

    let guid = part_guid(&mut s, &insert);
    let block = s.doc().unwrap().block(&guid).unwrap();
    assert_eq!(block.entities.len(), 1, "only the outline should remain");
}

#[test]
fn reference_marks_add_list_and_delete_by_label() {
    let (mut s, insert) = session_with_part();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "A", "at": [0.0, 0.0], "label": "Datum" })).unwrap();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "A", "at": [10.0, 4.0], "label": "Far corner" })).unwrap();
    let r = s.execute("woodmarks", &json!({ "handles": [insert.clone()] })).unwrap();
    let marks = r["referenceMarks"].as_array().unwrap();
    assert_eq!(marks.len(), 2);
    assert_eq!(marks[0], json!({ "face": "A", "x": 0.0, "y": 0.0, "label": "Datum" }));
    assert_eq!(marks[1], json!({ "face": "A", "x": 10.0, "y": 4.0, "label": "Far corner" }));

    s.execute("woodreferencemarkdelete", &json!({ "handles": [insert.clone()], "label": "Datum" })).unwrap();
    let r = s.execute("woodmarks", &json!({ "handles": [insert.clone()] })).unwrap();
    let marks = r["referenceMarks"].as_array().unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0]["label"], json!("Far corner"));

    assert!(s.execute("woodreferencemarkdelete", &json!({ "handles": [insert], "label": "Datum" })).is_err());
}

#[test]
fn reference_marks_are_copied_with_the_part() {
    let (mut s, insert) = session_with_part();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "A", "at": [0.0, 0.0], "label": "Datum" })).unwrap();
    let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
    let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
    let marks = data["Reference mark"].as_array().unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0]["Label"], json!("Datum"));
}

fn part_guid(s: &mut Session, insert_hex: &str) -> String {
    let h = cadcraft_engine::doc::Handle::parse_hex(insert_hex).unwrap();
    match s.doc().unwrap().entity(h).map(|e| &e.kind) {
        Some(cadcraft_engine::doc::EntityKind::Insert(ins)) => ins.block.clone(),
        _ => panic!("not an insert"),
    }
}
