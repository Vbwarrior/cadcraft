//! WoodCraft project files: new, open and save of `.wood` files (Phase 1 of the WoodCraft plan).
//! A `.wood` file is DXF content under a different extension. The io layer (`cadcraft-io`, a
//! lower layer than `engine`) only recognizes `dxf`/`dwg`/`dwt`/`svg`/`png`/`pdf` by name, so
//! writing calls the host's [`super::file::IoHooks::write`] with a `.dxf` name to get DXF bytes,
//! then writes those bytes to the real `.wood` path. Reading already works through the hook as
//! written: an unrecognized extension still falls back to a DXF parse attempt. Standard 2D
//! `open`/`qsave`/`saveas` (`crates/engine/src/cmd/file.rs`) are untouched by this module.

use cadcraft_doc::Drawing;
use serde_json::{Value, json};

use super::file::io;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodnew", "New Wood Project...", run_woodnew).enabled(always).noundo(),
        CommandSpec::new("woodopen", "Open Wood Project...", run_woodopen).params("{path} (must end in .wood)").enabled(always).noundo(),
        CommandSpec::new("woodsave", "Save Wood Project", run_woodsave).params("{path?}").noundo(),
        CommandSpec::new("woodsaveas", "Save Wood Project As...", run_woodsaveas).params("{path} (must end in .wood)").noundo(),
    ]
}

/// The file's base name, for the drawing's display title.
fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string())
}

/// Whether `path` ends in the WoodCraft extension (`.wood`), case-insensitively.
fn is_wood_file(path: &str) -> bool {
    std::path::Path::new(path).extension().map(|e| e.eq_ignore_ascii_case(cadcraft_wood::EXTENSION)).unwrap_or(false)
}

/// NEW WOOD PROJECT: an empty drawing that already holds an empty part JSON (so Save always
/// writes a document a reader can parse, even before any part exists).
fn run_woodnew(s: &mut Session, _p: &Value) -> Result<Value> {
    s.untitled_counter += 1;
    let mut d = Drawing::new_imperial();
    cadcraft_wood::store::put(&mut d, &cadcraft_wood::WoodData::default()).map_err(|e| EngineError::Other(e.to_string()))?;
    let i = s.open_drawing(d, &format!("Drawing{}.{}", s.untitled_counter, cadcraft_wood::EXTENSION), None);
    Ok(json!({ "index": i }))
}

/// OPEN WOOD PROJECT: `.wood` only. A plain `.dxf` (or any other extension) is refused with a
/// message naming the problem, rather than silently opened as an ordinary drawing.
fn run_woodopen(s: &mut Session, p: &Value) -> Result<Value> {
    let path = str_param(p, "path").ok_or_else(|| bad("woodopen", "`path` is required"))?;
    if !is_wood_file(path) {
        return Err(bad(
            "woodopen",
            format!("\"{}\" is not a WoodCraft project file; it must end in .{}", file_name(path), cadcraft_wood::EXTENSION),
        ));
    }
    let hooks = io().ok_or_else(|| bad("woodopen", "file formats are not available in this build"))?;
    let bytes = std::fs::read(path).map_err(|e| bad("woodopen", format!("{path}: {e}")))?;
    let d = (hooks.read)(&bytes, path).map_err(|e| bad("woodopen", format!("{path}: {e}")))?;
    let i = s.open_drawing(d, &file_name(path), Some(path.to_string()));
    Ok(json!({ "index": i }))
}

/// Write the active drawing's DXF content to `path` as a `.wood` file, then mark it saved there.
/// Asks the io hook to write as DXF (by passing a `.dxf`-named stand-in) since the hook itself
/// does not know the `.wood` extension; the bytes it returns go to the real path.
fn write_wood(s: &mut Session, cmd: &str, path: &str) -> Result<usize> {
    if !is_wood_file(path) {
        return Err(bad(cmd, format!("\"{}\" must end in .{} to be a WoodCraft project file", file_name(path), cadcraft_wood::EXTENSION)));
    }
    let hooks = io().ok_or_else(|| bad(cmd, "file formats are not available in this build"))?;
    let bytes = (hooks.write)(s.doc()?, "woodcraft-project.dxf").map_err(|e| bad(cmd, e))?;
    let tmp = format!("{path}.cadcraft-tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| bad(cmd, format!("{path}: {e}")))?;
    std::fs::rename(&tmp, path).map_err(|e| bad(cmd, format!("{path}: {e}")))?;
    let st = s.state_mut()?;
    st.saved = st.drawing().clone();
    st.path = Some(path.to_string());
    st.title = file_name(path);
    st.end_undo_group();
    Ok(bytes.len())
}

/// SAVE WOOD PROJECT: like QSAVE, but for `.wood` files — saves to the drawing's known path, or
/// `path` the first time.
fn run_woodsave(s: &mut Session, p: &Value) -> Result<Value> {
    let path = match str_param(p, "path") {
        Some(p) => p.to_string(),
        None => s.state()?.path.clone().ok_or_else(|| bad("woodsave", "project has no file name yet; use woodsaveas {path}"))?,
    };
    let n = write_wood(s, "woodsave", &path)?;
    Ok(json!({ "path": path, "bytes": n }))
}

/// SAVE WOOD PROJECT AS: like SAVEAS, but refuses anything not ending in `.wood`.
fn run_woodsaveas(s: &mut Session, p: &Value) -> Result<Value> {
    let path = str_param(p, "path").ok_or_else(|| bad("woodsaveas", "`path` is required"))?.to_string();
    let n = write_wood(s, "woodsaveas", &path)?;
    Ok(json!({ "path": path, "bytes": n }))
}
