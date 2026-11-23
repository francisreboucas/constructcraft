//! Wall reinforcement: horizontal bars (ring bars in an arc wall) and
//! vertical bars near each face of a wall's structure, and ties between
//! the faces. Bars lie `cover` inside the face of the wall's core (its
//! whole body when it has no layers), stop at its free ends and around its
//! openings (horizontal bars run to an end joined to another wall), follow
//! the arc of an arc wall at their own radius, and are lap spliced past the
//! stock length.

use crate::WallGeom;
use crate::rebar::{Bar, MAX_BARS};
use cc_core::{BarEnd, Rebar, RebarLayout, SpacingZone, Splice, WallSide};
use glam::DVec3;

/// What a set needs to know of the other sets of its wall.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WallSets<'a> {
    /// Largest bar of the other family (horizontal or vertical) near the
    /// same face: an `inner` set lies inside it.
    pub other: f64,
    /// The exterior face's horizontal and vertical sets with their bar
    /// diameters, and the interior face's cover and outer bar diameter:
    /// what ties are placed by.
    pub horizontal: Option<(&'a Rebar, f64)>,
    pub vertical: Option<(&'a Rebar, f64)>,
    pub interior: Option<(f64, f64)>,
    /// How far horizontal bars run past the wall's start and end, where
    /// another wall carries on in the same line (0: they stop at the joint).
    pub past: (f64, f64),
}

/// Offset of the face of the wall's structure on `side`.
fn face(g: &WallGeom, side: WallSide) -> f64 {
    match (g.core, side) {
        (Some((left, _)), WallSide::Exterior) => left,
        (Some((_, right)), WallSide::Interior) => right,
        (None, side) => side.sign() * g.half,
    }
}

/// Offset of the line of bars `inset` inside the face on `side` (the
/// middle of a wall thinner than two insets).
fn line(g: &WallGeom, side: WallSide, inset: f64) -> f64 {
    let middle = (face(g, WallSide::Exterior) + face(g, WallSide::Interior)) * 0.5;
    let moved = face(g, side) - side.sign() * inset;
    if (moved - middle) * side.sign() < 0.0 { middle } else { moved }
}

/// Length along the wall's face line `v` per meter of its centerline.
fn scale(g: &WallGeom, v: f64) -> f64 {
    g.arc.as_ref().map_or(1.0, |a| (a.radius_at(v) / a.radius.max(1e-9)).max(1e-9))
}

/// Heights above the base of the horizontal bars of `zones` in a wall
/// `height` high: the first `cover` up, each next one the spacing of the
/// zone it starts in further, the last at most `cover` below the top.
pub(crate) fn levels(zones: &[SpacingZone], height: f64, cover: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut h = cover.max(0.0);
    while h <= height - cover + 1e-9 && out.len() < 5000 {
        out.push(h);
        let spacing = zones.iter().find(|z| h < z.up_to - 1e-9).or(zones.last()).map_or(0.0, |z| z.spacing);
        if !(spacing.is_finite() && spacing >= cc_core::wall_bars::MIN_SPACING) {
            break;
        }
        h += spacing;
    }
    out
}

/// Positions along the wall of its vertical bars: every `spacing` from
/// `offset` after `cover` inside the start, to `cover` inside the end.
pub(crate) fn stations(range: (f64, f64), spacing: f64, offset: f64, cover: f64) -> Vec<f64> {
    if !(spacing.is_finite() && spacing >= cc_core::wall_bars::MIN_SPACING && offset.is_finite()) {
        return Vec::new();
    }
    let (start, end) = (range.0 + cover, range.1 - cover);
    // The first station at or after the start.
    let first = start + offset.rem_euclid(spacing);
    let count = if end >= first { ((end - first) / spacing + 1e-9).floor() as usize + 1 } else { 0 };
    (0..count.min(5000)).map(|i| first + spacing * i as f64).collect()
}

/// The `offset` that puts the vertical bars of set `r` (in wall `g`) where
/// they stand now once the wall is flipped (see `cc_core::wall_flip`): the
/// set then counts its stations from the other end of the same face, so
/// its first bar must be the last one now. `None` for other sets and for
/// sets with no bars. Snapped to the nanometer, so flipping twice gives
/// back the offset it started from (taken within one spacing).
pub fn flipped_vertical_offset(g: &WallGeom, r: &Rebar) -> Option<f64> {
    let RebarLayout::WallVertical { side, spacing, offset, .. } = &r.layout else { return None };
    let range = if *side == WallSide::Exterior { (g.start_ext.0, g.end_ext.0) } else { (g.start_ext.1, g.end_ext.1) };
    let last = *stations(range, *spacing, *offset, r.cover).last()?;
    // What is left after the last bar, before the cover at the end (never
    // below 0: a station may sit a hair past the end within the tolerance).
    let rest = (range.1 - r.cover - last).max(0.0);
    Some((rest * 1e9).round() / 1e9)
}

/// The stretches of the wall on `side` that are solid at elevation `z`,
/// along its centerline, neighbours joined.
fn solid_runs(g: &WallGeom, side: WallSide, z: f64) -> Vec<(f64, f64)> {
    let mut runs: Vec<(f64, f64)> = Vec::new();
    // A wall with an elevation profile is solid at `z` only between its chains.
    for c in g.cut_columns(z).iter().filter(|c| c.is_solid_at(z)) {
        let (a, b) = if side == WallSide::Exterior { c.left } else { c.right };
        match runs.last_mut() {
            Some(last) if a <= last.1 + 1e-6 => last.1 = last.1.max(b),
            _ => runs.push((a, b)),
        }
    }
    runs
}

/// The bars of the set `r` (bars `d` thick) in the wall `g`.
pub(crate) fn wall_bars(g: &WallGeom, r: &Rebar, d: f64, sets: &WallSets) -> Vec<Bar> {
    let cover = r.cover;
    match &r.layout {
        RebarLayout::WallHorizontal { side, zones, inner } => {
            let v = line(g, *side, cover + if *inner { sets.other } else { 0.0 } + d / 2.0);
            let k = scale(g, v);
            let range =
                if *side == WallSide::Exterior { (g.start_ext.0, g.end_ext.0) } else { (g.start_ext.1, g.end_ext.1) };
            let mut bars = Vec::new();
            for (index, h) in levels(zones, g.top_z - g.base_z, cover).into_iter().enumerate() {
                let z = g.base_z + h;
                for (a, b) in solid_runs(g, *side, z) {
                    // The cover at free ends and openings; at an end joined to
                    // another wall the bar runs to the joint, where that wall's begin,
                    // or past it to lap with them.
                    let a = if g.start_joined && (a - range.0).abs() < 1e-6 { a - sets.past.0 / k } else { a + cover };
                    let b = if g.end_joined && (b - range.1).abs() < 1e-6 { b + sets.past.1 / k } else { b - cover };
                    if b - a < 1e-6 {
                        continue;
                    }
                    // Pieces of the stock length, measured on the bar's own line.
                    let length = (b - a) * k;
                    for (from, to) in pieces(r.splice, length, index) {
                        if bars.len() >= MAX_BARS {
                            return bars;
                        }
                        let (u0, u1) = (a + from / k, a + to / k);
                        let points = g.path(u0, u1, v).into_iter().map(|p| p.extend(z)).collect();
                        bars.push(Bar { true_length: g.arc.is_some().then_some(to - from), ..Bar::new(points, false) });
                    }
                }
            }
            bars
        }
        RebarLayout::WallVertical { side, spacing, offset, from, to, bottom, top, inner } => {
            let v = line(g, *side, cover + if *inner { sets.other } else { 0.0 } + d / 2.0);
            let range =
                if *side == WallSide::Exterior { (g.start_ext.0, g.end_ext.0) } else { (g.start_ext.1, g.end_ext.1) };
            // The heights the set spans, its cover inside the wall's base and top.
            let low = g.base_z + from.max(cover);
            let high = if *to > 0.0 { g.base_z + *to } else { f64::INFINITY };
            let mut bars = Vec::new();
            for (index, u) in stations(range, *spacing, *offset, cover).into_iter().enumerate() {
                let Some(column) = g.column_at(u) else { continue };
                // The stretches of the bar: the solid heights here, within the set's.
                let mut cuts: Vec<(f64, f64)> = Vec::new();
                let profiled = g.is_profiled();
                // An elevation profile's heights here, its base with the cover too.
                let solid = if profiled { g.intervals_at(column, u) } else { column.solid.clone() };
                for (k, &(z0, z1)) in solid.iter().enumerate() {
                    // Under a top that follows a roof the wall is lower here.
                    let z1 = if profiled { z1 } else { g.profile_top((z0 + z1) * 0.5, u).map_or(z1, |t| t.min(z1)) };
                    let low = if profiled && k == 0 { low.max(z0 + cover) } else { low };
                    let (a, b) = (z0.max(low), (z1 - cover).min(high));
                    if b - a > 1e-6 {
                        cuts.extend(pieces(r.splice, b - a, index).into_iter().map(|(lo, hi)| (a + lo, a + hi)));
                    }
                }
                // Only the bar's own ends are bent: not where an opening or a splice cuts it.
                let last = cuts.len().saturating_sub(1);
                for (i, z) in cuts.into_iter().enumerate() {
                    if bars.len() >= MAX_BARS {
                        return bars;
                    }
                    let foot = if i == 0 { *bottom } else { BarEnd::default() };
                    let head = if i == last { *top } else { BarEnd::default() };
                    bars.push(vertical_bar(g, u, v, *side, z, foot, head));
                }
            }
            bars
        }
        RebarLayout::WallTies { every_horizontal, every_vertical, hooks } => {
            let (Some((h, dh)), Some((vs, dv))) = (sets.horizontal, sets.vertical) else { return Vec::new() };
            let (RebarLayout::WallHorizontal { zones, .. }, RebarLayout::WallVertical { spacing, offset, .. }) =
                (&h.layout, &vs.layout)
            else {
                return Vec::new();
            };
            // From the outer bars of one face to those of the other.
            let (cover_in, d_in) = sets.interior.unwrap_or((h.cover, dh));
            let v0 = line(g, WallSide::Exterior, h.cover.min(vs.cover) + dh.min(dv) / 2.0);
            let v1 = line(g, WallSide::Interior, cover_in + d_in / 2.0);
            // From the exterior face to the interior one, the hooks bent to
            // opposite sides of the tie (an S), each around the bar it holds.
            let ends = crate::rebar::EndHooks::of_tie(hooks[0].to_radians(), hooks[1].to_radians(), d, true);
            let hooks = ends.length();
            let range = (g.start_ext.0, g.end_ext.0);
            let us: Vec<f64> =
                stations(range, *spacing, *offset, vs.cover).into_iter().step_by(*every_vertical as usize).collect();
            let hs = levels(zones, g.top_z - g.base_z, h.cover).into_iter().step_by(*every_horizontal as usize);
            let mut bars = Vec::new();
            for z in hs.map(|h| g.base_z + h) {
                for &u in &us {
                    if bars.len() >= MAX_BARS {
                        return bars;
                    }
                    let solid = if g.is_profiled() {
                        g.solid_at(u, z)
                    } else {
                        g.column_at(u).is_some_and(|c| c.is_solid_at(z))
                    };
                    if !solid {
                        continue;
                    }
                    let points = vec![g.world(u, v0).extend(z), g.world(u, v1).extend(z)];
                    bars.push(Bar {
                        true_length: Some((v0 - v1).abs() + hooks),
                        end_hooks: Some(ends),
                        ..Bar::new(points, false)
                    });
                }
            }
            bars
        }
        _ => Vec::new(),
    }
}

/// The pieces of a bar `length` long: whole, or cut by the splice.
fn pieces(splice: Option<Splice>, length: f64, index: usize) -> Vec<(f64, f64)> {
    splice.map_or_else(|| vec![(0.0, length)], |s| s.pieces(length, index))
}

/// A vertical bar at `(u, v)` between the elevations `z`, its ends bent.
fn vertical_bar(g: &WallGeom, u: f64, v: f64, side: WallSide, z: (f64, f64), foot: BarEnd, head: BarEnd) -> Bar {
    let at = |v: f64, z: f64| g.world(u, v).extend(z);
    // An end's legs, from the bar outwards: across the wall, then along the bar.
    let legs = |end: BarEnd, z: f64, back: f64| -> Vec<DVec3> {
        let mut pts = Vec::new();
        if end.across != 0.0 {
            let out = v + side.sign() * end.across;
            pts.push(at(out, z));
            if end.along != 0.0 {
                pts.push(at(out, z + back * end.along));
            }
        }
        pts
    };
    let mut points: Vec<DVec3> = legs(foot, z.0, 1.0).into_iter().rev().collect();
    points.push(at(v, z.0));
    points.push(at(v, z.1));
    points.extend(legs(head, z.1, -1.0));
    Bar::new(points, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_bars_close_up_towards_the_base() {
        // Every 10 cm over the first 2 m, 15 cm up to 4 m, 20 cm above.
        let zones = [
            SpacingZone { up_to: 2.0, spacing: 0.10 },
            SpacingZone { up_to: 4.0, spacing: 0.15 },
            SpacingZone { up_to: 6.0, spacing: 0.20 },
        ];
        let h = levels(&zones, 6.0, 0.05);
        assert!((h[0] - 0.05).abs() < 1e-12 && (h[1] - 0.15).abs() < 1e-12);
        // 20 bars start below 2 m, 13 between 2 and 4 m, and the rest 20 cm apart towards the top.
        assert_eq!(h.iter().filter(|x| **x < 2.0).count(), 20);
        let mid: Vec<f64> = h.iter().copied().filter(|x| (2.0..4.0 - 1e-6).contains(x)).collect();
        assert!(mid.windows(2).all(|w| (w[1] - w[0] - 0.15).abs() < 1e-9) && mid.len() == 13, "{mid:?}");
        let top: Vec<f64> = h.iter().copied().filter(|x| *x >= 4.0 - 1e-6).collect();
        assert!(top.windows(2).all(|w| (w[1] - w[0] - 0.20).abs() < 1e-9));
        assert!(*h.last().unwrap() <= 5.95 + 1e-9 && *h.last().unwrap() > 5.75);
        // A wall higher than its last zone keeps that spacing to the top.
        assert!(levels(&zones[..1], 3.0, 0.05).len() == 30);
        // Values a file may hold: nothing rather than a loop.
        assert_eq!(levels(&[SpacingZone { up_to: 1.0, spacing: 0.0 }], 3.0, 0.05).len(), 1);
        assert!(levels(&[], 3.0, 0.05).len() == 1 && levels(&zones, 0.0, 0.05).is_empty());
    }

    #[test]
    fn vertical_bars_are_spaced_from_the_start_and_may_interleave() {
        let s = stations((0.0, 2.0), 0.2, 0.0, 0.05);
        assert_eq!(s.len(), 10);
        assert!((s[0] - 0.05).abs() < 1e-12 && (s[9] - 1.85).abs() < 1e-12);
        // Half a spacing on: between the bars of the first set.
        let t = stations((0.0, 2.0), 0.2, 0.1, 0.05);
        assert!((t[0] - 0.15).abs() < 1e-12 && t.len() == 10);
        assert!(stations((0.0, 2.0), 0.0, 0.0, 0.05).is_empty() && stations((0.0, 0.05), 0.2, 0.0, 0.05).is_empty());
    }
}
