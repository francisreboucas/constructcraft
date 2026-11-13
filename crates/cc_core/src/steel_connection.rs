//! Steel connections module using parametric macros.
//!
//! Recognizes structural steel members — columns, beams, rafters, trusses and
//! bracings — and builds parametric macro connections (`base_plate`,
//! `end_plate`, `single_plate`, `double_angle`, `gusset_plate`, `splice`) as
//! scripted components ([`crate::ComponentShape::Script`]) whose parameters can
//! be edited per instance or per type.

use crate::component::{ComponentDef, ComponentItem, ComponentKind, ComponentShape, DetailLevels};
use crate::properties::{Atom, IfcValue, Property, PropertySet, PropertyValue};
use crate::structural_profile::{ProfileMaterial, ProfileShape};
use crate::{Category, ColumnSection, Element, ElementId, ElementKind, LevelId, Project, SectionShape, TypeData};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Density of structural steel in kg/m³.
pub const STEEL_DENSITY: f64 = 7850.0;

/// Property set name stored on connection components.
pub const CONNECTION_PSET: &str = "CC_SteelConnection";

/// Round a positive length in meters up to the nearest 5 mm.
fn up5(m: f64) -> f64 {
    ((m * 1000.0 / 5.0) - 1e-9).ceil() * 5.0 / 1000.0
}

/// Round a positive length in meters up to the nearest 10 mm.
fn up10(m: f64) -> f64 {
    ((m * 100.0) - 1e-9).ceil() / 100.0
}

/// Format a length in meters as integer millimeters (`0.016` -> `"16"`).
fn mm_str(m: f64) -> String {
    format!("{}", (m * 1000.0).round() as i64)
}

/// Fillet weld throat thickness (m) for plate thickness `t` (m).
pub fn weld_throat(t: f64) -> f64 {
    ((0.5 * t * 1000.0).round() / 1000.0).max(0.003)
}

/// Typical steel connection macro kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConnectionKind {
    /// Column base plate (pinned or fixed, with grout pad, anchor bolts and optional stiffeners).
    BasePlate,
    /// Bolted end plate (flush or extended, for beam-to-column eaves/frames or apex ridges, with optional column stiffeners).
    EndPlate,
    /// Single plate / fin plate (shear tab welded to support, bolted to beam web).
    SinglePlate,
    /// Double angle cleat (two angle cleats bolted/welded to beam web and support face).
    DoubleAngle,
    /// Gusset plate (planar node plate for bracings and truss joints).
    GussetPlate,
    /// Bolted member splice (flange cover plates and web splice plates).
    Splice,
}

impl ConnectionKind {
    pub const ALL: [ConnectionKind; 6] =
        [Self::BasePlate, Self::EndPlate, Self::SinglePlate, Self::DoubleAngle, Self::GussetPlate, Self::Splice];

    /// Machine-readable identifier of the connection macro.
    pub fn code(self) -> &'static str {
        match self {
            Self::BasePlate => "base_plate",
            Self::EndPlate => "end_plate",
            Self::SinglePlate => "single_plate",
            Self::DoubleAngle => "double_angle",
            Self::GussetPlate => "gusset_plate",
            Self::Splice => "splice",
        }
    }

    /// Parse a connection macro kind from its code or alias.
    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "base_plate" | "base" | "chapa_de_base" => Some(Self::BasePlate),
            "end_plate" | "eaves" | "apex" | "chapa_de_topo" => Some(Self::EndPlate),
            "single_plate" | "fin_plate" | "shear_tab" | "post" | "chapa_simples" => Some(Self::SinglePlate),
            "double_angle" | "cleat" | "dupla_cantoneira" => Some(Self::DoubleAngle),
            "gusset_plate" | "gusset" | "chapa_de_no" => Some(Self::GussetPlate),
            "splice" | "emenda" => Some(Self::Splice),
            _ => None,
        }
    }

    /// Default component/type name for this macro.
    pub fn component_name(self) -> &'static str {
        match self {
            Self::BasePlate => "Steel Connection: Base Plate",
            Self::EndPlate => "Steel Connection: End Plate",
            Self::SinglePlate => "Steel Connection: Single Plate",
            Self::DoubleAngle => "Steel Connection: Double Angle",
            Self::GussetPlate => "Steel Connection: Gusset Plate",
            Self::Splice => "Steel Connection: Splice",
        }
    }

    /// Human-readable English description of the connection macro.
    pub fn description(self) -> &'static str {
        match self {
            Self::BasePlate => {
                "Column base plate (pinned or fixed) with grout pad, anchor bolts and optional stiffener ribs."
            }
            Self::EndPlate => {
                "Flush or extended bolted end plate for beam-to-column joints and pitched rafters, with optional column web stiffeners."
            }
            Self::SinglePlate => {
                "Single shear plate (fin plate) welded to the primary column or girder and bolted to the secondary beam web."
            }
            Self::DoubleAngle => {
                "Double angle cleats bolted to the supported beam web and the supporting column or girder."
            }
            Self::GussetPlate => {
                "Gusset plate for diagonal bracings (angles, tubes, round bars) and truss panel nodes."
            }
            Self::Splice => "Bolted flange and web splice plates joining two collinear steel members.",
        }
    }
}

/// Optional user overrides for sizing a steel connection macro.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionParams {
    /// Plate thickness in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plate_thickness: Option<f64>,
    /// Bolt or anchor diameter in meters (e.g. `0.016`, `0.020`, `0.024`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bolt_diameter: Option<f64>,
    /// Bolt or anchor steel grade (`"8.8"`, `"10.9"`, `"A325"`, `"A490"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bolt_grade: Option<String>,
    /// Number of horizontal bolt rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    /// Number of vertical bolt columns (`2` or `4` for end plates, `1` or `2` for shear plates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u32>,
    /// Vertical bolt spacing (pitch) in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f64>,
    /// Horizontal bolt gauge across the flange/web in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<f64>,
    /// Edge distance from bolt center to plate edge in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_distance: Option<f64>,
    /// Top extension of an extended end plate above the beam top flange, in meters (`0` for flush).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<f64>,
    /// Whether to add stiffeners (column web stiffeners on end plates, or base ribs on fixed base plates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stiffeners: Option<bool>,
    /// Column base restraint mode: `"pinned"` or `"fixed"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_mode: Option<String>,
    /// Anchor bolt end type: `"hooked"`, `"plate"` or `"straight"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_shape: Option<String>,
    /// Total number of anchor bolts on a base plate (`2`, `4`, `6` or `8`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchors: Option<u32>,
    /// Embedment length of anchor bolts in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_length: Option<f64>,
    /// Grout pad thickness under a base plate in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grout_thickness: Option<f64>,
    /// Clearance gap between the supported member end and the support face in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
}

/// Cross-section dimensions of a connected structural member.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemberSection {
    /// Section depth `d` (m).
    pub depth: f64,
    /// Section flange width `bf` (m).
    pub width: f64,
    /// Web thickness `tw` (m).
    pub web: f64,
    /// Flange thickness `tf` (m).
    pub flange: f64,
    /// Shape family (`"I"`, `"U"`, `"L"`, `"tube"`, `"pipe"`, `"round"`, `"cold"`).
    pub shape_kind: &'static str,
    /// True if the member is made of steel or cold-formed steel.
    pub is_steel: bool,
}

impl Default for MemberSection {
    fn default() -> Self {
        Self { depth: 0.30, width: 0.15, web: 0.0071, flange: 0.0107, shape_kind: "I", is_steel: true }
    }
}

/// Resolved engineering specification of a steel connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionSpec {
    pub kind: ConnectionKind,
    pub plate_thickness: f64,
    pub plate_width: f64,
    pub plate_height: f64,
    pub bolt_diameter: f64,
    pub bolt_grade: String,
    pub rows: u32,
    pub cols: u32,
    pub pitch: f64,
    pub gauge: f64,
    pub edge_distance: f64,
    pub extension: f64,
    pub stiffeners: bool,
    pub base_mode: String,
    pub anchor_shape: String,
    pub anchors: u32,
    pub anchor_length: f64,
    pub grout_thickness: f64,
    pub gap: f64,
    #[serde(default)]
    pub support_clearance: f64,
    pub weld_throat: f64,
    pub weld_length: f64,
    pub weight_kg: f64,
}

impl ConnectionSpec {
    /// Resolve all connection dimensions and fastener sizes from the primary
    /// (host) and secondary (bolted) member sections and optional overrides.
    pub fn resolve(
        kind: ConnectionKind,
        primary: MemberSection,
        secondary: Option<MemberSection>,
        params: &ConnectionParams,
    ) -> Self {
        let s = secondary.unwrap_or(primary);
        let h = s.depth.max(0.08);
        let b = s.width.max(0.05);
        let tw = s.web.max(0.004);
        let tf = s.flange.max(tw);

        let default_bolt = match kind {
            ConnectionKind::BasePlate => {
                if h < 0.25 {
                    0.020
                } else if h < 0.45 {
                    0.024
                } else {
                    0.030
                }
            }
            ConnectionKind::GussetPlate => {
                if h < 0.15 {
                    0.016
                } else {
                    0.020
                }
            }
            _ => {
                if h < 0.22 {
                    0.016
                } else if h < 0.36 {
                    0.020
                } else if h < 0.55 {
                    0.024
                } else {
                    0.027
                }
            }
        };
        let bolt = params.bolt_diameter.filter(|&d| d > 0.005).unwrap_or(default_bolt);

        let base_mode = match params.base_mode.as_deref() {
            Some("fixed") => "fixed".to_string(),
            _ => "pinned".to_string(),
        };
        let fixed_base = base_mode == "fixed";

        let anchor_shape = match params.anchor_shape.as_deref() {
            Some("plate") => "plate".to_string(),
            Some("straight") => "straight".to_string(),
            _ => "hooked".to_string(),
        };

        let default_plate = match kind {
            ConnectionKind::BasePlate => {
                if fixed_base {
                    up5((1.25 * bolt).max(1.5 * tf).max(0.025))
                } else {
                    up5((0.8 * bolt).max(tf).max(0.016))
                }
            }
            ConnectionKind::SinglePlate | ConnectionKind::DoubleAngle => up5((0.5 * bolt).max(0.008)),
            ConnectionKind::GussetPlate => up5((0.6 * bolt).max(0.010)),
            ConnectionKind::EndPlate | ConnectionKind::Splice => up5((0.8 * bolt).max(tf).max(0.012)),
        };
        let plate_thickness = params.plate_thickness.filter(|&t| t > 0.002).unwrap_or(default_plate);

        let pitch = params.pitch.filter(|&p| p > 0.02).unwrap_or_else(|| up5(3.0 * bolt));
        let edge_distance = params.edge_distance.filter(|&e| e > 0.015).unwrap_or_else(|| up5(1.75 * bolt));
        let gauge = params
            .gauge
            .filter(|&g| g > 0.03)
            .unwrap_or_else(|| up5(0.55 * b).clamp((tw + 2.0 * bolt + 0.03).max(0.06), 0.20));

        let default_ext = if kind == ConnectionKind::EndPlate { 2.0 * edge_distance } else { 0.0 };
        let extension = params.extension.filter(|&x| x >= 0.0).unwrap_or(default_ext);

        let default_stiffeners = match kind {
            ConnectionKind::EndPlate => true,
            ConnectionKind::BasePlate => fixed_base,
            _ => false,
        };
        let stiffeners = params.stiffeners.unwrap_or(default_stiffeners);

        let anchors = match params.anchors {
            Some(2 | 4 | 6 | 8) => params.anchors.unwrap_or(4),
            _ => 4,
        };
        let anchor_length = params.anchor_length.filter(|&l| l > 0.05).unwrap_or_else(|| up10((18.0 * bolt).max(0.35)));
        let grout_thickness = params.grout_thickness.filter(|&g| g >= 0.0).unwrap_or(0.030);
        let gap = params.gap.filter(|&g| g >= 0.0).unwrap_or(0.010);

        let usable_web = (h - 2.0 * tf - 2.0 * edge_distance).max(0.04);
        let default_rows = match kind {
            ConnectionKind::BasePlate => {
                if anchors >= 6 {
                    3
                } else {
                    2
                }
            }
            ConnectionKind::GussetPlate => 3,
            ConnectionKind::Splice => 2,
            _ => ((usable_web / pitch).floor() as u32 + 1).clamp(2, 8),
        };
        let rows = params.rows.filter(|&r| r >= 1).unwrap_or(default_rows);

        let default_cols = match kind {
            ConnectionKind::SinglePlate | ConnectionKind::GussetPlate => 1,
            ConnectionKind::DoubleAngle => 2,
            ConnectionKind::EndPlate | ConnectionKind::Splice | ConnectionKind::BasePlate => 2,
        };
        let cols = params.cols.filter(|&c| c >= 1).unwrap_or(default_cols);

        let (plate_width, plate_height) = match kind {
            ConnectionKind::BasePlate => {
                let gx = (gauge * 0.5).max(tw * 0.5 + 0.75 * bolt + 0.012);
                let out_y = if fixed_base {
                    h * 0.5 + up5(bolt + 0.03) + if anchors == 8 { pitch } else { 0.0 }
                } else {
                    (h * 0.5 - tf - bolt - 0.015).max(0.04)
                };
                let len = up10(if fixed_base {
                    2.0 * (out_y + edge_distance)
                } else {
                    (h + 0.06).max(2.0 * (out_y + edge_distance))
                });
                let wid = up10((b + 0.06).max(2.0 * (gx + edge_distance)));
                (wid, len)
            }
            ConnectionKind::EndPlate => {
                let wid = up10(b.max(gauge + 2.0 * edge_distance));
                let hgt = up10(h + extension + 0.020);
                (wid, hgt)
            }
            ConnectionKind::SinglePlate | ConnectionKind::DoubleAngle => {
                let wid = up10((2.0 * edge_distance + gap).max(0.08));
                let hgt = up10(2.0 * edge_distance + pitch * (rows.saturating_sub(1) as f64));
                (wid, hgt)
            }
            ConnectionKind::GussetPlate => {
                let wid = up10((2.0 * edge_distance + pitch * (rows.saturating_sub(1) as f64) + 0.08).max(0.24));
                let hgt = up10((h + 0.12).max(0.24));
                (wid, hgt)
            }
            ConnectionKind::Splice => {
                let wid = up10(2.0 * (2.0 * edge_distance + pitch * (rows.saturating_sub(1) as f64)) + gap);
                let hgt = up10((h - 2.0 * tf - 0.02).max(0.08));
                (wid, hgt)
            }
        };

        let throat = weld_throat(plate_thickness.min(tf.max(tw)));
        let weld_length = match kind {
            ConnectionKind::BasePlate | ConnectionKind::EndPlate => 2.0 * (h + 2.0 * b - 2.0 * tw),
            ConnectionKind::SinglePlate | ConnectionKind::DoubleAngle => 2.0 * plate_height,
            ConnectionKind::GussetPlate => 2.0 * plate_width,
            ConnectionKind::Splice => 4.0 * plate_height,
        };

        let plate_vol = plate_thickness * plate_width * plate_height;
        let bolt_count = (rows * cols) as f64;
        let bolt_vol = bolt_count * std::f64::consts::PI * (bolt * 0.5).powi(2) * (2.0 * plate_thickness + 0.04);
        let stiff_vol = if stiffeners {
            2.0 * plate_thickness * ((b - tw).max(0.04) * 0.5) * (h - 2.0 * tf).max(0.06)
        } else {
            0.0
        };
        let weight_kg = ((plate_vol + bolt_vol + stiff_vol) * STEEL_DENSITY * 100.0).round() / 100.0;

        let bolt_grade = params.bolt_grade.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or("8.8").to_string();

        Self {
            kind,
            plate_thickness,
            plate_width,
            plate_height,
            bolt_diameter: bolt,
            bolt_grade,
            rows,
            cols,
            pitch,
            gauge,
            edge_distance,
            extension,
            stiffeners,
            base_mode,
            anchor_shape,
            anchors,
            anchor_length,
            grout_thickness,
            gap,
            support_clearance: 0.0,
            weld_throat: throat,
            weld_length: (weld_length * 1000.0).round() / 1000.0,
            weight_kg,
        }
    }

    /// Build the instance parameter maps (`overrides` and `text_overrides`)
    /// for a placed connection component.
    pub fn instance_overrides(
        &self,
        primary: MemberSection,
        secondary: Option<MemberSection>,
    ) -> (BTreeMap<String, f64>, BTreeMap<String, String>) {
        let s = secondary.unwrap_or(primary);
        let mut nums = BTreeMap::new();
        nums.insert("member_depth".into(), s.depth);
        nums.insert("member_width".into(), s.width);
        nums.insert("web_thickness".into(), s.web);
        nums.insert("flange_thickness".into(), s.flange);
        nums.insert("host_depth".into(), primary.depth);
        nums.insert("host_width".into(), primary.width);
        nums.insert("host_web".into(), primary.web);
        nums.insert("host_flange".into(), primary.flange);
        nums.insert("plate_thickness".into(), self.plate_thickness);
        nums.insert("plate_width".into(), self.plate_width);
        nums.insert("plate_height".into(), self.plate_height);
        nums.insert("bolt_diameter".into(), self.bolt_diameter);
        nums.insert("bolt_rows".into(), f64::from(self.rows));
        nums.insert("bolt_cols".into(), f64::from(self.cols));
        nums.insert("pitch".into(), self.pitch);
        nums.insert("gauge".into(), self.gauge);
        nums.insert("edge_distance".into(), self.edge_distance);
        nums.insert("extension".into(), self.extension);
        nums.insert("stiffeners".into(), f64::from(u8::from(self.stiffeners)));
        nums.insert("anchors".into(), f64::from(self.anchors));
        nums.insert("anchor_length".into(), self.anchor_length);
        nums.insert("grout_thickness".into(), self.grout_thickness);
        nums.insert("gap".into(), self.gap);
        nums.insert("support_clearance".into(), self.support_clearance);

        let mut texts = BTreeMap::new();
        if self.kind == ConnectionKind::BasePlate {
            texts.insert("base_mode".into(), self.base_mode.clone());
            texts.insert("anchor_shape".into(), self.anchor_shape.clone());
        }
        texts.insert("bolt_grade".into(), self.bolt_grade.clone());
        (nums, texts)
    }

    /// Build the `CC_SteelConnection` property set attached to the component.
    pub fn property_set(&self, members: &[ElementId]) -> PropertySet {
        let member_list = members.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(",");
        let plate_desc = format!(
            "PL {}x{}x{}",
            mm_str(self.plate_thickness),
            mm_str(self.plate_width.min(self.plate_height)),
            mm_str(self.plate_width.max(self.plate_height))
        );
        let bolt_desc = format!("M{}", mm_str(self.bolt_diameter));
        let text = |name: &str, s: String| Property {
            name: name.to_string(),
            value: PropertyValue::Single(Some(IfcValue { measure: "IfcText".to_string(), value: Atom::Text(s) })),
        };
        let int = |name: &str, v: i64| Property {
            name: name.to_string(),
            value: PropertyValue::Single(Some(IfcValue { measure: "IfcInteger".to_string(), value: Atom::Integer(v) })),
        };
        let real = |name: &str, measure: &str, v: f64| Property {
            name: name.to_string(),
            value: PropertyValue::Single(Some(IfcValue { measure: measure.to_string(), value: Atom::Real(v) })),
        };
        PropertySet {
            name: CONNECTION_PSET.to_string(),
            quantities: false,
            properties: vec![
                text("Kind", self.kind.code().into()),
                text("Members", member_list),
                text("Plate", plate_desc),
                text("Bolt", bolt_desc),
                text("BoltGrade", self.bolt_grade.clone()),
                int("Rows", i64::from(self.rows)),
                int("Cols", i64::from(self.cols)),
                real("WeldThroat", "IfcPositiveLengthMeasure", self.weld_throat),
                real("WeldLength", "IfcPositiveLengthMeasure", self.weld_length),
                real("WeightKg", "IfcMassMeasure", self.weight_kg),
            ],
        }
    }
}

/// Extract the [`MemberSection`] of a column or beam element in `project`.
pub fn member_section(project: &Project, element: &Element) -> Option<MemberSection> {
    let ty = project.element_type(element.type_id)?;
    let is_metal_mat = |mat: &str| {
        matches!(project.profile_material_for(mat), ProfileMaterial::Steel | ProfileMaterial::ColdFormed)
            || mat.to_lowercase().contains("steel")
            || mat.to_lowercase().contains("aço")
            || mat.to_lowercase().contains("aco")
            || mat.to_lowercase().contains("astm")
            || mat.to_lowercase().contains("ar ")
            || mat.to_lowercase().contains("mr ")
    };

    let shape_dims = |shape: &ProfileShape, is_steel: bool| -> MemberSection {
        match *shape {
            ProfileShape::I { d, bf, tw, tf, .. } => {
                MemberSection { depth: d, width: bf, web: tw, flange: tf, shape_kind: "I", is_steel }
            }
            ProfileShape::IAsym { d, bf_top, tf_top, bf_bot, tf_bot, tw } => MemberSection {
                depth: d,
                width: bf_top.max(bf_bot),
                web: tw,
                flange: tf_top.max(tf_bot),
                shape_kind: "I",
                is_steel,
            },
            ProfileShape::Channel { d, bf, tw, tf, .. } => {
                MemberSection { depth: d, width: bf, web: tw, flange: tf, shape_kind: "U", is_steel }
            }
            ProfileShape::T { d, bf, tw, tf, .. } => {
                MemberSection { depth: d, width: bf, web: tw, flange: tf, shape_kind: "I", is_steel }
            }
            ProfileShape::Angle { h, b, t, .. } => {
                MemberSection { depth: h, width: b, web: t, flange: t, shape_kind: "L", is_steel }
            }
            ProfileShape::ColdU { d, bf, t, .. }
            | ProfileShape::ColdLippedU { d, bf, t, .. }
            | ProfileShape::ColdZ { d, bf, t, .. }
            | ProfileShape::ColdHat { d, bf, t, .. } => {
                MemberSection { depth: d, width: bf, web: t, flange: t, shape_kind: "cold", is_steel: true }
            }
            ProfileShape::RectTube { h, b, t, .. } => {
                MemberSection { depth: h, width: b, web: t, flange: t, shape_kind: "tube", is_steel }
            }
            ProfileShape::RoundTube { d, t } => {
                MemberSection { depth: d, width: d, web: t, flange: t, shape_kind: "pipe", is_steel }
            }
            ProfileShape::Round { d } => {
                MemberSection { depth: d, width: d, web: d * 0.5, flange: d * 0.5, shape_kind: "round", is_steel }
            }
            ProfileShape::Rect { b, h, .. } => {
                MemberSection { depth: h, width: b, web: b * 0.2, flange: h * 0.1, shape_kind: "rect", is_steel }
            }
            _ => MemberSection { depth: 0.25, width: 0.15, web: 0.008, flange: 0.012, shape_kind: "I", is_steel },
        }
    };

    match &ty.data {
        TypeData::Column { section, material, profile, .. } => {
            if let Some(pid) = profile
                && let Some(def) = project.profile(*pid)
            {
                let steel = matches!(def.material, ProfileMaterial::Steel | ProfileMaterial::ColdFormed)
                    || is_metal_mat(material);
                return Some(shape_dims(&def.shape, steel));
            }
            let steel =
                is_metal_mat(material) || matches!(section, ColumnSection::Profile { .. } | ColumnSection::Pipe { .. });
            Some(shape_dims(&ProfileShape::from_column_section(*section), steel))
        }
        TypeData::Beam { width, height, material, shape, profile, .. } => {
            if let Some(pid) = profile
                && let Some(def) = project.profile(*pid)
            {
                let steel = matches!(def.material, ProfileMaterial::Steel | ProfileMaterial::ColdFormed)
                    || is_metal_mat(material);
                return Some(shape_dims(&def.shape, steel));
            }
            let steel = is_metal_mat(material) || !matches!(shape, SectionShape::Solid);
            Some(shape_dims(&ProfileShape::from_beam(*width, *height, *shape), steel))
        }
        _ => None,
    }
}

/// 3D centerline and orientation of a structural column or beam.
#[derive(Debug, Clone, Copy)]
pub struct MemberAxis {
    pub id: ElementId,
    pub level: LevelId,
    pub category: Category,
    pub start: DVec3,
    pub end: DVec3,
    /// Unit vector from `start` to `end`.
    pub dir: DVec3,
    /// Unit vector in plan along the member's web/depth direction (or X axis for columns).
    pub plan_dir: DVec2,
    pub length: f64,
    pub section: MemberSection,
    pub is_vertical: bool,
    pub is_sloped: bool,
}

impl MemberAxis {
    /// Extract the 3D axis of a column or beam element.
    pub fn of(project: &Project, element: &Element) -> Option<Self> {
        let section = member_section(project, element)?;
        match &element.kind {
            ElementKind::Column(c) => {
                let (z0, z1) = project.column_span(element, c);
                let start = c.position.extend(z0);
                let end = c.position.extend(z1);
                let length = (z1 - z0).max(0.01);
                let rot = if c.rotation_deg.is_finite() { c.rotation_deg.to_radians() } else { 0.0 };
                // Profile depth runs along +Y at rotation 0, so rotated +Y is (-sin, cos).
                let plan_dir = DVec2::new(-rot.sin(), rot.cos());
                Some(Self {
                    id: element.id,
                    level: element.level,
                    category: Category::Column,
                    start,
                    end,
                    dir: DVec3::Z,
                    plan_dir,
                    length,
                    section,
                    is_vertical: true,
                    is_sloped: false,
                })
            }
            ElementKind::Beam(b) => {
                let (a2, b2) = project.beam_axis(element, b);
                let (z_bot, z_top) = project.beam_z(element, b);
                let z0 = (z_bot + z_top) * 0.5;
                let z1 = z0 + b.rise();
                let start = a2.extend(z0);
                let end = b2.extend(z1);
                let delta = end - start;
                let length = delta.length();
                if length < 1e-4 {
                    return None;
                }
                let dir = delta / length;
                let plan_dir = (b2 - a2).try_normalize().unwrap_or(DVec2::X);
                let rise_ratio = dir.z.abs();
                Some(Self {
                    id: element.id,
                    level: element.level,
                    category: Category::Beam,
                    start,
                    end,
                    dir,
                    plan_dir,
                    length,
                    section,
                    is_vertical: rise_ratio > 0.98,
                    is_sloped: rise_ratio > 0.08 && rise_ratio <= 0.98,
                })
            }
            _ => None,
        }
    }

    /// True if this member acts as a diagonal brace or truss web diagonal.
    pub fn is_brace_or_web(&self) -> bool {
        if self.category != Category::Beam {
            return false;
        }
        let rise_ratio = self.dir.z.abs();
        // Diagonal slope (> ~14 deg) or typical bracing profile (round bar, angle).
        rise_ratio > 0.24 || matches!(self.section.shape_kind, "round" | "L")
    }
}

/// A recognized candidate joint where a connection macro can be placed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognizedConnection {
    pub kind: ConnectionKind,
    /// Connected member IDs (primary/host member first, then secondary/bolted members).
    pub members: Vec<ElementId>,
    /// Level where the connection belongs.
    pub level: LevelId,
    /// World 3D position `[x, y, z]` of the connection origin.
    pub position: DVec3,
    /// Plan rotation in degrees counter-clockwise.
    pub rotation_deg: f64,
    /// Resolved engineering specification for this joint.
    pub spec: ConnectionSpec,
}

/// Closest points between a 3D point `p` and segment `a..=b`, returning
/// `(t_along_ab_in_0_1, closest_point, distance)`.
fn point_to_segment(p: DVec3, a: DVec3, b: DVec3) -> (f64, DVec3, f64) {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-12 {
        return (0.0, a, p.distance(a));
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    let q = a + ab * t;
    (t, q, p.distance(q))
}

/// Recognize the best connection macro for a user-supplied list of member IDs.
pub fn recognize_for_members(
    project: &Project,
    member_ids: &[ElementId],
    explicit_kind: Option<ConnectionKind>,
    params: &ConnectionParams,
) -> Option<RecognizedConnection> {
    let axes: Vec<MemberAxis> =
        member_ids.iter().filter_map(|id| project.element(*id)).filter_map(|e| MemberAxis::of(project, e)).collect();
    if axes.is_empty() {
        return None;
    }

    // 1. Single member -> BasePlate on a column (or end plate at start of a beam).
    if axes.len() == 1 {
        let m = axes[0];
        let kind = explicit_kind.unwrap_or(ConnectionKind::BasePlate);
        let spec = ConnectionSpec::resolve(kind, m.section, None, params);
        let rot = m.plan_dir.to_angle().to_degrees();
        return Some(RecognizedConnection {
            kind,
            members: vec![m.id],
            level: m.level,
            position: m.start,
            rotation_deg: rot,
            spec,
        });
    }

    // Separate columns, chord/girder beams, and diagonal braces/webs.
    let mut primary_idx = 0;
    for (i, ax) in axes.iter().enumerate() {
        if ax.category == Category::Column {
            primary_idx = i;
            break;
        }
        if !ax.is_brace_or_web() && axes[primary_idx].is_brace_or_web() {
            primary_idx = i;
        }
    }
    let primary = axes[primary_idx];
    let secondaries: Vec<MemberAxis> =
        axes.iter().enumerate().filter(|(i, _)| *i != primary_idx).map(|(_, a)| *a).collect();
    let secondary = secondaries[0];

    // Find the joint point: closest end of `secondary` to `primary`'s segment.
    let (_, q_start, d_start) = point_to_segment(secondary.start, primary.start, primary.end);
    let (_, q_end, d_end) = point_to_segment(secondary.end, primary.start, primary.end);
    let (sec_end_pt, host_pt, sec_out_dir) = if d_start <= d_end {
        (secondary.start, q_start, secondary.dir)
    } else {
        (secondary.end, q_end, -secondary.dir)
    };

    let auto_kind = if secondaries.iter().any(MemberAxis::is_brace_or_web) {
        ConnectionKind::GussetPlate
    } else if primary.category == Category::Column && secondary.category == Category::Beam {
        // Check if the beam approaches the column flange (along column depth axis `plan_dir`) or web.
        let beam_plan = sec_out_dir.truncate().try_normalize().unwrap_or(secondary.plan_dir);
        let align_flange = beam_plan.dot(primary.plan_dir).abs();
        if align_flange >= 0.65 || secondary.is_sloped { ConnectionKind::EndPlate } else { ConnectionKind::SinglePlate }
    } else if primary.category == Category::Beam && secondary.category == Category::Beam {
        let cos_angle = primary.dir.dot(secondary.dir).abs();
        let plan_cos = primary.plan_dir.dot(secondary.plan_dir).abs();
        let end_to_end = host_pt.distance(primary.start) < 0.25 || host_pt.distance(primary.end) < 0.25;
        if cos_angle > 0.98 && end_to_end {
            ConnectionKind::Splice
        } else if plan_cos > 0.95 && end_to_end && (primary.is_sloped || secondary.is_sloped) {
            ConnectionKind::EndPlate
        } else {
            ConnectionKind::SinglePlate
        }
    } else {
        ConnectionKind::EndPlate
    };

    let kind = explicit_kind.unwrap_or(auto_kind);
    let mut spec = ConnectionSpec::resolve(kind, primary.section, Some(secondary.section), params);

    // Position and orientation:
    // +X of the connection component points from the host towards the secondary member.
    let out_plan = sec_out_dir.truncate().try_normalize().unwrap_or(secondary.plan_dir);
    let rotation_deg = out_plan.to_angle().to_degrees();
    let (offset_dist, support_clearance) = if primary.category == Category::Column {
        let align_flange = out_plan.dot(primary.plan_dir).abs();
        if align_flange >= 0.65 {
            (primary.section.depth * 0.5, 0.0)
        } else {
            (primary.section.web * 0.5, ((primary.section.width - primary.section.web) * 0.5).max(0.0))
        }
    } else if matches!(kind, ConnectionKind::Splice | ConnectionKind::EndPlate) {
        (0.0, 0.0)
    } else {
        (primary.section.web * 0.5, ((primary.section.width - primary.section.web) * 0.5).max(0.0))
    };
    spec.support_clearance = support_clearance;
    let joint_pos = (host_pt.truncate() + out_plan * offset_dist).extend(sec_end_pt.z);

    let mut ordered_members = vec![primary.id];
    for s in &secondaries {
        if !ordered_members.contains(&s.id) {
            ordered_members.push(s.id);
        }
    }

    Some(RecognizedConnection {
        kind,
        members: ordered_members,
        level: primary.level,
        position: joint_pos,
        rotation_deg,
        spec,
    })
}

/// Automatically scan structural members in `project` (filtered by `only_ids`
/// or `only_level`) and recognize all candidate steel connections:
/// - Column bases (`BasePlate`)
/// - Bracing & truss nodes (`GussetPlate`)
/// - Beam-to-column joints (`EndPlate` / `SinglePlate`)
/// - Collinear splices (`Splice`) and pitched apex ridges (`EndPlate`)
/// - Secondary beam-to-girder joints (`SinglePlate`)
pub fn recognize_all(
    project: &Project,
    only_ids: &[ElementId],
    only_level: Option<LevelId>,
    params: &ConnectionParams,
) -> Vec<RecognizedConnection> {
    let filter_set: std::collections::BTreeSet<ElementId> = only_ids.iter().copied().collect();
    let axes: Vec<MemberAxis> = project
        .elements
        .values()
        .filter(|e| {
            if !filter_set.is_empty() {
                filter_set.contains(&e.id)
            } else if let Some(lvl) = only_level {
                e.level == lvl
            } else {
                true
            }
        })
        .filter_map(|e| MemberAxis::of(project, e))
        .filter(|ax| !filter_set.is_empty() || ax.section.is_steel)
        .collect();

    let mut out: Vec<RecognizedConnection> = Vec::new();
    let mut seen_pairs: std::collections::BTreeSet<(ConnectionKind, Vec<u64>)> = std::collections::BTreeSet::new();

    let mut push_unique = |rc: RecognizedConnection| {
        let mut key_ids: Vec<u64> = rc.members.iter().map(|id| id.0).collect();
        key_ids.sort_unstable();
        if seen_pairs.insert((rc.kind, key_ids)) {
            out.push(rc);
        }
    };

    // 1. Column bases (skip short truss posts whose base sits on a bottom chord beam).
    let beams: Vec<MemberAxis> = axes.iter().copied().filter(|a| a.category == Category::Beam).collect();
    for col in axes.iter().filter(|a| a.category == Category::Column) {
        let sits_on_chord = beams.iter().any(|b| {
            let (_, _, dist) = point_to_segment(col.start, b.start, b.end);
            dist < (b.section.depth + 0.05)
        });
        if !sits_on_chord
            && let Some(rc) = recognize_for_members(project, &[col.id], Some(ConnectionKind::BasePlate), params)
        {
            push_unique(rc);
        }
    }

    // 2. Bracing and truss gusset nodes: group braces/diagonals meeting a host column or chord at the same node.
    let hosts: Vec<MemberAxis> = axes.iter().copied().filter(|a| !a.is_brace_or_web()).collect();
    let braces: Vec<MemberAxis> = axes.iter().copied().filter(|a| a.is_brace_or_web()).collect();

    for host in &hosts {
        let tol = (host.section.depth.max(host.section.width) * 0.75).max(0.15);
        // Cluster brace endpoints along the host.
        let mut clusters: Vec<(DVec3, Vec<ElementId>)> = Vec::new();
        for br in &braces {
            if br.id == host.id {
                continue;
            }
            for end_pt in [br.start, br.end] {
                let (_, q, dist) = point_to_segment(end_pt, host.start, host.end);
                if dist <= tol {
                    if let Some((_, list)) = clusters.iter_mut().find(|(c, _)| c.distance(q) < 0.20) {
                        if !list.contains(&br.id) {
                            list.push(br.id);
                        }
                    } else {
                        clusters.push((q, vec![br.id]));
                    }
                }
            }
        }
        for (_, br_ids) in clusters {
            let mut ids = vec![host.id];
            ids.extend(br_ids);
            if let Some(rc) = recognize_for_members(project, &ids, Some(ConnectionKind::GussetPlate), params) {
                push_unique(rc);
            }
        }
    }

    // 3. Beam-to-column joints (for non-brace beams).
    let main_beams: Vec<MemberAxis> =
        axes.iter().copied().filter(|a| a.category == Category::Beam && !a.is_brace_or_web()).collect();
    for col in axes.iter().filter(|a| a.category == Category::Column) {
        let tol = (col.section.depth.max(col.section.width) * 0.75).max(0.15);
        for bm in &main_beams {
            let (_, _, d0) = point_to_segment(bm.start, col.start, col.end);
            let (_, _, d1) = point_to_segment(bm.end, col.start, col.end);
            if d0.min(d1) <= tol
                && let Some(rc) = recognize_for_members(project, &[col.id, bm.id], None, params)
            {
                push_unique(rc);
            }
        }
    }

    // 4. Beam-to-beam joints (splices, apexes, and secondary-to-primary shear joints).
    for (i, a) in main_beams.iter().enumerate() {
        for b in &main_beams[i + 1..] {
            let tol = (a.section.depth.max(b.section.depth) * 0.75).max(0.12);
            let (_, _, da0) = point_to_segment(b.start, a.start, a.end);
            let (_, _, da1) = point_to_segment(b.end, a.start, a.end);
            let (_, _, db0) = point_to_segment(a.start, b.start, b.end);
            let (_, _, db1) = point_to_segment(a.end, b.start, b.end);
            if da0.min(da1).min(db0).min(db1) <= tol
                && let Some(rc) = recognize_for_members(project, &[a.id, b.id], None, params)
            {
                push_unique(rc);
            }
        }
    }

    out
}

/// Build the [`ComponentDef`] (with its parametric Lua macro script) for `kind`.
pub fn connection_component_def(kind: ConnectionKind, material: &str) -> ComponentDef {
    let source = macro_script(kind).to_string();
    let mut def = ComponentDef {
        kind: ComponentKind::Generic,
        params: Vec::new(),
        items: vec![ComponentItem {
            name: "Macro".to_string(),
            shape: ComponentShape::Script { source },
            material: material.to_string(),
            in_plan: true,
            void: false,
            plan_only: false,
            detail: DetailLevels::default(),
        }],
        opening: None,
        mep: None,
        ifc: None,
        sheet: Vec::new(),
        ref_planes: Vec::new(),
        ref_lines: Vec::new(),
        dimensions: Vec::new(),
        equals: Vec::new(),
        connectors: Vec::new(),
        hotspots: Vec::new(),
    };
    if let Ok(decl) = crate_script_declarations(kind) {
        def.params = decl;
    }
    def
}

/// Build the initial parameter list matching `macro_script(kind)` so `ComponentDef`
/// is immediately populated even before `cc_script::object::sync` runs.
fn crate_script_declarations(kind: ConnectionKind) -> Result<Vec<crate::ComponentParam>, ()> {
    use crate::ComponentParam;
    use crate::ParamKind as K;
    use crate::component_script::ParamChoice;

    let mut p = vec![
        ComponentParam {
            name: "member_depth".into(),
            value: 0.30,
            kind: K::Length,
            instance: true,
            label: "Member Depth".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "member_width".into(),
            value: 0.15,
            kind: K::Length,
            instance: true,
            label: "Member Width".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "web_thickness".into(),
            value: 0.0071,
            kind: K::Length,
            instance: true,
            label: "Web Thickness".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "flange_thickness".into(),
            value: 0.0107,
            kind: K::Length,
            instance: true,
            label: "Flange Thickness".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "host_depth".into(),
            value: 0.30,
            kind: K::Length,
            instance: true,
            label: "Host Depth".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "host_width".into(),
            value: 0.15,
            kind: K::Length,
            instance: true,
            label: "Host Width".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "host_web".into(),
            value: 0.0071,
            kind: K::Length,
            instance: true,
            label: "Host Web".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "host_flange".into(),
            value: 0.0107,
            kind: K::Length,
            instance: true,
            label: "Host Flange".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "plate_thickness".into(),
            value: 0.016,
            kind: K::Length,
            instance: true,
            label: "Plate Thickness".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "plate_width".into(),
            value: 0.20,
            kind: K::Length,
            instance: true,
            label: "Plate Width".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "plate_height".into(),
            value: 0.34,
            kind: K::Length,
            instance: true,
            label: "Plate Height".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "bolt_diameter".into(),
            value: 0.020,
            kind: K::Length,
            instance: true,
            label: "Bolt Diameter".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "bolt_grade".into(),
            value: 0.0,
            kind: K::Text,
            instance: true,
            label: "Bolt Grade".into(),
            text: "8.8".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "bolt_rows".into(),
            value: 3.0,
            kind: K::Number,
            instance: true,
            label: "Bolt Rows".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "bolt_cols".into(),
            value: 2.0,
            kind: K::Number,
            instance: true,
            label: "Bolt Columns".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "pitch".into(),
            value: 0.060,
            kind: K::Length,
            instance: true,
            label: "Bolt Pitch".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "gauge".into(),
            value: 0.090,
            kind: K::Length,
            instance: true,
            label: "Bolt Gauge".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "edge_distance".into(),
            value: 0.035,
            kind: K::Length,
            instance: true,
            label: "Edge Distance".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "extension".into(),
            value: 0.070,
            kind: K::Length,
            instance: true,
            label: "Top Extension".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "stiffeners".into(),
            value: 1.0,
            kind: K::Boolean,
            instance: true,
            label: "Stiffeners".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "anchors".into(),
            value: 4.0,
            kind: K::Number,
            instance: true,
            label: "Anchor Count".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "anchor_length".into(),
            value: 0.40,
            kind: K::Length,
            instance: true,
            label: "Anchor Length".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "grout_thickness".into(),
            value: 0.030,
            kind: K::Length,
            instance: true,
            label: "Grout Thickness".into(),
            from_script: true,
            ..Default::default()
        },
        ComponentParam {
            name: "gap".into(),
            value: 0.010,
            kind: K::Length,
            instance: true,
            label: "Clearance Gap".into(),
            from_script: true,
            ..Default::default()
        },
    ];

    if kind == ConnectionKind::BasePlate {
        p.push(ComponentParam {
            name: "base_mode".into(),
            value: 0.0,
            kind: K::Choice,
            instance: true,
            label: "Base Mode".into(),
            text: "pinned".into(),
            choices: vec![
                ParamChoice { key: "pinned".into(), label: "Pinned".into() },
                ParamChoice { key: "fixed".into(), label: "Fixed".into() },
            ],
            from_script: true,
        });
        p.push(ComponentParam {
            name: "anchor_shape".into(),
            value: 0.0,
            kind: K::Choice,
            instance: true,
            label: "Anchor Shape".into(),
            text: "hooked".into(),
            choices: vec![
                ParamChoice { key: "hooked".into(), label: "Hooked".into() },
                ParamChoice { key: "plate".into(), label: "Washer Plate".into() },
                ParamChoice { key: "straight".into(), label: "Straight".into() },
            ],
            from_script: true,
        });
    }
    Ok(p)
}

/// Parametric Lua macro source for each [`ConnectionKind`].
pub fn macro_script(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::BasePlate => BASE_PLATE_LUA,
        ConnectionKind::EndPlate => END_PLATE_LUA,
        ConnectionKind::SinglePlate => SINGLE_PLATE_LUA,
        ConnectionKind::DoubleAngle => DOUBLE_ANGLE_LUA,
        ConnectionKind::GussetPlate => GUSSET_PLATE_LUA,
        ConnectionKind::Splice => SPLICE_LUA,
    }
}

const BASE_PLATE_LUA: &str = const_concat_base();
const END_PLATE_LUA: &str = const_concat_end();
const SINGLE_PLATE_LUA: &str = const_concat_single();
const DOUBLE_ANGLE_LUA: &str = const_concat_double_angle();
const GUSSET_PLATE_LUA: &str = const_concat_gusset();
const SPLICE_LUA: &str = const_concat_splice();

const fn const_concat_base() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.30, instance = true }
param.length { "member_width", label = "Member Width", default = 0.15, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.0071, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.0107, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.020, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.28, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.40, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.020, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 2, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 2, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.070, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.100, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.040, instance = true }
param.length { "extension", label = "Top Extension", default = 0.0, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = false, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }
param.choice { "base_mode", { { "pinned", "Pinned" }, { "fixed", "Fixed" } }, label = "Base Mode", default = "pinned", instance = true }
param.choice { "anchor_shape", { { "hooked", "Hooked" }, { "plate", "Washer Plate" }, { "straight", "Straight" } }, label = "Anchor Shape", default = "hooked", instance = true }

function geometry(p)
  local out = {}
  local tp = math.max(0.008, p.plate_thickness)
  local w = math.max(0.12, p.plate_width)
  local l = math.max(0.12, p.plate_height)
  local d = math.max(0.012, p.bolt_diameter)
  local grout = math.max(0.0, p.grout_thickness)
  local alen = math.max(0.15, p.anchor_length)
  local h = math.max(0.10, p.member_depth)
  local tw = math.max(0.004, p.web_thickness)

  -- Base plate centered at column foot (x along depth, y across width)
  out[#out + 1] = box { -l / 2, -w / 2, 0, l, w, tp, name = "Base plate" }

  -- Grout pad under the base plate
  if grout > 0.002 then
    out[#out + 1] = box { -l / 2 - 0.02, -w / 2 - 0.02, -grout, l + 0.04, w + 0.04, grout, name = "Grout pad" }
  end

  -- Anchor bolt positions
  local gx = math.max(p.gauge / 2, tw / 2 + d + 0.01)
  local gy = (p.base_mode == "fixed") and math.min(l / 2 - p.edge_distance, h / 2 + d + 0.025)
    or math.max(0.035, h / 2 - p.flange_thickness - d - 0.015)
  local coords = {}
  if p.anchors <= 2 then
    coords = { { 0, -gx }, { 0, gx } }
  else
    coords = { { -gy, -gx }, { -gy, gx }, { gy, -gx }, { gy, gx } }
    if p.anchors >= 6 then
      coords[#coords + 1] = { 0, -gx }
      coords[#coords + 1] = { 0, gx }
    end
  end

  local function hex_pts(cu, cv, r)
    local pts = {}
    for i = 0, 5 do
      local a = 2 * math.pi * i / 6
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end

  for _, c in ipairs(coords) do
    local ax, ay = c[1], c[2]
    -- Anchor rod + washer + hexagonal nut
    out[#out + 1] = cylinder { ax, ay, -grout - alen, d / 2, alen + grout + tp + 2 * d, name = "Anchor rod" }
    out[#out + 1] = cylinder { ax, ay, tp, d * 1.15, 0.16 * d, name = "Anchor washer" }
    out[#out + 1] = extrusion { points = hex_pts(ax, ay, 0.92 * d), z = tp + 0.16 * d, height = 0.85 * d, name = "Anchor nut" }
    if p.anchor_shape == "plate" then
      local pw = 3.5 * d
      out[#out + 1] = box { ax - pw / 2, ay - pw / 2, -grout - alen, pw, pw, 0.6 * d, name = "Anchor washer plate" }
    elseif p.anchor_shape == "hooked" then
      local sign = (ax >= 0) and 1 or -1
      out[#out + 1] = box { ax, ay - d / 2, -grout - alen, sign * 4 * d, d, d, name = "Anchor hook" }
    end
  end

  -- Stiffener ribs on fixed bases
  if p.stiffeners or p.base_mode == "fixed" then
    local run = math.max(0.04, (l - h) / 2 - 0.01)
    local hs = math.min(0.25, math.max(0.10, 1.5 * run))
    local ts = math.max(0.010, tw * 1.2)
    out[#out + 1] = box { h / 2, -ts / 2, tp, run, ts, hs, name = "Stiffener +X" }
    out[#out + 1] = box { -h / 2 - run, -ts / 2, tp, run, ts, hs, name = "Stiffener -X" }
  end

  return out
end
"#
}

const fn const_concat_end() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.30, instance = true }
param.length { "member_width", label = "Member Width", default = 0.15, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.0071, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.0107, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.016, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.18, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.38, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.020, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 3, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 2, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.065, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.090, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.035, instance = true }
param.length { "extension", label = "Top Extension", default = 0.070, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = true, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }

function geometry(p)
  local out = {}
  local tp = math.max(0.008, p.plate_thickness)
  local w = math.max(0.10, p.plate_width)
  local h = math.max(0.12, p.member_depth)
  local ext = math.max(0.0, p.extension)
  local d = math.max(0.012, p.bolt_diameter)
  local tf = math.max(0.006, p.flange_thickness)
  local htf = math.max(0.006, p.host_flange)
  local z0 = -h / 2 - 0.015
  local z1 = h / 2 + ext

  -- Vertical end plate on column face (x from 0 to tp, y across width, z vertical)
  out[#out + 1] = box { 0, -w / 2, z0, tp, w, z1 - z0, name = "End plate" }

  -- Bolt columns across Y
  local ys = { -p.gauge / 2, p.gauge / 2 }
  if p.bolt_cols >= 4 then
    ys = { -p.gauge / 2 - p.pitch, -p.gauge / 2, p.gauge / 2, p.gauge / 2 + p.pitch }
  end

  -- Bolt rows along Z
  local zs = {}
  if ext >= 2 * p.edge_distance - 1e-4 then
    zs[#zs + 1] = h / 2 + ext / 2
  end
  local n = math.max(1, math.floor(p.bolt_rows + 0.5))
  local hi = h / 2 - tf - p.edge_distance
  local lo = -h / 2 + tf + p.edge_distance
  if n == 1 or hi <= lo then
    zs[#zs + 1] = (hi + lo) / 2
  else
    for k = 0, n - 1 do
      zs[#zs + 1] = hi - (hi - lo) * (k / (n - 1))
    end
  end

  local function reg_poly(cu, cv, r, n)
    local pts = {}
    for i = 0, n - 1 do
      local a = 2 * math.pi * i / n
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end
  local function bolt_x(x0, grip, cy, cz, bd, label)
    local wh = 0.14 * bd
    local hh = 0.65 * bd
    local nh = 0.80 * bd
    out[#out + 1] = profile { plane = "side", points = reg_poly(cy, cz, 0.92 * bd, 6), offset = x0 - wh - hh, depth = hh, name = label .. " head" }
    out[#out + 1] = profile { plane = "side", points = reg_poly(cy, cz, 1.05 * bd, 12), offset = x0 - wh, depth = wh, name = label .. " washer" }
    out[#out + 1] = profile { plane = "side", points = reg_poly(cy, cz, 0.50 * bd, 12), offset = x0, depth = grip + wh + nh + 0.25 * bd, name = label }
    out[#out + 1] = profile { plane = "side", points = reg_poly(cy, cz, 1.05 * bd, 12), offset = x0 + grip, depth = wh, name = label .. " nut washer" }
    out[#out + 1] = profile { plane = "side", points = reg_poly(cy, cz, 0.92 * bd, 6), offset = x0 + grip + wh, depth = nh, name = label .. " nut" }
  end

  local grip = tp + htf
  for _, z in ipairs(zs) do
    for _, y in ipairs(ys) do
      bolt_x(-htf, grip, y, z, d, "Bolt")
    end
  end

  -- Column web continuity stiffeners (aligned with top and bottom beam flanges)
  if p.stiffeners then
    local hc = math.max(0.12, p.host_depth)
    local bc = math.max(0.08, p.host_width)
    local twc = math.max(0.005, p.host_web)
    local sw = math.max(0.03, (bc - twc) / 2)
    local sl = math.max(0.06, hc - 2 * htf)
    for _, zf in ipairs { h / 2 - tf / 2, -h / 2 + tf / 2 } do
      out[#out + 1] = box { -htf - sl, twc / 2, zf - tf / 2, sl, sw, tf, name = "Column stiffener +Y" }
      out[#out + 1] = box { -htf - sl, -twc / 2 - sw, zf - tf / 2, sl, sw, tf, name = "Column stiffener -Y" }
    end
  end

  return out
end
"#
}

const fn const_concat_single() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.30, instance = true }
param.length { "member_width", label = "Member Width", default = 0.15, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.0071, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.0107, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.010, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.10, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.22, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.020, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 3, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 1, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.065, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.060, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.035, instance = true }
param.length { "extension", label = "Top Extension", default = 0.0, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = false, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }
param.length { "support_clearance", label = "Support Clearance", default = 0.0, instance = true }

function geometry(p)
  local out = {}
  local tp = math.max(0.006, p.plate_thickness)
  local tw = math.max(0.004, p.web_thickness)
  local tf = math.max(0.005, p.flange_thickness)
  local h = math.max(0.10, p.member_depth)
  local d = math.max(0.012, p.bolt_diameter)
  local rows = math.max(1, math.floor(p.bolt_rows + 0.5))
  local cols = math.max(1, math.floor(p.bolt_cols + 0.5))
  local gap = math.max(0.005, p.gap)
  local clear = math.max(0.0, p.support_clearance or 0.0)
  local edge = math.max(0.020, p.edge_distance)
  local pitch = math.max(0.035, p.pitch)
  local wf = clear + gap + 2 * edge + (cols - 1) * pitch
  local max_hf = math.max(0.06, h - 2 * tf - 0.016)
  local hf = math.min(2 * edge + (rows - 1) * pitch, max_hf)

  -- Shear tab beside the beam web (+Y side of web), starting at host web
  out[#out + 1] = box { 0, tw / 2, -hf / 2, wf, tp, hf, name = "Single plate (fin)" }

  local function reg_poly(cu, cv, r, n)
    local pts = {}
    for i = 0, n - 1 do
      local a = 2 * math.pi * i / n
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end
  local function bolt_y(cx, y0, grip, cz, bd, label)
    local wh = 0.14 * bd
    local hh = 0.65 * bd
    local nh = 0.80 * bd
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.92 * bd, 6), offset = y0 - wh - hh, depth = hh, name = label .. " head" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 1.05 * bd, 12), offset = y0 - wh, depth = wh, name = label .. " washer" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.50 * bd, 12), offset = y0, depth = grip + wh + nh + 0.25 * bd, name = label }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 1.05 * bd, 12), offset = y0 + grip, depth = wh, name = label .. " nut washer" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.92 * bd, 6), offset = y0 + grip + wh, depth = nh, name = label .. " nut" }
  end

  local grip = tp + tw
  local hi = hf / 2 - math.min(edge, hf * 0.25)
  local lo = -hi
  for r = 0, rows - 1 do
    local z = (rows == 1) and 0.0 or (hi - (hi - lo) * (r / (rows - 1)))
    for c = 0, cols - 1 do
      local x = clear + gap + edge + c * pitch
      bolt_y(x, -tw / 2, grip, z, d, "Shear bolt")
    end
  end

  return out
end
"#
}

const fn const_concat_double_angle() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.30, instance = true }
param.length { "member_width", label = "Member Width", default = 0.15, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.0071, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.0107, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.010, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.09, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.22, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.020, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 3, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 2, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.065, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.090, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.035, instance = true }
param.length { "extension", label = "Top Extension", default = 0.0, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = false, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }
param.length { "support_clearance", label = "Support Clearance", default = 0.0, instance = true }

function geometry(p)
  local out = {}
  local t = math.max(0.006, p.plate_thickness)
  local tw = math.max(0.004, p.web_thickness)
  local tf = math.max(0.005, p.flange_thickness)
  local md = math.max(0.10, p.member_depth)
  local d = math.max(0.012, p.bolt_diameter)
  local rows = math.max(1, math.floor(p.bolt_rows + 0.5))
  local edge = math.max(0.020, p.edge_distance)
  local pitch = math.max(0.035, p.pitch)
  local gap = math.max(0.005, p.gap)
  local clear = math.max(0.0, p.support_clearance or 0.0)
  local leg = math.max(0.065, p.plate_width)
  local web_leg = math.max(leg, clear + gap + 2 * edge)
  local max_h = math.max(0.06, md - 2 * tf - 0.016)
  local h = math.min(2 * edge + (rows - 1) * pitch, max_h)

  -- Two angles (+Y and -Y sides of the beam web)
  out[#out + 1] = box { 0, tw / 2, -h / 2, web_leg, t, h, name = "Angle cleat +Y web leg" }
  out[#out + 1] = box { 0, tw / 2 + t, -h / 2, t, leg - t, h, name = "Angle cleat +Y support leg" }
  out[#out + 1] = box { 0, -tw / 2 - t, -h / 2, web_leg, t, h, name = "Angle cleat -Y web leg" }
  out[#out + 1] = box { 0, -tw / 2 - leg, -h / 2, t, leg - t, h, name = "Angle cleat -Y support leg" }

  local function reg_poly(cu, cv, r, n)
    local pts = {}
    for i = 0, n - 1 do
      local a = 2 * math.pi * i / n
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end
  local function bolt_axis(pl, off, grip, cu, cv, bd, label)
    local wh = 0.14 * bd
    local hh = 0.65 * bd
    local nh = 0.80 * bd
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.92 * bd, 6), offset = off - wh - hh, depth = hh, name = label .. " head" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 1.05 * bd, 12), offset = off - wh, depth = wh, name = label .. " washer" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.50 * bd, 12), offset = off, depth = grip + wh + nh + 0.25 * bd, name = label }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 1.05 * bd, 12), offset = off + grip, depth = wh, name = label .. " nut washer" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.92 * bd, 6), offset = off + grip + wh, depth = nh, name = label .. " nut" }
  end

  local web_grip = 2 * t + tw
  local web_x = clear + gap + edge
  local hi = h / 2 - math.min(edge, h * 0.25)
  local lo = -hi
  for r = 0, rows - 1 do
    local z = (rows == 1) and 0.0 or (hi - (hi - lo) * (r / (rows - 1)))
    bolt_axis("front", -web_grip / 2, web_grip, web_x, z, d, "Web bolt")
    bolt_axis("side", 0, 2 * t, tw / 2 + leg * 0.55, z, d, "Support bolt +Y")
    bolt_axis("side", 0, 2 * t, -tw / 2 - leg * 0.55, z, d, "Support bolt -Y")
  end

  return out
end
"#
}

const fn const_concat_gusset() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.10, instance = true }
param.length { "member_width", label = "Member Width", default = 0.10, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.008, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.008, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.012, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.28, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.24, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.016, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 3, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 1, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.055, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.060, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.030, instance = true }
param.length { "extension", label = "Top Extension", default = 0.0, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = false, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }

function geometry(p)
  local out = {}
  local tp = math.max(0.008, p.plate_thickness)
  local w = math.max(0.18, p.plate_width)
  local h = math.max(0.16, p.plate_height)
  local d = math.max(0.012, p.bolt_diameter)
  local rows = math.max(2, math.floor(p.bolt_rows + 0.5))
  local pitch = math.max(0.035, p.pitch)
  local edge = math.max(0.025, p.edge_distance)

  -- Chamfered vertical gusset plate in the X-Z plane (extruded across Y from -tp/2 to +tp/2)
  out[#out + 1] = profile {
    plane = "front",
    offset = -tp / 2,
    depth = tp,
    points = {
      { 0, -h / 2 },
      { w, -h / 2 },
      { w, h / 6 },
      { w * 0.65, h / 2 },
      { 0, h / 2 },
    },
    name = "Gusset plate",
  }

  local function reg_poly(cu, cv, r, n)
    local pts = {}
    for i = 0, n - 1 do
      local a = 2 * math.pi * i / n
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end
  local function bolt_y(cx, y0, grip, cz, bd, label)
    local wh = 0.14 * bd
    local hh = 0.65 * bd
    local nh = 0.80 * bd
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.92 * bd, 6), offset = y0 - wh - hh, depth = hh, name = label .. " head" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 1.05 * bd, 12), offset = y0 - wh, depth = wh, name = label .. " washer" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.50 * bd, 12), offset = y0, depth = grip + wh + nh + 0.25 * bd, name = label }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 1.05 * bd, 12), offset = y0 + grip, depth = wh, name = label .. " nut washer" }
    out[#out + 1] = profile { plane = "front", points = reg_poly(cx, cz, 0.92 * bd, 6), offset = y0 + grip + wh, depth = nh, name = label .. " nut" }
  end

  local grip = tp + 2 * p.web_thickness
  for r = 0, rows - 1 do
    local dist = edge + 0.04 + r * pitch
    local bx = math.min(w - edge, dist * 0.8)
    local bz = math.min(h / 2 - edge, -h / 4 + dist * 0.5)
    bolt_y(bx, -grip / 2, grip, bz, d, "Gusset bolt")
  end

  return out
end
"#
}

const fn const_concat_splice() -> &'static str {
    r#"
param.length { "member_depth", label = "Member Depth", default = 0.30, instance = true }
param.length { "member_width", label = "Member Width", default = 0.15, instance = true }
param.length { "web_thickness", label = "Web Thickness", default = 0.0071, instance = true }
param.length { "flange_thickness", label = "Flange Thickness", default = 0.0107, instance = true }
param.length { "host_depth", label = "Host Depth", default = 0.30, instance = true }
param.length { "host_width", label = "Host Width", default = 0.15, instance = true }
param.length { "host_web", label = "Host Web", default = 0.0071, instance = true }
param.length { "host_flange", label = "Host Flange", default = 0.0107, instance = true }
param.length { "plate_thickness", label = "Plate Thickness", default = 0.012, instance = true }
param.length { "plate_width", label = "Plate Width", default = 0.28, instance = true }
param.length { "plate_height", label = "Plate Height", default = 0.22, instance = true }
param.length { "bolt_diameter", label = "Bolt Diameter", default = 0.020, instance = true }
param.text   { "bolt_grade", label = "Bolt Grade", default = "8.8", instance = true }
param.number { "bolt_rows", label = "Bolt Rows", default = 2, instance = true }
param.number { "bolt_cols", label = "Bolt Columns", default = 2, instance = true }
param.length { "pitch", label = "Bolt Pitch", default = 0.065, instance = true }
param.length { "gauge", label = "Bolt Gauge", default = 0.090, instance = true }
param.length { "edge_distance", label = "Edge Distance", default = 0.035, instance = true }
param.length { "extension", label = "Top Extension", default = 0.0, instance = true }
param.boolean { "stiffeners", label = "Stiffeners", default = false, instance = true }
param.number { "anchors", label = "Anchor Count", default = 4, instance = true }
param.length { "anchor_length", label = "Anchor Length", default = 0.40, instance = true }
param.length { "grout_thickness", label = "Grout Thickness", default = 0.030, instance = true }
param.length { "gap", label = "Clearance Gap", default = 0.010, instance = true }

function geometry(p)
  local out = {}
  local tp = math.max(0.008, p.plate_thickness)
  local h = math.max(0.14, p.member_depth)
  local b = math.max(0.08, p.member_width)
  local tw = math.max(0.004, p.web_thickness)
  local tf = math.max(0.006, p.flange_thickness)
  local d = math.max(0.012, p.bolt_diameter)
  local l = math.max(0.16, p.plate_width)
  local wh = math.max(0.08, h - 2 * tf - 0.02)

  -- Top and bottom flange splice cover plates
  out[#out + 1] = box { -l / 2, -b / 2, h / 2, l, b, tp, name = "Top flange splice plate" }
  out[#out + 1] = box { -l / 2, -b / 2, -h / 2 - tp, l, b, tp, name = "Bottom flange splice plate" }

  -- Web splice plates (+Y and -Y sides of web)
  out[#out + 1] = box { -l / 2, tw / 2, -wh / 2, l, tp, wh, name = "Web splice plate +Y" }
  out[#out + 1] = box { -l / 2, -tw / 2 - tp, -wh / 2, l, tp, wh, name = "Web splice plate -Y" }

  local function reg_poly(cu, cv, r, n)
    local pts = {}
    for i = 0, n - 1 do
      local a = 2 * math.pi * i / n
      pts[#pts + 1] = { cu + r * math.cos(a), cv + r * math.sin(a) }
    end
    return pts
  end
  local function bolt_axis(pl, off, grip, cu, cv, bd, label)
    local wh = 0.14 * bd
    local hh = 0.65 * bd
    local nh = 0.80 * bd
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.92 * bd, 6), offset = off - wh - hh, depth = hh, name = label .. " head" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 1.05 * bd, 12), offset = off - wh, depth = wh, name = label .. " washer" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.50 * bd, 12), offset = off, depth = grip + wh + nh + 0.25 * bd, name = label }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 1.05 * bd, 12), offset = off + grip, depth = wh, name = label .. " nut washer" }
    out[#out + 1] = profile { plane = pl, points = reg_poly(cu, cv, 0.92 * bd, 6), offset = off + grip + wh, depth = nh, name = label .. " nut" }
  end

  -- Bolts on both sides of the splice
  local xs = { -l * 0.35, -l * 0.15, l * 0.15, l * 0.35 }
  local ys = { -p.gauge / 2, p.gauge / 2 }
  local web_grip = 2 * tp + tw
  for _, x in ipairs(xs) do
    bolt_axis("front", -web_grip / 2, web_grip, x, wh * 0.25, d, "Web bolt top")
    bolt_axis("front", -web_grip / 2, web_grip, x, -wh * 0.25, d, "Web bolt bottom")
    for _, y in ipairs(ys) do
      bolt_axis("plan", h / 2 - tf, tf + tp, x, y, d, "Top flange bolt")
      bolt_axis("plan", -h / 2 - tp, tf + tp, x, y, d, "Bottom flange bolt")
    end
  end

  return out
end
"#
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn connection_kinds_round_trip_codes_and_aliases() {
        for kind in ConnectionKind::ALL {
            assert_eq!(ConnectionKind::from_code(kind.code()), Some(kind));
        }
        assert_eq!(ConnectionKind::from_code("chapa_de_base"), Some(ConnectionKind::BasePlate));
        assert_eq!(ConnectionKind::from_code("fin_plate"), Some(ConnectionKind::SinglePlate));
        assert_eq!(ConnectionKind::from_code("gusset"), Some(ConnectionKind::GussetPlate));
    }

    #[test]
    fn spec_resolves_sensible_fasteners_and_plates() {
        let sec =
            MemberSection { depth: 0.40, width: 0.18, web: 0.008, flange: 0.0135, shape_kind: "I", is_steel: true };
        let spec = ConnectionSpec::resolve(ConnectionKind::EndPlate, sec, Some(sec), &ConnectionParams::default());
        assert_eq!(spec.bolt_diameter, 0.024);
        assert!(spec.plate_thickness >= 0.016);
        assert!(spec.rows >= 2);
        assert_eq!(spec.cols, 2);
        assert!(spec.weight_kg > 5.0);
    }
}
