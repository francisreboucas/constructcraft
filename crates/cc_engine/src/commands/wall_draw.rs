//! Walls drawn as shapes (issue #497): the wall tool's lines, rectangles,
//! polygons, circles, arcs, ellipses, splines and picked lines become
//! walls joined end to end, in one undo step. The shape is the location
//! line of the walls; a `radius` rounds the corners between joined lines
//! and an `offset` moves the whole chain sideways. Walls are straight, arcs
//! or splines, so a circle becomes two half-circle walls, an arc of more
//! than half a turn two walls, an ellipse spline walls of up to a quarter
//! turn each, and a closed spline two walls.

use super::create::{check_arc, top};
use super::sketch::{CurveArg, sketch_of};
use super::{add_element, edit, ids_json};
use crate::error::{EngineError, Result};
use crate::refs::{self, Point, Ref, finite, point, positive};
use crate::registry::{Command, CommandKind};
use crate::session::Session;
use crate::t;
use cc_core::arc::Arc;
use cc_core::sketch::{JOIN_TOL, SketchCurve};
use cc_core::wall_spline::WallSpline;
use cc_core::{Category, ElementKind, LocationLine, Wall, WallTop, sketch_edit, spline};
use glam::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Most walls one command makes.
pub const MAX_WALLS: usize = 512;

/// Create walls along curves on a level: lines, arcs, circles, ellipses,
/// rectangles, polygons and splines, the walls' location line. Curves
/// joined end to end make walls joined end to end; `radius` rounds the
/// corners between joined straight lines with arcs, and `offset` moves each
/// chain of curves sideways (its corners meeting again). A circle becomes
/// two half-circle walls, an arc of more than half a turn two arc walls, an
/// ellipse spline walls of a quarter turn at most, a closed spline two
/// spline walls. All the walls are one undo step; their ids come back in
/// the order made.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct WallsCreate {
    /// The curves the walls' location line follows (as `floor_create` takes them).
    pub curves: Vec<CurveArg>,
    /// Distance in meters the walls lie from the curves: to the left of an open chain
    /// (looking from its start), outside a closed one; negative the other way (default 0).
    #[serde(default)]
    pub offset: Option<f64>,
    /// Radius in meters of the arcs that round the corners between joined straight
    /// lines (default 0: sharp corners).
    #[serde(default)]
    pub radius: Option<f64>,
    /// Level id or name (default: the lowest level).
    #[serde(default)]
    pub level: Option<Ref>,
    /// Wall type id or name (default: the default wall type).
    #[serde(default, rename = "type")]
    pub type_ref: Option<Ref>,
    /// Unconnected height in meters (default 3; 10'-0" in imperial projects).
    #[serde(default)]
    pub height: Option<f64>,
    /// Level the tops are attached to, instead of a height.
    #[serde(default)]
    pub top_level: Option<Ref>,
    /// Offset of the tops from `top_level`, meters.
    #[serde(default)]
    pub top_offset: Option<f64>,
    /// Base offset from the level, meters.
    #[serde(default)]
    pub base_offset: Option<f64>,
    /// Which line of the walls the curves are: `Centerline` (default), `CoreCenter`,
    /// `FinishExterior`, `FinishInterior`, `CoreExterior` or `CoreInterior`.
    #[serde(default)]
    pub location_line: Option<String>,
    /// Structural walls (default: as the wall type says).
    #[serde(default)]
    pub structural: Option<bool>,
    /// The walls' ends may join other walls (default true); false keeps them square.
    #[serde(default)]
    pub join: Option<bool>,
}

impl Command for WallsCreate {
    const NAME: &'static str = "walls_create";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let curves = sketch_of(&self.curves)?.curves;
        if curves.is_empty() {
            return Err(EngineError::invalid("give the curves the walls follow"));
        }
        let offset = finite(self.offset.unwrap_or(0.0), "offset")?;
        let radius = finite(self.radius.unwrap_or(0.0), "radius")?;
        if radius < 0.0 {
            return Err(EngineError::invalid("the radius cannot be negative"));
        }
        let lines = location_curves(&curves, offset, radius)?;
        let level = refs::level_or_first(p, self.level.as_ref())?;
        let type_id = refs::type_of(p, Category::Wall, self.type_ref.as_ref())?;
        let location_line = match self.location_line {
            Some(name) => serde_json::from_value::<LocationLine>(Value::String(name.clone()))
                .map_err(|_| EngineError::invalid(format!("unknown location line '{name}'")))?,
            None => LocationLine::default(),
        };
        let structural =
            self.structural.unwrap_or_else(|| p.element_type(type_id).is_some_and(|t| t.structural_wall()));
        let join = self.join.unwrap_or(true);
        let template = Wall {
            base_offset: finite(self.base_offset.unwrap_or(0.0), "base_offset")?,
            height: positive(self.height.unwrap_or(p.defaults().story), "height")?,
            top: top(p, self.top_level.as_ref(), self.top_offset)?.unwrap_or(WallTop::Unconnected),
            location_line,
            structural,
            join_start: join,
            join_end: join,
            ..Default::default()
        };
        let mut walls = Vec::new();
        for c in &lines {
            for piece in pieces(c)? {
                let wall = piece.wall(&template);
                check_arc(p, type_id, &wall)?;
                super::wall_spline::check_wall(p, None, type_id, &wall)?;
                walls.push(wall);
                if walls.len() > MAX_WALLS {
                    return Err(EngineError::invalid(format!("at most {MAX_WALLS} walls at once")));
                }
            }
        }
        let ids = edit(s, t!("tx-create-walls"), |e| {
            Ok(walls.into_iter().map(|w| add_element(e, level, type_id, ElementKind::Wall(w))).collect::<Vec<_>>())
        })?;
        Ok(json!({ "ids": ids_json(&ids) }))
    }
}

/// Round the corner between two straight walls with an arc wall of
/// `radius`: both walls are trimmed or extended to meet (keeping the part
/// each pick point is on, as `trim_corner` does), then cut back to where an
/// arc of `radius` tangent to both leaves them, and the arc, a wall like
/// the first, fills the gap. One undo step; returns the arc wall.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallFillet {
    /// First wall id.
    pub a: u64,
    /// A point on the part of `a` to keep.
    pub pick_a: Point,
    /// Second wall id.
    pub b: u64,
    /// A point on the part of `b` to keep.
    pub pick_b: Point,
    /// Radius of the arc's location line, meters.
    pub radius: f64,
}

impl Command for WallFillet {
    const NAME: &'static str = "wall_fillet";
    const KIND: CommandKind = CommandKind::Edit;
    fn run(self, s: &mut Session) -> Result<Value> {
        let p = s.project();
        let (a, b) = (refs::element(p, self.a)?, refs::element(p, self.b)?);
        if a == b {
            return Err(EngineError::invalid("pick two different walls"));
        }
        super::modify::not_pinned(p, a)?;
        super::modify::not_pinned(p, b)?;
        let radius = positive(self.radius, "radius")?;
        let straight =
            |id| p.element(id).and_then(|e| e.as_wall()).is_some_and(|w| !w.is_curved() && w.profile.is_none());
        if !straight(a) || !straight(b) {
            return Err(EngineError::invalid("a fillet joins two straight walls without an elevation profile"));
        }
        let (pa, pb) = (point(self.pick_a)?, point(self.pick_b)?);
        let arc = edit(s, t!("tx-wall-fillet"), |e| {
            e.trim_corner(a, pa, b, pb)?;
            let wall = |e: &cc_core::EditCtx, id| e.project().element(id).and_then(|el| el.as_wall()).cloned();
            let (Some(wa), Some(wb)) = (wall(e, a), wall(e, b)) else {
                return Err(EngineError::not_found("wall"));
            };
            let RoundedCorner { a_start, b_start, ta, tb, bulge } =
                round_corner((wa.start, wa.end), (wb.start, wb.end), radius, "walls")?;
            let set = |e: &mut cc_core::EditCtx, id, start: bool, to: DVec2| {
                if let Some(w) = e.element_mut(id).and_then(|el| match &mut el.kind {
                    ElementKind::Wall(w) => Some(w),
                    _ => None,
                }) {
                    if start { w.start = to } else { w.end = to }
                }
            };
            set(e, a, a_start, ta);
            set(e, b, b_start, tb);
            let (level, type_id) = e
                .project()
                .element(a)
                .map(|el| (el.level, el.type_id))
                .ok_or_else(|| EngineError::not_found("wall"))?;
            let arc = Wall {
                start: ta,
                end: tb,
                bulge,
                join_start: true,
                join_end: true,
                attach_top: None,
                profile: None,
                spline: None,
                curtain: Default::default(),
                ..wa
            };
            check_arc(e.project(), type_id, &arc)?;
            Ok(add_element(e, level, type_id, ElementKind::Wall(arc)))
        })?;
        Ok(json!({ "created": arc.0, "changed": [a.0, b.0] }))
    }
}

/// Where an arc of `radius` rounds the corner between two straight
/// segments that meet at an end: which end of each is at the corner, the
/// points the arc leaves them (from `a` to `b`) and its bulge.
pub(crate) struct RoundedCorner {
    pub a_start: bool,
    pub b_start: bool,
    pub ta: DVec2,
    pub tb: DVec2,
    pub bulge: f64,
}

/// The arc of `radius` between segments `a` and `b` (their two ends) that
/// meet at a corner; `what` names them in errors ("walls", "lines").
pub(crate) fn round_corner(a: (DVec2, DVec2), b: (DVec2, DVec2), radius: f64, what: &str) -> Result<RoundedCorner> {
    // The corner: the ends of the two segments that meet.
    let (corner, a_start) = [(a.0, true), (a.1, false)]
        .into_iter()
        .min_by(|x, y| {
            let d = |q: DVec2| q.distance(b.0).min(q.distance(b.1));
            d(x.0).total_cmp(&d(y.0))
        })
        .ok_or_else(|| EngineError::invalid(format!("the {what} do not meet")))?;
    let b_start = b.0.distance(corner) <= b.1.distance(corner);
    let far = |s: (DVec2, DVec2), start: bool| if start { s.1 } else { s.0 };
    let (fa, fb) = (far(a, a_start), far(b, b_start));
    let (Some(ua), Some(ub)) = ((fa - corner).try_normalize(), (fb - corner).try_normalize()) else {
        return Err(EngineError::invalid(format!("one of the {what} has no length left")));
    };
    // Heading into the corner along `a`, then out along `b`.
    let turn = (-ua).perp_dot(ub).atan2((-ua).dot(ub));
    if turn.abs() < 1e-6 || turn.abs() > std::f64::consts::PI - 1e-6 {
        return Err(EngineError::invalid(format!("the {what} are in line: there is no corner to round")));
    }
    let cut = radius * (turn.abs() / 2.0).tan();
    if cut >= corner.distance(fa) - JOIN_TOL || cut >= corner.distance(fb) - JOIN_TOL {
        return Err(EngineError::invalid(format!("the radius {radius} m is too large for these {what}")));
    }
    Ok(RoundedCorner { a_start, b_start, ta: corner + ua * cut, tb: corner + ub * cut, bulge: (turn / 4.0).tan() })
}

/// The location lines of the walls drawn along `curves`: ellipses as
/// splines, the corners between joined lines rounded to `radius`, and each
/// chain moved `offset` to its left (outward when closed). What the wall
/// tool previews and [`WallsCreate`] builds on, and the shapes
/// `detail_line_create` draws with an offset or a radius.
pub fn location_curves(curves: &[SketchCurve], offset: f64, radius: f64) -> Result<Vec<SketchCurve>> {
    let curves: Vec<SketchCurve> = curves.iter().map(no_ellipse).collect::<Result<_>>()?;
    let mut done = vec![false; curves.len()];
    let mut out = Vec::new();
    for i in 0..curves.len() {
        if done.get(i).copied().unwrap_or(true) {
            continue;
        }
        let (links, closed) = sketch_edit::chain(&curves, i);
        let mut chain = Vec::with_capacity(links.len());
        for &(k, rev) in &links {
            if let Some(flag) = done.get_mut(k) {
                *flag = true;
            }
            let c = curves.get(k).ok_or_else(|| EngineError::invalid("a curve of the chain is missing"))?;
            chain.push(if rev { c.reversed() } else { c.clone() });
        }
        if radius > 0.0 {
            chain = fillet(&chain, closed, radius)?;
        }
        if offset != 0.0 {
            // Closed chains move outward: to the right of a counter-clockwise loop.
            let ccw = closed && chain.iter().map(SketchCurve::swept_area).sum::<f64>() > 0.0;
            let v = if ccw { -offset } else { offset };
            chain = sketch_edit::offset_left_chain(&chain, closed, v)
                .ok_or_else(|| EngineError::invalid("the offset is larger than a curve's radius"))?;
        }
        out.extend(chain);
    }
    Ok(out)
}

/// An ellipse as the spline that follows it (other curves as they are).
fn no_ellipse(c: &SketchCurve) -> Result<SketchCurve> {
    match *c {
        SketchCurve::Ellipse { center, radii, rotation_deg, start_deg, sweep_deg } => {
            spline::ellipse(center, radii, rotation_deg, start_deg, sweep_deg)
                .map(|points| SketchCurve::Spline { points })
                .ok_or_else(|| EngineError::invalid("the ellipse has no size"))
        }
        _ => Ok(c.clone()),
    }
}

/// The chain `chain` (in order, back to the first when `closed`) with each
/// corner between two straight lines rounded by an arc of `radius` tangent
/// to both.
fn fillet(chain: &[SketchCurve], closed: bool, radius: f64) -> Result<Vec<SketchCurve>> {
    let n = chain.len();
    // How far each corner's arc cuts back the lines at corner k (between curve k and k + 1).
    let corners = if closed { n } else { n.saturating_sub(1) };
    let mut trims: Vec<Option<(f64, f64)>> = vec![None; n];
    let mut lines: Vec<SketchCurve> = chain.to_vec();
    let mut arcs: Vec<Option<SketchCurve>> = vec![None; n];
    let line_dir = |c: &SketchCurve| match *c {
        SketchCurve::Line { a, b } => (b - a).try_normalize().map(|d| (a, b, d)),
        _ => None,
    };
    for k in 0..corners {
        let j = (k + 1) % n;
        let (Some((a0, p, d1)), Some((_, b1, d2))) = (chain.get(k).and_then(line_dir), chain.get(j).and_then(line_dir))
        else {
            continue;
        };
        let turn = d1.perp_dot(d2).atan2(d1.dot(d2));
        // Straight on, or a line doubling back: no corner to round.
        if turn.abs() < 1e-6 || turn.abs() > std::f64::consts::PI - 1e-6 {
            continue;
        }
        let cut = radius * (turn.abs() / 2.0).tan();
        let (before, after) = (p - a0, b1 - p);
        // Each line's two corners share its length.
        let room = |len: f64, other: f64| cut + other <= len - JOIN_TOL;
        let k_other = trims.get(k).and_then(|t| t.map(|t| t.0)).unwrap_or(0.0);
        let j_other = trims.get(j).and_then(|t| t.map(|t| t.1)).unwrap_or(0.0);
        if !room(before.length(), k_other) || !room(after.length(), j_other) {
            return Err(EngineError::invalid(format!("the radius {radius} m is too large for a corner of the shape")));
        }
        let entry = trims.get_mut(k).ok_or_else(|| EngineError::invalid("corner out of range"))?;
        *entry = Some((entry.map_or(0.0, |t| t.0), cut));
        let exit = trims.get_mut(j).ok_or_else(|| EngineError::invalid("corner out of range"))?;
        *exit = Some((cut, exit.map_or(0.0, |t| t.1)));
        let bulge = (turn / 4.0).tan();
        if let Some(slot) = arcs.get_mut(k) {
            *slot = Some(SketchCurve::Arc { a: p - d1 * cut, b: p + d2 * cut, bulge });
        }
    }
    // Cut each line back by its corners' arcs.
    for (c, t) in lines.iter_mut().zip(&trims) {
        if let (SketchCurve::Line { a, b }, Some((start, end))) = (&mut *c, t) {
            let Some(d) = (*b - *a).try_normalize() else { continue };
            *a += d * *start;
            *b -= d * *end;
        }
    }
    let mut out = Vec::with_capacity(n * 2);
    for (c, arc) in lines.into_iter().zip(arcs) {
        out.push(c);
        out.extend(arc);
    }
    Ok(out)
}

/// One wall's location line.
enum Piece {
    Line(DVec2, DVec2),
    Arc(DVec2, DVec2, f64),
    Spline(Vec<DVec2>),
}

impl Piece {
    /// `template` along this piece.
    fn wall(self, template: &Wall) -> Wall {
        let mut w = template.clone();
        match self {
            Piece::Line(a, b) => (w.start, w.end) = (a, b),
            Piece::Arc(a, b, bulge) => (w.start, w.end, w.bulge) = (a, b, bulge),
            Piece::Spline(control) => {
                if let (Some(&a), Some(&b)) = (control.first(), control.last()) {
                    (w.start, w.end) = (a, b);
                }
                w.spline = Some(WallSpline { control });
            }
        }
        w
    }
}

/// The walls a location curve makes.
fn pieces(c: &SketchCurve) -> Result<Vec<Piece>> {
    const MIN: f64 = 1e-3;
    let pieces = match *c {
        SketchCurve::Line { a, b } => vec![Piece::Line(a, b)],
        SketchCurve::Arc { a, b, bulge } => {
            let arc = Arc::from_bulge(a, b, bulge);
            match arc {
                // More than half a turn: two walls of the half bulge each.
                Some(arc) if bulge.abs() > 1.0 => {
                    let m = arc.point(arc.length() * 0.5);
                    let half = (bulge.atan() / 2.0).tan();
                    vec![Piece::Arc(a, m, half), Piece::Arc(m, b, half)]
                }
                Some(_) => vec![Piece::Arc(a, b, bulge)],
                None => vec![Piece::Line(a, b)],
            }
        }
        SketchCurve::Circle { center, radius } => {
            let (e, w) = (center + DVec2::X * radius, center - DVec2::X * radius);
            vec![Piece::Arc(e, w, 1.0), Piece::Arc(w, e, 1.0)]
        }
        SketchCurve::Spline { ref points } if spline::is_closed(points) => {
            // Split where its segments join, halfway round.
            let at = 3 * (spline::segment_count(points).unwrap_or(0) / 2);
            match (points.get(..=at), points.get(at..)) {
                (Some(first), Some(second)) if at > 0 => {
                    vec![Piece::Spline(first.to_vec()), Piece::Spline(second.to_vec())]
                }
                _ => return Err(EngineError::invalid("a closed spline could not be split into walls")),
            }
        }
        SketchCurve::Spline { ref points } => vec![Piece::Spline(points.clone())],
        SketchCurve::Ellipse { .. } => return pieces(&no_ellipse(c)?),
    };
    let long_enough = |p: &Piece| match p {
        Piece::Line(a, b) | Piece::Arc(a, b, _) => a.distance(*b) >= MIN,
        Piece::Spline(points) => spline::length(points) >= MIN && spline::segment_count(points).is_some(),
    };
    if !pieces.iter().all(long_enough) {
        return Err(EngineError::invalid("a wall would be shorter than 1 mm"));
    }
    Ok(pieces)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> DVec2 {
        DVec2::new(x, y)
    }

    fn rect() -> Vec<SketchCurve> {
        let c = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)];
        (0..4).map(|i| SketchCurve::Line { a: c[i], b: c[(i + 1) % 4] }).collect()
    }

    #[test]
    fn a_radius_rounds_each_corner_of_a_rectangle() {
        let out = location_curves(&rect(), 0.0, 0.5).unwrap_or_default();
        assert_eq!(out.len(), 8);
        let arcs: Vec<_> = out.iter().filter(|c| matches!(c, SketchCurve::Arc { .. })).collect();
        assert_eq!(arcs.len(), 4);
        // A quarter turn counter-clockwise: tan(90° / 4).
        for a in arcs {
            let SketchCurve::Arc { bulge, .. } = a else { continue };
            assert!((bulge - (std::f64::consts::FRAC_PI_8).tan()).abs() < 1e-9);
        }
        // The pieces still join end to end.
        let (links, closed) = sketch_edit::chain(&out, 0);
        assert!(closed && links.len() == 8);
        assert!(location_curves(&rect(), 0.0, 2.0).is_err(), "a 3 m side has no room for two 2 m cuts");
    }

    #[test]
    fn an_offset_moves_a_closed_loop_outward_whichever_way_it_runs() {
        for curves in [rect(), rect().iter().rev().map(SketchCurve::reversed).collect()] {
            let out = location_curves(&curves, 0.5, 0.0).unwrap_or_default();
            let pts: Vec<DVec2> = out.iter().filter_map(SketchCurve::ends).map(|e| e.0).collect();
            let (lo, hi) = pts.iter().fold((p(9.0, 9.0), p(-9.0, -9.0)), |(lo, hi), q| (lo.min(*q), hi.max(*q)));
            assert!(lo.distance(p(-0.5, -0.5)) < 1e-9 && hi.distance(p(4.5, 3.5)) < 1e-9, "{lo} {hi}");
        }
    }

    #[test]
    fn circles_and_long_arcs_split_into_half_turns() {
        let circle = pieces(&SketchCurve::Circle { center: p(1.0, 1.0), radius: 2.0 }).unwrap_or_default();
        assert_eq!(circle.len(), 2);
        let long = pieces(&SketchCurve::Arc { a: p(0.0, 0.0), b: p(2.0, 0.0), bulge: 3.0 }).unwrap_or_default();
        let [Piece::Arc(a, m, h1), Piece::Arc(m2, b, h2)] = long.as_slice() else { panic!("two arcs expected") };
        assert!(m.distance(*m2) < 1e-12 && a.distance(p(0.0, 0.0)) < 1e-12 && b.distance(p(2.0, 0.0)) < 1e-12);
        assert!((h1 - h2).abs() < 1e-12 && (4.0 * h1.atan() * 2.0 - 4.0 * 3f64.atan()).abs() < 1e-9);
    }
}
