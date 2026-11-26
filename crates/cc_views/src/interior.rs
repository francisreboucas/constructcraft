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
use cc_core::{Category, ElementKind, Project, View, ViewKind};
use cc_geom::{ModelGeometry, polygon};
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

/// How `view` (an interior elevation) is drawn now: `None` when it is not
/// one, its marker is gone or no walls close a space around the marker.
pub fn resolve(project: &Project, geom: &ModelGeometry, view: &View) -> Option<Resolved> {
    let ViewKind::InteriorElevation { marker, origin, look, auto_extents, .. } = view.kind else { return None };
    let e = project.element(marker)?;
    let ElementKind::InteriorMarker(m) = &e.kind else { return None };
    let look = look.try_normalize()?;
    let boundary = geom.room_at(project, e.level, origin, Some(view.phase))?;
    let right = DVec2::new(look.y, -look.x);
    let across = |p: &DVec2| (*p - origin).dot(right);
    let ahead = |p: &DVec2| (*p - origin).dot(look);
    let (xa, xb) = boundary.iter().map(across).fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let far = boundary.iter().map(ahead).fold(0.0_f64, f64::max);
    let width = xb - xa;
    if !width.is_finite() || width <= 1e-6 || !far.is_finite() {
        return None;
    }
    let depth = far + thickest_wall(project, e.level).min(MAX_WALL) + 0.01;
    let mut frame = frame(view)?;
    frame.far = Some(depth);
    frame.crop_x = Some((xa, xb));
    let crop = if auto_extents {
        let z0 = project.level_elevation(e.level);
        let top = covered_at(geom, origin, z0)
            .or_else(|| room_top(project, m.room, z0))
            .or_else(|| level_above(project, z0))
            .unwrap_or(z0 + DEFAULT_HEIGHT);
        Some(CropBox::new(DVec2::new(xa, z0), DVec2::new(xb, top)))
    } else {
        view.crop
    };
    Some(Resolved { frame, crop: crop.filter(CropBox::is_valid) })
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
/// drawing number once on a sheet).
pub(crate) fn marks(d: &mut crate::Drawing2D, project: &Project, level: cc_core::LevelId, scale: u32) {
    let r = MARKER_RADIUS_MM * f64::from(scale.max(1)) / 1000.0;
    for e in project.elements.values().filter(|e| e.level == level) {
        let ElementKind::InteriorMarker(m) = &e.kind else { continue };
        let c = m.position;
        let circle: Vec<DVec2> =
            (0..24).map(|i| c + DVec2::from_angle(f64::from(i) / 24.0 * std::f64::consts::TAU) * r).collect();
        d.fill(circle.clone(), [255, 255, 255, 255], Some(e.id));
        d.polyline(&circle, true, crate::LineStyle::Annotation, Some(e.id));
        for (side, number, view) in project.marker_sides(e.id) {
            let look = m.look(side);
            let across = look.perp();
            let tip = c + look * (r * 1.9);
            let arrow =
                vec![c + across * (r * 0.95) + look * (r * 0.3), tip, c - across * (r * 0.95) + look * (r * 0.3)];
            d.fill(arrow, [0, 0, 0, 255], Some(e.id));
            d.interior_arrows.push(crate::InteriorArrow { view, center: c + look * (r * 1.1), radius: r * 0.9 });
            let label = project
                .view(view)
                .map(|v| crate::plan::section_labels(project, v))
                .filter(|(_, sheet)| sheet != "-")
                .map_or_else(|| number.to_string(), |(n, _)| n);
            d.text_at(c + look * (r * 0.55), label, NUMBER_SIZE_MM, crate::Anchor::Center, 0.0);
        }
    }
}
