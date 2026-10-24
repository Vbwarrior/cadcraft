//! Phase 3: copy and array rules. Copy makes an independent part; Copy Array adds another shared
//! instance; Delete removes a part's entry once its last instance is gone; Split Array and Remove
//! From give the picked instances their own GUID; Explode Array does that to every instance.

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
fn woodcopy_makes_an_independent_part_with_copy_suffix() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    let r = s.execute("woodcopy", &json!({ "handle": insert, "to": [10, 0] })).unwrap();
    let new_guid = r["part"].as_str().unwrap().to_string();
    assert_ne!(new_guid, guid);

    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts[&new_guid].name, "Leg -Copy");
    assert_eq!(data.parts[&new_guid].thickness, 1.5);
    assert_eq!(data.parts[&guid].name, "Leg", "the original is untouched");
    assert_eq!(data.parts.len(), 2);
    assert!(s.doc().unwrap().block(&new_guid).is_some(), "a new block was made");

    // Editing the copy does not affect the original.
    let new_insert = r["insert"].as_str().unwrap();
    s.execute("woodthickness", &json!({ "handle": new_insert, "thickness": 0.75 })).unwrap();
    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts[&new_guid].thickness, 0.75);
    assert_eq!(data.parts[&guid].thickness, 1.5);
}

#[test]
fn woodcopy_refuses_without_a_wood_part_selected() {
    let mut s = Session::empty();
    s.open_drawing(Drawing::new_imperial(), "wood", None);
    assert!(s.execute("woodcopy", &json!({ "to": [0, 0] })).is_err());
}

#[test]
fn woodcopyarray_adds_a_shared_instance() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    let r = s.execute("woodcopyarray", &json!({ "handle": insert, "to": [10, 0] })).unwrap();
    let new_insert = r["insert"].as_str().unwrap();

    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts.len(), 1, "no new JSON entry: the GUID is shared");
    assert_eq!(data.parts[&guid].name, "Leg");

    // Both instances reference the same block.
    let h1 = cadcraft_doc::Handle::parse_hex(&insert).unwrap();
    let h2 = cadcraft_doc::Handle::parse_hex(new_insert).unwrap();
    for h in [h1, h2] {
        match s.doc().unwrap().entity(h).map(|e| &e.kind) {
            Some(cadcraft_doc::EntityKind::Insert(ins)) => assert_eq!(ins.block, guid),
            _ => panic!("expected an Insert"),
        }
    }
}

#[test]
fn editing_a_shared_part_with_more_than_one_instance_is_refused() {
    let (mut s, _guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();

    let err = s.execute("woodthickness", &json!({ "handle": insert, "thickness": 0.75 })).unwrap_err().to_string();
    assert!(err.contains("Split Array"), "{err}");
}

#[test]
fn wooddelete_removes_the_entry_only_when_the_last_instance_is_gone() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    let r = s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();
    let second = r["insert"].as_str().unwrap().to_string();

    s.execute("wooddelete", &json!({ "handles": [insert] })).unwrap();
    assert!(store::get(s.doc().unwrap()).unwrap().parts.contains_key(&guid), "one instance remains");

    s.execute("wooddelete", &json!({ "handles": [second] })).unwrap();
    assert!(!store::get(s.doc().unwrap()).unwrap().parts.contains_key(&guid), "the last instance is gone");
}

#[test]
fn woodsplitarray_gives_picked_instances_their_own_guid() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    let r = s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();
    let second = r["insert"].as_str().unwrap().to_string();
    let r = s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [20, 0] })).unwrap();
    let third = r["insert"].as_str().unwrap().to_string();

    let r = s.execute("woodsplitarray", &json!({ "handles": [second] })).unwrap();
    let changed = r["changed"].as_array().unwrap();
    assert_eq!(changed.len(), 1);
    let new_guid = changed[0]["part"].as_str().unwrap();
    assert_ne!(new_guid, guid);

    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts.len(), 2, "one split off, one remains shared");
    assert_eq!(data.parts[new_guid].name, "Leg");

    // The two untouched instances are still on the original GUID.
    for h in [&insert, &third] {
        let h = cadcraft_doc::Handle::parse_hex(h).unwrap();
        match s.doc().unwrap().entity(h).map(|e| &e.kind) {
            Some(cadcraft_doc::EntityKind::Insert(ins)) => assert_eq!(ins.block, guid),
            _ => panic!("expected an Insert"),
        }
    }
    // Editing the two remaining shared instances is now allowed again (down to 2, still > 1
    // though) — but editing the split-off one (now alone) is allowed.
    let new_insert = changed[0]["insert"].as_str().unwrap();
    s.execute("woodthickness", &json!({ "handle": new_insert, "thickness": 2.0 })).unwrap();
}

#[test]
fn woodsplitarray_refuses_whole_if_any_pick_is_not_an_instance() {
    let (mut s, _guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();
    s.execute("rectang", &json!({ "p1": [50, 50], "p2": [51, 51] })).unwrap();
    let other = s.doc().unwrap().model.iter().find(|e| matches!(e.kind, cadcraft_doc::EntityKind::LwPolyline(_))).unwrap().handle.hex();

    let before = store::get(s.doc().unwrap()).unwrap();
    let err = s.execute("woodsplitarray", &json!({ "handles": [insert, other] })).unwrap_err().to_string();
    assert!(!err.is_empty());
    assert_eq!(store::get(s.doc().unwrap()).unwrap(), before, "nothing changed");
}

#[test]
fn woodexplodearray_splits_every_instance() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();
    s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [20, 0] })).unwrap();

    let r = s.execute("woodexplodearray", &json!({ "handle": insert })).unwrap();
    let changed = r["changed"].as_array().unwrap();
    assert_eq!(changed.len(), 3, "all three instances split off");

    let data = store::get(s.doc().unwrap()).unwrap();
    assert_eq!(data.parts.len(), 3, "three independent parts now");
    assert!(!data.parts.contains_key(&guid), "the original shared entry is gone (its last instance was split off)");
    for c in changed {
        assert_eq!(data.parts[c["part"].as_str().unwrap()].name, "Leg");
    }
}

#[test]
fn woodundo_and_redo_carry_a_split() {
    let (mut s, guid, insert) = session_with_part("Leg", 1.5);
    s.execute("woodcopyarray", &json!({ "handle": insert.clone(), "to": [10, 0] })).unwrap();
    s.execute("woodsplitarray", &json!({ "handles": [insert] })).unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts.len(), 2);

    s.undo().unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts.len(), 1, "undo merges back to one shared part");
    assert!(store::get(s.doc().unwrap()).unwrap().parts.contains_key(&guid));

    s.redo().unwrap();
    assert_eq!(store::get(s.doc().unwrap()).unwrap().parts.len(), 2);
}
