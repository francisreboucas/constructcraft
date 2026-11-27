//! Schedules: one row per element of a category, one column per field of the
//! schedule definition, with filtering, sorting and totals.

use crate::format;
use cc_core::{Category, Element, ElementKind, Project, ScheduleDef, ScheduleField, TypeData};
use cc_geom::{ModelGeometry, polygon};
use std::cmp::Ordering;

/// A cell: plain text, or a translation key for the application to resolve.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Key(&'static str),
}

impl Cell {
    /// The text, or the key itself for untranslated cells.
    pub fn raw(&self) -> &str {
        match self {
            Cell::Text(s) => s,
            Cell::Key(k) => k,
        }
    }
}

/// A computed schedule. Headers are translation keys.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub headers: Vec<&'static str>,
    pub rows: Vec<Row>,
    /// Totals row aligned with `headers` (the first cell holds the row count,
    /// unless its column is totaled); empty when totals are hidden.
    pub totals: Vec<String>,
    /// A table under this one (a bending schedule's summary per diameter).
    pub summary: Option<Box<Table>>,
    /// The title of a table drawn under another (a translation key).
    pub caption: Option<&'static str>,
    /// A column's own heading, where the schedule gives one instead of the
    /// header's translation (aligned with `headers`; shorter: none).
    pub titles: Vec<Option<String>>,
    /// Where each column's texts stand (aligned with `headers`; shorter: automatic).
    pub align: Vec<cc_core::ColumnAlign>,
    /// How the table looks on paper.
    pub appearance: cc_core::ScheduleAppearance,
}

impl Table {
    /// The headings of the columns: each its own title, else `label` of its header key.
    pub fn header_texts(&self, label: &dyn Fn(&str) -> String) -> Vec<String> {
        self.headers
            .iter()
            .enumerate()
            .map(|(i, key)| match self.titles.get(i).and_then(|t| t.as_deref()) {
                Some(title) => title.to_string(),
                None => label(key),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub element: cc_core::ElementId,
    pub cells: Vec<Cell>,
    /// A group's heading (its first cell names the group, across the row)
    /// rather than a row of values.
    pub heading: bool,
    /// A bar's shape drawn in cell `.0` instead of its text (bending schedules).
    pub sketch: Option<(usize, crate::bending::BarSketch)>,
    /// A group's footer: how many elements it holds in its first cell (unless
    /// that column is totaled), and its totals in the totaled columns.
    pub footer: bool,
}

impl Row {
    /// A row of values of `element`.
    pub fn new(element: cc_core::ElementId, cells: Vec<Cell>) -> Self {
        Self { element, cells, heading: false, sketch: None, footer: false }
    }

    /// A group's heading (or, with no text, a blank line) across `columns` cells.
    pub fn heading(element: cc_core::ElementId, text: Cell, columns: usize) -> Self {
        let mut cells = vec![Cell::Text(String::new()); columns.max(1)];
        if let Some(first) = cells.first_mut() {
            *first = text;
        }
        Self { element, cells, heading: true, sketch: None, footer: false }
    }
}

/// A field value before formatting, used for sorting and totals.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Text(String),
    Key(&'static str),
    Count(usize),
    Length(f64),
    /// A small length (a thickness, a riser, a tray's section): as a length
    /// in metric projects, in inches without feet in imperial ones.
    Size(f64),
    Area(f64),
    Volume(f64),
    Money(f64),
    /// Kilograms.
    Weight(f64),
    /// Kilograms per meter.
    LinearMass(f64),
    /// Volt-amperes.
    Power(f64),
    /// A plain quantity (fixture units).
    Number(f64),
    /// A quantity written its own way (a bar's `Ø 12.5`, a slope's `2 %`),
    /// with the number it filters and sorts by, in SI units (diameters in
    /// meters, slopes as rise over run).
    Measured(String, f64),
    Missing,
}

impl Value {
    fn cell(&self) -> Cell {
        match self {
            Value::Text(s) | Value::Measured(s, _) => Cell::Text(s.clone()),
            Value::Key(k) => Cell::Key(k),
            Value::Count(n) => Cell::Text(n.to_string()),
            Value::Length(v) => Cell::Text(format::meters(*v)),
            Value::Size(v) if format::imperial() => Cell::Text(format::inch_size(*v)),
            Value::Size(v) => Cell::Text(format::meters(*v)),
            Value::Area(v) => Cell::Text(format::area(*v)),
            Value::Money(v) => Cell::Text(format::money(*v)),
            Value::Volume(v) => Cell::Text(format::volume(*v)),
            Value::Weight(v) => Cell::Text(format::quantity(format::Quantity::Mass, *v, 2)),
            Value::LinearMass(v) => Cell::Text(format::quantity(format::Quantity::LinearMass, *v, 2)),
            Value::Power(v) => Cell::Text(format!("{} VA", format::number(*v, 0))),
            Value::Number(v) => Cell::Text(format::number(*v, 1)),
            Value::Missing => Cell::Text(String::new()),
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Value::Count(n) => Some(*n as f64),
            Value::Length(v)
            | Value::Size(v)
            | Value::Area(v)
            | Value::Volume(v)
            | Value::Money(v)
            | Value::Weight(v)
            | Value::LinearMass(v)
            | Value::Power(v)
            | Value::Number(v)
            | Value::Measured(_, v) => Some(*v),
            _ => None,
        }
    }
}

/// Natural order: numbers numerically (so "10" follows "9"), then text.
fn natural(a: &str, b: &str) -> Ordering {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => x.total_cmp(&y),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        _ => a.cmp(b),
    }
}

/// What rows are sorted by, worked out once per row: a value's number, and
/// its text with the number [`natural`] reads from it.
struct SortKey {
    number: Option<f64>,
    text: String,
    text_number: Option<f64>,
}

impl SortKey {
    fn of(v: &Value) -> Self {
        let text = text_of(v);
        Self { number: v.number(), text_number: text.trim().parse::<f64>().ok(), text }
    }

    /// Numbers numerically, else the texts in [`natural`] order.
    fn compare(&self, other: &Self) -> Ordering {
        match (self.number, other.number) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            _ => match (self.text_number, other.text_number) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                _ => self.text.cmp(&other.text),
            },
        }
    }
}

fn text_of(v: &Value) -> String {
    v.cell().raw().to_string()
}

/// Mass per meter of a column or beam type: its section's area times the
/// material's density. Empty when the material has no usable density.
fn linear_mass(project: &Project, data: &TypeData) -> Value {
    let (TypeData::Column { material, .. } | TypeData::Beam { material, .. }) = data else { return Value::Missing };
    let density = project.material(material).and_then(|m| m.physical.density).filter(|d| d.is_finite() && *d > 0.0);
    let Some(props) = density.and_then(|d| project.type_section_properties(data, d)) else { return Value::Missing };
    let mass = props.linear_mass;
    if mass.is_finite() && mass > 0.0 { Value::LinearMass(mass) } else { Value::Missing }
}

/// Whether a schedule adds the column of `field` up unless its format says otherwise.
pub fn totaled_by_default(field: ScheduleField) -> bool {
    summable(field)
}

/// Whether a field's values add up in the totals row.
fn summable(field: ScheduleField) -> bool {
    matches!(
        field,
        ScheduleField::Length
            | ScheduleField::CutLength
            | ScheduleField::BarCount
            | ScheduleField::PlateMass
            | ScheduleField::BoltCount
            | ScheduleField::Weight
            | ScheduleField::Power
            | ScheduleField::DrainUnits
            | ScheduleField::Area
            | ScheduleField::Volume
            | ScheduleField::CutVolume
            | ScheduleField::FillVolume
            | ScheduleField::Cost
            | ScheduleField::PileCount
            | ScheduleField::Count
    )
}

/// Net side area of a wall: length x height (the area of its elevation
/// profile, see [`cc_core::wall_profile`]) minus its openings (with a
/// profile, those that lie inside it: the others cut nothing).
fn wall_area(project: &Project, e: &Element, w: &cc_core::Wall, cache: &mut Memo) -> f64 {
    // Every wall's doors and windows, in element order, found in one pass.
    let hosted = cache.hosted.get_or_insert_with(|| {
        let mut by_host: std::collections::HashMap<cc_core::ElementId, Vec<cc_core::ElementId>> = Default::default();
        for h in project.elements.values() {
            if let Some(x) = h.as_hosted() {
                by_host.entry(x.host).or_default().push(h.id);
            }
        }
        by_host
    });
    let chains = w.profile.as_ref().filter(|_| w.bulge == 0.0).and_then(|p| Some((p.area(), p.chains()?)));
    let openings: f64 = hosted
        .get(&e.id)
        .into_iter()
        .flatten()
        .filter_map(|h| project.element(*h))
        // Niches do not open the wall.
        .filter(|h| !matches!(h.kind, ElementKind::Niche(_)))
        .filter_map(|h| Some((h.as_hosted()?, project.element_type(h.type_id)?.opening_size()?)))
        .filter(|(x, (width, height))| {
            chains.as_ref().is_none_or(|(_, c)| {
                c.holds(x.offset - width * 0.5, x.offset + width * 0.5, x.sill_height, x.sill_height + height)
            })
        })
        .map(|(_, (w, h))| w * h)
        .sum();
    let gross = match &chains {
        Some((area, _)) => *area,
        None => project.wall_length(e, w) * project.wall_height(e, w),
    };
    (gross - openings).max(0.0)
}

/// A section as text: its size, with the profile's letter (I, U, L), a box
/// for tubes and Ø for round sections.
fn section_text(section: cc_core::ColumnSection) -> String {
    use cc_core::{ColumnSection as C, SectionShape as S};
    // Lengths in metric projects, inches in imperial ones (`8" × 8"`).
    let l = |v: f64| format::size(v, format::MetricSize::Length);
    match section {
        C::Rectangular { width, depth } | C::Profile { width, depth, shape: S::Solid } => {
            format!("{} × {}", l(width), l(depth))
        }
        C::Round { diameter } => format!("Ø {}", l(diameter)),
        C::Pipe { diameter, wall } => format!("Ø {} × {}", l(diameter), l(wall)),
        C::Profile { width, depth, shape } => {
            let shape = shape.fitted(width, depth);
            let (letter, thickness) = match shape {
                S::I { web, flange } => ("I", format!("{} / {}", l(web), l(flange))),
                S::Channel { web, flange } => ("U", format!("{} / {}", l(web), l(flange))),
                S::Angle { thickness } => ("L", l(thickness)),
                S::Tube { wall } => ("□", l(wall)),
                S::Solid => ("", String::new()),
            };
            format!("{letter} {} × {} × {thickness}", l(width), l(depth))
        }
    }
}

/// A profile's section as a schedule shows it: the legacy wording for the
/// shapes legacy sections had (`Ø 300`, `I 100 × 200 × 5.6 / 8.5`), the
/// profile's name otherwise.
fn profile_text(def: &cc_core::structural_profile::ProfileDef) -> String {
    use cc_core::structural_profile::ProfileShape as P;
    let l = |v: f64| format::size(v, format::MetricSize::Length);
    match def.shape {
        P::Rect { b, h, .. } => format!("{} × {}", l(b), l(h)),
        P::Round { d } => format!("Ø {}", l(d)),
        P::RoundTube { d, t } => format!("Ø {} × {}", l(d), l(t)),
        P::I { d, bf, tw, tf, .. } => format!("I {} × {} × {} / {}", l(bf), l(d), l(tw), l(tf)),
        P::Channel { d, bf, tw, tf, .. } => format!("U {} × {} × {} / {}", l(bf), l(d), l(tw), l(tf)),
        P::Angle { h, b, t, .. } => format!("L {} × {} × {}", l(b), l(h), l(t)),
        P::RectTube { h, b, t, .. } => format!("□ {} × {} × {}", l(b), l(h), l(t)),
        _ => def.name.clone(),
    }
}

/// A pile's section as text: Ø and the diameter, or the square's sides.
fn pile_section_text(section: &cc_core::pile::PileSection) -> String {
    match *section {
        cc_core::pile::PileSection::Round { diameter } => format!("Ø {}", format::length(diameter)),
        cc_core::pile::PileSection::Square { side } => format!("{} × {}", format::length(side), format::length(side)),
    }
}

/// The pipe a valve sits on (`None` for other elements).
fn valve_pipe<'a>(project: &'a Project, e: &Element) -> Option<&'a Element> {
    match &e.kind {
        ElementKind::Valve(v) => project.valve_host(v).map(|(pipe, _)| pipe),
        _ => None,
    }
}

/// A component type, or one element of it when it has overrides.
type ExtentKey = (cc_core::ElementTypeId, Option<cc_core::ElementId>);

/// What one schedule works out once and reads for many cells: components'
/// extents, per type for elements without overrides (they resolve alike),
/// else per element; and the doors and windows of each wall.
#[derive(Default)]
struct Memo {
    extents: std::collections::HashMap<ExtentKey, Option<(f64, f64, f64)>>,
    hosted: Option<std::collections::HashMap<cc_core::ElementId, Vec<cc_core::ElementId>>>,
}

/// [`component_extents`] through `cache`.
fn cached_extents(project: &Project, e: &Element, cache: &mut Memo) -> Option<(f64, f64, f64)> {
    let ElementKind::Component(c) = &e.kind else { return None };
    let key = (e.type_id, (!c.overrides.is_empty()).then_some(e.id));
    *cache.extents.entry(key).or_insert_with(|| component_extents(project, e))
}

/// Width (X), depth (Y) and height of a component's pieces in its own frame.
fn component_extents(project: &Project, e: &Element) -> Option<(f64, f64, f64)> {
    let ElementKind::Component(c) = &e.kind else { return None };
    let Some(TypeData::Component { def, .. }) = project.element_type(e.type_id).map(|t| &t.data) else { return None };
    let (parts, _) = def.resolve(&c.overrides);
    let mut lo = glam::DVec3::splat(f64::INFINITY);
    let mut hi = glam::DVec3::splat(f64::NEG_INFINITY);
    for p in parts.iter().filter(|p| !p.void && !p.plan_only) {
        for q in &p.footprint {
            lo = lo.min(q.extend(p.z0));
            hi = hi.max(q.extend(p.z1));
        }
    }
    let size = hi - lo;
    size.is_finite().then_some((size.x, size.y, size.z))
}

/// A field's value for `e` written as a schedule cell writes it, keys
/// translated by `label`; empty when the element has none.
pub fn cell_text(
    project: &Project,
    geom: &ModelGeometry,
    e: &Element,
    field: ScheduleField,
    label: &dyn Fn(&str) -> String,
) -> String {
    match value(project, geom, e, field).cell() {
        Cell::Text(s) => s,
        Cell::Key(k) => label(k),
    }
}

fn value(project: &Project, geom: &ModelGeometry, e: &Element, field: ScheduleField) -> Value {
    value_cached(project, geom, e, field, &mut Memo::default())
}

/// [`value`], with the extents of components kept in `cache`.
fn value_cached(project: &Project, geom: &ModelGeometry, e: &Element, field: ScheduleField, cache: &mut Memo) -> Value {
    use ScheduleField as F;
    let ty = project.element_type(e.type_id);
    let info = ty.map(|t| &t.info);
    let text = |s: &str| if s.is_empty() { Value::Missing } else { Value::Text(s.to_string()) };
    let room_boundary = geom.rooms.get(&e.id);
    match field {
        F::Mark => text(&e.mark),
        F::Count => Value::Count(1),
        F::Type => text(project.type_name(e.type_id)),
        F::Level => text(project.level_name(e.level)),
        F::Comments => text(&e.comments),
        F::PhaseCreated if e.category().is_phased() => {
            project.phases.get(project.created_index(e)).map_or(Value::Missing, |ph| text(&ph.name))
        }
        F::PhaseDemolished => {
            e.phase_demolished.and_then(|d| project.phase(d)).map_or(Value::Missing, |ph| text(&ph.name))
        }
        F::PhaseCreated => Value::Missing,
        F::TypeMark => info.map_or(Value::Missing, |i| text(&i.type_mark)),
        F::Description => info.map_or(Value::Missing, |i| text(&i.description)),
        F::Manufacturer => info.map_or(Value::Missing, |i| text(&i.manufacturer)),
        F::Model => info.map_or(Value::Missing, |i| text(&i.model)),
        F::FireRating => info.map_or(Value::Missing, |i| text(&i.fire_rating)),
        F::Cost => info.and_then(|i| i.cost).map_or(Value::Missing, Value::Money),
        F::Length => match &e.kind {
            ElementKind::Wall(w) => Value::Length(project.wall_length(e, w)),
            ElementKind::Beam(b) => Value::Length(b.true_length()),
            ElementKind::Rebar(_) | ElementKind::ShapedRebar(_) => {
                geom.rebar.get(&e.id).map_or(Value::Missing, |r| Value::Length(r.total_length()))
            }
            ElementKind::Foundation(f) => match &f.shape {
                shape @ cc_core::FoundationShape::Strip { .. } => Value::Length(shape.strip_length()),
                _ => Value::Missing,
            },
            ElementKind::Railing(r) => Value::Length(cc_geom::railing_length(project, e, r)),
            ElementKind::Pipe(s) | ElementKind::Conduit(s) | ElementKind::CableTray(s) => Value::Length(s.length()),
            ElementKind::LightningConductor(c) => Value::Length(c.run.length()),
            ElementKind::Sweep(sw) => Value::Length(sw.length()),
            ElementKind::Pile(p) => Value::Length(p.length()),
            _ => Value::Missing,
        },
        F::Height => match (&e.kind, ty.map(|t| &t.data)) {
            (ElementKind::Wall(w), _) => Value::Length(project.wall_height(e, w)),
            (ElementKind::Column(c), _) => {
                let (base, top) = project.column_span(e, c);
                Value::Length(top - base)
            }
            (ElementKind::Room(r), _) => Value::Length(r.height),
            (ElementKind::Mass(m), _) => Value::Length(m.height),
            (ElementKind::Shaft(s), _) => {
                let (base, top) = project.shaft_span(e, s);
                Value::Length(top - base)
            }
            (ElementKind::Door(_) | ElementKind::Window(_) | ElementKind::Niche(_), _) => {
                ty.and_then(|t| t.opening_size()).map_or(Value::Missing, |(_, h)| Value::Length(h))
            }
            (_, Some(TypeData::Railing { height, .. })) => Value::Length(*height),
            (_, Some(TypeData::CableTray { height, .. })) => Value::Size(*height),
            (ElementKind::Component(_), _) => {
                cached_extents(project, e, cache).map_or(Value::Missing, |s| Value::Length(s.2))
            }
            _ => Value::Missing,
        },
        F::Width => match ty.map(|t| &t.data) {
            _ if matches!(e.kind, ElementKind::Door(_) | ElementKind::Window(_) | ElementKind::Niche(_)) => {
                ty.and_then(|t| t.opening_size()).map_or(Value::Missing, |(w, _)| Value::Length(w))
            }
            Some(TypeData::Stair { width, .. } | TypeData::Ramp { width, .. }) => Value::Length(*width),
            Some(TypeData::CableTray { width, .. }) => Value::Size(*width),
            _ => cached_extents(project, e, cache).map_or(Value::Missing, |s| Value::Length(s.0)),
        },
        F::Depth => cached_extents(project, e, cache).map_or(Value::Missing, |s| Value::Length(s.1)),
        F::ComponentName => match ty.map(|t| &t.data) {
            Some(TypeData::Component { component, .. }) => text(component),
            _ => Value::Missing,
        },
        // Doors and windows have their category instead.
        F::ComponentKind => match ty.map(|t| &t.data) {
            Some(TypeData::Component { def, .. }) if e.category() == cc_core::Category::Component => {
                Value::Key(def.kind.key())
            }
            _ => Value::Missing,
        },
        F::Thickness => match ty {
            Some(t) if t.thickness() > 0.0 => Value::Size(t.thickness()),
            _ => Value::Missing,
        },
        F::Area => match &e.kind {
            ElementKind::Wall(w) => Value::Area(wall_area(project, e, w, cache)),
            ElementKind::Room(_) => room_boundary.map_or(Value::Missing, |b| Value::Area(polygon::area(b))),
            ElementKind::Foundation(_) => geom.foundations.get(&e.id).map_or(Value::Missing, |g| Value::Area(g.area())),
            // A joined roof counts the part it keeps (within its walls and its partners').
            ElementKind::Roof(r) => {
                Value::Area(geom.roof_joins.get(&e.id).filter(|j| j.active()).map_or_else(|| r.area(), |j| j.area))
            }
            ElementKind::Floor(f) => Value::Area(f.area()),
            ElementKind::Shaft(s) => Value::Area(polygon::area(&s.outline)),
            ElementKind::Pad(p) => Value::Area(polygon::area(&p.outline)),
            ElementKind::Topography(_) => geom.topographies.get(&e.id).map_or(Value::Missing, |g| Value::Area(g.area)),
            _ => e.sketch().map_or(Value::Missing, |s| Value::Area(polygon::area(s))),
        },
        F::Perimeter => match &e.kind {
            ElementKind::Room(_) => room_boundary.map_or(Value::Missing, |b| Value::Length(polygon::perimeter(b))),
            _ => e.sketch().map_or(Value::Missing, |s| Value::Length(polygon::perimeter(s))),
        },
        F::Volume => match &e.kind {
            ElementKind::Room(r) => {
                room_boundary.map_or(Value::Missing, |b| Value::Volume(polygon::area(b) * r.height))
            }
            _ => Value::Volume(geom.volume(e.id)),
        },
        F::Sill => e.as_hosted().map_or(Value::Missing, |h| Value::Length(h.sill_height)),
        F::Partition => e.rebar_partition().map_or(Value::Missing, text),
        F::Host => e
            .as_hosted()
            .map(|h| h.host)
            .or(match &e.kind {
                ElementKind::Rebar(r) => Some(r.host),
                ElementKind::Valve(v) => Some(v.host),
                ElementKind::Connection(c) => Some(c.member),
                ElementKind::Pile(p) => p.cap.map(|c| c.cap),
                _ => None,
            })
            .and_then(|h| project.element(h))
            .map_or(Value::Missing, |w| text(&if w.mark.is_empty() { w.id.to_string() } else { w.mark.clone() })),
        F::Function => match &e.kind {
            ElementKind::LightningConductor(c) => Value::Key(c.role.key()),
            _ => ty.and_then(|t| t.wall_function()).map_or(Value::Missing, |f| Value::Key(f.key())),
        },
        F::CableLength => project.outlet_cable(e).map_or(Value::Missing, |(n, l)| Value::Length(l * n as f64)),
        F::Structural => match &e.kind {
            ElementKind::Wall(w) => Value::Key(if w.structural { "value-yes" } else { "value-no" }),
            ElementKind::Column(c) => Value::Key(if c.structural { "value-yes" } else { "value-no" }),
            _ => Value::Missing,
        },
        F::Section => match ty.map(|t| &t.data) {
            // A profile-made type's legacy fields are only its box.
            Some(data) if project.type_profile_def(data).is_some() => {
                project.type_profile_def(data).map_or(Value::Missing, |def| Value::Text(profile_text(def)))
            }
            Some(TypeData::Column { section, .. }) => Value::Text(section_text(*section)),
            Some(TypeData::Beam { width, height, shape, .. }) => {
                let section = cc_core::ColumnSection::Profile { width: *width, depth: *height, shape: *shape };
                Value::Text(section_text(section))
            }
            Some(TypeData::Foundation { kind: cc_core::FoundationKind::Pad, width, length, .. }) => {
                Value::Text(format!("{} × {}", format::length(*width), format::length(*length)))
            }
            Some(TypeData::Foundation { kind: cc_core::FoundationKind::Strip, width, .. }) => {
                Value::Text(format::length(*width))
            }
            Some(data @ TypeData::Foundation { kind: cc_core::FoundationKind::PileCap, .. }) => {
                match cc_core::pile::cap_of_type(data).map(|(cap, _)| cap.outline()) {
                    Some(cc_core::pile::CapOutline::Disk { radius }) => {
                        Value::Text(format!("Ø {}", format::length(2.0 * radius)))
                    }
                    Some(cc_core::pile::CapOutline::Ring { inner, outer }) => {
                        Value::Text(format!("Ø {} / {}", format::length(2.0 * outer), format::length(2.0 * inner)))
                    }
                    Some(outline) => {
                        let (lo, hi) = outline.bounds();
                        Value::Text(format!("{} × {}", format::length(hi.x - lo.x), format::length(hi.y - lo.y)))
                    }
                    None => Value::Missing,
                }
            }
            Some(TypeData::Pile { section, .. }) => Value::Text(pile_section_text(section)),
            _ => Value::Missing,
        },
        F::Profile => ty
            .and_then(|t| match &t.data {
                TypeData::Column { profile, .. } | TypeData::Beam { profile, .. } => profile.as_ref(),
                _ => None,
            })
            .and_then(|id| project.profile(*id))
            .map_or(Value::Missing, |p| Value::Text(p.name.clone())),
        F::LinearMass => ty.map_or(Value::Missing, |t| linear_mass(project, &t.data)),
        F::CutLength => match e.kind {
            ElementKind::Pipe(_) => geom.pipe_cut_length(e.id).map_or(Value::Missing, Value::Length),
            _ => geom.beams.get(&e.id).map_or(Value::Missing, |b| Value::Length(b.cut_length())),
        },
        F::InsideDiameter => match ty.filter(|_| matches!(e.kind, ElementKind::Pipe(_))) {
            Some(t) => t.inside_diameter(project).map_or(Value::Missing, |d| {
                Value::Measured(format!("Ø {}", format::size(d, format::MetricSize::Mm(1))), d)
            }),
            None => Value::Missing,
        },
        F::Series | F::Size => match ty.filter(|_| matches!(e.kind, ElementKind::Pipe(_))) {
            Some(t) => project.pipe_row(&t.data).map_or(Value::Missing, |(series, row)| {
                Value::Text(if field == F::Series { series.name.clone() } else { row.size.clone() })
            }),
            None => Value::Missing,
        },
        F::Material => ty.map_or(Value::Missing, |t| text(t.main_material())),
        F::Diameter => {
            match valve_pipe(project, e).map_or(ty, |pipe| project.element_type(pipe.type_id)).map(|t| &t.data) {
                Some(TypeData::Rebar { diameter, .. } | TypeData::LightningConductor { diameter, .. }) => {
                    let text = format!("Ø {}", format::bar_size(*diameter));
                    Value::Measured(text, *diameter)
                }
                Some(TypeData::Pipe { diameter, .. } | TypeData::Conduit { diameter, .. }) => {
                    Value::Measured(format!("Ø {}", format::size(*diameter, format::MetricSize::Mm(0))), *diameter)
                }
                // A pile's diameter, or a square pile's side.
                Some(TypeData::Pile { section, .. }) => Value::Length(section.size()),
                _ => Value::Missing,
            }
        }
        F::CutOff | F::Toe => match &e.kind {
            ElementKind::Pile(p) => {
                let offset = if field == F::CutOff { p.cut_off } else { p.toe };
                Value::Length(project.level_elevation(e.level) + offset)
            }
            _ => Value::Missing,
        },
        F::PileKind => match ty.map(|t| &t.data) {
            Some(TypeData::Pile { kind, .. }) => Value::Key(kind.key()),
            _ => Value::Missing,
        },
        F::PileCount => match &e.kind {
            ElementKind::Foundation(_) if project.pile_cap(e).is_some() => Value::Count(project.piles_of(e.id).count()),
            _ => Value::Missing,
        },
        F::System => {
            // Valves are in their pipe's system.
            let systems = valve_pipe(project, e).unwrap_or(e).systems();
            let labels: Vec<&str> = systems.iter().filter_map(|id| project.system(*id)).map(|s| s.label()).collect();
            if labels.is_empty() { Value::Missing } else { Value::Text(labels.join(", ")) }
        }
        F::Slope => match &e.kind {
            ElementKind::Pipe(s) if !s.is_vertical() => slope(s.slope()),
            ElementKind::Roof(_) => {
                geom.roof_framing.get(&e.id).and_then(|g| g.slopes).map_or(Value::Missing, |(lo, _)| slope(lo))
            }
            ElementKind::Ramp(r) => cc_geom::ramp_geom(project, e, r).map_or(Value::Missing, |g| slope(g.check.slope)),
            _ => Value::Missing,
        },
        F::Power => match project.mep_data(e) {
            Some(m @ cc_core::MepData::Device { .. }) => Value::Power(m.power()),
            _ => Value::Missing,
        },
        F::DrainUnits => match project.mep_data(e) {
            Some(cc_core::MepData::Fixture { drain_units, .. }) => Value::Number(drain_units),
            _ => Value::Missing,
        },
        F::BarCount => geom.rebar.get(&e.id).map_or(Value::Missing, |r| Value::Count(r.count())),
        F::FloorArea => geom.masses.get(&e.id).map_or(Value::Missing, |m| Value::Area(m.floor_area())),
        F::FloorCount => geom.masses.get(&e.id).map_or(Value::Missing, |m| Value::Count(m.floors.len())),
        F::BarLength => geom.rebar.get(&e.id).map_or(Value::Missing, |r| Value::Length(r.bar_length())),
        F::Weight => match (geom.rebar.get(&e.id), roof_covering(project, geom, e)) {
            (Some(r), _) => Value::Weight(r.weight()),
            (None, Some((spec, g))) => Value::Weight(spec.weight_on(g.sloped_area)),
            _ => Value::Missing,
        },
        F::TileCount => roof_covering(project, geom, e)
            .and_then(|(spec, g)| spec.tiles(g.sloped_area).map(|n| Value::Count(n as usize)))
            .unwrap_or(Value::Missing),
        F::FramingLength => geom
            .roof_framing
            .get(&e.id)
            .filter(|g| !g.members.is_empty())
            .map_or(Value::Missing, |g| Value::Length(g.total_length())),
        F::FramingVolume => geom
            .roof_framing
            .get(&e.id)
            .filter(|g| !g.members.is_empty())
            .map_or(Value::Missing, |g| Value::Volume(g.volume())),
        F::PanelCount | F::GlazedArea | F::MullionLength => {
            match geom.walls.get(&e.id).and_then(|g| g.curtain.as_ref()) {
                Some(c) if field == F::PanelCount => Value::Count(c.layout.panel_count()),
                Some(c) if field == F::GlazedArea => Value::Area(c.layout.glazed_area()),
                Some(c) => Value::Length(c.layout.mullion_length()),
                None => Value::Missing,
            }
        }
        F::Elevation => match &e.kind {
            ElementKind::Pad(p) => Value::Length(p.elevation),
            _ => Value::Missing,
        },
        F::CutVolume | F::FillVolume => {
            let pick = |g: &cc_geom::pad::PadGeom| if field == F::CutVolume { g.cut } else { g.fill };
            match &e.kind {
                ElementKind::Pad(_) => geom.pads.get(&e.id).map_or(Value::Missing, |g| Value::Volume(pick(g))),
                ElementKind::Topography(_) => {
                    let mut pads = project.pads_of(e.id).filter_map(|(p, _)| geom.pads.get(&p.id)).peekable();
                    if pads.peek().is_some() { Value::Volume(pads.map(pick).sum()) } else { Value::Missing }
                }
                _ => Value::Missing,
            }
        }
        F::ConnectionKind => match &e.kind {
            ElementKind::Connection(c) => Value::Key(c.kind.key()),
            _ => Value::Missing,
        },
        F::PlateMass => geom.connections.get(&e.id).map_or(Value::Missing, |g| Value::Weight(g.plate_mass())),
        F::BoltCount => geom.connections.get(&e.id).map_or(Value::Missing, |g| Value::Count(g.bolt_count())),
        F::BoltDiameter => geom.connections.get(&e.id).map_or(Value::Missing, |g| {
            let text = format!("Ø {}", format::size(g.bolt_diameter, format::MetricSize::Mm(0)));
            Value::Measured(text, g.bolt_diameter)
        }),
        F::Number => match &e.kind {
            ElementKind::Room(r) => text(&r.number),
            _ => Value::Missing,
        },
        F::Name => match &e.kind {
            ElementKind::Room(r) => text(&r.name),
            _ => Value::Missing,
        },
        F::Risers | F::RiserHeight | F::Run => match &e.kind {
            ElementKind::Stair(s) => match cc_geom::stair_geom(project, e, s) {
                Some(g) if field == F::Risers => Value::Count(g.risers),
                Some(g) if field == F::RiserHeight => Value::Size(g.riser),
                Some(g) => Value::Length(g.run()),
                None => Value::Missing,
            },
            ElementKind::Ramp(r) if field == F::Run => {
                cc_geom::ramp_geom(project, e, r).map_or(Value::Missing, |g| Value::Length(g.run()))
            }
            _ => Value::Missing,
        },
    }
}

/// A slope as schedules write it, filtered and sorted by its rise over run.
fn slope(value: f64) -> Value {
    if value.is_finite() { Value::Measured(format::slope(value), value) } else { Value::Missing }
}

/// A field's value for view filters: its text (keys reduced to their last
/// word, e.g. `yes`, `exterior`) and, for quantities, its number in SI
/// units (meters, square and cubic meters; yes/no as 1/0; diameters in
/// meters; slopes as rise over run). `None` when the element has no value.
pub fn field_value(
    project: &Project,
    geom: &ModelGeometry,
    e: &Element,
    field: ScheduleField,
) -> Option<(String, Option<f64>)> {
    let v = value(project, geom, e, field);
    match v {
        Value::Missing => None,
        Value::Key(k) => {
            let word = k.rsplit('-').next().unwrap_or(k).to_string();
            let number = match word.as_str() {
                "yes" => Some(1.0),
                "no" => Some(0.0),
                _ => None,
            };
            Some((word, number))
        }
        Value::Text(t) => {
            let number = t.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite());
            Some((t, number))
        }
        other => Some((text_of(&other), other.number())),
    }
}

/// Default row order: by mark (rooms by number), naturally, then by id.
fn default_order(a: &Element, b: &Element) -> Ordering {
    let key = |e: &Element| match &e.kind {
        ElementKind::Room(r) => r.number.clone(),
        _ => e.mark.clone(),
    };
    natural(&key(a), &key(b)).then(a.id.cmp(&b.id))
}

pub fn schedule(project: &Project, geom: &ModelGeometry, category: Category, def: &ScheduleDef) -> Table {
    schedule_where(project, geom, category, def, &mut |_| true)
}

/// A schedule of the elements `keep` accepts (those a schedule view's
/// phase and phase filter show, see [`crate::graphics::PhaseView`]).
pub fn schedule_where(
    project: &Project,
    geom: &ModelGeometry,
    category: Category,
    def: &ScheduleDef,
    keep: &mut dyn FnMut(&Element) -> bool,
) -> Table {
    if let (Category::Rebar, Some(options)) = (category, &def.bending) {
        return crate::bending::table(project, geom, def, options, keep);
    }
    let fields = def.fields_for(category);
    let mut elements: Vec<&Element> = project.elements_of(category).filter(|e| keep(e)).collect();
    elements.sort_by(|a, b| default_order(a, b));

    let mut cache = Memo::default();
    let mut rows: Vec<(&Element, Vec<Value>)> = elements
        .into_iter()
        .map(|e| (e, fields.iter().map(|&f| value_cached(project, geom, e, f, &mut cache)).collect()))
        .collect();
    // A field's value in a row: the column's when the field is shown.
    let mut field_of = |e: &Element, values: &[Value], field: ScheduleField| match fields
        .iter()
        .position(|f| *f == field)
        .and_then(|i| values.get(i))
    {
        Some(v) => v.clone(),
        None => value_cached(project, geom, e, field, &mut cache),
    };

    // Filters: a row stays when it passes every one.
    for filter in def.all_filters() {
        rows.retain(|(e, values)| {
            let value = field_of(e, values, filter.field);
            let text = text_of(&value);
            let actual = (!matches!(value, Value::Missing)).then(|| (text.as_str(), value.number()));
            filter.op.test(actual, &filter.value)
        });
    }

    // Sorting: each row's keys once; stable, so ties keep the default order. Rows gathered with no
    // level of sorting are those alike in every column shown but the counted and totaled ones.
    let mut levels = def.all_sorting();
    if def.collapse && levels.is_empty() {
        levels = fields
            .iter()
            .filter(|f| {
                let format = def.format_of(**f);
                **f != ScheduleField::Count && !format.hidden && !format.total.unwrap_or_else(|| summable(**f))
            })
            .map(|f| cc_core::SortLevel::new(*f))
            .collect();
    }
    let mut keyed: Vec<Keyed> = rows
        .into_iter()
        .map(|(e, values)| {
            let at: Vec<Value> = levels.iter().map(|l| field_of(e, &values, l.field)).collect();
            Keyed {
                keys: at.iter().map(SortKey::of).collect(),
                labels: at.iter().map(Value::cell).collect(),
                e,
                values,
            }
        })
        .collect();
    let order = |a: &[SortKey], b: &[SortKey]| {
        for (level, (x, y)) in levels.iter().zip(a.iter().zip(b)) {
            let o = x.compare(y);
            if o != Ordering::Equal {
                return if level.descending { o.reverse() } else { o };
            }
        }
        Ordering::Equal
    };
    if !levels.is_empty() {
        keyed.sort_by(|a, b| order(&a.keys, &b.keys));
    }

    // The columns: the fields shown, less the hidden ones (all of them when every one is hidden).
    let formats: Vec<cc_core::ColumnFormat> = fields.iter().map(|f| def.format_of(*f)).collect();
    let mut shown: Vec<usize> = (0..fields.len()).filter(|i| formats.get(*i).is_none_or(|f| !f.hidden)).collect();
    if shown.is_empty() {
        shown = (0..fields.len()).collect();
    }
    let totaled = |i: usize| match (fields.get(i), formats.get(i)) {
        (Some(field), Some(format)) => format.total.unwrap_or_else(|| summable(*field)),
        _ => false,
    };
    let total_text = |i: usize, sum: f64| fields.get(i).map_or_else(String::new, |f| format_total(*f, sum));

    // The rows: one per element, or one per run of elements alike in every sorting level.
    let mut items: Vec<Item> = Vec::new();
    for k in keyed {
        let sums: Vec<f64> = k.values.iter().map(|v| v.number().unwrap_or(0.0)).collect();
        match items.last_mut().filter(|last| def.collapse && order(&last.keys, &k.keys) == Ordering::Equal) {
            Some(last) => {
                last.count += 1;
                for (i, value) in k.values.iter().enumerate() {
                    if let Some(sum) = last.sums.get_mut(i) {
                        *sum += value.number().unwrap_or(0.0);
                    }
                    // Values that differ among the row's elements read as varying.
                    if let Some(cell) = last.cells.get_mut(i)
                        && *cell != value.cell()
                    {
                        *cell = Cell::Key("schedule-varies");
                    }
                }
            }
            None => items.push(Item {
                element: k.e.id,
                keys: k.keys,
                labels: k.labels,
                count: 1,
                sums,
                cells: k.values.iter().map(Value::cell).collect(),
            }),
        }
    }
    let columns = shown.len();
    let cells_of = |item: &Item| -> Vec<Cell> {
        shown
            .iter()
            .map(|&i| {
                let cell = item.cells.get(i).cloned().unwrap_or_else(|| Cell::Text(String::new()));
                let sum = item.sums.get(i).copied().unwrap_or(0.0);
                match fields.get(i) {
                    // A row of several elements counts them, and adds what is totaled.
                    Some(ScheduleField::Count) => Cell::Text(item.count.to_string()),
                    Some(_) if item.count > 1 && totaled(i) => Cell::Text(total_text(i, sum)),
                    _ => cell,
                }
            })
            .collect()
    };
    // The count of a footer or the totals row goes in the first cell, unless
    // that column is totaled: its total is not written over (#434).
    let count_first = !shown.first().is_some_and(|&i| totaled(i));
    // A footer: the count of its elements first, then the totals of the totaled columns.
    let footer_of = |element: cc_core::ElementId, count: usize, sums: &[f64]| {
        let mut cells: Vec<Cell> = shown
            .iter()
            .map(|&i| {
                Cell::Text(if totaled(i) { total_text(i, sums.get(i).copied().unwrap_or(0.0)) } else { String::new() })
            })
            .collect();
        if let Some(first) = cells.first_mut().filter(|_| count_first) {
            *first = Cell::Text(count.to_string());
        }
        Row { footer: true, ..Row::new(element, cells) }
    };

    // Rows in order, with the headings, footers and blank lines of the levels that group.
    let mut out: Vec<Row> = Vec::new();
    let mut open: Vec<Group> = levels.iter().map(|_| Group::default()).collect();
    let close = |out: &mut Vec<Row>, open: &mut [Group], from: usize| {
        let mut blank = false;
        for (level, group) in levels.iter().zip(open.iter_mut()).enumerate().skip(from).map(|(_, x)| x).rev() {
            if level.footer && group.count > 0 {
                out.push(footer_of(group.element, group.count, &group.sums));
            }
            blank |= level.blank_line && group.count > 0;
            *group = Group::default();
        }
        blank
    };
    let mut previous: Option<&Item> = None;
    for item in &items {
        // The outermost level this row starts a new group of.
        let from = match previous {
            None => 0,
            Some(p) => (0..levels.len())
                .find(|l| match (p.keys.get(*l), item.keys.get(*l)) {
                    (Some(a), Some(b)) => a.compare(b) != Ordering::Equal,
                    _ => true,
                })
                .unwrap_or(levels.len()),
        };
        if previous.is_some() && close(&mut out, &mut open, from) {
            out.push(Row::heading(item.element, Cell::Text(String::new()), columns));
        }
        for (l, level) in levels.iter().enumerate().skip(from) {
            if level.header {
                let label = item.labels.get(l).cloned().unwrap_or_else(|| Cell::Text(String::new()));
                out.push(Row::heading(item.element, label, columns));
            }
        }
        for group in open.iter_mut() {
            if group.count == 0 {
                group.element = item.element;
                group.sums = vec![0.0; fields.len()];
            }
            group.count += item.count;
            for (sum, value) in group.sums.iter_mut().zip(&item.sums) {
                *sum += value;
            }
        }
        out.push(Row::new(item.element, cells_of(item)));
        previous = Some(item);
    }
    close(&mut out, &mut open, 0);

    let totals = if def.hide_totals {
        Vec::new()
    } else {
        let mut totals: Vec<String> = shown
            .iter()
            .map(|&i| {
                if !totaled(i) {
                    return String::new();
                }
                total_text(i, items.iter().filter_map(|item| item.sums.get(i)).sum())
            })
            .collect();
        if let Some(first) = totals.first_mut().filter(|_| count_first) {
            *first = items.iter().map(|item| item.count).sum::<usize>().to_string();
        }
        totals
    };

    let system = project.measurement_system();
    Table {
        headers: shown.iter().filter_map(|&i| fields.get(i)).map(|f| f.key_in(system)).collect(),
        rows: out,
        totals,
        summary: None,
        caption: None,
        titles: shown
            .iter()
            .map(|&i| formats.get(i).and_then(|f| f.heading.clone()).filter(|h| !h.trim().is_empty()))
            .collect(),
        align: shown.iter().map(|&i| formats.get(i).map(|f| f.align).unwrap_or_default()).collect(),
        appearance: def.appearance,
    }
}

/// A row before it is laid out: its element, its values and what it is sorted by.
struct Keyed<'a> {
    e: &'a Element,
    values: Vec<Value>,
    keys: Vec<SortKey>,
    /// The value of each sorting level, as a heading writes it.
    labels: Vec<Cell>,
}

/// A row of the table: one element, or the elements alike in every sorting level.
struct Item {
    element: cc_core::ElementId,
    keys: Vec<SortKey>,
    labels: Vec<Cell>,
    count: usize,
    /// Per field, its elements' numbers added up.
    sums: Vec<f64>,
    /// Per field, the value all its elements share (else the varying mark).
    cells: Vec<Cell>,
}

/// An open group of a sorting level: its first element, how many it holds
/// and their numbers added up per field.
struct Group {
    element: cc_core::ElementId,
    count: usize,
    sums: Vec<f64>,
}

impl Default for Group {
    fn default() -> Self {
        Self { element: cc_core::ElementId(0), count: 0, sums: Vec::new() }
    }
}

/// A total of `field` as the totals row writes it.
fn format_total(field: ScheduleField, sum: f64) -> String {
    match field {
        ScheduleField::Length | ScheduleField::CutLength => format::meters(sum),
        ScheduleField::BarCount | ScheduleField::Count => format::number(sum, 0),
        ScheduleField::Weight => format::quantity(format::Quantity::Mass, sum, 2),
        ScheduleField::Volume | ScheduleField::CutVolume | ScheduleField::FillVolume => format::volume(sum),
        ScheduleField::Area => format::area(sum),
        ScheduleField::Cost => format::money(sum),
        _ => format::number(sum, 2),
    }
}

/// The table as CSV: `;`-separated when the decimal separator is a comma, so
/// spreadsheets in those locales open it directly. `header` and `cell`
/// translate keys.
pub fn to_csv(table: &Table, header: impl Fn(&str) -> String, cell: impl Fn(&Cell) -> String) -> String {
    let header = |key: &str| header(key);
    let sep = if format::number(0.5, 1).contains(',') { ';' } else { ',' };
    let quote = |s: &str| {
        if formula_like(s) {
            // A spreadsheet would run it as a formula: kept as text.
            format!("\"'{}\"", s.replace('"', "\"\""))
        } else if s.contains(sep) || s.contains('"') || s.contains('\n') || s.contains('\r') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let mut out = String::new();
    let mut line = |cells: Vec<String>| {
        out.push_str(&cells.iter().map(|c| quote(c)).collect::<Vec<_>>().join(&sep.to_string()));
        out.push_str("\r\n");
    };
    let mut tables = vec![table];
    tables.extend(table.summary.as_deref());
    for (i, t) in tables.into_iter().enumerate() {
        if i > 0 {
            // A table under another: a blank line and its caption first.
            line(Vec::new());
            if let Some(caption) = t.caption {
                line(vec![header(caption)]);
            }
        }
        line(t.header_texts(&header));
        for row in &t.rows {
            line(row.cells.iter().map(&cell).collect());
        }
        if !t.totals.is_empty() {
            line(t.totals.clone());
        }
    }
    out
}

/// Whether a spreadsheet would read the cell `s` as a formula: it starts
/// with `=`, `@`, a tab or a carriage return, or with `+` or `-` and is not
/// a number (with its unit, such as `-3.50 m`).
fn formula_like(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some('=' | '@' | '\t' | '\r') => true,
        Some('+' | '-') => {
            let rest = chars.as_str();
            let number = rest.starts_with(|c: char| c.is_ascii_digit())
                && rest.chars().all(|c| {
                    c.is_ascii_digit()
                        || c.is_alphabetic()
                        || matches!(c, '.' | ',' | ' ' | '\u{a0}' | '\u{202f}' | '²' | '³' | '°' | '%' | '\'' | '"')
                });
            !number
        }
        _ => false,
    }
}

/// A roof's covering (from its type) and its framing figures.
fn roof_covering<'a>(
    project: &Project,
    geom: &'a ModelGeometry,
    e: &Element,
) -> Option<(cc_core::roofing::TileSpec, &'a cc_geom::roof_framing::RoofFramingGeom)> {
    let Some(TypeData::Roof { covering: Some(kind), .. }) = project.element_type(e.type_id).map(|t| &t.data) else {
        return None;
    };
    Some((kind.spec(), geom.roof_framing.get(&e.id)?))
}

/// Usual text height of a schedule's cells on paper, millimeters (column widths scale from it).
const TEXT: f64 = 2.2;
/// Row height, millimeters.
const ROW: f64 = 6.0;
/// Narrowest and widest column, millimeters.
const MIN_COLUMN: f64 = 14.0;
const MAX_COLUMN: f64 = 90.0;
/// Cell padding, millimeters.
const PAD: f64 = 1.5;
/// Most element rows drawn; the rest are counted in a last row.
pub const MAX_DRAWN_ROWS: usize = 2000;

/// Space between a table and the one under it, millimeters.
const TABLE_GAP: f64 = 6.0;

/// A schedule as a drawing, for sheets: a title row, the headers, one row
/// per element and the totals, ruled like a table, then any table under it
/// (a bending schedule's summary) with its caption. Sizes are paper
/// millimeters at the view's `scale` (as the panel diagrams), so a sheet
/// places it at its printed size; numbers are right-aligned, and `label`
/// translates the headers and the key cells. Group headings run across
/// their row; rows with a bar's sketch are as tall as it needs.
pub fn drawing(table: &Table, title: &str, scale: u32, label: &dyn Fn(&str) -> String) -> crate::Drawing2D {
    let mut d = crate::Drawing2D::default();
    let k = scale.max(1) as f64 / 1000.0;
    let bottom = draw_table(&mut d, table, title, 0.0, k, label);
    if let Some(under) = &table.summary {
        let caption = under.caption.map(label).unwrap_or_default();
        draw_table(&mut d, under, &caption, bottom - TABLE_GAP, k, label);
    }
    d
}

/// A drawn row: its texts, whether it is a heading, its sketch.
type DrawnRow<'a> = (Vec<String>, bool, Option<&'a (usize, crate::bending::BarSketch)>);

/// Draw `table` with its title row from height `top` (paper millimeters)
/// down, as its appearance says; returns where it ends.
fn draw_table(
    d: &mut crate::Drawing2D,
    table: &Table,
    title: &str,
    top: f64,
    k: f64,
    label: &dyn Fn(&str) -> String,
) -> f64 {
    use crate::{Anchor, LineStyle};
    use cc_core::ColumnAlign;
    let look = {
        let mut a = table.appearance;
        a.repair();
        a
    };
    let (text_size, header_size, title_size) = (look.text_size, look.header_size, look.title_size);
    // Rows grow with their text.
    let row_for = |size: f64| ROW.max(size * 2.0);
    let at = |x: f64, y: f64| glam::DVec2::new(x, y) * k;
    let headers: Vec<String> = table.header_texts(label);
    let cell_text = |c: &Cell| match c {
        Cell::Text(t) => t.clone(),
        Cell::Key(key) => label(key),
    };
    let mut rows: Vec<DrawnRow> = table
        .rows
        .iter()
        .take(MAX_DRAWN_ROWS)
        .map(|r| (r.cells.iter().map(&cell_text).collect(), r.heading, r.sketch.as_ref()))
        .collect();
    if table.rows.len() > MAX_DRAWN_ROWS {
        let more = (table.rows.len() - MAX_DRAWN_ROWS).to_string();
        rows.push((vec![format!("… +{more}")], false, None));
    }
    let totals = (!table.totals.is_empty()).then(|| table.totals.clone());
    let columns = headers.len().max(1);
    let sketched: Vec<usize> = rows.iter().filter_map(|(_, _, s)| s.map(|s| s.0)).collect();
    // Each column as wide as its longest text (an estimate from the
    // character count; texts are proportional), within the limits; a
    // column of sketches as wide as they need. Headings run across.
    let widths: Vec<f64> = (0..columns)
        .map(|c| {
            let texts = rows.iter().filter(|(_, heading, sketch)| !heading && sketch.is_none_or(|s| s.0 != c));
            let longest = texts
                .map(|(r, ..)| r.get(c))
                .chain(totals.iter().map(|t| t.get(c)))
                .flatten()
                .map(|t| t.chars().count())
                .max()
                .unwrap_or(0);
            let header = if look.show_headers { headers.get(c).map_or(0, |h| h.chars().count()) } else { 0 };
            let needed = (longest as f64 * text_size).max(header as f64 * header_size) * 0.6 + 2.0 * PAD;
            let width = needed.clamp(MIN_COLUMN, MAX_COLUMN.max(MIN_COLUMN * text_size / TEXT));
            if sketched.contains(&c) { width.max(crate::bending::SKETCH_WIDTH) } else { width }
        })
        .collect();
    let width: f64 = widths.iter().sum();
    // Row heights: the title, the headers, the rows, the totals.
    let lead = usize::from(look.show_title) + usize::from(look.show_headers);
    let mut heights = Vec::new();
    if look.show_title {
        heights.push(row_for(title_size));
    }
    if look.show_headers {
        heights.push(row_for(header_size));
    }
    let body_row = row_for(text_size);
    heights.extend(
        rows.iter().map(|(_, _, s)| if s.is_some() { crate::bending::SKETCH_ROW.max(body_row) } else { body_row }),
    );
    if totals.is_some() {
        heights.push(body_row);
    }
    let mut ys = vec![top];
    for h in &heights {
        ys.push(ys.last().copied().unwrap_or(top) - h);
    }
    let bottom = ys.last().copied().unwrap_or(top);
    let lines = heights.len();

    // The title, centered in its row.
    if look.show_title
        && let (Some(&y0), Some(&y1)) = (ys.first(), ys.get(1))
    {
        d.text_at(at(width / 2.0, (y0 + y1) / 2.0 - title_size / 2.0), title, title_size, Anchor::CenterBottom, 0.0);
    }
    // Rules: the frame, and the lines under the title, the headers and above
    // the totals stronger than those between rows.
    for (r, y) in ys.iter().enumerate() {
        let edge = r == 0 || r == lines;
        let strong = edge || r <= lead || (totals.is_some() && r == lines - 1);
        let drawn = if edge { look.outline } else { look.grid_lines || strong };
        if drawn {
            let style = if strong { LineStyle::Projection } else { LineStyle::Thin };
            d.line(at(0.0, *y), at(width, *y), style, None);
        }
    }
    if look.outline {
        d.line(at(0.0, top), at(0.0, bottom), LineStyle::Projection, None);
        d.line(at(width, top), at(width, bottom), LineStyle::Projection, None);
    }
    // Column rules, row by row (the title and headings run across).
    let heading_at = |r: usize| r >= lead && rows.get(r - lead).is_some_and(|(_, heading, _)| *heading);
    if look.grid_lines {
        for r in usize::from(look.show_title)..lines {
            if heading_at(r) {
                continue;
            }
            let (Some(&y0), Some(&y1)) = (ys.get(r), ys.get(r + 1)) else { continue };
            let mut x = 0.0;
            for w in widths.iter().take(columns - 1) {
                x += w;
                d.line(at(x, y0), at(x, y1), LineStyle::Thin, None);
            }
        }
    }
    // The texts, a row at a time from the headers down.
    let head = look.show_headers.then_some((&headers, false, None, true));
    let body = head
        .into_iter()
        .chain(rows.iter().map(|(r, heading, sketch)| (r, *heading, *sketch, false)))
        .chain(totals.iter().map(|t| (t, false, None, false)));
    let first = usize::from(look.show_title);
    for (r, (row, heading, sketch, is_header)) in body.enumerate() {
        let (Some(&y0), Some(&y1)) = (ys.get(r + first), ys.get(r + first + 1)) else { continue };
        let size = if is_header { header_size } else { text_size };
        let baseline = (y0 + y1) / 2.0 - size / 2.0;
        if heading {
            if let Some(text) = row.first().filter(|t| !t.is_empty()) {
                d.text_at(at(PAD, baseline), text.clone(), size, Anchor::LeftBottom, 0.0);
            }
            continue;
        }
        let mut x = 0.0;
        for (c, text) in row.iter().enumerate() {
            let w = widths.get(c).copied().unwrap_or(MIN_COLUMN);
            if let Some((_, s)) = sketch.filter(|s| s.0 == c) {
                let (lo, hi) = (glam::DVec2::new(x, y1), glam::DVec2::new(x + w, y0));
                crate::bending::draw_sketch(d, s, (lo, hi), k);
            } else if !text.is_empty() {
                // Numbers to the right and texts to the left, unless the column says otherwise.
                let numeric = !is_header && text.trim_start().starts_with(|ch: char| ch.is_ascii_digit() || ch == '-');
                let align = match table.align.get(c).copied().unwrap_or_default() {
                    ColumnAlign::Auto if numeric => ColumnAlign::Right,
                    ColumnAlign::Auto => ColumnAlign::Left,
                    other => other,
                };
                let (px, anchor) = match align {
                    ColumnAlign::Right => (x + w - PAD, Anchor::RightBottom),
                    ColumnAlign::Center => (x + w / 2.0, Anchor::CenterBottom),
                    _ => (x + PAD, Anchor::LeftBottom),
                };
                d.text_at(at(px, baseline), text.clone(), size, anchor, 0.0);
            }
            x += w;
        }
    }
    bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Project, ModelGeometry) {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        (project, geom)
    }

    /// Filters refuse rules on fields that apply to none of their categories,
    /// so every value an element has must be on a field that applies to it.
    #[test]
    fn fields_with_values_apply_to_the_category() {
        let mut fields: std::collections::BTreeSet<ScheduleField> =
            Category::ALL.into_iter().flat_map(ScheduleField::available).collect();
        fields.extend([ScheduleField::Type, ScheduleField::Level, ScheduleField::Material, ScheduleField::Volume]);
        let files = [
            include_str!("../../../examples/casa_simples.ccad"),
            include_str!("../../../examples/showcase.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/features.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/structure.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/mep.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/arcs.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/roofs.ccad"),
            include_str!("../../cc_ifc/tests/fixtures/phases.ccad"),
        ];
        for file in files {
            let project = cc_core::from_ron(file).unwrap();
            let geom = ModelGeometry::build(&project);
            for e in project.elements.values() {
                for &field in &fields {
                    if value(&project, &geom, e, field) != Value::Missing {
                        assert!(field.applies_to(e.category()), "{field:?} has a value on {:?}", e.category());
                    }
                }
            }
        }
    }

    #[test]
    fn sorting_filtering_and_totals() {
        let (project, geom) = sample();
        let def = ScheduleDef {
            fields: vec![ScheduleField::Mark, ScheduleField::Width, ScheduleField::Area],
            sort_by: Some(ScheduleField::Mark),
            descending: true,
            ..Default::default()
        };
        let t = schedule(&project, &geom, Category::Window, &def);
        assert_eq!(t.headers, vec!["col-mark", "col-width", "col-area"]);
        assert_eq!(t.rows.len(), 4);
        // Marks 1..4, descending; windows have no area (empty cell).
        let marks: Vec<&str> = t.rows.iter().map(|r| r.cells[0].raw()).collect();
        assert_eq!(marks, ["4", "3", "2", "1"]);
        assert_eq!(t.rows[0].cells[1], Cell::Text(format::meters(1.2)));
        assert_eq!(t.totals[0], "4");

        let filtered = ScheduleDef { filter: Some((ScheduleField::Mark, "3".into())), ..def };
        let t = schedule(&project, &geom, Category::Window, &filtered);
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.totals[0], "1");
    }

    /// Rows sort by keys worked out once per row in the order comparing the
    /// values themselves gives, by any field, shown or not.
    #[test]
    fn sorting_by_keys_matches_comparing_values() {
        let (project, geom) = sample();
        let compare = |a: &Value, b: &Value| match (a.number(), b.number()) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            _ => natural(&text_of(a), &text_of(b)),
        };
        for category in Category::ALL {
            let mut shown = ScheduleField::available(category);
            shown.truncate(3);
            for field in ScheduleField::available(category) {
                for (fields, descending) in [(vec![ScheduleField::Mark], false), (shown.clone(), true)] {
                    let def = ScheduleDef { fields, sort_by: Some(field), descending, ..Default::default() };
                    let t = schedule(&project, &geom, category, &def);
                    let mut expected: Vec<&Element> = project.elements_of(category).collect();
                    expected.sort_by(|a, b| default_order(a, b));
                    expected.sort_by(|a, b| {
                        let order = compare(&value(&project, &geom, a, field), &value(&project, &geom, b, field));
                        if descending { order.reverse() } else { order }
                    });
                    let got: Vec<_> = t.rows.iter().map(|r| r.element).collect();
                    let expected: Vec<_> = expected.iter().map(|e| e.id).collect();
                    assert_eq!(got, expected, "{category:?} by {field:?}");
                }
            }
        }
    }

    #[test]
    fn structural_quantities() {
        let (project, geom) = sample();
        // The porch beam spans 5 m between two 0.20 m columns: 4.80 m of body.
        let t = schedule(&project, &geom, Category::Beam, &ScheduleDef::default());
        let col = |t: &Table, key: &str| t.headers.iter().position(|h| *h == key).unwrap();
        assert_eq!(t.rows.len(), 1);
        let row = &t.rows[0].cells;
        assert_eq!(
            row[col(&t, "col-section")],
            Cell::Text(format!("{} × {}", format::length(0.15), format::length(0.4)))
        );
        assert_eq!(row[col(&t, "col-length")], Cell::Text(format::meters(5.0)));
        assert_eq!(row[col(&t, "col-cut-length")], Cell::Text(format::meters(4.8)));
        assert_eq!(row[col(&t, "col-volume")], Cell::Text(format::volume(0.15 * 0.4 * 4.8)));
        assert_ne!(row[col(&t, "col-material")], Cell::Text(String::new()));
        assert_eq!(t.totals[col(&t, "col-cut-length")], format::meters(4.8));
        let t = schedule(&project, &geom, Category::Column, &ScheduleDef::default());
        assert_eq!(t.rows.len(), 2);
        let section = format!("{} × {}", format::length(0.2), format::length(0.2));
        assert!(t.rows.iter().all(|r| r.cells[col(&t, "col-section")] == Cell::Text(section.clone())));
    }

    #[test]
    fn csv_quotes_and_separates() {
        let table = Table {
            headers: vec!["a", "b"],
            rows: vec![Row {
                element: cc_core::ElementId(1),
                cells: vec![Cell::Text("x;y".into()), Cell::Key("k")],
                heading: false,
                sketch: None,
                footer: false,
            }],
            totals: vec!["1".into(), String::new()],
            ..Default::default()
        };
        let csv = to_csv(&table, |h| h.to_uppercase(), |c| c.raw().to_string());
        assert_eq!(csv, "A;B\r\n\"x;y\";k\r\n1;\r\n");
        // A table under it follows a blank line and its caption.
        let under = Table { headers: vec!["c"], totals: vec!["2".into()], caption: Some("cap"), ..Default::default() };
        let both = Table { summary: Some(Box::new(under)), ..table };
        let csv = to_csv(&both, |h| h.to_uppercase(), |c| c.raw().to_string());
        assert!(csv.ends_with("1;\r\n\r\nCAP\r\nC\r\n2\r\n"), "{csv:?}");
    }

    #[test]
    fn csv_keeps_formulas_as_text() {
        let text = |s: &str| Cell::Text(s.into());
        let table = Table {
            headers: vec!["a", "b", "c", "d", "e", "f"],
            rows: vec![Row {
                element: cc_core::ElementId(1),
                cells: vec![
                    text("=1+1"),
                    text("+cmd|' /C calc'!A0"),
                    text("-2+3"),
                    text("@SUM(A1)"),
                    text("-3.50 m"),
                    text("\t=1"),
                ],
                heading: false,
                sketch: None,
                footer: false,
            }],
            totals: vec!["-12".into()],
            ..Default::default()
        };
        let csv = to_csv(&table, |h| h.to_uppercase(), |c| c.raw().to_string());
        let row = csv.lines().nth(1).unwrap();
        let cells: Vec<&str> = row.split(';').collect();
        assert_eq!(cells[0], "\"'=1+1\"");
        assert!(cells[1].starts_with("\"'+cmd|"), "{row}");
        assert_eq!(cells[2], "\"'-2+3\"");
        assert_eq!(cells[3], "\"'@SUM(A1)\"");
        // Numbers, also negative and with their unit, stay as they are.
        assert_eq!(cells[4], "-3.50 m");
        assert_eq!(cells.last().unwrap(), &"\"'\t=1\"");
        assert!(csv.ends_with("\r\n-12\r\n"), "{csv}");
    }

    /// A schedule drawn for a sheet: title, headers, a row per element and
    /// the totals, ruled at its printed size (paper millimeters at 1:1).
    #[test]
    fn schedules_draw_as_ruled_tables() {
        let (project, geom) = sample();
        let def = ScheduleDef { fields: vec![ScheduleField::Mark, ScheduleField::Area], ..ScheduleDef::default() };
        let t = schedule(&project, &geom, Category::Window, &def);
        assert!(!t.rows.is_empty());
        let d = drawing(&t, "Windows", 1, &|key| format!("<{key}>"));
        let texts: Vec<&str> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                crate::Prim::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts.first(), Some(&"Windows"));
        assert!(texts.iter().any(|t| t.starts_with('<')), "headers go through the label: {texts:?}");
        assert!(texts.contains(&t.totals[0].as_str()), "the totals row");
        // Title, headers, rows and totals: as many rows of 6 mm.
        let (lo, hi) = d.bounds().unwrap();
        let rows = 2 + t.rows.len() + 1;
        assert!(((hi.y - lo.y) * 1000.0 - 6.0 * rows as f64).abs() < 1e-6, "{lo} {hi}");
        // At 1:50 the same table is 50 times larger in view units.
        let big = drawing(&t, "Windows", 50, &|key| key.to_string());
        let (blo, bhi) = big.bounds().unwrap();
        assert!(((bhi.y - blo.y) - (hi.y - lo.y) * 50.0).abs() < 1e-9);
    }
}
