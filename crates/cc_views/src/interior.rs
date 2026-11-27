//! Interior elevations (see [`cc_core::interior`]): where a view of a
//! marker's side looks and how it is cropped, worked out from the space
//! closed by walls around the marker each time the view is drawn.
//!
//! The view looks from the marker along its side's direction, through the
//! space up to the far wall's outer face. Across, it keeps the space's
//! extent (the side walls' inner faces); up, from the level to the
//! underside of what covers the space above the marker (a ceiling, the
//! floor above or a roof), else the room's height, else the level above.

use crate::projected::Frame;
use cc_core::callout::CropBox;
use cc_core::interior::{Face, FaceMode};
use cc_core::{Category, ElementKind, Project, View, ViewKind};
use cc_geom::{BoundaryEdge, ModelGeometry, polygon};
use glam::{DVec2, DVec3};

/// Height of a space with nothing above it, no room and no level above, meters.
const DEFAULT_HEIGHT: f64 = 3.0;
/// Least room between the floor and what covers the space, meters (lower
/// faces belong to furniture or the floor itself).
const MIN_HEADROOM: f64 = 0.5;
/// Thickest wall the view looks into past the space, meters.
const MAX_WALL: f64 = 3.0;

/// How an interior elevation is drawn: its frame and its crop.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub frame: Frame,
    /// The crop of the drawing (the space's, or the one drawn on the view).
    pub crop: Option<CropBox>,
}

/// The frame of an interior elevation without the space: it looks from its
/// origin along its direction (no depth or width limits). `None` for other views.
pub fn frame(view: &View) -> Option<Frame> {
    let ViewKind::InteriorElevation { origin, look, .. } = view.kind else { return None };
    let mut frame = Frame::looking(DVec3::new(origin.x, origin.y, 0.0), DVec3::new(look.x, look.y, 0.0));
    frame.near = Some(0.0);
    Some(frame)
}

/// Why an interior elevation has nothing to draw now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unresolved {
    /// Not an interior elevation, or its marker is gone.
    NoMarker,
    /// No walls close a space around the marker.
    NoSpace,
    /// The view's wall is not one of the walls of the marker's space (or
    /// not on the face the view shows).
    WallGone,
}

impl Unresolved {
    /// The message the view draws instead (an i18n key).
    pub fn key(self) -> &'static str {
        match self {
            Unresolved::NoMarker | Unresolved::NoSpace => "msg-interior-no-space",
            Unresolved::WallGone => "msg-interior-wall-gone",
        }
    }
}

/// [`try_resolve`], without the reason.
pub fn resolve(project: &Project, geom: &ModelGeometry, view: &View) -> Option<Resolved> {
    try_resolve(project, geom, view).ok()
}

/// How `view` (an interior elevation) is drawn now, or why it is not.
///
/// A side's view keeps the whole space across and looks to its far end; a
/// wall's view keeps the stretch of the boundary on the face it shows and
/// looks to the wall's other face.
pub fn try_resolve(project: &Project, geom: &ModelGeometry, view: &View) -> Result<Resolved, Unresolved> {
    let ViewKind::InteriorElevation { marker, face, left, origin, look, auto_extents, .. } = view.kind else {
        return Err(Unresolved::NoMarker);
    };
    let e = project.element(marker).ok_or(Unresolved::NoMarker)?;
    let ElementKind::InteriorMarker(m) = &e.kind else { return Err(Unresolved::NoMarker) };
    let look = look.try_normalize().ok_or(Unresolved::NoMarker)?;
    // The points the view must reach, and how thick the wall behind them may be.
    let (points, wall): (Vec<DVec2>, f64) = match face {
        Face::Side(_) => {
            let boundary = geom.room_at(project, e.level, m.position, Some(view.phase)).ok_or(Unresolved::NoSpace)?;
            (boundary, thickest_wall(project, e.level))
        }
        Face::Wall(id) => {
            let edges =
                geom.room_edges_at(project, e.level, m.position, Some(view.phase)).ok_or(Unresolved::NoSpace)?;
            let points = stretch_points(project, &edges, id, left);
            let thickness =
                project.element(id).and_then(|w| project.types.get(&w.type_id)).map_or(0.0, |t| t.thickness());
            (points, thickness)
        }
    };
    let gone = if matches!(face, Face::Wall(_)) { Unresolved::WallGone } else { Unresolved::NoSpace };
    let (xa, xb, far) = extents(&points, origin, look).ok_or(gone)?;
    let depth = far + if wall.is_finite() { wall.clamp(0.0, MAX_WALL) } else { 0.0 } + 0.01;
    let mut frame = frame(view).ok_or(Unresolved::NoMarker)?;
    frame.far = Some(depth);
    frame.crop_x = Some((xa, xb));
    let crop = if auto_extents {
        let z0 = project.level_elevation(e.level);
        let top = covered_at(geom, m.position, z0)
            .or_else(|| room_top(project, m.room, z0))
            .or_else(|| level_above(project, z0))
            .unwrap_or(z0 + DEFAULT_HEIGHT);
        Some(CropBox::new(DVec2::new(xa, z0), DVec2::new(xb, top)))
    } else {
        view.crop
    };
    Ok(Resolved { frame, crop: crop.filter(CropBox::is_valid) })
}

/// The extents of `points` seen from `origin` along `look`: the least and
/// the largest across (to the right) and the farthest ahead. `None` when
/// there are none, they have no width or none lies ahead of the cut plane.
fn extents(points: &[DVec2], origin: DVec2, look: DVec2) -> Option<(f64, f64, f64)> {
    let right = DVec2::new(look.y, -look.x);
    let across = points.iter().map(|p| (*p - origin).dot(right));
    let (xa, xb) = across.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let far = points.iter().map(|p| (*p - origin).dot(look)).fold(f64::MIN, f64::max);
    let width = xb - xa;
    (!points.is_empty() && width.is_finite() && width > 1e-6 && far.is_finite() && far > 1e-6).then_some((xa, xb, far))
}

/// The ends of the stretches of `edges` on the `left` (or right) face of wall `wall`.
fn stretch_points(project: &Project, edges: &[BoundaryEdge], wall: cc_core::ElementId, left: bool) -> Vec<DVec2> {
    edges
        .iter()
        .filter(|edge| edge.wall == Some(wall))
        .filter(|edge| project.wall_face_is_left(wall, (edge.a + edge.b) * 0.5) == Some(left))
        .flat_map(|edge| [edge.a, edge.b])
        .collect()
}

/// A wall a marker in walls mode can show: the face of it that bounds the
/// space, how much of that face lies along the space and its middle there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallFace {
    pub wall: cc_core::ElementId,
    /// The wall's left face (see [`Project::wall_face_is_left`]), else its right one.
    pub left: bool,
    pub length: f64,
    pub mid: DVec2,
}

/// Most walls a marker lists.
pub const MAX_WALL_FACES: usize = 256;

/// Every wall of the space around `marker` (an element on `level`) in
/// `phase` (unknown: the last one), whatever its length: of a wall bounding
/// the space on both faces (a fin standing into it), the face with more of
/// it along the space. Clockwise from north as seen from the marker (by the
/// middle of each wall's stretch). Empty where no walls close a space.
pub fn space_walls(
    project: &Project,
    geom: &ModelGeometry,
    level: cc_core::LevelId,
    marker: &cc_core::interior::InteriorMarker,
    phase: Option<cc_core::PhaseId>,
) -> Vec<WallFace> {
    let Some(edges) = geom.room_edges_at(project, level, marker.position, phase) else { return Vec::new() };
    // Per wall and face: its stretches, in the order of the boundary.
    let mut faces: std::collections::BTreeMap<(cc_core::ElementId, bool), Vec<(DVec2, DVec2)>> = Default::default();
    for edge in &edges {
        let Some(id) = edge.wall else { continue };
        let Some(left) = project.wall_face_is_left(id, (edge.a + edge.b) * 0.5) else { continue };
        faces.entry((id, left)).or_default().push((edge.a, edge.b));
    }
    let mut best: std::collections::BTreeMap<cc_core::ElementId, WallFace> = Default::default();
    for ((wall, left), stretches) in faces {
        let length: f64 = stretches.iter().map(|(a, b)| a.distance(*b)).sum();
        let face = WallFace { wall, left, length, mid: middle(&stretches, length) };
        if best.get(&wall).is_none_or(|other| length > other.length) {
            best.insert(wall, face);
        }
    }
    let bearing = |p: DVec2| {
        let d = p - marker.position;
        d.x.atan2(d.y).rem_euclid(std::f64::consts::TAU)
    };
    let mut walls: Vec<WallFace> = best.into_values().collect();
    walls.sort_by(|a, b| bearing(a.mid).total_cmp(&bearing(b.mid)));
    walls.truncate(MAX_WALL_FACES);
    walls
}

/// The point half way along `stretches` (`length` in all).
fn middle(stretches: &[(DVec2, DVec2)], length: f64) -> DVec2 {
    let mut left = length * 0.5;
    for (a, b) in stretches {
        let d = a.distance(*b);
        if d >= left && d > 0.0 {
            return a.lerp(*b, left / d);
        }
        left -= d;
    }
    stretches.last().map_or(DVec2::ZERO, |(_, b)| *b)
}

/// The walls [`space_walls`] gives that the marker's walls mode can show:
/// those with at least the mode's `min_length` along the space (the default
/// one in directions mode).
pub fn wall_faces(
    project: &Project,
    geom: &ModelGeometry,
    level: cc_core::LevelId,
    marker: &cc_core::interior::InteriorMarker,
    phase: Option<cc_core::PhaseId>,
) -> Vec<WallFace> {
    let min_length = min_length(marker);
    space_walls(project, geom, level, marker, phase).into_iter().filter(|f| f.length + 1e-9 >= min_length).collect()
}

/// The least length along the space a wall needs for `marker` to show it.
pub fn min_length(marker: &cc_core::interior::InteriorMarker) -> f64 {
    match marker.mode {
        FaceMode::Walls { min_length, .. } => min_length,
        FaceMode::Directions => cc_core::interior::DEFAULT_MIN_LENGTH,
    }
}

/// The thickest type of the walls on `level`, meters.
fn thickest_wall(project: &Project, level: cc_core::LevelId) -> f64 {
    project
        .walls()
        .filter(|(e, _)| e.level == level)
        .filter_map(|(e, _)| project.types.get(&e.type_id).map(|t| t.thickness()))
        .filter(|t| t.is_finite())
        .fold(0.0, f64::max)
}

/// The lowest underside of a ceiling, floor or roof over `at`, at least
/// [`MIN_HEADROOM`] above `z0`.
fn covered_at(geom: &ModelGeometry, at: DVec2, z0: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for s in &geom.solids {
        if !matches!(s.category, Category::Ceiling | Category::Floor | Category::Roof) || s.secondary {
            continue;
        }
        for f in &s.faces {
            // Faces looking down, over the point.
            if f.normal.z > -0.1 || f.verts.len() < 3 {
                continue;
            }
            let plan: Vec<DVec2> = f.verts.iter().map(|v| v.truncate()).collect();
            if !polygon::contains(&plan, at) {
                continue;
            }
            let p0 = f.verts[0];
            let z = p0.z - (f.normal.x * (at.x - p0.x) + f.normal.y * (at.y - p0.y)) / f.normal.z;
            if z.is_finite() && z >= z0 + MIN_HEADROOM && best.is_none_or(|b| z < b) {
                best = Some(z);
            }
        }
    }
    best
}

/// The top of room `room`, its height above `z0`.
fn room_top(project: &Project, room: Option<cc_core::ElementId>, z0: f64) -> Option<f64> {
    match project.element(room?).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) if r.height.is_finite() && r.height > MIN_HEADROOM => Some(z0 + r.height),
        _ => None,
    }
}

/// The elevation of the lowest level above `z0` by more than the headroom.
fn level_above(project: &Project, z0: f64) -> Option<f64> {
    project.levels.iter().map(|l| l.elevation).filter(|z| *z >= z0 + MIN_HEADROOM).reduce(f64::min)
}

/// Radius of a marker's circle on paper, millimeters.
const MARKER_RADIUS_MM: f64 = 3.0;
/// Height of the numbers at its arrows on paper, millimeters.
const NUMBER_SIZE_MM: f64 = 2.2;

/// The interior elevation markers on `level` in a plan at 1:`scale`: a
/// circle (the marker, to pick and move) and, for each of its views, a
/// filled arrow pointing at the wall it shows with the view's number (its
/// drawing number once on a sheet). A side's arrow stands on the circle; a
/// wall's stands where its view is cut, in front of the middle of the
/// wall's stretch of the space (in the view's phase).
pub(crate) fn marks(
    d: &mut crate::Drawing2D,
    project: &Project,
    geom: &ModelGeometry,
    level: cc_core::LevelId,
    scale: u32,
) {
    let r = MARKER_RADIUS_MM * f64::from(scale.max(1)) / 1000.0;
    for e in project.elements.values().filter(|e| e.level == level) {
        let ElementKind::InteriorMarker(m) = &e.kind else { continue };
        let c = m.position;
        let circle: Vec<DVec2> =
            (0..24).map(|i| c + DVec2::from_angle(f64::from(i) / 24.0 * std::f64::consts::TAU) * r).collect();
        d.fill(circle.clone(), [255, 255, 255, 255], Some(e.id));
        d.polyline(&circle, true, crate::LineStyle::Annotation, Some(e.id));
        // The space's edges, found once per phase its walls' views are in.
        let mut spaces: std::collections::BTreeMap<cc_core::PhaseId, Option<Vec<BoundaryEdge>>> = Default::default();
        for (face, number, view) in project.marker_faces(e.id) {
            let Some(v) = project.view(view) else { continue };
            let ViewKind::InteriorElevation { origin, look, left, .. } = v.kind else { continue };
            let Some(look) = look.try_normalize() else { continue };
            let across = look.perp();
            // Where the arrow's base stands, and where its number goes.
            let (base, text) = match face {
                Face::Side(_) => (c + look * (r * 0.3), c + look * (r * 0.55)),
                Face::Wall(id) => {
                    let edges = spaces
                        .entry(v.phase)
                        .or_insert_with(|| geom.room_edges_at(project, level, m.position, Some(v.phase)));
                    // In front of the middle of what the view shows, on the cut plane.
                    let shown = edges.as_deref().map(|edges| stretch_points(project, edges, id, left));
                    let along = shown
                        .and_then(|points| extents(&points, origin, look))
                        .map_or(0.0, |(xa, xb, _)| (xa + xb) * 0.5);
                    let on_plane = origin - across * along;
                    (on_plane, on_plane - look * (r * 0.7))
                }
            };
            let arrow = vec![base + across * (r * 0.95), base + look * (r * 1.6), base - across * (r * 0.95)];
            d.fill(arrow, [0, 0, 0, 255], Some(e.id));
            // Picked on the arrow's body, clear of the marker's circle.
            d.interior_arrows.push(crate::InteriorArrow { view, center: base + look * (r * 0.8), radius: r * 0.9 });
            let (drawing, sheet) = crate::plan::section_labels(project, v);
            let label = if sheet == "-" { number.to_string() } else { drawing };
            d.text_at(text, label, NUMBER_SIZE_MM, crate::Anchor::Center, 0.0);
        }
    }
}
