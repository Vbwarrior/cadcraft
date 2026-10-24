//! Phase 5: reading a wood part's outline into named edges through the `woodedges` command.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A session whose settings file is its own temporary file for the duration of the closure, so
/// the straightness tolerance setting other tests may have changed never crosses into this one.
fn with_settings<R>(f: impl FnOnce() -> R) -> R {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("woodcraft-engine-edges-test-{}-{n}", std::process::id()));
    set_test_path(Some(dir.join("settings.json")));
    let r = f();
    set_test_path(None);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

fn session_with_rect(w: f64, h: f64) -> (Session, String) {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    s.execute("rectang", &json!({ "p1": [0, 0], "p2": [w, h] })).unwrap();
    let handle = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
    let r = s.execute("woodpart", &json!({ "handles": [handle], "name": "Leg", "thickness": 0.75 })).unwrap();
    let insert = r["insert"].as_str().unwrap().to_string();
    (s, insert)
}

#[test]
fn a_rectangle_part_reads_back_four_named_edges() {
    with_settings(|| {
        let (mut s, insert) = session_with_rect(10.0, 4.0);
        let r = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
        let edges = r["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 4);
        let mut names: Vec<&str> = edges.iter().map(|e| e["name"].as_str().unwrap()).collect();
        names.sort_unstable();
        assert_eq!(names, ["Bottom", "Left", "Right", "Top"]);
        assert!(edges.iter().all(|e| e["flat"] == json!(true) && e["arc"] == json!(false)));
    });
}

#[test]
fn woodedges_defaults_to_the_current_selection() {
    with_settings(|| {
        let (mut s, insert) = session_with_rect(10.0, 4.0);
        s.set_selection(vec![cadcraft_engine::doc::Handle::parse_hex(&insert).unwrap()]);
        let r = s.execute("woodedges", &json!({})).unwrap();
        assert_eq!(r["edges"].as_array().unwrap().len(), 4);
    });
}

#[test]
fn woodedges_refuses_without_exactly_one_part_selected() {
    with_settings(|| {
        let (mut s, _) = session_with_rect(10.0, 4.0);
        assert!(s.execute("woodedges", &json!({ "handles": [] })).is_err());
    });
}

#[test]
fn woodedges_refuses_a_handle_that_is_not_a_wood_part() {
    with_settings(|| {
        let mut s = Session::empty();
        s.open_drawing(Drawing::new_imperial(), "wood", None);
        s.execute("line", &json!({ "points": [[0, 0], [10, 0]] })).unwrap();
        let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
        assert!(s.execute("woodedges", &json!({ "handles": [h] })).is_err());
    });
}

#[test]
fn a_circle_part_has_no_named_edges() {
    with_settings(|| {
        let mut s = Session::empty();
        s.open_drawing(Drawing::new_imperial(), "wood", None);
        s.execute("circle", &json!({ "center": [0, 0], "radius": 2.0 })).unwrap();
        let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
        let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Disc", "thickness": 0.5 })).unwrap();
        let insert = r["insert"].as_str().unwrap().to_string();
        let edges = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
        assert_eq!(edges["edges"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn woodedges_reads_the_live_straightness_tolerance_setting() {
    with_settings(|| {
        // A right edge leaning by a shade under one degree.
        let mut s = Session::empty();
        s.open_drawing(Drawing::new_imperial(), "wood", None);
        s.execute("pline", &json!({ "vertices": [[0, 0], [10, 0], [10.05, 4], [0, 4]], "closed": true })).unwrap();
        let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap().hex();
        let r = s.execute("woodpart", &json!({ "handles": [h], "name": "Panel", "thickness": 0.5 })).unwrap();
        let insert = r["insert"].as_str().unwrap().to_string();

        let strict = s.execute("woodedges", &json!({ "handles": [insert.clone()] })).unwrap();
        let right = strict["edges"].as_array().unwrap().iter().find(|e| e["side"] == json!("Right")).unwrap();
        assert_eq!(right["flat"], json!(false));

        s.execute("woodsettingsset", &json!({ "straightnessTolerance": 2.0 })).unwrap();
        let lenient = s.execute("woodedges", &json!({ "handles": [insert] })).unwrap();
        let right = lenient["edges"].as_array().unwrap().iter().find(|e| e["side"] == json!("Right")).unwrap();
        assert_eq!(right["flat"], json!(true));
    });
}
