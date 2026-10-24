//! Phase 1: `.wood` project files. WOODNEW makes an empty project; WOODSAVE/WOODSAVEAS write it as
//! DXF content under the `.wood` extension; WOODOPEN accepts only `.wood` and refuses a plain
//! `.dxf`. Standard 2D OPEN/QSAVE/SAVEAS are not touched by any of this.

use cadcraft_doc::Drawing;
use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, set_io};
use cadcraft_wood::store;
use serde_json::json;

fn read(bytes: &[u8], name: &str) -> Result<Drawing, String> {
    cadcraft_io::read(bytes, name).map_err(|e| e.to_string())
}
fn write(d: &Drawing, name: &str) -> Result<Vec<u8>, String> {
    cadcraft_io::write(d, name).map_err(|e| e.to_string())
}

/// Install the real DXF read/write hooks (not a stand-in), so a wood round-trip is genuine.
fn install_real_io() {
    set_io(IoHooks { read, write, plot: None });
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cadcraft-wood-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn woodnew_makes_an_empty_project_with_empty_part_data() {
    install_real_io();
    let mut s = Session::new();
    let r = s.execute("woodnew", &json!({})).unwrap();
    assert!(r["index"].is_number());
    assert!(store::get(s.doc().unwrap()).unwrap().parts.is_empty());
    assert!(s.state().unwrap().path.is_none(), "a new project has no file name yet");
}

#[test]
fn save_and_reopen_keeps_geometry_and_part_data() {
    install_real_io();
    let dir = temp_dir("roundtrip");
    let path = dir.join("bench.wood").to_string_lossy().to_string();

    let mut s = Session::new();
    s.execute("woodnew", &json!({})).unwrap();
    s.cmdline("rectang 0,0 3.5,38").unwrap();
    let h = s.doc().unwrap().model.iter().next().map(|e| e.handle).unwrap();
    let made = s.execute("woodpart", &json!({ "handles": [h.hex()], "name": "Leg", "thickness": 1.5 })).unwrap();
    let guid = made["part"].as_str().unwrap().to_string();

    let r = s.execute("woodsaveas", &json!({ "path": path })).unwrap();
    assert_eq!(r["path"], json!(path));
    assert!(r["bytes"].as_u64().unwrap() > 0);
    assert!(!s.state().unwrap().is_dirty());
    assert_eq!(s.state().unwrap().path.as_deref(), Some(path.as_str()));

    // A fresh session reopens it.
    let mut s2 = Session::new();
    s2.execute("woodopen", &json!({ "path": path })).unwrap();
    let data = store::get(s2.doc().unwrap()).unwrap();
    let part = &data.parts[&guid];
    assert_eq!(part.name, "Leg");
    assert_eq!(part.thickness, 1.5);
    assert_eq!(s2.doc().unwrap().entity_count(), 1, "the outline became one block reference, not a loose entity");
    assert!(s2.doc().unwrap().block(&guid).is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn woodsave_without_a_path_reuses_the_known_one() {
    install_real_io();
    let dir = temp_dir("resave");
    let path = dir.join("again.wood").to_string_lossy().to_string();

    let mut s = Session::new();
    s.execute("woodnew", &json!({})).unwrap();
    s.execute("woodsaveas", &json!({ "path": path })).unwrap();
    s.cmdline("circle 0,0 5").unwrap();
    let r = s.execute("woodsave", &json!({})).unwrap();
    assert_eq!(r["path"], json!(path));
    assert!(!s.state().unwrap().is_dirty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn woodsave_with_no_path_yet_is_refused() {
    install_real_io();
    let mut s = Session::new();
    s.execute("woodnew", &json!({})).unwrap();
    let err = s.execute("woodsave", &json!({})).unwrap_err().to_string();
    assert!(err.contains("woodsaveas"), "the error should point to woodsaveas: {err}");
}

#[test]
fn opening_a_plain_dxf_as_a_wood_project_is_refused() {
    install_real_io();
    let dir = temp_dir("refuse-dxf");
    let path = dir.join("plain.dxf").to_string_lossy().to_string();
    std::fs::write(&path, cadcraft_io::write_dxf(&Drawing::new_imperial())).unwrap();

    let mut s = Session::new();
    let before = s.docs.len();
    let err = s.execute("woodopen", &json!({ "path": path })).unwrap_err().to_string();
    assert!(err.contains(".wood"), "the error should explain the required extension: {err}");
    assert_eq!(s.docs.len(), before, "nothing was opened");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saving_a_wood_project_to_a_non_wood_path_is_refused() {
    install_real_io();
    let dir = temp_dir("refuse-save-ext");
    let path = dir.join("oops.dxf").to_string_lossy().to_string();

    let mut s = Session::new();
    s.execute("woodnew", &json!({})).unwrap();
    let err = s.execute("woodsaveas", &json!({ "path": path.clone() })).unwrap_err().to_string();
    assert!(err.contains(".wood"), "{err}");
    assert!(!std::path::Path::new(&path).exists(), "nothing is written");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn woodopen_and_woodsave_refuse_bad_input_without_crashing() {
    install_real_io();
    let mut s = Session::new();
    assert!(s.execute("woodopen", &json!({})).is_err(), "no path");
    assert!(s.execute("woodopen", &json!({ "path": "" })).is_err(), "empty path");
    assert!(s.execute("woodopen", &json!({ "path": "/nonexistent/x.wood" })).is_err(), "missing file");
    assert!(s.execute("woodsaveas", &json!({})).is_err(), "no path");
}
