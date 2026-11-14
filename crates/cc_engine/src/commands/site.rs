//! The site location (latitude, longitude, elevation, true north and the
//! postal address) and the map data brought in around it (a site drawing
//! and context masses from OpenStreetMap).

use super::{add_element, edit};
use crate::error::{EngineError, Result};
use crate::osm;
use crate::refs::{self, Ref, finite, positive};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::cad::CadImport;
use cc_core::mass::Mass;
use cc_core::site::{SiteLocation, normalize_angle};
use cc_core::{Category, ElementId, ElementKind, ElementType, ElementTypeId, TypeData, ViewKind};
use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// The site as commands show it (`null` when the project has no location).
pub fn site_json(s: &Session) -> Value {
    let info = &s.project().info;
    match &info.site {
        None => Value::Null,
        Some(site) => json!({
            "latitude": site.latitude,
            "longitude": site.longitude,
            "elevation": site.elevation,
            "true_north": site.true_north,
            "place": site.place,
            "address": site.address.parts().iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
            "utc_offset": site.utc_offset,
            "utc_offset_used": site.offset_hours(),
        }),
    }
}

/// The project's site location: latitude and longitude of the project
/// origin (degrees, WGS 84), its elevation above sea level (meters), the
/// angle from project north to true north (degrees counter-clockwise),
/// the place name, the parts of the postal address, and local time's
/// offset from UTC in hours (`utc_offset`, null when not set, and
/// `utc_offset_used`: the one set, else the longitude's nominal time zone,
/// which the sun uses); `site` is null when no location is set. `address` is the project address line.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SiteGet {}

impl Command for SiteGet {
    const NAME: &'static str = "site_get";
    const KIND: CommandKind = CommandKind::Query;
    fn run(self, s: &mut Session) -> Result<Value> {
        Ok(json!({ "site": site_json(s), "address": s.project().info.address }))
    }
}

/// Set the project's site location, in one undo step: the latitude and
/// longitude of the project origin (both needed when the project has no
/// location yet), its elevation above sea level, true north and the
/// place and address found by a map search, and the offset from UTC.
/// Fields left out keep their values. When address parts are given the project address (title
/// blocks, IFC) is rewritten from them unless `project_address` is false.
/// `clear` removes the location.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SiteSet {
    /// Latitude of the project origin, degrees (WGS 84, -90 to 90, north positive).
    #[serde(default)]
    pub latitude: Option<f64>,
    /// Longitude of the project origin, degrees (WGS 84, -180 to 180, east positive).
    #[serde(default)]
    pub longitude: Option<f64>,
    /// Height of the project's elevation 0 above sea level, meters (-500 to 9000).
    #[serde(default)]
    pub elevation: Option<f64>,
    /// Angle from project north (+Y) to true north, degrees counter-clockwise.
    #[serde(default)]
    pub true_north: Option<f64>,
    /// The place as the map search named it.
    #[serde(default)]
    pub place: Option<String>,
    /// Address parts by key (`street`, `number`, `district`, `city`, `region`, `postcode`, `country`); others are refused.
    #[serde(default)]
    pub address: Option<serde_json::Map<String, Value>>,
    /// Rewrite the project address from the address parts (default true when `address` is given).
    #[serde(default)]
    pub project_address: Option<bool>,
    /// Local time's offset from UTC, hours (-12 to 14; fractions allowed),
    /// or `null` for the longitude's nominal time zone.
    #[serde(default, deserialize_with = "refs::nullable", skip_serializing_if = "Option::is_none")]
    pub utc_offset: Option<Option<f64>>,
    /// Remove the site location (nothing else may be given).
    #[serde(default)]
    pub clear: Option<bool>,
}

impl Command for SiteSet {
    const NAME: &'static str = "site_set";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        if self.clear == Some(true) {
            let others = self.latitude.is_some()
                || self.longitude.is_some()
                || self.elevation.is_some()
                || self.true_north.is_some()
                || self.place.is_some()
                || self.address.is_some()
                || self.utc_offset.is_some();
            if others {
                return Err(EngineError::invalid("clear takes no other field"));
            }
            edit(s, t!("tx-site-location"), |e| {
                e.info_mut().site = None;
                Ok(())
            })?;
            return Ok(json!({ "site": Value::Null, "address": s.project().info.address }));
        }
        let mut site = match (&s.project().info.site, self.latitude, self.longitude) {
            (Some(site), _, _) => site.clone(),
            (None, Some(lat), Some(lon)) => SiteLocation::at(lat, lon),
            (None, _, _) => return Err(EngineError::invalid("give latitude and longitude for a new site location")),
        };
        if let Some(v) = self.latitude {
            site.latitude = finite(v, "latitude")?;
        }
        if let Some(v) = self.longitude {
            site.longitude = finite(v, "longitude")?;
        }
        if !(-90.0..=90.0).contains(&site.latitude) {
            return Err(EngineError::invalid("latitude must be between -90 and 90 degrees"));
        }
        if !(-180.0..=180.0).contains(&site.longitude) {
            return Err(EngineError::invalid("longitude must be between -180 and 180 degrees"));
        }
        if let Some(v) = self.elevation {
            let v = finite(v, "elevation")?;
            if !cc_core::site::ELEVATION_RANGE.contains(&v) {
                return Err(EngineError::invalid("elevation must be between -500 and 9000 meters"));
            }
            site.elevation = v;
        }
        if let Some(v) = self.true_north {
            site.true_north = normalize_angle(finite(v, "true_north")?);
        }
        match self.utc_offset {
            Some(Some(v)) => site.utc_offset = Some(super::sun::utc_offset(v)?),
            Some(None) => site.utc_offset = None,
            None => {}
        }
        if let Some(v) = self.place {
            site.place = v.trim().to_string();
        }
        let address_given = self.address.is_some();
        for (key, value) in self.address.iter().flatten() {
            let text = match value {
                Value::String(v) => v.trim().to_string(),
                Value::Null => String::new(),
                other => return Err(EngineError::invalid(format!("address '{key}': expected text, got {other}"))),
            };
            let part = site.address.part_mut(key).ok_or_else(|| {
                EngineError::invalid(format!(
                    "unknown address part '{key}' (street, number, district, city, region, postcode, country)"
                ))
            })?;
            *part = text;
        }
        let (valid, _) = site.repair();
        if !valid {
            return Err(EngineError::invalid("the site location is not valid"));
        }
        let line = (address_given && self.project_address != Some(false)).then(|| site.address.one_line());
        edit(s, t!("tx-site-location"), |e| {
            let info = e.info_mut();
            if let Some(line) = line.filter(|l| !l.is_empty()) {
                info.address = line;
            }
            info.site = Some(site);
            Ok(())
        })?;
        Ok(json!({ "site": site_json(s), "address": s.project().info.address }))
    }
}

/// Bring OpenStreetMap data around the site into a floor plan, in one undo
/// step: a site drawing (an imported drawing in meters, pinned, under the
/// model: layers for buildings, streets, paths, railways, water, green
/// areas, trees and street names) and, unless `masses` is false, a
/// context mass per building footprint (height from the map's `height` or
/// `building:levels`, else `default_levels` storeys; no mass floors) of
/// the "Context" mass type. `data` is the JSON an Overpass query answers
/// (`[out:json]`, `out geom` or ways with their nodes; the program builds
/// the query for the square it shows); points are placed with the site
/// location, which must be set. Map data © OpenStreetMap contributors (ODbL).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SiteContextImport {
    /// Overpass JSON text (or `path`).
    #[serde(default)]
    pub data: Option<String>,
    /// Path of a file holding Overpass JSON (or `data`).
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Floor plan id or name (default: the first floor plan of the lowest level).
    #[serde(default)]
    pub view: Option<Ref>,
    /// Half the side of the square kept around the project origin, meters (default: everything given).
    #[serde(default)]
    pub extent: Option<f64>,
    /// Make context masses from the building footprints (default true).
    #[serde(default)]
    pub masses: Option<bool>,
    /// Height of a storey, meters, for `building:levels` (default 3).
    #[serde(default)]
    pub level_height: Option<f64>,
    /// Storeys of a building whose height the map does not give (default 2).
    #[serde(default)]
    pub default_levels: Option<f64>,
    /// Height of street names, meters (default 2).
    #[serde(default)]
    pub text_height: Option<f64>,
}

/// The plan a site import goes into and its level.
fn site_view(s: &Session, view: Option<&Ref>) -> Result<(cc_core::ViewId, cc_core::LevelId)> {
    let p = s.project();
    let view_id = match view {
        Some(r) => refs::view(p, r)?,
        None => {
            let plans = p.views.values().filter_map(|v| match v.kind {
                ViewKind::FloorPlan { level, mep: None, .. } => Some((p.level_elevation(level), v.id)),
                _ => None,
            });
            plans
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
                .map(|(_, id)| id)
                .ok_or_else(|| EngineError::rejected("the project has no floor plan"))?
        }
    };
    let view = p.view(view_id).ok_or_else(|| EngineError::not_found("view"))?;
    match view.kind {
        ViewKind::FloorPlan { level, .. } => Ok((view_id, level)),
        _ => Err(EngineError::invalid(t!("msg-site-view"))),
    }
}

/// The context mass type, creating it if the project has none.
fn context_type(e: &mut cc_core::EditCtx) -> ElementTypeId {
    let name = t!("tpl-type-mass-context");
    if let Some(t) = e.project().types_of(Category::Mass).find(|t| t.name == name) {
        return t.id;
    }
    let id = ElementTypeId(e.alloc_id());
    let material = e.project().materials.first().map(|m| m.name.clone()).unwrap_or_default();
    e.add_type(ElementType {
        id,
        name,
        data: TypeData::Mass { material },
        info: Default::default(),
        uid: Default::default(),
    })
}

impl Command for SiteContextImport {
    const NAME: &'static str = "site_context_import";
    const KIND: CommandKind = CommandKind::Edit;
    const READS_FILES: bool = true;
    fn run(self, s: &mut Session) -> Result<Value> {
        let site = s.project().info.site.clone().ok_or_else(|| EngineError::rejected(t!("msg-site-none")))?;
        let text = match (self.data, &self.path) {
            (Some(_), Some(_)) => return Err(EngineError::invalid("give data or path, not both")),
            (Some(d), None) => d,
            (None, Some(path)) => {
                s.require_files("path")?;
                let meta =
                    std::fs::metadata(path).map_err(|e| EngineError::invalid(format!("{}: {e}", path.display())))?;
                if meta.len() > osm::MAX_JSON as u64 {
                    return Err(EngineError::invalid("the map data file is too large"));
                }
                std::fs::read_to_string(path).map_err(|e| EngineError::invalid(format!("{}: {e}", path.display())))?
            }
            (None, None) => return Err(EngineError::invalid("give data (Overpass JSON) or path")),
        };
        let (view_id, level) = site_view(s, self.view.as_ref())?;
        let opts = osm::Options {
            extent: self.extent.map(|v| positive(v, "extent")).transpose()?,
            level_height: positive(self.level_height.unwrap_or(3.0), "level_height")?,
            default_levels: positive(self.default_levels.unwrap_or(2.0), "default_levels")?,
            text_height: positive(self.text_height.unwrap_or(2.0), "text_height")?,
        };
        let read = osm::read(&text, &site, &opts).map_err(EngineError::invalid)?;
        let with_masses = self.masses != Some(false);
        if read.drawing.prims.is_empty() {
            return Err(EngineError::invalid(t!("msg-site-empty")));
        }
        let import = CadImport {
            view: view_id,
            name: format!(
                "OpenStreetMap ({})",
                if site.place.is_empty() { t!("osm-site") } else { site.place.clone() }
            ),
            drawing: Arc::new(read.drawing),
            units: cc_core::CadUnit::Meters,
            scale: 1.0,
            position: DVec2::ZERO,
            rotation_deg: 0.0,
            mirrored: false,
            hidden_layers: Default::default(),
            color_mode: cc_core::CadColorMode::Layers,
            halftone: false,
        };
        let buildings = if with_masses { read.buildings } else { Vec::new() };
        let (drawing, masses) = edit(s, t!("tx-site-context"), |e| {
            let cad_type = super::cad::cad_type(e);
            let drawing = add_element(e, level, cad_type, ElementKind::CadImport(Box::new(import)));
            e.set_pinned(&[drawing], true);
            let mut masses: Vec<ElementId> = Vec::new();
            if !buildings.is_empty() {
                let mass_type = context_type(e);
                for b in buildings {
                    let mass = Mass {
                        outline: b.outline,
                        base_offset: b.base,
                        height: b.height,
                        top: None,
                        floors: Vec::new(),
                        sketch: None,
                    };
                    let id = add_element(e, level, mass_type, ElementKind::Mass(mass));
                    if let Some(el) = e.element_mut(id) {
                        el.comments = match b.label.is_empty() {
                            true => format!("OpenStreetMap {}", b.source),
                            false => format!("{} (OpenStreetMap {})", b.label, b.source),
                        };
                    }
                    masses.push(id);
                }
            }
            Ok((drawing, masses))
        })?;
        Ok(json!({
            "drawing": drawing.0,
            "masses": super::ids_json(&masses),
            "layers": read.counts.iter().map(|(l, n)| (l.code().to_string(), json!(n))).collect::<serde_json::Map<_, _>>(),
            "attribution": osm::ATTRIBUTION,
        }))
    }
}
