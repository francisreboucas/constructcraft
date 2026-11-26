use crate::{ElementTypeId, Uid};
use serde::{Deserialize, Serialize};

/// BIM category of an element or element type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    Wall,
    Door,
    Window,
    Floor,
    Ceiling,
    Roof,
    Room,
    Grid,
    /// Levels as drawn in elevations and sections: a line across the view
    /// with a head (name and elevation) at its ends (see [`crate::level_head`]).
    Level,
    Dimension,
    Text,
    Column,
    Beam,
    Stair,
    Railing,
    Component,
    Foundation,
    Rebar,
    /// Plumbing pipes (water supply, drainage, vent, rainwater).
    Pipe,
    /// Electrical conduits.
    Conduit,
    Ramp,
    /// Imported DWG/DXF drawings (underlays).
    CadImport,
    /// Conceptual masses (massing studies).
    Mass,
    /// Profiles swept along paths (baseboards, moldings, fascias, gutters).
    Sweep,
    /// Shaft openings through floors, ceilings and roofs.
    Shaft,
    /// Markers of interior elevations in plans.
    InteriorMarker,
    /// Parametric steel buildings: hosts of portal frames, purlins and girts.
    SteelBuilding,
    /// Parametric trusses: hosts of chords, diagonals and posts.
    Truss,
    /// Parametric bracings: hosts of the braces of a bay of a frame.
    Bracing,
    /// Light steel and wood framing of walls: hosts of studs, tracks, headers and sills.
    WallFraming,
    /// Terrain surfaces.
    Topography,
    /// Building pads graded into terrain.
    Pad,
    /// Title blocks: sheet layouts (components drawn on sheets).
    TitleBlock,
    /// Detail lines: 2D lines of one view.
    DetailLine,
    /// Filled regions: 2D areas of one view filled with a pattern.
    FilledRegion,
    /// Masking regions: 2D areas of one view hiding the model under them.
    MaskingRegion,
    /// Level marks: the elevation of a point in a plan, elevation or section.
    LevelMark,
    /// Bar tags: the bars of a reinforcement set written in a view.
    BarTag,
    /// Piles (deep foundations), free or under pile caps.
    Pile,
    /// Detail components: reusable 2D components placed in one view.
    DetailComponent,
    /// Cable trays: open runs carrying electrical circuits.
    CableTray,
    /// Valves on pipes.
    Valve,
    /// Conductors of lightning protection systems.
    LightningConductor,
    /// Steel and timber connections: plates and bolts at member ends.
    Connection,
    /// Niches: recesses in one face of a wall, hosted like doors and windows.
    Niche,
    /// Tag types: what plans write by doors, windows, rooms, columns, beams
    /// and footings (see [`crate::tag`]); their tags are drawn with their element.
    Tag,
    /// View mark types: how plans mark sections and elevations (see
    /// [`crate::view_mark`]).
    ViewMark,
}

impl Category {
    pub const ALL: [Category; 44] = [
        Category::Wall,
        Category::Door,
        Category::Window,
        Category::Niche,
        Category::Floor,
        Category::Ceiling,
        Category::Roof,
        Category::Column,
        Category::Beam,
        Category::Foundation,
        Category::Pile,
        Category::Rebar,
        Category::Connection,
        Category::Pipe,
        Category::Conduit,
        Category::CableTray,
        Category::Valve,
        Category::LightningConductor,
        Category::Stair,
        Category::Ramp,
        Category::Railing,
        Category::Sweep,
        Category::Component,
        Category::Room,
        Category::Grid,
        Category::Level,
        Category::Dimension,
        Category::Text,
        Category::DetailLine,
        Category::FilledRegion,
        Category::MaskingRegion,
        Category::DetailComponent,
        Category::LevelMark,
        Category::BarTag,
        Category::CadImport,
        Category::Mass,
        Category::Shaft,
        Category::InteriorMarker,
        Category::SteelBuilding,
        Category::Truss,
        Category::Bracing,
        Category::WallFraming,
        Category::Topography,
        Category::Pad,
    ];

    /// Every category by name: [`Category::ALL`] (the ones views draw and
    /// filter), title blocks, whose types sheets use, tags, whose types plans
    /// tag elements with, and view marks, whose types plans mark sections
    /// and elevations with.
    pub const NAMED: [Category; Category::ALL.len() + 3] = {
        let mut out = [Category::TitleBlock; Category::ALL.len() + 3];
        let mut i = 0;
        while i < Category::ALL.len() {
            out[i] = Category::ALL[i];
            i += 1;
        }
        out[Category::ALL.len() + 1] = Category::Tag;
        out[Category::ALL.len() + 2] = Category::ViewMark;
        out
    };

    /// Categories with physical elements (shown in legends and 3D).
    pub const MODEL: [Category; 24] = [
        Category::Wall,
        Category::Door,
        Category::Window,
        Category::Niche,
        Category::Floor,
        Category::Ceiling,
        Category::Roof,
        Category::Column,
        Category::Beam,
        Category::Foundation,
        Category::Pile,
        Category::Rebar,
        Category::Connection,
        Category::Pipe,
        Category::Conduit,
        Category::CableTray,
        Category::Valve,
        Category::LightningConductor,
        Category::Stair,
        Category::Ramp,
        Category::Railing,
        Category::Sweep,
        Category::Component,
        Category::Topography,
    ];

    /// Categories that can be listed in a schedule.
    pub const SCHEDULABLE: [Category; 27] = [
        Category::Wall,
        Category::Door,
        Category::Window,
        Category::Niche,
        Category::Floor,
        Category::Ceiling,
        Category::Roof,
        Category::Column,
        Category::Beam,
        Category::Foundation,
        Category::Pile,
        Category::Rebar,
        Category::Connection,
        Category::Pipe,
        Category::Conduit,
        Category::CableTray,
        Category::Valve,
        Category::LightningConductor,
        Category::Stair,
        Category::Ramp,
        Category::Railing,
        Category::Sweep,
        Category::Component,
        Category::Room,
        Category::Mass,
        Category::Topography,
        Category::Pad,
    ];

    pub fn is_model(self) -> bool {
        Category::MODEL.contains(&self)
    }

    /// Translation key for the category name.
    pub fn key(self) -> &'static str {
        match self {
            Category::Wall => "category-wall",
            Category::Door => "category-door",
            Category::Window => "category-window",
            Category::Niche => "category-niche",
            Category::Floor => "category-floor",
            Category::Ceiling => "category-ceiling",
            Category::Roof => "category-roof",
            Category::Room => "category-room",
            Category::Grid => "category-grid",
            Category::Level => "category-level",
            Category::Dimension => "category-dimension",
            Category::Text => "category-text",
            Category::LevelMark => "category-level-mark",
            Category::BarTag => "category-bar-tag",
            Category::Pile => "category-pile",
            Category::TitleBlock => "category-title-block",
            Category::Tag => "category-tag",
            Category::ViewMark => "category-view-mark",
            Category::Column => "category-column",
            Category::Beam => "category-beam",
            Category::Stair => "category-stair",
            Category::Ramp => "category-ramp",
            Category::Railing => "category-railing",
            Category::Sweep => "category-sweep",
            Category::Shaft => "category-shaft",
            Category::InteriorMarker => "category-interior-marker",
            Category::SteelBuilding => "category-steel-building",
            Category::Truss => "category-truss",
            Category::Bracing => "category-bracing",
            Category::Connection => "category-connection",
            Category::WallFraming => "category-wall-framing",
            Category::Topography => "category-topography",
            Category::Pad => "category-pad",
            Category::Component => "category-component",
            Category::Foundation => "category-foundation",
            Category::Rebar => "category-rebar",
            Category::Pipe => "category-pipe",
            Category::Conduit => "category-conduit",
            Category::CableTray => "category-cable-tray",
            Category::Valve => "category-valve",
            Category::LightningConductor => "category-lightning-conductor",
            Category::CadImport => "category-cad-import",
            Category::Mass => "category-mass",
            Category::DetailLine => "category-detail-line",
            Category::FilledRegion => "category-filled-region",
            Category::MaskingRegion => "category-masking-region",
            Category::DetailComponent => "category-detail-component",
        }
    }

    /// Whether the category belongs to the building services (MEP).
    pub fn is_mep(self) -> bool {
        matches!(
            self,
            Category::Pipe | Category::Conduit | Category::CableTray | Category::Valve | Category::LightningConductor
        )
    }
}

/// Role of a layer in a compound type. The core (structure layers) is the
/// reference for core-based wall location lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LayerFunction {
    #[default]
    Structure,
    Substrate,
    Insulation,
    Finish,
    Membrane,
}

impl LayerFunction {
    pub const ALL: [LayerFunction; 5] = [
        LayerFunction::Structure,
        LayerFunction::Substrate,
        LayerFunction::Insulation,
        LayerFunction::Finish,
        LayerFunction::Membrane,
    ];

    /// Translation key of the function name.
    pub fn key(self) -> &'static str {
        match self {
            LayerFunction::Structure => "layer-structure",
            LayerFunction::Substrate => "layer-substrate",
            LayerFunction::Insulation => "layer-insulation",
            LayerFunction::Finish => "layer-finish",
            LayerFunction::Membrane => "layer-membrane",
        }
    }
}

/// One material layer of a compound wall/floor/roof type, listed exterior first.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Layer {
    pub material: String,
    /// Thickness in meters.
    pub thickness: f64,
    #[serde(default)]
    pub function: LayerFunction,
    /// Walls only: where the layer starts, meters above (negative: below)
    /// the wall's bottom at each point along it (see [`crate::layer_extent`]).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub base_offset: f64,
    /// Walls only: the layer's height from its own bottom, never past the
    /// wall's top (`None`: up to the wall's top).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// Purpose of a wall type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WallFunction {
    #[default]
    Interior,
    Exterior,
    Foundation,
    Retaining,
}

impl WallFunction {
    pub const ALL: [WallFunction; 4] =
        [WallFunction::Interior, WallFunction::Exterior, WallFunction::Foundation, WallFunction::Retaining];

    pub fn key(self) -> &'static str {
        match self {
            WallFunction::Interior => "wall-fn-interior",
            WallFunction::Exterior => "wall-fn-exterior",
            WallFunction::Foundation => "wall-fn-foundation",
            WallFunction::Retaining => "wall-fn-retaining",
        }
    }

    /// Whether walls of this function face the outside of the building.
    pub fn is_external(self) -> bool {
        !matches!(self, WallFunction::Interior)
    }
}

/// Descriptive data shared by all instances of a type.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TypeInfo {
    /// Short code shown in tags and schedules.
    pub type_mark: String,
    pub description: String,
    pub manufacturer: String,
    pub model: String,
    pub url: String,
    pub keynote: String,
    /// Classification or assembly code (e.g. a national construction classification).
    pub classification: String,
    pub fire_rating: String,
    pub comments: String,
    /// Unit cost in the project currency.
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// A project holds few types, so the size of the component variant does not
// matter; boxing its definition would only add indirection.
#[allow(clippy::large_enum_variant)]
pub enum TypeData {
    Wall {
        layers: Vec<Layer>,
        #[serde(default)]
        function: WallFunction,
        /// A curtain wall type: a grid of panels and mullions instead of layers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        curtain: Option<crate::curtain::CurtainSystem>,
        /// Walls made of this type are structural unless told otherwise.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        structural: bool,
    },
    Door {
        width: f64,
        height: f64,
        material: String,
    },
    Window {
        width: f64,
        height: f64,
        default_sill: f64,
        material: String,
    },
    Floor {
        layers: Vec<Layer>,
    },
    Ceiling {
        thickness: f64,
        material: String,
    },
    Roof {
        layers: Vec<Layer>,
        /// The tiles or sheets it is covered with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        covering: Option<crate::roofing::TileKind>,
        /// Purlins, rafters and battens under its sloped faces.
        #[serde(default)]
        framing: crate::roofing::RoofFraming,
    },
    /// Rooms have no type parameters yet; the type groups them in schedules.
    Room {},
    /// Grid line with a bubble of `bubble` paper millimeters, drawn as `style` says.
    Grid {
        bubble: f64,
        #[serde(default)]
        style: crate::grid::GridStyle,
    },
    /// Dimensions with text of `text_size` paper millimeters, drawn as `style` says.
    Dimension {
        text_size: f64,
        #[serde(default)]
        style: crate::dim_style::DimensionStyle,
    },
    /// Text notes of `size` paper millimeters; new leaders get `symbol`.
    Text {
        size: f64,
        #[serde(default)]
        symbol: crate::LeaderSymbol,
    },
    /// Level marks with text of `text_size` paper millimeters, writing
    /// `datum` plus each level, followed by `finished_suffix` (the finished
    /// level) or `raw_suffix` (the raw, structural one).
    /// Bar tags with text of `text_size` paper millimeters.
    BarTag {
        text_size: f64,
    },
    /// Tags of `target` elements in plans, drawn as `style` says.
    Tag {
        target: crate::tag::TagTarget,
        #[serde(default)]
        style: crate::tag::TagStyle,
    },
    /// Marks of sections or elevations in plans, drawn as `style` says.
    ViewMark {
        kind: crate::view_mark::MarkKind,
        #[serde(default)]
        style: crate::view_mark::MarkStyle,
    },
    LevelMark {
        text_size: f64,
        #[serde(default)]
        datum: f64,
        #[serde(default)]
        finished_suffix: String,
        #[serde(default)]
        raw_suffix: String,
    },
    Column {
        section: ColumnSection,
        material: String,
        /// The structural profile of the section; when set, `section` is its
        /// bounding box, kept in step.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<crate::ProfileId>,
        /// Turn of the profile in its section plane, degrees counterclockwise.
        #[serde(default, skip_serializing_if = "is_zero")]
        profile_rotation_deg: f64,
        /// The profile mirrored across its vertical axis (before turning).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        profile_mirror: bool,
    },
    /// Beam section `width` x `height`, solid or a profile within that box.
    Beam {
        width: f64,
        height: f64,
        material: String,
        #[serde(default)]
        shape: SectionShape,
        /// The structural profile of the section; when set, `width`, `height`
        /// and `shape` are its bounding box, kept in step.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<crate::ProfileId>,
        /// Turn of the profile in its section plane, degrees counterclockwise.
        #[serde(default, skip_serializing_if = "is_zero")]
        profile_rotation_deg: f64,
        /// The profile mirrored across its vertical axis (before turning).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        profile_mirror: bool,
    },
    /// Reinforcing bars of `diameter`.
    Rebar {
        diameter: f64,
        material: String,
        /// Steel grade (e.g. "CA-50"); empty for none. Bars of different
        /// grades never share a mark, and bending schedules total by it.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        grade: String,
        /// Mass per meter, kg/m, when it is not that of a round bar of
        /// `diameter` in steel of 7850 kg/m³ (ribbed bars are listed by
        /// their nominal mass).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mass_per_meter: Option<f64>,
    },
    /// Pipes of outside `diameter`. A type of a pipe series takes its
    /// diameter from the series row `size` (kept in step by
    /// `EditCtx::finish`); a free type may give its inside diameter.
    Pipe {
        diameter: f64,
        material: String,
        /// The pipe series the type is a size of.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        series: Option<crate::PipeSeriesId>,
        /// The series row's size label.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        size: String,
        /// Inside diameter of a free type, m.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inside: Option<f64>,
    },
    /// Conduits of outside `diameter`.
    Conduit {
        diameter: f64,
        material: String,
    },
    /// Cable trays of outside `width` x `height`, built as `style` says.
    CableTray {
        width: f64,
        height: f64,
        #[serde(default)]
        style: crate::TrayStyle,
        material: String,
    },
    /// Valves of `kind` (their diameter is their pipe's).
    Valve {
        #[serde(default)]
        kind: crate::ValveKind,
        material: String,
    },
    /// Lightning protection conductors: round conductors of `diameter`.
    LightningConductor {
        diameter: f64,
        material: String,
    },
    /// Foundation: a pad `width` x `length`, a strip `width` wide or a slab,
    /// each `thickness` deep. A strip with a `haunch` is an inverted T: a
    /// footing slab `thickness` deep at the faces of a beam on top of it.
    Foundation {
        kind: FoundationKind,
        width: f64,
        length: f64,
        thickness: f64,
        material: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        haunch: Option<StripHaunch>,
        /// Pile caps only: the piles and their layout (the type's `width`
        /// and `length` are kept as its outline's size, see [`crate::pile`]).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pile_cap: Option<crate::pile::PileCap>,
    },
    /// Piles of `section` made as `kind` says, in `material`.
    Pile {
        section: crate::pile::PileSection,
        #[serde(default)]
        kind: crate::pile::PileKind,
        material: String,
    },
    /// Stair: risers at most `max_riser` high, treads `tread` deep (or from
    /// Blondel's formula, see `rules`), built as `build` says.
    Stair {
        width: f64,
        max_riser: f64,
        tread: f64,
        material: String,
        #[serde(default)]
        build: crate::StairBuild,
        #[serde(default)]
        rules: crate::StairRules,
    },
    /// Ramp: a slab `thickness` thick, `width` wide, within `rules`.
    Ramp {
        width: f64,
        thickness: f64,
        material: String,
        #[serde(default)]
        rules: crate::RampRules,
    },
    /// Railing with a top rail at `height` and posts every `post_spacing`.
    Railing {
        height: f64,
        post_spacing: f64,
        material: String,
    },
    /// Parametric component: `component` names the definition shared by its
    /// types; the parameter values in `def` are this type's.
    Component {
        component: String,
        def: crate::component::ComponentDef,
    },
    /// Imported CAD drawings have no type parameters; the type names them.
    CadImport {},
    /// Detail lines have no type parameters (each line has its style).
    DetailLine {},
    /// Filled regions: how they are filled (see [`crate::region`]).
    FilledRegion {
        #[serde(default)]
        fill: crate::PatternFill,
    },
    /// Masking regions have no type parameters (they are opaque white).
    MaskingRegion {},
    /// Conceptual masses, shown in `material`.
    Mass {
        material: String,
    },
    /// Swept profiles: the closed `profile` (x across, to the left of the
    /// path; y up), shown in `material`.
    Sweep {
        profile: Vec<glam::DVec2>,
        material: String,
        /// The curves the profile was drawn with (arcs kept as arcs), dropped
        /// when the profile is changed without them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sketch: Option<crate::sketch::Sketch>,
    },
    /// Shaft openings (no properties of their own).
    Shaft {},
    /// Interior elevation markers (no properties of their own).
    InteriorMarker {},
    /// Steel buildings (no properties of their own; types name them).
    SteelBuilding {},
    /// Trusses (no properties of their own; types name them).
    Truss {},
    /// Bracings (no properties of their own; types name them).
    Bracing {},
    /// Connections (no properties of their own; types name them).
    Connection {},
    /// Wall framings (no properties of their own; types name them).
    WallFraming {},
    /// Terrain surfaces: the earth `material` they are cut in, and the
    /// spacing of their contour lines, every `major_every`-th one a major.
    Topography {
        material: String,
        #[serde(default = "crate::types::default_contour_interval")]
        contour_interval: f64,
        #[serde(default = "crate::types::default_major_every")]
        major_every: u32,
    },
    /// Building pads: the horizontal run per unit of rise of their cut and
    /// fill slopes.
    Pad {
        #[serde(default = "crate::types::default_cut_slope")]
        cut_slope: f64,
        #[serde(default = "crate::types::default_fill_slope")]
        fill_slope: f64,
    },
}

pub(crate) fn default_cut_slope() -> f64 {
    crate::pad::DEFAULT_CUT_SLOPE
}

pub(crate) fn default_fill_slope() -> f64 {
    crate::pad::DEFAULT_FILL_SLOPE
}

pub(crate) fn default_contour_interval() -> f64 {
    crate::topography::DEFAULT_CONTOUR_INTERVAL
}

pub(crate) fn default_major_every() -> u32 {
    crate::topography::DEFAULT_MAJOR_EVERY
}

impl TypeData {
    /// A type of reinforcing bars of `diameter` in `material`, with no grade
    /// and the mass per meter of a round bar.
    pub fn rebar(diameter: f64, material: String) -> TypeData {
        TypeData::Rebar { diameter, material, grade: String::new(), mass_per_meter: None }
    }

    /// A type of reinforcing bars of `diameter` in `material` and steel
    /// `grade` (`CA-50`), with the mass per meter of a round bar.
    pub fn graded_rebar(diameter: f64, material: String, grade: &str) -> TypeData {
        TypeData::Rebar { diameter, material, grade: grade.to_string(), mass_per_meter: None }
    }

    /// A free pipe type (no series) of outside `diameter` in `material`.
    pub fn pipe(diameter: f64, material: String) -> TypeData {
        TypeData::Pipe { diameter, material, series: None, size: String::new(), inside: None }
    }

    /// Material names referenced by this type.
    pub fn materials(&self) -> Vec<&str> {
        match self {
            TypeData::Wall { curtain: Some(c), .. } => c.materials(),
            TypeData::Wall { layers, .. } | TypeData::Floor { layers } => {
                layers.iter().map(|l| l.material.as_str()).collect()
            }
            TypeData::Roof { layers, framing, .. } => {
                let framed = (!framing.material.is_empty()).then_some(framing.material.as_str());
                layers.iter().map(|l| l.material.as_str()).chain(framed).collect()
            }
            TypeData::Door { material, .. }
            | TypeData::Window { material, .. }
            | TypeData::Ceiling { material, .. }
            | TypeData::Column { material, .. }
            | TypeData::Beam { material, .. }
            | TypeData::Foundation { material, .. }
            | TypeData::Rebar { material, .. }
            | TypeData::Pile { material, .. }
            | TypeData::Pipe { material, .. }
            | TypeData::Conduit { material, .. }
            | TypeData::CableTray { material, .. }
            | TypeData::Valve { material, .. }
            | TypeData::LightningConductor { material, .. }
            | TypeData::Stair { material, .. }
            | TypeData::Ramp { material, .. }
            | TypeData::Railing { material, .. }
            | TypeData::Mass { material }
            | TypeData::Topography { material, .. }
            | TypeData::Sweep { material, .. } => vec![material.as_str()],
            TypeData::Component { def, .. } => def
                .items
                .iter()
                .map(|i| i.material.as_str())
                .chain(
                    def.params
                        .iter()
                        .filter(|p| p.kind == crate::ParamKind::Material && !p.text.is_empty())
                        .map(|p| p.text.as_str()),
                )
                .collect(),
            TypeData::Room {}
            | TypeData::Grid { .. }
            | TypeData::Dimension { .. }
            | TypeData::Text { .. }
            | TypeData::LevelMark { .. }
            | TypeData::BarTag { .. }
            | TypeData::Tag { .. }
            | TypeData::ViewMark { .. }
            | TypeData::CadImport {}
            | TypeData::DetailLine {}
            | TypeData::FilledRegion { .. }
            | TypeData::MaskingRegion {}
            | TypeData::Shaft {}
            | TypeData::InteriorMarker {}
            | TypeData::SteelBuilding {}
            | TypeData::Truss {}
            | TypeData::Bracing {}
            | TypeData::Connection {}
            | TypeData::WallFraming {}
            | TypeData::Pad { .. } => Vec::new(),
        }
    }

    /// Material shown where the type is cut as a whole (sections): the first
    /// structure layer of layered types (else the thickest), or the main material.
    pub fn cut_material(&self) -> &str {
        match self {
            TypeData::Wall { curtain: Some(c), .. } => c.main_material(),
            TypeData::Wall { layers, .. } | TypeData::Floor { layers } | TypeData::Roof { layers, .. } => layers
                .iter()
                .find(|l| l.function == LayerFunction::Structure)
                .or_else(|| layers.iter().max_by(|a, b| a.thickness.total_cmp(&b.thickness)))
                .map_or("", |l| l.material.as_str()),
            other => other.materials().first().copied().unwrap_or(""),
        }
    }

    pub fn uses_material(&self, name: &str) -> bool {
        self.materials().contains(&name)
    }

    /// Point references to material `old` at `new`.
    pub fn rename_material(&mut self, old: &str, new: &str) {
        let fix = |m: &mut String| {
            if m == old {
                *m = new.to_string();
            }
        };
        match self {
            TypeData::Wall { layers, curtain, .. } => {
                layers.iter_mut().for_each(|l| fix(&mut l.material));
                if let Some(c) = curtain {
                    c.rename_material(old, new);
                }
            }
            TypeData::Floor { layers } => layers.iter_mut().for_each(|l| fix(&mut l.material)),
            TypeData::Roof { layers, framing, .. } => {
                layers.iter_mut().for_each(|l| fix(&mut l.material));
                fix(&mut framing.material);
            }
            TypeData::Door { material, .. }
            | TypeData::Window { material, .. }
            | TypeData::Ceiling { material, .. }
            | TypeData::Column { material, .. }
            | TypeData::Beam { material, .. }
            | TypeData::Foundation { material, .. }
            | TypeData::Rebar { material, .. }
            | TypeData::Pile { material, .. }
            | TypeData::Pipe { material, .. }
            | TypeData::Conduit { material, .. }
            | TypeData::CableTray { material, .. }
            | TypeData::Valve { material, .. }
            | TypeData::LightningConductor { material, .. }
            | TypeData::Stair { material, .. }
            | TypeData::Ramp { material, .. }
            | TypeData::Railing { material, .. }
            | TypeData::Mass { material }
            | TypeData::Topography { material, .. }
            | TypeData::Sweep { material, .. } => fix(material),
            TypeData::Component { def, .. } => {
                def.items.iter_mut().for_each(|i| fix(&mut i.material));
                def.params.iter_mut().filter(|p| p.kind == crate::ParamKind::Material).for_each(|p| fix(&mut p.text));
            }
            TypeData::Room {}
            | TypeData::Grid { .. }
            | TypeData::Dimension { .. }
            | TypeData::Text { .. }
            | TypeData::LevelMark { .. }
            | TypeData::BarTag { .. }
            | TypeData::Tag { .. }
            | TypeData::ViewMark { .. }
            | TypeData::CadImport {}
            | TypeData::DetailLine {}
            | TypeData::FilledRegion { .. }
            | TypeData::MaskingRegion {}
            | TypeData::Shaft {}
            | TypeData::InteriorMarker {}
            | TypeData::SteelBuilding {}
            | TypeData::Truss {}
            | TypeData::Bracing {}
            | TypeData::Connection {}
            | TypeData::WallFraming {}
            | TypeData::Pad { .. } => {}
        }
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Cross-section of a column type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColumnSection {
    Rectangular {
        width: f64,
        depth: f64,
    },
    Round {
        diameter: f64,
    },
    /// A profile (I, channel, angle, tube) within a `width` x `depth` box.
    Profile {
        width: f64,
        depth: f64,
        shape: SectionShape,
    },
    /// A circular hollow section.
    Pipe {
        diameter: f64,
        wall: f64,
    },
}

impl ColumnSection {
    /// The bounding box, `(width, depth)`.
    pub fn size(&self) -> (f64, f64) {
        match *self {
            ColumnSection::Rectangular { width, depth } | ColumnSection::Profile { width, depth, .. } => (width, depth),
            ColumnSection::Round { diameter } | ColumnSection::Pipe { diameter, .. } => (diameter, diameter),
        }
    }
}

/// What fills a section's `width` x `depth` box. Profiles are laid out as in
/// steel tables: the web of an I or channel runs along the depth, the legs of
/// an angle lie on the left and bottom sides of the box, and the flanges of a
/// channel point to the right.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum SectionShape {
    #[default]
    Solid,
    /// I or H: two flanges `flange` thick joined by a web `web` thick.
    I { web: f64, flange: f64 },
    /// Channel (U): a web on the left and two flanges.
    Channel { web: f64, flange: f64 },
    /// Angle (L): two legs `thickness` thick.
    Angle { thickness: f64 },
    /// Rectangular hollow section with walls `wall` thick.
    Tube { wall: f64 },
}

impl SectionShape {
    pub fn key(&self) -> &'static str {
        match self {
            SectionShape::Solid => "section-solid",
            SectionShape::I { .. } => "section-i",
            SectionShape::Channel { .. } => "section-channel",
            SectionShape::Angle { .. } => "section-angle",
            SectionShape::Tube { .. } => "section-tube",
        }
    }

    /// The same shape with each thickness clamped to fit a `width` x `depth` box.
    pub fn fitted(self, width: f64, depth: f64) -> Self {
        let fit = |t: f64, max: f64| if t.is_finite() { t.clamp(max.min(0.001), max) } else { max };
        match self {
            SectionShape::Solid => self,
            SectionShape::I { web, flange } => {
                SectionShape::I { web: fit(web, width), flange: fit(flange, depth * 0.5) }
            }
            SectionShape::Channel { web, flange } => {
                SectionShape::Channel { web: fit(web, width), flange: fit(flange, depth * 0.5) }
            }
            SectionShape::Angle { thickness } => SectionShape::Angle { thickness: fit(thickness, width.min(depth)) },
            SectionShape::Tube { wall } => SectionShape::Tube { wall: fit(wall, width.min(depth) * 0.5) },
        }
    }
}

/// What a foundation type is: a pad under a column, a strip under a wall, a
/// slab, or a cap on piles (see [`crate::pile`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FoundationKind {
    #[default]
    Pad,
    Strip,
    Slab,
    PileCap,
}

/// The section of a strip footing with a beam on top (an inverted T): the
/// footing slab is the type's `thickness` deep at the faces of the beam and
/// `edge_thickness` at its edges, its top sloping between them (a haunch;
/// flat when the two are equal), and the beam `beam_width` wide rises
/// `beam_height` above the slab (0: a sloped footing with a flat middle).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StripHaunch {
    pub edge_thickness: f64,
    pub beam_width: f64,
    pub beam_height: f64,
}

impl StripHaunch {
    /// Highest beam, meters.
    pub const MAX_HEIGHT: f64 = 10.0;

    /// Whether the values make a section in a strip `width` wide and
    /// `thickness` deep at the beam: an edge no thicker than that, a beam
    /// narrower than the strip.
    pub fn fits(&self, width: f64, thickness: f64) -> bool {
        let positive = |v: f64| v.is_finite() && v > 1e-4;
        positive(self.edge_thickness)
            && self.edge_thickness <= thickness + 1e-9
            && positive(self.beam_width)
            && self.beam_width < width - 1e-4
            && self.beam_height.is_finite()
            && (0.0..=Self::MAX_HEIGHT).contains(&self.beam_height)
    }

    /// The section of a strip `width` wide and `thickness` deep at the beam:
    /// `x` across (centered), `y` up from the bottom, counter-clockwise.
    pub fn outline(&self, width: f64, thickness: f64) -> Vec<glam::DVec2> {
        let (w, b) = (width / 2.0, self.beam_width / 2.0);
        let (edge, top) = (self.edge_thickness, thickness + self.beam_height);
        let mut pts = vec![
            glam::DVec2::new(-w, 0.0),
            glam::DVec2::new(w, 0.0),
            glam::DVec2::new(w, edge),
            glam::DVec2::new(b, thickness),
        ];
        if self.beam_height > 0.0 {
            pts.extend([glam::DVec2::new(b, top), glam::DVec2::new(-b, top)]);
        }
        pts.extend([glam::DVec2::new(-b, thickness), glam::DVec2::new(-w, edge)]);
        pts
    }

    /// Area of that section.
    pub fn area(&self, width: f64, thickness: f64) -> f64 {
        let slope = (width - self.beam_width) * (self.edge_thickness + thickness) / 2.0;
        slope + self.beam_width * (thickness + self.beam_height)
    }
}

impl TypeData {
    /// Depth of a foundation from its top to its bottom: its thickness,
    /// plus the beam on top of a haunched strip. 0 for other types.
    pub fn foundation_depth(&self) -> f64 {
        match self {
            TypeData::Foundation { thickness, haunch, kind, .. } => {
                thickness + haunch.filter(|_| *kind == FoundationKind::Strip).map_or(0.0, |h| h.beam_height)
            }
            _ => 0.0,
        }
    }

    /// The haunch of a strip footing type (`None` for a plain strip and other types).
    pub fn strip_haunch(&self) -> Option<StripHaunch> {
        match self {
            TypeData::Foundation { kind: FoundationKind::Strip, width, thickness, haunch, .. } => {
                haunch.filter(|h| h.fits(*width, *thickness))
            }
            _ => None,
        }
    }
}

impl FoundationKind {
    pub const ALL: [FoundationKind; 4] =
        [FoundationKind::Pad, FoundationKind::Strip, FoundationKind::Slab, FoundationKind::PileCap];

    pub fn key(self) -> &'static str {
        match self {
            FoundationKind::Pad => "foundation-pad",
            FoundationKind::Strip => "foundation-strip",
            FoundationKind::Slab => "foundation-slab",
            FoundationKind::PileCap => "foundation-pile-cap",
        }
    }
}

/// An element type: the shared parameters of a group of instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementType {
    pub id: ElementTypeId,
    pub name: String,
    pub data: TypeData,
    #[serde(default)]
    pub info: TypeInfo,
    #[serde(default)]
    pub uid: Uid,
}

impl ElementType {
    pub fn category(&self) -> Category {
        match &self.data {
            TypeData::Wall { .. } => Category::Wall,
            TypeData::Door { .. } => Category::Door,
            TypeData::Window { .. } => Category::Window,
            TypeData::Floor { .. } => Category::Floor,
            TypeData::Ceiling { .. } => Category::Ceiling,
            TypeData::Roof { .. } => Category::Roof,
            TypeData::Room {} => Category::Room,
            TypeData::Grid { .. } => Category::Grid,
            TypeData::Dimension { .. } => Category::Dimension,
            TypeData::Text { .. } => Category::Text,
            TypeData::LevelMark { .. } => Category::LevelMark,
            TypeData::BarTag { .. } => Category::BarTag,
            TypeData::Tag { .. } => Category::Tag,
            TypeData::ViewMark { .. } => Category::ViewMark,
            TypeData::Pile { .. } => Category::Pile,
            TypeData::Column { .. } => Category::Column,
            TypeData::Beam { .. } => Category::Beam,
            TypeData::Foundation { .. } => Category::Foundation,
            TypeData::Rebar { .. } => Category::Rebar,
            TypeData::Pipe { .. } => Category::Pipe,
            TypeData::Conduit { .. } => Category::Conduit,
            TypeData::CableTray { .. } => Category::CableTray,
            TypeData::Valve { .. } => Category::Valve,
            TypeData::LightningConductor { .. } => Category::LightningConductor,
            TypeData::Stair { .. } => Category::Stair,
            TypeData::Ramp { .. } => Category::Ramp,
            TypeData::Railing { .. } => Category::Railing,
            TypeData::Component { def, .. } => def.category(),
            TypeData::CadImport {} => Category::CadImport,
            TypeData::DetailLine {} => Category::DetailLine,
            TypeData::FilledRegion { .. } => Category::FilledRegion,
            TypeData::MaskingRegion {} => Category::MaskingRegion,
            TypeData::Mass { .. } => Category::Mass,
            TypeData::Sweep { .. } => Category::Sweep,
            TypeData::Shaft {} => Category::Shaft,
            TypeData::InteriorMarker {} => Category::InteriorMarker,
            TypeData::SteelBuilding {} => Category::SteelBuilding,
            TypeData::Truss {} => Category::Truss,
            TypeData::Bracing {} => Category::Bracing,
            TypeData::Connection {} => Category::Connection,
            TypeData::WallFraming {} => Category::WallFraming,
            TypeData::Topography { .. } => Category::Topography,
            TypeData::Pad { .. } => Category::Pad,
        }
    }

    /// The layers of a layered type (none for curtain walls).
    pub fn layers(&self) -> &[Layer] {
        match &self.data {
            TypeData::Wall { curtain: Some(_), .. } => &[],
            TypeData::Wall { layers, .. } | TypeData::Floor { layers } | TypeData::Roof { layers, .. } => layers,
            _ => &[],
        }
    }

    /// Total thickness for layered types, 0 for openings.
    pub fn thickness(&self) -> f64 {
        match &self.data {
            TypeData::Ceiling { thickness, .. } | TypeData::Foundation { thickness, .. } => *thickness,
            TypeData::Wall { curtain: Some(c), .. } => c.thickness(),
            _ => self.layers().iter().map(|l| l.thickness).sum(),
        }
    }

    /// Outside diameter of pipe, conduit and rebar types.
    pub fn diameter(&self) -> Option<f64> {
        match self.data {
            TypeData::Pipe { diameter, .. }
            | TypeData::Conduit { diameter, .. }
            | TypeData::Rebar { diameter, .. }
            | TypeData::LightningConductor { diameter, .. } => Some(diameter),
            _ => None,
        }
    }

    /// Outside width, height and style of cable tray types.
    pub fn tray_section(&self) -> Option<(f64, f64, crate::TrayStyle)> {
        match self.data {
            TypeData::CableTray { width, height, style, .. } => Some((width, height, style)),
            _ => None,
        }
    }

    /// What valves of a valve type do.
    pub fn valve_kind(&self) -> Option<crate::ValveKind> {
        match self.data {
            TypeData::Valve { kind, .. } => Some(kind),
            _ => None,
        }
    }

    /// The curtain grid of a curtain wall type.
    pub fn curtain(&self) -> Option<&crate::curtain::CurtainSystem> {
        match &self.data {
            TypeData::Wall { curtain, .. } => curtain.as_ref(),
            _ => None,
        }
    }

    /// Wall function, for wall types.
    pub fn wall_function(&self) -> Option<WallFunction> {
        match self.data {
            TypeData::Wall { function, .. } => Some(function),
            _ => None,
        }
    }

    /// Distances from the exterior (first) face to the exterior and interior
    /// faces of the core: the span from the first to the last structure layer,
    /// or the whole thickness when there is none.
    pub fn core_range(&self) -> (f64, f64) {
        let layers = self.layers();
        let first = layers.iter().position(|l| l.function == LayerFunction::Structure);
        let last = layers.iter().rposition(|l| l.function == LayerFunction::Structure);
        match (first, last) {
            (Some(a), Some(b)) => {
                let before: f64 = layers[..a].iter().map(|l| l.thickness).sum();
                let core: f64 = layers[a..=b].iter().map(|l| l.thickness).sum();
                (before, before + core)
            }
            _ => (0.0, self.thickness()),
        }
    }

    /// Position of a wall's location line `line` across a wall of this type,
    /// measured from the centerline towards the exterior (left) face.
    pub fn location_offset(&self, line: crate::LocationLine) -> f64 {
        use crate::LocationLine as L;
        let half = self.thickness() * 0.5;
        let (core_ext, core_int) = self.core_range();
        match line {
            L::Centerline => 0.0,
            L::FinishExterior => half,
            L::FinishInterior => -half,
            L::CoreExterior => half - core_ext,
            L::CoreInterior => half - core_int,
            L::CoreCenter => half - (core_ext + core_int) * 0.5,
        }
    }

    /// Width/height of an opening type (doors, windows).
    /// Whether walls of this type are made structural (a wall type's default).
    pub fn structural_wall(&self) -> bool {
        matches!(self.data, TypeData::Wall { structural: true, .. })
    }

    pub fn opening_size(&self) -> Option<(f64, f64)> {
        match &self.data {
            TypeData::Door { width, height, .. } | TypeData::Window { width, height, .. } => Some((*width, *height)),
            TypeData::Component { def, .. } => def.opening_size(),
            _ => None,
        }
    }

    /// The depth of a niche type's recess; `None` for types that open their
    /// wall through (doors, windows) and for anything else.
    pub fn opening_depth(&self) -> Option<f64> {
        match &self.data {
            TypeData::Component { def, .. } if def.kind == crate::ComponentKind::Niche => def.opening_depth(),
            _ => None,
        }
    }

    /// How the panels of a window type open (fixed for anything else).
    pub fn window_operation(&self) -> crate::WindowOperation {
        match &self.data {
            TypeData::Component { def, .. } if def.kind == crate::ComponentKind::Window => {
                def.opening.as_ref().map(|o| o.operation).unwrap_or_default()
            }
            _ => crate::WindowOperation::Fixed,
        }
    }

    /// How the leaves of a door type swing (one leaf hinged on the left for anything else).
    pub fn door_operation(&self) -> crate::DoorOperation {
        match &self.data {
            TypeData::Component { def, .. } if def.kind == crate::ComponentKind::Door => {
                def.opening.as_ref().map(|o| o.door_operation).unwrap_or_default()
            }
            _ => crate::DoorOperation::SingleSwingLeft,
        }
    }

    /// Sill height of new elements of a door or window type.
    pub fn default_sill(&self) -> f64 {
        match &self.data {
            TypeData::Window { default_sill, .. } => *default_sill,
            TypeData::Component { def, .. } => def.default_sill(),
            _ => 0.0,
        }
    }

    /// Material used to color the element in shaded views: the exterior finish
    /// layer for walls and roofs, the thickest layer for floors.
    pub fn main_material(&self) -> &str {
        match &self.data {
            TypeData::Door { material, .. }
            | TypeData::Window { material, .. }
            | TypeData::Ceiling { material, .. } => material,
            TypeData::Wall { curtain: Some(c), .. } => c.main_material(),
            TypeData::Wall { layers, .. } | TypeData::Roof { layers, .. } => {
                layers.first().map_or("", |l| l.material.as_str())
            }
            TypeData::Room {}
            | TypeData::Grid { .. }
            | TypeData::Dimension { .. }
            | TypeData::Text { .. }
            | TypeData::LevelMark { .. }
            | TypeData::BarTag { .. }
            | TypeData::Tag { .. }
            | TypeData::ViewMark { .. }
            | TypeData::CadImport {}
            | TypeData::DetailLine {}
            | TypeData::FilledRegion { .. }
            | TypeData::MaskingRegion {}
            | TypeData::Shaft {}
            | TypeData::InteriorMarker {}
            | TypeData::SteelBuilding {}
            | TypeData::Truss {}
            | TypeData::Bracing {}
            | TypeData::Connection {}
            | TypeData::WallFraming {}
            | TypeData::Pad { .. } => "",
            TypeData::Column { material, .. }
            | TypeData::Beam { material, .. }
            | TypeData::Foundation { material, .. }
            | TypeData::Rebar { material, .. }
            | TypeData::Pile { material, .. }
            | TypeData::Pipe { material, .. }
            | TypeData::Conduit { material, .. }
            | TypeData::CableTray { material, .. }
            | TypeData::Valve { material, .. }
            | TypeData::LightningConductor { material, .. }
            | TypeData::Stair { material, .. }
            | TypeData::Ramp { material, .. }
            | TypeData::Railing { material, .. }
            | TypeData::Mass { material }
            | TypeData::Topography { material, .. }
            | TypeData::Sweep { material, .. } => material,
            TypeData::Component { def, .. } => def.main_material(),
            TypeData::Floor { .. } => self
                .layers()
                .iter()
                .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
                .map(|l| l.material.as_str())
                .unwrap_or(""),
        }
    }
}

/// Riser count, riser height and run length of a straight stair of `height`
/// with at most `max_riser` per riser and `tread`-deep treads (one tread fewer
/// than risers: the last riser lands on the upper floor).
pub fn stair_layout(height: f64, max_riser: f64, tread: f64) -> (usize, f64, f64) {
    let max_riser = if max_riser.is_finite() && max_riser > 0.01 { max_riser } else { 0.18 };
    let risers = ((height / max_riser).ceil().max(1.0) as usize).min(200);
    let riser = height / risers as f64;
    (risers, riser, tread.max(0.0) * (risers.saturating_sub(1)) as f64)
}
