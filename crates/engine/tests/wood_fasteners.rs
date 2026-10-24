//! Phase 6, group 6 (part 1): glue and fasteners.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-fasteners-test-{}-{n}", std::process::id()));
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
    let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Side", "thickness": 0.75 })).unwrap();
    (s, r["insert"].as_str().unwrap().to_string())
}

#[test]
fn a_new_part_has_no_fasteners() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["glue"], json!([]));
        assert_eq!(r["screws"], json!([]));
        assert_eq!(r["nails"], json!([]));
        assert_eq!(r["dowels"], json!([]));
    });
}

#[test]
fn glue_type_must_be_in_the_settings_list() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(s.execute("woodglueset", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Not a glue" })).is_err());
        s.execute("woodglueset", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Wood glue" })).unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["glue"], json!([{ "faceEdge": "Top", "type": "Wood glue" }]));
    });
}

#[test]
fn setting_glue_twice_for_the_same_edge_replaces_it() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodglueset", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Wood glue" })).unwrap();
        s.execute("woodglueset", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Epoxy" })).unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        let glue = r["glue"].as_array().unwrap();
        assert_eq!(glue.len(), 1);
        assert_eq!(glue[0]["type"], json!("Epoxy"));
    });
}

#[test]
fn screw_type_must_be_in_the_settings_list() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        assert!(
            s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Torx", "at": [1, 1], "size": 1.25 }))
                .is_err()
        );
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 1], "size": 1.25 }))
            .unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        let screws = r["screws"].as_array().unwrap();
        assert_eq!(screws.len(), 1);
        assert_eq!(screws[0]["type"], json!("Phillips"));
    });
}

#[test]
fn two_matching_screws_share_one_entry() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 1], "size": 1.25 }))
            .unwrap();
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 3], "size": 1.25 }))
            .unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        let screws = r["screws"].as_array().unwrap();
        assert_eq!(screws.len(), 1);
        assert_eq!(screws[0]["at"], json!([[1.0, 1.0], [1.0, 3.0]]));
    });
}

#[test]
fn deleting_a_screw_position_leaves_the_other() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 1], "size": 1.25 }))
            .unwrap();
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 3], "size": 1.25 }))
            .unwrap();
        s.execute("woodscrewdelete", &json!({ "handles": [insert.clone()], "at": [1, 1] })).unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["screws"].as_array().unwrap()[0]["at"], json!([[1.0, 3.0]]));
    });
}

#[test]
fn nails_have_diameter_and_length_but_no_type() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodnailadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "at": [2, 2], "diameter": 0.1, "length": 1.5 })).unwrap();
        let r = s.execute("woodfasteners", &json!({ "handles": [insert] })).unwrap();
        let nails = r["nails"].as_array().unwrap();
        assert_eq!(nails.len(), 1);
        assert_eq!(nails[0]["diameter"], json!(0.1));
        assert_eq!(nails[0]["length"], json!(1.5));
        assert!(nails[0].get("type").is_none());
    });
}

#[test]
fn dowel_fastener_is_distinct_from_hole_dowel() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("wooddowelfastenadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "at": [3, 2], "size": 0.25 })).unwrap();
        s.execute("woodholeadd", &json!({ "handles": [insert.clone()], "kind": "Hole-Dowel", "face": "A", "at": [4, 2], "dia": 0.25, "depth": 0.5 }))
            .unwrap();
        let fasteners = s.execute("woodfasteners", &json!({ "handles": [insert.clone()] })).unwrap();
        let holes = s.execute("woodholes", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(fasteners["dowels"].as_array().unwrap().len(), 1);
        assert_eq!(holes["holes"]["Hole-Dowel"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn the_fastener_mark_is_never_mistaken_for_the_outline() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodscrewadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Phillips", "at": [1, 1], "size": 1.25 }))
            .unwrap();
        let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(r["edges"].as_array().unwrap().len(), 4);
    });
}

#[test]
fn fasteners_are_copied_with_the_part() {
    with_settings(|| {
        let (mut s, insert) = session_with_part();
        s.execute("woodglueset", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "type": "Wood glue" })).unwrap();
        s.execute("woodnailadd", &json!({ "handles": [insert.clone()], "faceEdge": "Top", "at": [2, 2], "diameter": 0.1, "length": 1.5 })).unwrap();
        let copy = s.execute("woodcopy", &json!({ "handles": [insert], "to": [20, 0] })).unwrap();
        let data = s.execute("woodpartinfo", &json!({ "handles": [copy["insert"].as_str().unwrap()] })).unwrap();
        assert_eq!(data["Glue"].as_array().unwrap().len(), 1);
        assert_eq!(data["Nail"].as_array().unwrap().len(), 1);
    });
}
