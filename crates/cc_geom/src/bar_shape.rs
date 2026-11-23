//! The shape of a bar as a bending schedule describes it: open or closed,
//! its legs in order with their lengths, the radii and turns of arc legs
//! and the bends between legs, read from the bar's center line (and the
//! hooks that close a stirrup or end a tie, as legs marked `hook`). Independent
//! of where the bar lies, so bars of one shape compare equal wherever they
//! are ([`ShapeSummary::matches`]).

use crate::rebar::{Bar, EndHooks, LoopHooks};
use glam::{DVec2, DVec3};

/// Segments shorter than this are no segments.
const MIN_SEGMENT: f64 = 1e-7;
/// Turns smaller than this are none: the segments make one straight leg, radians.
const STRAIGHT: f64 = 1e-4;
/// Chords of an arc turn by at most this at each point, radians (round ties
/// are drawn with 16 chords).
const ARC_TURN: f64 = 25.0 * std::f64::consts::PI / 180.0;
/// Two chords alone make an arc only when they turn by at most this, radians.
const SHORT_ARC_TURN: f64 = 6.5 * std::f64::consts::PI / 180.0;
/// At most this many points of a bar are read; the rest of a longer one is left out.
const MAX_POINTS: usize = 4096;

/// One leg of a [`ShapeSummary`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSummary {
    /// Length along the leg, meters (a stirrup's hook leg: its straight end).
    pub length: f64,
    /// Center-line radius of an arc leg; none for a straight one.
    pub radius: Option<f64>,
    /// Angle an arc leg turns through, radians (0 for a straight leg),
    /// signed like the bends.
    pub sweep: f64,
    /// Turn at the corner after this leg, radians: positive the way the bar
    /// first turns (along an arc or at a corner), negative the other way; 0
    /// after the last leg of an open bar (a closed bar's last leg ends at
    /// its closing corner). Bars that do not lie in a plane take the sign
    /// of the turn's axis against the first.
    pub bend: f64,
    /// The leg is part of a hook.
    pub hook: bool,
    /// The rise per turn of a coiled leg (a spiral, whose sweep is its
    /// turns), meters; 0 for a leg in a plane.
    pub pitch: f64,
}

/// The shape of a bar: see the module.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapeSummary {
    pub closed: bool,
    pub legs: Vec<LegSummary>,
}

impl ShapeSummary {
    /// Length of the bar.
    pub fn length(&self) -> f64 {
        self.legs.iter().map(|l| l.length).sum()
    }

    /// Whether the bar is one straight leg.
    pub fn is_straight(&self) -> bool {
        !self.closed && self.legs.len() == 1 && self.legs.iter().all(|l| l.radius.is_none())
    }

    /// The same shape read from its other end.
    pub fn reversed(&self) -> ShapeSummary {
        let n = self.legs.len();
        let closing = if self.closed { self.legs.last().map_or(0.0, |l| -l.bend) } else { 0.0 };
        let legs = self
            .legs
            .iter()
            .rev()
            .enumerate()
            .map(|(j, old)| {
                // The corner after the new leg `j` is the one before the old leg `n - 1 - j`.
                let bend = match (n - 1 - j).checked_sub(1).and_then(|k| self.legs.get(k)) {
                    Some(before) => -before.bend,
                    None => closing,
                };
                LegSummary { sweep: -old.sweep, bend, ..*old }
            })
            .collect();
        ShapeSummary { closed: self.closed, legs }
    }

    /// Whether `other` is the same shape: as many legs, of lengths within
    /// `length_tol` and radii alike, turns within `angle_tol`, read from
    /// either end, either way round (mirrored), and a closed bar from any
    /// corner.
    pub fn matches(&self, other: &ShapeSummary, length_tol: f64, angle_tol: f64) -> bool {
        let n = self.legs.len();
        if self.closed != other.closed || n != other.legs.len() {
            return false;
        }
        let same = |a: &LegSummary, b: &LegSummary, sign: f64| {
            (a.length - b.length).abs() <= length_tol
                && match (a.radius, b.radius) {
                    (Some(x), Some(y)) => (x - y).abs() <= length_tol,
                    (None, None) => true,
                    _ => false,
                }
                && (a.sweep - sign * b.sweep).abs() <= angle_tol
                && (a.bend - sign * b.bend).abs() <= angle_tol
                && a.hook == b.hook
                && (a.pitch - b.pitch).abs() <= length_tol
        };
        // A closed bar may start at any of its corners (bounded: long ones from their first only).
        let shifts = if self.closed && n <= 64 { n.max(1) } else { 1 };
        let reversed = other.reversed();
        [other, &reversed].into_iter().any(|o| {
            (0..shifts).any(|k| {
                [1.0, -1.0].into_iter().any(|sign| {
                    (0..n).all(|i| match (self.legs.get(i), o.legs.get((i + k) % n.max(1))) {
                        (Some(a), Some(b)) => same(a, b, sign),
                        _ => false,
                    })
                })
            })
        })
    }
}

/// The shape of `bar`, read from its center line. Points in a line make one
/// straight leg; equal chords turning evenly by a small angle make one arc
/// leg, with the radius and the length of the arc through their points. A
/// bar whose true length differs from its center line's (the hooks of a
/// tie, counted but not drawn) has the difference spread over its legs. A
/// loop closed by hooks (a stirrup's [`Bar::hooks`]) is read from the corner
/// they close it at, and opened there by a hook leg at each end. A spiral
/// ([`Bar::helix`]) is one coiled leg: its radius, its turns as the sweep,
/// its pitch and its exact length. An open bar with hooks at its ends
/// ([`Bar::end_hooks`], a tie) has a hook leg before and after its center
/// line's legs.
pub fn summary(bar: &Bar) -> ShapeSummary {
    if let Some(ends) = bar.end_hooks.filter(|_| !bar.closed) {
        // The center line without the hooks its true length counts.
        let line = summary(&Bar { true_length: None, end_hooks: None, ..bar.clone() });
        return with_end_hooks(line, &ends);
    }
    if let Some(h) = bar.helix.filter(|h| h.is_valid() && !bar.closed) {
        let leg = LegSummary {
            length: bar.length(),
            radius: Some(h.radius),
            sweep: std::f64::consts::TAU * h.turns,
            bend: 0.0,
            hook: false,
            pitch: h.pitch,
        };
        return ShapeSummary { closed: false, legs: vec![leg] };
    }
    let mut pts: Vec<DVec3> = Vec::with_capacity(bar.points.len().min(MAX_POINTS));
    for p in bar.points.iter().take(MAX_POINTS).filter(|p| p.is_finite()) {
        if pts.last().is_none_or(|q| q.distance(*p) > MIN_SEGMENT) {
            pts.push(*p);
        }
    }
    let mut closed = bar.closed;
    if closed && pts.len() > 1 && pts.first().zip(pts.last()).is_some_and(|(a, b)| a.distance(*b) <= MIN_SEGMENT) {
        pts.pop();
    }
    if pts.len() < 3 {
        closed = false;
    }
    if pts.len() < 2 {
        return ShapeSummary::default();
    }
    // Segments, and the turn after each (none after the last of an open bar).
    let count = if closed { pts.len() } else { pts.len() - 1 };
    let seg = |i: usize| -> DVec3 {
        match (pts.get(i % pts.len()), pts.get((i + 1) % pts.len())) {
            (Some(a), Some(b)) => *b - *a,
            _ => DVec3::ZERO,
        }
    };
    let mut lengths: Vec<f64> = (0..count).map(|i| seg(i).length()).collect();
    let mut axis: Option<DVec3> = None;
    let mut turns: Vec<f64> = (0..count)
        .map(|i| {
            if !closed && i + 1 == count {
                return 0.0;
            }
            let (a, b) = (seg(i).normalize_or_zero(), seg(i + 1).normalize_or_zero());
            let cross = a.cross(b);
            let angle = cross.length().atan2(a.dot(b));
            if angle < STRAIGHT {
                return 0.0;
            }
            // Signed against the first turn's axis: positive the way the bar first turns.
            let first = *axis.get_or_insert(cross);
            if cross.dot(first) >= 0.0 { angle } else { -angle }
        })
        .collect();
    // Segments in a line are one: each takes in those it runs on into.
    let mut i = 0;
    while i < lengths.len() {
        let last_open = !closed && i + 1 == lengths.len();
        if lengths.len() > 1 && !last_open && turns.get(i).is_some_and(|t| *t == 0.0) {
            let next = (i + 1) % lengths.len();
            let (len, turn) = (lengths.get(next).copied().unwrap_or(0.0), turns.get(next).copied().unwrap_or(0.0));
            if let (Some(l), Some(t)) = (lengths.get_mut(i), turns.get_mut(i)) {
                *l += len;
                *t = turn;
            }
            lengths.remove(next);
            turns.remove(next);
            if next < i {
                i -= 1;
            }
            if lengths.len() == 1 {
                break;
            }
        } else {
            i += 1;
        }
    }
    let n = lengths.len();
    if n == 1 {
        // A closed line folded onto itself is no loop.
        let leg =
            LegSummary { length: lengths.iter().sum(), radius: None, sweep: 0.0, bend: 0.0, hook: false, pitch: 0.0 };
        return scaled(ShapeSummary { closed: false, legs: vec![leg] }, bar.true_length);
    }
    // A point where the bar runs on along an arc: equal chords either side, a small turn.
    let joint = |i: usize| !(!closed && i + 1 >= n);
    let even = |a: f64, b: f64| (a - b).abs() <= 1e-3 * a.max(b).max(1e-9);
    let smooth = |i: usize| {
        let (Some(a), Some(b), Some(t)) = (lengths.get(i), lengths.get((i + 1) % n), turns.get(i)) else {
            return false;
        };
        joint(i) && t.abs() > 0.0 && t.abs() <= ARC_TURN && even(*a, *b)
    };
    let alike = |i: usize, j: usize| match (turns.get(i), turns.get(j)) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-3,
        _ => false,
    };
    // A closed bar is read from after a corner, so that no arc runs across
    // its start: from its first point when that is one (where hooks close it).
    let hooks = bar.hooks.filter(|_| closed);
    let start = match (closed, hooks.is_some() && !smooth(n - 1)) {
        (true, false) => (0..n).find(|i| !smooth(*i)).map_or(0, |i| (i + 1) % n),
        _ => 0,
    };
    let at = |k: usize| (start + k) % n;
    let mut legs = Vec::new();
    let mut k = 0;
    while k < n {
        let first = at(k);
        // The chords that follow on the same arc.
        let mut run = 0;
        while k + run + 1 < n && smooth(at(k + run)) && alike(at(k + run), first) {
            run += 1;
        }
        let turn = turns.get(first).copied().unwrap_or(0.0);
        let chord = lengths.get(first).copied().unwrap_or(0.0);
        let whole = closed && run + 1 == n && smooth(at(k + run)) && alike(at(k + run), first);
        if run >= 2 || (run == 1 && turn.abs() <= SHORT_ARC_TURN) || whole {
            let chords = run + 1;
            let radius = chord / (2.0 * (turn.abs() / 2.0).sin()).max(1e-12);
            // Each chord spans one turn of the arc; a ring turns once around.
            let sweep = if whole { turn.signum() * std::f64::consts::TAU } else { turn * chords as f64 };
            let last = at(k + run);
            let bend = if whole || !joint(last) { 0.0 } else { turns.get(last).copied().unwrap_or(0.0) };
            legs.push(LegSummary {
                length: radius * sweep.abs(),
                radius: Some(radius),
                sweep,
                bend,
                hook: false,
                pitch: 0.0,
            });
            k += chords;
        } else {
            let bend = if joint(first) { turn } else { 0.0 };
            legs.push(LegSummary { length: chord, radius: None, sweep: 0.0, bend, hook: false, pitch: 0.0 });
            k += 1;
        }
    }
    let shape = scaled(ShapeSummary { closed, legs }, bar.true_length);
    match hooks {
        Some(h) => hooked(shape, &h),
        None => shape,
    }
}

/// A closed loop opened at its closing corner by two hooks: an open shape,
/// a hook leg (its straight end), the loop's legs and the other hook leg,
/// each hook bent by its angle the way the loop turns. A hook's bend around
/// its pin is no leg, so the legs add up to the bar's length less the bends.
fn hooked(mut shape: ShapeSummary, hooks: &LoopHooks) -> ShapeSummary {
    if !shape.closed || shape.legs.is_empty() {
        return shape;
    }
    // The way the loop turns: its turns add up to a whole turn either way.
    let total: f64 = shape.legs.iter().map(|l| l.bend + l.sweep).sum();
    let way = if total < 0.0 { -1.0 } else { 1.0 };
    let bend = way * hooks.angle;
    let hook = |bend: f64| LegSummary { length: hooks.tail, radius: None, sweep: 0.0, bend, hook: true, pitch: 0.0 };
    if let Some(last) = shape.legs.last_mut() {
        last.bend = bend;
    }
    let mut legs = Vec::with_capacity(shape.legs.len() + 2);
    legs.push(hook(bend));
    legs.append(&mut shape.legs);
    legs.push(hook(0.0));
    ShapeSummary { closed: false, legs }
}

/// An open bar with a hook at each end: a hook leg bent by its angle into
/// the bar's first leg, the bar's legs, the last bent by the end hook's
/// angle (the same way as the first, or the other way for an S), and the
/// end hook leg. As in a stirrup, a hook's bend around its pin is no leg.
fn with_end_hooks(mut shape: ShapeSummary, ends: &EndHooks) -> ShapeSummary {
    if shape.closed || shape.legs.is_empty() {
        return shape;
    }
    // The way the bar first turns, if it does; else either.
    let way = shape.legs.iter().map(|l| l.bend + l.sweep).find(|t| t.abs() > 0.0).map_or(1.0, f64::signum);
    let start = way * ends.start.angle;
    let end = if ends.opposite { -way } else { way } * ends.end.angle;
    let hook = |h: &LoopHooks, bend: f64| LegSummary {
        length: h.tail,
        radius: None,
        sweep: 0.0,
        bend,
        hook: true,
        pitch: 0.0,
    };
    if let Some(last) = shape.legs.last_mut() {
        last.bend = end;
    }
    let mut legs = Vec::with_capacity(shape.legs.len() + 2);
    legs.push(hook(&ends.start, start));
    legs.append(&mut shape.legs);
    legs.push(hook(&ends.end, 0.0));
    ShapeSummary { closed: false, legs }
}

/// Whether `legs`, laid one after another with their bends, end where they
/// start (within a twentieth of their length).
fn closes(legs: &[LegSummary]) -> bool {
    let (mut at, mut heading, mut total) = (DVec2::ZERO, 0.0_f64, 0.0);
    for l in legs {
        match l.radius.filter(|_| l.sweep != 0.0) {
            Some(r) => {
                let chord = 2.0 * r * (l.sweep.abs() / 2.0).sin();
                at += DVec2::from_angle(heading + l.sweep / 2.0) * chord;
                heading += l.sweep;
            }
            None => at += DVec2::from_angle(heading) * l.length,
        }
        heading += l.bend;
        total += l.length;
    }
    total > 0.0 && at.length() <= 0.05 * total
}

impl ShapeSummary {
    /// The loop a stirrup's hooks close, when the shape is one (a hook leg,
    /// legs of no hook that close on themselves, a hook leg): those legs as
    /// a closed shape, the turn at its closing corner being what the others
    /// leave of a whole turn; and the two hook legs, each with the bend of
    /// its hook as `bend`.
    pub fn hooked_loop(&self) -> Option<(ShapeSummary, [LegSummary; 2])> {
        let ([first, middle @ .., last], false) = (self.legs.as_slice(), self.closed) else { return None };
        if !(first.hook && last.hook) || middle.is_empty() || middle.iter().any(|l| l.hook) || !closes(middle) {
            return None;
        }
        let mut legs = middle.to_vec();
        let n = legs.len();
        let others: f64 = legs.iter().take(n - 1).map(|l| l.bend).chain(legs.iter().map(|l| l.sweep)).sum();
        let way = if first.bend < 0.0 { -1.0 } else { 1.0 };
        let mut end = *last;
        if let Some(l) = legs.last_mut() {
            end.bend = l.bend;
            let closing = way * std::f64::consts::TAU - others;
            l.bend = if closing.abs() < 1e-9 { 0.0 } else { closing };
        }
        Some((ShapeSummary { closed: true, legs }, [*first, end]))
    }

    /// The bar a tie's hooks end, when the shape is one (a hook leg, legs of
    /// no hook that do not close, a hook leg): those legs as an open shape,
    /// its last leg bent no more; and the two hook legs, each with the bend
    /// of its hook as `bend`.
    pub fn hooked_ends(&self) -> Option<(ShapeSummary, [LegSummary; 2])> {
        let ([first, middle @ .., last], false) = (self.legs.as_slice(), self.closed) else { return None };
        if !(first.hook && last.hook) || middle.is_empty() || middle.iter().any(|l| l.hook) || closes(middle) {
            return None;
        }
        let mut legs = middle.to_vec();
        let mut end = *last;
        if let Some(l) = legs.last_mut() {
            end.bend = l.bend;
            l.bend = 0.0;
        }
        Some((ShapeSummary { closed: false, legs }, [*first, end]))
    }
}

/// `shape` with its legs adding up to `true_length`, when the bar has one:
/// the difference goes to its arc legs (their chords measure them short),
/// or to every leg of a bar without arcs (hooks counted but not drawn).
fn scaled(mut shape: ShapeSummary, true_length: Option<f64>) -> ShapeSummary {
    let Some(want) = true_length.filter(|l| l.is_finite() && *l > 0.0) else { return shape };
    let total = shape.length();
    if (want - total).abs() <= 1e-9 || total <= 0.0 {
        return shape;
    }
    let arcs: f64 = shape.legs.iter().filter(|l| l.radius.is_some()).map(|l| l.length).sum();
    let straight = total - arcs;
    if arcs > 0.0 && want > straight {
        let k = (want - straight) / arcs;
        shape.legs.iter_mut().filter(|l| l.radius.is_some()).for_each(|l| l.length *= k);
    } else {
        let k = want / total;
        shape.legs.iter_mut().for_each(|l| l.length *= k);
    }
    shape
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn v(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3::new(x, y, z)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn straight_bars_and_bends() {
        // Points in a line are one leg.
        let s = summary(&Bar::new(vec![v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(3.0, 0.0, 0.0)], false));
        assert!(s.is_straight() && close(s.length(), 3.0));
        // An L, and a Z whose second bend turns the other way.
        let l = summary(&Bar::new(vec![v(0.0, 0.0, 0.3), v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0)], false));
        assert_eq!(l.legs.len(), 2);
        assert!(close(l.legs[0].length, 0.3) && close(l.legs[0].bend, FRAC_PI_2) && close(l.legs[1].bend, 0.0));
        let z =
            summary(&Bar::new(vec![v(0.0, 0.0, 0.3), v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0), v(4.0, 0.0, -0.3)], false));
        assert!(close(z.legs[0].bend, FRAC_PI_2) && close(z.legs[1].bend, -FRAC_PI_2));
        // The same shapes elsewhere, read from the other end or mirrored, match.
        let back = summary(&Bar::new(vec![v(9.0, 5.0, 1.0), v(5.0, 5.0, 1.0), v(5.0, 5.3, 1.0)], false));
        assert!(l.matches(&back, 1e-6, 1e-6) && !l.matches(&z, 1e-6, 1e-6));
        assert!(!l.matches(
            &summary(&Bar::new(vec![v(0.0, 0.0, 0.31), v(0.0, 0.0, 0.0), v(4.0, 0.0, 0.0)], false)),
            0.005,
            0.01
        ));
        // Nothing to read: no legs.
        assert!(summary(&Bar::new(vec![v(1.0, 1.0, 1.0)], false)).legs.is_empty());
        assert!(summary(&Bar::new(vec![v(f64::NAN, 0.0, 0.0), v(1.0, 0.0, 0.0)], false)).legs.is_empty());
    }

    #[test]
    fn stirrups_close_on_their_corners() {
        let loop4 = vec![v(0.0, 0.0, 0.0), v(0.1, 0.0, 0.0), v(0.1, 0.0, 0.3), v(0.0, 0.0, 0.3)];
        let s = summary(&Bar::new(loop4.clone(), true));
        assert!(s.closed && s.legs.len() == 4 && close(s.length(), 0.8));
        assert!(s.legs.iter().all(|l| close(l.bend, FRAC_PI_2) && l.radius.is_none()));
        // From another corner and the other way round it is the same stirrup; a wider one is not.
        let mut other = loop4.clone();
        other.rotate_left(1);
        other.reverse();
        assert!(s.matches(&summary(&Bar::new(other, true)), 1e-6, 1e-6));
        let wide = vec![v(0.0, 0.0, 0.0), v(0.2, 0.0, 0.0), v(0.2, 0.0, 0.2), v(0.0, 0.0, 0.2)];
        assert!(!s.matches(&summary(&Bar::new(wide, true)), 0.005, 0.01));
    }

    #[test]
    fn chords_read_as_arcs() {
        // A quarter circle of radius 2.5 m in chords of 3 degrees.
        let arc = |r: f64, sweep: f64, n: usize| -> Vec<DVec3> {
            (0..=n).map(|i| (glam::DVec2::from_angle(sweep * i as f64 / n as f64) * r).extend(1.0)).collect()
        };
        let s = summary(&Bar {
            points: arc(2.5, FRAC_PI_2, 30),
            closed: false,
            true_length: Some(2.5 * FRAC_PI_2),
            hooks: None,
            helix: None,
            end_hooks: None,
        });
        assert_eq!(s.legs.len(), 1);
        let leg = s.legs[0];
        assert!(close(leg.radius.unwrap(), 2.5) && close(leg.length, 2.5 * FRAC_PI_2), "{leg:?}");
        assert!((leg.sweep - FRAC_PI_2).abs() < 1e-9 && close(leg.bend, 0.0));
        // Another radius is another shape; the same arc elsewhere, turning the other way, is the same.
        let other = summary(&Bar {
            points: arc(2.6, FRAC_PI_2, 30),
            closed: false,
            true_length: None,
            hooks: None,
            helix: None,
            end_hooks: None,
        });
        assert!(!s.matches(&other, 0.005, 0.01));
        let mirrored: Vec<DVec3> = arc(2.5, FRAC_PI_2, 30).into_iter().map(|p| v(p.x + 3.0, -p.y, 0.0)).collect();
        assert!(s.matches(&summary(&Bar::new(mirrored, false)), 1e-6, 1e-6));
        // A closed ring: one leg, a whole turn, the circle's length.
        let ring: Vec<DVec3> = arc(1.0, TAU, 120).into_iter().take(120).collect();
        let r = summary(&Bar {
            points: ring,
            closed: true,
            true_length: Some(TAU),
            hooks: None,
            helix: None,
            end_hooks: None,
        });
        assert!(r.closed && r.legs.len() == 1 && close(r.legs[0].radius.unwrap(), 1.0) && close(r.length(), TAU));
        // A round tie drawn with 16 chords is a ring too.
        let tie: Vec<DVec3> = arc(0.2, TAU, 16).into_iter().take(16).collect();
        let t = summary(&Bar::new(tie, true));
        assert!(t.legs.len() == 1 && close(t.legs[0].radius.unwrap(), 0.2) && close(t.length(), 0.4 * PI));
        // A straight leg, then an arc: two legs.
        let mut hooked = vec![v(2.5, -1.0, 1.0)];
        hooked.extend(arc(2.5, FRAC_PI_2, 30));
        let h = summary(&Bar::new(hooked, false));
        assert!(h.legs.len() == 2 && h.legs[0].radius.is_none() && close(h.legs[0].length, 1.0), "{h:?}");
        assert!(close(h.legs[1].radius.unwrap(), 2.5));
    }

    #[test]
    fn hooks_counted_in_the_length_are_spread_over_the_legs() {
        // A tie drawn as its straight part, 0.15 m of hooks counted in its length.
        let tie = Bar {
            points: vec![v(0.0, 0.0, 0.0), v(0.2, 0.0, 0.0)],
            closed: false,
            true_length: Some(0.35),
            hooks: None,
            helix: None,
            end_hooks: None,
        };
        let s = summary(&tie);
        assert!(s.is_straight() && close(s.length(), 0.35));
    }

    #[test]
    fn a_tie_has_a_hook_leg_at_each_end() {
        // A 5 mm tie 0.20 m between the bars it holds: 135° at its start, 180° at its end.
        let ends = EndHooks::of_tie(0.75 * PI, PI, 0.005, false);
        let tie = Bar {
            true_length: Some(0.2 + ends.length()),
            end_hooks: Some(ends),
            ..Bar::new(vec![v(0.0, 0.0, 0.0), v(0.2, 0.0, 0.0)], false)
        };
        let s = summary(&tie);
        assert!(!s.closed && s.legs.len() == 3 && !s.is_straight());
        let [a, b, c] = [s.legs[0], s.legs[1], s.legs[2]];
        assert!(a.hook && !b.hook && c.hook);
        // Straight ends of 5 cm (5 diameters is less), the bar between the hooks 0.20 m.
        assert!(close(a.length, 0.05) && close(b.length, 0.2) && close(c.length, 0.05));
        // Both hooks bent the same way (a C), by their angles.
        assert!(close(a.bend, 0.75 * PI) && close(b.bend, PI) && close(c.bend, 0.0));
        // The legs add up to the bar's length less the bends around the pins.
        assert!(close(s.length(), tie.length() - 1.75 * PI * 0.01));
        // Not a loop: its hooks end a bar.
        assert!(s.hooked_loop().is_none());
        let (bar, [h0, h1]) = s.hooked_ends().unwrap();
        assert!(bar.is_straight() && close(bar.length(), 0.2) && close(bar.legs[0].bend, 0.0));
        assert!(close(h0.bend, 0.75 * PI) && close(h1.bend, PI));
        // An S: the end hook bent the other way.
        let s_tie = Bar { end_hooks: Some(EndHooks { opposite: true, ..ends }), ..tie.clone() };
        let s2 = summary(&s_tie);
        assert!(s2.legs[0].bend * s2.legs[1].bend < 0.0);
        assert!(!s.matches(&s2, 0.005, 0.01));
        // Read from its other end, the same tie.
        let back = Bar {
            points: vec![v(0.2, 0.0, 0.0), v(0.0, 0.0, 0.0)],
            end_hooks: Some(EndHooks { start: ends.end, end: ends.start, opposite: false }),
            ..tie.clone()
        };
        assert!(s.matches(&summary(&back), 0.005, 0.01));
    }

    #[test]
    fn hooks_open_a_stirrup_at_its_first_corner() {
        let loop4 = vec![v(0.0, 0.0, 0.0), v(0.14, 0.0, 0.0), v(0.14, 0.0, 0.44), v(0.0, 0.0, 0.44)];
        let hooks = LoopHooks { angle: 0.75 * PI, radius: 0.01, tail: 0.05 };
        let bar = Bar { hooks: Some(hooks), ..Bar::new(loop4.clone(), true) };
        // 2 x (0.14 + 0.44), and each hook its bend (3/4 pi x 0.01) and its straight end.
        assert!(close(bar.length(), 1.16 + 2.0 * (0.75 * PI * 0.01 + 0.05)));
        let s = summary(&bar);
        assert!(!s.closed && s.legs.len() == 6, "{s:?}");
        let (first, last) = (s.legs[0], s.legs[5]);
        assert!(first.hook && last.hook && s.legs[1..5].iter().all(|l| !l.hook));
        assert!(close(first.length, 0.05) && close(first.bend, 0.75 * PI) && close(last.bend, 0.0));
        assert!(close(s.legs[1].length, 0.14) && close(s.legs[4].length, 0.44) && close(s.legs[4].bend, 0.75 * PI));
        // The loop comes back with its closing corner.
        let (closed, [a, b]) = s.hooked_loop().unwrap();
        assert!(closed.closed && closed.legs.iter().all(|l| close(l.bend, FRAC_PI_2)), "{closed:?}");
        assert!(a.hook && b.hook && summary(&Bar::new(loop4.clone(), true)).matches(&closed, 1e-9, 1e-9));
        // Read the other way round, it is the same stirrup; without hooks or with others, not.
        let mut back = loop4.clone();
        back.reverse();
        back.rotate_right(1);
        let other = summary(&Bar { hooks: Some(hooks), ..Bar::new(back, true) });
        assert!(s.matches(&other, 1e-9, 1e-9), "{other:?}");
        assert!(other.hooked_loop().is_some_and(|(c, _)| c.legs.iter().all(|l| close(l.bend.abs(), FRAC_PI_2))));
        assert!(!s.matches(&summary(&Bar::new(loop4.clone(), true)), 0.005, 0.01));
        let right = LoopHooks { angle: FRAC_PI_2, ..hooks };
        assert!(!s.matches(&summary(&Bar { hooks: Some(right), ..Bar::new(loop4, true) }), 0.005, 0.01));
        // A ring closed by hooks: a hook, the whole turn, a hook.
        let ring: Vec<DVec3> =
            (0..16).map(|i| (glam::DVec2::from_angle(TAU * i as f64 / 16.0) * 0.2).extend(0.0)).collect();
        let r = summary(&Bar { hooks: Some(hooks), ..Bar::new(ring, true) });
        assert!(r.legs.len() == 3 && r.legs[1].radius.is_some() && r.legs[0].hook, "{r:?}");
        assert!(r.hooked_loop().is_some_and(|(c, _)| c.legs.len() == 1 && close(c.legs[0].bend, 0.0)));
    }
}
