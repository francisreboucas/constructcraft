//! Associative dimensions: each end of a dimension may reference a point
//! of an element (stored in that element's own frame, plus a height in
//! elevations and sections) or a level. Every edit ends by recomputing
//! referenced ends ([`crate::EditCtx`]), inside the same transaction, so
//! dimensions follow moved, rotated, stretched or raised elements and
//! levels, and undo restores both together.

use crate::{Element, ElementId, ElementKind, LevelId, Project, View, ViewKind};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// What a dimension end follows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DimRef {
    pub target: RefTarget,
    pub anchor: RefAnchor,
    /// Height in elevations and sections (`None` in plans).
    #[serde(default)]
    pub z: Option<RefHeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefTarget {
    Element(ElementId),
    /// A level line of an elevation or section (only its height counts).
    Level(LevelId),
}

/// Where the point sits in the element's frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RefAnchor {
    /// On a wall, beam or grid: `along` meters from its start (or from its
    /// end when `from_end`, measured back toward the start), `across`
    /// meters to the left of its line.
    Line { from_end: bool, along: f64, across: f64 },
    /// In the frame of a column or component (its position and rotation).
    Local { x: f64, y: f64 },
    /// On a door or window: `along` its wall from the opening center,
    /// `across` to the left of the wall's line.
    Hosted { along: f64, across: f64 },
    /// A level: the end keeps its position along the view.
    Level,
}

/// Height of a referenced point: `dz` above the element's base, or above
/// its top when `from_top` (walls), so it follows height changes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RefHeight {
    pub from_top: bool,
    pub dz: f64,
}

impl DimRef {
    /// The referenced element, if the target is one.
    pub fn element(&self) -> Option<ElementId> {
        match self.target {
            RefTarget::Element(id) => Some(id),
            RefTarget::Level(_) => None,
        }
    }

    pub(crate) fn is_finite(&self) -> bool {
        let anchor = match self.anchor {
            RefAnchor::Line { along, across, .. } | RefAnchor::Hosted { along, across } => {
                along.is_finite() && across.is_finite()
            }
            RefAnchor::Local { x, y } => x.is_finite() && y.is_finite(),
            RefAnchor::Level => true,
        };
        anchor && self.z.is_none_or(|h| h.dz.is_finite())
    }

    /// The reference to the mirror image of this point once `target` (the
    /// referenced element, already mirrored) went through a mirror: a wall
    /// is walked the other way, beams, grids, columns and components keep
    /// their direction, and an opening's frame follows its wall.
    pub(crate) fn mirrored(self, target: &ElementKind) -> DimRef {
        let anchor = match (self.anchor, target) {
            (RefAnchor::Line { from_end, along, across }, ElementKind::Wall(_)) => {
                RefAnchor::Line { from_end: !from_end, along, across }
            }
            (RefAnchor::Line { from_end, along, across }, _) => RefAnchor::Line { from_end, along, across: -across },
            (RefAnchor::Local { x, y }, _) => RefAnchor::Local { x, y: -y },
            (RefAnchor::Hosted { along, across }, _) => RefAnchor::Hosted { along: -along, across },
            (RefAnchor::Level, _) => RefAnchor::Level,
        };
        DimRef { anchor, ..self }
    }

    /// Whether the target still exists in `project`.
    pub(crate) fn target_exists(&self, project: &Project) -> bool {
        match self.target {
            RefTarget::Element(id) => project.element(id).is_some(),
            RefTarget::Level(id) => project.level(id).is_some(),
        }
    }
}

/// Unit direction and its left normal of the segment `a`-`b`.
fn frame(a: DVec2, b: DVec2) -> Option<(DVec2, DVec2)> {
    let u = (b - a).try_normalize()?;
    Some((u, u.perp()))
}

/// How elevations and sections map the model: x along `right` from
/// `origin` (plan), y the height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    pub origin: DVec2,
    pub right: DVec2,
}

impl Projection {
    pub fn project(&self, p: DVec2, z: f64) -> DVec2 {
        DVec2::new((p - self.origin).dot(self.right), z)
    }
}

impl View {
    /// The projection of an elevation, interior elevation or section view.
    pub fn projection(&self) -> Option<Projection> {
        let (origin, look) = match &self.kind {
            ViewKind::Elevation { direction } => (DVec2::ZERO, direction.look()),
            ViewKind::Section { start, end, .. } => (*start, -(*end - *start).try_normalize()?.perp()),
            ViewKind::InteriorElevation { origin, look, .. } => (*origin, look.try_normalize()?),
            _ => return None,
        };
        // The view's right: the look direction crossed with up.
        Some(Projection { origin, right: DVec2::new(look.y, -look.x) })
    }

    pub fn is_plan(&self) -> bool {
        matches!(self.kind, ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. })
    }
}

impl Project {
    /// The reference of point `p` (plan coordinates) on element `id`, when
    /// the element kind can carry one.
    pub fn dim_ref_at(&self, id: ElementId, p: DVec2) -> Option<DimRef> {
        let e = self.element(id)?;
        let anchor = match &e.kind {
            // Arc walls measure along the arc and across it.
            ElementKind::Wall(w) if w.is_curved() => {
                let (along, across) = w.project(p);
                let length = w.length();
                let from_end = p.distance(w.end) < p.distance(w.start);
                RefAnchor::Line { from_end, along: if from_end { length - along } else { along }, across }
            }
            ElementKind::Wall(_) | ElementKind::Beam(_) | ElementKind::Grid(_) => {
                let (a, b) = crate::wall_ops::line_of(e)?;
                let (u, n) = frame(a, b)?;
                let from_end = p.distance(b) < p.distance(a);
                let along = if from_end { (p - b).dot(-u) } else { (p - a).dot(u) };
                RefAnchor::Line { from_end, along, across: (p - a).dot(n) }
            }
            ElementKind::Column(c) => local(c.position, c.rotation_deg, p),
            ElementKind::Component(c) => local(c.position, c.rotation_deg, p),
            ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h) => {
                let (center, u) = self.opening_frame(h)?;
                RefAnchor::Hosted { along: (p - center).dot(u), across: (p - center).dot(u.perp()) }
            }
            _ => return None,
        };
        let r = DimRef { target: RefTarget::Element(id), anchor, z: None };
        r.is_finite().then_some(r)
    }

    /// The reference of point `p` (in `view`'s drawing coordinates) on
    /// element `id`: in plans as [`Project::dim_ref_at`]; in elevations and
    /// sections a plan point of the element that projects to `p.x`, and a
    /// height from the element's base or top (whichever is nearer).
    pub fn dim_ref_in_view(&self, view: &View, id: ElementId, p: DVec2) -> Option<DimRef> {
        let Some(projection) = view.projection() else {
            return if view.is_plan() { self.dim_ref_at(id, p) } else { None };
        };
        let e = self.element(id)?;
        let c = self.base_point(e)?;
        let plan = c + projection.right * (p.x - projection.project(c, 0.0).x);
        let base = self.base_z(e);
        let z = match self.top_z(e) {
            Some(top) if (p.y - top).abs() < (p.y - base).abs() => RefHeight { from_top: true, dz: p.y - top },
            _ => RefHeight { from_top: false, dz: p.y - base },
        };
        let r = DimRef { z: Some(z), ..self.dim_ref_at(id, plan)? };
        r.is_finite().then_some(r)
    }

    /// A level as a reference in an elevation or section.
    pub fn level_ref(&self, view: &View, level: LevelId) -> Option<DimRef> {
        view.projection()?;
        self.level(level)?;
        Some(DimRef { target: RefTarget::Level(level), anchor: RefAnchor::Level, z: None })
    }

    /// The current plan point of an element reference.
    pub fn resolve_dim_ref(&self, r: &DimRef) -> Option<DVec2> {
        let e = self.element(r.element()?)?;
        let p = match (r.anchor, &e.kind) {
            (RefAnchor::Line { from_end, along, across }, ElementKind::Wall(w)) if w.is_curved() => {
                w.at(if from_end { w.length() - along } else { along }, across)
            }
            (RefAnchor::Line { from_end, along, across }, _) => {
                let (a, b) = crate::wall_ops::line_of(e)?;
                let (u, n) = frame(a, b)?;
                let base = if from_end { b - u * along } else { a + u * along };
                base + n * across
            }
            (RefAnchor::Local { x, y }, ElementKind::Column(c)) => from_local(c.position, c.rotation_deg, x, y),
            (RefAnchor::Local { x, y }, ElementKind::Component(c)) => from_local(c.position, c.rotation_deg, x, y),
            (
                RefAnchor::Hosted { along, across },
                ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h),
            ) => {
                let (center, u) = self.opening_frame(h)?;
                center + u * along + u.perp() * across
            }
            _ => return None,
        };
        p.is_finite().then_some(p)
    }

    /// The current point of a reference in `view`'s drawing coordinates;
    /// `current` is the end's present position (levels keep its x).
    pub fn resolve_in_view(&self, r: &DimRef, view: &View, current: DVec2) -> Option<DVec2> {
        let projection = view.projection();
        let p = match (r.target, projection, r.z) {
            (RefTarget::Level(id), Some(_), _) => DVec2::new(current.x, self.level(id)?.elevation),
            (RefTarget::Element(id), Some(projection), Some(h)) => {
                let e = self.element(id)?;
                let z = if h.from_top { self.top_z(e)? } else { self.base_z(e) } + h.dz;
                projection.project(self.resolve_dim_ref(r)?, z)
            }
            (RefTarget::Element(_), None, None) if view.is_plan() => self.resolve_dim_ref(r)?,
            _ => return None,
        };
        p.is_finite().then_some(p)
    }

    /// A plan point that moves with the element (its line's middle, its
    /// position or its opening's center).
    fn base_point(&self, e: &Element) -> Option<DVec2> {
        match &e.kind {
            ElementKind::Wall(w) if w.is_curved() => Some(w.point_at(w.length() / 2.0)),
            ElementKind::Wall(_) | ElementKind::Beam(_) | ElementKind::Grid(_) => {
                crate::wall_ops::line_of(e).map(|(a, b)| (a + b) / 2.0)
            }
            ElementKind::Column(c) => Some(c.position),
            ElementKind::Component(c) => Some(c.position),
            ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h) => {
                self.opening_frame(h).map(|(center, _)| center)
            }
            _ => None,
        }
    }

    /// Center of an opening on its wall's location line and the wall's
    /// direction there (straight walls keep their location-line frame).
    fn opening_frame(&self, h: &crate::Hosted) -> Option<(DVec2, DVec2)> {
        let host = self.element(h.host)?;
        let w = host.as_wall()?;
        if w.arc().is_some() {
            // Openings are placed along the body centerline; map back onto the location line.
            let (start, end) = self.wall_axis(host, w);
            let axis = crate::Wall { start, end, ..w.clone() };
            let (along, _) = w.project(axis.point_at(h.offset));
            return Some((w.point_at(along), w.tangent_at(along)));
        }
        let (u, _) = frame(w.start, w.end)?;
        Some((w.start + u * h.offset, u))
    }

    /// Height of an element's base, the reference for heights in views.
    fn base_z(&self, e: &Element) -> f64 {
        let level = self.level_elevation(e.level);
        match &e.kind {
            ElementKind::Wall(w) => self.wall_base_z(e, w),
            ElementKind::Column(c) => level + c.base_offset,
            ElementKind::Component(c) => level + c.base_offset,
            ElementKind::Beam(b) => self.beam_z(e, b).1,
            ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h) => {
                let host = self.element(h.host);
                let wall_base = host.and_then(|x| x.as_wall().map(|w| self.wall_base_z(x, w))).unwrap_or(level);
                wall_base + h.sill_height
            }
            _ => level,
        }
    }

    /// Height of an element's top, where it has one that can change.
    fn top_z(&self, e: &Element) -> Option<f64> {
        match &e.kind {
            ElementKind::Wall(w) => Some(self.wall_top_z(e, w)),
            _ => None,
        }
    }
}

fn local(position: DVec2, rotation_deg: f64, p: DVec2) -> RefAnchor {
    let d = DVec2::from_angle(-rotation_deg.to_radians()).rotate(p - position);
    RefAnchor::Local { x: d.x, y: d.y }
}

fn from_local(position: DVec2, rotation_deg: f64, x: f64, y: f64) -> DVec2 {
    position + DVec2::from_angle(rotation_deg.to_radians()).rotate(DVec2::new(x, y))
}

/// How many rounds of moves locked dimensions may cause in one edit (a
/// chain of locked dimensions moves one element per round).
const MAX_LOCK_ROUNDS: usize = 64;

impl crate::EditCtx<'_> {
    /// Keep locked dimensions at their values (#447): where this edit moved
    /// the element at one end of a locked dimension in a plan and not the
    /// element at the other, that one is moved along the dimension by what
    /// it lacks, in the same transaction; it may move others in turn. A lock
    /// whose two ends moved, or whose other element is pinned, is left
    /// broken, and the engine refuses the edit (see [`crate::Dimension::breaks_lock`]).
    pub(crate) fn hold_locked_dimensions(&mut self) {
        for _ in 0..MAX_LOCK_ROUNDS {
            let project = self.project();
            let moved = |id: ElementId| self.has_changed(id);
            let mut moves: Vec<(ElementId, DVec2)> = Vec::new();
            for e in project.elements.values() {
                let ElementKind::Dimension(d) = &e.kind else { continue };
                let (Some(value), true) = (d.locked, d.can_lock()) else { continue };
                let Some(view) = project.view(d.view).filter(|v| v.is_plan()) else { continue };
                let [Some(ra), Some(rb)] = d.refs else { continue };
                let (Some(ea), Some(eb)) = (ra.element(), rb.element()) else { continue };
                let (Some(pa), Some(pb)) =
                    (project.resolve_in_view(&ra, view, d.a), project.resolve_in_view(&rb, view, d.b))
                else {
                    continue;
                };
                let Some(dir) = d.direction() else { continue };
                // The dimension keeps its sense: the value is measured from `a` towards `b`.
                let sign = if (d.b - d.a).dot(dir) < 0.0 { -1.0 } else { 1.0 };
                let error = (pb - pa).dot(dir) - sign * value;
                if error.abs() <= crate::element::LOCK_TOLERANCE * 0.1 || !error.is_finite() {
                    continue;
                }
                let (target, shift) = match (moved(ea), moved(eb)) {
                    (true, false) => (eb, -dir * error),
                    (false, true) => (ea, dir * error),
                    _ => continue,
                };
                if !project.is_pinned(target) && !moves.iter().any(|(id, _)| *id == target) {
                    moves.push((target, shift));
                }
            }
            if moves.is_empty() {
                return;
            }
            for (id, shift) in moves {
                self.transform_elements(&[id], &crate::PlanTransform::translate(shift));
            }
        }
    }

    /// Move the referenced ends of every dimension to their targets'
    /// current points (and an angle's vertex to the intersection of its
    /// lines); references to deleted targets are dropped (the end stays
    /// where it was). Only changed dimensions are touched.
    pub(crate) fn sync_dimensions(&mut self) {
        let project = self.project();
        let updates: Vec<(ElementId, Box<crate::Dimension>)> = project
            .elements
            .values()
            .filter_map(|e| {
                let ElementKind::Dimension(d) = &e.kind else { return None };
                if !d.is_associated() {
                    return None;
                }
                let view = project.view(d.view)?;
                let mut n = d.clone();
                for (r, end) in n.refs.iter_mut().zip([&mut n.a, &mut n.b]) {
                    let Some(reference) = r else { continue };
                    match project.resolve_in_view(reference, view, *end) {
                        Some(p) => *end = p,
                        None if !reference.target_exists(project) => *r = None,
                        None => {}
                    }
                }
                let (a, b) = (n.a, n.b);
                if let crate::DimensionKind::Angular { vertex, rays } = &mut n.kind {
                    let mut lines = [None, None];
                    for ((r, line), end) in rays.iter_mut().zip(lines.iter_mut()).zip([a, b]) {
                        let Some(reference) = r else { continue };
                        match project.resolve_in_view(reference, view, *vertex) {
                            Some(p) => *line = Some((end, p)),
                            None if !reference.target_exists(project) => *r = None,
                            None => {}
                        }
                    }
                    if let [Some(l1), Some(l2)] = lines
                        && let Some(v) = intersect(l1, l2)
                    {
                        *vertex = v;
                    }
                }
                // A linear dimension keeps its line where it was placed.
                if let Some(dir) = n.direction() {
                    n.offset += (d.a - n.a).dot(dir.perp());
                }
                (n != *d).then_some((e.id, n))
            })
            .collect();
        for (id, n) in updates {
            if let Some(ElementKind::Dimension(d)) = self.element_mut(id).map(|e| &mut e.kind) {
                *d = n;
            }
        }
    }
}

/// Intersection of the lines through `(p1, q1)` and `(p2, q2)` (`None`
/// when either is degenerate or they are parallel).
fn intersect((p1, q1): (DVec2, DVec2), (p2, q2): (DVec2, DVec2)) -> Option<DVec2> {
    let (d1, d2) = (q1 - p1, q2 - p2);
    let den = d1.perp_dot(d2);
    if den.abs() <= 1e-9 * d1.length() * d2.length() || d1.length() < 1e-9 || d2.length() < 1e-9 {
        return None;
    }
    let v = p1 + d1 * ((p2 - p1).perp_dot(d2) / den);
    v.is_finite().then_some(v)
}

#[cfg(test)]
mod tests {
    use crate::{Dimension, DimensionKind, Document, ElementKind, PlanTransform};
    use glam::DVec2;

    fn sample() -> Document {
        Document::new(crate::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap())
    }

    /// A dimension from the inside face of one wall to a free point.
    fn dimension_to_wall(doc: &mut Document) -> (crate::ElementId, crate::ElementId) {
        let (wall, w) =
            doc.project.walls().map(|(e, w)| (e.id, w.clone())).find(|(_, w)| w.start.x == w.end.x).unwrap();
        let p = (w.start + w.end) / 2.0 + DVec2::new(0.075, 0.0);
        let r = doc.project.dim_ref_at(wall, p).unwrap();
        let view = doc.project.views.values().find(|v| v.is_plan()).map(|v| v.id).unwrap();
        let level = doc.project.levels[0].id;
        let ty = doc.project.default_type(crate::Category::Dimension).unwrap();
        let dim = Dimension {
            view,
            a: p,
            b: p + DVec2::new(3.0, 0.0),
            offset: 1.0,
            kind: DimensionKind::Linear { dir: DVec2::X },
            refs: [Some(r), None],
            locked: None,
        };
        let id = doc.edit("dim", |e| {
            let el = crate::Element {
                id: crate::ElementId(0),
                level,
                type_id: ty,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Dimension(Box::new(dim)),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            };
            e.add_element(el)
        });
        (wall, id)
    }

    fn dim(doc: &Document, id: crate::ElementId) -> Dimension {
        match &doc.project.element(id).unwrap().kind {
            ElementKind::Dimension(d) => (**d).clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dimensions_follow_moved_walls_and_undo_with_them() {
        let mut doc = sample();
        let (wall, id) = dimension_to_wall(&mut doc);
        let before = dim(&doc, id);
        assert!((before.value() - 3.0).abs() < 1e-9);
        doc.edit("move", |e| e.transform_elements(&[wall], &PlanTransform::translate(DVec2::new(0.5, 0.0))));
        let after = dim(&doc, id);
        assert!((after.a - before.a - DVec2::new(0.5, 0.0)).length() < 1e-9);
        assert!((after.value() - 2.5).abs() < 1e-9);
        // The dimension line stays where it was placed.
        let line = |d: &Dimension| d.a.y + d.offset;
        assert!((line(&after) - line(&before)).abs() < 1e-9);
        doc.undo();
        assert_eq!(dim(&doc, id), before);
    }

    #[test]
    fn deleting_the_element_frees_the_dimension_end() {
        let mut doc = sample();
        let (wall, id) = dimension_to_wall(&mut doc);
        let before = dim(&doc, id);
        doc.edit("delete", |e| e.remove_element(wall));
        let after = dim(&doc, id);
        assert_eq!(after.refs, [None, None]);
        assert_eq!((after.a, after.b), (before.a, before.b));
    }

    #[test]
    fn references_follow_rotated_columns_and_stretched_walls() {
        let doc = sample();
        let p = &doc.project;
        let (wall, w) = p.walls().map(|(e, w)| (e.id, w.clone())).next().unwrap();
        // A point near the end stays at the same distance from that end when the wall stretches.
        let near_end = w.end + (w.start - w.end).normalize() * 0.2;
        let r = p.dim_ref_at(wall, near_end).unwrap();
        let mut stretched = p.clone();
        if let Some(ElementKind::Wall(x)) = stretched.elements.get_mut(&wall).map(|e| &mut e.kind) {
            x.start -= (x.end - x.start).normalize();
        }
        assert!(stretched.resolve_dim_ref(&r).unwrap().distance(near_end) < 1e-9);
    }

    #[test]
    fn angular_dimensions_follow_a_rotated_wall() {
        let mut doc = sample();
        // Two walls meeting at a corner: one along X, one along Y.
        let walls: Vec<_> = doc.project.walls().map(|(e, w)| (e.id, w.start, w.end)).collect();
        let (wx, sx, ex) = *walls.iter().find(|(_, s, e)| (s.y - e.y).abs() < 1e-9).unwrap();
        let (wy, sy, ey) = *walls
            .iter()
            .find(|(_, s, e)| {
                (s.x - e.x).abs() < 1e-9
                    && (s.distance(sx) < 1e-9
                        || e.distance(sx) < 1e-9
                        || s.distance(ex) < 1e-9
                        || e.distance(ex) < 1e-9)
            })
            .unwrap();
        let corner = [sx, ex].into_iter().find(|p| p.distance(sy) < 1e-9 || p.distance(ey) < 1e-9).unwrap();
        let along_x = if sx.distance(corner) < 1e-9 { ex } else { sx };
        let along_y = if sy.distance(corner) < 1e-9 { ey } else { sy };
        let a = corner + (along_x - corner).normalize();
        let b = corner + (along_y - corner).normalize();
        let p = &doc.project;
        let refs = [p.dim_ref_at(wx, a), p.dim_ref_at(wy, b)];
        let rays = [p.dim_ref_at(wx, corner), p.dim_ref_at(wy, corner)];
        let view = p.views.values().find(|v| v.is_plan()).map(|v| v.id).unwrap();
        let level = p.levels[0].id;
        let ty = p.default_type(crate::Category::Dimension).unwrap();
        let dim = Dimension {
            view,
            a,
            b,
            offset: 0.5,
            kind: DimensionKind::Angular { vertex: corner, rays },
            refs,
            locked: None,
        };
        let id = doc.edit("dim", |e| {
            e.add_element(crate::Element {
                id: crate::ElementId(0),
                level,
                type_id: ty,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Dimension(Box::new(dim)),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        assert!((self::dim(&doc, id).value().to_degrees() - 90.0).abs() < 1e-9);
        // Turn the Y wall by 30 degrees about its far end: the angle opens to 60 or 120.
        let far = along_y;
        doc.edit("rotate", |e| e.transform_elements(&[wy], &PlanTransform::rotate_about(far, 30f64.to_radians())));
        let d = self::dim(&doc, id);
        let deg = d.value().to_degrees();
        assert!((deg - 60.0).abs() < 1e-6 || (deg - 120.0).abs() < 1e-6, "{deg}");
        // The vertex is where the X wall's line meets the turned wall's line.
        let DimensionKind::Angular { vertex, .. } = d.kind else { panic!() };
        assert!((vertex.y - corner.y).abs() < 1e-9, "{vertex}");
        doc.undo();
        assert!((self::dim(&doc, id).value().to_degrees() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn elevation_dimensions_follow_wall_heights_and_levels() {
        let mut doc = sample();
        let elevation = doc.project.views.values().find(|v| v.projection().is_some()).cloned().unwrap();
        let projection = elevation.projection().unwrap();
        // A wall seen in this elevation (its line across the view's depth).
        let (wall, top, mid) = doc
            .project
            .walls()
            .filter(|(_, w)| (w.end - w.start).normalize().dot(projection.right).abs() > 0.99)
            .map(|(e, w)| (e.id, doc.project.wall_top_z(e, w), (w.start + w.end) / 2.0))
            .next()
            .unwrap();
        let level = doc.project.levels[0].id;
        let x = projection.project(mid, 0.0).x;
        let p = &doc.project;
        // From the level line to the wall's top.
        let refs = [p.level_ref(&elevation, level), p.dim_ref_in_view(&elevation, wall, DVec2::new(x, top))];
        assert!(refs[1].is_some_and(|r| r.z.is_some_and(|h| h.from_top)));
        let base = p.level_elevation(level);
        let ty = p.default_type(crate::Category::Dimension).unwrap();
        let dim = Dimension {
            view: elevation.id,
            a: DVec2::new(x, base),
            b: DVec2::new(x, top),
            offset: 1.0,
            kind: DimensionKind::Linear { dir: DVec2::Y },
            refs,
            locked: None,
        };
        let id = doc.edit("dim", |e| {
            e.add_element(crate::Element {
                id: crate::ElementId(0),
                level,
                type_id: ty,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Dimension(Box::new(dim)),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        let height = |doc: &Document| self::dim(doc, id).value();
        let before = height(&doc);
        assert!((before - (top - base)).abs() < 1e-9);
        // A taller wall: the dimension follows its top.
        doc.edit("taller", |e| {
            if let Some(ElementKind::Wall(w)) = e.element_mut(wall).map(|x| &mut x.kind) {
                w.top = crate::WallTop::Unconnected;
                w.height += 0.5;
            }
        });
        let raised = doc.project.element(wall).and_then(|e| e.as_wall().map(|w| doc.project.wall_top_z(e, w))).unwrap();
        assert!((height(&doc) - (raised - base)).abs() < 1e-9);
        // Moving the wall along the view keeps the dimension on it.
        doc.edit("move", |e| e.transform_elements(&[wall], &PlanTransform::translate(projection.right * 0.4)));
        assert!((self::dim(&doc, id).b.x - (x + 0.4)).abs() < 1e-9);
        // Lowering the level moves the other end.
        doc.edit("level", |e| {
            if let Some(l) = e.levels_mut().iter_mut().find(|l| l.id == level) {
                l.elevation -= 0.25;
            }
        });
        assert!((self::dim(&doc, id).a.y - (base - 0.25)).abs() < 1e-9);
    }

    /// One dimension per element that can carry a reference, from a point
    /// off its axis to a free point. Returns (dimension, measured point).
    fn dimensions_to_everything(doc: &mut Document) -> Vec<(crate::ElementId, DVec2)> {
        let view = doc.project.views.values().find(|v| v.is_plan()).map(|v| v.id).unwrap();
        let ty = doc.project.default_type(crate::Category::Dimension).unwrap();
        let targets: Vec<(crate::ElementId, crate::LevelId)> =
            doc.project.elements.values().map(|e| (e.id, e.level)).collect();
        let mut out = Vec::new();
        for (target, level) in targets {
            let Some(base) = doc.project.element(target).and_then(|e| doc.project.base_point(e)) else { continue };
            let p = base + DVec2::new(0.13, 0.07);
            let r = doc.project.dim_ref_at(target, p).unwrap();
            let dim = Dimension {
                view,
                a: p,
                b: p + DVec2::new(3.0, 0.0),
                offset: 1.0,
                kind: DimensionKind::Aligned,
                refs: [Some(r), None],
                locked: None,
            };
            let id = doc.edit("dim", |e| {
                let id = crate::ElementId(e.alloc_id());
                e.add_element(crate::Element {
                    id,
                    level,
                    type_id: ty,
                    mark: String::new(),
                    comments: String::new(),
                    kind: ElementKind::Dimension(Box::new(dim)),
                    uid: Default::default(),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                })
            });
            out.push((id, p));
        }
        out
    }

    #[test]
    fn dimensions_keep_their_points_through_a_mirror() {
        let mut doc = sample();
        let dims = dimensions_to_everything(&mut doc);
        // Walls (lines walked the other way), beams and grids (lines that keep
        // their direction), columns and components (local frames), doors and windows.
        assert!(dims.len() >= 20, "{}", dims.len());
        let t = PlanTransform::mirror_across(DVec2::new(30.0, 0.0), DVec2::new(0.3, 1.0));
        let check = |doc: &Document, id: crate::ElementId, p: DVec2| {
            let d = dim(doc, id);
            assert!(d.refs[0].is_some(), "{d:?}");
            assert!((d.a - t.apply(p)).length() < 1e-9, "{:?}: {} (expected {})", d.refs[0], d.a, t.apply(p));
        };
        // Copies: each target (or a door's or window's wall) mirrored with its dimension.
        for (id, p) in &dims {
            let target = dim(&doc, *id).refs[0].and_then(|r| r.element()).unwrap();
            let host = doc.project.element(target).and_then(|e| e.as_hosted()).map(|h| h.host);
            let created = doc.edit("mirror copy", |e| e.copy_elements(&[host.unwrap_or(target), *id], &[t]));
            let copy = created
                .iter()
                .copied()
                .find(|c| matches!(doc.project.element(*c).map(|e| &e.kind), Some(ElementKind::Dimension(_))));
            check(&doc, copy.unwrap(), *p);
        }
        // In place: the elements are mirrored, their dimensions follow.
        let originals: Vec<crate::ElementId> =
            dims.iter().filter_map(|(id, _)| dim(&doc, *id).refs[0].and_then(|r| r.element())).collect();
        doc.edit("mirror", |e| e.transform_elements(&originals, &t));
        for (id, p) in &dims {
            check(&doc, *id, *p);
        }
    }

    #[test]
    fn every_copy_of_an_array_keeps_its_dimensions() {
        let mut doc = sample();
        let (wall, id) = dimension_to_wall(&mut doc);
        let a = dim(&doc, id).a;
        let steps: Vec<PlanTransform> =
            (1..=3).map(|i| PlanTransform::translate(DVec2::new(0.0, 20.0 * i as f64))).collect();
        let created = doc.edit("array", |e| e.copy_elements(&[wall, id], &steps));
        let dims: Vec<Dimension> = created
            .iter()
            .filter_map(|c| match &doc.project.element(*c)?.kind {
                ElementKind::Dimension(d) => Some((**d).clone()),
                _ => None,
            })
            .collect();
        assert_eq!(dims.len(), 3);
        for (d, t) in dims.iter().zip(&steps) {
            let target = d.refs[0].and_then(|r| r.element());
            assert!(target.is_some_and(|w| created.contains(&w)), "{d:?}");
            assert!((d.a - t.apply(a)).length() < 1e-9);
        }
    }
}
