//! Phase 10: layout tools under the shop names (tape measure, T-square, compass, chalk line)
//! and the marks-for-transfer query.

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
fn tape_measure_reports_distance_and_angle_like_dist() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let r = s.execute("woodtapemeasure", &json!({ "p1": [0, 0], "p2": [3, 4] })).unwrap();
    assert_eq!(r["distance"], json!(5.0));
    assert_eq!(r["dx"], json!(3.0));
    assert_eq!(r["dy"], json!(4.0));
}

#[test]
fn tape_measure_does_not_change_the_drawing() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let before = s.doc().unwrap().model.iter().count();
    s.execute("woodtapemeasure", &json!({ "p1": [0, 0], "p2": [3, 4] })).unwrap();
    let after = s.doc().unwrap().model.iter().count();
    assert_eq!(before, after);
}

#[test]
fn tape_measure_refuses_bad_input_without_changing_anything() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let before = s.doc().unwrap().model.iter().count();
    assert!(s.execute("woodtapemeasure", &json!({ "p1": [0, 0] })).is_err());
    assert_eq!(before, s.doc().unwrap().model.iter().count());
}

#[test]
fn chalk_line_draws_an_unbounded_line_like_ray() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let before = s.doc().unwrap().model.iter().count();
    s.execute("woodchalkline", &json!({ "base": [0, 0], "through": [10, 0] })).unwrap();
    let after = s.doc().unwrap().model.iter().count();
    assert_eq!(after, before + 1);
    let added = s.doc().unwrap().model.iter().last().unwrap();
    assert!(matches!(&added.kind, cadcraft_engine::doc::EntityKind::Ray(_)));
}

#[test]
fn chalk_line_undoes_and_redoes() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("woodchalkline", &json!({ "base": [0, 0], "through": [10, 0] })).unwrap();
    let after_draw = s.doc().unwrap().model.iter().count();
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().model.iter().count(), after_draw - 1);
    s.redo().unwrap();
    assert_eq!(s.doc().unwrap().model.iter().count(), after_draw);
}

#[test]
fn chalk_line_refuses_through_equal_to_base() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    assert!(s.execute("woodchalkline", &json!({ "base": [1, 1], "through": [1, 1] })).is_err());
}

#[test]
fn compass_strikes_a_circle_from_a_center() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    let before = s.doc().unwrap().model.iter().count();
    s.execute("woodcompass", &json!({ "center": [5, 5], "radius": 2 })).unwrap();
    let after = s.doc().unwrap().model.iter().count();
    assert_eq!(after, before + 1);
    let added = s.doc().unwrap().model.iter().last().unwrap();
    assert!(matches!(&added.kind, cadcraft_engine::doc::EntityKind::Circle(_)));
}

#[test]
fn compass_refuses_a_non_positive_radius() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    assert!(s.execute("woodcompass", &json!({ "center": [0, 0], "radius": 0 })).is_err());
    assert!(s.execute("woodcompass", &json!({ "center": [0, 0], "radius": -1 })).is_err());
}

#[test]
fn t_square_draws_a_construction_line_square_to_the_named_edge() {
    let (mut s, insert) = session_with_part();
    let before = s.doc().unwrap().model.iter().count();
    s.execute("woodtsquare", &json!({ "handles": [insert], "edge": "Top", "at": [5.0, 4.0] })).unwrap();
    let after = s.doc().unwrap().model.iter().count();
    assert_eq!(after, before + 1);
    let added = s.doc().unwrap().model.iter().last().unwrap();
    match &added.kind {
        cadcraft_engine::doc::EntityKind::XLine(xl) => {
            // "Top" runs along X, so the square line must run along Y: dir.x ~= 0.
            assert!(xl.dir.x.abs() < 1e-9, "expected a vertical construction line, got dir = {:?}", xl.dir);
        }
        other => panic!("expected an XLine, got {other:?}"),
    }
}

#[test]
fn t_square_refuses_an_unknown_edge_name_without_changing_anything() {
    let (mut s, insert) = session_with_part();
    let before = s.doc().unwrap().model.iter().count();
    assert!(s.execute("woodtsquare", &json!({ "handles": [insert], "edge": "Nowhere", "at": [5.0, 4.0] })).is_err());
    assert_eq!(before, s.doc().unwrap().model.iter().count());
}

#[test]
fn t_square_refuses_when_more_than_one_part_is_selected() {
    let (mut s, insert) = session_with_part();
    s.execute("rectang", &json!({ "p1": [20, 0], "p2": [30, 4] })).unwrap();
    let h2 = s.doc().unwrap().model.iter().last().unwrap().handle.hex();
    let r2 = s.execute("woodpart", &json!({ "handles": [h2], "name": "Leg 2", "thickness": 0.75 })).unwrap();
    let insert2 = r2["insert"].as_str().unwrap().to_string();
    assert!(s.execute("woodtsquare", &json!({ "handles": [insert, insert2], "edge": "Top", "at": [5.0, 4.0] })).is_err());
}

#[test]
fn marks_for_transfer_is_empty_for_a_new_part() {
    let (mut s, insert) = session_with_part();
    let r = s.execute("woodmarkstransfer", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(r["marks"].as_array().unwrap().len(), 0);
}

#[test]
fn marks_for_transfer_lists_orientation_then_reference_marks_in_order() {
    let (mut s, insert) = session_with_part();
    s.execute("woodorientationmark", &json!({ "handles": [insert.clone()], "face": "A", "edge": "Top", "at": [1.0, 4.0] })).unwrap();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "A", "at": [0.0, 0.0], "label": "Datum" })).unwrap();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "B", "at": [10.0, 4.0], "label": "Far corner" })).unwrap();

    let r = s.execute("woodmarkstransfer", &json!({ "handles": [insert] })).unwrap();
    let marks = r["marks"].as_array().unwrap();
    assert_eq!(marks.len(), 3);
    assert_eq!(marks[0], json!({ "kind": "Orientation", "face": "A", "edge": "Top" }));
    assert_eq!(marks[1], json!({ "kind": "Reference", "face": "A", "x": 0.0, "y": 0.0, "label": "Datum" }));
    assert_eq!(marks[2], json!({ "kind": "Reference", "face": "B", "x": 10.0, "y": 4.0, "label": "Far corner" }));
}

#[test]
fn marks_for_transfer_does_not_change_the_drawing() {
    let (mut s, insert) = session_with_part();
    s.execute("woodreferencemarkadd", &json!({ "handles": [insert.clone()], "face": "A", "at": [0.0, 0.0], "label": "Datum" })).unwrap();
    let before = s.doc().unwrap().model.iter().count();
    s.execute("woodmarkstransfer", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(before, s.doc().unwrap().model.iter().count());
}
