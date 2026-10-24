//! Reads and writes the part JSON in the drawing's `WOODCRAFT_PARTS` XRECORD.
//!
//! The XRECORD is kept in the drawing's `foreign` data (the groups of the file that CADCraft does
//! not model), which the drawing clones with every undo snapshot and the DXF writer puts back on
//! Save. Nothing here touches the file itself.

use std::sync::Arc;

use cadcraft_doc::foreign::ForeignObject;
use cadcraft_doc::{Drawing, Handle, RawTag};

use crate::data::WoodData;

/// The key in the named object dictionary.
pub const KEY: &str = "WOODCRAFT_PARTS";
/// The handle of the named object dictionary in a drawing that was not read from a file. The
/// writer maps it to the real one.
const NOD_HANDLE: Handle = Handle(0xC);
/// A DXF string value is kept to this many characters per group, as CADCraft's own XRECORDs.
const CHUNK: usize = 250;
/// The most the part data's JSON may weigh (bytes, before escaping). A document over this is
/// refused rather than written.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum WoodError {
    #[error("the WoodCraft data is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the dictionary names the WoodCraft data but the object is missing")]
    MissingObject,
    #[error("the WoodCraft part data is {0} bytes, over the {MAX_BYTES} byte limit")]
    TooLarge(usize),
}

/// The part data of the drawing; an empty document when the drawing has none yet.
pub fn get(d: &Drawing) -> Result<WoodData, WoodError> {
    let f = &d.foreign;
    let Some((_, h)) = f.dictionary.iter().find(|(k, _)| k.eq_ignore_ascii_case(KEY)) else {
        return Ok(WoodData::default());
    };
    let obj = f.objects.iter().find(|o| o.handle == *h).ok_or(WoodError::MissingObject)?;
    let text: String = obj.tags.iter().filter(|t| t.code == 1).map(|t| t.value.as_str()).collect();
    Ok(serde_json::from_str(&text)?)
}

/// What the user should see for `block`: a wood part's display name if `block` names one,
/// otherwise `block` itself unchanged (so the GUID is never shown). Errors (e.g. corrupt part
/// data) also fall back to the block name, since a display label must never fail.
pub fn display_name(d: &Drawing, block: &str) -> String {
    get(d).ok().and_then(|w| w.parts.get(block).map(|p| p.name.clone())).unwrap_or_else(|| block.to_string())
}

/// Replace the part data of the drawing. Refuses, without changing anything, data whose JSON
/// would be over [`MAX_BYTES`].
pub fn put(d: &mut Drawing, data: &WoodData) -> Result<(), WoodError> {
    let json = serde_json::to_string(data)?;
    if json.len() > MAX_BYTES {
        return Err(WoodError::TooLarge(json.len()));
    }
    let text = ascii_json(&json);
    let chunks: Vec<RawTag> = split(&text).into_iter().map(|c| RawTag { code: 1, value: c.to_string() }).collect();

    // The object already there keeps its own groups (handle, owner, subclass); only the text goes.
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

    // First time: a new XRECORD owned by the named object dictionary.
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

/// Read, change and write back. A part whose entry changed gets its revision raised by one; a
/// new part keeps the revision it was given. Nothing is written when nothing changed.
pub fn update<R>(d: &mut Drawing, change: impl FnOnce(&mut WoodData) -> R) -> Result<R, WoodError> {
    let before = get(d)?;
    let mut after = before.clone();
    let r = change(&mut after);
    for (guid, part) in after.parts.iter_mut() {
        if let Some(old) = before.parts.get(guid) {
            let mut same = old.clone();
            same.revision = part.revision;
            if same != *part {
                part.revision = old.revision + 1;
            } else {
                part.revision = old.revision;
            }
        }
    }
    if after != before {
        put(d, &after)?;
    }
    Ok(r)
}

/// JSON with every character outside ASCII written as `\uXXXX`, so the DXF text stays plain
/// ASCII and never holds the `\U+` sequence the DXF reader would decode.
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

/// Pieces of at most [`CHUNK`] characters of ASCII text, none starting or ending with a space
/// (a reader may trim a value). Joined again they are the original text.
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
    fn split_joins_back_and_avoids_edge_spaces() {
        let s: String = (0..400).map(|i| if i % 7 == 0 { ' ' } else { 'x' }).collect();
        let s = format!("{{{s}}}");
        let parts = split(&s);
        assert_eq!(parts.concat(), s);
        for p in &parts {
            assert!(p.len() <= CHUNK);
            assert!(!p.starts_with(' ') && !p.ends_with(' '));
        }
    }

    #[test]
    fn non_ascii_becomes_escapes() {
        let j = ascii_json("{\"name\":\"Tête 🪵\"}");
        assert!(j.is_ascii());
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["name"], "Tête 🪵");
    }

    #[test]
    fn oversized_part_data_is_refused_and_nothing_is_written() {
        use crate::data::Part;

        let mut d = Drawing::new_imperial();
        let mut data = WoodData::default();
        let mut big = Part::new("Big", 1.0);
        // A single `other` entry well over MAX_BYTES once serialized.
        big.other.insert("Filler".into(), serde_json::Value::String("x".repeat(MAX_BYTES + 1024)));
        data.parts.insert("3f2a9c10-0000-4000-8000-000000000099".into(), big);

        let err = put(&mut d, &data).unwrap_err();
        assert!(matches!(err, WoodError::TooLarge(_)), "expected TooLarge, got {err:?}");
        assert!(get(&d).unwrap().parts.is_empty(), "nothing was written to the drawing");
    }
}
