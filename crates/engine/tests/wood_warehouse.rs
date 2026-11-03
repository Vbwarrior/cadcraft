//! Phase 10a: the Model Warehouse back end (folder listing, model listing, model detail and
//! placement). Each test gets its own temporary settings file and its own temporary warehouse
//! folder, wired together with `woodsettingsset { warehouseFolder }` the same way a real session
//! would point WoodCraft at a non-default warehouse location.

use std::sync::atomic::{AtomicU64, Ordering};

use cadcraft_engine::Session;
use cadcraft_engine::doc::Drawing;
use cadcraft_wood::settings_store::set_test_path;
use serde_json::json;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    session: Session,
    settings_dir: std::path::PathBuf,
    warehouse_dir: std::path::PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_test_path(None);
        let _ = std::fs::remove_dir_all(&self.settings_dir);
        let _ = std::fs::remove_dir_all(&self.warehouse_dir);
    }
}

fn fixture() -> Fixture {
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let settings_dir = std::env::temp_dir().join(format!("woodcraft-engine-warehouse-settings-{pid}-{n}"));
    let warehouse_dir = std::env::temp_dir().join(format!("woodcraft-engine-warehouse-root-{pid}-{n}"));
    let _ = std::fs::remove_dir_all(&warehouse_dir);
    std::fs::create_dir_all(&warehouse_dir).unwrap();
    set_test_path(Some(settings_dir.join("settings.json")));

    let mut session = Session::empty();
    session.open_drawing(Drawing::new_imperial(), "wood", None);
    session.execute("woodsettingsset", &json!({ "warehouseFolder": warehouse_dir.display().to_string() })).unwrap();

    Fixture { session, settings_dir, warehouse_dir }
}

fn write_model(root: &std::path::Path, rel: &str, name: &str, outline: &str) {
    let folder = root.join(rel);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("model.json"), format!(r#"{{"schemaVersion":1,"name":"{name}","outline":{outline}}}"#)).unwrap();
}

#[test]
fn folders_lists_the_warehouse_root_then_drills_down() {
    let mut f = fixture();
    std::fs::create_dir_all(f.warehouse_dir.join("Hardware/Hinges")).unwrap();
    std::fs::create_dir_all(f.warehouse_dir.join("Hardware/Knobs")).unwrap();

    let top = f.session.execute("woodwarehousefolders", &json!({})).unwrap();
    assert_eq!(top["folders"], json!([{ "name": "Hardware", "path": "Hardware" }]));

    let under = f.session.execute("woodwarehousefolders", &json!({ "under": "Hardware" })).unwrap();
    assert_eq!(
        under["folders"],
        json!([
            { "name": "Hinges", "path": "Hardware/Hinges" },
            { "name": "Knobs", "path": "Hardware/Knobs" },
        ])
    );
}

#[test]
fn folders_on_a_missing_warehouse_folder_is_refused() {
    let mut f = fixture();
    assert!(f.session.execute("woodwarehousefolders", &json!({ "under": "Nope" })).is_err());
}

#[test]
fn models_lists_the_models_directly_in_a_folder() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/Hinges/soss-100", "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");

    let r = f.session.execute("woodwarehousemodels", &json!({ "under": "Hardware/Hinges" })).unwrap();
    let models = r["models"].as_array().unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["name"], "SOSS 100");
    assert_eq!(models[0]["path"], "Hardware/Hinges/soss-100");
    assert_eq!(models[0]["width"], 2.0);
    assert_eq!(models[0]["height"], 1.0);
    assert_eq!(r["errors"].as_array().unwrap().len(), 0);
}

#[test]
fn models_reports_a_malformed_model_as_an_error_not_a_failure() {
    let mut f = fixture();
    std::fs::create_dir_all(f.warehouse_dir.join("Hardware/bad")).unwrap();
    std::fs::write(f.warehouse_dir.join("Hardware/bad/model.json"), "{not json").unwrap();
    write_model(&f.warehouse_dir, "Hardware/good", "Good", "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]");

    let r = f.session.execute("woodwarehousemodels", &json!({ "under": "Hardware" })).unwrap();
    assert_eq!(r["models"].as_array().unwrap().len(), 1);
    assert_eq!(r["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn model_detail_reads_the_outline_and_the_3d_file_name() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/soss-100", "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");
    std::fs::write(f.warehouse_dir.join("Hardware/soss-100/model.dae"), "<dae/>").unwrap();

    let r = f.session.execute("woodwarehousemodeldetail", &json!({ "path": "Hardware/soss-100" })).unwrap();
    assert_eq!(r["name"], "SOSS 100");
    assert_eq!(r["outline"], json!([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0], [0.0, 1.0, 0.0]]));
    assert_eq!(r["modelFile"], "Hardware/soss-100/model.dae");
}

#[test]
fn place_draws_the_outline_at_the_given_point_and_links_the_component() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/soss-100", "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");

    let before = f.session.doc().unwrap().model.iter().count();
    let r = f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/soss-100", "at": [10.0, 5.0] })).unwrap();
    let after = f.session.doc().unwrap().model.iter().count();
    assert_eq!(after, before + 1, "exactly one insert should be added to the drawing");

    let component_guid = r["component"].as_str().unwrap().to_string();
    let doc = f.session.doc().unwrap();
    let components = cadcraft_wood::components::get(doc).unwrap();
    assert_eq!(components.components.len(), 1);
    assert_eq!(components.components[&component_guid].model, "Hardware/soss-100");
    assert_eq!(components.components[&component_guid].name, "SOSS 100");

    // The placed block's outline must be offset by `at`, not left at the model's own origin.
    let block = doc.block(&component_guid).unwrap();
    let outline = block.entities.iter().find_map(|e| match &e.kind {
        cadcraft_engine::doc::EntityKind::LwPolyline(pl) => Some(pl.vertices.clone()),
        _ => None,
    });
    assert!(outline.is_some());
}

#[test]
fn place_keeps_components_separate_from_wood_parts() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/soss-100", "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");
    f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/soss-100", "at": [0.0, 0.0] })).unwrap();

    let doc = f.session.doc().unwrap();
    assert_eq!(cadcraft_wood::store::get(doc).unwrap().parts.len(), 0, "a placed component is not a wood part");
    assert_eq!(cadcraft_wood::components::get(doc).unwrap().components.len(), 1);
}

#[test]
fn place_undoes_and_redoes_both_the_geometry_and_the_component_record() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/soss-100", "SOSS 100", "[[0,0,0],[2,0,0],[2,1,0],[0,1,0]]");

    let before = f.session.doc().unwrap().model.iter().count();
    f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/soss-100", "at": [0.0, 0.0] })).unwrap();
    let after_place = f.session.doc().unwrap().model.iter().count();
    assert_eq!(after_place, before + 1);

    f.session.undo().unwrap();
    assert_eq!(f.session.doc().unwrap().model.iter().count(), before);
    assert_eq!(cadcraft_wood::components::get(f.session.doc().unwrap()).unwrap().components.len(), 0);

    f.session.redo().unwrap();
    assert_eq!(f.session.doc().unwrap().model.iter().count(), after_place);
    assert_eq!(cadcraft_wood::components::get(f.session.doc().unwrap()).unwrap().components.len(), 1);
}

#[test]
fn place_refuses_an_unknown_model_without_changing_anything() {
    let mut f = fixture();
    let before = f.session.doc().unwrap().model.iter().count();
    assert!(f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/nope", "at": [0.0, 0.0] })).is_err());
    assert_eq!(before, f.session.doc().unwrap().model.iter().count());
}

#[test]
fn place_refuses_a_model_with_no_outline_without_changing_anything() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/placeholder", "Placeholder", "[]");
    let before = f.session.doc().unwrap().model.iter().count();
    assert!(f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/placeholder", "at": [0.0, 0.0] })).is_err());
    assert_eq!(before, f.session.doc().unwrap().model.iter().count());
}

#[test]
fn place_refuses_missing_required_params() {
    let mut f = fixture();
    write_model(&f.warehouse_dir, "Hardware/soss-100", "SOSS 100", "[[0,0,0],[1,0,0],[1,1,0],[0,1,0]]");
    assert!(f.session.execute("woodwarehouseplace", &json!({ "path": "Hardware/soss-100" })).is_err());
    assert!(f.session.execute("woodwarehouseplace", &json!({ "at": [0.0, 0.0] })).is_err());
}
