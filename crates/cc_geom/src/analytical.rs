//! The analytical model: columns and beams as center lines (members) between
//! shared points (nodes), for structural analysis programs. A beam framed
//! into a column, or running on over one it passes, ends on the column's
//! axis, a beam ending on another beam ends on that beam's line, a column
//! standing on or under a beam (a truss's vertical between its chords) ends
//! on the beam's line, and a member is split wherever another member ends
//! on it, so connected members share their nodes. An arc beam's members
//! follow its arc: each carries the bulge of its own part of the arc, nodes
//! where other members connect lie on the arc, and its length is the arc's;
//! [`AnalyticalModel::split_arcs`] turns them into straight members for
//! programs that only take those. A column whose top only beams frame into,
//! within their depth, ends on the highest beam's axis, so no piece of column
//! is left standing above the joint.

use crate::{BeamEnd, ModelGeometry};
use cc_core::arc::Arc;
use cc_core::{BaseSupport, ElementId, ElementKind, Project, TypeData};
use glam::{DVec2, DVec3};

/// Points closer than this are one node (1 cm).
const TOL: f64 = 0.01;

/// How far a beam end may lie from another beam's axis, beyond its half width, and still end on it.
const REACH: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Column,
    Beam,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: usize,
    pub position: DVec3,
    /// The base of a column, where the structure rests on a support, and
    /// what holds it there.
    pub support: Option<BaseSupport>,
}

/// A member between two nodes, straight or (part of an arc beam) a circular
/// arc in plan; an element split by connections gives several members.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub id: usize,
    pub element: ElementId,
    pub kind: MemberKind,
    pub start: usize,
    pub end: usize,
    /// The bulge (`cc_core::arc`, `tan(sweep / 4)`) of the member's plan arc
    /// from its start node to its end node: 0 for a straight member.
    pub bulge: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalyticalModel {
    pub nodes: Vec<Node>,
    pub members: Vec<Member>,
}

impl AnalyticalModel {
    /// The two end points of a member.
    pub fn line(&self, member: &Member) -> Option<(DVec3, DVec3)> {
        Some((self.nodes.get(member.start)?.position, self.nodes.get(member.end)?.position))
    }

    /// The plan arc of a curved member, from its start node to its end node
    /// (`None` for a straight one). Its height runs evenly along it from the
    /// start node's to the end node's.
    pub fn arc(&self, member: &Member) -> Option<Arc> {
        let (a, b) = self.line(member)?;
        Arc::from_bulge(a.truncate(), b.truncate(), member.bulge)
    }

    /// The length of a member: along its arc for a curved one.
    pub fn length(&self, member: &Member) -> Option<f64> {
        let (a, b) = self.line(member)?;
        Some(match self.arc(member) {
            Some(arc) => arc.length().hypot(b.z - a.z),
            None => a.distance(b),
        })
    }

    /// The member as a polyline: its two ends, or its arc in chords at most
    /// `max_step` radians apart.
    pub fn points(&self, member: &Member, max_step: f64) -> Vec<DVec3> {
        let Some((a, b)) = self.line(member) else { return Vec::new() };
        match self.arc(member) {
            Some(arc) => {
                let pts = arc.points(0.0, arc.length(), 0.0, max_step);
                let n = pts.len().saturating_sub(1).max(1) as f64;
                pts.iter().enumerate().map(|(i, p)| p.extend(a.z + (b.z - a.z) * i as f64 / n)).collect()
            }
            None => vec![a, b],
        }
    }

    /// Replace every curved member by `segments` straight members (at least
    /// one) between new nodes on its arc, evenly spaced along it, for
    /// programs that take only straight members. Members are numbered again
    /// in order; the existing nodes keep their ids.
    pub fn split_arcs(&mut self, segments: usize) {
        let segments = segments.max(1);
        let mut members = Vec::with_capacity(self.members.len());
        for m in std::mem::take(&mut self.members) {
            let (Some(arc), Some((a, b))) = (self.arc(&m), self.line(&m)) else {
                members.push(Member { id: members.len(), bulge: 0.0, ..m });
                continue;
            };
            let mut chain = vec![m.start];
            for k in 1..segments {
                let f = k as f64 / segments as f64;
                let id = self.nodes.len();
                let position = arc.point(arc.length() * f).extend(a.z + (b.z - a.z) * f);
                self.nodes.push(Node { id, position, support: None });
                chain.push(id);
            }
            chain.push(m.end);
            for w in chain.windows(2) {
                members.push(Member { id: members.len(), start: w[0], end: w[1], bulge: 0.0, ..m.clone() });
            }
        }
        self.members = members;
    }
}

/// A member line before nodes are shared.
struct Line {
    element: ElementId,
    kind: MemberKind,
    a: DVec3,
    b: DVec3,
    /// Half width in plan, for beam ends landing on it.
    half: f64,
    /// Half depth measured plumb, for column ends landing on it.
    half_depth: f64,
    /// A column whose base is on the ground, not on a beam, and its support.
    support: Option<BaseSupport>,
    /// The columns a beam's ends frame into.
    framed: [Option<ElementId>; 2],
    /// The centerline arc of an arc beam (in plan).
    curve: Option<Arc>,
    /// The plan arc from `a` to `b` along `curve` (kept in step by [`Line::set_end`]).
    arc: Option<Arc>,
}

impl Line {
    /// Move an end (`b` when `end`) and fit the arc to the new ends.
    fn set_end(&mut self, end: bool, q: DVec3) {
        if end {
            self.b = q;
        } else {
            self.a = q;
        }
        self.fit_arc();
    }

    /// The arc from `a` to `b` through the middle of the part of `curve`
    /// between them: the curve itself when both ends lie on it.
    fn fit_arc(&mut self) {
        self.arc = self.curve.and_then(|c| {
            let (a, b) = (self.a.truncate(), self.b.truncate());
            let mid = c.point((c.locate(a).0 + c.locate(b).0) * 0.5);
            Arc::from_bulge(a, b, Arc::bulge_through(a, mid, b)?)
        });
    }

    /// The point of the line nearest `p` in plan, its parameter (0 at `a`,
    /// 1 at `b`) and plan distance.
    fn closest_plan(&self, p: DVec2) -> (DVec3, f64, f64) {
        let (t, d) = match &self.arc {
            Some(arc) => {
                let len = arc.length().max(1e-12);
                let t = (arc.locate(p).0 / len).clamp(0.0, 1.0);
                (t, arc.point(len * t).distance(p))
            }
            None => project_plan(self.a, self.b, p),
        };
        (self.at(t), t, d)
    }

    /// The point at parameter `t` (along the arc for a curved line).
    fn at(&self, t: f64) -> DVec3 {
        let z = self.a.z + (self.b.z - self.a.z) * t;
        match &self.arc {
            Some(arc) => arc.point(arc.length() * t).extend(z),
            None => (self.a + (self.b - self.a) * t).truncate().extend(z),
        }
    }

    /// Parameter of `p` projected on the line, and its distance to it.
    fn project(&self, p: DVec3) -> (f64, f64) {
        match &self.arc {
            Some(_) => {
                let (_, t, _) = self.closest_plan(p.truncate());
                (t, p.distance(self.at(t)))
            }
            None => project_on(self.a, self.b, p),
        }
    }
}

/// Whether `p` is inside `hull` or within [`TOL`] of its boundary.
fn covers(hull: &[DVec2], p: DVec2) -> bool {
    hull.len() >= 3
        && (crate::polygon::contains(hull, p)
            || (0..hull.len()).any(|i| {
                let (q, _) = crate::polygon::closest_on_segment(hull[i], hull[(i + 1) % hull.len()], p);
                q.distance(p) < TOL
            }))
}

/// Parameter of `p` projected on segment `a`-`b`, and its distance to it.
fn project_on(a: DVec3, b: DVec3, p: DVec3) -> (f64, f64) {
    let d = b - a;
    let len2 = d.length_squared();
    if len2 < 1e-18 {
        return (0.0, p.distance(a));
    }
    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
    (t, p.distance(a + d * t))
}

/// Parameter of `p` on the plan projection of `a`-`b` and its plan distance.
fn project_plan(a: DVec3, b: DVec3, p: DVec2) -> (f64, f64) {
    let (a2, b2) = (a.truncate(), b.truncate());
    let (q, t) = crate::polygon::closest_on_segment(a2, b2, p);
    (t, q.distance(p))
}

/// End a column's top on the axis of the highest beam framed into it there,
/// when that axis lies within the beam's half depth (and [`REACH`]) below the
/// top and nothing else meets the top: no column above, no beam on it. The
/// piece of column above the beams' axes is inside the joint, and would be
/// a short member with a free end.
fn trim_column_tops(lines: &mut [Line], first_beam: usize) {
    for i in 0..first_beam {
        let top = lines[i].b;
        let element = lines[i].element;
        let others_meet = lines.iter().enumerate().any(|(j, l)| {
            j != i
                && (l.a.distance(top) < TOL || l.b.distance(top) < TOL || {
                    let (t, d) = l.project(top);
                    d < TOL && t > 0.0 && t < 1.0
                })
        });
        if others_meet {
            continue;
        }
        let highest = lines[first_beam..]
            .iter()
            .flat_map(|l| {
                let ends = [(l.a, l.framed[0]), (l.b, l.framed[1])];
                ends.into_iter().filter(move |(_, c)| *c == Some(element)).map(move |(p, _)| (p, l.half_depth))
            })
            .filter(|(p, half_depth)| p.z < top.z - TOL && top.z - p.z <= half_depth + REACH)
            .map(|(p, _)| p.z)
            .max_by(f64::total_cmp);
        if let Some(z) = highest
            && z > lines[i].a.z + TOL
        {
            lines[i].b = top.truncate().extend(z);
        }
    }
}

/// Build the analytical model of the structural columns and the beams.
pub fn analytical_model(project: &Project, geom: &ModelGeometry) -> AnalyticalModel {
    let mut lines: Vec<Line> = Vec::new();
    let mut column_axes: Vec<(ElementId, DVec2)> = Vec::new();
    // Footprint (hull) and span of each column, for beams that pass over or under it.
    let mut footprints: Vec<(DVec2, Vec<DVec2>, (f64, f64))> = Vec::new();
    for e in project.elements.values() {
        let ElementKind::Column(c) = &e.kind else { continue };
        if !c.structural || !matches!(project.element_type(e.type_id).map(|t| &t.data), Some(TypeData::Column { .. })) {
            continue;
        }
        let (base, top) = project.column_span(e, c);
        let (a, b) = (c.position.extend(base), c.position.extend(top));
        if a.is_finite() && b.is_finite() && top - base > TOL {
            lines.push(Line {
                element: e.id,
                kind: MemberKind::Column,
                a,
                b,
                half: 0.0,
                half_depth: 0.0,
                support: Some(c.base_support),
                framed: [None; 2],
                curve: None,
                arc: None,
            });
            column_axes.push((e.id, c.position));
            let outline = project
                .element_type(e.type_id)
                .map(|t| crate::build::column_outline(project, &t.data, c.position, c.rotation_deg))
                .unwrap_or_default();
            footprints.push((c.position, crate::polygon::convex_hull(&outline), (base, top)));
        }
    }
    let first_beam = lines.len();
    for g in geom.beams.values() {
        // The centroid line, at mid depth.
        let at = |u: f64| {
            let (z0, z1) = g.z_at(u);
            g.world(u, 0.0).extend((z0 + z1) * 0.5)
        };
        let end_point = |end: BeamEnd, u: f64| match end {
            // Onto the column's axis, at the beam's height there.
            BeamEnd::Column(c) => match column_axes.iter().find(|(id, _)| *id == c) {
                Some((_, axis)) => {
                    let uc = g.locate(*axis);
                    let (z0, z1) = g.z_at(uc);
                    axis.extend((z0 + z1) * 0.5)
                }
                None => at(u),
            },
            // An end that runs on over (or under) a column it does not frame
            // into, as a truss's chord over its end vertical: on that column's axis.
            _ => {
                let p = at(u);
                let (z0, z1) = g.z_at(u);
                let reach = (z1 - z0) * 0.5 + REACH;
                footprints
                    .iter()
                    .filter(|(_, hull, span)| {
                        p.z >= span.0 - reach && p.z <= span.1 + reach && covers(hull, p.truncate())
                    })
                    .map(|(axis, _, _)| {
                        let (z0, z1) = g.z_at((*axis - g.origin).dot(g.dir));
                        axis.extend((z0 + z1) * 0.5)
                    })
                    .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
                    .unwrap_or(p)
            }
        };
        let (a, b) = (end_point(g.start, 0.0), end_point(g.end, g.length));
        if a.is_finite() && b.is_finite() && a.distance(b) > TOL {
            let (z0, z1) = g.z_at(0.0);
            let half_depth = (z1 - z0) * 0.5;
            let column = |end: BeamEnd| if let BeamEnd::Column(c) = end { Some(c) } else { None };
            let mut line = Line {
                element: g.element,
                kind: MemberKind::Beam,
                a,
                b,
                half: g.half,
                half_depth,
                support: None,
                framed: [column(g.start), column(g.end)],
                curve: g.arc,
                arc: None,
            };
            line.fit_arc();
            lines.push(line);
        }
    }
    // Beam ends that stop on another beam end on its line.
    for i in first_beam..lines.len() {
        for end in [false, true] {
            let p = if end { lines[i].b } else { lines[i].a };
            // Beams beside it in plan, at a height where the depths meet (a beam
            // far above or below stays apart); the nearest in space wins, so
            // members in one vertical plane (a truss) find their own chord.
            let host = (first_beam..lines.len())
                .filter(|&j| j != i)
                .filter_map(|j| {
                    let (q, _, d) = lines[j].closest_plan(p.truncate());
                    let near = d <= lines[j].half + REACH
                        && (q.z - p.z).abs() <= lines[i].half.max(lines[j].half) * 4.0 + REACH;
                    near.then_some((q, q.distance(p)))
                })
                .min_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((q, _)) = host {
                lines[i].set_end(end, q);
            }
        }
    }

    // Column ends that stop at a beam passing them (its face, or inside its
    // section) end on its line; a base that does stands on the beam, not on
    // a support. A beam framed into the column meets it on its axis already.
    for i in 0..first_beam {
        for top in [false, true] {
            let p = if top { lines[i].b } else { lines[i].a };
            let host = (first_beam..lines.len())
                .filter(|&j| !lines[j].framed.contains(&Some(lines[i].element)))
                .filter_map(|j| {
                    let (q, _, d) = lines[j].closest_plan(p.truncate());
                    let near = d <= lines[j].half + REACH && (q.z - p.z).abs() <= lines[j].half_depth + REACH;
                    near.then_some((q, q.distance(p)))
                })
                .min_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((q, _)) = host {
                // On the column's own axis, at the beam's height.
                let q = p.truncate().extend(q.z);
                if top {
                    lines[i].b = q;
                } else {
                    lines[i].a = q;
                    lines[i].support = None;
                }
            }
        }
    }

    trim_column_tops(&mut lines, first_beam);

    // Shared nodes.
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_at = |p: DVec3, support: Option<BaseSupport>| -> usize {
        if let Some(n) = nodes.iter_mut().find(|n| n.position.distance(p) < TOL) {
            // Two column bases on one node: the stiffer support holds.
            n.support = match (n.support, support) {
                (Some(BaseSupport::Fixed), _) | (_, Some(BaseSupport::Fixed)) => Some(BaseSupport::Fixed),
                (a, b) => a.or(b),
            };
            return n.id;
        }
        let id = nodes.len();
        nodes.push(Node { id, position: p, support });
        id
    };
    let ends: Vec<(usize, usize)> = lines.iter().map(|l| (node_at(l.a, l.support), node_at(l.b, None))).collect();

    // Split each line at the nodes lying on it.
    let mut members = Vec::new();
    for (l, &(s, e)) in lines.iter().zip(&ends) {
        let mut on: Vec<(f64, usize)> = nodes
            .iter()
            .filter(|n| n.id != s && n.id != e)
            .filter_map(|n| {
                let (t, d) = l.project(n.position);
                (d < TOL && t > 1e-9 && t < 1.0 - 1e-9).then_some((t, n.id))
            })
            .collect();
        on.sort_by(|x, y| x.0.total_cmp(&y.0));
        let chain: Vec<(f64, usize)> = std::iter::once((0.0, s)).chain(on).chain([(1.0, e)]).collect();
        for w in chain.windows(2) {
            let ((t0, start), (t1, end)) = (w[0], w[1]);
            if start != end {
                // Each part of an arc keeps the bulge of its own part of the arc.
                let bulge = l.arc.map_or(0.0, |arc| arc.bulge_between(arc.length() * t0, arc.length() * t1));
                members.push(Member { id: members.len(), element: l.element, kind: l.kind, start, end, bulge });
            }
        }
    }
    AnalyticalModel { nodes, members }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::{Beam, Category, Column, Document, Element, WallTop};

    fn frame() -> (Project, Vec<ElementId>) {
        // Two columns 5 m apart and a beam between them, and a beam from the
        // middle of that beam going north.
        let beams = vec![
            Beam::new(DVec2::ZERO, DVec2::new(5.0, 0.0), 3.0),
            Beam::new(DVec2::new(2.5, 0.0), DVec2::new(2.5, 4.0), 3.0),
        ];
        structure(&[DVec2::ZERO, DVec2::new(5.0, 0.0)], beams)
    }

    /// Structural columns 3 m tall at `columns`, then `beams`; the ids in that order.
    fn structure(columns: &[DVec2], beams: Vec<Beam>) -> (Project, Vec<ElementId>) {
        let mut doc = Document::new(Project::from_template());
        let mut ids = Vec::new();
        doc.edit("frame", |e| {
            let level = e.project().levels[0].id;
            let mut add = |e: &mut cc_core::EditCtx, category, kind| {
                let type_id = e.project().default_type(category).unwrap();
                let id = ElementId(e.alloc_id());
                let (mark, comments, uid) = (String::new(), String::new(), Default::default());
                e.add_element(Element {
                    id,
                    level,
                    type_id,
                    mark,
                    comments,
                    kind,
                    uid,
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                });
                ids.push(id);
            };
            for &position in columns {
                let column = Column {
                    position,
                    rotation_deg: 0.0,
                    base_offset: 0.0,
                    height: 3.0,
                    top: WallTop::Unconnected,
                    structural: true,
                    base_support: Default::default(),
                };
                add(e, Category::Column, ElementKind::Column(column));
            }
            for beam in beams {
                add(e, Category::Beam, ElementKind::Beam(beam));
            }
        });
        (doc.project, ids)
    }

    #[test]
    fn members_share_nodes_at_their_connections() {
        let (project, ids) = frame();
        let geom = ModelGeometry::build(&project);
        let m = analytical_model(&project, &geom);
        let of = |id: ElementId| m.members.iter().filter(|x| x.element == id).collect::<Vec<_>>();
        // The 0.40 m beams' axes are at 2.80 m: the columns end there, the
        // piece above inside the joint.
        assert_eq!(of(ids[0]).len(), 1);
        assert_eq!(of(ids[1]).len(), 1);
        // The main beam runs from column axis to column axis, split by the secondary beam.
        let main = of(ids[2]);
        assert_eq!(main.len(), 2);
        let (a, _) = m.line(main[0]).unwrap();
        let (_, b) = m.line(main[1]).unwrap();
        assert!(a.distance(DVec3::new(0.0, 0.0, 2.8)) < 1e-9 && b.distance(DVec3::new(5.0, 0.0, 2.8)) < 1e-9);
        // The secondary beam starts on the main beam's line.
        let (start, _) = m.line(of(ids[3])[0]).unwrap();
        assert!(start.distance(DVec3::new(2.5, 0.0, 2.8)) < 1e-9, "{start}");
        // Column bases (the supports), two beam-column joints, the tee and the free end.
        assert_eq!(m.nodes.len(), 6);
        assert_eq!(m.nodes.iter().filter(|n| n.support == Some(BaseSupport::Fixed)).count(), 2);
        assert!(!m.nodes.iter().any(|n| n.position.distance(DVec3::new(0.0, 0.0, 3.0)) < 1e-9));
    }

    /// A quarter ring beam of radius 2.5 m about the origin between two
    /// columns, and a radial beam from the origin to its middle.
    #[test]
    fn arc_beams_keep_their_arcs() {
        use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let r = 2.5;
        let mut ring = Beam::new(DVec2::new(r, 0.0), DVec2::new(0.0, r), 3.0);
        ring.bulge = (PI / 8.0).tan();
        let mid = DVec2::from_angle(FRAC_PI_4) * r;
        let radial = Beam::new(DVec2::ZERO, mid, 3.0);
        let (project, ids) = structure(&[DVec2::new(r, 0.0), DVec2::new(0.0, r)], vec![ring, radial]);
        let geom = ModelGeometry::build(&project);
        let mut m = analytical_model(&project, &geom);
        let parts: Vec<Member> = m.members.iter().filter(|x| x.element == ids[2]).cloned().collect();
        assert_eq!(parts.len(), 2);
        let total: f64 = parts.iter().map(|x| m.length(x).unwrap()).sum();
        assert!((total - r * FRAC_PI_2).abs() < 1e-9, "{total}");
        for part in &parts {
            assert!((part.bulge - (PI / 16.0).tan()).abs() < 1e-9, "{}", part.bulge);
            let (a, b) = m.line(part).unwrap();
            assert!((a.truncate().length() - r).abs() < 1e-9 && (b.truncate().length() - r).abs() < 1e-9);
            // Drawn along the arc.
            let pts = m.points(part, cc_core::arc::ARC_STEP);
            assert!(pts.len() > 2 && pts.iter().all(|p| (p.truncate().length() - r).abs() < 1e-9));
        }
        // The radial beam ends on the arc, at the node splitting it.
        let (_, end) = m.line(m.members.iter().find(|x| x.element == ids[3]).unwrap()).unwrap();
        assert!(end.truncate().distance(mid) < 1e-9, "{end}");
        // Split into straight members: 3 chords per part, nodes on the arc.
        let nodes = m.nodes.len();
        m.split_arcs(3);
        assert_eq!(m.nodes.len(), nodes + 4);
        let chords: Vec<&Member> = m.members.iter().filter(|x| x.element == ids[2]).collect();
        assert_eq!(chords.len(), 6);
        assert!(m.members.iter().enumerate().all(|(i, x)| x.id == i && x.bulge == 0.0));
        // 15° each: 2 r sin(7.5°).
        let chord = 2.0 * r * (PI / 24.0).sin();
        for c in chords {
            let (a, b) = m.line(c).unwrap();
            assert!((a.truncate().length() - r).abs() < 1e-9 && (b.truncate().length() - r).abs() < 1e-9);
            assert!((m.length(c).unwrap() - chord).abs() < 1e-9);
        }
    }
}
