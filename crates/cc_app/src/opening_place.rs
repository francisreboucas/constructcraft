//! Placing a door or window on a wall in a plan: where its center goes
//! along the wall, from the cursor, a snap of its center or one of its
//! edges to an imported drawing or the underlay (issue #251), or a
//! distance typed from either end of the wall (issue #250), and the
//! temporary dimensions from its center to both ends.

use crate::tools::{TEMP_DIM, TypedInput, dim_text};
use cc_core::cad::CadSnap;
use cc_geom::WallGeom;
use cc_views::format;
use egui::{Painter, Stroke};
use glam::DVec2;

/// Clearance kept between an opening and the ends of its wall.
const MARGIN: f64 = 0.01;

/// Which part of the opening a snap lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Center,
    /// The edge toward the wall's start.
    StartEdge,
    /// The edge toward the wall's end.
    EndEdge,
}

/// A feature of a drawing the opening was lined up with: the point snapped
/// to and the part of the opening on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapped {
    pub point: DVec2,
    pub part: Part,
}

/// Where the opening goes: its center along the wall, or why it cannot.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Center along the wall (wall-local `u`).
    pub offset: f64,
    pub snapped: Option<Snapped>,
    /// The typed distance places it past an end of the wall.
    pub outside: bool,
}

/// The range of centers an opening `width` wide may take on `g`, or `None`
/// when the wall is too short for it.
pub fn center_range(g: &WallGeom, width: f64) -> Option<(f64, f64)> {
    let (lo, hi) = (width * 0.5 + MARGIN, g.length - width * 0.5 - MARGIN);
    (lo <= hi).then_some((lo, hi))
}

/// The center of an opening `width` wide on `g` for the cursor at wall-local
/// `(u, v)`: a typed distance first (the first field from the wall's start,
/// the second, after Tab, from its end; the active one wins when both are
/// typed), else a snap of its center or an edge to `features` within `r`,
/// else the cursor; always kept inside the wall. `None` when the wall is too short.
pub fn place(
    g: &WallGeom,
    width: f64,
    (u, v): (f64, f64),
    features: &CadSnap,
    r: f64,
    typed: &TypedInput,
) -> Option<Placement> {
    let (lo, hi) = center_range(g, width)?;
    if let Some(offset) = typed_offset(g, typed) {
        let outside = !(lo - 1e-9..=hi + 1e-9).contains(&offset);
        return Some(Placement { offset: offset.clamp(lo, hi), snapped: None, outside });
    }
    let snapped = snap(g, width, (u, v), features, r);
    let center = snapped.map_or(u, |(c, _)| c);
    let offset = center.clamp(lo, hi);
    // A snap the wall's ends would move is no snap.
    let snapped = snapped.filter(|(c, _)| (c - offset).abs() < 1e-9).map(|(_, s)| s);
    Some(Placement { offset, snapped, outside: false })
}

/// The center a typed distance asks for, along the wall from its start.
fn typed_offset(g: &WallGeom, typed: &TypedInput) -> Option<f64> {
    let from_start = typed.length_value();
    let from_end = typed.second_length().map(|d| g.length - d);
    if typed.angle_active { from_end.or(from_start) } else { from_start.or(from_end) }
}

/// The nearest feature lining up the opening's center (at `u`) or one of
/// its edges: points within `r` of where that part crosses the cursor's
/// line along the wall, and lines crossing the wall (or coming within `r`
/// of its faces) within `r` of that part. Returns the center that lines it up.
fn snap(g: &WallGeom, width: f64, (u, v): (f64, f64), f: &CadSnap, r: f64) -> Option<(f64, Snapped)> {
    let half = width * 0.5;
    let parts = [(Part::Center, 0.0), (Part::StartEdge, -half), (Part::EndEdge, half)];
    let mut best: Option<(f64, f64, Snapped)> = None;
    let mut consider = |d: f64, center: f64, s: Snapped| {
        if d <= r && best.is_none_or(|b| d < b.0) {
            best = Some((d, center, s));
        }
    };
    for (part, shift) in parts {
        let at = u + shift;
        let probe = g.world(at, v);
        for p in f.ends.iter().chain(&f.centers).chain(&f.mids) {
            let (pu, _) = g.local(*p);
            consider(p.distance(probe), pu - shift, Snapped { point: *p, part });
        }
        for (a, b) in &f.lines {
            let Some(cu) = crossing(g, *a, *b, r) else { continue };
            let point = g.world(cu, 0.0);
            consider((cu - at).abs(), cu - shift, Snapped { point, part });
        }
    }
    best.map(|(_, c, s)| (c, s))
}

/// Where the line from `a` to `b` crosses the wall's centerline, as `u`, for
/// lines across the wall that reach within `r` of it (a line along the wall
/// crosses nothing).
fn crossing(g: &WallGeom, a: DVec2, b: DVec2, r: f64) -> Option<f64> {
    let ((ua, va), (ub, vb)) = (g.local(a), g.local(b));
    let (du, dv) = (ub - ua, vb - va);
    // Lines more along the wall than across it are its faces, not marks on it.
    if dv.abs() < 1e-9 || du.abs() > dv.abs() {
        return None;
    }
    let t = (-va / dv).clamp(0.0, 1.0);
    let (u, v) = (ua + du * t, va + dv * t);
    (v.abs() <= g.half + r && (0.0..=g.length).contains(&u)).then_some(u)
}

/// Draw the temporary dimensions from the opening's center at `offset` to
/// both ends of the wall, beside its face on side `side` (1 left, -1
/// right), with the typed values in place of the measures being typed.
pub fn paint_dimensions(
    painter: &Painter,
    xf: &crate::paint::Xf,
    g: &WallGeom,
    offset: f64,
    side: f64,
    typed: &TypedInput,
) {
    let v = side * (g.half + 22.0 / xf.scale);
    let thin = Stroke::new(1.0, TEMP_DIM);
    // Witness lines from the face out past the dimension line.
    for u in [0.0, offset, g.length] {
        let from = g.world(u, side * (g.half + 3.0 / xf.scale));
        let to = g.world(u, v + side * 5.0 / xf.scale);
        painter.line_segment([xf.to_screen(from), xf.to_screen(to)], thin);
    }
    let fields =
        [(0.0, offset, &typed.length, !typed.angle_active), (offset, g.length, &typed.angle, typed.angle_active)];
    for (u0, u1, text, active) in fields {
        if u1 - u0 < 1e-6 {
            continue;
        }
        let pts: Vec<egui::Pos2> = g.path(u0, u1, v).into_iter().map(|p| xf.to_screen(p)).collect();
        painter.add(egui::Shape::line(pts, thin));
        for u in [u0, u1] {
            let tick = (g.tangent(u) + g.normal_at(u)).normalize_or_zero() * (5.0 / xf.scale);
            let q = g.world(u, v);
            painter.line_segment([xf.to_screen(q - tick), xf.to_screen(q + tick)], Stroke::new(1.5, TEMP_DIM));
        }
        let mid = (u0 + u1) * 0.5;
        let screen_dir = xf.to_screen(g.tangent(mid)) - xf.to_screen(DVec2::ZERO);
        let pos = xf.to_screen(g.world(mid, v + side * 10.0 / xf.scale));
        dim_text(painter, pos, value_text(u1 - u0, text), screen_dir.y.atan2(screen_dir.x), active);
    }
}

/// A measure, or the value typed in its place.
fn value_text(measure: f64, typed: &str) -> String {
    let unit = format::current().0.length.unit;
    match (typed.is_empty(), unit.is_fractional()) {
        (true, _) => format::length(measure),
        // Feet and inches carry their own marks.
        (false, true) => format!("{typed} \u{23ce}"),
        (false, false) => format!("{typed} {} \u{23ce}", format::unit_symbol(unit.symbol())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight wall 6 m long along X from the origin.
    fn wall() -> WallGeom {
        let mut s = cc_engine::Session::default();
        let wall = cc_engine::commands::create::WallCreate { start: [0.0, 0.0], end: [6.0, 0.0], ..Default::default() };
        s.run(wall).unwrap();
        s.geometry().walls.values().next().cloned().unwrap()
    }

    fn typed(length: &str, angle: &str, angle_active: bool) -> TypedInput {
        TypedInput { length: length.into(), angle: angle.into(), angle_active }
    }

    #[test]
    fn follows_the_cursor_inside_the_wall() {
        let g = wall();
        let none = CadSnap::default();
        let p = place(&g, 1.0, (2.3, 0.0), &none, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 2.3).abs() < 1e-9 && p.snapped.is_none());
        // Kept off the ends.
        let p = place(&g, 1.0, (0.1, 0.0), &none, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 0.51).abs() < 1e-9);
        assert!(place(&g, 7.0, (3.0, 0.0), &none, 0.05, &TypedInput::default()).is_none());
    }

    #[test]
    fn typed_distances_measure_from_either_end() {
        let g = wall();
        let none = CadSnap::default();
        let p = place(&g, 1.0, (4.0, 0.0), &none, 0.05, &typed("1,5", "", false)).unwrap();
        assert!((p.offset - 1.5).abs() < 1e-9 && !p.outside);
        // From the end after Tab.
        let p = place(&g, 1.0, (1.0, 0.0), &none, 0.05, &typed("1,5", "2", true)).unwrap();
        assert!((p.offset - 4.0).abs() < 1e-9);
        // The other field when the active one is empty.
        let p = place(&g, 1.0, (1.0, 0.0), &none, 0.05, &typed("", "2", false)).unwrap();
        assert!((p.offset - 4.0).abs() < 1e-9);
        // Past the end of the wall.
        let p = place(&g, 1.0, (1.0, 0.0), &none, 0.05, &typed("5,8", "", false)).unwrap();
        assert!(p.outside);
    }

    #[test]
    fn centers_and_edges_snap_to_lines_across_the_wall() {
        let g = wall();
        // A window's axis drawn across the wall, past both faces, at x = 2.
        let f = CadSnap { lines: vec![(DVec2::new(2.0, -0.6), DVec2::new(2.0, 0.6))], ..Default::default() };
        let p = place(&g, 1.0, (2.03, 0.05), &f, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 2.0).abs() < 1e-9);
        assert_eq!(p.snapped.map(|s| s.part), Some(Part::Center));
        // The cursor half a width further: the edge toward the start lines up.
        let p = place(&g, 1.0, (2.52, 0.05), &f, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 2.5).abs() < 1e-9);
        assert_eq!(p.snapped.map(|s| s.part), Some(Part::StartEdge));
        // Lines along the wall (its faces in the drawing) are no snaps.
        let faces = CadSnap { lines: vec![(DVec2::new(0.0, 0.1), DVec2::new(6.0, 0.1))], ..Default::default() };
        let p = place(&g, 1.0, (2.03, 0.05), &faces, 0.05, &TypedInput::default()).unwrap();
        assert!(p.snapped.is_none());
        // A typed distance wins over snaps.
        let p = place(&g, 1.0, (2.03, 0.05), &f, 0.05, &typed("3", "", false)).unwrap();
        assert!((p.offset - 3.0).abs() < 1e-9 && p.snapped.is_none());
    }

    #[test]
    fn points_outside_the_wall_snap_the_center() {
        let g = wall();
        // The end of an axis 0.5 m in front of the wall.
        let f = CadSnap { ends: vec![DVec2::new(3.2, 0.6)], ..Default::default() };
        let p = place(&g, 0.8, (3.17, 0.58), &f, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 3.2).abs() < 1e-9);
        assert_eq!(p.snapped.map(|s| s.point), Some(DVec2::new(3.2, 0.6)));
    }

    /// The lines of a plan's underlay (other levels drawn under it) are
    /// snapped to, its own lines are left to the model snaps.
    #[test]
    fn underlay_lines_are_snap_features() {
        use cc_views::{LineStyle, Prim};
        let line = |a: DVec2, b: DVec2| Prim::Line { a, b, style: LineStyle::Faint, element: None, pen: None };
        let mut d = cc_views::Drawing2D::default();
        d.prims.push(line(DVec2::new(2.0, -1.0), DVec2::new(2.0, 1.0)));
        d.prims.push(line(DVec2::new(9.0, -1.0), DVec2::new(9.0, 1.0)));
        d.underlay = 2;
        d.prims.push(line(DVec2::new(2.5, -1.0), DVec2::new(2.5, 1.0)));
        let mut f = CadSnap::default();
        crate::tools::underlay_snap(&d, DVec2::new(2.1, 0.0), 0.6, &mut f);
        assert_eq!(f.lines, vec![(DVec2::new(2.0, -1.0), DVec2::new(2.0, 1.0))]);
        assert_eq!(f.ends.len(), 2);
        assert_eq!(f.mids, vec![DVec2::new(2.0, 0.0)]);
        // A door aimed at it lines up with it.
        let p = place(&wall(), 0.9, (2.04, 0.0), &f, 0.05, &TypedInput::default()).unwrap();
        assert!((p.offset - 2.0).abs() < 1e-9);
    }
}
