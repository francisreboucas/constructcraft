//! Room boundaries: the region enclosed by room-bounding walls around a point.
//!
//! The wall footprints form a planar arrangement once their edges are split
//! where they meet. A ray from the room point finds the nearest edge, and the
//! face containing the point is traced by turning as far left as possible at
//! every vertex. Openings do not break the boundary (footprints are full walls).

use crate::polygon;
use glam::DVec2;
use std::collections::{BTreeMap, BTreeSet};

/// Vertices closer than this are merged (meters).
const MERGE: f64 = 1e-6;
const EPS: f64 = 1e-9;

type Key = (i64, i64);

fn key(p: DVec2) -> Key {
    ((p.x / MERGE).round() as i64, (p.y / MERGE).round() as i64)
}

/// Parameters along `a`-`b` where `c`-`d` touches or crosses it.
fn split_params(a: DVec2, b: DVec2, c: DVec2, d: DVec2, out: &mut Vec<f64>) {
    let r = b - a;
    let s = d - c;
    let len2 = r.length_squared();
    if len2 < EPS {
        return;
    }
    let denom = r.perp_dot(s);
    if denom.abs() < EPS * len2.sqrt() * s.length().max(EPS) {
        // Parallel: if collinear, the other segment's endpoints split this one.
        if (c - a).perp_dot(r).abs() < 1e-7 * len2.sqrt() {
            for p in [c, d] {
                let t = (p - a).dot(r) / len2;
                if t > 0.0 && t < 1.0 {
                    out.push(t);
                }
            }
        }
        return;
    }
    let t = (c - a).perp_dot(s) / denom;
    let u = (c - a).perp_dot(r) / denom;
    let tol = 1e-9;
    if (-tol..=1.0 + tol).contains(&t) && (-tol..=1.0 + tol).contains(&u) && t > tol && t < 1.0 - tol {
        out.push(t);
    }
}

/// Undirected edges of the arrangement of `polygons`' edges, keyed by vertex.
fn arrangement(polygons: &[Vec<DVec2>]) -> (BTreeMap<Key, DVec2>, BTreeSet<(Key, Key)>) {
    let segs: Vec<(DVec2, DVec2)> =
        polygons.iter().flat_map(|p| (0..p.len()).map(move |i| (p[i], p[(i + 1) % p.len()]))).collect();
    arrangement_of(segs)
}

/// Undirected edges of the arrangement of `segs`, split where they meet, keyed by vertex.
fn arrangement_of(segs: Vec<(DVec2, DVec2)>) -> (BTreeMap<Key, DVec2>, BTreeSet<(Key, Key)>) {
    let segs: Vec<(DVec2, DVec2)> =
        segs.into_iter().filter(|(a, b)| a.is_finite() && b.is_finite() && a.distance(*b) > MERGE).collect();
    let mut points = BTreeMap::new();
    let mut edges = BTreeSet::new();
    for (i, &(a, b)) in segs.iter().enumerate() {
        let mut ts = vec![0.0, 1.0];
        for (j, &(c, d)) in segs.iter().enumerate() {
            if i != j {
                split_params(a, b, c, d, &mut ts);
            }
        }
        ts.sort_by(f64::total_cmp);
        for w in ts.windows(2) {
            let (p, q) = (a.lerp(b, w[0]), a.lerp(b, w[1]));
            let (kp, kq) = (key(p), key(q));
            if kp == kq {
                continue;
            }
            points.entry(kp).or_insert(p);
            points.entry(kq).or_insert(q);
            edges.insert(if kp < kq { (kp, kq) } else { (kq, kp) });
        }
    }
    (points, edges)
}

/// Trace the face to the left of the directed edge `start`.
fn trace_face(
    points: &BTreeMap<Key, DVec2>,
    neighbors: &BTreeMap<Key, Vec<Key>>,
    start: (Key, Key),
) -> Option<Vec<DVec2>> {
    let mut face = Vec::new();
    let (mut a, mut b) = start;
    for _ in 0..100_000 {
        face.push(points[&a]);
        let (pa, pb) = (points[&a], points[&b]);
        let back = (pa - pb).y.atan2((pa - pb).x);
        // The neighbor first reached turning clockwise from the way back is the
        // sharpest left turn, which keeps the face on the left.
        let next = neighbors[&b]
            .iter()
            .map(|n| {
                let d = points[n] - pb;
                let mut turn = (back - d.y.atan2(d.x)).rem_euclid(std::f64::consts::TAU);
                if turn < 1e-12 {
                    turn = std::f64::consts::TAU;
                }
                (turn, *n)
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))?
            .1;
        (a, b) = (b, next);
        if (a, b) == start {
            return Some(face);
        }
    }
    None
}

/// Remove vertices where the boundary continues straight on.
fn simplify(poly: Vec<DVec2>) -> Vec<DVec2> {
    let n = poly.len();
    let keep: Vec<DVec2> = (0..n)
        .filter(|&i| {
            let (prev, p, next) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            (p - prev).perp_dot(next - p).abs() > 1e-9
        })
        .map(|i| poly[i])
        .collect();
    if keep.len() >= 3 { keep } else { poly }
}

/// Boundary (counter-clockwise) of the region enclosed by `walls` (wall
/// footprints) that contains `point`, or `None` if the point is not enclosed.
pub fn room_boundary(walls: &[Vec<DVec2>], point: DVec2) -> Option<Vec<DVec2>> {
    if walls.iter().any(|w| polygon::contains(w, point)) {
        return None;
    }
    RoomArrangement::new(walls.to_vec()).boundary(point)
}

/// The arrangement of a set of wall footprints, made once to find the
/// boundaries of many rooms among the same walls (see [`room_boundary`]).
pub struct RoomArrangement {
    walls: Vec<Vec<DVec2>>,
    points: BTreeMap<Key, DVec2>,
    edges: BTreeSet<(Key, Key)>,
    neighbors: BTreeMap<Key, Vec<Key>>,
}

impl RoomArrangement {
    pub fn new(walls: Vec<Vec<DVec2>>) -> Self {
        let (points, edges) = arrangement(&walls);
        let mut neighbors: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
        for &(a, b) in &edges {
            neighbors.entry(a).or_default().push(b);
            neighbors.entry(b).or_default().push(a);
        }
        Self { walls, points, edges, neighbors }
    }

    /// [`room_boundary`] among these walls.
    pub fn room_boundary(&self, point: DVec2) -> Option<Vec<DVec2>> {
        if self.walls.iter().any(|w| polygon::contains(w, point)) {
            return None;
        }
        self.boundary(point)
    }

    /// Every space the walls' outlines close, their own bodies among them:
    /// the bounded faces of the arrangement, each counter-clockwise (a face
    /// around others, such as a room around a free-standing wall, as its
    /// outer boundary alone). Whatever lies in none of them is open to the
    /// outside. Each face is traced once.
    pub fn spaces(&self) -> Vec<Vec<DVec2>> {
        let mut seen: BTreeSet<(Key, Key)> = BTreeSet::new();
        let mut out = Vec::new();
        for &(a, b) in &self.edges {
            for start in [(a, b), (b, a)] {
                if seen.contains(&start) {
                    continue;
                }
                let Some(face) = trace_face(&self.points, &self.neighbors, start) else {
                    seen.insert(start);
                    continue;
                };
                let keys: Vec<Key> = face.iter().map(|p| key(*p)).collect();
                for i in 0..keys.len() {
                    seen.insert((keys[i], keys[(i + 1) % keys.len()]));
                }
                // The face around a connected part of the walls runs clockwise.
                if polygon::signed_area(&face) > 1e-6 {
                    out.push(simplify(face));
                }
            }
        }
        out
    }

    /// The boundary of the face around `point` (not inside a wall).
    fn boundary(&self, point: DVec2) -> Option<Vec<DVec2>> {
        let (points, edges, neighbors) = (&self.points, &self.edges, &self.neighbors);
        // Edges hit by a ray from the point towards +X (nudged off vertex heights), nearest first.
        let y = point.y + 1.234_567e-7;
        let mut hits: Vec<(f64, (Key, Key))> = edges
            .iter()
            .filter_map(|&(ka, kb)| {
                let (a, b) = (points[&ka], points[&kb]);
                if (a.y > y) == (b.y > y) {
                    return None;
                }
                let x = a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x);
                (x > point.x).then_some((x - point.x, (ka, kb)))
            })
            .collect();
        hits.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (_, (ka, kb)) in hits {
            let (a, b) = (points[&ka], points[&kb]);
            // Orient the edge so the point lies on its left.
            let start = if (b - a).perp_dot(point - a) > 0.0 { (ka, kb) } else { (kb, ka) };
            let Some(face) = trace_face(points, neighbors, start) else { continue };
            if polygon::signed_area(&face) > 0.0 && polygon::contains(&face, point) {
                return Some(simplify(face));
            }
        }
        None
    }
}

/// Most edges an outline is traced from (the arrangement grows with the
/// square of the segments; a level's walls stay far below this).
const MAX_SEGMENTS: usize = 20_000;

/// The outer boundary (counter-clockwise) of the arrangement of `lines`
/// (open polylines, such as wall location lines): the loop around the
/// largest enclosed area, with lines inside it and lines hanging off it left
/// out. `None` when the lines enclose nothing.
pub fn outer_boundary(lines: &[Vec<DVec2>]) -> Option<Vec<DVec2>> {
    let segs: Vec<(DVec2, DVec2)> = lines.iter().flat_map(|l| l.windows(2).map(|w| (w[0], w[1]))).collect();
    if segs.len() > MAX_SEGMENTS {
        return None;
    }
    let (points, edges) = arrangement_of(segs);
    let mut neighbors: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    for &(a, b) in &edges {
        neighbors.entry(a).or_default().push(b);
        neighbors.entry(b).or_default().push(a);
    }
    // Every face, each traced once from one of its directed edges; the outer
    // face of each connected part runs clockwise.
    let mut seen: BTreeSet<(Key, Key)> = BTreeSet::new();
    let mut best: Option<(f64, Vec<DVec2>)> = None;
    for &(a, b) in &edges {
        for start in [(a, b), (b, a)] {
            if seen.contains(&start) {
                continue;
            }
            let Some(face) = trace_face(&points, &neighbors, start) else { continue };
            // Mark the face's directed edges as traced.
            let keys: Vec<Key> = face.iter().map(|p| key(*p)).collect();
            for i in 0..keys.len() {
                seen.insert((keys[i], keys[(i + 1) % keys.len()]));
            }
            let mut outline = without_spurs(face);
            outline.reverse();
            let area = polygon::signed_area(&outline);
            if area > 1e-4 && best.as_ref().is_none_or(|(a, _)| area > *a) {
                best = Some((area, outline));
            }
        }
    }
    best.map(|(_, outline)| simplify(outline))
}

/// A face without the spurs that dangling lines leave in it (out to a free
/// end and back the same way).
fn without_spurs(mut poly: Vec<DVec2>) -> Vec<DVec2> {
    let same = |a: DVec2, b: DVec2| key(a) == key(b);
    let mut changed = true;
    while changed && poly.len() >= 3 {
        changed = false;
        let n = poly.len();
        for i in 0..n {
            let (prev, next) = (poly[(i + n - 1) % n], poly[(i + 1) % n]);
            if same(prev, next) {
                // Drop the tip and one copy of the point it was reached from.
                let (hi, lo) = if (i + 1) % n > i { ((i + 1) % n, i) } else { (i, (i + 1) % n) };
                poly.remove(hi);
                poly.remove(lo);
                changed = true;
                break;
            }
        }
    }
    poly
}

/// A stretch of a space's boundary and the wall whose face it lies on
/// (`None`: a room separation line, or a face no wall of the list has).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryEdge {
    pub a: DVec2,
    pub b: DVec2,
    pub wall: Option<cc_core::ElementId>,
}

/// How far a boundary edge may lie from a wall's face and still be on it, meters.
const ON_FACE: f64 = 1e-4;

/// The edges of `boundary` (as [`room_boundary`] gives it) with the wall of
/// `walls` (footprints by id) each lies on. A side of the boundary that
/// runs along several walls in line is cut where they change.
pub fn boundary_edges(boundary: &[DVec2], walls: &[(cc_core::ElementId, Vec<DVec2>)]) -> Vec<BoundaryEdge> {
    let n = boundary.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (Some(&a), Some(&b)) = (boundary.get(i), boundary.get((i + 1) % n)) else { continue };
        let length = a.distance(b);
        let Some(u) = (b - a).try_normalize() else { continue };
        // The stretches of this side that lie on an edge of a wall's footprint.
        let mut pieces: Vec<(f64, f64, cc_core::ElementId)> = Vec::new();
        for (id, foot) in walls {
            let m = foot.len();
            for k in 0..m {
                let (Some(&p), Some(&q)) = (foot.get(k), foot.get((k + 1) % m)) else { continue };
                if (p - a).perp_dot(u).abs() > ON_FACE || (q - a).perp_dot(u).abs() > ON_FACE {
                    continue;
                }
                let (tp, tq) = ((p - a).dot(u), (q - a).dot(u));
                let (t0, t1) = (tp.min(tq).max(0.0), tp.max(tq).min(length));
                if t1 - t0 > ON_FACE {
                    pieces.push((t0, t1, *id));
                }
            }
        }
        pieces.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut at = 0.0;
        for (t0, t1, id) in pieces {
            // Faces overlapping an earlier one give way to it.
            let t0 = t0.max(at);
            if t1 - t0 <= ON_FACE {
                continue;
            }
            if t0 - at > ON_FACE {
                out.push(BoundaryEdge { a: a + u * at, b: a + u * t0, wall: None });
            }
            out.push(BoundaryEdge { a: a + u * t0, b: a + u * t1, wall: Some(id) });
            at = t1;
        }
        if length - at > ON_FACE {
            out.push(BoundaryEdge { a: a + u * at, b, wall: None });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline_area(lines: &[Vec<DVec2>]) -> f64 {
        polygon::area(&outer_boundary(lines).unwrap())
    }

    fn line(pts: &[(f64, f64)]) -> Vec<DVec2> {
        pts.iter().map(|&(x, y)| DVec2::new(x, y)).collect()
    }

    /// An L of outer walls with interior walls meeting its corners and sides,
    /// a wall hanging out of it and one standing free inside: the outline is the L.
    #[test]
    fn outer_boundary_ignores_interior_and_hanging_walls() {
        let l = line(&[(0.0, 0.0), (9.0, 0.0), (9.0, 5.0), (4.0, 5.0), (4.0, 9.0), (0.0, 9.0), (0.0, 0.0)]);
        let mut lines: Vec<Vec<DVec2>> = l.windows(2).map(|w| w.to_vec()).collect();
        // Interior walls: the old 9 x 5 rectangle's top side, a partition, and a free wall.
        lines.push(line(&[(0.0, 5.0), (4.0, 5.0)]));
        lines.push(line(&[(4.0, 0.0), (4.0, 5.0)]));
        lines.push(line(&[(1.0, 7.0), (3.0, 7.0)]));
        // A garden wall hanging out of a corner, and one out of a side.
        lines.push(line(&[(9.0, 0.0), (12.0, 0.0)]));
        lines.push(line(&[(0.0, 2.0), (-2.0, 2.0), (-2.0, 3.0)]));
        let outline = outer_boundary(&lines).unwrap();
        assert!((polygon::area(&outline) - (9.0 * 5.0 + 4.0 * 4.0)).abs() < 1e-9, "{outline:?}");
        assert_eq!(outline.len(), 6, "{outline:?}");
        assert!(polygon::signed_area(&outline) > 0.0);
        // Walls entered in any order and direction give the same outline.
        lines.reverse();
        for l in &mut lines {
            l.reverse();
        }
        assert!((outline_area(&lines) - 61.0).abs() < 1e-9);
    }

    #[test]
    fn outer_boundary_needs_a_closed_loop() {
        let open = vec![line(&[(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0)])];
        assert!(outer_boundary(&open).is_none());
        assert!(outer_boundary(&[]).is_none());
        // Two buildings: the larger one.
        let a = line(&[(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0), (0.0, 0.0)]);
        let b = line(&[(10.0, 0.0), (12.0, 0.0), (12.0, 2.0), (10.0, 2.0), (10.0, 0.0)]);
        assert!((outline_area(&[b, a]) - 12.0).abs() < 1e-9);
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<DVec2> {
        vec![DVec2::new(x0, y0), DVec2::new(x1, y0), DVec2::new(x1, y1), DVec2::new(x0, y1)]
    }

    /// Four walls 0.2 thick around a 4 x 3 interior, overlapping at the corners,
    /// plus a partition from the bottom wall to the top wall at x = 2.
    fn walls() -> Vec<Vec<DVec2>> {
        vec![
            rect(-0.2, -0.2, 4.2, 0.0),
            rect(-0.2, 3.0, 4.2, 3.2),
            rect(-0.2, -0.2, 0.0, 3.2),
            rect(4.0, -0.2, 4.2, 3.2),
            rect(1.95, 0.0, 2.05, 3.0),
        ]
    }

    #[test]
    fn finds_the_enclosed_region() {
        let b = room_boundary(&walls(), DVec2::new(1.0, 1.5)).unwrap();
        assert!((polygon::area(&b) - 1.95 * 3.0).abs() < 1e-9, "{b:?}");
        assert_eq!(b.len(), 4);
        let right = room_boundary(&walls(), DVec2::new(3.0, 1.0)).unwrap();
        assert!((polygon::area(&right) - 1.95 * 3.0).abs() < 1e-9);
    }

    #[test]
    fn open_or_inside_wall_is_not_a_room() {
        let mut open = walls();
        open.remove(3);
        open.remove(3);
        assert!(room_boundary(&open, DVec2::new(1.0, 1.5)).is_none());
        assert!(room_boundary(&walls(), DVec2::new(2.0, 1.5)).is_none());
        assert!(room_boundary(&walls(), DVec2::new(9.0, 1.5)).is_none());
    }

    /// The spaces of an arrangement are its rooms and its walls' bodies:
    /// together the whole of what the walls close, and nothing outside it
    /// or between walls that leave a side open.
    #[test]
    fn spaces_are_what_the_walls_close() {
        let spaces = RoomArrangement::new(walls()).spaces();
        let holding = |p: DVec2| spaces.iter().filter(|s| polygon::contains(s, p)).collect::<Vec<_>>();
        let room = holding(DVec2::new(1.0, 1.5));
        assert_eq!(room.len(), 1);
        assert!((polygon::area(room[0]) - 1.95 * 3.0).abs() < 1e-9);
        let partition = holding(DVec2::new(2.0, 1.5));
        assert_eq!(partition.len(), 1, "a wall's body is a space too");
        assert!((polygon::area(partition[0]) - 0.1 * 3.0).abs() < 1e-9);
        assert!(holding(DVec2::new(9.0, 1.5)).is_empty());
        assert!(spaces.iter().all(|s| polygon::signed_area(s) > 0.0));
        let total: f64 = spaces.iter().map(|s| polygon::area(s)).sum();
        assert!((total - 4.4 * 3.4).abs() < 1e-9, "{total}");
        // A side left open: only the walls' bodies are closed.
        let mut open = walls();
        open.remove(3);
        open.remove(3);
        let spaces = RoomArrangement::new(open).spaces();
        assert!(!spaces.iter().any(|s| polygon::contains(s, DVec2::new(1.0, 1.5))));
        assert!(spaces.iter().any(|s| polygon::contains(s, DVec2::new(-0.1, 1.5))));
        assert!(RoomArrangement::new(Vec::new()).spaces().is_empty());
    }

    #[test]
    fn free_standing_wall_inside_does_not_block() {
        let mut w = walls();
        w.remove(4);
        w.push(rect(2.5, 1.0, 2.7, 2.0));
        let b = room_boundary(&w, DVec2::new(1.0, 1.5)).unwrap();
        assert!((polygon::area(&b) - 4.0 * 3.0).abs() < 1e-9);
    }

    /// One arrangement finds every room of a grid of rooms, as tracing each
    /// on its own does.
    #[test]
    fn one_arrangement_bounds_many_rooms() {
        let n = 8;
        let mut w = Vec::new();
        for i in 0..=n {
            let x = i as f64 * 3.0;
            w.push(rect(x - 0.1, -0.1, x + 0.1, n as f64 * 3.0 + 0.1));
            w.push(rect(-0.1, x - 0.1, n as f64 * 3.0 + 0.1, x + 0.1));
        }
        let arrangement = RoomArrangement::new(w.clone());
        for i in 0..n {
            for j in 0..n {
                let p = DVec2::new(i as f64 * 3.0 + 1.5, j as f64 * 3.0 + 1.5);
                let b = arrangement.room_boundary(p).unwrap();
                assert!((polygon::area(&b) - 2.8 * 2.8).abs() < 1e-9, "{b:?}");
                assert_eq!(Some(b), room_boundary(&w, p));
            }
        }
        assert_eq!(arrangement.room_boundary(DVec2::new(3.0, 1.5)), None, "inside a wall");
    }

    #[test]
    fn boundary_edges_name_their_walls_and_split_where_they_change() {
        let id = cc_core::ElementId;
        // A 4 x 3 room: the south side is two walls in line (0..2.5 and 2.5..4), the north one has no wall.
        let boundary = vec![DVec2::new(0.0, 0.0), DVec2::new(4.0, 0.0), DVec2::new(4.0, 3.0), DVec2::new(0.0, 3.0)];
        let strip = |x0: f64, x1: f64, y0: f64, y1: f64| {
            vec![DVec2::new(x0, y0), DVec2::new(x1, y0), DVec2::new(x1, y1), DVec2::new(x0, y1)]
        };
        let walls = vec![
            (id(1), strip(-0.2, 2.5, -0.2, 0.0)),
            (id(2), strip(2.5, 4.2, -0.2, 0.0)),
            (id(3), strip(4.0, 4.2, 0.0, 3.0)),
            (id(4), strip(-0.2, 0.0, 0.0, 3.0)),
        ];
        let edges = boundary_edges(&boundary, &walls);
        let of =
            |w: Option<u64>| -> f64 { edges.iter().filter(|e| e.wall == w.map(id)).map(|e| e.a.distance(e.b)).sum() };
        assert!((of(Some(1)) - 2.5).abs() < 1e-9 && (of(Some(2)) - 1.5).abs() < 1e-9, "{edges:?}");
        assert!((of(Some(3)) - 3.0).abs() < 1e-9 && (of(Some(4)) - 3.0).abs() < 1e-9, "{edges:?}");
        assert!((of(None) - 4.0).abs() < 1e-9, "the north side has no wall: {edges:?}");
    }
}
