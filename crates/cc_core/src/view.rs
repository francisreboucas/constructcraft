use crate::{Category, ElementTypeId, LevelId, MeasurementSystem, ViewId};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// Grouping used by the Project Browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViewCategory {
    FloorPlan,
    CeilingPlan,
    ThreeD,
    Elevation,
    Section,
    Legend,
    /// 2D views drawn by hand, independent of the model.
    Drafting,
    Schedule,
    /// Single-line diagrams of distribution boards.
    Diagram,
    Sheet,
}

impl ViewCategory {
    pub const ALL: [ViewCategory; 10] = [
        ViewCategory::FloorPlan,
        ViewCategory::CeilingPlan,
        ViewCategory::ThreeD,
        ViewCategory::Elevation,
        ViewCategory::Section,
        ViewCategory::Legend,
        ViewCategory::Drafting,
        ViewCategory::Schedule,
        ViewCategory::Diagram,
        ViewCategory::Sheet,
    ];

    /// Translation key for the browser group heading.
    pub fn key(self) -> &'static str {
        match self {
            ViewCategory::FloorPlan => "browser-floor-plans",
            ViewCategory::CeilingPlan => "browser-ceiling-plans",
            ViewCategory::ThreeD => "browser-3d-views",
            ViewCategory::Elevation => "browser-elevations",
            ViewCategory::Section => "browser-sections",
            ViewCategory::Legend => "browser-legends",
            ViewCategory::Drafting => "browser-drafting-views",
            ViewCategory::Schedule => "browser-schedules",
            ViewCategory::Diagram => "browser-diagrams",
            ViewCategory::Sheet => "browser-sheets",
        }
    }
}

/// Elevation direction, named by the side the viewer stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    /// Unit vector the viewer looks along (from their side towards the model).
    pub fn look(self) -> DVec2 {
        match self {
            Direction::North => DVec2::NEG_Y,
            Direction::South => DVec2::Y,
            Direction::East => DVec2::NEG_X,
            Direction::West => DVec2::X,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    pub target: DVec3,
    /// Rotation around Z, degrees; 0 looks along +Y.
    pub yaw_deg: f64,
    /// Elevation angle above the horizon, degrees.
    pub pitch_deg: f64,
    pub distance: f64,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { target: DVec3::new(5.0, 4.0, 1.5), yaw_deg: 35.0, pitch_deg: 30.0, distance: 25.0 }
    }
}

impl CameraState {
    /// Unit vector from the eye towards the target.
    pub fn forward(&self) -> DVec3 {
        let (yaw, pitch) = (self.yaw_deg.to_radians(), self.pitch_deg.to_radians());
        DVec3::new(yaw.sin() * pitch.cos(), yaw.cos() * pitch.cos(), -pitch.sin())
    }

    pub fn eye(&self) -> DVec3 {
        self.target - self.forward() * self.distance
    }
}

/// Smallest and largest side of a custom sheet, millimeters.
pub const MIN_PAPER_MM: f64 = 50.0;
pub const MAX_PAPER_MM: f64 = 5000.0;

/// A custom paper side within the limits (the smallest when not a number).
fn paper_side(v: f64) -> f64 {
    if v.is_nan() { MIN_PAPER_MM } else { v.clamp(MIN_PAPER_MM, MAX_PAPER_MM) }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SheetSize {
    A0,
    A1,
    A2,
    A3,
    A4,
    /// Any size, millimeters (width and height as typed; `portrait` puts the long side up).
    Custom {
        width: f64,
        height: f64,
    },
    /// ANSI A, 11" × 8 1/2" (US letter).
    AnsiA,
    /// ANSI B, 17" × 11" (tabloid).
    AnsiB,
    /// ANSI C, 22" × 17".
    AnsiC,
    /// ANSI D, 34" × 22".
    AnsiD,
    /// ANSI E, 44" × 34".
    AnsiE,
    /// Architectural A, 12" × 9".
    ArchA,
    /// Architectural B, 18" × 12".
    ArchB,
    /// Architectural C, 24" × 18".
    ArchC,
    /// Architectural D, 36" × 24".
    ArchD,
    /// Architectural E1, 42" × 30".
    ArchE1,
    /// Architectural E, 48" × 36".
    ArchE,
}

/// Millimeters in an inch, for paper sizes.
const PAPER_INCH: f64 = 25.4;

fn yes() -> bool {
    true
}

impl SheetSize {
    /// Every named size: the ISO A sizes, then ANSI, then architectural.
    pub const ALL: [SheetSize; 16] = [
        SheetSize::A0,
        SheetSize::A1,
        SheetSize::A2,
        SheetSize::A3,
        SheetSize::A4,
        SheetSize::AnsiA,
        SheetSize::AnsiB,
        SheetSize::AnsiC,
        SheetSize::AnsiD,
        SheetSize::AnsiE,
        SheetSize::ArchA,
        SheetSize::ArchB,
        SheetSize::ArchC,
        SheetSize::ArchD,
        SheetSize::ArchE1,
        SheetSize::ArchE,
    ];

    /// The ISO A sizes.
    const ISO: [SheetSize; 5] = [SheetSize::A0, SheetSize::A1, SheetSize::A2, SheetSize::A3, SheetSize::A4];

    /// The US sizes: ANSI, then architectural.
    const US: [SheetSize; 11] = [
        SheetSize::AnsiA,
        SheetSize::AnsiB,
        SheetSize::AnsiC,
        SheetSize::AnsiD,
        SheetSize::AnsiE,
        SheetSize::ArchA,
        SheetSize::ArchB,
        SheetSize::ArchC,
        SheetSize::ArchD,
        SheetSize::ArchE1,
        SheetSize::ArchE,
    ];

    /// Every named size, the system's own first (metric: the A sizes;
    /// imperial: ANSI and architectural), for pickers.
    pub fn choices(system: MeasurementSystem) -> Vec<SheetSize> {
        match system {
            MeasurementSystem::Metric => Self::ISO.iter().chain(&Self::US).copied().collect(),
            MeasurementSystem::Imperial => Self::US.iter().chain(&Self::ISO).copied().collect(),
        }
    }

    /// The paper of a new sheet: A3, or ANSI B in imperial projects.
    pub fn default_for(system: MeasurementSystem) -> SheetSize {
        match system {
            MeasurementSystem::Metric => SheetSize::A3,
            MeasurementSystem::Imperial => SheetSize::AnsiB,
        }
    }

    /// Landscape paper size in millimeters (a custom size as typed, kept
    /// within [`MIN_PAPER_MM`] and [`MAX_PAPER_MM`]).
    pub fn size_mm(self) -> DVec2 {
        let inches = |w: f64, h: f64| DVec2::new(w * PAPER_INCH, h * PAPER_INCH);
        match self {
            SheetSize::A0 => DVec2::new(1189.0, 841.0),
            SheetSize::A1 => DVec2::new(841.0, 594.0),
            SheetSize::A2 => DVec2::new(594.0, 420.0),
            SheetSize::A3 => DVec2::new(420.0, 297.0),
            SheetSize::A4 => DVec2::new(297.0, 210.0),
            SheetSize::AnsiA => inches(11.0, 8.5),
            SheetSize::AnsiB => inches(17.0, 11.0),
            SheetSize::AnsiC => inches(22.0, 17.0),
            SheetSize::AnsiD => inches(34.0, 22.0),
            SheetSize::AnsiE => inches(44.0, 34.0),
            SheetSize::ArchA => inches(12.0, 9.0),
            SheetSize::ArchB => inches(18.0, 12.0),
            SheetSize::ArchC => inches(24.0, 18.0),
            SheetSize::ArchD => inches(36.0, 24.0),
            SheetSize::ArchE1 => inches(42.0, 30.0),
            SheetSize::ArchE => inches(48.0, 36.0),
            SheetSize::Custom { width, height } => DVec2::new(paper_side(width), paper_side(height)),
        }
    }

    /// The size's name (`A3`, `ANSI B`, `ARCH D`); a custom size in
    /// millimeters (see [`SheetSize::label_in`]).
    pub fn label(self) -> String {
        self.label_in(MeasurementSystem::Metric)
    }

    /// The size's name; a custom size in the system's paper unit
    /// (`1000 × 594 mm`, `17" × 11"`).
    pub fn label_in(self, system: MeasurementSystem) -> String {
        let name = match self {
            SheetSize::A0 => "A0",
            SheetSize::A1 => "A1",
            SheetSize::A2 => "A2",
            SheetSize::A3 => "A3",
            SheetSize::A4 => "A4",
            SheetSize::AnsiA => "ANSI A",
            SheetSize::AnsiB => "ANSI B",
            SheetSize::AnsiC => "ANSI C",
            SheetSize::AnsiD => "ANSI D",
            SheetSize::AnsiE => "ANSI E",
            SheetSize::ArchA => "ARCH A",
            SheetSize::ArchB => "ARCH B",
            SheetSize::ArchC => "ARCH C",
            SheetSize::ArchD => "ARCH D",
            SheetSize::ArchE1 => "ARCH E1",
            SheetSize::ArchE => "ARCH E",
            SheetSize::Custom { width, height } => {
                return match system {
                    MeasurementSystem::Metric => format!("{} × {} mm", width.round(), height.round()),
                    MeasurementSystem::Imperial => {
                        let inches = |mm: f64| crate::units::inch_text(mm / 1000.0);
                        format!("{} × {}", inches(width), inches(height))
                    }
                };
            }
        };
        name.to_string()
    }

    /// The named size called `name`, any case, with spaces, `_` or `-`
    /// between its parts or none (`A3`, `ANSI B`, `ansi_b`, `ArchD`).
    pub fn from_name(name: &str) -> Option<SheetSize> {
        let key: String =
            name.chars().filter(|c| !c.is_whitespace() && !matches!(c, '_' | '-')).collect::<String>().to_uppercase();
        Self::ALL.into_iter().find(|s| s.label().replace(' ', "") == key)
    }

    /// A custom size within the limits, finite.
    pub fn is_valid(self) -> bool {
        match self {
            SheetSize::Custom { width, height } => {
                [width, height].iter().all(|v| v.is_finite() && (MIN_PAPER_MM..=MAX_PAPER_MM).contains(v))
            }
            _ => true,
        }
    }
}

/// A view placed on a sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub view: ViewId,
    /// Center of the viewport on the sheet, millimeters from the bottom-left corner.
    pub center: DVec2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewKind {
    FloorPlan {
        level: LevelId,
        cut_height: f64,
        /// A structural (formwork) plan: only the structure, with section tags.
        #[serde(default)]
        structural: bool,
        /// Tag beams and columns with their mark and section.
        #[serde(default)]
        structure_tags: bool,
        /// A building services plan: the discipline's pipes or conduits and
        /// devices over the architecture, tagged. Other plans leave them out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mep: Option<MepDiscipline>,
        /// Other levels' plans drawn halftone under this one.
        #[serde(default, skip_serializing_if = "Underlay::is_empty")]
        underlay: Underlay,
    },
    CeilingPlan {
        level: LevelId,
        cut_height: f64,
        /// Other levels' plans drawn halftone under this one.
        #[serde(default, skip_serializing_if = "Underlay::is_empty")]
        underlay: Underlay,
    },
    ThreeD {
        camera: CameraState,
        /// Show the analytical model (member lines and nodes) over a faint model.
        #[serde(default)]
        analytical: bool,
        /// Show the reinforcing bars over a faint model.
        #[serde(default)]
        rebar: bool,
    },
    Elevation {
        direction: Direction,
    },
    /// A face of an interior elevation marker (see [`crate::interior`]): the
    /// wall on one of its sides seen from the marker, or one wall of its
    /// space seen square on. `origin` and `look` are worked out from the
    /// marker and the wall by every edit; the depth and the crop come from
    /// the space around the marker when the view is drawn.
    InteriorElevation {
        marker: crate::ElementId,
        #[serde(alias = "side")]
        face: crate::interior::Face,
        /// A wall's view: whether it shows the wall's left face (to the left
        /// of its line from start to end) or its right one.
        #[serde(default)]
        left: bool,
        /// The number in the view's name, among its marker's views.
        number: u32,
        /// Where the view looks from (see [`crate::Project::interior_frame`]).
        origin: DVec2,
        /// The unit direction it looks along.
        look: DVec2,
        /// Crop to the space around the marker; off, the view keeps the
        /// crop drawn on it ([`View::crop`]).
        #[serde(default = "yes")]
        auto_extents: bool,
    },
    /// Section line from `start` to `end`; looks to the right of the line.
    Section {
        start: DVec2,
        end: DVec2,
        depth: f64,
        /// A split line: plans draw only this much of it, in meters, next to
        /// the head at the start and at the end (`None`: the whole line).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        segments: Option<[f64; 2]>,
    },
    Legend {
        types: Vec<ElementTypeId>,
    },
    /// A 2D view independent of the model, with its own scale: it shows no
    /// model geometry, only what is drawn in it (the elements it owns, such
    /// as text notes, dimensions and imported CAD drawings).
    Drafting {},
    Schedule {
        category: Category,
        #[serde(default)]
        def: crate::ScheduleDef,
    },
    Sheet {
        number: String,
        size: SheetSize,
        /// The long side runs up.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        portrait: bool,
        viewports: Vec<Viewport>,
        /// The title block type drawn on it; `None` draws the blank paper.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title_block: Option<crate::ElementTypeId>,
        /// Values of the title block's per-sheet fields, by parameter name.
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        fields: std::collections::BTreeMap<String, String>,
        /// Issue date, ISO `YYYY-MM-DD`; empty when not issued.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        issue_date: String,
    },
    /// The single-line diagram and load schedule of a distribution board.
    Diagram {
        panel: crate::ElementId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub id: ViewId,
    pub name: String,
    /// Scale denominator (50 means 1:50).
    pub scale: u32,
    pub kind: ViewKind,
    /// Category visibility and overrides, filters, element overrides.
    #[serde(default, skip_serializing_if = "crate::ViewGraphics::is_empty")]
    pub graphics: crate::ViewGraphics,
    /// The template the view follows: the properties it controls are
    /// copied from it on every edit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<crate::TemplateId>,
    /// The phase the view shows the model in (see [`crate::phase`]).
    #[serde(default)]
    pub phase: crate::PhaseId,
    /// Which statuses the view shows, and how (`None`: all, by category).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_filter: Option<crate::PhaseFilterId>,
    /// The option the view shows of each design option set; sets left out
    /// show automatically (see [`crate::design_option`]).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub design_options: std::collections::BTreeMap<crate::OptionSetId, crate::OptionId>,
    /// Tags moved by hand in files before format 29, by the element they
    /// tag: read only, moved into [`View::tags`] by repair.
    #[serde(default, skip_serializing)]
    pub tag_offsets: std::collections::BTreeMap<crate::ElementId, glam::DVec2>,
    /// How this view adjusts the tags of elements (moved, turned, another
    /// type, hidden, leader), by the element they tag (see [`crate::tag`]).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub tags: std::collections::BTreeMap<crate::ElementId, crate::tag::TagPlacement>,
    /// The tag type this view tags each target with (missing: the
    /// project's first tag type of the target).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub tag_types: std::collections::BTreeMap<crate::tag::TagTarget, crate::ElementTypeId>,
    /// Sections and elevations: the view mark type their mark in plans is
    /// drawn with (`None`: the project's first of its kind; see [`crate::view_mark`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark_type: Option<crate::ElementTypeId>,
    /// Floor plans: how far this plan moved the markers of elevations from
    /// where they sit, meters in plan, by elevation view.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub mark_offsets: std::collections::BTreeMap<ViewId, glam::DVec2>,
    /// Elevations and sections: how the view shows each level (hidden,
    /// which ends carry a head, elbows), by level (see [`crate::level_head`]).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub level_heads: std::collections::BTreeMap<crate::LevelId, crate::level_head::LevelHead>,
    /// How much of the elements the view draws (see [`DetailLevel`]).
    #[serde(default, skip_serializing_if = "DetailLevel::is_medium")]
    pub detail: DetailLevel,
    /// How the view shows the model's surfaces (`None`: the usual style of
    /// its kind, see [`View::style`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_style: Option<VisualStyle>,
    /// The rectangle of the view's drawing outside which the model is not
    /// drawn (plans, sections and elevations; see [`crate::callout`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<crate::callout::CropBox>,
    /// The callout standing for this view in another one: a detail view
    /// of the model (cropped to it) or a drafting view it points to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callout: Option<crate::callout::Callout>,
    /// The view's sun: a date and time, a study or a light direction
    /// (see [`crate::sun`]); only views that [take one](ViewKind::takes_sun).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun: Option<crate::sun::SunSettings>,
    /// How the view is rendered photorealistically (see
    /// [`crate::render_settings`]); only views that [take them](ViewKind::takes_render).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<crate::render_settings::RenderSettings>,
    /// How light and dark the shaded, consistent colors and textures
    /// styles shade faces and cast shadows (see [`ShadingSettings`]); used
    /// by the views that [take a sun](ViewKind::takes_sun), kept at the
    /// defaults on the others.
    #[serde(default, skip_serializing_if = "ShadingSettings::is_default")]
    pub shading: ShadingSettings,
    /// Floor plans, elevations and sections: draw every reinforcing bar
    /// within the view's range over the model (see
    /// [`ViewKind::takes_rebar`]). 3D views show reinforcement with
    /// [`ViewKind::ThreeD`]'s `rebar` instead.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rebar: bool,
    /// Services plans: label each fitting of the plan's discipline (its
    /// catalog piece, else its kind and size; a missing one marked).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fitting_tags: bool,
    /// How the plan adjusts the label of each fitting, by the runs meeting
    /// there ([`crate::tag::fitting_key`]).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fitting_placements: std::collections::BTreeMap<String, crate::tag::TagPlacement>,
}

/// How a view shades faces and casts shadows in the styles that show more
/// than white faces (shaded, consistent colors, textures), in 2D and in
/// the 3D view (not in photorealistic renders). The defaults are the
/// look of [`VisualStyle::shaded_grey`] and [`VisualStyle::SHADED_SHADOW`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShadingSettings {
    /// The light of a face turned away from the light, as displayed:
    /// [`ShadingSettings::MIN_AMBIENT`] (dark grey) to 1 (as bright as a
    /// face square to the light: no shading).
    pub ambient: f64,
    /// How dark cast shadows are: 0 (no visible shadow) to 1 (black).
    /// In the shaded style a shadow's grey is `1 - shadow` (never lighter
    /// than the face it falls on); the ground's and the other styles'
    /// shadows grow from none to black with it, their defaults at the default.
    pub shadow: f64,
}

impl Default for ShadingSettings {
    fn default() -> Self {
        ShadingSettings { ambient: Self::DEFAULT_AMBIENT, shadow: Self::DEFAULT_SHADOW }
    }
}

impl ShadingSettings {
    pub const MIN_AMBIENT: f64 = 0.3;
    pub const MAX_AMBIENT: f64 = 1.0;
    pub const DEFAULT_AMBIENT: f64 = VisualStyle::SHADED_AMBIENT;
    pub const MIN_SHADOW: f64 = 0.0;
    pub const MAX_SHADOW: f64 = 1.0;
    /// The intensity whose shaded shadow is [`VisualStyle::SHADED_SHADOW`].
    pub const DEFAULT_SHADOW: f64 = 1.0 - VisualStyle::SHADED_SHADOW;

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Both values are numbers within their ranges.
    pub fn is_valid(&self) -> bool {
        (Self::MIN_AMBIENT..=Self::MAX_AMBIENT).contains(&self.ambient)
            && (Self::MIN_SHADOW..=Self::MAX_SHADOW).contains(&self.shadow)
    }

    /// Bring the values into range (the defaults when not numbers);
    /// returns how many it fixed.
    pub fn repair(&mut self) -> usize {
        fn fix(v: &mut f64, lo: f64, hi: f64, default: f64) -> usize {
            let fixed = if v.is_finite() { v.clamp(lo, hi) } else { default };
            usize::from(std::mem::replace(v, fixed) != fixed)
        }
        fix(&mut self.ambient, Self::MIN_AMBIENT, Self::MAX_AMBIENT, Self::DEFAULT_AMBIENT)
            + fix(&mut self.shadow, Self::MIN_SHADOW, Self::MAX_SHADOW, Self::DEFAULT_SHADOW)
    }

    /// The values brought into range (see [`Self::repair`]).
    pub fn clamped(mut self) -> Self {
        self.repair();
        self
    }

    /// The shaded style's grey, as displayed (0 black, 1 white), of a face
    /// whose unit normal has the cosine `cos` with the direction towards the
    /// light: white square to it, [`Self::ambient`] edge-on or turned away;
    /// not a number reads as turned away.
    pub fn grey(&self, cos: f64) -> f64 {
        let ambient = self.clamped().ambient;
        let cos = if cos.is_nan() { 0.0 } else { cos.clamp(0.0, 1.0) };
        ambient + (1.0 - ambient) * cos
    }

    /// The grey of a cast shadow in the shaded style on a face whose grey
    /// is `lit`: `1 - shadow`, never lighter than the face.
    pub fn shadow_grey(&self, lit: f64) -> f64 {
        lit.min(1.0 - self.clamped().shadow)
    }

    /// A shadow's darkening (0 none, 1 black) that grows with
    /// [`Self::shadow`] from none to black and is `at_default` at the
    /// default intensity (`at_default` between 0 and 1, exclusive): for
    /// the shadows whose darkness is not the shaded style's grey (the
    /// ground's, the unlit styles').
    pub fn darkening(&self, at_default: f64) -> f64 {
        let s = self.clamped().shadow;
        if s <= 0.0 || !(at_default > 0.0 && at_default < 1.0) {
            return if s <= 0.0 { 0.0 } else { at_default.clamp(0.0, 1.0) };
        }
        s.powf(at_default.ln() / Self::DEFAULT_SHADOW.ln()).clamp(0.0, 1.0)
    }
}

/// How a view shows the model's surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum VisualStyle {
    /// Every edge, seen through the model; no faces.
    Wireframe,
    /// Faces in the paper's color hide what is behind them; only edges show.
    HiddenLine,
    /// White faces with the hidden line's edges, shaded grey by the light
    /// (the view's sun, else a light from the viewer's side; see
    /// [`VisualStyle::shaded_grey`]) and, where the view's sun casts
    /// shadows, a darker grey in them; no material colors.
    #[default]
    Shaded,
    /// Faces in their material's color, the same on every side (no light).
    ConsistentColors,
    /// Faces in their material's appearance color with its surface pattern, lit.
    Textures,
}

impl VisualStyle {
    pub const ALL: [VisualStyle; 5] = [
        VisualStyle::Wireframe,
        VisualStyle::HiddenLine,
        VisualStyle::Shaded,
        VisualStyle::ConsistentColors,
        VisualStyle::Textures,
    ];

    pub fn key(self) -> &'static str {
        match self {
            VisualStyle::Wireframe => "style-wireframe",
            VisualStyle::HiddenLine => "style-hidden-line",
            VisualStyle::Shaded => "style-shaded",
            VisualStyle::ConsistentColors => "style-consistent-colors",
            VisualStyle::Textures => "style-textures",
        }
    }

    /// The name commands use: `wireframe`, `hidden_line`, `shaded`, `consistent_colors` or `textures`.
    pub fn name(self) -> &'static str {
        match self {
            VisualStyle::Wireframe => "wireframe",
            VisualStyle::HiddenLine => "hidden_line",
            VisualStyle::Shaded => "shaded",
            VisualStyle::ConsistentColors => "consistent_colors",
            VisualStyle::Textures => "textures",
        }
    }

    pub fn from_name(name: &str) -> Option<VisualStyle> {
        let name = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        VisualStyle::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Faces are filled (every style but the wireframe).
    pub fn fills(self) -> bool {
        self != VisualStyle::Wireframe
    }

    /// Faces show more than the paper's white: grey shading (shaded), their
    /// material's color (consistent colors) or appearance (textures).
    pub fn colored(self) -> bool {
        matches!(self, VisualStyle::Shaded | VisualStyle::ConsistentColors | VisualStyle::Textures)
    }

    /// The grey of a face lit in the shaded style where nothing casts a
    /// shadow on it, as displayed (0 black, 1 white).
    pub const SHADED_AMBIENT: f64 = 0.6;
    /// How much a face square to the light adds to [`Self::SHADED_AMBIENT`]: it is white.
    pub const SHADED_DIFFUSE: f64 = 0.4;
    /// The grey of a cast shadow in the shaded style: darker than a face
    /// turned away from the light, so shadows read apart from shade.
    pub const SHADED_SHADOW: f64 = 0.45;

    /// The shaded style's grey, as displayed (0 black, 1 white), of a face
    /// whose unit normal has the cosine `cos` with the direction towards the
    /// light: white square to it, [`Self::SHADED_AMBIENT`] (a mid grey, never
    /// black) edge-on or turned away; not a number reads as turned away.
    /// The [default shading](ShadingSettings::grey)'s; a view's own is [`View::shading`].
    pub fn shaded_grey(cos: f64) -> f64 {
        ShadingSettings::default().grey(cos)
    }
}

impl View {
    /// A sheet's paper size in millimeters, after its orientation.
    pub fn paper_mm(&self) -> Option<DVec2> {
        let ViewKind::Sheet { size, portrait, .. } = &self.kind else { return None };
        let s = size.size_mm();
        let (long, short) = (s.x.max(s.y), s.x.min(s.y));
        Some(match (size, portrait) {
            (SheetSize::Custom { .. }, false) => s,
            (_, true) => DVec2::new(short, long),
            (_, false) => DVec2::new(long, short),
        })
    }

    /// The style the view is drawn in: its own, else plans in hidden line
    /// (as drawn before styles existed) and other views shaded.
    pub fn style(&self) -> VisualStyle {
        self.visual_style.unwrap_or(match self.kind {
            ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. } => VisualStyle::HiddenLine,
            _ => VisualStyle::Shaded,
        })
    }
}

/// How much a view draws of elements: at `Coarse`, layered walls, floors
/// and roofs are cut as one region with a plain fill (no layers, no
/// material patterns); at `Medium` and `Fine` they show their layers and
/// patterns. Component pieces show at the levels they are set for
/// ([`crate::component::DetailLevels`]), so a component can draw a simple
/// shape at `Coarse` and its details at `Fine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DetailLevel {
    Coarse,
    #[default]
    Medium,
    Fine,
}

impl DetailLevel {
    pub const ALL: [DetailLevel; 3] = [DetailLevel::Coarse, DetailLevel::Medium, DetailLevel::Fine];

    pub fn key(self) -> &'static str {
        match self {
            DetailLevel::Coarse => "detail-coarse",
            DetailLevel::Medium => "detail-medium",
            DetailLevel::Fine => "detail-fine",
        }
    }

    /// The name commands use: `coarse`, `medium` or `fine`.
    pub fn name(self) -> &'static str {
        match self {
            DetailLevel::Coarse => "coarse",
            DetailLevel::Medium => "medium",
            DetailLevel::Fine => "fine",
        }
    }

    pub fn is_medium(&self) -> bool {
        *self == DetailLevel::Medium
    }

    /// Layered elements show their layers and material patterns.
    pub fn shows_layers(self) -> bool {
        self != DetailLevel::Coarse
    }
}

/// Longest piece of a split section line kept next to a head, meters.
pub const MAX_SECTION_SEGMENT: f64 = 1000.0;

impl ViewKind {
    /// Turn a section to look the other way: its ends swap, and the pieces
    /// of a split line go with them. `false` for other views.
    pub fn flip_section(&mut self) -> bool {
        let ViewKind::Section { start, end, segments, .. } = self else { return false };
        std::mem::swap(start, end);
        if let Some(s) = segments {
            s.swap(0, 1);
        }
        true
    }

    /// Whether views of this kind have visibility/graphics settings: the
    /// views of the model (those that take templates) and drafting views.
    pub fn has_graphics(&self) -> bool {
        crate::ViewTemplate::applies_to(self) || matches!(self, ViewKind::Drafting { .. })
    }

    /// Whether views of this kind show the model in a phase (plans,
    /// elevations, sections, 3D views and schedules).
    pub fn is_phased(&self) -> bool {
        matches!(
            self,
            ViewKind::FloorPlan { .. }
                | ViewKind::CeilingPlan { .. }
                | ViewKind::ThreeD { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::InteriorElevation { .. }
                | ViewKind::Section { .. }
                | ViewKind::Schedule { .. }
        )
    }

    /// Whether views of this kind can have a sun ([`View::sun`]): plans,
    /// ceiling plans, 3D views, elevations and sections.
    pub fn takes_sun(&self) -> bool {
        matches!(
            self,
            ViewKind::FloorPlan { .. }
                | ViewKind::CeilingPlan { .. }
                | ViewKind::ThreeD { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::InteriorElevation { .. }
                | ViewKind::Section { .. }
        )
    }

    /// Whether views of this kind can show the reinforcement ([`View::rebar`]):
    /// floor plans, elevations and sections.
    pub fn takes_rebar(&self) -> bool {
        matches!(
            self,
            ViewKind::FloorPlan { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::InteriorElevation { .. }
                | ViewKind::Section { .. }
        )
    }

    /// Whether views of this kind can have render settings ([`View::render`]):
    /// 3D views only.
    pub fn takes_render(&self) -> bool {
        matches!(self, ViewKind::ThreeD { .. })
    }

    /// An ordinary ceiling plan of `level` cut at 1.20 m.
    pub fn ceiling_plan(level: LevelId) -> Self {
        ViewKind::CeilingPlan { level, cut_height: 1.2, underlay: Underlay::default() }
    }

    /// The underlay of a floor or ceiling plan.
    pub fn underlay(&self) -> Option<&Underlay> {
        match self {
            ViewKind::FloorPlan { underlay, .. } | ViewKind::CeilingPlan { underlay, .. } => Some(underlay),
            _ => None,
        }
    }

    pub fn underlay_mut(&mut self) -> Option<&mut Underlay> {
        match self {
            ViewKind::FloorPlan { underlay, .. } | ViewKind::CeilingPlan { underlay, .. } => Some(underlay),
            _ => None,
        }
    }

    /// An ordinary floor plan of `level` cut at 1.20 m.
    pub fn floor_plan(level: LevelId) -> Self {
        ViewKind::FloorPlan {
            level,
            cut_height: 1.2,
            structural: false,
            structure_tags: false,
            mep: None,
            underlay: Underlay::default(),
        }
    }
}

/// The building services shown by a services plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MepDiscipline {
    /// Water supply, drainage, their valves and fixtures.
    Plumbing,
    /// Conduits, cable trays, outlets, switches, lights and boards.
    Electrical,
    /// Structured cabling: cable trays, conduits carrying it, data outlets and racks.
    Communications,
    /// Lightning protection: air terminals, conductors and earth electrodes.
    Lightning,
}

impl MepDiscipline {
    pub const ALL: [MepDiscipline; 4] =
        [MepDiscipline::Plumbing, MepDiscipline::Electrical, MepDiscipline::Communications, MepDiscipline::Lightning];

    /// Name in commands (`plumbing`, `electrical`, `communications`, `lightning`).
    pub fn name(self) -> &'static str {
        match self {
            MepDiscipline::Plumbing => "plumbing",
            MepDiscipline::Electrical => "electrical",
            MepDiscipline::Communications => "communications",
            MepDiscipline::Lightning => "lightning",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            MepDiscipline::Plumbing => "discipline-plumbing",
            MepDiscipline::Electrical => "discipline-electrical",
            MepDiscipline::Communications => "discipline-communications",
            MepDiscipline::Lightning => "discipline-lightning",
        }
    }
}

impl View {
    /// A view with default graphics, no template, in the (default) last phase.
    pub fn new(id: ViewId, name: impl Into<String>, scale: u32, kind: ViewKind) -> Self {
        Self {
            id,
            name: name.into(),
            scale,
            kind,
            graphics: Default::default(),
            template: None,
            phase: Default::default(),
            phase_filter: None,
            design_options: Default::default(),
            tag_offsets: Default::default(),
            tags: Default::default(),
            tag_types: Default::default(),
            mark_type: None,
            mark_offsets: Default::default(),
            level_heads: Default::default(),
            detail: DetailLevel::Medium,
            visual_style: None,
            crop: None,
            callout: None,
            sun: None,
            render: None,
            shading: ShadingSettings::default(),
            rebar: false,
            fitting_tags: false,
            fitting_placements: Default::default(),
        }
    }

    /// Whether the view can label fittings: a floor plan of services.
    pub fn takes_fitting_tags(&self) -> bool {
        matches!(self.kind, ViewKind::FloorPlan { mep: Some(_), .. })
    }

    /// Whether the view draws the reinforcement: a 3D view of it, or a
    /// plan, elevation or section with [`View::rebar`] on.
    pub fn shows_rebar(&self) -> bool {
        match self.kind {
            ViewKind::ThreeD { rebar, .. } => rebar,
            _ => self.rebar && self.kind.takes_rebar(),
        }
    }

    pub fn category(&self) -> ViewCategory {
        match self.kind {
            ViewKind::FloorPlan { .. } => ViewCategory::FloorPlan,
            ViewKind::CeilingPlan { .. } => ViewCategory::CeilingPlan,
            ViewKind::ThreeD { .. } => ViewCategory::ThreeD,
            ViewKind::Elevation { .. } | ViewKind::InteriorElevation { .. } => ViewCategory::Elevation,
            ViewKind::Section { .. } => ViewCategory::Section,
            ViewKind::Legend { .. } => ViewCategory::Legend,
            ViewKind::Drafting { .. } => ViewCategory::Drafting,
            ViewKind::Schedule { .. } => ViewCategory::Schedule,
            ViewKind::Sheet { .. } => ViewCategory::Sheet,
            ViewKind::Diagram { .. } => ViewCategory::Diagram,
        }
    }

    /// Level for plan views, where new elements are placed.
    pub fn level(&self) -> Option<LevelId> {
        match self.kind {
            ViewKind::FloorPlan { level, .. } | ViewKind::CeilingPlan { level, .. } => Some(level),
            _ => None,
        }
    }

    /// Whether the view is a drafting view: 2D, drawn by hand, showing no model geometry.
    pub fn is_drafting(&self) -> bool {
        matches!(self.kind, ViewKind::Drafting { .. })
    }

    /// Whether dimensions can be drawn in the view: plans, elevations,
    /// sections, legends and drafting views.
    pub fn takes_dimensions(&self) -> bool {
        self.is_plan()
            || self.projection().is_some()
            || matches!(self.kind, ViewKind::Legend { .. } | ViewKind::Drafting { .. })
    }

    /// Whether the view can be placed on a sheet.
    pub fn placeable(&self) -> bool {
        !matches!(self.kind, ViewKind::Sheet { .. })
    }
}

/// Other levels' plans drawn halftone under a floor or ceiling plan, for
/// reference while drawing: a plan below and one above, each seen looking
/// down (as a floor plan) or up (as a ceiling plan). They are not picked,
/// selected or snapped to, and are printed only when `printed`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Underlay {
    /// The plan of a level below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub below: Option<UnderlayPlan>,
    /// The plan of a level above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above: Option<UnderlayPlan>,
    /// Show it on sheets and in exports too (by default only on screen).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub printed: bool,
}

/// One plan shown under a view: which level, and from which side.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UnderlayPlan {
    pub level: UnderlayLevel,
    #[serde(default)]
    pub looking: Looking,
}

/// The level of an underlay: the next one (below or above the view's
/// level, so a template works for every level), or a given level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UnderlayLevel {
    #[default]
    Next,
    Level(LevelId),
}

/// Which way a plan looks: down (a floor plan) or up (a ceiling plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Looking {
    #[default]
    Down,
    Up,
}

impl Looking {
    pub const ALL: [Looking; 2] = [Looking::Down, Looking::Up];

    /// The translation key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Looking::Down => "underlay-looking-down",
            Looking::Up => "underlay-looking-up",
        }
    }
}

impl Underlay {
    pub fn is_empty(&self) -> bool {
        self.below.is_none() && self.above.is_none() && !self.printed
    }

    /// The levels shown under a plan of `level` and how they are seen,
    /// lowest first: the next level is the nearest below (or above) by
    /// elevation, a given level is shown when it still exists.
    pub fn plans(&self, levels: &[crate::Level], level: LevelId) -> Vec<(LevelId, Looking)> {
        let Some(own) = levels.iter().find(|l| l.id == level).map(|l| l.elevation) else { return Vec::new() };
        let next = |below: bool| {
            levels
                .iter()
                .filter(|l| if below { l.elevation < own } else { l.elevation > own })
                .min_by(|a, b| {
                    let (da, db) = ((a.elevation - own).abs(), (b.elevation - own).abs());
                    da.total_cmp(&db)
                })
                .map(|l| l.id)
        };
        let resolve = |plan: &Option<UnderlayPlan>, below: bool| {
            let p = (*plan)?;
            let id = match p.level {
                UnderlayLevel::Next => next(below)?,
                UnderlayLevel::Level(id) => levels.iter().find(|l| l.id == id && id != level)?.id,
            };
            Some((id, p.looking))
        };
        [resolve(&self.below, true), resolve(&self.above, false)].into_iter().flatten().collect()
    }

    /// Drop references to levels that no longer exist; returns how many.
    pub fn repair(&mut self, exists: impl Fn(LevelId) -> bool) -> usize {
        let mut n = 0;
        for slot in [&mut self.below, &mut self.above] {
            if let Some(UnderlayPlan { level: UnderlayLevel::Level(id), .. }) = slot
                && !exists(*id)
            {
                *slot = None;
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod paper_tests {
    use super::*;

    fn sheet(size: SheetSize, portrait: bool) -> View {
        let kind = ViewKind::Sheet {
            number: "A1".into(),
            size,
            portrait,
            viewports: Vec::new(),
            title_block: None,
            fields: Default::default(),
            issue_date: String::new(),
        };
        View::new(ViewId(1), "S", 100, kind)
    }

    #[test]
    fn paper_follows_size_and_orientation() {
        assert_eq!(sheet(SheetSize::A3, false).paper_mm(), Some(DVec2::new(420.0, 297.0)));
        assert_eq!(sheet(SheetSize::A3, true).paper_mm(), Some(DVec2::new(297.0, 420.0)));
        let custom = SheetSize::Custom { width: 1000.0, height: 594.0 };
        assert_eq!(sheet(custom, false).paper_mm(), Some(DVec2::new(1000.0, 594.0)));
        // Portrait puts the long side up whatever the order typed.
        assert_eq!(sheet(custom, true).paper_mm(), Some(DVec2::new(594.0, 1000.0)));
        assert_eq!(custom.label(), "1000 × 594 mm");
        assert_eq!(SheetSize::A1.label(), "A1");
        // Each side stays within the limits, whatever a file holds.
        let wild = SheetSize::Custom { width: 1e9, height: f64::NAN };
        assert_eq!(sheet(wild, false).paper_mm(), Some(DVec2::new(MAX_PAPER_MM, MIN_PAPER_MM)));
        let tiny = SheetSize::Custom { width: 1.0, height: f64::NEG_INFINITY };
        assert_eq!(sheet(tiny, true).paper_mm(), Some(DVec2::new(MIN_PAPER_MM, MIN_PAPER_MM)));
    }

    #[test]
    fn us_sheet_sizes() {
        let inches = |s: SheetSize| s.size_mm() / 25.4;
        for (size, w, h, label) in [
            (SheetSize::AnsiA, 11.0, 8.5, "ANSI A"),
            (SheetSize::AnsiB, 17.0, 11.0, "ANSI B"),
            (SheetSize::AnsiC, 22.0, 17.0, "ANSI C"),
            (SheetSize::AnsiD, 34.0, 22.0, "ANSI D"),
            (SheetSize::AnsiE, 44.0, 34.0, "ANSI E"),
            (SheetSize::ArchA, 12.0, 9.0, "ARCH A"),
            (SheetSize::ArchB, 18.0, 12.0, "ARCH B"),
            (SheetSize::ArchC, 24.0, 18.0, "ARCH C"),
            (SheetSize::ArchD, 36.0, 24.0, "ARCH D"),
            (SheetSize::ArchE1, 42.0, 30.0, "ARCH E1"),
            (SheetSize::ArchE, 48.0, 36.0, "ARCH E"),
        ] {
            let i = inches(size);
            assert!((i.x - w).abs() < 1e-9 && (i.y - h).abs() < 1e-9, "{label}: {i}");
            assert_eq!(size.label(), label);
            assert_eq!(size.label_in(MeasurementSystem::Imperial), label);
            assert!(size.is_valid());
            assert_eq!(SheetSize::from_name(label), Some(size));
            assert_eq!(SheetSize::from_name(&label.to_lowercase().replace(' ', "_")), Some(size));
            // Portrait turns it like the A sizes.
            assert_eq!(sheet(size, true).paper_mm(), Some(DVec2::new(h * 25.4, w * 25.4)));
        }
        assert_eq!(SheetSize::from_name("a3"), Some(SheetSize::A3));
        assert_eq!(SheetSize::from_name("ArchD"), Some(SheetSize::ArchD));
        assert_eq!(SheetSize::from_name("ansi-e"), Some(SheetSize::AnsiE));
        assert_eq!(SheetSize::from_name("Letter"), None);
        assert_eq!(SheetSize::from_name(""), None);
        let custom = SheetSize::Custom { width: 431.8, height: 279.4 };
        assert_eq!(custom.label_in(MeasurementSystem::Imperial), "17\" × 11\"");
        assert_eq!(custom.label_in(MeasurementSystem::Metric), "432 × 279 mm");
    }

    #[test]
    fn choices_put_the_system_first() {
        let metric = SheetSize::choices(MeasurementSystem::Metric);
        let imperial = SheetSize::choices(MeasurementSystem::Imperial);
        assert_eq!(metric[..5], [SheetSize::A0, SheetSize::A1, SheetSize::A2, SheetSize::A3, SheetSize::A4]);
        assert_eq!(imperial[0], SheetSize::AnsiA);
        assert_eq!(imperial[5], SheetSize::ArchA);
        assert_eq!(imperial[11], SheetSize::A0);
        // Both list every named size once.
        for list in [&metric, &imperial] {
            assert_eq!(list.len(), SheetSize::ALL.len());
            assert!(SheetSize::ALL.iter().all(|s| list.contains(s)));
        }
        assert_eq!(SheetSize::ALL[..5], metric[..5]);
        assert_eq!(SheetSize::default_for(MeasurementSystem::Metric), SheetSize::A3);
        assert_eq!(SheetSize::default_for(MeasurementSystem::Imperial), SheetSize::AnsiB);
    }

    #[test]
    fn us_sizes_round_trip() {
        for size in SheetSize::ALL {
            let view = sheet(size, false);
            let text = ron::to_string(&view.kind).unwrap();
            let back: ViewKind = ron::from_str(&text).unwrap();
            assert_eq!(back, view.kind);
        }
    }

    #[test]
    fn old_sheets_read_with_defaults() {
        let ron = r#"Sheet(number: "A101", size: A3, viewports: [])"#;
        let kind: ViewKind = ron::from_str(ron).unwrap();
        let ViewKind::Sheet { portrait, title_block, fields, issue_date, .. } = kind else { panic!() };
        assert!(!portrait && title_block.is_none() && fields.is_empty() && issue_date.is_empty());
    }

    /// The default shading is the shaded style's look; values out of range
    /// are brought in, and a view without its own shading writes none.
    #[test]
    fn shading_settings() {
        let d = ShadingSettings::default();
        for cos in [-1.0, 0.0, 0.3, 1.0, f64::NAN] {
            assert_eq!(d.grey(cos), VisualStyle::shaded_grey(cos));
        }
        assert_eq!(d.grey(0.0), VisualStyle::SHADED_AMBIENT);
        assert!((d.shadow_grey(1.0) - VisualStyle::SHADED_SHADOW).abs() < 1e-12);
        assert!((d.darkening(0.35) - 0.35).abs() < 1e-12);
        let none = ShadingSettings { shadow: 0.0, ..d };
        assert_eq!((none.shadow_grey(0.8), none.darkening(0.35)), (0.8, 0.0));
        let black = ShadingSettings { shadow: 1.0, ..d };
        assert_eq!((black.shadow_grey(0.8), black.darkening(0.35)), (0.0, 1.0));
        let flat = ShadingSettings { ambient: 1.0, ..d };
        assert_eq!((flat.grey(0.0), flat.grey(1.0)), (1.0, 1.0));
        let mut bad = ShadingSettings { ambient: 0.1, shadow: f64::NAN };
        assert!(!bad.is_valid());
        assert_eq!(bad.repair(), 2);
        assert_eq!(
            bad,
            ShadingSettings { ambient: ShadingSettings::MIN_AMBIENT, shadow: ShadingSettings::DEFAULT_SHADOW }
        );
        assert!(bad.is_valid());
        let view = View::new(ViewId(1), "v", 100, ViewKind::Drafting {});
        assert!(!ron::to_string(&view).unwrap().contains("shading"));
        let back: View = ron::from_str(&ron::to_string(&view).unwrap()).unwrap();
        assert_eq!(back.shading, d);
    }
}
