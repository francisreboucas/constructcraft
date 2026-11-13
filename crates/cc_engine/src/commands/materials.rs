//! Materials: create, duplicate, rename, change, delete.

use super::{edit, patch_value};
use crate::error::{EngineError, Result};
use crate::refs::{self, Ref};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::{Material, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// `base`, or `base 2`, `base 3`… when taken.
fn unique_name(p: &Project, base: &str) -> String {
    let base = base.trim();
    let mut name = base.to_string();
    let mut n = 2;
    while p.material(&name).is_some() {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

fn material_out(s: &Session, name: &str) -> Value {
    s.project().material(name).map(material_json).unwrap_or(json!({ "name": name }))
}

/// A material's fields as JSON, its texture's image as a summary (format,
/// size in pixels and bytes) instead of its bytes.
pub(crate) fn material_json(m: &Material) -> Value {
    let mut v = serde_json::to_value(m).unwrap_or(json!({ "name": m.name }));
    if let (Some(t), Some(slot)) = (&m.appearance.texture, v.pointer_mut("/appearance/texture/image")) {
        *slot = json!({
            "format": t.image.format,
            "width": t.image.width,
            "height": t.image.height,
            "bytes": t.image.data.len(),
        });
    }
    v
}

/// Create a material (default name "Material"; a number is added when taken).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialCreate {
    /// Name (default: "Material"; a number is added when taken).
    #[serde(default)]
    pub name: Option<String>,
    /// Shading color `[r, g, b]`, 0–255 (default light gray).
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

impl Command for MaterialCreate {
    const NAME: &'static str = "material_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let name = unique_name(s.project(), &self.name.unwrap_or_else(|| t!("mat-new-name")));
        let material = Material::new(name.clone(), self.color.unwrap_or([190, 190, 190]));
        edit(s, t!("tx-modify-material"), |e| {
            e.materials_mut().push(material);
            Ok(())
        })?;
        Ok(material_out(s, &name))
    }
}

/// Copy a material under a new name (default "<name> (copy)").
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialDuplicate {
    pub material: Ref,
    /// Name of the copy (default: "<name> (copy)").
    #[serde(default)]
    pub name: Option<String>,
}

impl Command for MaterialDuplicate {
    const NAME: &'static str = "material_duplicate";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let src = p
            .material(&refs::material(p, &self.material)?)
            .cloned()
            .ok_or_else(|| EngineError::not_found("material"))?;
        let name = unique_name(p, &self.name.unwrap_or_else(|| t!("copy-of", "name" => src.name)));
        let copy = Material { name: name.clone(), ..src };
        edit(s, t!("tx-modify-material"), |e| {
            e.materials_mut().push(copy);
            Ok(())
        })?;
        Ok(material_out(s, &name))
    }
}

/// Rename a material; every type using it follows.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialRename {
    pub material: Ref,
    /// New name.
    pub name: String,
}

impl Command for MaterialRename {
    const NAME: &'static str = "material_rename";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let old = refs::material(s.project(), &self.material)?;
        let new = self.name.trim().to_string();
        if new.is_empty() {
            return Err(EngineError::invalid("the name is empty"));
        }
        if new != old {
            edit(s, t!("tx-rename-material"), |e| {
                if e.rename_material(&old, &new) {
                    Ok(())
                } else {
                    Err(EngineError::rejected("the name is already used"))
                }
            })?;
        }
        Ok(material_out(s, &new))
    }
}

/// Change a material's fields (merged into what `material_get` shows:
/// description, class, colors, patterns, transparency, finish, physical
/// values…). Use `material_rename` to rename it.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialSet {
    pub material: Ref,
    /// Fields to change, shaped like the result of `material_get`.
    pub values: Value,
}

impl Command for MaterialSet {
    const NAME: &'static str = "material_set";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let name = refs::material(p, &self.material)?;
        let current = p.material(&name).ok_or_else(|| EngineError::not_found("material"))?;
        let mut changed = patch_value(current, &self.values)?;
        if changed.name != name {
            return Err(EngineError::invalid("use material_rename to rename a material"));
        }
        if changed.repair() > 0 {
            return Err(EngineError::invalid("invalid material values"));
        }
        edit(s, t!("tx-modify-material"), |e| {
            if let Some(m) = e.materials_mut().iter_mut().find(|m| m.name == name) {
                *m = changed;
            }
            Ok(())
        })?;
        Ok(material_out(s, &name))
    }
}

/// Delete a material no type uses.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialDelete {
    pub material: Ref,
}

impl Command for MaterialDelete {
    const NAME: &'static str = "material_delete";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let name = refs::material(p, &self.material)?;
        let users = p.material_users(&name).len();
        if users > 0 {
            return Err(EngineError::rejected(format!("{users} types use this material")));
        }
        let elements = p.material_instance_users(&name).len();
        if elements > 0 {
            return Err(EngineError::rejected(format!("{elements} components use this material")));
        }
        let painted = p.material_paint_users(&name).len();
        if painted > 0 {
            return Err(EngineError::rejected(format!("{painted} elements have faces painted with this material")));
        }
        edit(s, t!("tx-modify-material"), |e| {
            e.materials_mut().retain(|m| m.name != name);
            Ok(())
        })?;
        Ok(json!({ "deleted": name }))
    }
}

/// Give a material an image texture (shown on its faces in the textures
/// visual style of 3D views), or change how its texture is laid out. The
/// image comes from `path` or from `data` (base64); with neither, the
/// current texture's size, offset and rotation change. One repeat covers
/// `width` × `height` meters on the faces: on floors and other horizontal
/// faces along plan X and Y, on walls and slopes level across the face and
/// up it.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialTextureSet {
    pub material: Ref,
    /// Path of a PNG, JPEG, GIF, BMP or WebP file.
    #[serde(default)]
    pub path: Option<std::path::PathBuf>,
    /// The image file's bytes as base64 (instead of `path`).
    #[serde(default)]
    pub data: Option<String>,
    /// Width of one repeat, meters (default: the current one, else 1 m).
    #[serde(default)]
    pub width: Option<f64>,
    /// Height of one repeat, meters (default: the current one, else the
    /// width in the image's proportions).
    #[serde(default)]
    pub height: Option<f64>,
    /// Where a repeat starts, `[x, y]` meters along the face (default: the current one, else `[0, 0]`).
    #[serde(default)]
    pub offset: Option<[f64; 2]>,
    /// Rotation of the image on the faces, degrees counter-clockwise (default: the current one, else 0).
    #[serde(default)]
    pub rotation: Option<f64>,
}

impl Command for MaterialTextureSet {
    const NAME: &'static str = "material_texture_set";
    const KIND: CommandKind = CommandKind::Edit;
    const READS_FILES: bool = true;
    fn run(self, s: &mut Session) -> Result<Value> {
        use cc_core::material::{MAX_TEXTURE_BYTES, MAX_TEXTURE_SIZE, MIN_TEXTURE_SIZE, MaterialTexture};
        let p = s.project();
        let name = refs::material(p, &self.material)?;
        let current = p.material(&name).ok_or_else(|| EngineError::not_found("material"))?.appearance.texture.clone();
        let bytes = match (&self.path, &self.data) {
            (Some(_), Some(_)) => return Err(EngineError::invalid("give either path or data, not both")),
            (Some(path), None) => {
                s.require_files("path")?;
                if std::fs::metadata(path)?.len() > MAX_TEXTURE_BYTES as u64 {
                    return Err(EngineError::invalid(t!("err-image-too-large")));
                }
                Some(std::fs::read(path)?)
            }
            (None, Some(text)) => {
                if text.len() / 4 * 3 > MAX_TEXTURE_BYTES + 3 {
                    return Err(EngineError::invalid(t!("err-image-too-large")));
                }
                Some(
                    cc_core::raster::base64_decode(text)
                        .ok_or_else(|| EngineError::invalid("data is not valid base64"))?,
                )
            }
            (None, None) => None,
        };
        let size = |v: Option<f64>, what: &str| -> Result<Option<f64>> {
            v.map(|v| {
                if v.is_finite() && (MIN_TEXTURE_SIZE..=MAX_TEXTURE_SIZE).contains(&v) {
                    Ok(v)
                } else {
                    Err(EngineError::invalid(format!(
                        "{what} must be between {MIN_TEXTURE_SIZE} and {MAX_TEXTURE_SIZE} m"
                    )))
                }
            })
            .transpose()
        };
        let (width, height) = (size(self.width, "width")?, size(self.height, "height")?);
        let offset =
            self.offset.map(|[x, y]| Ok::<_, EngineError>([refs::finite(x, "offset")?, refs::finite(y, "offset")?]));
        let offset = offset.transpose()?;
        let rotation = self.rotation.map(|r| refs::finite(r, "rotation")).transpose()?;
        let mut texture = match bytes {
            Some(bytes) => {
                let image = crate::session::guard(|| cc_views::raster::probe(bytes))?
                    .ok_or_else(|| EngineError::invalid(t!("err-image-unreadable")))?;
                let mut texture = MaterialTexture::new(image, 1.0, 1.0);
                if !texture.image_ok() {
                    return Err(EngineError::invalid(t!("err-image-too-large")));
                }
                if let Some(c) = &current {
                    texture = MaterialTexture { image: texture.image, ..c.clone() };
                }
                texture
            }
            None => current.ok_or_else(|| EngineError::invalid("the material has no texture: give path or data"))?,
        };
        // A new texture without a size of its own keeps the image's proportions.
        let aspect = f64::from(texture.image.height) / f64::from(texture.image.width.max(1));
        match (width, height) {
            (Some(w), Some(h)) => (texture.width, texture.height) = (w, h),
            (Some(w), None) => {
                texture.width = w;
                texture.height = (w * aspect).clamp(MIN_TEXTURE_SIZE, MAX_TEXTURE_SIZE);
            }
            (None, Some(h)) => texture.height = h,
            (None, None) => {}
        }
        if let Some(o) = offset {
            texture.offset = o;
        }
        if let Some(r) = rotation {
            texture.rotation_deg = r;
        }
        if texture.repair() > 0 {
            return Err(EngineError::invalid("invalid texture values"));
        }
        edit(s, t!("tx-modify-material"), |e| {
            if let Some(m) = e.materials_mut().iter_mut().find(|m| m.name == name) {
                m.appearance.texture = Some(texture);
            }
            Ok(())
        })?;
        Ok(material_out(s, &name))
    }
}

/// Take a material's texture away (its faces show the appearance color).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterialTextureClear {
    pub material: Ref,
}

impl Command for MaterialTextureClear {
    const NAME: &'static str = "material_texture_clear";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let name = refs::material(p, &self.material)?;
        if p.material(&name).is_none_or(|m| m.appearance.texture.is_none()) {
            return Err(EngineError::invalid("the material has no texture"));
        }
        edit(s, t!("tx-modify-material"), |e| {
            if let Some(m) = e.materials_mut().iter_mut().find(|m| m.name == name) {
                m.appearance.texture = None;
            }
            Ok(())
        })?;
        Ok(material_out(s, &name))
    }
}

/// Import structural materials from a built-in regional library (`brazil`:
/// ABNT NBR 7007/7008 and ASTM steels, NBR 7480 reinforcing bars and
/// NBR 8953 / NBR 6118 concrete classes C20–C90) into the project. Materials
/// whose names are already in the project are kept untouched.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct MaterialLibraryImport {
    /// Library code: `brazil` (default).
    #[serde(default)]
    pub library: Option<String>,
    /// Category filter: `steel`, `rebar`, `concrete` or `all` (default: `all`).
    #[serde(default)]
    pub kind: Option<String>,
}

impl Command for MaterialLibraryImport {
    const NAME: &'static str = "material_library_import";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let lib = self.library.as_deref().unwrap_or("brazil").trim().to_lowercase();
        if !matches!(lib.as_str(), "brazil" | "br" | "brasil") {
            return Err(EngineError::invalid(format!("unknown material library '{lib}' (expected 'brazil')")));
        }
        let kind = self.kind.as_deref().unwrap_or("all").trim().to_lowercase();
        if !matches!(kind.as_str(), "all" | "steel" | "rebar" | "concrete") {
            return Err(EngineError::invalid(format!(
                "kind must be 'all', 'steel', 'rebar' or 'concrete', not '{kind}'"
            )));
        }
        let presets = cc_core::material::brazil_structural_materials();
        let p = s.project();
        let mut new = Vec::new();
        let mut kept = Vec::new();
        for m in presets {
            let matches_kind = match kind.as_str() {
                "steel" => m.name.starts_with("Steel "),
                "rebar" => m.name.starts_with("Rebar "),
                "concrete" => m.name.starts_with("Concrete "),
                _ => true,
            };
            if !matches_kind {
                continue;
            }
            if p.materials.iter().any(|existing| existing.name.trim().eq_ignore_ascii_case(m.name.trim())) {
                kept.push(m.name);
            } else {
                new.push(m);
            }
        }
        let imported: Vec<String> = new.iter().map(|m| m.name.clone()).collect();
        if !new.is_empty() {
            edit(s, t!("tx-modify-material"), |e| {
                e.materials_mut().extend(new);
                Ok(())
            })?;
        }
        Ok(json!({ "imported": imported, "kept": kept }))
    }
}
