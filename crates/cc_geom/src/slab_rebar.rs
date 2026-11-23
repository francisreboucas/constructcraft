//! Slab reinforcement: the bars of floors and foundation slabs. Straight
//! bars of a mesh, ring and radial bars, and trimming bars around openings
//! lie in the slab's top or bottom layer, `cover` inside its faces, edge and
//! openings. Each bar is clipped to the region the cover leaves, so every
//! bar has its own length; where the boundary was drawn with arcs or
//! circles the bars end on the true curve, not on its chords.

use crate::polygon;
use crate::rebar::{Bar, MAX_BARS};
use cc_core::arc::{ARC_STEP, segments};
use cc_core::{Element, ElementKind, Project, Rebar, RebarLayout, Splice, TypeData};
use glam::{DVec2, DVec3};
use std::f64::consts::TAU;

/// Bars shorter than this are left out.
const MIN_BAR: f64 = 0.05;

/// At most this many lines, rings or openings are laid out in one set.
const MAX_LINES: usize = 5000;

/// A slab as its bars see it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Slab {
    /// Boundary, counter-clockwise.
    pub outline: Vec<DVec2>,
    pub holes: Vec<Vec<DVec2>>,
    /// The circles its boundary and openings were drawn on (center, radius).
    pub circles: Vec<(DVec2, f64)>,
    /// Bottom and top of its structure.
    pub z: (f64, f64),
}

impl Slab {
    /// The slab of a floor or a foundation slab; `None` for other elements.
    pub fn of(project: &Project, e: &Element) -> Option<Slab> {
        let level = project.level_elevation(e.level);
        let ty = project.element_type(e.type_id)?;
        match &e.kind {
            ElementKind::Floor(f) => {
                // Bars lie in the structure: under the finishes, over the ceiling layers.
                let (above, below) = ty.core_range();
                let top = level + f.height_offset;
                let circles = f.sketch.iter().flat_map(|s| &s.curves).filter_map(circle_of).collect();
                Some(Slab {
                    outline: polygon::ccw(&f.boundary),
                    holes: f.all_openings().filter(|o| o.len() >= 3).cloned().collect(),
                    circles,
                    z: (top - below, top - above),
                })
            }
            ElementKind::Foundation(f) => {
                let cc_core::FoundationShape::Slab { boundary } = &f.shape else { return None };
                let TypeData::Foundation { thickness, .. } = &ty.data else { return None };
                let top = level + f.top_offset;
                let circles = f.sketch.iter().flat_map(|s| &s.curves).filter_map(circle_of).collect();
                Some(Slab { outline: polygon::ccw(boundary), holes: Vec::new(), circles, z: (top - thickness, top) })
            }
            _ => None,
        }
    }

    /// The middle of the slab's box: where rings and radial bars are centered by default.
    fn middle(&self) -> DVec2 {
        let lo = self.outline.iter().fold(DVec2::INFINITY, |m, p| m.min(*p));
        let hi = self.outline.iter().fold(DVec2::NEG_INFINITY, |m, p| m.max(*p));
        (lo + hi) * 0.5
    }
}

/// The circle a sketch curve lies on.
fn circle_of(curve: &cc_core::sketch::SketchCurve) -> Option<(DVec2, f64)> {
    match *curve {
        cc_core::sketch::SketchCurve::Circle { center, radius } => Some((center, radius)),
        cc_core::sketch::SketchCurve::Arc { a, b, bulge } => {
            cc_core::arc::Arc::from_bulge(a, b, bulge).map(|arc| (arc.center, arc.radius))
        }
        _ => None,
    }
}

/// One closed line of the region's edge: the boundary moved in, or an
/// opening grown, by the cover.
struct Loop {
    points: Vec<DVec2>,
    /// For each side (from point `i` to the next), the circle it is a chord of.
    circles: Vec<Option<(DVec2, f64)>>,
    /// The slab's own edge (not an opening).
    outer: bool,
}

/// Where bars may lie: inside the boundary and outside the openings, the
/// cover clear of both.
pub(crate) struct Region {
    loops: Vec<Loop>,
}

/// A crossing of a line with the region's edge: its distance along the
/// line, and whether it is on the slab's own edge.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Crossing {
    t: f64,
    outer: bool,
}

/// A stretch of a line inside the region, and whether each end lies on the slab's own edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Stretch {
    pub from: f64,
    pub to: f64,
    pub edge: (bool, bool),
}

impl Region {
    /// The region `cover` inside `slab`. Empty when the cover leaves nothing.
    pub fn new(slab: &Slab, cover: f64) -> Region {
        let mut loops = Vec::new();
        let rings = std::iter::once((&slab.outline, true)).chain(slab.holes.iter().map(|h| (h, false)));
        for (ring, outer) in rings {
            let ring = polygon::ccw(ring);
            let n = ring.len();
            if n < 3 {
                continue;
            }
            // The boundary moves in, an opening grows.
            let moved = polygon::offset(&ring, if outer { -cover } else { cover });
            let side = |p: &[DVec2], i: usize| p[(i + 1) % n] - p[i];
            if moved.len() != n || (0..n).any(|i| side(&moved, i).dot(side(&ring, i)) <= 1e-12) {
                // The cover has eaten the slab: nothing; an opening too small to trace is ignored.
                if outer {
                    return Region { loops: Vec::new() };
                }
                continue;
            }
            let circles = (0..n)
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    let on = |p: DVec2, c: DVec2, r: f64| (p.distance(c) - r).abs() < 1e-6;
                    let (center, radius) =
                        slab.circles.iter().copied().find(|(c, r)| on(a, *c, *r) && on(b, *c, *r))?;
                    // With the center on the left of the side the ring bulges out: the slab's
                    // edge then moves towards the center, an opening's away from it.
                    let bulges_out = (b - a).perp_dot(center - a) > 0.0;
                    let moved = if bulges_out == outer { radius - cover } else { radius + cover };
                    (moved > 1e-6).then_some((center, moved))
                })
                .collect();
            loops.push(Loop { points: moved, circles, outer });
        }
        Region { loops }
    }

    pub fn is_empty(&self) -> bool {
        self.loops.is_empty()
    }

    /// The crossings of the line through `origin` along the unit `dir`, in order.
    fn crossings(&self, origin: DVec2, dir: DVec2) -> Vec<Crossing> {
        let mut out = Vec::new();
        for l in &self.loops {
            let n = l.points.len();
            for i in 0..n {
                let (a, b) = (l.points[i], l.points[(i + 1) % n]);
                let e = b - a;
                let den = dir.perp_dot(e);
                if den.abs() < 1e-12 {
                    continue;
                }
                // The side from its first point up to, not including, its last: a corner counts once.
                let s = (a - origin).perp_dot(dir) / den;
                if !(0.0..1.0).contains(&s) {
                    continue;
                }
                let mut t = (a - origin).perp_dot(e) / den;
                // On a chord of a circle: where the line meets the circle itself.
                if let Some((c, r)) = l.circles[i] {
                    let along = (c - origin).dot(dir);
                    let off = (c - origin).perp_dot(dir);
                    let h = r * r - off * off;
                    if h >= 0.0 {
                        let (t0, t1) = (along - h.sqrt(), along + h.sqrt());
                        t = if (t0 - t).abs() < (t1 - t).abs() { t0 } else { t1 };
                    }
                }
                out.push(Crossing { t, outer: l.outer });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /// The stretches of the line through `origin` along the unit `dir` that lie inside the region.
    pub fn stretches(&self, origin: DVec2, dir: DVec2) -> Vec<Stretch> {
        let c = self.crossings(origin, dir);
        c.as_chunks::<2>()
            .0
            .iter()
            .map(|p| Stretch { from: p[0].t, to: p[1].t, edge: (p[0].outer, p[1].outer) })
            .filter(|s| s.to - s.from > 1e-9)
            .collect()
    }

    /// Whether `p` lies inside the region.
    pub fn contains(&self, p: DVec2) -> bool {
        // A direction no side is likely to be parallel to.
        let dir = DVec2::from_angle(0.61);
        self.crossings(p, dir).iter().filter(|c| c.t > 0.0).count() % 2 == 1
    }

    /// The parts of the circle around `center` inside the region, as
    /// `(start angle, sweep)`; a whole turn is one part with a sweep of τ.
    pub fn arcs(&self, center: DVec2, radius: f64) -> Vec<(f64, f64)> {
        let mut angles: Vec<f64> = Vec::new();
        for l in &self.loops {
            let n = l.points.len();
            for i in 0..n {
                let (a, b) = (l.points[i], l.points[(i + 1) % n]);
                let e = b - a;
                let len2 = e.length_squared();
                if len2 < 1e-18 {
                    continue;
                }
                // |a + s e - center| = radius
                let f = a - center;
                let (qa, qb, qc) = (len2, 2.0 * f.dot(e), f.length_squared() - radius * radius);
                let disc = qb * qb - 4.0 * qa * qc;
                if disc <= 0.0 {
                    continue;
                }
                for s in [(-qb - disc.sqrt()) / (2.0 * qa), (-qb + disc.sqrt()) / (2.0 * qa)] {
                    if (0.0..1.0).contains(&s) {
                        let p = f + e * s;
                        angles.push(p.y.atan2(p.x).rem_euclid(TAU));
                    }
                }
            }
        }
        let at = |angle: f64| center + DVec2::from_angle(angle) * radius;
        if angles.is_empty() {
            return if self.contains(at(0.0)) { vec![(0.0, TAU)] } else { Vec::new() };
        }
        angles.sort_by(f64::total_cmp);
        let mut out: Vec<(f64, f64)> = Vec::new();
        for i in 0..angles.len() {
            let from = angles[i];
            let sweep = if i + 1 < angles.len() { angles[i + 1] - from } else { angles[0] + TAU - from };
            if sweep < 1e-9 || !self.contains(at(from + sweep * 0.5)) {
                continue;
            }
            // Two parts met at a crossing that only touched the edge: one bar.
            match out.last_mut() {
                Some(last) if (last.0 + last.1 - from).abs() < 1e-9 => last.1 += sweep,
                _ => out.push((from, sweep)),
            }
        }
        out
    }
}

/// The pieces of a bar `length` long: whole, or cut by the splice.
fn pieces(splice: Option<Splice>, length: f64, index: usize) -> Vec<(f64, f64)> {
    splice.map_or_else(|| vec![(0.0, length)], |s| s.pieces(length, index))
}

/// The bars of the set `r` (bars `d` thick) in `slab`; `other` is the
/// largest bar of the other sets of its layer, which an `inner` set lies inside.
pub(crate) fn slab_bars(slab: &Slab, r: &Rebar, d: f64, other: f64) -> Vec<Bar> {
    // Trimming bars lie right on the line the cover draws around an opening:
    // their region stops a hair short of it, so that line is inside.
    let hair = if matches!(r.layout, RebarLayout::SlabTrim { .. }) { 1e-6 } else { 0.0 };
    let region = Region::new(slab, (r.cover + d / 2.0 - hair).max(0.0));
    if region.is_empty() {
        return Vec::new();
    }
    // The height of the layer: the cover and half a bar inside the face, an inner set a bar further.
    let height = |top: bool, inner: bool| {
        let inset = r.cover + d / 2.0 + if inner { other } else { 0.0 };
        let middle = (slab.z.0 + slab.z.1) * 0.5;
        if top { (slab.z.1 - inset).max(middle) } else { (slab.z.0 + inset).min(middle) }
    };
    let mut bars: Vec<Bar> = Vec::new();
    // A straight bar from `a` to `b`, cut by the splice, with a leg at the ends on the slab's edge.
    let straight = |bars: &mut Vec<Bar>, a: DVec2, b: DVec2, z: f64, index: usize, legs: (f64, f64)| {
        let length = a.distance(b);
        if length < MIN_BAR {
            return;
        }
        let dir = (b - a) / length;
        let cuts = pieces(r.splice, length, index);
        let last = cuts.len().saturating_sub(1);
        for (i, (from, to)) in cuts.into_iter().enumerate() {
            if bars.len() >= MAX_BARS {
                return;
            }
            let mut points = Vec::with_capacity(4);
            if i == 0 && legs.0 != 0.0 {
                points.push((a + dir * from).extend(z + legs.0));
            }
            points.push((a + dir * from).extend(z));
            points.push((a + dir * to).extend(z));
            if i == last && legs.1 != 0.0 {
                points.push((a + dir * to).extend(z + legs.1));
            }
            bars.push(Bar::new(points, false));
        }
    };
    match r.layout {
        RebarLayout::SlabStraight { top, angle_deg, spacing, inner, hook } => {
            let z = height(top, inner);
            let dir = DVec2::from_angle(angle_deg.to_radians());
            let across = dir.perp();
            let middle = slab.middle();
            // Lines either side of the one through the middle of the slab.
            let reach = slab.outline.iter().map(|p| (*p - middle).dot(across));
            let (lo, hi) = reach.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
            let (first, last) = ((lo / spacing).ceil() as i64, (hi / spacing).floor() as i64);
            // A positive hook turns towards the middle of the slab: down from a top bar.
            let leg = if top { -hook } else { hook };
            for (index, k) in (first..=last).take(MAX_LINES).enumerate() {
                let origin = middle + across * (k as f64 * spacing);
                for s in region.stretches(origin, dir) {
                    let legs = (if s.edge.0 { leg } else { 0.0 }, if s.edge.1 { leg } else { 0.0 });
                    straight(&mut bars, origin + dir * s.from, origin + dir * s.to, z, index, legs);
                }
            }
        }
        RebarLayout::SlabRing { top, spacing, center, inner } => {
            let z = height(top, inner);
            let center = center.unwrap_or_else(|| slab.middle());
            let far = slab.outline.iter().map(|p| p.distance(center)).fold(0.0, f64::max);
            let count = ((far / spacing).floor() as usize).min(MAX_LINES);
            for index in 1..=count {
                let radius = index as f64 * spacing;
                for (start, sweep) in region.arcs(center, radius) {
                    let whole = (sweep - TAU).abs() < 1e-9;
                    let length = radius * sweep;
                    // A whole ring closes on itself; cut from stock, each piece laps the next around.
                    let cuts: Vec<(f64, f64)> = match (whole, r.splice.filter(Splice::is_valid)) {
                        (true, Some(s)) => {
                            let n = (length / (s.max_length - s.lap) - 1e-9).ceil().max(1.0);
                            let pitch = length / n;
                            let shift = if index % 2 == 1 { pitch * 0.5 } else { 0.0 };
                            (0..n as usize)
                                .map(|i| (shift + pitch * i as f64, shift + pitch * (i + 1) as f64 + s.lap))
                                .collect()
                        }
                        (true, None) => vec![(0.0, length)],
                        (false, _) => pieces(r.splice, length, index),
                    };
                    let closed = whole && r.splice.is_none();
                    for (from, to) in cuts {
                        if bars.len() >= MAX_BARS || to - from < MIN_BAR {
                            continue;
                        }
                        let (a0, a1) = (start + from / radius, start + to / radius);
                        let n = segments(a1 - a0, ARC_STEP);
                        // A closed ring's last point is its first: left out.
                        let steps = if closed { n.saturating_sub(1) } else { n };
                        let points: Vec<DVec3> = (0..=steps)
                            .map(|i| {
                                (center + DVec2::from_angle(a0 + (a1 - a0) * i as f64 / n as f64) * radius).extend(z)
                            })
                            .collect();
                        bars.push(Bar { true_length: Some(to - from), ..Bar::new(points, closed) });
                    }
                }
            }
        }
        RebarLayout::SlabRadial { top, count, center, start, intermediate, inner } => {
            let z = height(top, inner);
            let center = center.unwrap_or_else(|| slab.middle());
            let step = TAU / count as f64;
            let mut index = 0;
            for k in 0..count {
                // A full bar, and with an intermediate radius a shorter one half a step on.
                for (angle, from) in [(k as f64 * step, start), ((k as f64 + 0.5) * step, intermediate)] {
                    if from <= 0.0 && angle != k as f64 * step {
                        continue;
                    }
                    let dir = DVec2::from_angle(angle);
                    for s in region.stretches(center, dir) {
                        let a = s.from.max(from);
                        if s.to > a {
                            straight(&mut bars, center + dir * a, center + dir * s.to, z, index, (0.0, 0.0));
                        }
                    }
                    index += 1;
                }
            }
        }
        RebarLayout::SlabTrim { top, count, spacing, anchorage, diagonals } => {
            let z = height(top, true);
            for hole in slab.holes.iter().take(MAX_LINES) {
                let lo = hole.iter().fold(DVec2::INFINITY, |m, p| m.min(*p));
                let hi = hole.iter().fold(DVec2::NEG_INFINITY, |m, p| m.max(*p));
                let clear = r.cover + d / 2.0;
                // Along each side of the opening's box, the bars one after another away from it.
                let sides = [
                    (DVec2::new(lo.x, lo.y), DVec2::X, -DVec2::Y, hi.x - lo.x),
                    (DVec2::new(lo.x, hi.y), DVec2::X, DVec2::Y, hi.x - lo.x),
                    (DVec2::new(lo.x, lo.y), DVec2::Y, -DVec2::X, hi.y - lo.y),
                    (DVec2::new(hi.x, lo.y), DVec2::Y, DVec2::X, hi.y - lo.y),
                ];
                for (corner, dir, out, length) in sides {
                    for j in 0..count {
                        let origin = corner + out * (clear + spacing * j as f64);
                        let (a, b) = (-anchorage, length + anchorage);
                        for s in region.stretches(origin, dir) {
                            let (from, to) = (s.from.max(a), s.to.min(b));
                            if to > from {
                                straight(&mut bars, origin + dir * from, origin + dir * to, z, 0, (0.0, 0.0));
                            }
                        }
                    }
                }
                if diagonals {
                    for corner in [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)] {
                        // Across the corner, square to the line from the opening's middle through it.
                        let Some(out) = (corner - (lo + hi) * 0.5).try_normalize() else { continue };
                        let (origin, dir) = (corner + out * (clear * std::f64::consts::SQRT_2), out.perp());
                        for s in region.stretches(origin, dir) {
                            let (from, to) = (s.from.max(-anchorage), s.to.min(anchorage));
                            if to > from {
                                straight(&mut bars, origin + dir * from, origin + dir * to, z, 0, (0.0, 0.0));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disc(radius: f64) -> Slab {
        let n = 120;
        let outline = (0..n).map(|i| DVec2::from_angle(i as f64 / n as f64 * TAU) * radius).collect();
        Slab { outline, holes: Vec::new(), circles: vec![(DVec2::ZERO, radius)], z: (0.0, 0.2) }
    }

    fn square(side: f64) -> Slab {
        let h = side / 2.0;
        let outline = vec![DVec2::new(-h, -h), DVec2::new(h, -h), DVec2::new(h, h), DVec2::new(-h, h)];
        Slab { outline, holes: Vec::new(), circles: Vec::new(), z: (0.0, 0.2) }
    }

    #[test]
    fn a_line_across_a_disc_ends_on_the_circle_not_its_chords() {
        let region = Region::new(&disc(2.0), 0.05);
        for y in [0.0, 0.3, 1.0, 1.7, 1.9] {
            let s = region.stretches(DVec2::new(0.0, y), DVec2::X);
            assert_eq!(s.len(), 1, "{y}");
            let half = (1.95f64 * 1.95 - y * y).sqrt();
            assert!((s[0].from + half).abs() < 1e-12 && (s[0].to - half).abs() < 1e-12, "{y}: {:?}", s[0]);
            assert_eq!(s[0].edge, (true, true));
        }
        // Outside the cover: nothing.
        assert!(region.stretches(DVec2::new(0.0, 1.97), DVec2::X).is_empty());
        assert!(region.contains(DVec2::new(1.0, 1.0)) && !region.contains(DVec2::new(1.5, 1.5)));
        // A cover as deep as the slab leaves nothing.
        assert!(Region::new(&disc(2.0), 2.5).is_empty());
    }

    #[test]
    fn openings_cut_lines_and_circles() {
        let mut slab = square(4.0);
        slab.holes.push(vec![
            DVec2::new(-0.3, -0.3),
            DVec2::new(0.3, -0.3),
            DVec2::new(0.3, 0.3),
            DVec2::new(-0.3, 0.3),
        ]);
        let region = Region::new(&slab, 0.05);
        // Through the opening: two stretches, the cover clear of it, their inner ends not on the slab's edge.
        let s = region.stretches(DVec2::new(0.0, 0.1), DVec2::X);
        assert_eq!(s.len(), 2);
        assert!((s[0].from + 1.95).abs() < 1e-12 && (s[0].to + 0.35).abs() < 1e-12 && (s[1].from - 0.35).abs() < 1e-12);
        assert_eq!((s[0].edge, s[1].edge), ((true, false), (false, true)));
        // Beside it: one.
        assert_eq!(region.stretches(DVec2::new(0.0, 1.0), DVec2::X).len(), 1);
        // A circle around the opening is whole; one through the corners of the slab is four arcs.
        assert_eq!(region.arcs(DVec2::ZERO, 1.0), vec![(0.0, TAU)]);
        let arcs = region.arcs(DVec2::ZERO, 2.2);
        assert_eq!(arcs.len(), 4);
        let sweep = (1.95f64 / 2.2).asin() - (1.95f64 / 2.2).acos();
        assert!(arcs.iter().all(|(_, s)| (s - sweep).abs() < 1e-9), "{arcs:?}");
        // Inside the opening and outside the slab: none.
        assert!(region.arcs(DVec2::ZERO, 0.2).is_empty() && region.arcs(DVec2::ZERO, 3.0).is_empty());
    }
}
