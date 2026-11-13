//! The commands, by area. Each is a parameter struct implementing
//! [`crate::Command`]; [`crate::catalog`] lists them all.

pub mod analytical;
pub mod annotate;
pub mod bracing;
pub mod browser;
pub mod cable_tray;
pub mod cabling;
pub mod cad;
pub mod cad_explode;
pub mod callout;
pub mod component;
pub mod component_connector;
pub mod component_piece;
pub mod component_script;
pub mod connection;
pub mod create;
pub mod curtain;
pub mod design_options;
pub mod detail;
pub mod detail_component;
pub mod devices;
pub mod doors;
pub mod edit;
pub mod fill_patterns;
pub mod foundation;
pub mod framing;
pub mod graph;
pub mod graphics;
pub mod grid_frame;
pub mod io;
pub mod level_mark;
pub mod levels;
pub mod lightning;
pub mod line_style;
pub mod mass;
pub mod materials;
pub mod mep;
pub mod modify;
pub mod pad;
pub mod paint;
pub mod phases;
pub mod profiles;
pub mod project;
pub mod query;
pub mod ramp;
pub mod rebar;
pub mod region;
pub mod roofing;
pub mod shaft;
pub mod sheets;
pub mod site;
pub mod sketch;
pub mod stair;
pub mod steel_building;
pub mod steel_connection;
pub mod structure;
pub mod structure_material;
pub mod sweep;
pub mod topography;
pub mod types;
pub mod valve;
pub mod views;
pub mod wall_framing;

use crate::error::{EngineError, Result};
use crate::session::Session;
use cc_core::{Category, EditCtx, Element, ElementId, ElementKind, ElementTypeId, LevelId, Project};
use serde_json::{Value, json};

/// Run an edit as one undo step, kept only if it succeeds and leaves a
/// project that needs no repair (no dangling references, no invalid numbers).
pub(crate) fn edit<R>(s: &mut Session, tx: String, f: impl FnOnce(&mut EditCtx) -> Result<R>) -> Result<R> {
    let key = s.take_merge_key();
    s.doc.try_edit_merged(tx, key, f, check)
}

/// The project is consistent: repairing a copy would change nothing.
pub(crate) fn check(p: &Project) -> Result<()> {
    match p.repairs_needed() {
        0 => Ok(()),
        n => Err(EngineError::rejected(format!("the edit would leave the model invalid ({n} problems)"))),
    }
}

/// Add an element (doors and windows get the next free mark).
pub(crate) fn add_element(e: &mut EditCtx, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> ElementId {
    let id = ElementId(e.alloc_id());
    let mut element = Element {
        id,
        level,
        type_id,
        mark: String::new(),
        comments: String::new(),
        kind,
        uid: Default::default(),
        phase_created: Default::default(),
        phase_demolished: None,
        design_option: None,
        paints: Vec::new(),
    };
    if matches!(element.category(), Category::Door | Category::Window) {
        element.mark = e.project().next_mark(element.category());
    }
    e.add_element(element)
}

/// The fields of an enum value's single variant: `{"Wall": {...}}` gives
/// `("Wall", {...})`; unit variants give an empty object.
pub(crate) fn variant(v: Value) -> (String, Value) {
    match v {
        Value::Object(o) if o.len() == 1 => match o.into_iter().next() {
            Some((k, inner)) => (k, inner),
            None => (String::new(), json!({})),
        },
        Value::String(name) => (name, json!({})),
        other => (String::new(), other),
    }
}

/// Whether `v` is an enum value in serde's form: one key, a variant name.
fn variant_name(v: &serde_json::Map<String, Value>) -> Option<&str> {
    let mut keys = v.keys();
    match (keys.next(), keys.next()) {
        (Some(k), None) if k.starts_with(|c: char| c.is_ascii_uppercase()) => Some(k),
        _ => None,
    }
}

/// Merge patch: objects merge key by key, anything else (arrays, numbers,
/// `null` for an optional value) replaces. Unlike RFC 7396, `null` sets a
/// field to null rather than removing it (model fields are never absent).
/// An enum value given as another variant (`{"Round": ..}` over
/// `{"Rectangular": ..}`) replaces the old one.
pub(crate) fn merge(target: &mut Value, patch: &Value) {
    if let (Value::Object(t), Value::Object(p)) = (&*target, patch)
        && let (Some(a), Some(b)) = (variant_name(t), variant_name(p))
        && a != b
    {
        *target = patch.clone();
        return;
    }
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                merge(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (t, p) => *t = p.clone(),
    }
}

/// Most fields a patch may have to clear (bounds the search on hostile input).
const MAX_CLEARED: usize = 256;

/// A merge patch from `old` to `new` (two JSON forms of one value, which
/// leave out empty fields): `new` itself, plus a value clearing each field
/// `old` has and `new` leaves out. `check` merges a whole patch into the
/// old value and tells whether the result is valid (`Some`) and is the new
/// value (`true`). Each missing field takes the first of `null`, `false`,
/// `[]`, `""` and `0` that keeps the patch valid, or, for an object, its
/// own fields cleared the same way. When that does not give `new` (a field
/// cannot be cleared this way), the patch is `new` as before.
pub(crate) fn clearing_patch(old: &Value, new: &Value, check: &dyn Fn(&Value) -> Option<bool>) -> Value {
    let mut patch = new.clone();
    if check(&patch) == Some(true) {
        return patch;
    }
    // Paths (keys from the root) present in `old` and missing from `new`.
    let mut todo: std::collections::VecDeque<Vec<String>> = std::collections::VecDeque::new();
    missing(old, new, &mut Vec::new(), &mut todo);
    let mut done = 0;
    while let Some(path) = todo.pop_front() {
        done += 1;
        if done > MAX_CLEARED {
            break;
        }
        let candidates = [Value::Null, Value::Bool(false), json!([]), json!(""), json!(0)];
        let mut set = false;
        for c in candidates {
            if put(&mut patch, &path, c) && check(&patch).is_some() {
                set = true;
                break;
            }
        }
        if set {
            continue;
        }
        // An object (a struct with defaults): clear its own fields.
        let Some(Value::Object(fields)) = lookup(old, &path) else { continue };
        if put(&mut patch, &path, json!({})) {
            for k in fields.keys() {
                let mut sub = path.clone();
                sub.push(k.clone());
                todo.push_back(sub);
            }
        }
    }
    if check(&patch) == Some(true) { patch } else { new.clone() }
}

/// Collect the paths of fields `old` has and `new` leaves out, looking into
/// objects both have.
fn missing(old: &Value, new: &Value, path: &mut Vec<String>, out: &mut std::collections::VecDeque<Vec<String>>) {
    let (Value::Object(o), Value::Object(n)) = (old, new) else { return };
    // An enum given as another variant replaces the old one whole.
    if let (Some(a), Some(b)) = (variant_name(o), variant_name(n))
        && a != b
    {
        return;
    }
    for (k, v) in o {
        if out.len() >= MAX_CLEARED || path.len() >= 32 {
            return;
        }
        path.push(k.clone());
        match n.get(k) {
            None => out.push_back(path.clone()),
            Some(nv) => missing(v, nv, path, out),
        }
        path.pop();
    }
}

fn lookup<'a>(v: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(v, |v, k| v.get(k))
}

/// Set the value at `path` in `v`, creating objects on the way; false when
/// something on the way is not an object.
fn put(v: &mut Value, path: &[String], value: Value) -> bool {
    let Some((last, parents)) = path.split_last() else { return false };
    let mut cur = v;
    for k in parents {
        let Value::Object(o) = cur else { return false };
        cur = o.entry(k.clone()).or_insert_with(|| json!({}));
    }
    let Value::Object(o) = cur else { return false };
    o.insert(last.clone(), value);
    true
}

/// Apply a merge patch to the fields of an enum value (keeping its variant).
pub(crate) fn patch_variant<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T, patch: &Value) -> Result<T> {
    let (name, mut fields) = variant(serde_json::to_value(value).map_err(|e| EngineError::invalid(e.to_string()))?);
    merge(&mut fields, patch);
    let rebuilt = if fields.as_object().is_some_and(|o| o.is_empty()) && !patch.is_object() {
        Value::String(name)
    } else {
        json!({ name: fields })
    };
    serde_json::from_value(rebuilt).map_err(|e| EngineError::invalid(format!("invalid params: {e}")))
}

/// Apply a merge patch to a whole serializable value.
pub(crate) fn patch_value<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T, patch: &Value) -> Result<T> {
    let mut v = serde_json::to_value(value).map_err(|e| EngineError::invalid(e.to_string()))?;
    merge(&mut v, patch);
    serde_json::from_value(v).map_err(|e| EngineError::invalid(format!("invalid values: {e}")))
}

/// What a created or changed element looks like now.
pub(crate) fn created(s: &mut Session, id: ElementId) -> Value {
    query::element_json(s, id).unwrap_or_else(|| json!({ "id": id.0 }))
}

pub(crate) fn ids_json(ids: &[ElementId]) -> Value {
    json!(ids.iter().map(|i| i.0).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
    struct Inner {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pick: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        on: bool,
    }

    impl Inner {
        fn is_empty(&self) -> bool {
            *self == Self::default()
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum Thing {
        Shape {
            size: f64,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            label: Option<String>,
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            holes: Vec<u32>,
            #[serde(default, skip_serializing_if = "String::is_empty")]
            note: String,
            #[serde(default, skip_serializing_if = "Inner::is_empty")]
            inner: Inner,
        },
    }

    fn patch(old: &Thing, new: &Thing) -> Value {
        let check = |p: &Value| patch_variant(old, p).ok().map(|v| v == *new);
        clearing_patch(&crate::params_of(old), &crate::params_of(new), &check)
    }

    /// Issue #225: fields that go back to their empty value are left out of
    /// the new value's JSON; the patch must clear them, whatever their kind.
    #[test]
    fn a_clearing_patch_empties_left_out_fields() {
        let old = Thing::Shape {
            size: 1.0,
            label: Some("a".into()),
            holes: vec![1, 2],
            note: "n".into(),
            inner: Inner { pick: Some(3), on: true },
        };
        let new =
            Thing::Shape { size: 2.0, label: None, holes: Vec::new(), note: String::new(), inner: Inner::default() };
        // The plain JSON of the new value keeps the old fields when merged.
        assert_ne!(patch_variant(&old, &crate::params_of(&new)).ok(), Some(new.clone()));
        let p = patch(&old, &new);
        assert_eq!(patch_variant(&old, &p).ok(), Some(new.clone()), "{p}");
        // Only part of a nested struct.
        let shape = |size: f64, inner: Inner| Thing::Shape {
            size,
            label: Some("a".into()),
            holes: vec![1, 2],
            note: "n".into(),
            inner,
        };
        let partly = shape(1.0, Inner { pick: None, on: true });
        assert_eq!(patch_variant(&old, &patch(&old, &partly)).ok(), Some(partly));
        // Nothing to clear: the new value's own JSON.
        let grown = shape(5.0, Inner { pick: Some(3), on: true });
        assert_eq!(patch(&old, &grown), crate::params_of(&grown));
    }
}
