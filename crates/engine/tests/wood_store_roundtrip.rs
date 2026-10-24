//! The part data written into a drawing survives Save and Open, and a revision counts changes.

use cadcraft_engine::doc::Drawing;
use cadcraft_io::{read_dxf, write_dxf};
use cadcraft_wood::data::{At, Face, HoleEntry};
use cadcraft_wood::{CommonSize, Part, WoodData, store};

fn sample() -> WoodData {
    let mut data = WoodData::default();
    let mut leg = Part::new("Leg – Tête (front)", 1.5);
    leg.grain_direction = Some(90.0);
    leg.common_size = Some(CommonSize { preset_name: "2x4".into(), width: 3.5, thickness: 1.5 });
    // Many entries, so the JSON is far longer than one DXF group.
    let at: Vec<At> = (0..80).map(|i| At { x: 1.5, y: i as f64 * 0.25 }).collect();
    leg.hole_through = vec![HoleEntry { face: Face::A, dia: 0.25, depth: None, extra1: None, extra2: None, at }];
    data.parts.insert("3f2a9c10-0000-4000-8000-000000000001".into(), leg);
    data
}

#[test]
fn a_new_drawing_has_no_part_data() {
    let d = Drawing::new_imperial();
    assert!(store::get(&d).unwrap().parts.is_empty());
}

#[test]
fn part_data_survives_save_and_open() {
    let mut d = Drawing::new_imperial();
    store::put(&mut d, &sample()).unwrap();
    assert_eq!(store::get(&d).unwrap(), sample());

    let text = write_dxf(&d);
    assert!(text.contains(store::KEY), "the dictionary key was not written");
    assert!(text.is_ascii() || !text.contains("Tête"), "plain text of a non-ASCII name was written");

    let back = read_dxf(text.as_bytes()).unwrap();
    assert_eq!(store::get(&back).unwrap(), sample(), "the data changed through Save and Open");

    // A second Save and Open, and a change on the opened drawing.
    let mut again = read_dxf(write_dxf(&back).as_bytes()).unwrap();
    assert_eq!(store::get(&again).unwrap(), sample());
    store::update(&mut again, |w| {
        w.parts.values_mut().next().unwrap().thickness = 2.0;
    })
    .unwrap();
    let last = read_dxf(write_dxf(&again).as_bytes()).unwrap();
    assert_eq!(store::get(&last).unwrap().parts.values().next().unwrap().thickness, 2.0);
}

#[test]
fn update_raises_the_revision_only_for_a_changed_part() {
    let mut d = Drawing::new_imperial();
    store::put(&mut d, &sample()).unwrap();
    let guid = "3f2a9c10-0000-4000-8000-000000000001";

    store::update(&mut d, |_| {}).unwrap();
    assert_eq!(store::get(&d).unwrap().parts[guid].revision, 1, "nothing changed");

    store::update(&mut d, |w| w.parts.get_mut(guid).unwrap().thickness = 0.75).unwrap();
    assert_eq!(store::get(&d).unwrap().parts[guid].revision, 2);

    store::update(&mut d, |w| w.parts.get_mut(guid).unwrap().thickness = 0.75).unwrap();
    assert_eq!(store::get(&d).unwrap().parts[guid].revision, 2, "same value again");
}
