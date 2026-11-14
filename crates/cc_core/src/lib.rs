//! Core BIM data model for ConstructCraft.
//!
//! The [`Project`] is the single source of truth. Every view, schedule and
//! sheet is a *query* over it; all geometry is derived. Edits go through
//! [`Document::edit`], which records a [`Transaction`] for undo/redo.

pub mod arc;
pub mod bar_marks;
pub mod bar_tag;
pub mod bracing;
pub mod browser;
pub mod cad;
pub mod callout;
pub mod component;
pub mod component_file;
pub mod component_ref;
pub mod component_script;
pub mod connection;
pub mod curtain;
pub mod design_option;
pub mod detail;
pub mod detail_component;
pub mod dim_line;
pub mod dim_refs;
pub mod dim_style;
mod document;
mod element;
pub mod expr;
pub mod fill_pattern;
pub mod graph_binding;
pub mod graphics;
pub mod grid;
mod ids;
pub mod level_mark;
pub mod lightning;
pub mod mass;
pub mod material;
pub mod member_host;
pub mod mep;
pub mod openings;
pub mod pad;
pub mod paint;
pub mod pat;
mod persist;
pub mod phase;
pub mod pile;
pub mod profile;
pub mod profile_catalog;
pub mod profile_sweep;
pub mod profiles_project;
mod project;
pub mod properties;
pub mod ramp;
pub mod raster;
pub mod recovery;
pub mod region;
mod repair;
pub mod roofing;
pub mod scale;
pub mod schedule;
pub mod section_props;
pub mod shaft;
pub mod site;
pub mod sketch;
pub mod sketch_edit;
pub mod stair;
pub mod steel_building;
pub mod structural_material;
pub mod structural_profile;
pub mod sun;
pub mod sweep;
mod template_spec;
pub use template_spec::Defaults;
pub mod title_block;
pub mod topography;
pub mod transform;
pub mod truss;
mod types;
pub mod units;
mod view;
pub mod wall_framing;
pub mod wall_ops;

pub use cad::{CadColorMode, CadDrawing, CadImport, CadLayer, CadPrim, CadUnit};
pub use callout::{Callout, CropBox};
pub use component::{
    ComponentDef, ComponentItem, ComponentKind, ComponentParam, ComponentShape, DoorOperation, ParamKind, ResolveCtx,
    SymbolLine, WindowOperation,
};
pub use component_ref::{PlaneAxis, RefDimension, RefEqual, RefLine, RefPlane};
pub use component_script::{Hotspot, HotspotShape, ParamChoice, ScriptEval, ScriptInput, ScriptOwner, ScriptResult};
pub use connection::{Connection, ConnectionKind, ConnectionParams, MemberEnd};
pub use design_option::{DesignOption, OptionSet};
pub use detail::{DetailLine, DetailStyle, LineStyleDef, LineStyleRef};
pub use detail_component::DetailComponent;
pub use dim_refs::{DimRef, Projection, RefAnchor, RefHeight, RefTarget};
pub use document::{Document, EditCtx, Transaction};
pub use element::{
    BarEnd, BarFace, Beam, BeamLateral, BeamVertical, Ceiling, Column, Component, Dimension, DimensionKind, Element,
    ElementKind, FaceBars, Floor, Foundation, FoundationShape, Grid, Hosted, Leader, LeaderSymbol, LocationLine,
    Railing, Ramp, Rebar, RebarLayout, Roof, RoofShape, Room, ShapedRebar, SpacingZone, Splice, Stair, TextNote, Wall,
    WallSide, WallTop, slab_bars, wall_bars,
};
pub use fill_pattern::{FillPattern, FillPatternKind, PatternLine};
pub use graphics::{
    CategoryGraphics, FillGraphics, FilterOp, FilterParam, FilterRule, FilterUse, GraphicOverride, LineGraphics,
    LinePattern, TemplateIncludes, TemporaryVisibility, ViewFilter, ViewGraphics, ViewTemplate,
};
pub use ids::{
    BrowserSchemeId, ElementId, ElementTypeId, FillPatternId, FilterId, GroupId, LevelId, LineStyleId, OptionId,
    OptionSetId, PhaseFilterId, PhaseId, ProfileId, SystemId, TemplateId, Uid, ViewId,
};
pub use lightning::{ConductorRole, LightningConductor, LpsClass};
pub use material::{Material, MaterialClass, Pattern, PatternFill};
pub use mep::{DeviceKind, MepData, MepSegment, MepSystem, SystemClass, SystemKind, TrayStyle, Valve, ValveKind};
pub use persist::{
    FORMAT_VERSION, PersistError, from_ron, from_ron_with_repairs, from_ron_with_repairs_localized, load,
    load_with_repairs, load_with_repairs_localized, read_to_string_limited, save, to_ron,
};
pub use phase::{Phase, PhaseDisplay, PhaseFilter, PhaseGraphics, PhaseState, PhaseStatus};
pub use profile::{ProfilePoint, SketchPlane};
pub use project::{
    Group, Level, Project, ProjectField, ProjectInfo, can_join_geometry, join_priority, template_default,
};
pub use ramp::{RampCheck, RampIssue, RampLayout, RampRules, RampShape, ramp_flights};
pub use region::Region;
pub use schedule::{
    BendingOptions, ColumnAlign, ColumnFormat, ScheduleAppearance, ScheduleDef, ScheduleField, ScheduleFilter,
    SortLevel,
};
pub use section_props::SectionProperties;
pub use stair::{
    LandingBeams, LandingPosts, PostAt, StairBuild, StairCheck, StairConstruction, StairIssue, StairLayout,
    StairRailing, StairRules, StairShape, StairSide, Stringers, TreadRule, Turn, stair_steps,
};
pub use structural_material::StructuralMaterial;
pub use structural_profile::{ProfileDef, ProfileMaterial, ProfileShape, SectionOutline};
pub use sun::{SunDate, SunInterval, SunMode, SunPreset, SunSettings, SunSite, SunState};
pub use transform::PlanTransform;
pub use types::{
    Category, ColumnSection, ElementType, FoundationKind, Layer, LayerFunction, SectionShape, StripHaunch, TypeData,
    TypeInfo, WallFunction, stair_layout,
};
pub use units::{AreaUnit, LengthUnit, MeasurementSystem, NumberStyle, SlopeUnit, Units, VolumeUnit};
pub use view::{
    CameraState, DetailLevel, Direction, Looking, MAX_PAPER_MM, MAX_SECTION_SEGMENT, MIN_PAPER_MM, MepDiscipline,
    SheetSize, Underlay, UnderlayLevel, UnderlayPlan, View, ViewCategory, ViewKind, Viewport, VisualStyle,
};

pub use glam::{DVec2, DVec3};
