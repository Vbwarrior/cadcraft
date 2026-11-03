//! WoodCraft Phase 10a: the Model Warehouse back end. Reads the warehouse folder
//! (`cadcraft_wood::warehouse`; the folder itself is written by a separate importer app, never
//! by WoodCraft) and places a model's flat top-view outline into the drawing, linked back to
//! the warehouse model by its own component record (`cadcraft_wood::components`,
//! `WOODCRAFT_COMPONENTS`), kept separate from `WOODCRAFT_PARTS` because placed hardware is not
//! a wood part (`plans/WoodCraft_Decisions.md` item 25).
//!
//! No menu path yet, like every WoodCraft command so far — menu placement is Phase 12.

use cadcraft_geom::{PolyVertex, Vec2};
use cadcraft_wood::components::{self, Component};
use cadcraft_wood::{settings_store, warehouse};
use serde_json::{Value, json};

use super::blocks::{Keep, insert, make_block};
use super::helpers::lwpoly;
use super::*;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("woodwarehousefolders", "Model Warehouse Folders", run_woodwarehousefolders)
            .params("{under?: \"Category/Sub\"} -> {folders: [{name, path}]}: the categories directly inside the warehouse root, or inside `under`")
            .noundo()
            .transparent(),
        CommandSpec::new("woodwarehousemodels", "Model Warehouse Models", run_woodwarehousemodels)
            .params(
                "{under: \"Category/Sub\"} -> {models: [{name, path, icon?, width, height}], errors: [{path, message}]}: \
                 every model directly inside the folder, plus any that could not be read",
            )
            .noundo()
            .transparent(),
        CommandSpec::new("woodwarehousemodeldetail", "Model Warehouse Model Detail", run_woodwarehousemodeldetail)
            .params("{path: \"Category/Sub/model\"} -> {name, path, icon?, outline: [[x,y,bulge]], modelFile?}: one model's full detail")
            .noundo()
            .transparent(),
        CommandSpec::new("woodwarehouseplace", "Place Model Warehouse Model", run_woodwarehouseplace).params(
            "{path: \"Category/Sub/model\", at: [x,y]} -> {component, insert}: draws the model's outline at `at` and links it to the warehouse model",
        ),
    ]
}

fn warehouse_root(cmd: &str) -> Result<std::path::PathBuf> {
    settings_store::warehouse_dir().map_err(|e| bad(cmd, e.to_string()))
}

fn run_woodwarehousefolders(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodwarehousefolders";
    let under = str_param(p, "under").map(str::trim).filter(|v| !v.is_empty());
    let root = warehouse_root(cmd)?;
    let folders = warehouse::categories(&root, under).map_err(|e| bad(cmd, e.to_string()))?;
    Ok(json!({ "folders": folders.into_iter().map(|c| json!({ "name": c.name, "path": c.path })).collect::<Vec<_>>() }))
}

fn run_woodwarehousemodels(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodwarehousemodels";
    let under = str_param(p, "under").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`under` is required"))?;
    let root = warehouse_root(cmd)?;
    let results = warehouse::models(&root, under).map_err(|e| bad(cmd, e.to_string()))?;

    let mut models = Vec::new();
    let mut errors = Vec::new();
    for (i, r) in results.into_iter().enumerate() {
        match r {
            Ok(m) => models.push(json!({ "name": m.name, "path": m.path, "icon": m.icon, "width": m.size.0, "height": m.size.1 })),
            Err(e) => errors.push(json!({ "path": format!("{under} #{i}"), "message": e.to_string() })),
        }
    }
    Ok(json!({ "models": models, "errors": errors }))
}

fn run_woodwarehousemodeldetail(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodwarehousemodeldetail";
    let path = str_param(p, "path").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`path` is required"))?;
    let root = warehouse_root(cmd)?;
    let d = warehouse::model_detail(&root, path).map_err(|e| bad(cmd, e.to_string()))?;
    let outline: Vec<Value> = d.outline.iter().map(|[x, y, b]| json!([x, y, b])).collect();
    Ok(json!({ "name": d.name, "path": d.path, "icon": d.icon, "outline": outline, "modelFile": d.model_file }))
}

/// PLACE MODEL WAREHOUSE MODEL: draws the model's outline (as a closed polyline, in the
/// drawing's own units, at `at`) and converts it to a block the same way `woodpart` does,
/// then records which warehouse model it is in the component store. Geometry and JSON change in
/// this one command, so one undo step restores both.
fn run_woodwarehouseplace(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "woodwarehouseplace";
    let path = str_param(p, "path").map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad(cmd, "`path` is required"))?.to_string();
    let at = point_req(cmd, p, "at")?;

    let root = warehouse_root(cmd)?;
    let detail = warehouse::model_detail(&root, &path).map_err(|e| bad(cmd, e.to_string()))?;
    if detail.outline.len() < 2 {
        return Err(bad(cmd, "the model has no outline to place"));
    }

    let vertices: Vec<PolyVertex> = detail.outline.iter().map(|[x, y, b]| PolyVertex::with_bulge(Vec2::new(*x, *y) + at, *b)).collect();
    let outline_handle = s.add_entity(lwpoly(vertices, true))?;

    let guid = cadcraft_wood::guid::new_guid();
    let insert_handle = make_block(s, &guid, at, &[outline_handle], Keep::Convert, &detail.name)?;
    let insert_handle = match insert_handle {
        Some(h) => h,
        None => insert(s, &guid, at, 1.0, 0.0, &serde_json::Map::new())?,
    };

    components::update(s.doc_mut()?, |c| {
        c.components.insert(guid.clone(), Component { model: path.clone(), name: detail.name.clone() });
    })
    .map_err(|e| EngineError::Other(e.to_string()))?;

    Ok(json!({ "component": guid, "insert": insert_handle.hex() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warehouse_root_fails_cleanly_without_a_config_dir() {
        // Smoke test only: real folder resolution and placement are covered end to end in
        // `crates/engine/tests/wood_warehouse.rs`, which sets `WOODCRAFT_CONFIG_DIR`.
        let _ = warehouse_root("woodwarehousefolders");
    }
}
