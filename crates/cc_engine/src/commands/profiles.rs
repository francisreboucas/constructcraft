//! Structural profiles (see [`cc_core::structural_profile`]): the named
//! cross-sections column and beam types are drawn and built with. A profile
//! is a standard parametric shape (`i`, `channel`, `rect_tube`, `rect`…) of
//! a material (`steel`, `cold_formed`, `timber`, `concrete`) or a sketch
//! whose points are expressions over its own parameters. Profiles are
//! project settings: each command changes them in one undo step, and the
//! types using a changed profile follow it (their width and height become
//! its bounding box).

use super::edit;
use super::sketch::{CurveArg, sketch_error, sketch_of};
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::component::{ComponentParam, ParamKind};
use cc_core::structural_profile::{MAX_NAME, MAX_PROFILES, MAX_SKETCH_PARAMS, ParamSketch};
use cc_core::{
    Category, ElementType, ElementTypeId, Material, ProfileDef, ProfileId, ProfileMaterial, ProfilePoint, ProfileShape,
    Project, TypeData, Uid, profile_catalog,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// Density used for `linear_mass_steel`, kg/m³.
const STEEL_DENSITY: f64 = 7850.0;

fn material_arg(code: &str) -> Result<ProfileMaterial> {
    ProfileMaterial::from_code(code).ok_or_else(|| {
        let codes: Vec<&str> = ProfileMaterial::ALL.iter().map(|m| m.code()).collect();
        EngineError::invalid(format!("unknown profile material '{code}' ({})", codes.join(", ")))
    })
}

fn shape_arg(code: &str) -> Result<&'static str> {
    ProfileShape::CODES.iter().copied().find(|c| c.eq_ignore_ascii_case(code.trim())).ok_or_else(|| {
        EngineError::invalid(format!("unknown profile shape '{code}' ({})", ProfileShape::CODES.join(", ")))
    })
}

/// A free name: not empty, at most `MAX_NAME` characters, no other profile's.
fn profile_name(p: &Project, name: &str, current: Option<ProfileId>) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(EngineError::invalid(format!("a profile needs a name of 1 to {MAX_NAME} characters")));
    }
    if p.profile_named(name).is_some_and(|f| Some(f.id) != current) {
        return Err(EngineError::invalid(format!("a profile named '{name}' already exists")));
    }
    Ok(name.to_string())
}

fn room_for_one(p: &Project) -> Result<()> {
    if p.profiles.len() >= MAX_PROFILES {
        return Err(EngineError::rejected(format!("a project has at most {MAX_PROFILES} profiles")));
    }
    Ok(())
}

/// Puts `params` into `shape`; an unknown name lists the shape's own.
fn apply_params(shape: &mut ProfileShape, params: &BTreeMap<String, f64>) -> Result<()> {
    for (name, v) in params {
        if !shape.set_param(name, *v) {
            let names: Vec<String> = shape.params().into_iter().map(|(n, _)| n).collect();
            return Err(EngineError::invalid(format!(
                "the {} shape has no parameter '{name}' ({})",
                shape.code(),
                names.join(", ")
            )));
        }
    }
    Ok(())
}

/// The shape's outline must resolve, else the profile is refused.
fn check_shape(shape: &ProfileShape) -> Result<()> {
    shape.outline().map(|_| ()).map_err(|e| EngineError::invalid(e.to_string()))
}

/// The density of a profile material's usual material, kg/m³ (0 if none).
fn density(m: ProfileMaterial) -> f64 {
    let mut probe = Material::new("", [0, 0, 0]);
    cc_core::material::template_properties(&mut probe, m.material_class());
    probe.physical.density.unwrap_or(0.0)
}

/// The section of a column or beam type as element and analytical queries show it.
pub(crate) struct TypeSectionInfo {
    /// The profile's name; `None` for a type drawn from its own section.
    pub profile: Option<String>,
    /// Properties with the density of the type's material.
    pub properties: cc_core::section_props::SectionProperties,
    /// Whether the type's material has a usable density (else `linear_mass` is 0).
    pub has_density: bool,
}

/// The section of a column or beam type: its profile's (turned and mirrored),
/// or its own legacy section's when it has none.
pub(crate) fn section(p: &Project, data: &TypeData) -> Option<TypeSectionInfo> {
    let (TypeData::Column { material, .. } | TypeData::Beam { material, .. }) = data else { return None };
    let sec = p.type_section(data)?;
    let density = p.material(material).and_then(|m| m.physical.density).filter(|d| d.is_finite() && *d > 0.0);
    Some(TypeSectionInfo {
        profile: sec.profile.and_then(|id| p.profile(id)).map(|f| f.name.clone()),
        properties: sec.outline.properties(density.unwrap_or(0.0)),
        has_density: density.is_some(),
    })
}

fn pt(p: glam::DVec2) -> [f64; 2] {
    [p.x, p.y]
}

fn params_json(shape: &ProfileShape) -> Value {
    Value::Object(shape.params().into_iter().map(|(n, v)| (n, json!(v))).collect::<Map<String, Value>>())
}

fn type_names(p: &Project, id: ProfileId) -> Vec<String> {
    p.profile_users(id).into_iter().filter_map(|t| p.element_type(t).map(|t| t.name.clone())).collect()
}

fn profile_json(p: &Project, f: &ProfileDef) -> Value {
    let mut out = json!({
        "id": f.id.0,
        "name": f.name,
        "material": f.material.code(),
        "shape": f.shape.code(),
        "params": params_json(&f.shape),
        "catalog": f.catalog,
        "description": f.description,
        "used_by": type_names(p, f.id),
    });
    if let (Some(o), ProfileShape::Sketch(sk)) = (out.as_object_mut(), &f.shape) {
        o.insert("sketch".into(), serde_json::to_value(sk).unwrap_or(Value::Null));
    }
    if let (Some(o), Ok(outline)) = (out.as_object_mut(), f.outline()) {
        let holes: Vec<Vec<[f64; 2]>> = outline.holes.iter().map(|h| h.iter().copied().map(pt).collect()).collect();
        o.insert(
            "outline".into(),
            json!({ "outer": outline.outer.iter().copied().map(pt).collect::<Vec<_>>(), "holes": holes }),
        );
        o.insert("properties".into(), json!(outline.properties(density(f.material))));
    }
    out
}

fn profile_out(s: &Session, id: ProfileId) -> Result<Value> {
    let p = s.project();
    p.profile(id).map(|f| profile_json(p, f)).ok_or_else(|| EngineError::not_found("profile"))
}

fn created(s: &Session, id: ProfileId) -> Result<Value> {
    let f = s.project().profile(id).ok_or_else(|| EngineError::not_found("profile"))?;
    Ok(json!({ "id": id.0, "name": f.name }))
}

/// The project's structural profiles, optionally of one material or shape:
/// id, name, material, shape, area (m²), mass per meter in steel (kg/m) and
/// the number of column and beam types using each.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfilesList {
    /// Only profiles of this material: `steel`, `cold_formed`, `timber` or `concrete`.
    #[serde(default)]
    pub material: Option<String>,
    /// Only profiles of this shape code (`i`, `channel`, `rect`, `sketch`…).
    #[serde(default)]
    pub shape: Option<String>,
}

impl Command for ProfilesList {
    const NAME: &'static str = "profiles_list";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let material = self.material.as_deref().map(material_arg).transpose()?;
        let shape = self.shape.as_deref().map(shape_arg).transpose()?;
        let p = s.project();
        let rows = p
            .profiles
            .values()
            .filter(|f| material.is_none_or(|m| f.material == m) && shape.is_none_or(|c| f.shape.code() == c))
            .map(|f| {
                let area = f.outline().map(|o| o.properties(0.0).area).unwrap_or(0.0);
                json!({
                    "id": f.id.0,
                    "name": f.name,
                    "material": f.material.code(),
                    "shape": f.shape.code(),
                    "area": area,
                    "linear_mass_steel": area * STEEL_DENSITY,
                    "users": p.profile_users(f.id).len(),
                })
            })
            .collect();
        Ok(Value::Array(rows))
    }
}

/// One structural profile: its material, shape and parameters (and the
/// sketch of a sketched one), the outline (outer loop counter-clockwise,
/// holes clockwise, meters, centered on its bounding box), its section
/// properties (SI units; mass per length with the usual density of its
/// material), its catalog entry and the names of the types using it.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileGet {
    /// Profile id or name.
    pub profile: Ref,
}

impl Command for ProfileGet {
    const NAME: &'static str = "profile_get";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        let id = refs::profile(s.project(), &self.profile)?;
        profile_out(s, id)
    }
}

/// A numeric parameter of a sketched profile.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SketchParamArg {
    /// Name used in the points' expressions.
    pub name: String,
    /// Value, meters.
    pub value: f64,
}

/// Profile points with literal coordinates for a closed loop of curves.
fn loop_points(curves: &[CurveArg]) -> Result<Vec<ProfilePoint>> {
    let sketch = sketch_of(curves)?;
    let vertices = cc_core::profile::from_curves(&sketch.curves, true).map_err(sketch_error)?;
    Ok(cc_core::profile::written(&vertices))
}

fn sketch_params(args: &[SketchParamArg]) -> Result<Vec<ComponentParam>> {
    let params: Vec<ComponentParam> =
        args.iter().map(|a| ComponentParam::number(a.name.trim(), a.value, ParamKind::Length)).collect();
    check_sketch_params(&params)?;
    Ok(params)
}

/// The parameters a sketch declares, given by `profile_create` or patched by
/// `profile_set`: at most `MAX_SKETCH_PARAMS`, valid and distinct names,
/// numeric kinds and finite values.
fn check_sketch_params(params: &[ComponentParam]) -> Result<()> {
    if params.len() > MAX_SKETCH_PARAMS {
        return Err(EngineError::invalid(format!("a sketch has at most {MAX_SKETCH_PARAMS} parameters")));
    }
    for (i, p) in params.iter().enumerate() {
        let name = p.name.as_str();
        if !cc_core::expr::is_valid_name(name) {
            return Err(EngineError::invalid(format!("'{name}' is not a valid parameter name")));
        }
        if params.iter().take(i).any(|q| q.name == name) {
            return Err(EngineError::invalid(format!("the parameter '{name}' is declared twice")));
        }
        if !ParamKind::ALL.contains(&p.kind) {
            return Err(EngineError::invalid(format!("the parameter '{name}' must be a length, an angle or a number")));
        }
        if !p.value.is_finite() {
            return Err(EngineError::invalid(format!("the parameter '{name}' needs a number")));
        }
    }
    Ok(())
}

/// Create a structural profile: a standard shape of a material (its default
/// dimensions, changed by `params`), or a sketch drawn with `curves` (and
/// `openings`), whose points keep the drawn coordinates and which declares
/// `sketch_params`. Without a name it gets one from its shape and size.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileCreate {
    /// Name (unique, ignoring case; default: from the shape and its size).
    #[serde(default)]
    pub name: Option<String>,
    /// Material: `steel`, `cold_formed`, `timber` or `concrete`.
    pub material: String,
    /// Shape code: `i`, `i_asym`, `channel`, `t`, `angle`, `cold_u`,
    /// `cold_lipped_u`, `cold_z`, `cold_hat`, `rect_tube`, `round_tube`,
    /// `rect`, `round`, `concrete_t`, `concrete_l` or `sketch` (the default
    /// with `curves`; required without them).
    #[serde(default)]
    pub shape: Option<String>,
    /// Parameter values by name, meters (degrees for `angle`), over the
    /// shape's defaults; see `profile_get` `params` for the names.
    #[serde(default)]
    pub params: Option<BTreeMap<String, f64>>,
    /// The closed outline of a sketched profile, in meters.
    #[serde(default)]
    pub curves: Option<Vec<CurveArg>>,
    /// The openings of a sketched profile: each a closed loop of curves inside the outline.
    #[serde(default)]
    pub openings: Option<Vec<Vec<CurveArg>>>,
    /// Numeric parameters a sketched profile declares (lengths).
    #[serde(default)]
    pub sketch_params: Option<Vec<SketchParamArg>>,
}

impl Command for ProfileCreate {
    const NAME: &'static str = "profile_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        room_for_one(p)?;
        let material = material_arg(&self.material)?;
        let code = match (&self.shape, &self.curves) {
            (Some(c), _) => shape_arg(c)?,
            (None, Some(_)) => "sketch",
            (None, None) => return Err(EngineError::invalid("give a shape, or curves for a sketched profile")),
        };
        let sketched = self.curves.is_some() || self.openings.is_some() || self.sketch_params.is_some();
        if sketched && code != "sketch" {
            return Err(EngineError::invalid("curves, openings and sketch_params make a sketched profile"));
        }
        let mut shape =
            ProfileShape::default_for(code).ok_or_else(|| EngineError::invalid(format!("unknown shape '{code}'")))?;
        if let ProfileShape::Sketch(sk) = &mut shape {
            if let Some(curves) = &self.curves {
                sk.outline = loop_points(curves)?;
                sk.params = Vec::new();
                sk.openings = Vec::new();
            }
            if let Some(openings) = &self.openings {
                sk.openings = openings.iter().map(|o| loop_points(o)).collect::<Result<_>>()?;
            }
            if let Some(params) = &self.sketch_params {
                sk.params = sketch_params(params)?;
            }
        }
        if let Some(params) = &self.params {
            apply_params(&mut shape, params)?;
        }
        check_shape(&shape)?;
        let name = match &self.name {
            Some(n) => profile_name(p, n, None)?,
            None => p.free_profile_name(&cc_core::profiles_project::section_name_in(
                &shape,
                material,
                p.measurement_system(),
            )),
        };
        let mut def = ProfileDef {
            id: ProfileId(0),
            uid: Uid::generate(),
            name,
            material,
            shape,
            catalog: None,
            description: String::new(),
        };
        let id = edit(s, t!("tx-profile-create"), |e| {
            let id = ProfileId(e.alloc_id());
            def.id = id;
            e.profiles_mut().insert(id, def);
            Ok(id)
        })?;
        created(s, id)
    }
}

/// Change a structural profile: its name, material, parameter values or,
/// for a sketched one, its sketch (a merge patch over the `sketch` that
/// `profile_get` shows: `params`, `outline` and `openings`). A change that
/// leaves no valid outline is refused. The column and beam types using it
/// follow; a profile whose size or material changes is no longer its
/// catalog entry.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileSet {
    /// Profile id or name.
    pub profile: Ref,
    /// New name (unique, ignoring case).
    #[serde(default)]
    pub name: Option<String>,
    /// New material: `steel`, `cold_formed`, `timber` or `concrete`.
    #[serde(default)]
    pub material: Option<String>,
    /// Parameter values by name, meters (degrees for `angle`).
    #[serde(default)]
    pub params: Option<BTreeMap<String, f64>>,
    /// Merge patch over a sketched profile's sketch.
    #[serde(default)]
    pub sketch: Option<Value>,
}

impl Command for ProfileSet {
    const NAME: &'static str = "profile_set";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let id = refs::profile(p, &self.profile)?;
        let mut def = p.profile(id).cloned().ok_or_else(|| EngineError::not_found("profile"))?;
        if let Some(n) = &self.name {
            def.name = profile_name(p, n, Some(id))?;
        }
        if let Some(m) = &self.material {
            def.material = material_arg(m)?;
        }
        if let Some(patch) = &self.sketch {
            let ProfileShape::Sketch(sk) = &mut def.shape else {
                return Err(EngineError::invalid("only a sketched profile has a sketch"));
            };
            let mut value = serde_json::to_value(&*sk).map_err(|e| EngineError::invalid(e.to_string()))?;
            super::merge(&mut value, patch);
            *sk = serde_json::from_value::<ParamSketch>(value)
                .map_err(|e| EngineError::invalid(format!("sketch: {e}")))?;
            check_sketch_params(&sk.params)?;
        }
        if let Some(params) = &self.params {
            apply_params(&mut def.shape, params)?;
        }
        check_shape(&def.shape)?;
        if p.profile(id).is_some_and(|old| old.shape != def.shape || old.material != def.material) {
            def.catalog = None;
        }
        edit(s, t!("tx-profile-modify"), |e| {
            e.profiles_mut().insert(id, def);
            Ok(())
        })?;
        profile_out(s, id)
    }
}

/// Copy a structural profile under a new name (default: the name with a number).
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileDuplicate {
    /// Profile id or name.
    pub profile: Ref,
    /// Name of the copy (unique, ignoring case).
    #[serde(default)]
    pub name: Option<String>,
}

impl Command for ProfileDuplicate {
    const NAME: &'static str = "profile_duplicate";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        room_for_one(p)?;
        let id = refs::profile(p, &self.profile)?;
        let mut copy = p.profile(id).cloned().ok_or_else(|| EngineError::not_found("profile"))?;
        copy.name = match &self.name {
            Some(n) => profile_name(p, n, None)?,
            None => p.free_profile_name(&copy.name),
        };
        copy.uid = Uid::generate();
        let new = edit(s, t!("tx-profile-create"), |e| {
            let new = ProfileId(e.alloc_id());
            copy.id = new;
            e.profiles_mut().insert(new, copy);
            Ok(new)
        })?;
        created(s, new)
    }
}

/// Delete a structural profile. Refused while column or beam types use it,
/// unless `replace_with` names the profile they take instead, in the same
/// undo step.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileDelete {
    /// Profile id or name.
    pub profile: Ref,
    /// The profile (id or name) the types using it take instead.
    #[serde(default)]
    pub replace_with: Option<Ref>,
}

impl Command for ProfileDelete {
    const NAME: &'static str = "profile_delete";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let id = refs::profile(p, &self.profile)?;
        let with = self.replace_with.as_ref().map(|r| refs::profile(p, r)).transpose()?;
        if with == Some(id) {
            return Err(EngineError::invalid("a profile cannot replace itself"));
        }
        let users = p.profile_users(id);
        if !users.is_empty() && with.is_none() {
            return Err(EngineError::rejected(format!(
                "the profile is used by {}; give replace_with",
                type_names(p, id).join(", ")
            )));
        }
        edit(s, t!("tx-profile-delete"), |e| {
            if let Some(with) = with {
                for t in &users {
                    if let Some(ty) = e.type_mut(*t)
                        && let cc_core::TypeData::Column { profile, .. } | cc_core::TypeData::Beam { profile, .. } =
                            &mut ty.data
                    {
                        *profile = Some(with);
                    }
                }
            }
            e.profiles_mut().remove(&id);
            Ok(())
        })?;
        Ok(json!({ "deleted": id.0, "moved": users.len() }))
    }
}

/// A catalog series by code, ignoring case.
fn series_arg(code: &str) -> Result<&'static profile_catalog::Series> {
    let code = code.trim();
    profile_catalog::catalog().iter().find(|s| s.code.eq_ignore_ascii_case(code)).ok_or_else(|| {
        let codes: Vec<&str> = profile_catalog::catalog().iter().map(|s| s.code).collect();
        EngineError::not_found(format!("no catalog series '{code}' ({})", codes.join(", ")))
    })
}

/// A series' name in the interface language (its code until translated).
fn series_name(s: &profile_catalog::Series) -> String {
    crate::i18n::try_tr(s.name_key).unwrap_or_else(|| s.code.to_string())
}

/// The built-in profile catalogs (standard steel, cold-formed, timber and
/// hollow sections). Without `series`: each series' `code`, `name`,
/// `material`, `shape` and row `count`. With it: the series' `code`,
/// `name` and `rows`, each a `designation`, the `name` an imported profile
/// gets and its `params` (meters; degrees for `angle`). Catalogs are not
/// part of the project; `profile_import` copies rows in.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileCatalog {
    /// Series code (`W`, `HP`, `IPE`, `HEA`, `HEB`, `NBR6355-Ue`, `SHS`…; ignoring case).
    #[serde(default)]
    pub series: Option<String>,
    /// Only rows whose designation contains this text (ignoring case).
    #[serde(default)]
    pub search: Option<String>,
}

impl Command for ProfileCatalog {
    const NAME: &'static str = "profile_catalog";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, _: &mut Session) -> Result<Value> {
        let Some(code) = &self.series else {
            let rows = profile_catalog::catalog()
                .iter()
                .map(|s| {
                    json!({
                        "code": s.code,
                        "name": series_name(s),
                        "library": s.library.code(),
                        "material": s.material.code(),
                        "shape": s.shape,
                        "count": s.rows.len(),
                    })
                })
                .collect();
            return Ok(Value::Array(rows));
        };
        let series = series_arg(code)?;
        if self.search.as_deref().is_some_and(|t| t.chars().count() > MAX_NAME) {
            return Err(EngineError::invalid(format!("a search has at most {MAX_NAME} characters")));
        }
        let search = self.search.as_deref().map(|t| t.trim().to_lowercase()).unwrap_or_default();
        let rows: Vec<Value> = series
            .rows
            .iter()
            .filter(|r| r.designation.to_lowercase().contains(&search))
            .filter_map(|r| {
                let def = profile_catalog::row_to_def(series, r)?;
                Some(json!({ "designation": r.designation, "name": def.name, "params": params_json(&def.shape) }))
            })
            .collect();
        Ok(json!({ "code": series.code, "name": series_name(series), "rows": rows }))
    }
}

/// Copy rows of a built-in catalog series into the project as profiles that
/// remember their catalog entry, in one undo step (none when nothing is
/// imported). A row whose profile name is taken keeps the existing profile
/// (`on_clash: "keep"`, the default) or comes in under a free name
/// (`"rename"`). Returns the `imported` profiles (`id`, `name`), the names
/// `kept` and the designations the series does not have (`unknown`).
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileImport {
    /// Series code (see `profile_catalog`).
    pub series: String,
    /// Designations to import, as `profile_catalog` lists them (ignoring case).
    pub designations: Vec<String>,
    /// `keep` (default) or `rename`.
    #[serde(default)]
    pub on_clash: Option<String>,
}

impl Command for ProfileImport {
    const NAME: &'static str = "profile_import";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let series = series_arg(&self.series)?;
        let rename = match self.on_clash.as_deref().map(str::trim) {
            None | Some("keep") => false,
            Some("rename") => true,
            Some(other) => return Err(EngineError::invalid(format!("on_clash must be keep or rename, not '{other}'"))),
        };
        if self.designations.len() > MAX_PROFILES {
            return Err(EngineError::invalid(format!("import at most {MAX_PROFILES} profiles at a time")));
        }
        if self.designations.iter().any(|d| d.chars().count() > MAX_NAME) {
            return Err(EngineError::invalid(format!("a designation has at most {MAX_NAME} characters")));
        }
        let p = s.project();
        let (mut new, mut kept, mut unknown): (Vec<ProfileDef>, Vec<String>, Vec<String>) = Default::default();
        // The profile already named so: in the project or among those about to come in.
        let taken = |new: &[ProfileDef], name: &str| {
            p.profile_named(name).map(|f| f.name.clone()).or_else(|| {
                let wanted = name.trim().to_lowercase();
                new.iter().find(|f| f.name.trim().to_lowercase() == wanted).map(|f| f.name.clone())
            })
        };
        for d in &self.designations {
            let row = series.rows.iter().find(|r| r.designation.trim().eq_ignore_ascii_case(d.trim()));
            let Some(mut def) = row.and_then(|r| profile_catalog::row_to_def(series, r)) else {
                unknown.push(d.clone());
                continue;
            };
            if let Some(existing) = taken(&new, &def.name) {
                if !rename {
                    kept.push(existing);
                    continue;
                }
                // `name 2`, `name 3`…: bounded, as only the project's names and
                // `new.len()` pending ones can be taken.
                let base = def.name.clone();
                let limit = p.profiles.len() + new.len() + 2;
                def.name = (2..=limit)
                    .map(|n| format!("{base} {n}"))
                    .find(|n| taken(&new, n).is_none())
                    .unwrap_or_else(|| p.free_profile_name(&base));
            }
            new.push(def);
        }
        if p.profiles.len() + new.len() > MAX_PROFILES {
            return Err(EngineError::rejected(format!("a project has at most {MAX_PROFILES} profiles")));
        }
        let imported = if new.is_empty() {
            Vec::new()
        } else {
            edit(s, t!("tx-profile-import"), |e| {
                let mut out = Vec::with_capacity(new.len());
                for mut def in new {
                    let id = ProfileId(e.alloc_id());
                    def.id = id;
                    out.push(json!({ "id": id.0, "name": def.name }));
                    e.profiles_mut().insert(id, def);
                }
                Ok(out)
            })?
        };
        Ok(json!({ "imported": imported, "kept": kept, "unknown": unknown }))
    }
}

/// The material a type made for a profile gets: the first project material
/// of the profile's material class, else the template's concrete, else `fallback`.
fn type_material(p: &Project, m: ProfileMaterial, fallback: &str) -> String {
    let class = m.material_class();
    if let Some(found) = p.materials.iter().find(|x| x.identity.class == class) {
        return found.name.clone();
    }
    let concrete = t!("tpl-mat-concrete");
    if p.material(&concrete).is_some() { concrete } else { fallback.to_string() }
}

/// Make a column or beam type of a structural profile: a copy of the
/// category's first type given the profile, its width and height becoming
/// the profile's bounding box. Returns its `id` and `name`.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileTypeCreate {
    /// Profile id or name.
    pub profile: Ref,
    /// `column` or `beam`.
    pub category: String,
    /// Type name (unique in its category; default: the profile's name, made unique).
    #[serde(default)]
    pub name: Option<String>,
    /// Material name (default: the first project material of the profile's
    /// class, metal, wood or concrete; else the template's concrete).
    #[serde(default)]
    pub material: Option<String>,
}

impl Command for ProfileTypeCreate {
    const NAME: &'static str = "profile_type_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let category = refs::category(&self.category)?;
        if !matches!(category, Category::Column | Category::Beam) {
            return Err(EngineError::invalid(format!(
                "a profile makes column or beam types, not {}",
                refs::category_name(category)
            )));
        }
        let pid = refs::profile(p, &self.profile)?;
        let def = p.profile(pid).ok_or_else(|| EngineError::not_found("profile"))?;
        let template = p.types_of(category).next().ok_or_else(|| {
            EngineError::rejected(format!(
                "the project has no {} type to copy; create one first",
                refs::category_name(category)
            ))
        })?;
        let taken = |n: &str| p.types_of(category).any(|t| t.name.trim().eq_ignore_ascii_case(n.trim()));
        let name = match &self.name {
            Some(n) if n.trim().chars().count() > MAX_NAME => {
                return Err(EngineError::invalid(format!("a type name has at most {MAX_NAME} characters")));
            }
            Some(n) if n.trim().is_empty() || taken(n) => {
                return Err(EngineError::invalid(format!("the name '{n}' is empty or already used")));
            }
            Some(n) => n.trim().to_string(),
            None => {
                let base = def.name.trim().to_string();
                // Bounded: no more names are taken than there are types.
                (1..=p.types.len() + 1)
                    .map(|k| if k == 1 { base.clone() } else { format!("{base} {k}") })
                    .find(|n| !taken(n))
                    .unwrap_or(base)
            }
        };
        let mut data = template.data.clone();
        let (TypeData::Column { material, profile, profile_rotation_deg, profile_mirror, .. }
        | TypeData::Beam { material, profile, profile_rotation_deg, profile_mirror, .. }) = &mut data
        else {
            return Err(EngineError::invalid("not a column or beam type"));
        };
        *material = match &self.material {
            Some(m) => refs::material(p, &Ref::Name(m.clone()))?,
            None => type_material(p, def.material, material),
        };
        *profile = Some(pid);
        *profile_rotation_deg = 0.0;
        *profile_mirror = false;
        let id = edit(s, t!("tx-profile-type-create"), |e| {
            let id = ElementTypeId(e.alloc_id());
            Ok(e.add_type(ElementType { id, name: name.clone(), data, info: Default::default(), uid: Uid::generate() }))
        })?;
        Ok(json!({ "id": id.0, "name": name }))
    }
}
