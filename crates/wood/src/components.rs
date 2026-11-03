//! WoodCraft Phase 10a: tracks placed Model Warehouse components (hinges, knobs, slides, and
//! the like — bought hardware, not a wood part being cut, so it never joins
//! [`crate::data::WoodData::parts`]/`WOODCRAFT_PARTS`). Stored the same way, in its own XRECORD
//! under the key `WOODCRAFT_COMPONENTS` in the drawing's named object dictionary
//! (`plans/WoodCraft_Decisions.md` item 25): JSON, chunked into [`crate::store`]-sized groups,
//! kept in the drawing's `foreign` data so undo, redo, copy-on-write and Save carry it with the
//! geometry, exactly as [`crate::store`] does for parts. Nothing here touches the file itself.

use std::sync::Arc;

use cadcraft_doc::foreign::ForeignObject;
use cadcraft_doc::{Drawing, Handle, RawTag};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The key in the named object dictionary.
pub const KEY: &str = "WOODCRAFT_COMPONENTS";
/// The handle of the named object dictionary in a drawing that was not read from a file. The
/// writer maps it to the real one. Matches [`crate::store::NOD_HANDLE`] (the same dictionary
/// holds both XRECORDs).
const NOD_HANDLE: Handle = Handle(0xC);
const CHUNK: usize = 250;
/// The most the component data's JSON may weigh (bytes, before escaping).
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("the WoodCraft component data is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the dictionary names the WoodCraft component data but the object is missing")]
    MissingObject,
    #[error("the WoodCraft component data is {0} bytes, over the {MAX_BYTES} byte limit")]
    TooLarge(usize),
}

/// One placed warehouse model: which model it is, and the block name (also its own id, like a
/// part's GUID) of the insert carrying its flat outline in the drawing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    /// The warehouse model's path, relative to the warehouse root (as `warehouse::models`
    /// returns it), e.g. `"Hardware/Hinges/soss-100"`.
    pub model: String,
    /// The display name the model had when placed (kept so a later warehouse change or a
    /// missing model does not blank out what is already in the drawing).
    pub name: String,
}

/// All placed components of a drawing, by their own id (the block name of the insert).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WoodComponents {
    pub components: BTreeMap<String, Component>,
}

/// The component data of the drawing; empty when the drawing has none yet.
pub fn get(d: &Drawing) -> Result<WoodComponents, ComponentError> {
    let f = &d.foreign;
    let Some((_, h)) = f.dictionary.iter().find(|(k, _)| k.eq_ignore_ascii_case(KEY)) else {
        return Ok(WoodComponents::default());
    };
    let obj = f.objects.iter().find(|o| o.handle == *h).ok_or(ComponentError::MissingObject)?;
    let text: String = obj.tags.iter().filter(|t| t.code == 1).map(|t| t.value.as_str()).collect();
    Ok(serde_json::from_str(&text)?)
}

/// Replace the component data of the drawing. Refuses, without changing anything, data whose
/// JSON would be over [`MAX_BYTES`].
pub fn put(d: &mut Drawing, data: &WoodComponents) -> Result<(), ComponentError> {
    let json = serde_json::to_string(data)?;
    if json.len() > MAX_BYTES {
        return Err(ComponentError::TooLarge(json.len()));
    }
    let text = ascii_json(&json);
    let chunks: Vec<RawTag> = split(&text).into_iter().map(|c| RawTag { code: 1, value: c.to_string() }).collect();

    let existing = d.foreign.dictionary.iter().find(|(k, _)| k.eq_ignore_ascii_case(KEY)).map(|(_, h)| *h);
    if let Some(h) = existing
        && d.foreign.objects.iter().any(|o| o.handle == h)
    {
        let f = Arc::make_mut(&mut d.foreign);
        if let Some(o) = f.objects.iter_mut().find(|o| o.handle == h) {
            o.tags.retain(|t| t.code != 1);
            o.tags.extend(chunks);
        }
        return Ok(());
    }

    let existing_nod = d.foreign.targets.iter().find(|(_, k)| k.0 == "NOD" && k.1.is_empty()).map(|(h, _)| *h);
    let nod = existing_nod.unwrap_or(NOD_HANDLE);
    let handle = d.new_handle();
    let mut tags = vec![
        RawTag { code: 5, value: handle.hex() },
        RawTag { code: 330, value: nod.hex() },
        RawTag { code: 100, value: "AcDbXrecord".into() },
        RawTag { code: 280, value: "1".into() },
    ];
    tags.extend(chunks);
    let f = Arc::make_mut(&mut d.foreign);
    if existing_nod.is_none() {
        f.targets.insert(nod, ("NOD".into(), String::new()));
    }
    f.dictionary.retain(|(k, _)| !k.eq_ignore_ascii_case(KEY));
    f.dictionary.push((KEY.to_string(), handle));
    f.objects.push(ForeignObject { handle, kind: "XRECORD".into(), tags });
    Ok(())
}

/// Read, change and write back. Nothing is written when nothing changed.
pub fn update<R>(d: &mut Drawing, change: impl FnOnce(&mut WoodComponents) -> R) -> Result<R, ComponentError> {
    let before = get(d)?;
    let mut after = before.clone();
    let r = change(&mut after);
    if after != before {
        put(d, &after)?;
    }
    Ok(r)
}

/// Same escaping [`crate::store`] uses: every character outside ASCII as `\uXXXX`.
fn ascii_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut buf = [0u16; 2];
            for u in c.encode_utf16(&mut buf) {
                out.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    out
}

/// Same chunking [`crate::store`] uses: pieces of at most [`CHUNK`] characters, none starting or
/// ending with a space.
fn split(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let mut end = (i + CHUNK).min(b.len());
        if end < b.len() {
            while end > i + 1 && (b[end - 1] == b' ' || b[end] == b' ') {
                end -= 1;
            }
        }
        out.push(&s[i..end]);
        i = end;
    }
    if out.is_empty() {
        out.push("");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_drawing_has_no_components() {
        let d = Drawing::new_imperial();
        assert!(get(&d).unwrap().components.is_empty());
    }

    #[test]
    fn put_then_get_round_trips_a_component() {
        let mut d = Drawing::new_imperial();
        let mut data = WoodComponents::default();
        data.components.insert("guid-1".into(), Component { model: "Hardware/Hinges/soss-100".into(), name: "SOSS 100".into() });
        put(&mut d, &data).unwrap();
        assert_eq!(get(&d).unwrap(), data);
    }

    #[test]
    fn update_changes_and_writes_back() {
        let mut d = Drawing::new_imperial();
        update(&mut d, |c| {
            c.components.insert("guid-1".into(), Component { model: "Hardware/Knobs/round-1".into(), name: "Round knob".into() });
        })
        .unwrap();
        assert_eq!(get(&d).unwrap().components.len(), 1);
    }

    #[test]
    fn components_and_parts_live_in_separate_records() {
        use crate::data::{Part, WoodData};
        let mut d = Drawing::new_imperial();
        crate::store::update(&mut d, |w: &mut WoodData| {
            w.parts.insert("part-1".into(), Part::new("Leg", 0.75));
        })
        .unwrap();
        update(&mut d, |c| {
            c.components.insert("comp-1".into(), Component { model: "Hardware/Hinges/soss-100".into(), name: "SOSS 100".into() });
        })
        .unwrap();
        assert_eq!(crate::store::get(&d).unwrap().parts.len(), 1);
        assert_eq!(get(&d).unwrap().components.len(), 1);
    }

    #[test]
    fn oversized_component_data_is_refused_and_nothing_is_written() {
        let mut d = Drawing::new_imperial();
        let mut data = WoodComponents::default();
        data.components.insert("guid-1".into(), Component { model: "x".repeat(MAX_BYTES + 1024), name: "Big".into() });
        let err = put(&mut d, &data).unwrap_err();
        assert!(matches!(err, ComponentError::TooLarge(_)));
        assert!(get(&d).unwrap().components.is_empty());
    }

    #[test]
    fn non_ascii_becomes_escapes() {
        let j = ascii_json("{\"name\":\"Tête\"}");
        assert!(j.is_ascii());
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["name"], "Tête");
    }
}
