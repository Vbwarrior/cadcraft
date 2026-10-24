//! MAKE WOOD PART changes the geometry and the part JSON together, and undo and redo carry both.

use cadcraft_engine::Session;
use cadcraft_engine::doc::{Drawing, EntityKind};
use cadcraft_wood::store;
use serde_json::json;

fn session_with_rectangle() -> (Session, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [3.5, 38] })).unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap();
    (s, h.hex())
}

fn kinds(s: &Session) -> Vec<&'static str> {
    s.doc()
        .unwrap()
        .model
        .iter()
        .map(|e| match &e.kind {
            EntityKind::Insert(_) => "insert",
            EntityKind::LwPolyline(_) => "polyline",
            _ => "other",
        })
        .collect()
}

#[test]
fn make_part_stores_geometry_and_properties_together() {
    let (mut s, h) = session_with_rectangle();
    let r = s
        .execute(
            "woodpart",
            &json!({ "handles": [h], "name": "Leg", "thickness": 1.5, "commonSize": { "name": "2x4", "width": 3.5, "thickness": 1.5 } }),
        )
        .unwrap();
    let guid = r["part"].as_str().unwrap().to_string();

    assert_eq!(kinds(&s), ["insert"], "the outline is replaced by a reference to the part block");
    assert!(s.doc().unwrap().block(&guid).is_some(), "the block is named with the GUID");
    let data = store::get(s.doc().unwrap()).unwrap();
    let part = &data.parts[&guid];
    assert_eq!(part.name, "Leg");
    assert_eq!(part.thickness, 1.5);
    assert_eq!(part.common_size.as_ref().unwrap().preset_name, "2x4");
}

#[test]
fn undo_and_redo_carry_the_geometry_and_the_json() {
    let (mut s, h) = session_with_rectangle();
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Leg", "thickness": 1.5 })).unwrap();
    let guid = r["part"].as_str().unwrap().to_string();

    s.undo().unwrap();
    assert_eq!(kinds(&s), ["polyline"], "undo brought the outline back");
    assert!(s.doc().unwrap().block(&guid).is_none());
    assert!(store::get(s.doc().unwrap()).unwrap().parts.is_empty(), "undo took the part out of the JSON");

    s.redo().unwrap();
    assert_eq!(kinds(&s), ["insert"]);
    assert!(store::get(s.doc().unwrap()).unwrap().parts.contains_key(&guid), "redo put the part back in the JSON");
}

#[test]
fn make_part_refuses_bad_input_and_changes_nothing() {
    let (mut s, h) = session_with_rectangle();
    assert!(s.execute("woodpart", &json!({ "handles": [h.clone()], "name": "Leg" })).is_err(), "no thickness");
    assert!(s.execute("woodpart", &json!({ "handles": [h.clone()], "name": "", "thickness": 1.0 })).is_err(), "no name");
    assert!(s.execute("woodpart", &json!({ "handles": [h], "name": "Leg", "thickness": -1.0 })).is_err(), "negative thickness");
    assert_eq!(kinds(&s), ["polyline"]);
    assert!(store::get(s.doc().unwrap()).unwrap().parts.is_empty());
}
