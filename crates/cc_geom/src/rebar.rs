//! Reinforcing bars: the center lines of each set of bars in its beam or
//! column, from the host's body, the cover and the bar diameters.
//!
//! Longitudinal bars sit inside the stirrups (cover plus the host's largest
//! stirrup diameter); stirrups are closed loops `cover` inside the faces,
//! spaced evenly and centered along the host. Along an arc beam the bars
//! follow the arc, each at its own radius, and the stirrups are radial. Top
//! and bottom bars that do not fit side by side go in further layers, side
//! bars run between them, and bars longer than the stock length are spliced.
//!
//! Anchored bars of a beam run on into the columns and pile caps its ends
//! frame into, bending down or up where the straight length does not fit,
//! and lap inside a support with the bars of a beam carrying on in line.
//!
//! A column's bars stand around its section ([`crate::faces`]): a count
//! spread around it, or so many on each face with the corner bars shared;
//! its ties follow the section's outline, cross-ties link the bars between
//! the corners to the bars opposite, and the vertical bars may be cut in
//! lifts that lap.

use crate::beam::ColumnInput;
use crate::faces::{self, FaceBar, SectionFace};
use crate::polygon;
use crate::{BeamEnd, BeamGeom};
use cc_core::profiles_project::SectorPlacement;
use cc_core::{AnchorHook, Anchorage, BarFace, ElementId, ElementKind, Project, Rebar, RebarLayout, Splice, TypeData};
use glam::{DVec2, DVec3};
use std::collections::BTreeMap;

/// Density of steel, kg/m³.
pub const STEEL_DENSITY: f64 = 7850.0;

/// Segments of round ties.
const ROUND: usize = 16;

/// At most this many stirrups per set (a crafted spacing cannot explode).
const MAX_STIRRUPS: usize = 2000;

/// At most this many layers of top or bottom bars; the rest crowd the last.
const MAX_LAYERS: usize = 4;

/// At most this many bars in one set, splice pieces included.
pub(crate) const MAX_BARS: usize = 20_000;

/// Least clear distance between parallel bars: the bar diameter, and 20 mm.
fn clear_distance(d: f64) -> f64 {
    d.max(0.02)
}

/// One bar: its center line, closed for stirrups.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub points: Vec<DVec3>,
    pub closed: bool,
    /// The bar's true length when its center line is drawn with chords (a
    /// bar following an arc): the arc's length at the bar's radius. The
    /// hooks of a closed bar (`hooks`) come on top of it.
    pub true_length: Option<f64>,
    /// The hooks that close a closed bar (a stirrup), counted in its length
    /// but not drawn in its center line.
    pub hooks: Option<LoopHooks>,
    /// The coil an open bar winds along when it is a spiral (its center line
    /// is the coil in chords and `true_length` the coil's exact length).
    pub helix: Option<Helix>,
    /// The hooks at the two ends of an open bar (a cross-tie), counted in
    /// its `true_length` but not drawn in its center line.
    pub end_hooks: Option<EndHooks>,
}

/// A spiral's coil: its center-line radius, its rise per turn and its
/// number of turns (meters, and turns not necessarily whole).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Helix {
    pub radius: f64,
    pub pitch: f64,
    pub turns: f64,
}

impl Helix {
    /// Whether the values are finite and above zero.
    pub fn is_valid(&self) -> bool {
        [self.radius, self.pitch, self.turns].iter().all(|v| v.is_finite() && *v > 0.0)
    }

    /// The exact length of the coil: each turn `sqrt((2πr)² + pitch²)`.
    pub fn length(&self) -> f64 {
        self.turns * (std::f64::consts::TAU * self.radius).hypot(self.pitch)
    }
}

/// The two hooks that close a stirrup, both at its first corner
/// (`points[0]`), bent into the section the way the loop turns there: each
/// turns by `angle` radians around a pin, its center line at `radius`, then
/// runs `tail` straight. The loop's sides are measured to the sharp corner,
/// and each hook adds its bend and its straight end, as a tie's hooks do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopHooks {
    pub angle: f64,
    pub radius: f64,
    pub tail: f64,
}

impl LoopHooks {
    /// The hooks of a stirrup of bars `d` thick (none for unusable values).
    pub fn of(hooks: &cc_core::StirrupHooks, d: f64) -> Option<Self> {
        let usable = hooks.is_valid() && d.is_finite() && d > 0.0;
        usable.then(|| Self { angle: hooks.angle.to_radians(), radius: PIN_RADIUS * d, tail: hooks.tail_length(d) })
    }

    /// Length of one hook: its bend and its straight end.
    pub fn hook_length(&self) -> f64 {
        self.angle * self.radius + self.tail
    }

    /// Length of the two hooks.
    pub fn length(&self) -> f64 {
        2.0 * self.hook_length()
    }
}

/// The hooks at the ends of an open bar, at its first point (`start`) and
/// its last (`end`): each turns by its `angle` around a pin and runs its
/// `tail` straight, as a stirrup's hooks do. Both bend to the same side of
/// the bar (a C-shaped tie), or to opposite sides when `opposite` (an
/// S-shaped tie). The bar's center line runs between the bars it ties, and
/// its `true_length` adds the two hooks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndHooks {
    pub start: LoopHooks,
    pub end: LoopHooks,
    pub opposite: bool,
}

impl EndHooks {
    /// The hooks of a tie of bars `d` thick turning by `start` and `end`
    /// radians: each bent around a pin three bar diameters across, then a
    /// straight end of five diameters (at least 5 cm).
    pub fn of_tie(start: f64, end: f64, d: f64, opposite: bool) -> Self {
        let tail = (5.0 * d).max(cc_core::StirrupHooks::MIN_TAIL_LENGTH);
        let one = |angle: f64| LoopHooks { angle, radius: PIN_RADIUS * d, tail };
        Self { start: one(start), end: one(end), opposite }
    }

    /// Length of the two hooks: their bends and their straight ends.
    pub fn length(&self) -> f64 {
        self.start.hook_length() + self.end.hook_length()
    }
}

impl Bar {
    /// A bar along straight segments.
    pub fn new(points: Vec<DVec3>, closed: bool) -> Self {
        Self { points, closed, true_length: None, hooks: None, helix: None, end_hooks: None }
    }

    /// The length of the bar: its center line (or true length) and its hooks.
    pub fn length(&self) -> f64 {
        let hooks = match (self.closed, self.hooks) {
            (true, Some(h)) => h.length(),
            _ => 0.0,
        };
        if let Some(length) = self.true_length {
            return length + hooks;
        }
        let open: f64 = self.points.windows(2).map(|w| w[0].distance(w[1])).sum();
        let closing = match (self.closed, self.points.first(), self.points.last()) {
            (true, Some(a), Some(b)) => a.distance(*b),
            _ => 0.0,
        };
        open + closing + hooks
    }
}

/// The bars of one reinforcement set.
#[derive(Debug, Clone, PartialEq)]
pub struct RebarGeom {
    pub element: ElementId,
    /// The beam or column the set is laid out in (none for shaped bars).
    pub host: Option<ElementId>,
    pub diameter: f64,
    /// The steel grade of the set's type (empty: none).
    pub grade: String,
    /// The type's mass per meter, kg/m, when it gives one.
    pub mass_per_meter: Option<f64>,
    pub bars: Vec<Bar>,
}

impl RebarGeom {
    /// Mass of a meter of bar, kilograms: the type's, else a round bar of
    /// the diameter in steel of [`STEEL_DENSITY`].
    pub fn mass_per_meter(&self) -> f64 {
        self.mass_per_meter
            .filter(|m| m.is_finite() && *m > 0.0)
            .unwrap_or(std::f64::consts::PI * self.diameter * self.diameter / 4.0 * STEEL_DENSITY)
    }

    pub fn count(&self) -> usize {
        self.bars.len()
    }

    pub fn total_length(&self) -> f64 {
        self.bars.iter().map(Bar::length).sum()
    }

    /// Mean length of one bar.
    pub fn bar_length(&self) -> f64 {
        if self.bars.is_empty() { 0.0 } else { self.total_length() / self.bars.len() as f64 }
    }

    /// Steel weight in kilograms.
    pub fn weight(&self) -> f64 {
        self.total_length() * self.mass_per_meter()
    }
}

/// `n` positions spread evenly over `[-half, half]` (the middle for one).
fn spread(n: usize, half: f64) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![0.0],
        _ => (0..n).map(|i| -half + 2.0 * half * i as f64 / (n - 1) as f64).collect(),
    }
}

/// Positions `spacing` apart filling `[a, b]`, centered.
pub(crate) fn stations(a: f64, b: f64, spacing: f64) -> Vec<f64> {
    if b < a || spacing <= 0.0 {
        return Vec::new();
    }
    let count = (((b - a) / spacing).floor() as usize + 1).min(MAX_STIRRUPS);
    let start = a + ((b - a) - spacing * (count - 1) as f64) / 2.0;
    (0..count).map(|i| start + spacing * i as f64).collect()
}

/// `n` points around a `2a` x `2b` rectangle: the corners first, the rest
/// along the sides, each to the side with the widest gap.
fn perimeter(n: usize, a: f64, b: f64) -> Vec<DVec2> {
    let corners = [DVec2::new(-a, -b), DVec2::new(a, -b), DVec2::new(a, b), DVec2::new(-a, b)];
    if n <= 4 {
        // One or two bars sit on the bottom side's corners; three take three corners.
        return corners.into_iter().take(n).collect();
    }
    let lengths = [2.0 * a, 2.0 * b, 2.0 * a, 2.0 * b];
    let mut extra = [0usize; 4];
    for _ in 4..n {
        let side = (0..4)
            .max_by(|&i, &j| (lengths[i] / (extra[i] + 1) as f64).total_cmp(&(lengths[j] / (extra[j] + 1) as f64)))
            .unwrap_or(0);
        extra[side] += 1;
    }
    let mut points = corners.to_vec();
    for side in 0..4 {
        let (p, q) = (corners[side], corners[(side + 1) % 4]);
        for k in 1..=extra[side] {
            points.push(p + (q - p) * (k as f64 / (extra[side] + 1) as f64));
        }
    }
    points
}

fn circle(n: usize, r: f64) -> Vec<DVec2> {
    (0..n).map(|i| DVec2::from_angle(i as f64 / n as f64 * std::f64::consts::TAU) * r).collect()
}

/// The diameter of a rebar element's type.
fn diameter(project: &Project, e: &cc_core::Element) -> Option<f64> {
    match project.element_type(e.type_id).map(|t| &t.data) {
        Some(TypeData::Rebar { diameter, .. }) if diameter.is_finite() && *diameter > 0.0 => Some(*diameter),
        _ => None,
    }
}

/// The steel grade and the mass per meter of a rebar element's type.
fn steel(project: &Project, e: &cc_core::Element) -> (String, Option<f64>) {
    match project.element_type(e.type_id).map(|t| &t.data) {
        Some(TypeData::Rebar { grade, mass_per_meter, .. }) => (grade.clone(), *mass_per_meter),
        _ => (String::new(), None),
    }
}

/// Bars of every reinforcement set whose host has a body.
pub fn build_rebar(
    project: &Project,
    beams: &BTreeMap<ElementId, BeamGeom>,
    walls: &BTreeMap<ElementId, crate::WallGeom>,
    foundations: &BTreeMap<ElementId, crate::FoundationGeom>,
    piles: &BTreeMap<ElementId, crate::pile::PileGeom>,
) -> BTreeMap<ElementId, RebarGeom> {
    // The largest stirrup of each host: longitudinal bars sit inside it. A
    // cap's side loops, and a pile's spiral and ties, are kept apart.
    let mut stirrup: BTreeMap<ElementId, f64> = BTreeMap::new();
    let mut skin: BTreeMap<ElementId, f64> = BTreeMap::new();
    let mut tie: BTreeMap<ElementId, f64> = BTreeMap::new();
    for e in project.elements.values() {
        let ElementKind::Rebar(r) = &e.kind else { continue };
        let map = match r.layout {
            RebarLayout::Stirrups { .. } => &mut stirrup,
            RebarLayout::CapSides { .. } => &mut skin,
            RebarLayout::PileSpiral { .. } | RebarLayout::PileTies { .. } => &mut tie,
            _ => continue,
        };
        if let Some(d) = diameter(project, e) {
            let s = map.entry(r.host).or_insert(0.0);
            *s = s.max(d);
        }
    }
    // The first longitudinal set of each host: its cross-ties hook around those bars.
    let mut main: BTreeMap<ElementId, (&Rebar, f64)> = BTreeMap::new();
    for e in project.elements.values() {
        if let ElementKind::Rebar(r) = &e.kind
            && matches!(r.layout, RebarLayout::Longitudinal { .. })
            && let Some(d) = diameter(project, e)
        {
            main.entry(r.host).or_insert((r, d));
        }
    }
    // The sets of each wall, by family and face: layers and ties are placed by them.
    let mut wall_sets: BTreeMap<ElementId, Vec<(&Rebar, f64)>> = BTreeMap::new();
    for e in project.elements.values() {
        if let ElementKind::Rebar(r) = &e.kind
            && r.layout.in_wall()
            && let Some(d) = diameter(project, e)
        {
            wall_sets.entry(r.host).or_default().push((r, d));
        }
    }
    // The sets of each slab: the layers of a mesh are stacked by them.
    let mut slab_sets: BTreeMap<ElementId, Vec<(&Rebar, f64)>> = BTreeMap::new();
    for e in project.elements.values() {
        if let ElementKind::Rebar(r) = &e.kind
            && r.layout.in_slab()
            && let Some(d) = diameter(project, e)
        {
            slab_sets.entry(r.host).or_default().push((r, d));
        }
    }
    // The ends of the hosts' axes, when a set laps its bars past them.
    let laps = project.elements.values().any(|e| matches!(&e.kind, ElementKind::Rebar(r) if r.lap_at_joins));
    let ends_of = |g: &BeamGeom| [(g.world(0.0, 0.0), -g.dir_at(0.0)), (g.world(g.length, 0.0), g.dir_at(g.length))];
    let mut wall_ends: AxisEnds = BTreeMap::new();
    let mut beam_ends: AxisEnds = BTreeMap::new();
    let mut strip_ends: AxisEnds = BTreeMap::new();
    if laps {
        wall_ends = walls
            .iter()
            .map(|(id, g)| (*id, [(g.world(0.0, 0.0), -g.tangent(0.0)), (g.world(g.length, 0.0), g.tangent(g.length))]))
            .collect();
        beam_ends = beams.iter().map(|(id, g)| (*id, ends_of(g))).collect();
        strip_ends = foundations.iter().filter_map(|(id, f)| Some((*id, ends_of(f.strip.as_ref()?)))).collect();
    }
    // The supports beams frame into, when a set is anchored into them.
    let anchored = project.elements.values().any(|e| matches!(&e.kind, ElementKind::Rebar(r) if r.anchorage.is_some()));
    let supports = if anchored { Supports::of(project, beams) } else { Supports::default() };
    let mut out = BTreeMap::new();
    for e in project.elements.values() {
        let ElementKind::Rebar(r) = &e.kind else { continue };
        let Some(d) = diameter(project, e) else { continue };
        let Some(host) = project.element(r.host) else { continue };
        let ds = stirrup.get(&r.host).copied().unwrap_or(0.0);
        let bars = match &host.kind {
            ElementKind::Beam(_) => {
                let Some(g) = beams.get(&r.host) else { continue };
                let Some(TypeData::Beam { height, .. }) = project.element_type(host.type_id).map(|t| &t.data) else {
                    continue;
                };
                let anchors = supports.anchors(g, r, d);
                beam_bars(g, *height, r, d, ds, beam_past(r, d, g, &beam_ends), anchors)
            }
            // A strip footing: its own bars, or those of a beam in its beam part.
            ElementKind::Foundation(f) if matches!(f.shape, cc_core::FoundationShape::Strip { .. }) => {
                let Some(g) = foundations.get(&r.host).and_then(|f| f.strip.as_ref()) else { continue };
                let Some(data @ TypeData::Foundation { width, thickness, .. }) =
                    project.element_type(host.type_id).map(|t| &t.data)
                else {
                    continue;
                };
                let section = crate::strip_rebar::StripSection {
                    width: *width,
                    thickness: *thickness,
                    haunch: data.strip_haunch(),
                };
                if r.layout.in_strip() {
                    // The bars along the strip lie inside those across it, in the same layer.
                    let across = |s: &Rebar| match (&s.layout, &r.layout) {
                        (
                            RebarLayout::StripTransverse { top: a, .. },
                            RebarLayout::StripLongitudinal { top: b, .. },
                        ) => a == b,
                        _ => false,
                    };
                    let other = project
                        .rebar_of(r.host)
                        .filter(|(_, s)| across(s))
                        .filter_map(|(e, _)| diameter(project, e))
                        .fold(0.0, f64::max);
                    crate::strip_rebar::strip_bars(g, &section, r, d, other, beam_past(r, d, g, &strip_ends))
                } else {
                    // As a beam as wide as the beam part and as deep as the whole section.
                    let part = BeamGeom { half: section.beam_half(), ..g.clone() };
                    beam_bars(&part, data.foundation_depth(), r, d, ds, beam_past(r, d, g, &strip_ends), [None, None])
                }
            }
            // A pile cap: its own bars, and stirrups across a cap of two piles or a line.
            ElementKind::Foundation(_) if foundations.get(&r.host).is_some_and(|f| f.cap.is_some()) => {
                let Some(f) = foundations.get(&r.host) else { continue };
                let Some(cap) = f.cap.as_ref() else { continue };
                let Some(placed) = project.pile_cap(host) else { continue };
                let pile_size = project
                    .cap_pile_type(placed.cap)
                    .and_then(|t| project.element_type(t))
                    .and_then(|t| match t.data {
                        TypeData::Pile { section, .. } => Some(section.size()),
                        _ => None,
                    })
                    .unwrap_or(cc_core::pile::DEFAULT_DIAMETER);
                let input = crate::cap_rebar::CapInput {
                    cap,
                    layout: placed.cap.layout,
                    z: f.z_range,
                    embedment: placed.cap.embedment,
                    pile_size,
                    stirrup: ds,
                    skin: skin.get(&r.host).copied().unwrap_or(0.0),
                };
                crate::cap_rebar::cap_bars(&input, r, d)
            }
            // A pile: its bars reach into its cap, at most to the cap's top less the cover.
            ElementKind::Pile(_) => {
                let Some(g) = piles.get(&r.host) else { continue };
                let cap_top = g.cap.and_then(|c| foundations.get(&c)).map(|f| f.z_range.1);
                let input =
                    crate::pile_rebar::PileInput { pile: g, cap_top, tie: tie.get(&r.host).copied().unwrap_or(0.0) };
                crate::pile_rebar::pile_bars(&input, r, d)
            }
            ElementKind::Floor(_) | ElementKind::Foundation(_) => {
                let Some(slab) = crate::slab_rebar::Slab::of(project, host) else { continue };
                // The largest bar of the other sets of the layer that an inner set lies inside.
                let other = slab_sets
                    .get(&r.host)
                    .into_iter()
                    .flatten()
                    .filter(|(s, _)| !std::ptr::eq(*s, r) && s.layout.slab_top() == r.layout.slab_top())
                    .filter(|(s, _)| !matches!(s.layout, RebarLayout::SlabTrim { .. }) && !slab_inner(&s.layout))
                    .map(|(_, d)| *d)
                    .fold(0.0, f64::max);
                crate::slab_rebar::slab_bars(&slab, r, d, other)
            }
            ElementKind::Wall(_) => {
                // A spline wall takes no bars yet.
                let Some(g) = walls.get(&r.host).filter(|g| g.spline.is_none()) else { continue };
                let all = wall_sets.get(&r.host).map_or(&[][..], |v| v.as_slice());
                let mut context = wall_context(r, all);
                if r.lap_at_joins {
                    let half = half_lap(r, d);
                    let on = |joined: bool, end: usize| {
                        if joined && carries_on(r.host, end, &wall_ends) { half } else { 0.0 }
                    };
                    context.past = (on(g.start_joined, 0), on(g.end_joined, 1));
                }
                crate::wall_rebar::wall_bars(g, r, d, &context)
            }
            ElementKind::Column(c) => {
                let Some(data @ TypeData::Column { section, .. }) = project.element_type(host.type_id).map(|t| &t.data)
                else {
                    continue;
                };
                let outline = project.type_section(data).map(|s| s.outline.outer).unwrap_or_default();
                let shape = ColumnShape {
                    size: section.size(),
                    round: project.section_is_round(data),
                    sector: project.type_sector(data),
                    boxed: is_box(&outline),
                    outline,
                };
                let frame =
                    ColumnFrame { position: c.position, rotation_deg: c.rotation_deg, z: project.column_span(host, c) };
                column_bars(&shape, &frame, r, d, ds, main.get(&r.host).copied())
            }
            _ => continue,
        };
        let (grade, mass_per_meter) = steel(project, e);
        out.insert(e.id, RebarGeom { element: e.id, host: Some(r.host), diameter: d, grade, mass_per_meter, bars });
    }
    // Shaped bars stand where they are drawn.
    for e in project.elements.values() {
        let ElementKind::ShapedRebar(r) = &e.kind else { continue };
        let Some(d) = diameter(project, e) else { continue };
        let z = project.level_elevation(e.level);
        let bars = r.bars.iter().filter_map(|b| shaped_bar(b, z)).collect();
        let (grade, mass_per_meter) = steel(project, e);
        out.insert(e.id, RebarGeom { element: e.id, host: None, diameter: d, grade, mass_per_meter, bars });
    }
    out
}

/// The two ends of each host's axis: the point, and the direction the axis leaves it by.
type AxisEnds = BTreeMap<ElementId, [(DVec2, DVec2); 2]>;

/// Ends of two axes this near are one joint.
const JOINT_REACH: f64 = 0.05;

/// Whether another host of `ends` carries on from the `end` (0: start, 1:
/// end) of `host`: one of its ends lies there and its axis leaves the
/// joint the opposite way, within some ten degrees.
fn carries_on(host: ElementId, end: usize, ends: &AxisEnds) -> bool {
    let Some((at, out)) = ends.get(&host).and_then(|e| e.get(end)).copied() else { return false };
    let others = ends.iter().filter(|(id, _)| **id != host).flat_map(|(_, e)| e.iter());
    others.into_iter().any(|(p, dir)| p.distance(at) <= JOINT_REACH && dir.dot(out) < -0.985)
}

/// Half the lap of the set `r` of bars `d` thick: its splices', else 40 diameters.
fn half_lap(r: &Rebar, d: f64) -> f64 {
    lap_of(r, d) * 0.5
}

/// The lap of the set `r` of bars `d` thick: its splices', else 40 diameters.
pub(crate) fn lap_of(r: &Rebar, d: f64) -> f64 {
    r.splice.filter(Splice::is_valid).map_or(40.0 * d, |s| s.lap)
}

/// How far the bars of `r` along the beam or strip `g` run past its start
/// and end: half a lap where it is joined to another that carries on.
fn beam_past(r: &Rebar, d: f64, g: &BeamGeom, ends: &AxisEnds) -> (f64, f64) {
    if !r.lap_at_joins {
        return (0.0, 0.0);
    }
    let half = half_lap(r, d);
    let on = |joined: bool, end: usize| if joined && carries_on(g.element, end, ends) { half } else { 0.0 };
    (on(g.start == BeamEnd::Beam, 0), on(g.end == BeamEnd::Beam, 1))
}

/// The stretch of the axis a bar along its host covers: `inside` is the
/// stretch the `cover` clear of both end faces, and where the bar runs
/// `past` an end (a length on its own line, `scale` times the axis's) it
/// starts that far beyond the face instead.
pub(crate) fn run_past(inside: (f64, f64), cover: f64, past: (f64, f64), scale: f64) -> (f64, f64) {
    let scale = scale.max(1e-9);
    let from = if past.0 > 0.0 { inside.0 - cover - past.0 / scale } else { inside.0 };
    let to = if past.1 > 0.0 { inside.1 + cover + past.1 / scale } else { inside.1 };
    (from, to)
}

/// Two beam ends framed into one support whose axes leave it this close to
/// opposite ways (some ten degrees) carry on in line through it.
const IN_LINE: f64 = -0.985;

/// How far behind a beam's end face the line of a bar may meet the support
/// and still run into it, meters.
const SUPPORT_GAP: f64 = 0.02;

/// The supports beams frame into (structural columns and pile caps), and the
/// beam ends framed into each: what anchored bars run into.
#[derive(Default)]
struct Supports {
    /// Each support's plan outline and its bottom and top elevations.
    shapes: BTreeMap<ElementId, ColumnInput>,
    /// The beam ends framed into each support.
    ends: BTreeMap<ElementId, Vec<FramedEnd>>,
}

/// A beam end framed into a support: the beam, the direction its axis
/// leaves the support by and the beam's lowest and highest elevations.
type FramedEnd = (ElementId, DVec2, (f64, f64));

impl Supports {
    fn of(project: &Project, beams: &BTreeMap<ElementId, BeamGeom>) -> Self {
        let mut out = Self::default();
        for g in beams.values() {
            let ends = [(g.start, -g.dir_at(0.0)), (g.end, g.dir_at(g.length))];
            for (end, out_dir) in ends {
                let BeamEnd::Column(support) = end else { continue };
                out.ends.entry(support).or_default().push((g.element, out_dir, g.z_bounds()));
                if !out.shapes.contains_key(&support)
                    && let Some(shape) = project.element(support).and_then(|e| crate::build::support_input(project, e))
                {
                    out.shapes.insert(support, shape);
                }
            }
        }
        out
    }

    /// How the bars of the set `r` (bars `d` thick) in the beam `g` run into
    /// the supports at its start and end: none where it is not anchored or
    /// the end frames into no support.
    fn anchors(&self, g: &BeamGeom, r: &Rebar, d: f64) -> [Option<EndAnchor<'_>>; 2] {
        let Some(a) = r.anchorage.filter(|a| a.is_valid() && Anchorage::fits(&r.layout)) else { return [None, None] };
        let lap = lap_of(r, d);
        let z = g.z_bounds();
        let end = |end: BeamEnd, out: DVec2| {
            let BeamEnd::Column(support) = end else { return None };
            let shape = self.shapes.get(&support).filter(|s| s.outline.len() >= 3)?;
            let in_line = self.ends.get(&support).is_some_and(|ends| {
                ends.iter().any(|(id, dir, zr)| *id != g.element && dir.dot(out) < IN_LINE && overlaps(*zr, z))
            });
            Some(EndAnchor {
                outline: &shape.outline,
                z: shape.z_range,
                in_line,
                length: a.length * d,
                lap,
                min_hook: Anchorage::MIN_HOOK * d,
                hook: a.hook,
            })
        };
        [end(g.start, -g.dir_at(0.0)), end(g.end, g.dir_at(g.length))]
    }
}

/// Whether two ranges of elevations overlap.
fn overlaps(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 - 1e-6 && b.0 < a.1 - 1e-6
}

/// How the bars of a set run into the support at one end of a beam.
#[derive(Debug, Clone, Copy)]
struct EndAnchor<'a> {
    /// The support's plan outline, and its bottom and top elevations.
    outline: &'a [DVec2],
    z: (f64, f64),
    /// Another beam carries on in line from the other side of the support.
    in_line: bool,
    /// The anchorage, the lap and the shortest hook leg, meters.
    length: f64,
    lap: f64,
    min_hook: f64,
    hook: AnchorHook,
}

/// The stretches `(from, to)` along the axis of the beam `g` where the bar
/// line `v` (its circle on an arc beam) lies inside `outline`, rising: the
/// parts of a straight line, or of the circle within half a turn either
/// way of `face`, the end face's place on the line.
fn support_span(g: &BeamGeom, v: f64, outline: &[DVec2], face: f64) -> Vec<(f64, f64)> {
    let Some(arc) = &g.arc else { return polygon::clip_line(outline, g.world(0.0, v), g.dir) };
    let (c, r, big) = (arc.center, arc.radius_at(v), arc.radius);
    if !(r.is_finite() && r > 1e-9 && big.is_finite() && big > 1e-9) {
        return Vec::new();
    }
    // Where the circle crosses each side, as a place along the axis from the face's.
    let at_face = arc.angle_at(face);
    let mut us: Vec<f64> = Vec::new();
    for (i, p) in outline.iter().enumerate() {
        let Some(q) = outline.get((i + 1) % outline.len()) else { continue };
        let (dir, from_c) = (*q - *p, *p - c);
        let a = dir.length_squared();
        let b = 2.0 * from_c.dot(dir);
        let disc = b * b - 4.0 * a * (from_c.length_squared() - r * r);
        if !(a > 1e-18 && disc >= 0.0) {
            continue;
        }
        let root = disc.sqrt();
        for t in [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)] {
            if (0.0..1.0).contains(&t) {
                let hit = *p + dir * t - c;
                let turn = (hit.y.atan2(hit.x) - at_face + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                us.push(face + arc.sign() * turn * big);
            }
        }
    }
    us.retain(|u| u.is_finite());
    us.sort_by(f64::total_cmp);
    us.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    // The stretches between crossings whose middle is inside.
    let mut out: Vec<(f64, f64)> = Vec::new();
    for w in us.windows(2) {
        let (a, b) = (w[0], w[1]);
        if !polygon::contains(outline, arc.at((a + b) * 0.5, v)) {
            continue;
        }
        match out.last_mut() {
            Some(last) if (last.1 - a).abs() < 1e-12 => last.1 = b,
            _ => out.push((a, b)),
        }
    }
    out
}

/// Where a bar on the line `(v, z)` of the section of the beam `g` ends in
/// the support of `anchor` at the beam's start (`at_end` false) or end: its
/// place along the axis, and the signed leg of its hook (up when positive;
/// 0 for none), the hook bending `bend` (1 up, -1 down) when it may. `scale`
/// is the bar's length per meter of axis. `None` where the bar's line misses
/// the support behind the end face or lies above or below it, or the
/// support leaves no room past the cover: the bar then stops as it would.
///
/// The bar runs on along its line (its circle) past the support's near face,
/// up to its far face less the cover, at most the anchorage length; where
/// that is short of the anchorage and the bar may bend, a 90° leg up or down
/// makes up the rest, at least `min_hook` long and inside the support's
/// height less the cover (none where that leaves less). Where another beam
/// carries on in line, the bar runs half a lap past the support's middle
/// along its line instead (still at most to the far face less the cover).
#[allow(clippy::too_many_arguments)]
fn anchor_end(
    g: &BeamGeom,
    v: f64,
    z: f64,
    at_end: bool,
    anchor: &EndAnchor,
    cover: f64,
    scale: f64,
    bend: Option<f64>,
) -> Option<(f64, f64)> {
    let face = g.face_u(at_end, v);
    let spans = support_span(g, v, anchor.outline, face);
    // The stretch behind the face (or across it) nearest to it: the bar meets the
    // support's near face where its own line does (a round column bulges past the
    // end face, which runs straight between the beam's sides).
    let (near, far) = if at_end {
        let (t0, t1) = spans.into_iter().filter(|s| s.1 > face + 1e-9).min_by(|a, b| a.0.total_cmp(&b.0))?;
        (t0 <= face + SUPPORT_GAP).then_some((t0, t1))?
    } else {
        let (t0, t1) = spans.into_iter().filter(|s| s.0 < face - 1e-9).max_by(|a, b| a.1.total_cmp(&b.1))?;
        (t1 >= face - SUPPORT_GAP).then_some((t1, t0))?
    };
    let depth = (far - near).abs() * scale;
    let room = depth - cover;
    if !(room.is_finite() && room > 1e-6) {
        return None;
    }
    let (reach, hook) = if anchor.in_line {
        ((depth + anchor.lap) * 0.5, None)
    } else {
        (anchor.length, bend.filter(|_| anchor.length > room + 1e-9))
    };
    let run = reach.min(room);
    let u = if at_end { near + run / scale } else { near - run / scale };
    // The bar lies within the support's height.
    let elevation = g.z_at(u).0 + z * g.stretch();
    if !(elevation > anchor.z.0 && elevation < anchor.z.1) {
        return None;
    }
    let leg = hook.map_or(0.0, |dir| {
        let height = if dir < 0.0 { elevation - (anchor.z.0 + cover) } else { anchor.z.1 - cover - elevation };
        let want = (anchor.length - run).max(anchor.min_hook);
        if height >= anchor.min_hook { want.min(height) * dir.signum() } else { 0.0 }
    });
    Some((u, leg))
}

/// A bar along the line `(v, z)` of a beam's section from `a` to `b` along
/// the axis (`scale` meters of bar per meter of axis), with a vertical leg
/// bent at each end (up when positive, meters; 0 for none). The corners are
/// sharp: the legs are measured to the line of the run, as a wall bar's
/// bent ends are.
struct BeamRun {
    v: f64,
    z: f64,
    a: f64,
    b: f64,
    scale: f64,
    legs: [f64; 2],
}

impl BeamRun {
    /// The developed length: the legs and the run.
    fn length(&self) -> f64 {
        self.legs[0].abs() + (self.b - self.a) * self.scale + self.legs[1].abs()
    }

    /// The point `u` along the axis of `g` on the bar's line.
    fn at(&self, g: &BeamGeom, u: f64) -> DVec3 {
        g.world(u, self.v).extend(g.z_at(u).0 + self.z * g.stretch())
    }

    /// The piece of the bar from `from` to `to` along its developed length
    /// (following the arc of an arc beam in chords, with its true length).
    fn piece(&self, g: &BeamGeom, from: f64, to: f64) -> Bar {
        let (l0, l1) = (self.legs[0].abs(), self.legs[1].abs());
        let s = self.scale.max(1e-9);
        let run = ((self.b - self.a) * s).max(0.0);
        let point = |x: f64| -> DVec3 {
            if x < l0 {
                self.at(g, self.a) + DVec3::Z * self.legs[0] * (1.0 - x / l0)
            } else if x > l0 + run && l1 > 0.0 {
                self.at(g, self.b) + DVec3::Z * self.legs[1] * ((x - l0 - run) / l1).min(1.0)
            } else {
                self.at(g, self.a + (x - l0).clamp(0.0, run) / s)
            }
        };
        let mut points = vec![point(from)];
        if from < l0 && to > l0 {
            points.push(self.at(g, self.a));
        }
        // An arc in chords, each at most the arc step.
        let (ra, rb) = (from.max(l0), to.min(l0 + run));
        if let Some(arc) = &g.arc
            && rb > ra
        {
            let (ua, ub) = (self.a + (ra - l0) / s, self.a + (rb - l0) / s);
            let n = cc_core::arc::segments((ub - ua) / arc.radius.max(1e-9), cc_core::arc::ARC_STEP);
            points.extend((1..n).map(|i| self.at(g, ua + (ub - ua) * i as f64 / n as f64)));
        }
        if l1 > 0.0 && from < l0 + run && to > l0 + run {
            points.push(self.at(g, self.b));
        }
        points.push(point(to));
        let all = points.clone();
        points.dedup_by(|p, q| p.distance(*q) < 1e-9);
        let points = if points.len() >= 2 { points } else { all };
        Bar { true_length: g.arc.is_some().then_some(to - from), ..Bar::new(points, false) }
    }
}

/// Whether a slab's set lies inside the other bars of its layer.
fn slab_inner(layout: &RebarLayout) -> bool {
    matches!(
        layout,
        RebarLayout::SlabStraight { inner: true, .. }
            | RebarLayout::SlabRing { inner: true, .. }
            | RebarLayout::SlabRadial { inner: true, .. }
    )
}

/// What the set `r` needs to know of `all` the sets of its wall.
fn wall_context<'a>(r: &Rebar, all: &[(&'a Rebar, f64)]) -> crate::wall_rebar::WallSets<'a> {
    use RebarLayout::{WallHorizontal as H, WallVertical as V};
    let horizontal = |s: &Rebar| matches!(s.layout, H { .. });
    let on = |side| move |(s, _): &&(&'a Rebar, f64)| s.layout.wall_side() == Some(side);
    // The other family near the same face.
    let other = all
        .iter()
        .filter(|(s, _)| s.layout.wall_side() == r.layout.wall_side() && horizontal(s) != horizontal(r))
        .filter(|(s, _)| matches!(s.layout, H { .. } | V { .. }))
        .map(|(_, d)| *d)
        .fold(0.0, f64::max);
    let first = |side, wanted: bool| {
        all.iter()
            .filter(on(side))
            .find(|(s, _)| matches!(s.layout, H { .. } | V { .. }) && horizontal(s) == wanted)
            .copied()
    };
    // The interior face's outer layer: the set not marked `inner`, else any.
    let interior = all
        .iter()
        .filter(on(cc_core::WallSide::Interior))
        .filter(|(s, _)| matches!(&s.layout, H { inner: false, .. } | V { inner: false, .. }))
        .chain(all.iter().filter(on(cc_core::WallSide::Interior)))
        .map(|(s, d)| (s.cover, *d))
        .next();
    crate::wall_rebar::WallSets {
        other,
        horizontal: first(cc_core::WallSide::Exterior, true),
        vertical: first(cc_core::WallSide::Exterior, false),
        interior,
        past: (0.0, 0.0),
    }
}

/// A shaped bar in the model, `z` up (its level's elevation): closed when its
/// ends meet (the last point dropped); `None` with less than two points apart.
fn shaped_bar(points: &[DVec3], z: f64) -> Option<Bar> {
    let mut points: Vec<DVec3> = points.iter().filter(|p| p.is_finite()).map(|p| *p + DVec3::Z * z).collect();
    points.dedup_by(|a, b| a.distance(*b) < 1e-9);
    let closed = points.len() > 3 && points.first().zip(points.last()).is_some_and(|(a, b)| a.distance(*b) < 1e-6);
    if closed {
        points.pop();
    }
    (points.len() >= 2).then_some(Bar::new(points, closed))
}

/// How many bars go in each layer: `count` bars `d` thick across a width of
/// `across` between the outermost bar centers. Bars that do not fit side by
/// side at the least clear distance go in further layers, the last of
/// [`MAX_LAYERS`] taking what is left.
fn layers(count: usize, across: f64, d: f64) -> Vec<usize> {
    let pitch = d + clear_distance(d);
    let fit = if across.is_finite() && pitch > 0.0 { (across.max(0.0) / pitch).floor() as usize + 1 } else { 1 };
    // Two corner bars always share a layer.
    let fit = fit.max(2);
    let mut out = Vec::new();
    let mut left = count;
    while left > 0 {
        let n = if out.len() + 1 == MAX_LAYERS { left } else { left.min(fit) };
        out.push(n);
        left -= n;
    }
    out
}

/// Heights of side bars between `lo` and `hi` (the lowest and highest bar
/// lines of the section): `count` evenly spread, or with a count of 0 as
/// many as keep them at most `spacing` apart.
pub(crate) fn side_heights(count: u32, spacing: f64, lo: f64, hi: f64) -> Vec<f64> {
    if hi <= lo {
        return Vec::new();
    }
    let n = if count > 0 {
        count as usize
    } else if spacing.is_finite() && spacing > 0.0 {
        (((hi - lo) / spacing) - 1e-9).ceil().max(1.0) as usize - 1
    } else {
        0
    };
    let n = n.min(200);
    (1..=n).map(|i| lo + (hi - lo) * i as f64 / (n + 1) as f64).collect()
}

/// Bars along a beam: longitudinal and side bars from end to end inside the
/// cover (following the arc of an arc beam), stirrups in vertical planes
/// across it (radial on an arc). The bars along it run `past` its ends
/// (half a lap where another beam carries on), or into the supports of
/// `anchors` (which come first at an end that has both).
fn beam_bars(
    g: &BeamGeom,
    height: f64,
    r: &Rebar,
    d: f64,
    ds: f64,
    past: (f64, f64),
    anchors: [Option<EndAnchor>; 2],
) -> Vec<Bar> {
    let (layout, cover, splice) = (&r.layout, r.cover, r.splice);
    let k = g.stretch();
    let u0 = g.start_ext.0.max(g.start_ext.1) + cover;
    let u1 = g.end_ext.0.min(g.end_ext.1) - cover;
    if u1 <= u0 {
        return Vec::new();
    }
    let at = |u: f64, v: f64, z: f64| g.world(u, v).extend(g.z_at(u).0 + z * k);
    // The way the anchored bars' hooks bend: top and bottom bars may, side bars do not.
    let face = match layout {
        RebarLayout::Longitudinal { face, .. } => Some(*face),
        _ => None,
    };
    let bend = anchors.map(|a| a.and_then(|a| a.hook.direction(face)));
    // The bars along the lines `(v, z)` of the section, spliced where longer than the stock.
    let runs = |lines: Vec<(f64, f64)>| {
        let mut bars = Vec::new();
        for (index, (v, z)) in lines.into_iter().enumerate() {
            // Length of the bar per meter of the axis: its own radius on an arc, the slope.
            let scale = g.arc.as_ref().map_or(k, |a| (a.radius_at(v) / a.radius.max(1e-9)).max(1e-9));
            let (mut a, mut b) = run_past((u0, u1), cover, past, scale);
            // Into the supports at the ends framed into one, with the hooks' legs.
            let mut legs = [0.0; 2];
            for (end, anchor) in anchors.iter().enumerate() {
                let Some(anchor) = anchor else { continue };
                let bend = bend.get(end).copied().flatten();
                let Some((u, leg)) = anchor_end(g, v, z, end == 1, anchor, cover, scale, bend) else { continue };
                if end == 0 {
                    a = u;
                } else {
                    b = u;
                }
                legs[end] = leg;
            }
            if !(b - a).is_finite() || b - a <= 1e-9 {
                continue;
            }
            // The developed bar (legs and run) cut into the stock's pieces.
            let bar = BeamRun { v, z, a, b, scale, legs };
            let length = bar.length();
            let pieces = splice.map_or_else(|| vec![(0.0, length)], |s| s.pieces(length, index));
            for (from, to) in pieces {
                if bars.len() >= MAX_BARS {
                    return bars;
                }
                bars.push(bar.piece(g, from, to));
            }
        }
        bars
    };
    let inset = cover + ds + d / 2.0;
    match *layout {
        RebarLayout::Longitudinal { face, count } => {
            let half = (g.half - inset).max(0.0);
            let step = d + clear_distance(d);
            let mut lines = Vec::new();
            for (layer, n) in layers(count as usize, 2.0 * half, d).into_iter().enumerate() {
                let z = match face {
                    BarFace::Top => (height - inset - step * layer as f64).max(inset),
                    BarFace::Bottom | BarFace::Perimeter => {
                        (inset + step * layer as f64).min((height - inset).max(inset))
                    }
                };
                lines.extend(spread(n, half).into_iter().map(|v| (v, z)));
            }
            runs(lines)
        }
        RebarLayout::Sides { count, spacing } => {
            let v = (g.half - inset).max(0.0);
            let heights = side_heights(count, spacing, inset, height - inset);
            runs(heights.into_iter().flat_map(|z| [(-v, z), (v, z)]).collect())
        }
        // Cross-ties belong to columns, wall bars to walls, slab bars to slabs.
        RebarLayout::Ties { .. }
        | RebarLayout::WallHorizontal { .. }
        | RebarLayout::WallVertical { .. }
        | RebarLayout::WallTies { .. }
        | RebarLayout::SlabStraight { .. }
        | RebarLayout::SlabRing { .. }
        | RebarLayout::SlabRadial { .. }
        | RebarLayout::SlabTrim { .. }
        | RebarLayout::StripTransverse { .. }
        | RebarLayout::StripLongitudinal { .. }
        | RebarLayout::CapBottom { .. }
        | RebarLayout::CapTop { .. }
        | RebarLayout::CapSides { .. }
        | RebarLayout::PileLongitudinal { .. }
        | RebarLayout::PileSpiral { .. }
        | RebarLayout::PileTies { .. } => Vec::new(),
        RebarLayout::Stirrups { spacing, hooks } => {
            let inset = cover + d / 2.0;
            let (v, z0, z1) = ((g.half - inset).max(0.0), inset, (height - inset).max(inset));
            let hooks = hooks.and_then(|h| LoopHooks::of(&h, d));
            stations(u0, u1, spacing)
                .into_iter()
                .map(|u| Bar {
                    hooks,
                    ..Bar::new(vec![at(u, -v, z0), at(u, v, z0), at(u, v, z1), at(u, -v, z1)], true)
                })
                .collect()
        }
    }
}

/// A column's section as its bars see it, in the column's own frame.
struct ColumnShape {
    /// Width and depth of its box.
    size: (f64, f64),
    round: bool,
    /// An annular sector's ring (its faces are then exact arcs).
    sector: Option<SectorPlacement>,
    /// Whether the outline is that box and nothing else.
    boxed: bool,
    outline: Vec<DVec2>,
}

impl ColumnShape {
    /// The section's faces `inset` inside it.
    fn faces(&self, inset: f64) -> Vec<SectionFace> {
        match &self.sector {
            Some(s) => faces::sector_faces(s, inset),
            None => faces::outline_faces(&self.outline, inset),
        }
    }

    /// The longitudinal bars of the set `r` (bars `d` thick inside ties `ds` thick).
    fn bars(&self, r: &Rebar, d: f64, ds: f64) -> Vec<FaceBar> {
        let RebarLayout::Longitudinal { count, .. } = r.layout else { return Vec::new() };
        let inset = r.cover + ds + d / 2.0;
        let (w, depth) = self.size;
        let bar = |at: DVec2, inward: DVec2, corner: bool| FaceBar { at, inward, corner };
        if self.round {
            return circle(count as usize, (w / 2.0 - inset).max(0.0))
                .into_iter()
                .map(|p| bar(p, -p.normalize_or_zero(), false))
                .collect();
        }
        if self.boxed && r.faces.is_empty() {
            let (a, b) = ((w / 2.0 - inset).max(0.0), (depth / 2.0 - inset).max(0.0));
            return perimeter(count as usize, a, b)
                .into_iter()
                .enumerate()
                .map(|(i, p)| {
                    // A bar between corners looks across the side it stands on.
                    let inward =
                        if (p.y.abs() - b).abs() < 1e-9 { -DVec2::Y * p.y.signum() } else { -DVec2::X * p.x.signum() };
                    bar(p, inward, i < 4)
                })
                .collect();
        }
        faces::face_bars(&self.faces(inset), &r.faces, count as usize)
    }
}

/// The faces of a column type's section `inset` inside it, in the order
/// bars are given face by face: counter-clockwise, an annular sector's
/// from its outer arc. Empty for other types and failed sections.
pub fn column_faces(project: &Project, data: &TypeData, inset: f64) -> Vec<SectionFace> {
    let TypeData::Column { section, .. } = data else { return Vec::new() };
    let outline = project.type_section(data).map(|s| s.outline.outer).unwrap_or_default();
    let shape = ColumnShape {
        size: section.size(),
        round: project.section_is_round(data),
        sector: project.type_sector(data),
        boxed: is_box(&outline),
        outline,
    };
    shape.faces(inset)
}

/// Whether `outline` is its own bounding box: four corners, nothing else.
fn is_box(outline: &[DVec2]) -> bool {
    let lo = outline.iter().fold(DVec2::INFINITY, |m, p| m.min(*p));
    let hi = outline.iter().fold(DVec2::NEG_INFINITY, |m, p| m.max(*p));
    let at = |v: f64, a: f64, b: f64| (v - a).abs() < 1e-9 || (v - b).abs() < 1e-9;
    outline.len() == 4 && outline.iter().all(|p| at(p.x, lo.x, hi.x) && at(p.y, lo.y, hi.y))
}

/// Where a column stands: its point, turn and base and top elevations.
struct ColumnFrame {
    position: DVec2,
    rotation_deg: f64,
    z: (f64, f64),
}

/// The center-line radius of a hook's bend, bar diameters: around a pin
/// three bar diameters across.
const PIN_RADIUS: f64 = 2.0;

/// `outline` (a closed loop, counter-clockwise) from its sharpest convex
/// corner, where a stirrup's hooks go: the corner that turns furthest
/// inward, the first of those alike (a loop without one as it is).
pub(crate) fn from_convex_corner(outline: Vec<DVec2>) -> Vec<DVec2> {
    let n = outline.len();
    if n < 3 {
        return outline;
    }
    let turn = |i: usize| {
        let (Some(a), Some(b), Some(c)) = (outline.get((i + n - 1) % n), outline.get(i), outline.get((i + 1) % n))
        else {
            return 0.0;
        };
        let (u, v) = (*b - *a, *c - *b);
        u.perp_dot(v).atan2(u.dot(v))
    };
    let mut best = 0;
    for i in 1..n {
        if turn(i) > turn(best) + 1e-9 {
            best = i;
        }
    }
    let mut out = outline;
    out.rotate_left(best);
    out
}

/// Whether each of a column's vertical bars `placed` is an odd one in the
/// order the bars stand around the section (the staggered ones): the order
/// they are given in, except a plain box's, whose corners come first and
/// are put in order by their angle about the middle.
fn around_parity(shape: &ColumnShape, r: &Rebar, placed: &[FaceBar]) -> Vec<bool> {
    if shape.round || !shape.boxed || !r.faces.is_empty() {
        return (0..placed.len()).map(|i| i % 2 == 1).collect();
    }
    let mut order: Vec<usize> = (0..placed.len()).collect();
    let angle = |i: usize| placed.get(i).map_or(0.0, |b| b.at.y.atan2(b.at.x));
    order.sort_by(|a, b| angle(*a).total_cmp(&angle(*b)).then(a.cmp(b)));
    let mut odd = vec![false; placed.len()];
    for (rank, i) in order.into_iter().enumerate() {
        if let Some(o) = odd.get_mut(i) {
            *o = rank % 2 == 1;
        }
    }
    odd
}

/// Bars of a column: vertical bars around the section over its height
/// (cut in lifts by a splice), horizontal ties up the column following its
/// outline, and cross-ties between the bars of its longitudinal set `main`.
fn column_bars(
    shape: &ColumnShape,
    frame: &ColumnFrame,
    r: &Rebar,
    d: f64,
    ds: f64,
    main: Option<(&Rebar, f64)>,
) -> Vec<Bar> {
    let rot = DVec2::from_angle(frame.rotation_deg.to_radians());
    let place = |p: DVec2, z: f64| (frame.position + rot.rotate(p)).extend(z);
    let (base, top) = frame.z;
    let (w, depth) = shape.size;
    match r.layout {
        RebarLayout::Longitudinal { .. } => {
            // The bars are cut in the lifts the column is cast in, at the heights
            // given or equal ones under the stock length; with a stagger, every
            // other bar around the section is cut that much higher.
            let lap = lap_of(r, d);
            let plain = r.column_pieces(top - base, lap, false);
            let staggered = r.column_pieces(top - base, lap, true);
            let placed = shape.bars(r, d, ds);
            let mut bars = Vec::new();
            for (bar, odd) in placed.iter().zip(around_parity(shape, r, &placed)) {
                for (from, to) in if odd { &staggered } else { &plain } {
                    if bars.len() >= MAX_BARS {
                        return bars;
                    }
                    bars.push(Bar::new(vec![place(bar.at, base + from), place(bar.at, base + to)], false));
                }
            }
            bars
        }
        // Side bars belong to beams, wall bars to walls, slab bars to slabs.
        RebarLayout::Sides { .. }
        | RebarLayout::WallHorizontal { .. }
        | RebarLayout::WallVertical { .. }
        | RebarLayout::WallTies { .. }
        | RebarLayout::SlabStraight { .. }
        | RebarLayout::SlabRing { .. }
        | RebarLayout::SlabRadial { .. }
        | RebarLayout::SlabTrim { .. }
        | RebarLayout::StripTransverse { .. }
        | RebarLayout::StripLongitudinal { .. }
        | RebarLayout::CapBottom { .. }
        | RebarLayout::CapTop { .. }
        | RebarLayout::CapSides { .. }
        | RebarLayout::PileLongitudinal { .. }
        | RebarLayout::PileSpiral { .. }
        | RebarLayout::PileTies { .. } => Vec::new(),
        RebarLayout::Stirrups { spacing, hooks } => {
            let inset = r.cover + d / 2.0;
            let hooks = hooks.and_then(|h| LoopHooks::of(&h, d));
            // Overlapping stirrups: one around each rectangle the section is made of.
            let rects = if r.overlapping && !shape.round { faces::rectangles(&shape.outline) } else { Vec::new() };
            if rects.len() > 1 {
                let loops: Vec<[DVec2; 4]> = rects
                    .iter()
                    .map(|(lo, hi)| (*lo + DVec2::splat(inset), *hi - DVec2::splat(inset)))
                    .filter(|(lo, hi)| hi.x - lo.x > 1e-6 && hi.y - lo.y > 1e-6)
                    .map(|(lo, hi)| [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)])
                    .collect();
                let mut out = Vec::new();
                for z in stations(base + r.cover, top - r.cover, spacing) {
                    for l in &loops {
                        if out.len() >= MAX_BARS {
                            return out;
                        }
                        out.push(Bar { hooks, ..Bar::new(l.iter().map(|p| place(*p, z)).collect(), true) });
                    }
                }
                return out;
            }
            let (outline, length) = if shape.round {
                (circle(ROUND, (w / 2.0 - inset).max(0.0)), None)
            } else if shape.boxed {
                let (a, b) = ((w / 2.0 - inset).max(0.0), (depth / 2.0 - inset).max(0.0));
                (vec![DVec2::new(-a, -b), DVec2::new(a, -b), DVec2::new(a, b), DVec2::new(-a, b)], None)
            } else {
                // Around the outline, its curved faces as curves.
                let (points, length) = faces::face_loop(&shape.faces(inset));
                (points, Some(length))
            };
            if outline.len() < 3 {
                return Vec::new();
            }
            // The hooks close the stirrup at a convex corner.
            let outline = if hooks.is_some() { from_convex_corner(outline) } else { outline };
            stations(base + r.cover, top - r.cover, spacing)
                .into_iter()
                .map(|z| Bar {
                    true_length: length,
                    hooks,
                    ..Bar::new(outline.iter().map(|p| place(*p, z)).collect(), true)
                })
                .collect()
        }
        RebarLayout::Ties { spacing } => {
            let Some((set, dm)) = main else { return Vec::new() };
            let bars = shape.bars(set, dm, ds);
            // The lines the main bars stand on, to find the bar opposite.
            let lines = match shape.round || (shape.boxed && set.faces.is_empty()) {
                true => faces::outline_faces(&bars.iter().filter(|b| b.corner).map(|b| b.at).collect::<Vec<_>>(), 0.0),
                false => shape.faces(set.cover + ds + dm / 2.0),
            };
            let near = (3.0 * dm).max(0.03);
            let mut ties: Vec<(DVec2, DVec2)> = Vec::new();
            for bar in bars.iter().filter(|b| !b.corner) {
                let Some(hit) = faces::across(&lines, bar.at, bar.inward, near) else { continue };
                // Onto the bar that stands there, when one does.
                let to = bars
                    .iter()
                    .map(|b| b.at)
                    .filter(|p| p.distance(hit) < near)
                    .min_by(|p, q| p.distance(hit).total_cmp(&q.distance(hit)))
                    .unwrap_or(hit);
                // One tie between two bars, whichever it was found from.
                let same = |a: DVec2, b: DVec2| a.distance(b) < near;
                if !ties.iter().any(|(p, q)| same(*p, to) && same(*q, bar.at)) {
                    ties.push((bar.at, to));
                }
            }
            let ends = EndHooks::of_tie(0.75 * std::f64::consts::PI, std::f64::consts::PI, d, false);
            let hooks = ends.length();
            let mut out = Vec::new();
            for z in stations(base + r.cover, top - r.cover, spacing) {
                for (a, b) in &ties {
                    if out.len() >= MAX_BARS {
                        return out;
                    }
                    let points = vec![place(*a, z), place(*b, z)];
                    out.push(Bar {
                        true_length: Some(a.distance(*b) + hooks),
                        end_hooks: Some(ends),
                        ..Bar::new(points, false)
                    });
                }
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stirrups_fill_the_length_centered() {
        // 1.00 m at 0.15: 7 stirrups spanning 0.90, starting 0.05 in.
        let s = stations(0.0, 1.0, 0.15);
        assert_eq!(s.len(), 7);
        assert!((s[0] - 0.05).abs() < 1e-12 && (s[6] - 0.95).abs() < 1e-12);
        assert!(stations(1.0, 0.0, 0.15).is_empty());
    }

    #[test]
    fn perimeter_bars_take_the_corners_then_the_longer_sides() {
        let p = perimeter(6, 0.2, 0.1);
        assert_eq!(p.len(), 6);
        // The extra two go to the long (0.4 m) sides, at their middles.
        assert!(p.contains(&DVec2::new(0.0, -0.1)) && p.contains(&DVec2::new(0.0, 0.1)));
        assert_eq!(perimeter(4, 0.2, 0.1).len(), 4);
    }

    #[test]
    fn bars_that_do_not_fit_take_a_second_layer() {
        // 0.14 m between the outer bar centers, 16 mm bars 36 mm apart: four fit.
        assert_eq!(layers(4, 0.14, 0.016), vec![4]);
        assert_eq!(layers(6, 0.14, 0.016), vec![4, 2]);
        // Two corner bars share a layer however narrow the beam.
        assert_eq!(layers(3, 0.01, 0.016), vec![2, 1]);
        // The last layer takes what is left.
        assert_eq!(layers(11, 0.01, 0.016), vec![2, 2, 2, 5]);
        assert!(layers(0, 0.14, 0.016).is_empty());
    }

    #[test]
    fn side_bars_come_from_a_count_or_a_spacing() {
        let h = side_heights(2, 0.0, 0.05, 0.65);
        assert!((h[0] - 0.25).abs() < 1e-12 && (h[1] - 0.45).abs() < 1e-12, "{h:?}");
        // 0.60 m at most 0.20 apart: two bars, 0.20 from each other and the ends.
        assert_eq!(side_heights(0, 0.2, 0.05, 0.65).len(), 2);
        assert_eq!(side_heights(0, 0.25, 0.05, 0.65).len(), 2);
        assert!(side_heights(0, 0.6, 0.05, 0.65).is_empty());
        assert!(side_heights(3, 0.0, 0.5, 0.5).is_empty());
    }

    #[test]
    fn long_bars_are_cut_in_the_fewest_pieces_and_staggered() {
        let s = Splice::new(12.0, 0.5);
        // Within the stock: one piece, whatever the bar.
        assert_eq!(s.pieces(12.0, 0), vec![(0.0, 12.0)]);
        assert_eq!(s.pieces(8.0, 1), vec![(0.0, 8.0)]);
        // 23.5 m is exactly two pieces of 12 m lapping 0.5 m.
        let two = s.pieces(23.5, 0);
        assert_eq!(two.len(), 2);
        assert!((two[0].1 - 12.0).abs() < 1e-9 && (two[1].0 - 11.5).abs() < 1e-9 && (two[1].1 - 23.5).abs() < 1e-9);
        // A little more takes three, each as long as the others.
        let three = s.pieces(23.6, 0);
        assert_eq!(three.len(), 3);
        let lengths: Vec<f64> = three.iter().map(|(a, b)| b - a).collect();
        assert!(lengths.iter().all(|l| (l - lengths[0]).abs() < 1e-9 && *l <= 12.0), "{lengths:?}");
        assert!((lengths.iter().sum::<f64>() - (23.6 + 2.0 * 0.5)).abs() < 1e-9);
        // The next bar is cut half a piece off: one more piece, short ones at both ends.
        let odd = s.pieces(23.6, 1);
        assert_eq!(odd.len(), 4);
        assert!((odd[0].0).abs() < 1e-12 && (odd[3].1 - 23.6).abs() < 1e-9);
        assert!(odd.windows(2).all(|w| (w[0].1 - w[1].0 - 0.5).abs() < 1e-9), "{odd:?}");
        assert!(((odd[0].1 - odd[0].0) - (odd[3].1 - odd[3].0)).abs() < 1e-9);
        // Values a file may hold: no splices rather than a loop.
        assert_eq!(Splice::new(0.0, 0.5).pieces(30.0, 0), vec![(0.0, 30.0)]);
        assert_eq!(Splice::new(12.0, 12.0).pieces(30.0, 0), vec![(0.0, 30.0)]);
        assert_eq!(s.pieces(f64::NAN, 0).len(), 1);
        assert!(s.pieces(1e9, 0).len() <= 10_001);
    }

    #[test]
    fn a_stirrup_is_a_closed_loop() {
        let bar = Bar::new(
            vec![DVec3::ZERO, DVec3::new(0.1, 0.0, 0.0), DVec3::new(0.1, 0.0, 0.3), DVec3::new(0.0, 0.0, 0.3)],
            true,
        );
        assert!((bar.length() - 0.8).abs() < 1e-12);
    }

    /// A 20 x 50 cm beam from `a` to `b` bending by `bulge`, framed into `columns`.
    fn framed(a: DVec2, b: DVec2, bulge: f64, columns: &[ColumnInput]) -> BeamGeom {
        let input = crate::beam::BeamInput {
            element: ElementId(1),
            start: a,
            end: b,
            half: 0.1,
            z_range: (2.5, 3.0),
            slope: 0.0,
            section: crate::beam::box_section(0.2, 0.5),
            curved: false,
            join_start: true,
            join_end: true,
            bulge,
        };
        crate::beam::resolve_beams(&[input], columns).remove(0)
    }

    fn square(id: u64, c: DVec2, half: f64) -> ColumnInput {
        let outline =
            [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| c + DVec2::new(x, y) * half).to_vec();
        ColumnInput { element: ElementId(id), outline, z_range: (0.0, 3.0) }
    }

    fn anchor(outline: &[DVec2]) -> EndAnchor<'_> {
        EndAnchor {
            outline,
            z: (0.0, 3.0),
            in_line: false,
            length: 0.4,
            lap: 0.4,
            min_hook: 0.08,
            hook: AnchorHook::Auto,
        }
    }

    #[test]
    fn a_bar_runs_to_the_far_face_of_its_support() {
        let column = square(10, DVec2::ZERO, 0.15);
        let g = framed(DVec2::ZERO, DVec2::new(6.0, 0.0), 0.0, std::slice::from_ref(&column));
        assert_eq!(g.start, BeamEnd::Column(ElementId(10)));
        // 0.3 deep: 0.275 straight from the face at 0.15, and a leg up of the 0.125 left.
        let a = anchor(&column.outline);
        let (u, leg) = anchor_end(&g, 0.07, 0.03, false, &a, 0.025, 1.0, Some(1.0)).unwrap();
        assert!((u - (0.15 - 0.275)).abs() < 1e-12 && (leg - 0.125).abs() < 1e-12, "{u} {leg}");
        // In line with another beam: half the column and half a lap, at most to the far face.
        let (u, leg) =
            anchor_end(&g, 0.07, 0.03, false, &EndAnchor { in_line: true, ..a }, 0.025, 1.0, Some(1.0)).unwrap();
        assert!((u - (0.15 - 0.275)).abs() < 1e-12 && leg == 0.0, "{u} {leg}");
        // A bar above the support's top runs into nothing.
        let low = EndAnchor { z: (0.0, 2.6), ..a };
        assert!(anchor_end(&g, 0.07, 0.3, false, &low, 0.025, 1.0, Some(1.0)).is_none());
    }

    /// Supports a file may hold: anchored bars stop as unanchored ones do, and nothing panics.
    #[test]
    fn hostile_supports_anchor_nothing() {
        let column = square(10, DVec2::ZERO, 0.15);
        let g = framed(DVec2::ZERO, DVec2::new(6.0, 0.0), 0.0, std::slice::from_ref(&column));
        let flat = [DVec2::new(-0.15, 0.0), DVec2::new(0.15, 0.0), DVec2::new(0.0, 0.0)];
        let nan = [DVec2::new(f64::NAN, 0.0), DVec2::new(0.15, -0.15), DVec2::new(0.15, 0.15)];
        let far_away = square(11, DVec2::new(-5.0, 0.0), 0.15).outline;
        let thin = square(12, DVec2::ZERO, 0.01).outline;
        for outline in [&flat[..], &nan[..], &far_away[..], &thin[..], &[][..]] {
            for bend in [None, Some(-1.0), Some(1.0)] {
                assert!(anchor_end(&g, 0.07, 0.03, false, &anchor(outline), 0.025, 1.0, bend).is_none());
            }
        }
        // A cover deeper than the support.
        assert!(anchor_end(&g, 0.0, 0.03, false, &anchor(&column.outline), 0.5, 1.0, None).is_none());
        // Not numbers.
        let a = EndAnchor { length: f64::NAN, lap: f64::NAN, min_hook: f64::NAN, ..anchor(&column.outline) };
        let _ = anchor_end(&g, f64::NAN, f64::NAN, true, &a, f64::NAN, f64::NAN, Some(f64::NAN));
        let _ = anchor_end(&g, 0.07, 0.03, false, &a, 0.025, 1.0, Some(1.0));
        // An arc of a tiny radius, and an arc whose circle misses the support.
        let tiny = framed(DVec2::new(0.11, 0.0), DVec2::new(-0.11, 0.0), 1.0, &[]);
        let _ = anchor_end(&tiny, 0.0, 0.03, false, &anchor(&column.outline), 0.025, 1.0, Some(1.0));
        let _ = support_span(&tiny, 0.05, &column.outline, 0.0);
        let ring = framed(DVec2::new(4.0, 0.0), DVec2::new(0.0, 4.0), (std::f64::consts::FRAC_PI_2 / 4.0).tan(), &[]);
        assert!(support_span(&ring, 0.0, &far_away, 0.0).is_empty());
    }

    /// Along an arc beam the bar runs on along its own circle, to where that
    /// circle leaves the support.
    #[test]
    fn an_arc_bar_runs_on_along_its_circle() {
        let column = square(10, DVec2::new(4.0, 0.0), 0.3);
        let bulge = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
        let g = framed(DVec2::new(4.0, 0.0), DVec2::new(0.0, 4.0), bulge, std::slice::from_ref(&column));
        assert_eq!(g.start, BeamEnd::Column(ElementId(10)));
        // The circle of radius 3.93 crosses the faces y = ±0.3 at ±asin(0.3 / 3.93).
        let rho: f64 = 3.93;
        let spans = support_span(&g, 0.07, &column.outline, g.face_u(false, 0.07));
        let edge = (0.3 / rho).asin() * 4.0;
        assert!(spans.iter().any(|(a, b)| (a + edge).abs() < 1e-9 && (b - edge).abs() < 1e-9), "{spans:?}");
        // 40 diameters straight, along the circle: 0.4 m of it past the near face.
        let scale = rho / 4.0;
        let (u, leg) = anchor_end(&g, 0.07, 0.03, false, &anchor(&column.outline), 0.025, scale, Some(1.0)).unwrap();
        assert!((u - (edge - 0.4 / scale)).abs() < 1e-9 && leg == 0.0, "{u}");
    }
}
