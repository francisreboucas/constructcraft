//! The analytical model of the structure, for structural analysis programs.

use crate::error::{EngineError, Result};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use cc_core::{ElementKind, TypeData};
use cc_geom::analytical::{MemberKind, analytical_model};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Format version of the analytical model document.
/// Version 2: a member's `section` holds its profile and properties.
/// Version 3: a curved member (part of an arc beam) has an `arc`, and its
/// `length` is the arc's; straight members are written as before.
/// Version 4: a support node's `base_support` and `restraint`, the
/// `materials` with their mechanical properties, a section's `shape` and
/// profile `material`, the `axes` of the section properties and every unit;
/// a column top that only beams frame into ends on their axis.
const FORMAT: u32 = 4;

/// Most straight members `arc_segments` may split an arc member into.
const MAX_ARC_SEGMENTS: u32 = 360;

/// What the section properties and restraints refer to, written into the
/// document so programs need not guess.
const AXES: &str = "Global axes: x east, y north, z up. Restraints are [ux, uy, uz, rx, ry, rz] in global axes. Section axes: x across the section, along its width, y up the section, along its height; ix and iy are the second moments about them. A beam's section x axis is horizontal, square to the beam; a column's is the plan x axis turned by the column's rotation (counter-clockwise, seen from above).";

/// The analytical model as JSON: nodes (`id`, `x`, `y`, `z` in meters,
/// `support` at column bases, with its `base_support` and `restraint`) and members (`element`, `kind`, `start` and
/// `end` nodes, `length`, `type`, `section` and `material`, and `arc` for a
/// curved member). A section is
/// the `profile`'s name (null for a type drawn from its own section), the
/// type's `width` and `height` (m, the profile's box), its `area` (m²), second moments `ix`, `iy` and torsion constant `j` (m⁴), and
/// a column's `rotation` (degrees). An arc is the member's plan arc from its
/// start node to its end node: `bulge` (`tan(sweep / 4)`), `center` (`x`, `y`,
/// and `z` midway between the ends), `radius` (m) and `sweep` (degrees,
/// positive counter-clockwise seen from above). `arc_segments` splits each
/// curved member into that many straight ones.
fn model_json(s: &mut Session, arc_segments: Option<u32>) -> Result<Value> {
    if let Some(n) = arc_segments.filter(|n| !(1..=MAX_ARC_SEGMENTS).contains(n)) {
        return Err(EngineError::invalid(format!("arc_segments must be between 1 and {MAX_ARC_SEGMENTS}, not {n}")));
    }
    let geom = s.geometry().clone();
    let p = s.project();
    let mut model = analytical_model(p, &geom);
    if let Some(n) = arc_segments {
        model.split_arcs(n as usize);
    }
    let nodes: Vec<Value> = model
        .nodes
        .iter()
        .map(|n| {
            let mut out = json!({"id": n.id, "x": n.position.x, "y": n.position.y, "z": n.position.z, "support": n.support.is_some()});
            if let (Some(support), Some(o)) = (n.support, out.as_object_mut()) {
                o.insert("base_support".into(), json!(support.code()));
                o.insert("restraint".into(), json!(support.restraint()));
            }
            out
        })
        .collect();
    let mut material_names: Vec<String> = Vec::new();
    let members: Vec<Value> = model
        .members
        .iter()
        .filter_map(|m| {
            let (a, b) = model.line(m)?;
            let e = p.element(m.element)?;
            let ty = p.element_type(e.type_id)?;
            let section = match super::profiles::section(p, &ty.data) {
                Some(sec) => {
                    let props = &sec.properties;
                    let (width, height) = match &ty.data {
                        TypeData::Column { section, .. } => section.size(),
                        TypeData::Beam { width, height, .. } => (*width, *height),
                        _ => (0.0, 0.0),
                    };
                    let def = p.type_section(&ty.data).and_then(|t| t.profile).and_then(|id| p.profile(id));
                    let mut out = json!({
                        "profile": sec.profile,
                        "shape": def.map(|f| f.shape.code()),
                        "material": def.map(|f| f.material.code()),
                        "width": width,
                        "height": height,
                        "area": props.area,
                        "ix": props.ix,
                        "iy": props.iy,
                        "j": props.j,
                    });
                    if let (TypeData::Column { .. }, ElementKind::Column(c), Some(o)) =
                        (&ty.data, &e.kind, out.as_object_mut())
                    {
                        o.insert("rotation".into(), json!(c.rotation_deg));
                    }
                    out
                }
                None => Value::Null,
            };
            let material = ty.main_material();
            if !material.is_empty() && !material_names.iter().any(|n| n == material) {
                material_names.push(material.to_string());
            }
            let kind = match m.kind {
                MemberKind::Column => "column",
                MemberKind::Beam => "beam",
            };
            let mut out = json!({
                "id": m.id,
                "element": m.element.0,
                "uid": e.uid.to_string(),
                "kind": kind,
                "start": m.start,
                "end": m.end,
                "length": model.length(m)?,
                "type": ty.name,
                "section": section,
                "material": material,
            });
            if let (Some(arc), Some(o)) = (model.arc(m), out.as_object_mut()) {
                let center = json!({"x": arc.center.x, "y": arc.center.y, "z": (a.z + b.z) * 0.5});
                o.insert(
                    "arc".into(),
                    json!({"bulge": m.bulge, "center": center, "radius": arc.radius, "sweep": arc.sweep.to_degrees()}),
                );
            }
            Some(out)
        })
        .collect();
    Ok(json!({
        "format": "constructcad-analytical",
        "version": FORMAT,
        "project": p.info.name,
        "units": {
            "length": "m", "angle": "deg", "area": "m2", "inertia": "m4",
            "modulus": "Pa", "density": "kg/m3",
        },
        "axes": AXES,
        "nodes": nodes,
        "members": members,
        "materials": material_names.iter().map(|name| material_json(p, name)).collect::<Vec<_>>(),
    }))
}

/// A member material and its mechanical properties (null where the material
/// has none): `class`, Young's `modulus` and `shear_modulus` (Pa, the shear
/// modulus from the modulus and `poisson` of an isotropic material), `poisson`,
/// `density` (kg/m³) and `yield_strength` and `compressive_strength` (Pa).
fn material_json(p: &cc_core::Project, name: &str) -> Value {
    let Some(m) = p.material(name) else {
        return json!({"name": name, "class": null, "modulus": null, "shear_modulus": null, "poisson": null,
            "density": null, "yield_strength": null, "compressive_strength": null});
    };
    let ph = &m.physical;
    let positive = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0);
    let mpa = |v: Option<f64>| positive(v).map(|v| v * 1e6);
    let modulus = mpa(ph.youngs_modulus);
    let poisson = ph.poisson.filter(|v| v.is_finite() && *v > -1.0 && *v < 0.5);
    let shear = match (modulus, poisson, ph.behavior) {
        (Some(e), Some(nu), cc_core::material::Behavior::Isotropic) => Some(e / (2.0 * (1.0 + nu))),
        _ => None,
    };
    json!({
        "name": name,
        "class": m.identity.class,
        "modulus": modulus,
        "shear_modulus": shear,
        "poisson": poisson,
        "density": positive(ph.density),
        "yield_strength": mpa(ph.yield_strength),
        "compressive_strength": mpa(ph.compressive_strength),
    })
}

/// The analytical model of the structure: structural columns and beams as
/// members along their center lines between shared nodes. Beams framed into
/// columns end on the column axis, beams ending on beams end on their line,
/// and members are split where others connect; a column top that only beams
/// frame into ends on their axis. Support nodes (column bases) carry their
/// `base_support` (`fixed` or `pinned`, the column's) and `restraint`
/// (`[ux, uy, uz, rx, ry, rz]`, true where held). Each member carries its
/// `section`: the profile's name, `shape` and `material`, `width` and `height` (m), `area` (m²), `ix`, `iy` and `j` (m⁴),
/// about the section axes the document's `axes` describes.
/// The members of an arc beam follow its arc: each has an `arc` (`bulge`,
/// `center`, `radius`, `sweep` in degrees) and the arc's `length`.
/// `materials` lists the members' materials with their `modulus`,
/// `shear_modulus`, `poisson`, `density` and strengths (SI units, null where unknown).
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalModelGet {
    /// Split each curved member (part of an arc beam) into this many straight
    /// members, between new nodes on its arc, for programs that take only
    /// straight members (1 to 360; default: keep the arcs).
    #[serde(default)]
    pub arc_segments: Option<u32>,
}

impl Command for AnalyticalModelGet {
    const NAME: &'static str = "analytical_model";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        model_json(s, self.arc_segments)
    }
}

/// Write the analytical model (as `analytical_model` returns it) to a JSON file.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalyticalExport {
    /// Path of the .json file to write.
    pub path: PathBuf,
    /// Split each curved member (part of an arc beam) into this many straight
    /// members, between new nodes on its arc, for programs that take only
    /// straight members (1 to 360; default: keep the arcs).
    #[serde(default)]
    pub arc_segments: Option<u32>,
}

impl Command for AnalyticalExport {
    const NAME: &'static str = "analytical_export";
    const KIND: CommandKind = CommandKind::File;
    const CHECKS_PATHS: bool = true;
    fn run(self, s: &mut Session) -> Result<Value> {
        s.require_write("path", &self.path)?;
        let model = model_json(s, self.arc_segments)?;
        let text = serde_json::to_string_pretty(&model).map_err(|e| EngineError::invalid(e.to_string()))?;
        std::fs::write(&self.path, text).map_err(|e| EngineError::io(e.to_string()))?;
        let count = |k: &str| model[k].as_array().map_or(0, Vec::len);
        Ok(json!({"path": self.path, "nodes": count("nodes"), "members": count("members")}))
    }
}
