//! Schedule definitions: which fields a schedule shows, how rows are
//! filtered, sorted and grouped, how each column is formatted and how the
//! table looks on paper. The values are computed by `cc_views::schedule`.

use crate::Category;
use crate::graphics::FilterOp;
use serde::{Deserialize, Serialize};

/// A column a schedule can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScheduleField {
    Mark,
    Type,
    Level,
    Comments,
    TypeMark,
    Description,
    Manufacturer,
    Model,
    FireRating,
    Cost,
    Length,
    Height,
    Width,
    Thickness,
    Area,
    Perimeter,
    Volume,
    Sill,
    Host,
    Function,
    Structural,
    Number,
    Name,
    Risers,
    RiserHeight,
    Run,
    Depth,
    ComponentName,
    ComponentKind,
    /// Section size of a column or beam.
    Section,
    /// Name of the profile of a column or beam type, whatever its material (tags show it only for steel and
    /// cold-formed profiles).
    Profile,
    /// Mass per meter of a column or beam, kg/m.
    LinearMass,
    /// Length of a beam's body after its joins.
    CutLength,
    /// Main material of the type.
    Material,
    /// Bar diameter of reinforcement.
    Diameter,
    /// Number of bars in a reinforcement set.
    BarCount,
    /// Length of one bar.
    BarLength,
    /// Steel weight, kilograms.
    Weight,
    /// The part of the structure a reinforcement set belongs to.
    Partition,
    /// Elevation of a pile's head (its cut-off).
    CutOff,
    /// Elevation of a pile's toe.
    Toe,
    /// How a pile is made.
    PileKind,
    /// Number of piles under a pile cap.
    PileCount,
    /// Plumbing system or electrical circuits.
    System,
    /// Fall of a pipe per unit of length.
    Slope,
    /// Apparent power of an electrical device, VA.
    Power,
    /// Drainage fixture units of a plumbing fixture.
    DrainUnits,
    /// The phase the element is built in.
    PhaseCreated,
    /// The phase the element is demolished in.
    PhaseDemolished,
    /// Gross floor area of a mass: the sum of its mass floors.
    FloorArea,
    /// Number of tiles on a roof.
    TileCount,
    /// Total length of a roof's framing members.
    FramingLength,
    /// Volume of a roof's framing members.
    FramingVolume,
    /// How many mass floors a mass has.
    FloorCount,
    /// Number of panels of a curtain wall.
    PanelCount,
    /// Area of a curtain wall's glass panels.
    GlazedArea,
    /// Total length of a curtain wall's mullions.
    MullionLength,
    /// Elevation of a building pad above its level.
    Elevation,
    /// Volume of earth cut by a pad (a terrain: by its pads).
    CutVolume,
    /// Volume of earth filled by a pad (a terrain: by its pads).
    FillVolume,
    /// Estimated length of the horizontal cables from a data outlet to its rack.
    CableLength,
    /// Kind of a connection (base plate, end plate…).
    ConnectionKind,
    /// Steel mass of a connection's plates, kilograms.
    PlateMass,
    /// Number of bolts of a connection.
    BoltCount,
    /// Bolt diameter of a connection.
    BoltDiameter,
    /// Inside diameter of a pipe (its series row's, else its type's own).
    InsideDiameter,
    /// The pipe series of a pipe's type.
    Series,
    /// The size of a pipe in its series (`"25 mm"`, `"1/2\""`).
    Size,
    /// How many elements the row stands for (1, or the group's when rows are not itemized).
    Count,
}

impl ScheduleField {
    /// The field named `name` as written in files (`Width`, `FireRating`).
    pub fn from_name(name: &str) -> Option<ScheduleField> {
        use serde::de::IntoDeserializer;
        let de: serde::de::value::StrDeserializer<'_, serde::de::value::Error> = name.into_deserializer();
        ScheduleField::deserialize(de).ok()
    }

    /// Translation key of the column header.
    pub fn key(self) -> &'static str {
        match self {
            ScheduleField::Mark => "col-mark",
            ScheduleField::Count => "col-element-count",
            ScheduleField::Type => "col-type",
            ScheduleField::Level => "col-level",
            ScheduleField::Comments => "col-comments",
            ScheduleField::TypeMark => "info-type-mark",
            ScheduleField::Description => "info-description",
            ScheduleField::Manufacturer => "info-manufacturer",
            ScheduleField::Model => "info-model",
            ScheduleField::FireRating => "info-fire-rating",
            ScheduleField::Cost => "info-cost",
            ScheduleField::Length => "col-length",
            ScheduleField::Height => "col-height",
            ScheduleField::Width => "col-width",
            ScheduleField::Thickness => "col-thickness",
            ScheduleField::Area => "col-area",
            ScheduleField::Perimeter => "col-perimeter",
            ScheduleField::Volume => "col-volume",
            ScheduleField::Sill => "col-sill",
            ScheduleField::Host => "col-host",
            ScheduleField::Function => "prop-function",
            ScheduleField::Structural => "prop-structural",
            ScheduleField::Number => "col-number",
            ScheduleField::Name => "col-name",
            ScheduleField::Risers => "col-risers",
            ScheduleField::RiserHeight => "col-riser-height",
            ScheduleField::Run => "col-run",
            ScheduleField::Depth => "col-depth",
            ScheduleField::ComponentName => "col-component",
            ScheduleField::ComponentKind => "col-component-kind",
            ScheduleField::Section => "col-section",
            ScheduleField::Profile => "col-profile",
            ScheduleField::LinearMass => "col-linear-mass",
            ScheduleField::CutLength => "col-cut-length",
            ScheduleField::Material => "col-material",
            ScheduleField::Diameter => "col-diameter",
            ScheduleField::BarCount => "col-bar-count",
            ScheduleField::BarLength => "col-bar-length",
            ScheduleField::Weight => "col-weight",
            ScheduleField::Partition => "col-partition",
            ScheduleField::CutOff => "col-cut-off",
            ScheduleField::Toe => "col-toe",
            ScheduleField::PileKind => "col-pile-kind",
            ScheduleField::PileCount => "col-pile-count",
            ScheduleField::System => "col-system",
            ScheduleField::Slope => "col-slope",
            ScheduleField::Power => "col-power",
            ScheduleField::DrainUnits => "col-drain-units",
            ScheduleField::PhaseCreated => "col-phase-created",
            ScheduleField::PhaseDemolished => "col-phase-demolished",
            ScheduleField::FloorArea => "col-floor-area",
            ScheduleField::FloorCount => "col-floor-count",
            ScheduleField::TileCount => "col-tile-count",
            ScheduleField::FramingLength => "col-framing-length",
            ScheduleField::FramingVolume => "col-framing-volume",
            ScheduleField::PanelCount => "col-panel-count",
            ScheduleField::GlazedArea => "col-glazed-area",
            ScheduleField::MullionLength => "col-mullion-length",
            ScheduleField::Elevation => "col-elevation",
            ScheduleField::CutVolume => "col-cut-volume",
            ScheduleField::FillVolume => "col-fill-volume",
            ScheduleField::CableLength => "col-cable-length",
            ScheduleField::InsideDiameter => "col-inside-diameter",
            ScheduleField::Series => "col-pipe-series",
            ScheduleField::Size => "col-pipe-size",
            ScheduleField::ConnectionKind => "col-connection-kind",
            ScheduleField::PlateMass => "col-plate-mass",
            ScheduleField::BoltCount => "col-bolt-count",
            ScheduleField::BoltDiameter => "col-bolt-diameter",
        }
    }

    /// Translation key of the column header in a project of `system`. The
    /// metric headers of lengths, areas and volumes name their unit
    /// (`Length (m)`); imperial values carry their own marks and symbols
    /// (`12'-6"`, `100.00 ft²`), so those headers name none.
    pub fn key_in(self, system: crate::MeasurementSystem) -> &'static str {
        if system == crate::MeasurementSystem::Metric {
            return self.key();
        }
        match self {
            ScheduleField::Length => "col-imp-length",
            ScheduleField::Thickness => "col-imp-thickness",
            ScheduleField::Area => "col-imp-area",
            ScheduleField::Perimeter => "col-imp-perimeter",
            ScheduleField::Volume => "col-imp-volume",
            _ => self.key(),
        }
    }

    /// Fields that make sense for `category`, in a sensible order.
    pub fn available(category: Category) -> Vec<ScheduleField> {
        use ScheduleField as F;
        let common = [F::Mark, F::Type, F::Level, F::Comments, F::Count];
        let type_info = [F::TypeMark, F::Description, F::Manufacturer, F::Model, F::FireRating, F::Cost];
        let phases = [F::PhaseCreated, F::PhaseDemolished];
        let specific: &[F] = match category {
            Category::Wall => &[
                F::Length,
                F::Height,
                F::Thickness,
                F::Area,
                F::Volume,
                F::Function,
                F::Structural,
                F::PanelCount,
                F::GlazedArea,
                F::MullionLength,
            ],
            Category::Door => &[F::Width, F::Height, F::Sill, F::Host, F::ComponentName],
            Category::Window => &[F::Width, F::Height, F::Sill, F::Host, F::ComponentName],
            Category::Niche => &[F::Width, F::Height, F::Sill, F::Host, F::ComponentName, F::Volume],
            Category::Floor | Category::Ceiling => &[F::Thickness, F::Area, F::Perimeter, F::Volume],
            Category::Roof => &[
                F::Thickness,
                F::Area,
                F::Perimeter,
                F::Volume,
                F::Slope,
                F::TileCount,
                F::Weight,
                F::FramingLength,
                F::FramingVolume,
            ],
            Category::Column => {
                &[F::Section, F::Profile, F::LinearMass, F::Height, F::Volume, F::Material, F::Structural]
            }
            Category::Beam => &[F::Section, F::Profile, F::LinearMass, F::Length, F::CutLength, F::Volume, F::Material],
            Category::Foundation => {
                &[F::Section, F::Length, F::Area, F::Thickness, F::Volume, F::Material, F::PileCount]
            }
            Category::Pile => {
                &[F::PileKind, F::Diameter, F::Section, F::CutOff, F::Toe, F::Length, F::Volume, F::Host, F::Material]
            }
            Category::Rebar => {
                &[F::Host, F::Partition, F::Diameter, F::BarCount, F::BarLength, F::Length, F::Weight, F::Material]
            }
            Category::Stair => &[F::Width, F::Risers, F::RiserHeight, F::Run, F::Volume],
            Category::Ramp => &[F::Width, F::Slope, F::Run, F::Volume],
            Category::Railing => &[F::Length, F::Height],
            Category::Sweep => &[F::Length, F::Volume],
            Category::Shaft => &[F::Area, F::Height],
            Category::InteriorMarker => &[],
            Category::SteelBuilding => &[],
            Category::Truss => &[],
            Category::Bracing => &[],
            Category::WallFraming => &[],
            Category::Connection => {
                &[F::ConnectionKind, F::Host, F::PlateMass, F::BoltCount, F::BoltDiameter, F::Volume]
            }
            Category::Topography => &[F::Area, F::Volume, F::Material, F::CutVolume, F::FillVolume],
            Category::Pad => &[F::Elevation, F::Area, F::CutVolume, F::FillVolume],
            Category::Pipe => &[
                F::Diameter,
                F::InsideDiameter,
                F::Series,
                F::Size,
                F::Length,
                F::CutLength,
                F::System,
                F::Slope,
                F::Material,
            ],
            Category::Conduit => &[F::Diameter, F::Length, F::System, F::Material],
            Category::CableTray => &[F::Width, F::Height, F::Length, F::System, F::Material],
            Category::Valve => &[F::Host, F::Diameter, F::System, F::Material],
            Category::LightningConductor => &[F::Function, F::Diameter, F::Length, F::System, F::Material],
            Category::Component => &[
                F::ComponentName,
                F::ComponentKind,
                F::Width,
                F::Depth,
                F::Height,
                F::Volume,
                F::System,
                F::Power,
                F::DrainUnits,
                F::CableLength,
            ],
            Category::Mass => &[F::Height, F::FloorCount, F::FloorArea, F::Volume],
            Category::Room => {
                return vec![
                    F::Number,
                    F::Name,
                    F::Count,
                    F::Level,
                    F::Area,
                    F::Perimeter,
                    F::Height,
                    F::Volume,
                    F::Comments,
                    F::PhaseCreated,
                    F::PhaseDemolished,
                ];
            }
            Category::Grid
            | Category::Level
            | Category::Dimension
            | Category::Text
            | Category::DetailLine
            | Category::FilledRegion
            | Category::MaskingRegion
            | Category::DetailComponent
            | Category::LevelMark
            | Category::BarTag
            | Category::Tag
            | Category::ViewMark
            | Category::CadImport
            | Category::TitleBlock => {
                return Vec::new();
            }
        };
        common.iter().chain(specific).chain(&type_info).chain(&phases).copied().collect()
    }

    /// Whether elements of `category` can have a value for this field: the
    /// fields [`available`](Self::available) to its schedules, plus the type,
    /// level, main material, volume and count, which most elements have. A filter
    /// rule on a field that applies to none of its categories never matches.
    pub fn applies_to(self, category: Category) -> bool {
        use ScheduleField as F;
        matches!(self, F::Type | F::Level | F::Material | F::Volume | F::Count)
            || Self::available(category).contains(&self)
    }

    /// Fields shown by a new schedule of `category`.
    pub fn defaults(category: Category) -> Vec<ScheduleField> {
        use ScheduleField as F;
        match category {
            Category::Wall => vec![F::Mark, F::Type, F::Level, F::Length, F::Height, F::Thickness, F::Area, F::Volume],
            Category::Door => vec![F::Mark, F::Type, F::Width, F::Height, F::Level, F::Host, F::Comments],
            Category::Window => vec![F::Mark, F::Type, F::Width, F::Height, F::Sill, F::Level, F::Host, F::Comments],
            Category::Niche => vec![F::Mark, F::Type, F::Width, F::Height, F::Sill, F::Volume, F::Level, F::Host],
            Category::Floor | Category::Ceiling | Category::Roof => {
                vec![F::Mark, F::Type, F::Level, F::Area, F::Perimeter, F::Volume]
            }
            Category::Column => vec![F::Mark, F::Type, F::Level, F::Section, F::Height, F::Volume, F::Material],
            Category::Beam => {
                vec![F::Mark, F::Type, F::Level, F::Section, F::Length, F::CutLength, F::Volume, F::Material]
            }
            Category::Foundation => vec![F::Mark, F::Type, F::Level, F::Section, F::Thickness, F::Volume, F::Material],
            Category::Pile => {
                vec![F::Mark, F::Type, F::Diameter, F::CutOff, F::Toe, F::Length, F::Volume, F::Host]
            }
            Category::Rebar => vec![F::Host, F::Type, F::Diameter, F::BarCount, F::BarLength, F::Length, F::Weight],
            Category::Stair => vec![F::Mark, F::Type, F::Level, F::Risers, F::RiserHeight, F::Run, F::Volume],
            Category::Ramp => vec![F::Mark, F::Type, F::Level, F::Slope, F::Run, F::Volume],
            Category::Railing => vec![F::Mark, F::Type, F::Level, F::Length],
            Category::Sweep => vec![F::Mark, F::Type, F::Level, F::Length, F::Volume],
            Category::Pipe => vec![F::Type, F::System, F::Level, F::Diameter, F::Length, F::Slope],
            Category::Conduit => vec![F::Type, F::System, F::Level, F::Diameter, F::Length],
            Category::CableTray => vec![F::Type, F::System, F::Level, F::Width, F::Height, F::Length],
            Category::Valve => vec![F::Mark, F::Type, F::Diameter, F::System, F::Level],
            Category::LightningConductor => vec![F::Type, F::System, F::Level, F::Function, F::Diameter, F::Length],
            Category::Component => vec![F::Mark, F::ComponentName, F::Type, F::Level, F::Width, F::Depth, F::Height],
            Category::Room => vec![F::Number, F::Name, F::Level, F::Area, F::Perimeter],
            Category::Mass => vec![F::Mark, F::Type, F::Level, F::Height, F::FloorCount, F::FloorArea, F::Volume],
            Category::Shaft => vec![F::Mark, F::Type, F::Level, F::Area, F::Height],
            Category::InteriorMarker => Vec::new(),
            Category::SteelBuilding => Vec::new(),
            Category::Truss => Vec::new(),
            Category::Bracing => Vec::new(),
            Category::WallFraming => Vec::new(),
            Category::Connection => {
                vec![F::Mark, F::ConnectionKind, F::PlateMass, F::BoltCount, F::BoltDiameter, F::Comments]
            }
            Category::Topography => vec![F::Mark, F::Type, F::Level, F::Area, F::Volume],
            Category::Pad => vec![F::Mark, F::Type, F::Level, F::Elevation, F::Area, F::CutVolume, F::FillVolume],
            Category::Grid
            | Category::Level
            | Category::Dimension
            | Category::Text
            | Category::DetailLine
            | Category::FilledRegion
            | Category::MaskingRegion
            | Category::DetailComponent
            | Category::LevelMark
            | Category::BarTag
            | Category::Tag
            | Category::ViewMark
            | Category::CadImport
            | Category::TitleBlock => Vec::new(),
        }
    }
}

/// What a schedule shows. Empty `fields` means the category defaults (files
/// written before schedules were configurable).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleDef {
    pub fields: Vec<ScheduleField>,
    /// Sort rows by this field (the default order is by mark or number).
    /// The first level of sorting of files written before [`Self::sorting`].
    pub sort_by: Option<ScheduleField>,
    pub descending: bool,
    /// Keep only rows whose value of the field contains the text
    /// (case-insensitive). The filter of files written before [`Self::filters`].
    pub filter: Option<(ScheduleField, String)>,
    /// Hide the totals row.
    pub hide_totals: bool,
    /// Keep only the rows that pass every one of these.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<ScheduleFilter>,
    /// Sort the rows by these, the first outermost; a level may head and
    /// foot the rows it groups.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sorting: Vec<SortLevel>,
    /// One row for the elements alike in every sorting level, instead of
    /// one per element: values that differ among them read as varying, and
    /// totaled columns add up.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub collapse: bool,
    /// How columns are written, for the fields that differ from the usual.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ColumnFormat>,
    /// How the table looks on paper.
    #[serde(skip_serializing_if = "ScheduleAppearance::is_default")]
    pub appearance: ScheduleAppearance,
    /// A reinforcement schedule as a bending schedule: one row per bar mark
    /// instead of one per set (the fields, sort and filter do not apply).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bending: Option<BendingOptions>,
}

/// Most partitions a bending schedule is limited to.
pub const MAX_BENDING_PARTITIONS: usize = 100;
/// Largest waste a bending schedule adds, percent.
pub const MAX_WASTE_PERCENT: f64 = 100.0;

/// What a bending schedule shows: one row per bar mark (mark, diameter,
/// grade, shape sketch with its dimensions, length of one bar, count, total
/// length, weight; rows of bars longer than the stock length noted), then a
/// summary per diameter and grade (total length and weight, with the waste
/// added when given).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BendingOptions {
    /// Only the sets of these partitions (none: every set; an empty text
    /// stands for the sets without one).
    pub partitions: Vec<String>,
    /// Rows under a heading per partition (each partition's marks counted
    /// on their own).
    pub group_by_partition: bool,
    /// Waste added in the summary, percent (none: no waste columns).
    pub waste_percent: Option<f64>,
}

impl BendingOptions {
    /// Why the options cannot be kept (`None`: they can), as a message.
    pub fn check(&self) -> Option<String> {
        if self.partitions.len() > MAX_BENDING_PARTITIONS {
            return Some(format!("at most {MAX_BENDING_PARTITIONS} partitions"));
        }
        if self.partitions.iter().any(|p| p.chars().count() > crate::bar_marks::MAX_MARK_TEXT) {
            return Some(format!("partitions have at most {} characters", crate::bar_marks::MAX_MARK_TEXT));
        }
        if self.waste_percent.is_some_and(|w| !(w.is_finite() && (0.0..=MAX_WASTE_PERCENT).contains(&w))) {
            return Some(format!("waste_percent must be between 0 and {MAX_WASTE_PERCENT}"));
        }
        None
    }

    /// Bring the options into range; returns how many changed.
    pub fn repair(&mut self) -> usize {
        let mut n = 0;
        if self.partitions.len() > MAX_BENDING_PARTITIONS {
            self.partitions.truncate(MAX_BENDING_PARTITIONS);
            n += 1;
        }
        let before = self.partitions.len();
        self.partitions.retain(|p| p.chars().count() <= crate::bar_marks::MAX_MARK_TEXT);
        n += before - self.partitions.len();
        if self.waste_percent.is_some_and(|w| !(w.is_finite() && (0.0..=MAX_WASTE_PERCENT).contains(&w))) {
            self.waste_percent = None;
            n += 1;
        }
        n
    }

    /// Whether a set of `partition` is shown.
    pub fn shows(&self, partition: &str) -> bool {
        self.partitions.is_empty() || self.partitions.iter().any(|p| p.trim().eq_ignore_ascii_case(partition.trim()))
    }
}

/// Most filters of one schedule.
pub const MAX_FILTERS: usize = 8;
/// Most levels of sorting of one schedule.
pub const MAX_SORT_LEVELS: usize = 4;
/// Longest filter value or column heading, in characters.
pub const MAX_SCHEDULE_TEXT: usize = 120;
/// Smallest and largest text of a schedule on paper, millimeters.
pub const MIN_SCHEDULE_TEXT_SIZE: f64 = 1.0;
pub const MAX_SCHEDULE_TEXT_SIZE: f64 = 20.0;

/// One filter: rows whose `field` compares as `op` says with `value` stay.
/// A number is compared as a number when the row's value is one (lengths in
/// meters, areas in square meters, and so on), else as text without case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleFilter {
    pub field: ScheduleField,
    /// The comparison, as view filters have them ([`FilterOp::test`]).
    pub op: FilterOp,
    #[serde(default)]
    pub value: String,
}

/// One level of sorting: rows in the order of `field`; a level may write a
/// heading before each group of rows alike in it, a footer after it (the
/// group's count and totals) and leave a blank line between groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortLevel {
    pub field: ScheduleField,
    #[serde(default)]
    pub descending: bool,
    #[serde(default)]
    pub header: bool,
    #[serde(default)]
    pub footer: bool,
    #[serde(default)]
    pub blank_line: bool,
}

impl SortLevel {
    pub fn new(field: ScheduleField) -> Self {
        Self { field, descending: false, header: false, footer: false, blank_line: false }
    }

    /// Whether the level groups its rows (a heading, a footer or a blank line).
    pub fn groups(&self) -> bool {
        self.header || self.footer || self.blank_line
    }
}

/// Where a column's texts stand in their cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ColumnAlign {
    /// Numbers to the right, texts to the left.
    #[default]
    Auto,
    Left,
    Center,
    Right,
}

impl ColumnAlign {
    pub const ALL: [ColumnAlign; 4] = [ColumnAlign::Auto, ColumnAlign::Left, ColumnAlign::Center, ColumnAlign::Right];

    /// Translation key of the alignment's name.
    pub fn key(self) -> &'static str {
        match self {
            ColumnAlign::Auto => "align-auto",
            ColumnAlign::Left => "align-left",
            ColumnAlign::Center => "align-center",
            ColumnAlign::Right => "align-right",
        }
    }
}

/// How the column of `field` is written: its own heading, its alignment,
/// whether it is left out of the table (a hidden field still sorts and
/// filters) and whether it is totaled (`None`: as the field usually is).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnFormat {
    pub field: ScheduleField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    #[serde(default)]
    pub align: ColumnAlign,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<bool>,
}

impl ColumnFormat {
    pub fn new(field: ScheduleField) -> Self {
        Self { field, heading: None, align: ColumnAlign::Auto, hidden: false, total: None }
    }

    /// Whether nothing differs from the usual (the entry may be dropped).
    pub fn is_plain(&self) -> bool {
        self.heading.as_deref().is_none_or(|h| h.trim().is_empty())
            && self.align == ColumnAlign::Auto
            && !self.hidden
            && self.total.is_none()
    }
}

/// How a schedule looks on paper (sheets and exports): its rules, its title
/// and headers, and the sizes of its texts in millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleAppearance {
    /// Rules between rows and columns.
    pub grid_lines: bool,
    /// The heavier rule around the table.
    pub outline: bool,
    /// The row with the schedule's name.
    pub show_title: bool,
    /// The row with the columns' headings.
    pub show_headers: bool,
    pub title_size: f64,
    pub header_size: f64,
    pub text_size: f64,
}

impl Default for ScheduleAppearance {
    fn default() -> Self {
        Self {
            grid_lines: true,
            outline: true,
            show_title: true,
            show_headers: true,
            title_size: 3.0,
            header_size: 2.2,
            text_size: 2.2,
        }
    }
}

impl ScheduleAppearance {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    fn sizes(&mut self) -> [(&mut f64, f64); 3] {
        let d = Self::default();
        [
            (&mut self.title_size, d.title_size),
            (&mut self.header_size, d.header_size),
            (&mut self.text_size, d.text_size),
        ]
    }

    /// Bring the text sizes within their limits; returns how many changed.
    pub fn repair(&mut self) -> usize {
        let mut n = 0;
        for (size, default) in self.sizes() {
            if !(size.is_finite() && (MIN_SCHEDULE_TEXT_SIZE..=MAX_SCHEDULE_TEXT_SIZE).contains(size)) {
                *size =
                    if size.is_finite() { size.clamp(MIN_SCHEDULE_TEXT_SIZE, MAX_SCHEDULE_TEXT_SIZE) } else { default };
                n += 1;
            }
        }
        n
    }
}

impl ScheduleDef {
    /// The fields to show for `category`.
    pub fn fields_for(&self, category: Category) -> Vec<ScheduleField> {
        if self.fields.is_empty() { ScheduleField::defaults(category) } else { self.fields.clone() }
    }

    /// Every filter: the older single one (a text the value contains), then [`Self::filters`].
    pub fn all_filters(&self) -> Vec<ScheduleFilter> {
        let old = self.filter.iter().filter(|(_, text)| !text.trim().is_empty()).map(|(field, text)| ScheduleFilter {
            field: *field,
            op: FilterOp::Contains,
            value: text.clone(),
        });
        old.chain(self.filters.iter().cloned()).take(MAX_FILTERS).collect()
    }

    /// Every level of sorting: [`Self::sorting`], or the older single field.
    pub fn all_sorting(&self) -> Vec<SortLevel> {
        if !self.sorting.is_empty() {
            return self.sorting.iter().copied().take(MAX_SORT_LEVELS).collect();
        }
        self.sort_by
            .map(|field| SortLevel { descending: self.descending, ..SortLevel::new(field) })
            .into_iter()
            .collect()
    }

    /// How the column of `field` is written.
    pub fn format_of(&self, field: ScheduleField) -> ColumnFormat {
        self.columns.iter().find(|c| c.field == field).cloned().unwrap_or_else(|| ColumnFormat::new(field))
    }

    /// The older single filter and sort field moved into [`Self::filters`]
    /// and [`Self::sorting`], which the dialogs edit.
    pub fn normalize(&mut self) {
        if self.filter.is_some() {
            self.filters = self.all_filters();
            self.filter = None;
        }
        if self.sort_by.is_some() {
            self.sorting = self.all_sorting();
            self.sort_by = None;
            self.descending = false;
        }
    }

    /// Why the definition cannot be kept (`None`: it can), as a message.
    pub fn check(&self) -> Option<String> {
        let long = |t: &str| t.chars().count() > MAX_SCHEDULE_TEXT;
        if self.filters.len() > MAX_FILTERS {
            return Some(format!("a schedule takes at most {MAX_FILTERS} filters"));
        }
        if self.sorting.len() > MAX_SORT_LEVELS {
            return Some(format!("a schedule takes at most {MAX_SORT_LEVELS} levels of sorting"));
        }
        if self.filters.iter().any(|f| long(&f.value))
            || self.columns.iter().any(|c| c.heading.as_deref().is_some_and(long))
        {
            return Some(format!("filter values and headings have at most {MAX_SCHEDULE_TEXT} characters"));
        }
        let mut seen = std::collections::BTreeSet::new();
        if !self.columns.iter().all(|c| seen.insert(c.field)) {
            return Some("a field has one column format".into());
        }
        if self.appearance.clone().repair() > 0 {
            return Some(format!(
                "text sizes must be between {MIN_SCHEDULE_TEXT_SIZE} and {MAX_SCHEDULE_TEXT_SIZE} mm"
            ));
        }
        self.bending.as_ref().and_then(BendingOptions::check)
    }

    /// Bring the definition within its limits; returns how many values changed.
    pub fn repair(&mut self) -> usize {
        let mut n = 0;
        let cut = |t: &mut String| match t.char_indices().nth(MAX_SCHEDULE_TEXT) {
            Some((end, _)) => {
                t.truncate(end);
                1
            }
            None => 0,
        };
        if self.filters.len() > MAX_FILTERS {
            self.filters.truncate(MAX_FILTERS);
            n += 1;
        }
        if self.sorting.len() > MAX_SORT_LEVELS {
            self.sorting.truncate(MAX_SORT_LEVELS);
            n += 1;
        }
        n += self.filters.iter_mut().map(|f| cut(&mut f.value)).sum::<usize>();
        n += self.columns.iter_mut().filter_map(|c| c.heading.as_mut()).map(cut).sum::<usize>();
        let mut seen = std::collections::BTreeSet::new();
        let before = self.columns.len();
        self.columns.retain(|c| seen.insert(c.field));
        n += before - self.columns.len();
        n += self.appearance.repair();
        n + self.bending.as_mut().map_or(0, BendingOptions::repair)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_available() {
        for c in Category::SCHEDULABLE {
            let available = ScheduleField::available(c);
            for f in ScheduleField::defaults(c) {
                assert!(available.contains(&f), "{c:?} default {f:?} not available");
            }
        }
    }
}
