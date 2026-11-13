//! Steel connection macros: recognizing structural steel joints (columns,
//! beams, rafters, trusses and bracings) and placing parametric scripted
//! connection components (`base_plate`, `end_plate`, `single_plate`,
//! `double_angle`, `gusset_plate`, `splice`).

use super::{add_element, edit, ids_json};
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref, finite, positive};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::properties::{Atom, IfcValue, Property, PropertyValue};
use cc_core::steel_connection::{
    CONNECTION_PSET, ConnectionKind, ConnectionParams, RecognizedConnection, connection_component_def, recognize_all,
    recognize_for_members,
};
use cc_core::{
    Component, EditCtx, ElementId, ElementKind, ElementType, ElementTypeId, MaterialClass, Project, TypeData, TypeInfo,
    Uid,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Find the steel material name in `project` for connection plates and bolts.
fn steel_material(project: &Project) -> String {
    project
        .materials
        .iter()
        .find(|m| m.identity.class == MaterialClass::Metal)
        .or_else(|| project.materials.first())
        .map_or_else(String::new, |m| m.name.clone())
}

/// Find or create the scripted component type for `kind` in `ctx`.
fn ensure_connection_type(ctx: &mut EditCtx, kind: ConnectionKind) -> ElementTypeId {
    let target_name = kind.component_name();
    if let Some(existing) = ctx.project().types.values().find(|t| {
        t.name.eq_ignore_ascii_case(target_name)
            || matches!(&t.data, TypeData::Component { component, .. } if component.eq_ignore_ascii_case(target_name))
    }) {
        return existing.id;
    }
    let mat = steel_material(ctx.project());
    let mut def = connection_component_def(kind, &mat);
    if let Some(cc_core::ComponentShape::Script { source }) = def.items.first().map(|i| &i.shape)
        && let Ok(decl) = cc_script::declare(source)
    {
        let _ = cc_script::object::sync(&mut def.params, &mut def.hotspots, &decl);
    }
    let type_id = ElementTypeId(ctx.alloc_id());
    ctx.add_type(ElementType {
        id: type_id,
        name: target_name.to_string(),
        data: TypeData::Component { component: target_name.to_string(), def },
        info: TypeInfo { description: kind.description().to_string(), ..Default::default() },
        uid: Uid::default(),
    })
}

/// Whether `project` already has a connection component of `kind` for `members`.
fn connection_exists(project: &Project, kind: ConnectionKind, members: &[ElementId]) -> bool {
    let mut wanted: Vec<u64> = members.iter().map(|id| id.0).collect();
    wanted.sort_unstable();
    for e in project.elements.values() {
        let ElementKind::Component(c) = &e.kind else { continue };
        let Some(pset) = c.properties.iter().find(|p| p.name == CONNECTION_PSET) else { continue };
        let existing_kind = pset.properties.iter().find_map(|Property { name, value }| match (name.as_str(), value) {
            ("Kind", PropertyValue::Single(Some(IfcValue { value: Atom::Text(s), .. }))) => {
                ConnectionKind::from_code(s)
            }
            _ => None,
        });
        if existing_kind != Some(kind) {
            continue;
        }
        let existing_members: Option<Vec<u64>> =
            pset.properties.iter().find_map(|Property { name, value }| match (name.as_str(), value) {
                ("Members", PropertyValue::Single(Some(IfcValue { value: Atom::Text(s), .. }))) => {
                    let mut ids: Vec<u64> = s.split(',').filter_map(|x| x.trim().parse::<u64>().ok()).collect();
                    ids.sort_unstable();
                    Some(ids)
                }
                _ => None,
            });
        if existing_members.as_deref() == Some(wanted.as_slice()) {
            return true;
        }
    }
    false
}

/// Place a recognized connection in `ctx` and return `(element_id, type_id, json_summary)`.
fn place_recognized(ctx: &mut EditCtx, rc: &RecognizedConnection) -> (ElementId, ElementTypeId, Value) {
    let type_id = ensure_connection_type(ctx, rc.kind);
    let primary_sec = rc
        .members
        .first()
        .and_then(|id| ctx.project().element(*id))
        .and_then(|e| cc_core::steel_connection::member_section(ctx.project(), e))
        .unwrap_or_default();
    let secondary_sec = rc
        .members
        .get(1)
        .and_then(|id| ctx.project().element(*id))
        .and_then(|e| cc_core::steel_connection::member_section(ctx.project(), e));
    let (overrides, text_overrides) = rc.spec.instance_overrides(primary_sec, secondary_sec);
    let pset = rc.spec.property_set(&rc.members);
    let level_z = ctx.project().level_elevation(rc.level);
    let comp = Component {
        position: rc.position.truncate(),
        rotation_deg: rc.rotation_deg,
        base_offset: rc.position.z - level_z,
        overrides,
        properties: vec![pset],
        system: None,
        text_overrides,
        hotspots: Default::default(),
    };
    let id = add_element(ctx, rc.level, type_id, ElementKind::Component(comp));
    let info = json!({
        "id": id.0,
        "type_id": type_id.0,
        "kind": rc.kind.code(),
        "members": ids_json(&rc.members),
        "position": [rc.position.x, rc.position.y, rc.position.z],
        "rotation_deg": rc.rotation_deg,
        "spec": {
            "plate_thickness": rc.spec.plate_thickness,
            "plate_width": rc.spec.plate_width,
            "plate_height": rc.spec.plate_height,
            "bolt_diameter": rc.spec.bolt_diameter,
            "bolt_grade": rc.spec.bolt_grade,
            "rows": rc.spec.rows,
            "cols": rc.spec.cols,
            "pitch": rc.spec.pitch,
            "gauge": rc.spec.gauge,
            "edge_distance": rc.spec.edge_distance,
            "extension": rc.spec.extension,
            "stiffeners": rc.spec.stiffeners,
            "base_mode": rc.spec.base_mode,
            "anchor_shape": rc.spec.anchor_shape,
            "anchors": rc.spec.anchors,
            "anchor_length": rc.spec.anchor_length,
            "grout_thickness": rc.spec.grout_thickness,
            "gap": rc.spec.gap,
            "weld_throat": rc.spec.weld_throat,
            "weld_length": rc.spec.weld_length,
            "weight_kg": rc.spec.weight_kg,
        }
    });
    (id, type_id, info)
}

/// List the available steel connection macros (`base_plate`, `end_plate`,
/// `single_plate`, `double_angle`, `gusset_plate`, `splice`) and, when
/// `elements` or `level` are given, recognize the candidate joints among those
/// structural members.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SteelConnectionMacros {
    /// Element ids to inspect for candidate joints (columns, beams, trusses, braces).
    #[serde(default)]
    pub elements: Vec<u64>,
    /// Level id or name to inspect when `elements` is empty.
    #[serde(default)]
    pub level: Option<Ref>,
}

impl Command for SteelConnectionMacros {
    const NAME: &'static str = "steel_connection_macros";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let macros: Vec<Value> = ConnectionKind::ALL
            .into_iter()
            .map(|k| {
                json!({
                    "kind": k.code(),
                    "name": k.component_name(),
                    "description": k.description(),
                })
            })
            .collect();

        let expanded: Vec<ElementId> = if self.elements.is_empty() {
            Vec::new()
        } else {
            let raw = refs::elements(p, &self.elements)?;
            p.with_groups(raw).into_iter().collect()
        };
        let level_id = match &self.level {
            Some(r) => Some(refs::level(p, r)?),
            None => None,
        };

        let recognized: Vec<Value> = if !expanded.is_empty() || level_id.is_some() {
            recognize_all(p, &expanded, level_id, &ConnectionParams::default())
                .into_iter()
                .map(|rc| {
                    json!({
                        "kind": rc.kind.code(),
                        "members": ids_json(&rc.members),
                        "position": [rc.position.x, rc.position.y, rc.position.z],
                        "rotation_deg": rc.rotation_deg,
                        "bolt_diameter": rc.spec.bolt_diameter,
                        "plate_thickness": rc.spec.plate_thickness,
                        "rows": rc.spec.rows,
                        "cols": rc.spec.cols,
                        "weight_kg": rc.spec.weight_kg,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(json!({
            "macros": macros,
            "recognized": recognized,
        }))
    }
}

/// Create a parametric steel connection macro on the given structural members
/// (1 column for `base_plate`; 2 or more members for `end_plate`,
/// `single_plate`, `double_angle`, `gusset_plate` or `splice`). When `kind` is
/// omitted, the macro kind is automatically recognized from the member
/// geometry (column base, beam-column flange/web, beam-beam shear, splice,
/// truss node or bracing gusset).
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SteelConnectionCreate {
    /// Connected structural member ids (columns, beams, rafters, truss chords/diagonals, braces).
    pub elements: Vec<u64>,
    /// Connection macro kind: `base_plate`, `end_plate`, `single_plate`, `double_angle`, `gusset_plate` or `splice` (default: auto-detected).
    #[serde(default)]
    pub kind: Option<String>,
    /// Plate thickness override in meters.
    #[serde(default)]
    pub plate_thickness: Option<f64>,
    /// Bolt or anchor diameter override in meters (e.g. `0.016`, `0.020`, `0.024`).
    #[serde(default)]
    pub bolt_diameter: Option<f64>,
    /// Bolt steel grade (`"8.8"`, `"10.9"`, `"A325"`, `"A490"`).
    #[serde(default)]
    pub bolt_grade: Option<String>,
    /// Number of bolt rows.
    #[serde(default)]
    pub rows: Option<u32>,
    /// Number of bolt columns.
    #[serde(default)]
    pub cols: Option<u32>,
    /// Bolt vertical spacing (pitch) in meters.
    #[serde(default)]
    pub pitch: Option<f64>,
    /// Bolt horizontal gauge in meters.
    #[serde(default)]
    pub gauge: Option<f64>,
    /// Bolt edge distance in meters.
    #[serde(default)]
    pub edge_distance: Option<f64>,
    /// Top extension of an extended end plate in meters (`0` for flush).
    #[serde(default)]
    pub extension: Option<f64>,
    /// Add stiffeners (column web continuity plates or fixed base ribs).
    #[serde(default)]
    pub stiffeners: Option<bool>,
    /// Base plate mode: `"pinned"` or `"fixed"`.
    #[serde(default)]
    pub base_mode: Option<String>,
    /// Anchor bolt end shape: `"hooked"`, `"plate"` or `"straight"`.
    #[serde(default)]
    pub anchor_shape: Option<String>,
    /// Number of anchor bolts on a base plate (`2`, `4`, `6` or `8`).
    #[serde(default)]
    pub anchors: Option<u32>,
    /// Anchor embedment length in meters.
    #[serde(default)]
    pub anchor_length: Option<f64>,
    /// Grout pad thickness under a base plate in meters.
    #[serde(default)]
    pub grout_thickness: Option<f64>,
    /// Clearance gap between supported member and support face in meters.
    #[serde(default)]
    pub gap: Option<f64>,
}

impl SteelConnectionCreate {
    fn to_params(&self) -> Result<ConnectionParams> {
        let pos = |v: Option<f64>, name: &str| v.map(|x| positive(x, name)).transpose();
        let non_neg = |v: Option<f64>, name: &str| match v {
            Some(x) => {
                let x = finite(x, name)?;
                if x < 0.0 {
                    return Err(EngineError::invalid(format!("{name} must be >= 0")));
                }
                Ok(Some(x))
            }
            None => Ok(None),
        };
        Ok(ConnectionParams {
            plate_thickness: pos(self.plate_thickness, "plate_thickness")?,
            bolt_diameter: pos(self.bolt_diameter, "bolt_diameter")?,
            bolt_grade: self.bolt_grade.clone(),
            rows: self.rows,
            cols: self.cols,
            pitch: pos(self.pitch, "pitch")?,
            gauge: pos(self.gauge, "gauge")?,
            edge_distance: pos(self.edge_distance, "edge_distance")?,
            extension: non_neg(self.extension, "extension")?,
            stiffeners: self.stiffeners,
            base_mode: self.base_mode.clone(),
            anchor_shape: self.anchor_shape.clone(),
            anchors: self.anchors,
            anchor_length: pos(self.anchor_length, "anchor_length")?,
            grout_thickness: non_neg(self.grout_thickness, "grout_thickness")?,
            gap: non_neg(self.gap, "gap")?,
        })
    }
}

impl Command for SteelConnectionCreate {
    const NAME: &'static str = "steel_connection_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        if self.elements.is_empty() {
            return Err(EngineError::invalid("give at least one structural element in `elements`"));
        }
        let ids = refs::elements(p, &self.elements)?;
        let explicit_kind = match &self.kind {
            Some(k) => Some(ConnectionKind::from_code(k).ok_or_else(|| {
                EngineError::invalid(format!(
                    "unknown connection kind '{k}' (use base_plate, end_plate, single_plate, double_angle, gusset_plate or splice)"
                ))
            })?),
            None => None,
        };
        if explicit_kind.is_some_and(|k| k != ConnectionKind::BasePlate) && ids.len() < 2 {
            return Err(EngineError::invalid("this connection macro requires at least 2 connected members"));
        }
        let params = self.to_params()?;
        let rc = recognize_for_members(p, &ids, explicit_kind, &params)
            .ok_or_else(|| EngineError::invalid("the given elements are not structural columns or beams"))?;

        let (_, _, info) = edit(s, t!("tx-create-steel-connection"), |ctx| Ok(place_recognized(ctx, &rc)))?;
        Ok(info)
    }
}

/// Automatically recognize and place typical steel connections (`base_plate`,
/// `end_plate`, `single_plate`, `gusset_plate`, `splice`) across the given
/// `elements` (columns, beams, trusses and bracings), or across `level` (or
/// the whole project when neither is given). Existing connections are skipped.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SteelConnectionsAuto {
    /// Structural element ids to connect (default: all steel members on `level` or in the project).
    #[serde(default)]
    pub elements: Vec<u64>,
    /// Level id or name to scan when `elements` is empty.
    #[serde(default)]
    pub level: Option<Ref>,
    /// Default base restraint mode for column bases (`"pinned"` or `"fixed"`).
    #[serde(default)]
    pub base_mode: Option<String>,
    /// Default bolt steel grade (`"8.8"`, `"10.9"`, `"A325"`, `"A490"`).
    #[serde(default)]
    pub bolt_grade: Option<String>,
    /// Whether end plates and fixed bases include stiffeners by default.
    #[serde(default)]
    pub stiffeners: Option<bool>,
}

impl Command for SteelConnectionsAuto {
    const NAME: &'static str = "steel_connections_auto";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let expanded: Vec<ElementId> = if self.elements.is_empty() {
            Vec::new()
        } else {
            let raw = refs::elements(p, &self.elements)?;
            p.with_groups(raw).into_iter().collect()
        };
        let level_id = match &self.level {
            Some(r) => Some(refs::level(p, r)?),
            None => None,
        };
        let params = ConnectionParams {
            base_mode: self.base_mode,
            bolt_grade: self.bolt_grade,
            stiffeners: self.stiffeners,
            ..Default::default()
        };

        let candidates = recognize_all(p, &expanded, level_id, &params);
        let mut todo = Vec::new();
        let mut skipped = 0usize;
        for rc in candidates {
            if connection_exists(p, rc.kind, &rc.members) {
                skipped += 1;
            } else {
                todo.push(rc);
            }
        }

        if todo.is_empty() {
            return Ok(json!({
                "created": [],
                "skipped": skipped,
                "connections": [],
            }));
        }

        let (created, connections) = edit(s, t!("tx-create-steel-connection"), |ctx| {
            let mut ids = Vec::with_capacity(todo.len());
            let mut list = Vec::with_capacity(todo.len());
            for rc in &todo {
                let (id, _, info) = place_recognized(ctx, rc);
                ids.push(id);
                list.push(info);
            }
            Ok((ids, list))
        })?;

        Ok(json!({
            "created": ids_json(&created),
            "skipped": skipped,
            "connections": connections,
        }))
    }
}
