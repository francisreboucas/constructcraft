//! Sketch mode: the outline of a floor, ceiling, roof, foundation slab or
//! mass drawn with lines, rectangles, polygons, circles, arcs and ellipses
//! (see [`cc_core::sketch`]), in the active plan. The ribbon shows the
//! drawing tools and *Finish*/*Cancel* while the sketch is open; in its
//! *Select* mode a click picks a curve, whose grips stretch it and whose
//! values the properties palette edits; a window picks the curves inside it
//! (or, dragged to the left, those it touches) and a double click the chain
//! of joined curves. The modify tools move, copy, rotate and mirror the
//! picked curves, and offset, trim, extend and split curves (see
//! [`cc_core::sketch_edit`]). *Finish* creates the element (or draws the
//! edited element's outline again, *Edit Boundary*) through the engine's
//! `curves`; the sketch's own changes undo inside it. A terrain's sketch
//! also takes its measured points (*Point*, at the elevation in the options
//! bar), which undo with the curves.

use crate::commands::UiCommand;
use crate::paint::{HIGHLIGHT, PREVIEW, Xf};
use crate::state::AppState;
use crate::t;
use crate::theme::{self, PALETTE};
use crate::tools::{Snap, Tool, TypedInput, dimension_line, draw_snap, snap_at, temp_dimension};
use cc_core::arc::Arc;
use cc_core::sketch::{JOIN_TOL, Sketch, SketchCurve, SketchError};
use cc_core::sketch_edit;
use cc_core::{Category, ElementId, ElementTypeId, LevelId, PlanTransform, ProfileId};
use cc_engine::Ref;
use cc_engine::commands::sketch::CurveArg;
use egui::{Color32, Painter, Response, Stroke, Ui};
use glam::{DVec2, DVec3};
use std::collections::BTreeSet;

/// Color of the sketch's curves.
const SKETCH: Color32 = Color32::from_rgb(200, 40, 160);
/// Most sketch states kept for undo.
const MAX_UNDO: usize = 200;
/// The fewest and most sides a polygon takes.
pub(crate) const MIN_SIDES: usize = 3;
pub(crate) const MAX_SIDES: usize = 64;
/// Pick distance on screen, points.
const PICK_PX: f64 = 6.0;

/// What finishing the sketch makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A new element of `category` and type.
    New { category: Category, type_id: ElementTypeId },
    /// A new outline for an element (*Edit Boundary*).
    Edit { id: ElementId },
    /// A piece of the component environment (see [`Session::piece`]).
    Piece,
    /// The profile of a sweep type (*Edit Profile*, see [`crate::profile_sketch`]).
    Profile { type_id: ElementTypeId },
    /// Section = a structural profile's sketch (*Edit sketch* in the Profile Manager,
    /// see [`crate::profile_sketch`]).
    Section { profile: ProfileId },
    /// A wall's elevation profile, in its own elevation frame (`u` along the
    /// wall from its start, `z` above its base; see [`crate::wall_profile_ui`]).
    WallProfile { wall: ElementId },
    /// Lines of a title block on its Sheet view (see [`crate::title_block_env`]).
    Sheet,
    /// Detail lines being drawn (see [`crate::detail_ui`]): each curve
    /// becomes an element as soon as it is drawn, so there is nothing to finish.
    Detail,
    /// Walls drawn as shapes (see [`crate::wall_draw_ui`]): each shape
    /// becomes walls as soon as it is drawn.
    Walls,
}

/// How clicks draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Draw {
    /// Pick curves to change them.
    #[default]
    Select,
    /// Two points, chained by default.
    Line,
    /// Two opposite corners.
    Rect,
    /// The center, then a corner.
    Polygon,
    /// The center, then the middle of a side.
    PolygonOut,
    /// The center, then a point on it.
    Circle,
    /// The two ends, then a point on the arc.
    Arc3,
    /// The center, the start, then the end.
    ArcCenter,
    /// From the free end of a curve, along it, to the end.
    ArcTangent,
    /// The center, the end of an axis, then the other axis's half length.
    Ellipse,
    /// Points a smooth curve passes through: a click on the last point
    /// (a double click) or Enter ends it, a click on the first closes it.
    Spline,
    /// Move the picked curves: base point, then target.
    Move,
    /// Copies of the picked curves: base point, then targets.
    Copy,
    /// Rotate the picked curves: center, reference direction, final direction.
    Rotate,
    /// Mirror the picked curves across a line of the sketch.
    MirrorPick,
    /// Mirror the picked curves across an axis drawn with two points.
    MirrorAxis,
    /// A copy of a curve, or of its chain, at a distance.
    Offset,
    /// Two curves trimmed or extended to meet.
    TrimCorner,
    /// Curves trimmed or extended to a reference curve.
    TrimExtend,
    /// A curve split in two.
    Split,
    /// A terrain's measured point, at the elevation of the options bar.
    Point,
}

impl Draw {
    /// The tools that act on the picked curves.
    pub fn on_selection(self) -> bool {
        matches!(self, Draw::Move | Draw::Copy | Draw::Rotate | Draw::MirrorPick | Draw::MirrorAxis)
    }

    /// The modify tools.
    fn modifies(self) -> bool {
        self.on_selection() || matches!(self, Draw::Offset | Draw::TrimCorner | Draw::TrimExtend | Draw::Split)
    }
}

/// A grip of a selected curve: `which` is an end (0, 1), the middle of an
/// arc (2), a center (3), or a point on a circle or an ellipse's axis (4, 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Grip {
    curve: usize,
    which: u8,
}

/// What the sketch's undo brings back: its curves, a terrain's points and
/// which lines came from the walls.
#[derive(Debug, Clone)]
struct Snapshot {
    curves: Vec<SketchCurve>,
    points: Vec<DVec3>,
    from_walls: Vec<Vec<SketchCurve>>,
}

/// An open sketch.
#[derive(Debug, Clone)]
pub struct Session {
    pub target: Target,
    pub curves: Vec<SketchCurve>,
    pub draw: Draw,
    /// Points picked for the curve being drawn.
    pub picks: Vec<DVec2>,
    /// Lines continue from the last end.
    pub chain: bool,
    /// Sides of the next polygon.
    pub sides: usize,
    pub selected: BTreeSet<usize>,
    /// The curve picked first by *Trim corner* (with where it was clicked)
    /// or the reference of *Trim/Extend*.
    picked: Option<(usize, DVec2)>,
    /// Where a selection window started.
    boxing: Option<DVec2>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// The grip being dragged, and where the curves were when it started.
    drag: Option<(Grip, Vec<SketchCurve>)>,
    /// What a sketch of the component environment makes.
    pub piece: Option<crate::component_sketch::PieceSketch>,
    /// A terrain's measured points (plan position and elevation above the
    /// level); the elevation of the next one is in the options bar
    /// ([`crate::modify::ModifyOptions::point_elevation`]).
    pub points: Vec<DVec3>,
    /// The picked points (indices into `points`), deleted with the curves.
    pub selected_points: BTreeSet<usize>,
    /// Values typed after the first point (a length and an angle, or a
    /// rectangle's width and depth; Tab switches the field).
    pub typed: TypedInput,
    /// The lines each *From Walls* added, side by side along the walls'
    /// loop, so a new wall face moves those not changed since (#496).
    pub from_walls: Vec<Vec<SketchCurve>>,
}

impl Session {
    pub fn new(target: Target, curves: Vec<SketchCurve>) -> Self {
        Self {
            target,
            curves,
            draw: Draw::Line,
            picks: Vec::new(),
            chain: true,
            sides: 6,
            selected: BTreeSet::new(),
            picked: None,
            boxing: None,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            piece: None,
            points: Vec::new(),
            selected_points: BTreeSet::new(),
            typed: TypedInput::default(),
            from_walls: Vec::new(),
        }
    }

    /// The tool takes typed values: it has its first point and the next
    /// one ends a segment from it (a line, a radius, an axis, a move) or is
    /// a rectangle's opposite corner.
    pub fn typing(&self) -> bool {
        self.picks.len() == 1
            && matches!(
                self.draw,
                Draw::Line
                    | Draw::Rect
                    | Draw::Polygon
                    | Draw::PolygonOut
                    | Draw::Circle
                    | Draw::Arc3
                    | Draw::ArcCenter
                    | Draw::Ellipse
                    | Draw::Move
                    | Draw::Copy
            )
    }

    /// The point the cursor `p` stands for with the typed values: along
    /// the cursor's direction from the last pick at the typed length (and
    /// angle from the X axis), or a rectangle's corner with the typed width
    /// and depth on the cursor's side.
    pub fn typed_point(&self, p: DVec2) -> DVec2 {
        let Some(&a) = self.picks.last() else { return p };
        if !self.typing() || self.typed.is_empty() {
            return p;
        }
        if self.draw != Draw::Rect {
            return self.typed.apply(a, p, None);
        }
        let d = p - a;
        let side = |v: f64| if v < 0.0 { -1.0 } else { 1.0 };
        let dx = self.typed.length_value().map_or(d.x, |w| side(d.x) * w);
        let dy = self.typed.second_length().map_or(d.y, |h| side(d.y) * h);
        a + DVec2::new(dx, dy)
    }

    /// Nothing is picked: no curve and no point.
    pub fn nothing_selected(&self) -> bool {
        self.selected.is_empty() && self.selected_points.is_empty()
    }

    /// Unpick every curve and point.
    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.selected_points.clear();
    }

    /// The point nearest `p` in plan within `tol`.
    fn point_at(&self, p: DVec2, tol: f64) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .map(|(i, q)| (i, q.truncate().distance(p)))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { curves: self.curves.clone(), points: self.points.clone(), from_walls: self.from_walls.clone() }
    }

    fn restore(&mut self, to: Snapshot) -> Snapshot {
        let now = Snapshot {
            curves: std::mem::replace(&mut self.curves, to.curves),
            points: std::mem::take(&mut self.points),
            from_walls: std::mem::replace(&mut self.from_walls, to.from_walls),
        };
        self.points = to.points;
        now
    }

    /// Add a terrain point (one undo step).
    pub fn add_point(&mut self, p: DVec3) {
        if p.is_finite() {
            self.remember();
            self.points.push(p);
        }
    }

    /// The sketch is a sweep's path, open by design.
    pub fn open_path(&self) -> bool {
        self.piece.as_ref().is_some_and(|p| p.kind == crate::component_sketch::PieceKind::Sweep)
    }

    /// Keep the curves as they are for undo, before a change.
    fn remember(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn add(&mut self, curves: Vec<SketchCurve>) {
        let curves: Vec<SketchCurve> = curves.into_iter().filter(SketchCurve::is_valid).collect();
        if curves.is_empty() {
            return;
        }
        self.remember();
        self.curves.extend(curves);
    }

    /// Add the lines *From Walls* made (one undo step), remembering them so
    /// a new wall face can move them.
    pub fn add_from_walls(&mut self, curves: Vec<SketchCurve>) {
        let curves: Vec<SketchCurve> = curves.into_iter().filter(SketchCurve::is_valid).collect();
        if curves.is_empty() {
            return;
        }
        self.remember();
        self.curves.extend(curves.iter().cloned());
        self.from_walls.push(curves);
    }

    /// Move the lines *From Walls* added, those still as they were made, to
    /// `fresh`: the same walls' loop on another face, side for side (the
    /// loop has the same sides in the same order whatever the face). Lines
    /// the user moved, trimmed or deleted stay as they are. One undo step;
    /// returns how many lines moved.
    pub fn refit_walls(&mut self, fresh: &[SketchCurve]) -> usize {
        let mut groups = self.from_walls.clone();
        let mut moves: Vec<(usize, SketchCurve)> = Vec::new();
        for group in groups.iter_mut().filter(|g| g.len() == fresh.len()) {
            for (old, new) in group.iter_mut().zip(fresh) {
                let taken = |j: usize| moves.iter().any(|(k, _)| *k == j);
                let found = self.curves.iter().enumerate().position(|(j, c)| c == old && !taken(j));
                if let Some(j) = found {
                    moves.push((j, new.clone()));
                    *old = new.clone();
                }
            }
        }
        if moves.is_empty() {
            return 0;
        }
        self.remember();
        for (j, new) in &moves {
            if let Some(c) = self.curves.get_mut(*j) {
                *c = new.clone();
            }
        }
        self.from_walls = groups;
        moves.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            let now = self.restore(prev);
            self.redo.push(now);
            self.clear_selection();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let now = self.restore(next);
            self.undo.push(now);
            self.clear_selection();
        }
    }

    /// Remove the selected curves and points (one undo step).
    pub fn delete_selected(&mut self) {
        if self.nothing_selected() {
            return;
        }
        self.remember();
        let selected = std::mem::take(&mut self.selected);
        let mut i = 0;
        self.curves.retain(|_| {
            let keep = !selected.contains(&i);
            i += 1;
            keep
        });
        let selected = std::mem::take(&mut self.selected_points);
        let mut i = 0;
        self.points.retain(|_| {
            let keep = !selected.contains(&i);
            i += 1;
            keep
        });
    }

    /// Change curve `i` (one undo step).
    pub fn set_curve(&mut self, i: usize, curve: SketchCurve) {
        if !curve.is_valid() || self.curves.get(i) == Some(&curve) {
            return;
        }
        self.remember();
        if let Some(c) = self.curves.get_mut(i) {
            *c = curve;
        }
    }

    /// Change some curves and add others, in one undo step.
    fn change(&mut self, changes: &[(usize, SketchCurve)], added: &[SketchCurve]) {
        if changes.iter().map(|(_, c)| c).chain(added).any(|c| !c.is_valid()) {
            return;
        }
        self.remember();
        for (i, c) in changes {
            if let Some(slot) = self.curves.get_mut(*i) {
                *slot = c.clone();
            }
        }
        self.curves.extend_from_slice(added);
    }

    /// The picked curves moved by `t` (one undo step), or copies of them;
    /// returns the copies' indices.
    pub fn transform_selected(&mut self, t: &PlanTransform, copy: bool) -> BTreeSet<usize> {
        let moved: Vec<(usize, SketchCurve)> = self
            .selected
            .iter()
            .filter_map(|&i| {
                let mut c = self.curves.get(i)?.clone();
                c.transform(t);
                Some((i, c))
            })
            .collect();
        if moved.is_empty() {
            return BTreeSet::new();
        }
        let start = self.curves.len();
        if copy {
            let added: Vec<SketchCurve> = moved.iter().map(|(_, c)| c.clone()).collect();
            self.change(&[], &added);
            (start..self.curves.len()).collect()
        } else {
            self.change(&moved, &[]);
            BTreeSet::new()
        }
    }

    /// Ends, centers and middles to snap to.
    pub fn snap_points(&self) -> Vec<DVec2> {
        let mut out = Vec::new();
        for c in &self.curves {
            if let Some((a, b)) = c.ends() {
                out.extend([a, b]);
            }
            match *c {
                SketchCurve::Line { a, b } => out.push((a + b) / 2.0),
                SketchCurve::Arc { a, b, bulge } => {
                    if let Some(arc) = Arc::from_bulge(a, b, bulge) {
                        out.extend([arc.center, arc.point(arc.length() / 2.0)]);
                    }
                }
                SketchCurve::Circle { center, .. } | SketchCurve::Ellipse { center, .. } => out.push(center),
                // The points it passes through.
                SketchCurve::Spline { ref points } => out.extend(points.iter().step_by(3).copied()),
            }
        }
        out
    }

    /// The curve nearest `p` within `tol`.
    fn curve_at(&self, p: DVec2, tol: f64) -> Option<usize> {
        self.curves
            .iter()
            .enumerate()
            .map(|(i, c)| (i, distance(c, p)))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

/// Distance from `p` to a curve (through its chords).
fn distance(c: &SketchCurve, p: DVec2) -> f64 {
    c.points()
        .windows(2)
        .map(|w| cc_geom::polygon::closest_on_segment(w[0], w[1], p).0.distance(p))
        .fold(f64::INFINITY, f64::min)
}

/// The grips of a curve, each with its handle number (see [`moved`]).
pub(crate) fn curve_grips(c: &SketchCurve) -> Vec<(u8, DVec2)> {
    grips(c, 0).into_iter().map(|(g, p)| (g.which, p)).collect()
}

/// The grips of curve `i`.
fn grips(c: &SketchCurve, curve: usize) -> Vec<(Grip, DVec2)> {
    let g = |which| Grip { curve, which };
    match *c {
        SketchCurve::Line { a, b } => vec![(g(0), a), (g(1), b)],
        SketchCurve::Arc { a, b, bulge } => {
            let mut out = vec![(g(0), a), (g(1), b)];
            if let Some(arc) = Arc::from_bulge(a, b, bulge) {
                out.push((g(2), arc.point(arc.length() / 2.0)));
            }
            out
        }
        SketchCurve::Circle { center, radius } => vec![(g(3), center), (g(4), center + DVec2::X * radius)],
        SketchCurve::Ellipse { center, radii, rotation_deg, .. } => {
            let turn = DVec2::from_angle(rotation_deg.to_radians());
            let mut out = vec![
                (g(3), center),
                (g(4), center + turn.rotate(DVec2::X * radii.x)),
                (g(5), center + turn.rotate(DVec2::Y * radii.y)),
            ];
            if let Some((a, b)) = c.ends() {
                out.extend([(g(0), a), (g(1), b)]);
            }
            out
        }
        // Its ends (0, 1, so joined ends move together), then every other
        // control point: the points it passes through and their handles.
        SketchCurve::Spline { ref points } => {
            let last = points.len().saturating_sub(1);
            let mut out: Vec<(Grip, DVec2)> = Vec::with_capacity(points.len());
            if let Some((a, b)) = c.ends() {
                out.push((g(0), a));
                if !c.is_closed() {
                    out.push((g(1), b));
                }
            }
            for (k, p) in points.iter().enumerate().skip(1).take(last.saturating_sub(1)) {
                if let Ok(which) = u8::try_from(SPLINE_GRIP + k) {
                    out.push((g(which), *p));
                }
            }
            out
        }
    }
}

fn spline_closed(points: &[DVec2]) -> bool {
    cc_core::spline::is_closed(points)
}

/// The grip number of a spline's control point 0 (see [`grips`]).
const SPLINE_GRIP: usize = 10;

/// A spline with its control point `k` moved to `p`: a point it passes
/// through takes its handles along; a handle keeps the curve smooth there,
/// turning the other handle of its point to stay in line.
pub(crate) fn spline_moved(points: &[DVec2], closed: bool, k: usize, p: DVec2) -> Vec<DVec2> {
    let mut out = points.to_vec();
    let last = out.len().saturating_sub(1);
    let Some(&old) = out.get(k) else { return out };
    let shift = p - old;
    match k % 3 {
        0 => {
            let mut moving = vec![k];
            if closed && (k == 0 || k == last) {
                moving = vec![0, last];
            }
            for &i in &moving {
                if let Some(q) = out.get_mut(i) {
                    *q += shift;
                }
            }
            // Its handles: next to it, and across the closing point.
            let mut handles = vec![k.wrapping_sub(1), k + 1];
            if closed && (k == 0 || k == last) {
                handles = vec![1, last.wrapping_sub(1)];
            }
            for i in handles {
                if let Some(q) = out.get_mut(i) {
                    *q += shift;
                }
            }
        }
        r => {
            if let Some(q) = out.get_mut(k) {
                *q = p;
            }
            // Its point, and the handle on the other side of it.
            let (knot, other) = if r == 1 { (k - 1, k.wrapping_sub(2)) } else { (k + 1, k + 2) };
            let (knot, other) = match (closed, knot, other) {
                (true, 0, _) => (0, last.wrapping_sub(1)),
                (true, kn, _) if kn == last => (last, 1),
                _ => (knot, other),
            };
            if let (Some(&center), Some(&o)) = (out.get(knot), out.get(other)) {
                let length = o.distance(center);
                if let Some(dir) = (center - p).try_normalize()
                    && let Some(q) = out.get_mut(other)
                {
                    *q = center + dir * length;
                }
            }
        }
    }
    out
}

/// Curve `c` with its grip `which` moved to `p`.
pub(crate) fn moved(c: &SketchCurve, which: u8, p: DVec2) -> SketchCurve {
    if let SketchCurve::Spline { points } = c {
        let k = match which {
            0 => 0,
            1 => points.len().saturating_sub(1),
            w => (w as usize).saturating_sub(SPLINE_GRIP),
        };
        return SketchCurve::Spline { points: spline_moved(points, c.is_closed(), k, p) };
    }
    match (c.clone(), which) {
        (SketchCurve::Line { b, .. }, 0) => SketchCurve::Line { a: p, b },
        (SketchCurve::Line { a, .. }, 1) => SketchCurve::Line { a, b: p },
        (SketchCurve::Arc { b, bulge, .. }, 0) => SketchCurve::Arc { a: p, b, bulge },
        (SketchCurve::Arc { a, bulge, .. }, 1) => SketchCurve::Arc { a, b: p, bulge },
        (SketchCurve::Arc { a, b, bulge }, 2) => {
            SketchCurve::Arc { a, b, bulge: Arc::bulge_through(a, p, b).unwrap_or(bulge) }
        }
        (SketchCurve::Circle { radius, .. }, 3) => SketchCurve::Circle { center: p, radius },
        (SketchCurve::Circle { center, .. }, 4) => SketchCurve::Circle { center, radius: center.distance(p) },
        (SketchCurve::Ellipse { radii, rotation_deg, start_deg, sweep_deg, .. }, 3) => {
            SketchCurve::Ellipse { center: p, radii, rotation_deg, start_deg, sweep_deg }
        }
        (SketchCurve::Ellipse { center, radii, start_deg, sweep_deg, .. }, 4) => {
            let d = p - center;
            SketchCurve::Ellipse {
                center,
                radii: DVec2::new(d.length(), radii.y),
                rotation_deg: d.y.atan2(d.x).to_degrees(),
                start_deg,
                sweep_deg,
            }
        }
        (SketchCurve::Ellipse { center, radii, rotation_deg, start_deg, sweep_deg }, 5) => {
            let axis = DVec2::from_angle(rotation_deg.to_radians()).perp();
            SketchCurve::Ellipse {
                center,
                radii: DVec2::new(radii.x, (p - center).dot(axis).abs()),
                rotation_deg,
                start_deg,
                sweep_deg,
            }
        }
        _ => c.clone(),
    }
}

/// The direction a curve leaves its end `at` (0: its start, 1: its end),
/// pointing away from it.
fn end_tangent(c: &SketchCurve, at: u8) -> Option<DVec2> {
    let pts = c.points();
    let (p, q) = match at {
        0 => (*pts.get(1)?, *pts.first()?),
        _ => (*pts.get(pts.len().checked_sub(2)?)?, *pts.last()?),
    };
    (q - p).try_normalize()
}

/// The element whose boundary *Edit Boundary* would open: one selected
/// floor, ceiling, roof or mass.
pub fn boundary_target(state: &AppState) -> Option<ElementId> {
    let mut sel = state.selection.iter();
    let (Some(id), None) = (sel.next(), sel.next()) else { return None };
    state.project().element(*id).filter(|e| e.sketched_profile().is_some()).map(|e| e.id)
}

/// Open the sketch of the selected element's outline (*Edit Boundary*).
pub fn edit_boundary(state: &mut AppState) {
    let Some(id) = boundary_target(state) else { return };
    let Some(e) = state.project().element(id) else { return };
    let (category, type_id) = (e.category(), e.type_id);
    let curves = match (e.drawn_sketch(), e.sketched_profile()) {
        (Some(s), _) => s.curves.clone(),
        (None, Some((outline, openings))) => Sketch::from_polygons(outline, openings).curves,
        _ => return,
    };
    // A region's own view shows it; else the element's level shows its
    // outline: open its plan if another view is active.
    let level = e.level;
    if let Some(view) = e.owner_view() {
        if state.active_view != Some(view) {
            state.open_view(view);
        }
    } else if state.active_level() != Some(level)
        && let Some(view) = state.project().views.values().find(|v| v.level() == Some(level)).map(|v| v.id)
    {
        state.open_view(view);
    }
    state.set_tool(Tool::Sketch { category, type_id });
    let mut session = Session::new(Target::Edit { id }, curves);
    session.draw = Draw::Select;
    if let Some(cc_core::ElementKind::Topography(t)) = state.project().element(id).map(|e| &e.kind) {
        session.points = t.points.clone();
    }
    state.sketch = Some(session);
    state.selection.clear();
}

/// The open sketch makes (or edits) a terrain, which takes measured points.
pub fn takes_points(state: &AppState) -> bool {
    match state.sketch.as_ref().map(|s| s.target) {
        Some(Target::New { category, .. }) => category == Category::Topography,
        Some(Target::Edit { id }) => state.project().element(id).is_some_and(|e| e.category() == Category::Topography),
        _ => false,
    }
}

/// Leave the sketch without changing the model.
pub fn cancel(state: &mut AppState) {
    let keep_tool = state.sketch.as_ref().is_some_and(|s| s.piece.is_some() || s.target == Target::Sheet);
    // A structural profile's sketch goes back to the Profile Manager.
    if let Some(Target::Section { profile }) = state.sketch.as_ref().map(|s| s.target) {
        crate::profile_sketch::leave_section(state, profile);
        return;
    }
    state.sketch = None;
    if let Some(ed) = state.component_env.as_mut().and_then(|e| e.sheet.as_mut()) {
        ed.editing = None;
    }
    if !keep_tool {
        state.set_tool(Tool::Select);
    }
}

/// Make the element (or its new outline) from the sketch; a sketch that
/// does not close shows where and stays open.
pub fn finish(state: &mut AppState) {
    let Some(session) = state.sketch.clone() else { return };
    let sketch = Sketch { curves: session.curves.clone() };
    let allows_openings = match session.target {
        Target::New { category, .. } => crate::region_ui::takes_openings(category),
        Target::Edit { id } => {
            state.project().element(id).is_some_and(|e| crate::region_ui::takes_openings(e.category()))
        }
        Target::Piece => {
            crate::component_sketch::finish(state);
            return;
        }
        Target::Profile { .. } => false,
        Target::Section { .. } => {
            crate::profile_sketch::finish_section(state);
            return;
        }
        Target::WallProfile { .. } => {
            crate::wall_profile_ui::finish(state);
            return;
        }
        Target::Sheet => {
            crate::title_block_env::finish_stroke(state);
            return;
        }
        Target::Detail | Target::Walls => return,
    };
    let checked = if allows_openings { sketch.profile().map(|_| ()) } else { sketch.outline().map(|_| ()) };
    if let Err(e) = checked {
        state.message = error_text(&e);
        return;
    }
    let curves: Vec<CurveArg> = session.curves.iter().map(CurveArg::of).collect();
    let out = match session.target {
        Target::Edit { id } => {
            // A terrain's boundary and points change in one command (one undo step).
            let points = match state.project().element(id).map(|e| &e.kind) {
                Some(cc_core::ElementKind::Topography(t)) if t.points != session.points => {
                    Some(session.points.iter().map(|p| p.to_array()).collect())
                }
                _ => None,
            };
            let command = cc_engine::commands::sketch::SketchSet { id: id.0, curves, points };
            crate::actions::run_command(state, command).map(|_| id)
        }
        Target::New { category, type_id } => {
            // The type picked in the type selector since the sketch opened.
            let type_id = match state.tool {
                Tool::Sketch { type_id, .. } => type_id,
                _ => type_id,
            };
            // Regions go in the active view, in its drawing coordinates.
            if matches!(category, Category::FilledRegion | Category::MaskingRegion) {
                crate::region_ui::create(state, category, type_id, curves)
            } else {
                let Some(level) = state.active_level() else {
                    state.message = t!("msg-open-plan");
                    return;
                };
                create(state, category, type_id, level, curves, &session.points)
            }
        }
        Target::Profile { type_id } => {
            let command = cc_engine::commands::sweep::SweepProfileSet {
                type_ref: Ref::Id(type_id.0),
                profile: None,
                curves: Some(curves),
            };
            if crate::actions::run_command(state, command).is_some() {
                state.sketch = None;
                state.set_tool(Tool::Select);
            }
            return;
        }
        Target::Piece
        | Target::Section { .. }
        | Target::WallProfile { .. }
        | Target::Sheet
        | Target::Detail
        | Target::Walls => None,
    };
    if let Some(id) = out {
        state.sketch = None;
        state.set_tool(Tool::Select);
        state.selection = BTreeSet::from([id]);
    }
}

/// Run the command that makes a new element from `curves`.
fn create(
    state: &mut AppState,
    category: Category,
    type_id: ElementTypeId,
    level: LevelId,
    curves: Vec<CurveArg>,
    points: &[DVec3],
) -> Option<ElementId> {
    use cc_engine::commands::create::{CeilingCreate, FloorCreate, RoofCreate, SketchArgs};
    let sketch = SketchArgs {
        curves: Some(curves.clone()),
        level: Some(Ref::Id(level.0)),
        type_ref: Some(Ref::Id(type_id.0)),
        ..Default::default()
    };
    let out = match category {
        Category::Floor => {
            crate::actions::run_command(state, FloorCreate { sketch, openings: Vec::new(), structural: None })
        }
        Category::Ceiling => crate::actions::run_command(state, CeilingCreate { sketch }),
        Category::Foundation => {
            crate::actions::run_command(state, cc_engine::commands::foundation::FoundationSlabCreate { sketch })
        }
        Category::Mass => crate::actions::run_command(
            state,
            cc_engine::commands::mass::MassCreate {
                curves: Some(curves),
                level: Some(Ref::Id(level.0)),
                type_ref: Some(Ref::Id(type_id.0)),
                ..Default::default()
            },
        ),
        Category::Shaft => crate::actions::run_command(
            state,
            cc_engine::commands::shaft::ShaftCreate {
                curves: Some(curves),
                level: Some(Ref::Id(level.0)),
                type_ref: Some(Ref::Id(type_id.0)),
                ..Default::default()
            },
        ),
        Category::Pad => {
            let elevation = state.modify.pad_elevation;
            crate::actions::run_command(
                state,
                cc_engine::commands::pad::PadCreate {
                    curves: Some(curves),
                    level: Some(Ref::Id(level.0)),
                    elevation,
                    type_ref: Some(Ref::Id(type_id.0)),
                    ..Default::default()
                },
            )
        }
        Category::Topography => crate::actions::run_command(
            state,
            cc_engine::commands::topography::TopographyCreate {
                points: points.iter().map(|p| p.to_array()).collect(),
                curves: Some(curves),
                level: Some(Ref::Id(level.0)),
                type_ref: Some(Ref::Id(type_id.0)),
                ..Default::default()
            },
        ),
        _ => crate::actions::run_command(state, RoofCreate { sketch, overhang: None, holes: Vec::new() }),
    };
    out.and_then(|o| o["id"].as_u64()).map(ElementId)
}

/// A sketch error as a message.
pub fn error_text(e: &SketchError) -> String {
    crate::i18n::tr(e.key())
}

/// Escape: end the spline or drop the curve being drawn, then leave the drawing tool, then the selection.
pub fn escape(state: &mut AppState) {
    if let Some(s) = state.sketch.as_mut() {
        escape_session(s);
    }
}

/// [`escape`] in the sketch `s`.
fn escape_session(s: &mut Session) {
    // A revolution waiting for its axis goes back to its outline.
    if let Some(p) = s.piece.as_mut().filter(|p| p.axis) {
        p.axis = false;
        return;
    }
    // Typed values go first.
    if !s.typed.is_empty() || s.typed.angle_active {
        s.typed.clear();
    } else if s.draw == Draw::Spline && s.picks.len() >= 2 {
        // A spline ends where it is, as a chain of lines does (issue #401).
        finish_spline(s, false);
    } else if !s.picks.is_empty() || s.picked.is_some() {
        s.picks.clear();
        s.picked = None;
    } else if s.draw != Draw::Select {
        s.draw = Draw::Select;
    } else {
        s.clear_selection();
    }
}

/// Pick a drawing or modify tool; those acting on the picked curves need some.
pub fn set_draw(state: &mut AppState, draw: Draw) {
    let Some(s) = state.sketch.as_mut() else { return };
    if draw.on_selection() && s.selected.is_empty() {
        state.message = t!("msg-select-first");
        return;
    }
    s.draw = draw;
    s.picks.clear();
    s.picked = None;
    s.typed.clear();
}

/// The category of the element sketched in a plan, whose outline may follow the walls.
fn walls_category(state: &AppState) -> Option<Category> {
    match state.sketch.as_ref().map(|s| s.target)? {
        Target::New { category, .. } => Some(category),
        Target::Edit { id } => Some(state.project().element(id).map_or(Category::Floor, |e| e.category())),
        Target::Piece
        | Target::Profile { .. }
        | Target::Section { .. }
        | Target::WallProfile { .. }
        | Target::Sheet
        | Target::Detail
        | Target::Walls => None,
    }
}

/// The face of the walls *From Walls* follows: the one chosen in the options
/// bar, else the outside finish faces (the inside ones for ceilings).
fn wall_face(state: &AppState, category: Category) -> cc_core::LocationLine {
    state.modify.wall_face.unwrap_or_else(|| cc_engine::default_wall_face(category))
}

/// Add lines along the loop of the level's walls, on the face of each wall
/// chosen in the options bar (issue #311).
pub fn pick_walls(state: &mut AppState) {
    let Some(level) = state.active_level() else {
        state.message = t!("msg-open-plan");
        return;
    };
    let Some(category) = walls_category(state) else { return };
    match cc_engine::walls_loop_at(state.project(), level, wall_face(state, category)) {
        Some(pts) => {
            let lines = Sketch::from_polygons(&pts, &[]).curves;
            if let Some(s) = state.sketch.as_mut() {
                s.add_from_walls(lines);
            }
        }
        None => state.message = t!("msg-no-wall-loop"),
    }
}

/// Choose the face of the walls *From Walls* follows; the lines it already
/// added and that were not changed since move to the new face (#496).
pub fn set_wall_face(state: &mut AppState, face: cc_core::LocationLine) {
    state.modify.wall_face = Some(face);
    let Some(level) = state.active_level() else { return };
    if state.sketch.as_ref().is_none_or(|s| s.from_walls.is_empty()) {
        return;
    }
    let Some(pts) = cc_engine::walls_loop_at(state.project(), level, face) else { return };
    let fresh: Vec<SketchCurve> =
        Sketch::from_polygons(&pts, &[]).curves.into_iter().filter(SketchCurve::is_valid).collect();
    if let Some(s) = state.sketch.as_mut() {
        s.refit_walls(&fresh);
    }
}

/// The sketch in a plan (or, for regions, any view that holds drawings,
/// with no level): its curves, the tool's preview, snapping, picks and grips.
pub fn canvas(ui: &Ui, resp: &Response, painter: &Painter, state: &mut AppState, level: Option<LevelId>, xf: &Xf) {
    let Some(mut s) = state.sketch.take() else { return };
    let raw = resp.hover_pos().map(|p| xf.to_world(p));
    let shift = ui.input(|i| i.modifiers.shift);
    let extra = s.snap_points();
    let anchor = s.picks.last().copied();
    let snapped = raw.map(|r| snap_at(state, level, r, anchor, &extra, xf.scale, shift));
    if let Some(message) = interact(ui, resp, painter, &mut s, &state.modify, raw, snapped, xf) {
        state.message = message;
    }
    state.sketch = Some(s);
}

/// The sketch in a canvas (a plan, or a view of the component environment):
/// its curves, where it does not close, and the active tool at the cursor
/// (`raw`, and `snapped` where it snaps). Returns a message for the status bar.
#[allow(clippy::too_many_arguments)]
pub fn interact(
    ui: &Ui,
    resp: &Response,
    painter: &Painter,
    s: &mut Session,
    opts: &crate::modify::ModifyOptions,
    raw: Option<DVec2>,
    snapped: Option<Snap>,
    xf: &Xf,
) -> Option<String> {
    let tol = PICK_PX / xf.scale;
    // The curves; the selected ones highlighted; where the sketch does not close.
    for (i, c) in s.curves.iter().enumerate() {
        let color = if s.selected.contains(&i) { HIGHLIGHT } else { SKETCH };
        paint_curve(painter, xf, c, 1.8, color);
    }
    if let Err(e) = (Sketch { curves: s.curves.clone() }).loops()
        && let Some(p) = e.point()
        && s.picks.is_empty()
        && !s.open_path()
    {
        painter.circle_stroke(xf.to_screen(p), 6.0, Stroke::new(1.5, PALETTE.danger));
    }
    paint_points(painter, xf, &s.points, &s.selected_points);
    // Typed values belong to the segment being drawn; Enter places them
    // as a click would (issue #179).
    if !s.typing() {
        s.typed.clear();
    }
    let enter = !s.typed.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    // Enter ends a spline where it is.
    if s.draw == Draw::Spline && s.typed.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        finish_spline(s, false);
    }
    let click = resp.clicked() || enter;
    let had_typed = !s.typed.is_empty();
    if s.draw == Draw::Select {
        select(ui, resp, painter, s, raw, snapped.map(|x| x.point), xf, tol);
        None
    } else if s.draw == Draw::Point {
        let sn = snapped?;
        draw_snap(painter, xf, sn);
        if resp.clicked() {
            s.add_point(sn.point.extend(opts.point_elevation));
        }
        None
    } else if s.draw.modifies() {
        let (Some(raw), Some(sn)) = (raw, snapped) else { return None };
        draw_snap(painter, xf, sn);
        let p = s.typed_point(sn.point);
        let message = modify(click, painter, s, opts, raw, p, xf, tol);
        if click && had_typed {
            s.typed.clear();
        }
        message
    } else {
        let sn = snapped?;
        draw_snap(painter, xf, sn);
        let p = s.typed_point(sn.point);
        draw(click, painter, s, p, xf, tol);
        if click && had_typed {
            s.typed.clear();
        }
        arc_tangent_hint(s, sn.point, tol)
    }
}

/// A rectangle's temporary dimensions: its width and depth outside it,
/// with the typed values.
fn rect_dimensions(painter: &Painter, xf: &Xf, a: DVec2, c: DVec2, typed: &TypedInput) {
    let center = (a + c) * 0.5;
    let corner = DVec2::new(c.x, a.y);
    for (u, v, text, active) in
        [(a, corner, &typed.length, !typed.angle_active), (corner, c, &typed.angle, typed.angle_active)]
    {
        // Dimension lines go on the left of their segment: run it so that is outside.
        let (u, v) = if (v - u).perp().dot(center - u) > 0.0 { (v, u) } else { (u, v) };
        dimension_line(painter, xf, u, v, text, active, 0.0);
    }
}

/// A terrain's points: small crosses with their elevations, the picked ones highlighted.
fn paint_points(painter: &Painter, xf: &Xf, points: &[DVec3], selected: &BTreeSet<usize>) {
    // Labels only while they can be read.
    let labels = points.len() <= 400;
    for (i, p) in points.iter().enumerate() {
        let color = if selected.contains(&i) { HIGHLIGHT } else { SKETCH };
        let stroke = Stroke::new(1.4, color);
        let c = xf.to_screen(p.truncate());
        painter.line_segment([c + egui::vec2(-4.0, -4.0), c + egui::vec2(4.0, 4.0)], stroke);
        painter.line_segment([c + egui::vec2(-4.0, 4.0), c + egui::vec2(4.0, -4.0)], stroke);
        if labels {
            painter.text(
                c + egui::vec2(6.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                cc_views::format::length(p.z),
                egui::FontId::proportional(10.0),
                color,
            );
        }
    }
}

/// The tangent arc needs a free end to start from.
fn arc_tangent_hint(s: &Session, p: DVec2, tol: f64) -> Option<String> {
    (s.draw == Draw::ArcTangent && s.picks.is_empty() && free_end(s, p, tol).is_none())
        .then(|| t!("hint-sketch-arc-tangent"))
}

/// A free end of the sketch near `p`: the curve, which end and where.
fn free_end(s: &Session, p: DVec2, tol: f64) -> Option<(usize, u8, DVec2)> {
    let ends: Vec<(usize, u8, DVec2)> = s
        .curves
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.ends().map(|(a, b)| [(i, 0, a), (i, 1, b)]))
        .flatten()
        .collect();
    ends.iter()
        .copied()
        .filter(|(i, _, q)| q.distance(p) <= tol && !ends.iter().any(|(j, _, r)| j != i && r.distance(*q) <= JOIN_TOL))
        .min_by(|a, b| a.2.distance(p).total_cmp(&b.2.distance(p)))
}

/// Picking and dragging grips in the *Select* mode.
#[allow(clippy::too_many_arguments)]
fn select(
    ui: &Ui,
    resp: &Response,
    painter: &Painter,
    s: &mut Session,
    raw: Option<DVec2>,
    snapped: Option<DVec2>,
    xf: &Xf,
    tol: f64,
) {
    let grip_list: Vec<(Grip, DVec2)> =
        s.selected.iter().filter_map(|&i| s.curves.get(i).map(|c| grips(c, i))).flatten().collect();
    for (_, p) in &grip_list {
        let c = xf.to_screen(*p);
        painter.rect_filled(egui::Rect::from_center_size(c, egui::vec2(7.0, 7.0)), 1.0, PALETTE.paper);
        painter.rect_stroke(
            egui::Rect::from_center_size(c, egui::vec2(7.0, 7.0)),
            1.0,
            Stroke::new(1.2, HIGHLIGHT),
            egui::StrokeKind::Middle,
        );
    }
    let Some(raw) = raw else { return };
    // Dragging a grip: ends joined to it move along, so loops stay closed.
    // Dragged elsewhere, a window picks curves.
    if resp.drag_started_by(egui::PointerButton::Primary) {
        match grip_list.iter().find(|(_, p)| p.distance(raw) <= tol * 1.5) {
            Some((grip, _)) => s.drag = Some((*grip, s.curves.clone())),
            None => s.boxing = Some(raw),
        }
    }
    if let Some(start) = s.boxing {
        let rect = egui::Rect::from_two_pos(xf.to_screen(start), xf.to_screen(raw));
        // Dragged to the left, the window picks the curves it touches.
        let touching = xf.to_screen(raw).x < xf.to_screen(start).x;
        painter.rect_filled(rect, 0.0, HIGHLIGHT.gamma_multiply(0.08));
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, HIGHLIGHT), egui::StrokeKind::Middle);
        if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
            let picked: BTreeSet<usize> = s
                .curves
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    let pts: Vec<egui::Pos2> = c.points().iter().map(|p| xf.to_screen(*p)).collect();
                    if touching {
                        pts.windows(2).any(|w| segment_touches(w[0], w[1], rect))
                    } else {
                        pts.iter().all(|p| rect.contains(*p))
                    }
                })
                .map(|(i, _)| i)
                .collect();
            // Points inside the window, either way it was dragged.
            let points: BTreeSet<usize> = (0..s.points.len())
                .filter(|&i| s.points.get(i).is_some_and(|p| rect.contains(xf.to_screen(p.truncate()))))
                .collect();
            if ui.input(|i| i.modifiers.command) {
                s.selected.extend(picked);
                s.selected_points.extend(points);
            } else {
                s.selected = picked;
                s.selected_points = points;
            }
            s.boxing = None;
        }
        return;
    }
    if let Some((grip, before)) = s.drag.clone() {
        let p = snapped.unwrap_or(raw);
        let mut curves = before.clone();
        if let Some(c) = before.get(grip.curve) {
            let at = grips(c, grip.curve).into_iter().find(|(g, _)| *g == grip).map(|(_, q)| q);
            if let (Some(at), true) = (at, grip.which <= 1) {
                // Every curve end at the grip's point follows it.
                for (k, other) in before.iter().enumerate() {
                    if let Some((a, b)) = other.ends() {
                        let mut c2 = other.clone();
                        if a.distance(at) <= JOIN_TOL {
                            c2 = moved(&c2, 0, p);
                        }
                        if b.distance(at) <= JOIN_TOL {
                            c2 = moved(&c2, 1, p);
                        }
                        curves[k] = c2;
                    }
                }
            } else {
                curves[grip.curve] = moved(c, grip.which, p);
            }
        }
        if curves.iter().all(SketchCurve::is_valid) {
            s.curves = curves;
        }
        if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
            if s.curves != before {
                s.undo.push(Snapshot { curves: before, points: s.points.clone(), from_walls: s.from_walls.clone() });
                s.redo.clear();
            }
            s.drag = None;
        }
        return;
    }
    // A terrain's point under the cursor comes before the curves (Ctrl adds
    // or removes it), so points on the boundary can be picked too.
    if let Some(i) = s.point_at(raw, tol) {
        if !s.selected_points.contains(&i)
            && let Some(p) = s.points.get(i)
        {
            painter.circle_stroke(xf.to_screen(p.truncate()), 7.0, Stroke::new(2.0, HIGHLIGHT.gamma_multiply(0.6)));
        }
        if resp.clicked() {
            if ui.input(|i| i.modifiers.command) {
                if !s.selected_points.remove(&i) {
                    s.selected_points.insert(i);
                }
            } else {
                s.selected.clear();
                s.selected_points = BTreeSet::from([i]);
            }
        }
        return;
    }
    // Hover and click pick a curve (Ctrl adds or removes it).
    let hit = s.curve_at(raw, tol);
    if let Some(i) = hit
        && !s.selected.contains(&i)
        && let Some(c) = s.curves.get(i)
    {
        let pts: Vec<egui::Pos2> = c.points().iter().map(|p| xf.to_screen(*p)).collect();
        painter.add(egui::Shape::line(pts, Stroke::new(2.4, HIGHLIGHT.gamma_multiply(0.6))));
    }
    if resp.double_clicked()
        && let Some(i) = hit
    {
        // The chain of curves joined to it.
        s.selected = sketch_edit::chain(&s.curves, i).0.into_iter().map(|(k, _)| k).collect();
    } else if resp.clicked() {
        let ctrl = ui.input(|i| i.modifiers.command);
        match (hit, ctrl) {
            (Some(i), true) => {
                if !s.selected.remove(&i) {
                    s.selected.insert(i);
                }
            }
            (Some(i), false) => {
                s.selected = BTreeSet::from([i]);
                s.selected_points.clear();
            }
            (None, false) => s.clear_selection(),
            (None, true) => {}
        }
    }
}

/// The drawing tools: preview at `p`, and a click adds a point or the curve.
pub(crate) fn draw(click: bool, painter: &Painter, s: &mut Session, p: DVec2, xf: &Xf, tol: f64) {
    let preview = |painter: &Painter, curves: &[SketchCurve]| {
        for c in curves.iter().filter(|c| c.is_valid()) {
            let pts: Vec<egui::Pos2> = c.points().iter().map(|q| xf.to_screen(*q)).collect();
            painter.add(egui::Shape::line(pts, Stroke::new(1.2, PREVIEW)));
        }
    };
    if s.draw == Draw::Spline {
        draw_spline(click, painter, s, p, xf, tol);
        return;
    }
    let picks = s.picks.clone();
    let shape = next_curves(s, &picks, p, tol);
    match (&shape, picks.last()) {
        (Some(curves), _) => preview(painter, curves),
        (None, Some(&last)) => preview(painter, &[SketchCurve::Line { a: last, b: p }]),
        _ => {}
    }
    if let (Some(&a), true) = (picks.last(), s.typing()) {
        if s.draw == Draw::Rect {
            rect_dimensions(painter, xf, a, p, &s.typed);
        } else {
            temp_dimension(painter, xf, a, p, None, Some(&s.typed), 0.0);
        }
    }
    if !click {
        return;
    }
    let needed = match s.draw {
        Draw::Line | Draw::Rect | Draw::Polygon | Draw::PolygonOut | Draw::Circle | Draw::ArcTangent => 2,
        Draw::Arc3 | Draw::ArcCenter | Draw::Ellipse => 3,
        _ => return,
    };
    if picks.len() + 1 < needed {
        // The tangent arc starts at a free end of the sketch.
        let start = match s.draw {
            Draw::ArcTangent => match free_end(s, p, tol) {
                Some((_, _, q)) => q,
                None => return,
            },
            _ => p,
        };
        if picks.last().is_none_or(|l| l.distance(start) > JOIN_TOL) {
            s.picks.push(start);
        }
        return;
    }
    let Some(curves) = shape else { return };
    s.add(curves);
    s.picks.clear();
    // Lines and tangent arcs go on from their end, until the chain closes.
    let closed = |s: &Session| (Sketch { curves: s.curves.clone() }).loops().is_ok();
    if matches!(s.draw, Draw::Line | Draw::ArcTangent) && s.chain && !closed(s) {
        s.picks.push(p);
    }
}

/// The spline tool: the curve through the picks and the cursor; a click
/// adds a point, ends the curve on the last one or closes it on the first.
fn draw_spline(click: bool, painter: &Painter, s: &mut Session, p: DVec2, xf: &Xf, tol: f64) {
    let picks = s.picks.clone();
    let on_first = picks.len() >= 3 && picks.first().is_some_and(|f| f.distance(p) <= tol);
    let on_last = picks.len() >= 2 && picks.last().is_some_and(|l| l.distance(p) <= tol);
    let mut through = picks.clone();
    if !on_last && !on_first {
        through.push(p);
    }
    let points = cc_core::spline::interpolate(&through, on_first);
    let curve = SketchCurve::Spline { points };
    if curve.is_valid() {
        paint_curve(painter, xf, &curve, 1.2, PREVIEW);
    }
    for q in &picks {
        painter.circle_stroke(xf.to_screen(*q), 3.0, Stroke::new(1.2, PREVIEW));
    }
    if !click {
        return;
    }
    if on_first {
        finish_spline(s, true);
    } else if on_last {
        finish_spline(s, false);
    } else if picks.len() < cc_core::spline::MAX_SEGMENTS && picks.last().is_none_or(|l| l.distance(p) > JOIN_TOL) {
        s.picks.push(p);
    }
}

/// Add the spline through the picks (closed back to the first when
/// `closed`) and start a new one.
pub(crate) fn finish_spline(s: &mut Session, closed: bool) {
    let points = cc_core::spline::interpolate(&s.picks, closed);
    let curve = SketchCurve::Spline { points };
    if curve.is_valid() {
        s.add(vec![curve]);
    }
    s.picks.clear();
}

/// The curves the picks and the point `p` make with the active tool.
fn next_curves(s: &Session, picks: &[DVec2], p: DVec2, tol: f64) -> Option<Vec<SketchCurve>> {
    use cc_engine::commands::sketch::CurveArg as A;
    let deg = |v: DVec2| v.y.atan2(v.x).to_degrees();
    match (s.draw, picks) {
        (Draw::Line, [a]) => Some(vec![SketchCurve::Line { a: *a, b: p }]),
        (Draw::Rect, [a]) => A::Rectangle { corner: a.to_array(), opposite: p.to_array(), rotation: 0.0 }.curves().ok(),
        (Draw::Polygon | Draw::PolygonOut, [c]) => {
            let sides = s.sides.clamp(MIN_SIDES, MAX_SIDES);
            let inscribed = s.draw == Draw::Polygon;
            // Circumscribed, the cursor marks the middle of a side: the first corner is half a side further round.
            let half = if inscribed { 0.0 } else { 180.0 / sides as f64 };
            let shape = A::Polygon {
                center: c.to_array(),
                radius: c.distance(p),
                sides,
                inscribed,
                rotation: deg(p - *c) + half,
            };
            shape.curves().ok()
        }
        (Draw::Circle, [c]) => Some(vec![SketchCurve::Circle { center: *c, radius: c.distance(p) }]),
        (Draw::Arc3, [a]) => Some(vec![SketchCurve::Line { a: *a, b: p }]),
        (Draw::Arc3, [a, b]) => {
            Arc::bulge_through(*a, p, *b).map(|bulge| vec![SketchCurve::Arc { a: *a, b: *b, bulge }])
        }
        (Draw::ArcCenter, [c]) => Some(vec![SketchCurve::Circle { center: *c, radius: c.distance(p) }]),
        (Draw::ArcCenter, [c, start]) => {
            let r = c.distance(*start);
            let (u, v) = (*start - *c, p - *c);
            let end = *c + v.normalize_or_zero() * r;
            // The arc turns towards the cursor the short way round.
            let sweep = u.angle_to(v);
            let bulge = (sweep / 4.0).tan();
            (sweep.abs() > 1e-6).then(|| vec![SketchCurve::Arc { a: *start, b: end, bulge }])
        }
        (Draw::ArcTangent, [a]) => {
            let (i, which, _) = free_end(s, *a, tol)?;
            let tangent = end_tangent(s.curves.get(i)?, which)?;
            Arc::bulge_tangent(*a, tangent, p).map(|bulge| vec![SketchCurve::Arc { a: *a, b: p, bulge }])
        }
        (Draw::Ellipse, [c]) => Some(vec![SketchCurve::Line { a: *c, b: p }]),
        (Draw::Ellipse, [c, axis]) => {
            let d = *axis - *c;
            let ry = (p - *c).dot(d.perp().normalize_or_zero()).abs();
            Some(vec![SketchCurve::ellipse(*c, DVec2::new(d.length(), ry), deg(d))])
        }
        _ => None,
    }
}

/// Whether the segment `a`–`b` (screen points) touches `rect`.
fn segment_touches(a: egui::Pos2, b: egui::Pos2, rect: egui::Rect) -> bool {
    // Clip the segment's parameter to the rectangle's slabs.
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    let d = b - a;
    for (p, q) in [(-d.x, a.x - rect.min.x), (d.x, rect.max.x - a.x), (-d.y, a.y - rect.min.y), (d.y, rect.max.y - a.y)]
    {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    t0 <= t1
}

/// A curve of the sketch drawn over it.
fn paint_curve(painter: &Painter, xf: &Xf, c: &SketchCurve, width: f32, color: Color32) {
    let pts: Vec<egui::Pos2> = c.points().iter().map(|p| xf.to_screen(*p)).collect();
    painter.add(egui::Shape::line(pts, Stroke::new(width, color)));
}

/// The line of the sketch under the cursor, as a mirror axis.
fn mirror_axis(s: &Session, p: DVec2, tol: f64) -> Option<(DVec2, DVec2)> {
    s.curves
        .iter()
        .filter_map(|c| match *c {
            SketchCurve::Line { a, b } => Some((a, b, distance(c, p))),
            _ => None,
        })
        .filter(|(_, _, d)| *d <= tol)
        .min_by(|x, y| x.2.total_cmp(&y.2))
        .map(|(a, b, _)| (a, b))
}

/// The modify tools at the cursor (`raw`, snapped `p`): the transforms of
/// the picked curves preview and apply them; offset, trim and split act on
/// the curves clicked. Returns a message when a click does nothing.
#[allow(clippy::too_many_arguments)]
fn modify(
    click: bool,
    painter: &Painter,
    s: &mut Session,
    opts: &crate::modify::ModifyOptions,
    raw: DVec2,
    p: DVec2,
    xf: &Xf,
    tol: f64,
) -> Option<String> {
    let hit = s.curve_at(raw, tol);
    let hover = |painter: &Painter, i: Option<usize>| {
        if let Some(c) = i.and_then(|i| s.curves.get(i)) {
            paint_curve(painter, xf, c, 2.4, HIGHLIGHT.gamma_multiply(0.6));
        }
    };
    if s.draw.on_selection() {
        if let Some(&last) = s.picks.last() {
            painter.line_segment([xf.to_screen(last), xf.to_screen(p)], Stroke::new(1.0, PREVIEW));
        }
        let t = match (s.draw, s.picks.as_slice()) {
            (Draw::Move | Draw::Copy, [base]) => {
                temp_dimension(painter, xf, *base, p, None, Some(&s.typed), 0.0);
                Some(PlanTransform::translate(p - *base))
            }
            (Draw::Rotate, [center, from]) => {
                Some(PlanTransform::rotate_about(*center, (*from - *center).angle_to(p - *center)))
            }
            (Draw::MirrorAxis, [a]) => (a.distance(p) > JOIN_TOL).then(|| PlanTransform::mirror_across(*a, p - *a)),
            (Draw::MirrorPick, _) => {
                let axis = mirror_axis(s, raw, tol);
                if let Some((a, b)) = axis {
                    paint_curve(painter, xf, &SketchCurve::Line { a, b }, 2.4, HIGHLIGHT.gamma_multiply(0.6));
                }
                axis.map(|(a, b)| PlanTransform::mirror_across(a, b - a))
            }
            _ => None,
        };
        if let Some(t) = &t {
            for c in s.selected.iter().filter_map(|&i| s.curves.get(i)) {
                let mut c = c.clone();
                c.transform(t);
                paint_curve(painter, xf, &c, 1.2, PREVIEW);
            }
        }
        if !click {
            return None;
        }
        let Some(t) = t else {
            if s.draw == Draw::MirrorPick {
                return Some(t!("hint-sketch-mirror-pick"));
            }
            if s.picks.last().is_none_or(|l| l.distance(p) > JOIN_TOL) {
                s.picks.push(p);
            }
            return None;
        };
        let copy = match s.draw {
            Draw::Copy => true,
            Draw::Rotate => opts.rotate_copy,
            Draw::MirrorPick | Draw::MirrorAxis => opts.mirror_copy,
            _ => false,
        };
        let copies = s.transform_selected(&t, copy);
        // Copying several times goes on from the same base point.
        if s.draw == Draw::Copy && opts.multiple {
            return None;
        }
        if copy {
            s.selected = copies;
        }
        s.picks.clear();
        s.draw = Draw::Select;
        return None;
    }
    match s.draw {
        Draw::Offset => {
            hover(painter, hit);
            let i = hit?;
            let out = if s.chain {
                sketch_edit::offset_chain(&s.curves, i, opts.offset, raw)
            } else {
                s.curves.get(i).and_then(|c| sketch_edit::offset(c, opts.offset, raw)).map(|c| vec![c])
            };
            match out {
                Some(curves) => {
                    for c in &curves {
                        paint_curve(painter, xf, c, 1.2, PREVIEW);
                    }
                    if click {
                        s.add(curves);
                    }
                    None
                }
                None => click.then(|| t!("sketch-no-offset")),
            }
        }
        Draw::TrimCorner | Draw::TrimExtend => {
            let Some((first, at)) = s.picked else {
                hover(painter, hit);
                if click && let Some(i) = hit {
                    s.picked = Some((i, raw));
                }
                return None;
            };
            if let Some(c) = s.curves.get(first) {
                paint_curve(painter, xf, c, 2.4, HIGHLIGHT);
            }
            let j = hit.filter(|j| *j != first)?;
            let (c1, c2) = (s.curves.get(first)?.clone(), s.curves.get(j)?.clone());
            hover(painter, Some(j));
            let changes = if s.draw == Draw::TrimCorner {
                sketch_edit::trim_corner(&c1, at, &c2, raw).map(|(a, b)| vec![(first, a), (j, b)])
            } else {
                sketch_edit::trim_extend(&c2, raw, &c1).map(|b| vec![(j, b)])
            };
            match changes {
                Some(changes) => {
                    for (_, c) in &changes {
                        paint_curve(painter, xf, c, 1.2, PREVIEW);
                    }
                    if click {
                        s.change(&changes, &[]);
                        // Trim/Extend goes on with the same reference.
                        if s.draw == Draw::TrimCorner {
                            s.picked = None;
                        }
                    }
                    None
                }
                None if !click => None,
                None if (c1.is_closed() && s.draw == Draw::TrimCorner) || c2.is_closed() => Some(t!("sketch-no-trim")),
                None => Some(t!("sketch-no-meet")),
            }
        }
        Draw::Split => {
            hover(painter, hit);
            let i = hit?;
            let c = s.curves.get(i)?.clone();
            let Some((a, b)) = sketch_edit::split(&c, p) else {
                return click.then(|| t!("sketch-no-split"));
            };
            if let Some((_, at)) = a.ends() {
                painter.circle_stroke(xf.to_screen(at), 4.0, Stroke::new(1.5, HIGHLIGHT));
            }
            if click {
                s.change(&[(i, a)], &[b]);
                s.selected.clear();
            }
            None
        }
        _ => None,
    }
}

// --- Ribbon, options bar and properties ---

/// What the open sketch makes or edits, as the ribbon names it.
pub fn title(state: &AppState) -> String {
    use crate::commands::UiCommand::*;
    // The tool's name says what the sketch makes (the edited element's category opens it).
    let what = crate::i18n::tr(state.tool.key());
    let piece = crate::component_sketch::piece(state).map(|p| p.kind);
    match (state.sketch.as_ref().map(|s| s.target), piece) {
        (_, Some(kind)) => {
            let what = match kind {
                crate::component_sketch::PieceKind::Extrusion => CompExtrusion,
                crate::component_sketch::PieceKind::Revolution => CompRevolution,
                crate::component_sketch::PieceKind::Sweep => CompSweep,
                crate::component_sketch::PieceKind::Blend => CompBlend,
            };
            t!("sketch-title-piece", "kind" => what.label(state))
        }
        (Some(Target::Edit { .. }), _) => t!("sketch-title-edit", "category" => what),
        (Some(Target::Profile { type_id }), _) => {
            let name = state.project().element_type(type_id).map(|t| t.name.clone()).unwrap_or_default();
            t!("sketch-title-profile", "type" => name)
        }
        (Some(Target::Section { profile }), _) => {
            let name = state.project().profile(profile).map(|p| p.name.clone()).unwrap_or_default();
            t!("sketch-title-profile", "type" => name)
        }
        (Some(Target::WallProfile { wall }), _) => {
            let p = state.project();
            let name = p.element(wall).and_then(|e| p.element_type(e.type_id)).map(|t| t.name.clone());
            t!("sketch-title-wall-profile", "type" => name.unwrap_or_default())
        }
        (Some(Target::Sheet), _) => t!("sketch-title-sheet"),
        _ => t!("sketch-title-new", "category" => what),
    }
}

/// The sketch's own ribbon (in the component environment): its title over its groups.
pub fn ribbon(ui: &mut Ui, state: &mut AppState) {
    let title = title(state);
    ui.horizontal(|ui| {
        ui.label(theme::title(title));
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        groups(ui, state, false);
    });
}

/// The sketch's groups: Select, the Modify tools, the Draw group (the
/// drawing tools and what this sketch draws besides: lines from walls,
/// points of a terrain) and Mode (Finish, Cancel). In the main ribbon's
/// Modify tab (`contextual`) Draw and Mode stand on the accent ground, as
/// a placing tool's do.
pub fn groups(ui: &mut Ui, state: &mut AppState, contextual: bool) {
    use crate::commands::UiCommand::*;
    let piece = crate::component_sketch::piece(state).is_some();
    crate::ribbon::env_group(ui, state, t!("group-selection"), &[Select]);
    crate::ribbon::env_stacks(
        ui,
        state,
        t!("group-modify"),
        &[
            &[Move, CopyElements, Rotate],
            &[MirrorPick, MirrorDraw, Offset],
            &[TrimCorner, TrimExtend, Split],
            &[Delete],
        ],
    );
    // A wall's profile is one outline of lines and arcs.
    let wall_profile = crate::wall_profile_ui::is_open(state);
    let mut draw: Vec<&[_]> = if wall_profile {
        vec![&[SketchLine, SketchRect], &[SketchArc3, SketchArcCenter, SketchArcTangent]]
    } else {
        vec![
            &[SketchLine, SketchRect, SketchPolygon],
            &[SketchPolygonOut, SketchCircle, SketchEllipse],
            &[SketchArc3, SketchArcCenter, SketchArcTangent],
            &[SketchSpline],
        ]
    };
    // Walls are picked in plans only, for what stands on a level.
    let profile = matches!(
        state.sketch.as_ref().map(|s| s.target),
        Some(Target::Profile { .. } | Target::Section { .. } | Target::WallProfile { .. })
    );
    let sheet = matches!(state.sketch.as_ref().map(|s| s.target), Some(Target::Sheet));
    let picks: Vec<UiCommand> =
        [(!(piece || profile || sheet)).then_some(SketchPickWalls), takes_points(state).then_some(SketchPoint)]
            .into_iter()
            .flatten()
            .collect();
    if !picks.is_empty() {
        draw.push(&picks);
    }
    crate::ribbon::sketch_stacks(ui, state, t!("group-draw"), contextual, &draw);
    crate::ribbon::sketch_group(ui, state, t!("group-mode"), contextual, &[SketchFinish, SketchCancel]);
}

/// The options bar: chaining, polygon sides, the modify tools' options and the hint.
pub fn options(ui: &mut Ui, state: &mut AppState) {
    crate::component_sketch::options(ui, state);
    // Which face of the walls *From Walls* follows, in plans.
    if let Some(category) = walls_category(state).filter(|_| state.active_level().is_some()) {
        let mut face = wall_face(state, category);
        ui.label(t!("sketch-wall-face"));
        egui::ComboBox::from_id_salt("sketch-wall-face").selected_text(crate::i18n::tr(face.key())).show_ui(ui, |ui| {
            for option in cc_core::LocationLine::ALL {
                ui.selectable_value(&mut face, option, crate::i18n::tr(option.key()));
            }
        });
        if face != wall_face(state, category) {
            set_wall_face(state, face);
        }
        ui.separator();
    }
    let opts = &mut state.modify;
    let Some(s) = state.sketch.as_mut() else { return };
    // A new pad's elevation, whatever is being drawn.
    if matches!(s.target, Target::New { category: Category::Pad, .. }) {
        ui.label(t!("opt-elevation"));
        ui.add(crate::properties::length_drag(&mut opts.pad_elevation).range(-10_000.0..=10_000.0));
        ui.separator();
    }
    match s.draw {
        Draw::Line => {
            ui.checkbox(&mut s.chain, t!("sketch-chain"));
            ui.separator();
        }
        Draw::Polygon | Draw::PolygonOut => {
            ui.label(t!("sketch-sides"));
            ui.add(egui::DragValue::new(&mut s.sides).range(MIN_SIDES..=MAX_SIDES));
            ui.separator();
        }
        Draw::Copy => {
            ui.checkbox(&mut opts.multiple, t!("opt-multiple"));
            ui.separator();
        }
        Draw::Rotate => {
            ui.checkbox(&mut opts.rotate_copy, t!("opt-copy"));
            ui.separator();
            ui.label(t!("prop-rotation"));
            ui.add(egui::DragValue::new(&mut opts.angle).speed(1.0).suffix("°"));
            // About the center clicked, or the middle of the picked curves.
            if ui.button(t!("action-apply")).clicked()
                && let Some(center) = s.picks.first().copied().or_else(|| selection_center(s))
            {
                let copies = s.transform_selected(
                    &PlanTransform::rotate_about(center, opts.angle.to_radians()),
                    opts.rotate_copy,
                );
                if opts.rotate_copy {
                    s.selected = copies;
                }
                s.picks.clear();
                s.draw = Draw::Select;
            }
            ui.separator();
        }
        Draw::MirrorPick | Draw::MirrorAxis => {
            ui.checkbox(&mut opts.mirror_copy, t!("opt-copy"));
            ui.separator();
        }
        Draw::Point => {
            ui.label(t!("opt-elevation"));
            ui.add(crate::properties::length_drag(&mut opts.point_elevation).range(-10_000.0..=10_000.0));
            ui.separator();
        }
        Draw::Offset => {
            ui.label(t!("opt-distance"));
            ui.add(crate::properties::length_drag(&mut opts.offset).range(0.001..=100.0));
            ui.checkbox(&mut s.chain, t!("sketch-chain"));
            ui.separator();
        }
        _ => {}
    }
    let hint = match s.draw {
        Draw::TrimCorner if s.picked.is_some() => t!("hint-sketch-trim-corner-second"),
        Draw::TrimExtend if s.picked.is_some() => t!("hint-sketch-trim-extend-second"),
        draw => draw_hint(draw, s.picks.len()),
    };
    ui.label(theme::muted(hint));
}

/// What to click next with `draw`, `picks` points picked (also the Detail
/// Line tool's hint).
pub(crate) fn draw_hint(draw: Draw, picks: usize) -> String {
    match (draw, picks) {
        (Draw::Select, _) => t!("hint-sketch-select"),
        (Draw::Line, 0) => t!("hint-sketch-line"),
        (Draw::Line, _) => t!("hint-sketch-line-next"),
        (Draw::Rect, 0) => t!("hint-sketch-rect"),
        (Draw::Rect, _) => t!("hint-sketch-rect-next"),
        (Draw::Polygon | Draw::PolygonOut | Draw::Circle | Draw::ArcCenter | Draw::Ellipse, 0) => {
            t!("hint-sketch-center")
        }
        (Draw::Polygon, _) => t!("hint-sketch-corner"),
        (Draw::PolygonOut, _) => t!("hint-sketch-side"),
        (Draw::Circle, _) => t!("hint-sketch-radius"),
        (Draw::Arc3, 0) => t!("hint-sketch-arc-start"),
        (Draw::Arc3, 1) => t!("hint-sketch-arc-end"),
        (Draw::Arc3, _) => t!("hint-sketch-arc-through"),
        (Draw::ArcCenter, 1) => t!("hint-sketch-arc-start"),
        (Draw::ArcCenter, _) => t!("hint-sketch-arc-end"),
        (Draw::ArcTangent, 0) => t!("hint-sketch-arc-tangent"),
        (Draw::ArcTangent, _) => t!("hint-sketch-arc-end"),
        (Draw::Ellipse, 1) => t!("hint-sketch-axis"),
        (Draw::Spline, 0) => t!("hint-sketch-spline"),
        (Draw::Spline, _) => t!("hint-sketch-spline-next"),
        (Draw::Ellipse, _) => t!("hint-sketch-other-axis"),
        (Draw::Move | Draw::Copy, 0) => t!("hint-base-point"),
        (Draw::Move | Draw::Copy, _) => t!("hint-sketch-move-target"),
        (Draw::Rotate, 0) => t!("hint-sketch-rotate-center"),
        (Draw::Rotate, 1) => t!("hint-sketch-rotate-from"),
        (Draw::Rotate, _) => t!("hint-rotate-target"),
        (Draw::MirrorPick, _) => t!("hint-sketch-mirror-pick"),
        (Draw::MirrorAxis, _) => t!("hint-mirror-draw"),
        (Draw::Offset, _) => t!("hint-sketch-offset"),
        (Draw::TrimCorner, _) => t!("hint-sketch-trim-corner"),
        (Draw::TrimExtend, _) => t!("hint-sketch-trim-extend"),
        (Draw::Split, _) => t!("hint-sketch-split"),
        (Draw::Point, _) => t!("hint-sketch-point"),
    }
}

/// The middle of the picked curves' extents.
fn selection_center(s: &Session) -> Option<DVec2> {
    let pts = s.selected.iter().filter_map(|&i| s.curves.get(i)).flat_map(|c| c.points());
    let (lo, hi) =
        pts.fold(None, |acc: Option<(DVec2, DVec2)>, p| Some(acc.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p)))))?;
    Some((lo + hi) / 2.0)
}

/// The selected curve's values in the properties palette.
pub fn properties(ui: &mut Ui, state: &mut AppState) {
    use crate::properties::length_drag;
    use crate::table::{readonly_row, row, section};
    let Some(s) = state.sketch.as_mut() else { return };
    let mut sel = s.selected.iter();
    let (Some(&i), None) = (sel.next(), sel.next()) else {
        crate::table::padded(ui, |ui| {
            let text = match s.selected.len() {
                0 => t!("sketch-props-none"),
                n => t!("sketch-props-many", "count" => n),
            };
            ui.label(egui::RichText::new(text).color(PALETTE.text_faint));
        });
        return;
    };
    let Some(mut c) = s.curves.get(i).cloned() else { return };
    let before = c.clone();
    let point = |ui: &mut Ui, label: &str, p: &mut DVec2| {
        row(ui, label, |ui| {
            // X and Y share the value column.
            let w = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(40.0);
            ui.spacing_mut().interact_size.x = w;
            ui.add(length_drag(&mut p.x));
            ui.add(length_drag(&mut p.y));
        });
    };
    let title = match c {
        SketchCurve::Line { .. } => t!("sketch-line"),
        SketchCurve::Arc { .. } => t!("sketch-arc"),
        SketchCurve::Circle { .. } => t!("sketch-circle"),
        SketchCurve::Ellipse { .. } => t!("sketch-ellipse"),
        SketchCurve::Spline { .. } => t!("sketch-spline"),
    };
    section(ui, "sketch-curve", title, |ui| match &mut c {
        SketchCurve::Line { a, b } => {
            point(ui, &t!("prop-start"), a);
            point(ui, &t!("prop-end"), b);
            readonly_row(ui, &t!("prop-length"), cc_views::format::length(a.distance(*b)));
        }
        SketchCurve::Arc { a, b, bulge } => {
            point(ui, &t!("prop-start"), a);
            point(ui, &t!("prop-end"), b);
            if let Some(arc) = Arc::from_bulge(*a, *b, *bulge) {
                let mut radius = arc.radius;
                if row(ui, &t!("prop-radius"), |ui| ui.add(length_drag(&mut radius)).changed())
                    && let Some(v) = Arc::bulge_radius(*a, *b, radius, *bulge > 0.0, bulge.abs() > 1.0)
                {
                    *bulge = v;
                }
                readonly_row(ui, &t!("prop-length"), cc_views::format::length(arc.length()));
                readonly_row(ui, &t!("prop-angle"), cc_views::format::angle(arc.sweep.abs().to_degrees()));
            }
        }
        SketchCurve::Circle { center, radius } => {
            point(ui, &t!("prop-center"), center);
            row(ui, &t!("prop-radius"), |ui| ui.add(length_drag(radius)));
        }
        SketchCurve::Ellipse { center, radii, rotation_deg, .. } => {
            point(ui, &t!("prop-center"), center);
            row(ui, &t!("sketch-radius-x"), |ui| ui.add(length_drag(&mut radii.x)));
            row(ui, &t!("sketch-radius-y"), |ui| ui.add(length_drag(&mut radii.y)));
            row(ui, &t!("prop-rotation"), |ui| ui.add(egui::DragValue::new(rotation_deg).speed(1.0).suffix("°")));
        }
        SketchCurve::Spline { points } => {
            let count = points.len() / 3 + usize::from(!spline_closed(points));
            readonly_row(ui, &t!("sketch-spline-points"), count.to_string());
            readonly_row(ui, &t!("prop-length"), cc_views::format::length(cc_core::spline::length(points)));
        }
    });
    if c != before {
        s.set_curve(i, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(draw: Draw) -> Session {
        let mut s = Session::new(Target::New { category: Category::Mass, type_id: ElementTypeId(1) }, Vec::new());
        s.draw = draw;
        s
    }

    #[test]
    fn tools_make_their_curves() {
        let p = |x: f64, y: f64| DVec2::new(x, y);
        let s = session(Draw::Rect);
        assert_eq!(next_curves(&s, &[p(0.0, 0.0)], p(4.0, 2.0), 0.1).unwrap().len(), 4);
        let mut s = session(Draw::PolygonOut);
        s.sides = 4;
        let square = next_curves(&s, &[p(0.0, 0.0)], p(1.0, 0.0), 0.1).unwrap();
        // Circumscribed with a side's middle at (1, 0): a 2 m square.
        let sketch = Sketch { curves: square };
        let outline = sketch.outline().unwrap();
        assert!(outline.iter().any(|q| q.distance(p(1.0, 1.0)) < 1e-9), "{outline:?}");
        let s = session(Draw::Arc3);
        let arc = next_curves(&s, &[p(1.0, 0.0), p(-1.0, 0.0)], p(0.0, 1.0), 0.1).unwrap();
        assert!(matches!(arc[0], SketchCurve::Arc { bulge, .. } if (bulge - 1.0).abs() < 1e-9));
        let s = session(Draw::Ellipse);
        let e = next_curves(&s, &[p(0.0, 0.0), p(0.0, 3.0)], p(2.0, 1.0), 0.1).unwrap();
        assert!(matches!(e[0], SketchCurve::Ellipse { radii, rotation_deg, .. }
            if (radii - p(3.0, 2.0)).length() < 1e-9 && (rotation_deg - 90.0).abs() < 1e-9));
    }

    /// The spline tool: clicks add points the curve passes through; a click
    /// on the last point ends it, on the first closes it, Enter ends it.
    #[test]
    fn the_spline_tool_draws_through_its_points() {
        let ctx = egui::Context::default();
        let painter = egui::Painter::new(ctx, egui::LayerId::background(), egui::Rect::EVERYTHING);
        let xf = Xf { screen: egui::Pos2::new(0.0, 800.0), origin: DVec2::ZERO, scale: 100.0 };
        let p = |x: f64, y: f64| DVec2::new(x, y);
        let mut s = session(Draw::Spline);
        for q in [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0)] {
            draw(true, &painter, &mut s, q, &xf, 0.05);
        }
        assert_eq!(s.picks.len(), 3);
        // The last point again: an open spline through the three.
        draw(true, &painter, &mut s, p(2.0, 0.0), &xf, 0.05);
        assert!(s.picks.is_empty());
        let Some(SketchCurve::Spline { points }) = s.curves.first() else { panic!("{:?}", s.curves) };
        assert_eq!(cc_core::spline::segment_count(points), Some(2));
        assert!(cc_core::spline::point_at(points, 1.0).unwrap().distance(p(1.0, 1.0)) < 1e-12);
        // Back on the first point: closed.
        for q in [p(0.0, 3.0), p(1.0, 4.0), p(2.0, 3.0), p(0.0, 3.0)] {
            draw(true, &painter, &mut s, q, &xf, 0.05);
        }
        assert!(s.curves.get(1).is_some_and(|c| c.is_closed()), "{:?}", s.curves);
        assert!((Sketch { curves: vec![s.curves[1].clone()] }).outline().is_ok());
        // Enter ends it where it is; one point makes nothing.
        draw(true, &painter, &mut s, p(5.0, 5.0), &xf, 0.05);
        draw(true, &painter, &mut s, p(6.0, 5.0), &xf, 0.05);
        finish_spline(&mut s, false);
        assert_eq!(s.curves.len(), 3);
        draw(true, &painter, &mut s, p(9.0, 9.0), &xf, 0.05);
        finish_spline(&mut s, false);
        assert_eq!(s.curves.len(), 3);
        // Esc ends it where it is too (issue #401), drops a single point, then leaves the tool.
        draw(true, &painter, &mut s, p(7.0, 7.0), &xf, 0.05);
        draw(true, &painter, &mut s, p(8.0, 7.0), &xf, 0.05);
        escape_session(&mut s);
        assert_eq!((s.curves.len(), s.picks.len(), s.draw), (4, 0, Draw::Spline));
        draw(true, &painter, &mut s, p(9.0, 9.0), &xf, 0.05);
        escape_session(&mut s);
        assert_eq!((s.curves.len(), s.picks.len(), s.draw), (4, 0, Draw::Spline));
        escape_session(&mut s);
        assert_eq!((s.curves.len(), s.draw), (4, Draw::Select));
        // Its grips: a point it passes through takes its handles along.
        let spline = s.curves[0].clone();
        let SketchCurve::Spline { points } = &spline else { panic!() };
        let moved = moved(&spline, (SPLINE_GRIP + 3) as u8, p(1.0, 2.0));
        let SketchCurve::Spline { points: after } = &moved else { panic!() };
        assert_eq!(after[3], p(1.0, 2.0));
        assert!((after[2] - points[2] - p(0.0, 1.0)).length() < 1e-12);
        assert!((after[4] - points[4] - p(0.0, 1.0)).length() < 1e-12);
    }

    #[test]
    fn tangent_arcs_continue_a_line() {
        let mut s = session(Draw::ArcTangent);
        s.curves.push(SketchCurve::Line { a: DVec2::ZERO, b: DVec2::new(4.0, 0.0) });
        // From the line's end, turning back to (4, 2): a half circle.
        let arc = next_curves(&s, &[DVec2::new(4.0, 0.0)], DVec2::new(4.0, 2.0), 0.1).unwrap();
        assert!(matches!(arc[0], SketchCurve::Arc { bulge, .. } if (bulge - 1.0).abs() < 1e-9), "{arc:?}");
        // Only free ends start one.
        assert!(free_end(&s, DVec2::new(4.0, 0.0), 0.1).is_some());
        s.curves.push(SketchCurve::Line { a: DVec2::new(4.0, 0.0), b: DVec2::new(4.0, 4.0) });
        assert!(free_end(&s, DVec2::new(4.0, 0.0), 0.1).is_none());
    }

    /// Issue #179: the values typed after the first point.
    #[test]
    fn typed_values_place_the_next_point() {
        let a = DVec2::new(1.0, 1.0);
        let cursor = DVec2::new(-2.0, 3.0);
        let typed = |draw: Draw, length: &str, second: &str| {
            let mut s = session(draw);
            s.picks.push(a);
            s.typed.length = length.into();
            s.typed.angle = second.into();
            s.typed_point(cursor)
        };
        // Nothing typed, or no first point: the cursor.
        assert_eq!(typed(Draw::Rect, "", ""), cursor);
        assert!(!session(Draw::Rect).typing() && session(Draw::Rect).typed_point(cursor) == cursor);
        // A rectangle: width and depth on the cursor's side; either alone.
        assert!(typed(Draw::Rect, "5", "4").distance(DVec2::new(-4.0, 5.0)) < 1e-9);
        assert!(typed(Draw::Rect, "5", "").distance(DVec2::new(-4.0, 3.0)) < 1e-9);
        assert!(typed(Draw::Rect, "", "0,5").distance(DVec2::new(-2.0, 1.5)) < 1e-9);
        // A circle's radius and a move's distance go towards the cursor; an angle turns them.
        let toward = (cursor - a).normalize();
        assert!(typed(Draw::Circle, "2", "").distance(a + toward * 2.0) < 1e-9);
        assert!(typed(Draw::Move, "2", "").distance(a + toward * 2.0) < 1e-9);
        assert!(typed(Draw::Line, "2", "90").distance(DVec2::new(1.0, 3.0)) < 1e-9);
        // Tools without a segment from their pick take none.
        assert_eq!(typed(Draw::Rotate, "2", ""), cursor);
        // A rectangle's second field takes lengths, letters nowhere.
        let mut s = session(Draw::Rect);
        s.typed.angle_active = true;
        assert!(s.typed.type_char('3', true) && !s.typed.type_char('x', true));
        assert_eq!(s.typed.second_length(), Some(3.0));
    }

    #[test]
    fn picked_curves_move_copy_and_change_in_one_step() {
        let mut s = session(Draw::Select);
        let rect = CurveArg::Rectangle { corner: [0.0, 0.0], opposite: [4.0, 2.0], rotation: 0.0 }.curves().unwrap();
        s.add(rect);
        s.selected = BTreeSet::from([0, 1]);
        let copies = s.transform_selected(&PlanTransform::translate(DVec2::new(0.0, 10.0)), true);
        assert_eq!((s.curves.len(), copies), (6, BTreeSet::from([4, 5])));
        assert_eq!(s.curves[4], SketchCurve::Line { a: DVec2::new(0.0, 10.0), b: DVec2::new(4.0, 10.0) });
        // Moved, not copied: one undo step brings them back.
        s.transform_selected(&PlanTransform::rotate_about(DVec2::ZERO, std::f64::consts::PI), false);
        assert!(matches!(s.curves[0], SketchCurve::Line { b, .. } if b.distance(DVec2::new(-4.0, 0.0)) < 1e-9));
        s.undo();
        assert_eq!(s.curves[0], SketchCurve::Line { a: DVec2::ZERO, b: DVec2::new(4.0, 0.0) });
        // A change with an invalid curve changes nothing.
        let before = s.curves.clone();
        s.change(&[(0, SketchCurve::Line { a: DVec2::ZERO, b: DVec2::ZERO })], &[]);
        assert_eq!(s.curves, before);
        // Windows touch the segments that cross them, even with both ends outside.
        let r = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, 10.0));
        assert!(segment_touches(egui::pos2(-5.0, 5.0), egui::pos2(15.0, 5.0), r));
        assert!(!segment_touches(egui::pos2(-5.0, 15.0), egui::pos2(15.0, 12.0), r));
        // Tools on the picked curves need some.
        s.selected.clear();
        assert!(Draw::Move.on_selection() && !Draw::Offset.on_selection() && Draw::Split.modifies());
    }

    #[test]
    fn grips_keep_loops_closed_and_undo() {
        let mut s = session(Draw::Select);
        let rect = CurveArg::Rectangle { corner: [0.0, 0.0], opposite: [4.0, 2.0], rotation: 0.0 }.curves().unwrap();
        s.add(rect);
        assert!(s.can_undo());
        let circle = SketchCurve::Circle { center: DVec2::new(2.0, 1.0), radius: 0.5 };
        assert_eq!(
            moved(&circle, 4, DVec2::new(2.0, 1.8)),
            SketchCurve::Circle { center: DVec2::new(2.0, 1.0), radius: 0.8 }
        );
        s.set_curve(0, SketchCurve::Line { a: DVec2::ZERO, b: DVec2::new(5.0, 0.0) });
        assert!((Sketch { curves: s.curves.clone() }).loops().is_err(), "an end moved alone opens the loop");
        s.undo();
        assert!((Sketch { curves: s.curves.clone() }).loops().is_ok());
        s.selected.insert(0);
        s.delete_selected();
        assert_eq!(s.curves.len(), 3);
        s.undo();
        s.undo();
        assert!(s.curves.is_empty());
        s.redo();
        assert_eq!(s.curves.len(), 4);
    }

    #[test]
    fn terrain_points_are_picked_and_deleted_with_the_curves() {
        let mut s = session(Draw::Select);
        s.add(CurveArg::Rectangle { corner: [0.0, 0.0], opposite: [4.0, 2.0], rotation: 0.0 }.curves().unwrap());
        for x in [1.0, 2.0, 3.0] {
            s.add_point(DVec3::new(x, 1.0, x / 10.0));
        }
        assert_eq!(s.point_at(DVec2::new(2.05, 1.0), 0.1), Some(1));
        assert_eq!(s.point_at(DVec2::new(2.5, 1.0), 0.1), None);
        // A curve and two points go in one step, and come back with undo.
        s.selected.insert(0);
        s.selected_points.extend([0, 2]);
        assert!(!s.nothing_selected());
        s.delete_selected();
        assert!(s.nothing_selected());
        assert_eq!((s.curves.len(), s.points.len()), (3, 1));
        assert_eq!(s.points[0], DVec3::new(2.0, 1.0, 0.2));
        s.undo();
        assert_eq!((s.curves.len(), s.points.len()), (4, 3));
    }
}
