use crate::commands::*;
use crate::error::{EngineError, ErrorCode, Result};
use crate::session::Session;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// What a command does to the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    /// Reads the model; changes nothing.
    Query,
    /// Changes the model (one undo step).
    Edit,
    /// Works with files or replaces the document.
    File,
}

/// A command: typed parameters and what running it does. The type's doc
/// comment is its description; its fields' doc comments describe the
/// parameters in the generated schema.
pub trait Command: DeserializeOwned + JsonSchema {
    const NAME: &'static str;
    const KIND: CommandKind;
    /// Whether it may read a file named by one of its parameters (an import
    /// of an edit or query command); it then refuses the path when the run
    /// may not work with files ([`Session::require_files`]).
    const READS_FILES: bool = false;
    fn run(self, session: &mut Session) -> Result<Value>;
}

/// A command as listed by the catalog.
#[derive(Debug, Clone, Serialize)]
pub struct CommandInfo {
    pub name: String,
    pub kind: CommandKind,
    /// Area of the catalog (project, queries, elements…).
    pub group: String,
    pub description: String,
    /// JSON Schema of the parameters object: no references, valid under draft 7 and 2020-12.
    pub input_schema: Value,
    /// May read a file named by a parameter (besides `File` commands, which
    /// all work with files).
    pub reads_files: bool,
}

fn info<C: Command>(group: &'static str) -> CommandInfo {
    let generator = schemars::generate::SchemaSettings::draft07()
        .with(|s| {
            s.inline_subschemas = true;
            s.meta_schema = None;
        })
        .into_generator();
    let schema = generator.into_root_schema_for::<C>();
    let mut input_schema = serde_json::to_value(schema).unwrap_or_else(|_| serde_json::json!({"type": "object"}));
    portable_tuples(&mut input_schema, 0);
    let description = input_schema
        .get("description")
        .and_then(Value::as_str)
        .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    if let Some(o) = input_schema.as_object_mut() {
        o.remove("title");
        o.remove("description");
        o.entry("properties").or_insert_with(|| serde_json::json!({}));
    }
    CommandInfo {
        name: C::NAME.to_string(),
        kind: C::KIND,
        group: group.to_string(),
        description,
        input_schema,
        reads_files: C::READS_FILES,
    }
}

/// Rewrites draft 7 tuples (`items` as an array, which JSON Schema 2020-12 rejects, and with it MCP
/// clients and model APIs) as `prefixItems` for 2020-12 plus `items` holding any of the positions'
/// schemas for draft 7, which ignores `prefixItems`; the tuple's `minItems`/`maxItems` stay.
fn portable_tuples(v: &mut Value, depth: usize) {
    // Schemas of parameter structs are shallow; the bound only guards the recursion.
    if depth > 64 {
        return;
    }
    match v {
        Value::Object(o) => {
            if let Some(Value::Array(positions)) = o.get("items") {
                let positions = positions.clone();
                let mut distinct: Vec<Value> = Vec::new();
                for p in &positions {
                    if !distinct.contains(p) {
                        distinct.push(p.clone());
                    }
                }
                let items = match distinct.len() {
                    1 => distinct.remove(0),
                    _ => serde_json::json!({ "anyOf": distinct }),
                };
                o.insert("items".into(), items);
                o.insert("prefixItems".into(), Value::Array(positions));
                o.remove("additionalItems");
            }
            for child in o.values_mut() {
                portable_tuples(child, depth + 1);
            }
        }
        Value::Array(a) => {
            for child in a {
                portable_tuples(child, depth + 1);
            }
        }
        _ => {}
    }
}

fn run<C: Command>(session: &mut Session, args: Value) -> Result<Value> {
    let args = if args.is_null() { serde_json::json!({}) } else { args };
    let command: C = serde_json::from_value(args).map_err(|e| EngineError::invalid(format!("{}: {e}", C::NAME)))?;
    command.run(session)
}

macro_rules! registry {
    ($($group:literal => [$($c:ty),* $(,)?]),* $(,)?) => {
        /// Every command, in catalog order (grouped by area).
        pub fn catalog() -> Vec<CommandInfo> {
            vec![$($(info::<$c>($group)),*),*]
        }

        pub(crate) fn dispatch(session: &mut Session, name: &str, args: Value) -> Result<Value> {
            $($(
                if name == <$c as Command>::NAME {
                    return run::<$c>(session, args);
                }
            )*)*
            Err(EngineError::new(ErrorCode::UnknownCommand, format!("unknown command: {name}")))
        }

        /// The kind of a built-in command, without building the catalog.
        pub fn command_kind(name: &str) -> Option<CommandKind> {
            $($(
                if name == <$c as Command>::NAME {
                    return Some(<$c as Command>::KIND);
                }
            )*)*
            None
        }
    };
}

/// The catalog entry of one command.
pub fn describe(name: &str) -> Option<CommandInfo> {
    catalog().into_iter().find(|c| c.name == name)
}

registry![
    "Project and files" => [
        project::ProjectNew,
        project::ProjectOpen,
        project::ProjectSave,
        project::ProjectInfoGet,
        project::ProjectInfoSet,
        site::SiteGet,
        site::SiteSet,
        site::SiteContextImport,
        project::Undo,
        project::Redo,
        project::UnitsGet,
        project::UnitsSet,
        project::LanguageSet,
        io::IfcImport,
        io::SkpImport,
        io::SkpComponentLoad,
        io::ObjImport,
        io::GltfImport,
        io::IfcExport,
        io::SheetExport,
        io::ViewExport,
        io::ScheduleExportCsv,
        io::ComponentLoad,
        io::ComponentLibraryList,
        io::ComponentSave,
        cad::CadImportFile,
        cad::ImageImport,
        cad::CadLayers,
        cad::CadLayersSet,
        cad_explode::CadExplode,
    ],
    "Queries" => [
        query::CategoriesList,
        query::LevelsList,
        query::TypesList,
        query::TypeGet,
        query::ElementsList,
        query::ElementGet,
        query::GroupsList,
        query::Measure,
        query::ViewsList,
        query::ViewGet,
        query::MaterialsList,
        query::MaterialGet,
        query::ScheduleTable,
    ],
    "Creating elements" => [
        create::WallCreate,
        curtain::CurtainTypeCreate,
        curtain::CurtainGet,
        curtain::CurtainGridSet,
        curtain::CurtainPanelSet,
        create::DoorCreate,
        doors::DoorTypeCreate,
        create::WindowCreate,
        create::FloorCreate,
        create::FloorOpeningAdd,
        create::FloorOpeningRemove,
        create::CeilingCreate,
        create::RoofCreate,
        create::RoofEdgeSet,
        create::RoofOpeningRemove,
        create::RoomCreate,
        create::GridCreate,
        create::ColumnCreate,
        create::BeamCreate,
        structure::ColumnsAtGrids,
        structure::BeamsOnGrids,
        structure::BeamsBetweenColumns,
        grid_frame::GridFrameCreate,
        structure::SlabJoinSupports,
        structure::StructureNumber,
        framing::BraceCreate,
        framing::BracesBetweenColumns,
        framing::TrussCreate,
        bracing::BracingCreate,
        steel_connection::SteelConnectionMacros,
        steel_connection::SteelConnectionCreate,
        steel_connection::SteelConnectionsAuto,
        rebar::RebarCreate,
        rebar::RebarTypical,
        connection::ConnectionsCreate,
        analytical::AnalyticalModelGet,
        analytical::AnalyticalExport,
        foundation::FootingCreate,
        foundation::StripFootingCreate,
        foundation::FoundationSlabCreate,
        foundation::FootingsUnderColumns,
        foundation::StripFootingsUnderWalls,
        stair::StairCreate,
        stair::StairCheck,
        stair::StairRailingsAdd,
        stair::RailingHostSet,
        ramp::RampCreate,
        ramp::RampCheck,
        roofing::RoofCheck,
        mass::MassCreate,
        sketch::SketchGet,
        sketch::SketchSet,
        mass::MassFloorsSet,
        mass::MassFloors,
        mass::MassTypeCreate,
        sweep::SweepCreate,
        sweep::SweepTypeCreate,
        sweep::SweepProfileSet,
        shaft::ShaftCreate,
        shaft::ShaftTypeCreate,
        topography::TopographyCreate,
        topography::TopographyPointsSet,
        topography::TopographyTypeCreate,
        pad::PadCreate,
        pad::PadTypeCreate,
        mass::MassFaceMove,
        mass::MassWallsCreate,
        mass::MassSlabsCreate,
        mass::MassRoofCreate,
        ramp::RampTypeCreate,
        create::RailingCreate,
        create::ComponentPlace,
    ],
    "Building services" => [
        mep::PipeCreate,
        mep::ConduitCreate,
        cable_tray::CableTrayCreate,
        cable_tray::CableTrayTypeCreate,
        valve::ValvePlace,
        valve::ValveTypeCreate,
        devices::DeviceTypeCreate,
        lightning::LpsConductorCreate,
        lightning::LpsConductorTypeCreate,
        lightning::EarthRingCreate,
        lightning::AirTerminalPlace,
        lightning::EarthElectrodePlace,
        lightning::LpsCheck,
        cabling::RackPlace,
        cabling::DataOutletPlace,
        cabling::CablingList,
        mep::SystemsList,
        mep::SystemCreate,
        mep::SystemSet,
        mep::SystemDelete,
        mep::SystemAssign,
        mep::CircuitLoads,
        mep::MepFittings,
        mep::MepConnect,
        mep::MepSizing,
        mep::MepSizeApply,
    ],
    "Modifying elements" => [
        modify::ElementsDelete,
        modify::ElementsMove,
        modify::ElementsCopy,
        modify::ElementsRotate,
        modify::ElementsMirror,
        modify::ElementsPin,
        modify::ElementFlip,
        modify::ElementsTurn,
        modify::ElementsExport,
        modify::ElementsPaste,
        modify::WallJoin,
        modify::WallAttach,
        modify::GeometryJoin,
        modify::GeometryUnjoin,
        modify::GroupCreate,
        modify::GroupUngroup,
        modify::TrimCorner,
        modify::TrimExtend,
        modify::ElementSplit,
        modify::ElementOffset,
        modify::ElementAlign,
        modify::ElementMoveEnd,
        modify::GridsAlignEnds,
        edit::ElementSet,
        edit::ElementsSet,
        edit::ElementsSetType,
    ],
    "Dimensions and text" => [
        annotate::DimensionCreate,
        annotate::TagMove,
        annotate::DimensionChainCreate,
        annotate::DimensionOrdinateCreate,
        annotate::DimensionSetValue,
        annotate::TextCreate,
        detail::DetailLineCreate,
        detail_component::DetailComponentCreate,
        detail_component::DetailComponentPlace,
        browser::BrowserSchemesList,
        browser::BrowserSchemeCreate,
        browser::BrowserSchemeSet,
        browser::BrowserSchemeDelete,
        browser::BrowserSchemeUse,
        browser::BrowserTree,
        line_style::LineStylesList,
        line_style::LineStyleCreate,
        line_style::LineStyleSet,
        line_style::LineStyleDelete,
        region::FilledRegionCreate,
        region::MaskingRegionCreate,
        region::FilledRegionTypeCreate,
        level_mark::LevelMarkCreate,
        level_mark::LevelMarkTypeCreate,
        annotate::TextAddLeader,
        annotate::TextRemoveLeader,
    ],
    "Types, components, materials" => [
        types::TypeSet,
        types::TypeDuplicate,
        types::TypeDelete,
        types::ComponentCreate,
        component::ComponentGet,
        component::ComponentParamAdd,
        component::ComponentParamSet,
        component::ComponentParamDelete,
        component::ComponentRefPlaneAdd,
        component::ComponentRefPlaneSet,
        component::ComponentRefPlaneDelete,
        component::ComponentRefLineAdd,
        component::ComponentRefLineDelete,
        component::ComponentDimensionAdd,
        component::ComponentDimensionSet,
        component::ComponentDimensionDelete,
        component::ComponentEqualAdd,
        component::ComponentEqualDelete,
        component_piece::ComponentPieceAdd,
        component_piece::ComponentPieceSet,
        component_piece::ComponentPieceDelete,
        component_connector::ComponentConnectorAdd,
        component_connector::ComponentConnectorSet,
        component_connector::ComponentConnectorDelete,
        component_script::ComponentScriptSet,
        component_script::ComponentParamsGet,
        component_script::ComponentParamsSet,
        component_script::ComponentHotspotMove,
        materials::MaterialCreate,
        materials::MaterialDuplicate,
        materials::MaterialRename,
        materials::MaterialSet,
        materials::MaterialDelete,
        materials::MaterialTextureSet,
        materials::MaterialTextureClear,
        materials::MaterialLibraryImport,
        fill_patterns::FillPatternsList,
        fill_patterns::FillPatternGet,
        fill_patterns::FillPatternCreate,
        fill_patterns::FillPatternSet,
        fill_patterns::FillPatternDuplicate,
        fill_patterns::FillPatternDelete,
        fill_patterns::FillPatternImport,
        fill_patterns::FillPatternExport,
        profiles::ProfilesList,
        profiles::ProfileGet,
        profiles::ProfileCreate,
        profiles::ProfileSet,
        profiles::ProfileDuplicate,
        profiles::ProfileDelete,
        profiles::ProfileCatalog,
        profiles::ProfileImport,
        profiles::ProfileTypeCreate,
        structure_material::StructuralTypeFor,
        steel_building::SteelBuildingCreate,
        wall_framing::WallFramingCreate,
        paint::Paint,
        paint::PaintRemove,
        paint::PaintsList,
    ],
    "Levels and views" => [
        levels::LevelCreate,
        levels::LevelSet,
        levels::LevelDelete,
        views::ViewCreate,
        views::ViewSet,
        views::ViewDuplicate,
        views::ViewDelete,
        callout::CalloutCreate,
        callout::CalloutRemove,
        callout::CalloutsList,
        views::SheetPlaceView,
        views::SheetRemoveViewport,
        sheets::SheetSet,
        sheets::SheetFields,
        sheets::TitleBlockCreate,
        sheets::ComponentSheetPieceAdd,
        sheets::ComponentSheetPieceSet,
        sheets::ComponentSheetPieceDelete,
    ],
    "View graphics" => [
        graphics::ViewGraphicsGet,
        graphics::ViewGraphicsSet,
        graphics::ElementOverrideSet,
        graphics::ElementHideInView,
        graphics::FiltersList,
        graphics::FilterCreate,
        graphics::FilterSet,
        graphics::FilterDelete,
        graphics::FilterElements,
        graphics::ViewFilterAdd,
        graphics::ViewFilterSet,
        graphics::ViewFilterRemove,
        graphics::ViewFilterMove,
        graphics::TemplatesList,
        graphics::TemplateCreate,
        graphics::TemplateSet,
        graphics::TemplateDelete,
        graphics::ViewTemplateApply,
        graphics::ViewTemplateAssign,
        graphics::ViewTemporary,
        graphics::ViewTemporaryApply,
    ],
    "Phases" => [
        phases::PhasesList,
        phases::PhaseCreate,
        phases::PhaseSet,
        phases::PhaseDelete,
        phases::ElementsPhaseSet,
        phases::ElementsDemolish,
        phases::PhaseCreationSet,
        phases::PhaseFiltersList,
        phases::PhaseFilterCreate,
        phases::PhaseFilterSet,
        phases::PhaseFilterDelete,
        phases::PhaseGraphicsGet,
        phases::PhaseGraphicsSet,
    ],
    "Design options" => [
        design_options::OptionsList,
        design_options::OptionSetCreate,
        design_options::OptionSetRename,
        design_options::OptionSetDelete,
        design_options::OptionCreate,
        design_options::OptionSetProps,
        design_options::OptionDelete,
        design_options::OptionAcceptPrimary,
        design_options::ElementsOptionSet,
        design_options::ViewOptionsSet,
        design_options::OptionActiveSet,
    ],
    "Visual programming" => [
        graph::GraphRun,
        graph::GraphCheck,
        graph::GraphNodes,
        graph::GraphElements,
        graph::GraphRelease,
    ],
];
