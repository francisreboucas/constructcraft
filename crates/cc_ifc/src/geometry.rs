//! The IFC geometry resource turned into polygon meshes: placements,
//! curves (polylines, indexed curves with arcs, conics, trimmed and
//! composite curves, B-splines), parametric and arbitrary profiles (with
//! voids), swept solids (extrusions, revolutions, swept disks, directrix
//! sweeps), CSG primitives and trees, half-space clipping, boundary
//! representations, face sets and mapped items.
//!
//! Everything is read from public schema definitions; curved surfaces are
//! approximated by facets. Every walk over references is depth-bounded and
//! malformed values are skipped, so a hostile file yields less geometry,
//! never a panic.

use crate::parse::{Entity, Param, StepFile};
use cc_core::sketch::SketchCurve;
use cc_core::structural_profile::ProfileShape;
use cc_geom::polygon;
use glam::{DAffine3, DMat3, DVec2, DVec3};
use std::f64::consts::TAU;

const MAX_DEPTH: usize = 16;
const MAX_PLACEMENT_DEPTH: usize = 64;
/// Facets of a full circle.
const CIRCLE_SEGMENTS: usize = 24;
/// Segments of a full circle on center lines that are measured (the paths of
/// reinforcing bars): 2° chords keep a length within 0.005% of the arc's.
const FINE_CIRCLE_SEGMENTS: usize = 180;
/// Most points sampled from one curve and faces produced by one item.
const MAX_CURVE_POINTS: usize = 100_000;
/// Most solids read for one product.
const MAX_SOLIDS: usize = 100_000;
/// Most segments considered when capping a clipped solid (chaining is quadratic).
const MAX_CAP_SEGMENTS: usize = 4_000;
/// Most knots of a B-spline.
const MAX_KNOTS: usize = 10_000;

/// A planar polygon in world coordinates (counter-clockwise seen from outside).
pub type Polygon = Vec<DVec3>;

/// A profile in its plane: outer loop, inner loops and, for parametric
/// rectangles and circles, their exact parameters.
#[derive(Debug, Clone)]
pub struct Profile {
    pub pts: Vec<DVec2>,
    pub holes: Vec<Vec<DVec2>>,
    pub kind: ProfileKind,
    /// False for open profiles (a curve swept into a surface).
    pub closed: bool,
    /// The outline and voids as lines, arcs and circles in the profile's
    /// plane, when every one of them reads so and some are curved (see
    /// `plan_curves`): `pts` and `holes` split the curves.
    pub curves: Option<Vec<SketchCurve>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProfileKind {
    /// Rectangle `w` x `h` centered at `center`, with its width along `x`.
    Rect {
        center: DVec2,
        x: DVec2,
        w: f64,
        h: f64,
    },
    Circle {
        center: DVec2,
        r: f64,
    },
    Poly,
}

impl Profile {
    fn poly(pts: Vec<DVec2>) -> Self {
        Self { pts, holes: Vec::new(), kind: ProfileKind::Poly, closed: true, curves: None }
    }

    /// Exact rectangle or circle, else the bounding rectangle of the outline.
    pub fn rect(&self) -> (DVec2, DVec2, f64, f64) {
        match self.kind {
            ProfileKind::Rect { center, x, w, h } => (center, x, w, h),
            ProfileKind::Circle { center, r } => (center, DVec2::X, 2.0 * r, 2.0 * r),
            ProfileKind::Poly => {
                let (lo, hi) = bounds2(&self.pts);
                ((lo + hi) * 0.5, DVec2::X, hi.x - lo.x, hi.y - lo.y)
            }
        }
    }

    /// A solid rectangle or circle (what native columns and beams can hold).
    pub fn is_simple(&self) -> bool {
        self.holes.is_empty() && !matches!(self.kind, ProfileKind::Poly)
    }

    fn transformed(mut self, xf: &DAffine3) -> Self {
        let t = |p: DVec2| xf.transform_point3(p.extend(0.0)).truncate();
        self.pts = self.pts.into_iter().map(t).collect();
        self.holes = self.holes.into_iter().map(|h| h.into_iter().map(t).collect()).collect();
        let x_axis = xf.matrix3.x_axis.truncate();
        let y_axis = xf.matrix3.y_axis.truncate();
        let uniform = (x_axis.length() - y_axis.length()).abs() < 1e-9 && x_axis.dot(y_axis).abs() < 1e-9;
        self.kind = match self.kind {
            ProfileKind::Rect { center, x, w, h } if uniform => {
                let s = x_axis.length();
                let x = xf.transform_vector3(x.extend(0.0)).truncate().normalize_or_zero();
                ProfileKind::Rect { center: t(center), x, w: w * s, h: h * s }
            }
            ProfileKind::Circle { center, r } if uniform => {
                ProfileKind::Circle { center: t(center), r: r * x_axis.length() }
            }
            _ => ProfileKind::Poly,
        };
        self.curves = self.curves.and_then(|c| map_curves(&c, xf));
        if xf.matrix3.determinant() < 0.0 {
            self.pts.reverse();
            self.holes.iter_mut().for_each(|h| h.reverse());
        }
        self
    }
}

/// An extruded profile: its plane and extrusion vector in world coordinates.
#[derive(Debug, Clone)]
pub struct Extrusion {
    pub profile: Profile,
    /// Profile plane to world.
    pub xf: DAffine3,
    /// Extrusion vector in world coordinates (direction times depth).
    pub vector: DVec3,
    /// The profile definition entity extruded.
    pub source: Option<u32>,
}

impl Extrusion {
    pub fn is_vertical(&self) -> bool {
        self.vector.length() > 1e-9 && self.vector.normalize().z.abs() > 0.999
    }

    /// Whether the profile lies in a horizontal plane, independent of the extrusion direction.
    pub fn has_level_profile(&self) -> bool {
        let level = |v: DVec3| v.z.abs() < 1e-9 * v.length().max(1.0);
        level(self.xf.matrix3.x_axis) && level(self.xf.matrix3.y_axis)
    }

    /// World XY of the profile outline (the bottom cap of a vertical extrusion).
    pub fn footprint(&self) -> Vec<DVec2> {
        self.profile.pts.iter().map(|p| self.xf.transform_point3(p.extend(0.0)).truncate()).collect()
    }

    /// Lowest and highest z of the extruded profile.
    pub fn z_range(&self) -> (f64, f64) {
        let zs = self.profile.pts.iter().map(|p| self.plane_point(*p).z);
        zs.flat_map(|z| [z, z + self.vector.z])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z)))
    }

    /// [`Extrusion::footprint`] and its voids as lines, arcs and circles,
    /// when the profile has them and lies level.
    pub fn footprint_curves(&self) -> Option<Vec<SketchCurve>> {
        if !self.has_level_profile() {
            return None;
        }
        map_curves(self.profile.curves.as_deref()?, &self.xf)
    }

    /// World XY of the profile's voids, like [`Extrusion::footprint`].
    pub fn void_footprints(&self) -> Vec<Vec<DVec2>> {
        let xy = |p: &DVec2| self.xf.transform_point3(p.extend(0.0)).truncate();
        self.profile.holes.iter().map(|h| h.iter().map(xy).collect()).collect()
    }

    pub fn plane_point(&self, p: DVec2) -> DVec3 {
        self.xf.transform_point3(p.extend(0.0))
    }

    pub fn plane_vector(&self, v: DVec2) -> DVec3 {
        self.xf.transform_vector3(v.extend(0.0))
    }
}

/// A profile revolved about an axis, in world coordinates.
#[derive(Debug, Clone)]
pub struct Revolution {
    pub profile: Profile,
    /// Profile plane to world.
    pub xf: DAffine3,
    /// A point of the axis and its unit direction.
    pub origin: DVec3,
    pub axis: DVec3,
    /// The turn about the axis (radians, right-handed), within one full turn.
    pub angle: f64,
}

impl Revolution {
    /// The profile's outline in the half plane of the axis: distance from the
    /// axis and height along it (`None` when the plane does not hold the axis,
    /// the placement is not a similarity, or the outline reaches the axis).
    pub fn section(&self) -> Option<Vec<DVec2>> {
        let (x, y, z) = (self.xf.matrix3.x_axis, self.xf.matrix3.y_axis, self.xf.matrix3.z_axis);
        let scale = x.length();
        let normal = x.cross(y).try_normalize()?;
        let tol = 1e-9 * scale * scale;
        let lengths = [y, z].iter().all(|v| (v.length() - scale).abs() <= 1e-9 * scale);
        let square = x.dot(y).abs() <= tol && y.dot(z).abs() <= tol && z.dot(x).abs() <= tol;
        let similar = scale > 1e-12 && lengths && square;
        // The axis lies in the profile's plane.
        let planar = normal.dot(self.axis).abs() < 1e-6 && normal.dot(self.origin - self.xf.translation).abs() < 1e-6;
        if !(similar && planar) {
            return None;
        }
        let across = self.axis.cross(normal);
        let pts: Vec<DVec2> = self
            .profile
            .pts
            .iter()
            .map(|p| {
                let w = self.xf.transform_point3(p.extend(0.0)) - self.origin;
                DVec2::new(w.dot(across), w.dot(self.axis))
            })
            .collect();
        // All on one side of the axis, clear of it.
        let side = pts.first()?.x.signum();
        let clear = pts.iter().all(|p| p.x * side > 1e-6 && p.is_finite());
        clear.then(|| pts.into_iter().map(|p| DVec2::new(p.x * side, p.y)).collect())
    }

    /// The unit direction from the axis towards the profile, square to the
    /// axis (where the turn starts).
    pub fn start_direction(&self) -> Option<DVec3> {
        let p = self.xf.transform_point3(self.profile.pts.first()?.extend(0.0)) - self.origin;
        (p - self.axis * p.dot(self.axis)).try_normalize()
    }
}

/// A representation item resolved to world coordinates.
#[derive(Debug, Clone, Default)]
pub struct Solid {
    /// The extruded profile, when the item is a plain extrusion.
    pub extrusion: Option<Extrusion>,
    /// The revolved profile, when the item is a plain revolution of one profile.
    pub revolution: Option<Revolution>,
    /// Faces in world coordinates.
    pub mesh: Vec<Polygon>,
    /// Every vertex of the faces.
    pub points: Vec<DVec3>,
    /// Representation item (for styles).
    pub item: u32,
    /// A half space or boolean cut the item (its extrusion is the uncut one).
    pub clipped: bool,
    /// The plain extrusions a boolean difference subtracted from it (a slab
    /// with its shafts cut out of its body).
    pub cuts: Vec<Extrusion>,
    /// Something other than those extrusions changed it (half spaces,
    /// intersections, cutters of other shapes).
    pub other_cut: bool,
    /// The center line (world) and radius of a disk swept along it (bars).
    pub disk: Option<(Vec<DVec3>, f64)>,
}

impl Solid {
    fn new(item: u32, mesh: Vec<Polygon>, extrusion: Option<Extrusion>) -> Self {
        let points = mesh.iter().flatten().copied().collect();
        Self {
            extrusion,
            revolution: None,
            mesh,
            points,
            item,
            clipped: false,
            cuts: Vec::new(),
            other_cut: false,
            disk: None,
        }
    }

    /// Replace the faces by a cut version of them; a changed solid is
    /// marked clipped (its extrusion stays the uncut one).
    pub fn replace_mesh(&mut self, mesh: Vec<Polygon>) {
        if mesh != self.mesh {
            self.points = mesh.iter().flatten().copied().collect();
            self.mesh = mesh;
            self.clipped = true;
        }
    }
}

/// `a` minus `b` when their bounds overlap (`None`: nothing to cut or too large to cut).
pub fn subtract(a: &[Polygon], b: &[Polygon]) -> Option<Vec<Polygon>> {
    if !cc_geom::csg::bounds_overlap(a, b) {
        return None;
    }
    cc_geom::csg::subtract(a, b)
}

pub fn bounds2(pts: &[DVec2]) -> (DVec2, DVec2) {
    pts.iter()
        .fold((DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)), |(lo, hi), p| (lo.min(*p), hi.max(*p)))
}

pub fn affine(x: DVec3, y: DVec3, z: DVec3, origin: DVec3) -> DAffine3 {
    DAffine3 { matrix3: DMat3::from_cols(x, y, z), translation: origin }
}

/// A right-handed frame from a Z axis and an approximate X axis.
pub fn frame(z: Option<DVec3>, x: Option<DVec3>, origin: DVec3) -> DAffine3 {
    let z = z.and_then(|z| z.try_normalize()).unwrap_or(DVec3::Z);
    let fallback = if z.x.abs() < 0.9 { DVec3::X } else { DVec3::Y };
    let x = x
        .and_then(|x| (x - z * x.dot(z)).try_normalize())
        .or_else(|| (fallback - z * fallback.dot(z)).try_normalize())
        .unwrap_or(DVec3::X);
    affine(x, z.cross(x), z, origin)
}

/// Drop a repeated closing point and consecutive duplicates.
pub fn clean_loop<T: Copy + PartialEq>(mut pts: Vec<T>, close: impl Fn(&T, &T) -> bool) -> Vec<T> {
    pts.dedup_by(|a, b| close(a, b));
    if pts.len() > 1 && pts.first().zip(pts.last()).is_some_and(|(a, b)| close(a, b)) {
        pts.pop();
    }
    pts
}

fn clean2(pts: Vec<DVec2>) -> Vec<DVec2> {
    clean_loop(pts, |a, b| a.distance(*b) < 1e-9)
}

fn clean3(pts: Vec<DVec3>) -> Vec<DVec3> {
    clean_loop(pts, |a, b| a.distance(*b) < 1e-9)
}

/// Number of facets for an arc of `angle` radians.
fn arc_segments(angle: f64) -> usize {
    segments_of(angle, CIRCLE_SEGMENTS)
}

/// Number of chords for an arc of `angle` radians at `per_circle` chords a turn.
fn segments_of(angle: f64, per_circle: usize) -> usize {
    ((angle.abs() / TAU * per_circle as f64).ceil() as usize).clamp(2, per_circle * 4)
}

/// Plan curves through the plan part of `xf`, a similarity (uniform scale,
/// turn, maybe a mirror); `None` for anything else or ellipses.
fn map_curves(curves: &[SketchCurve], xf: &DAffine3) -> Option<Vec<SketchCurve>> {
    let (x, y) = (xf.matrix3.x_axis.truncate(), xf.matrix3.y_axis.truncate());
    let scale = x.length();
    if scale < 1e-12 || (y.length() - scale).abs() > 1e-9 * scale || x.dot(y).abs() > 1e-9 * scale * scale {
        return None;
    }
    let mirrored = x.perp_dot(y) < 0.0;
    let t = |p: DVec2| xf.transform_point3(p.extend(0.0)).truncate();
    curves
        .iter()
        .map(|c| match *c {
            SketchCurve::Line { a, b } => Some(SketchCurve::Line { a: t(a), b: t(b) }),
            SketchCurve::Arc { a, b, bulge } => {
                Some(SketchCurve::Arc { a: t(a), b: t(b), bulge: if mirrored { -bulge } else { bulge } })
            }
            SketchCurve::Circle { center, radius } => {
                Some(SketchCurve::Circle { center: t(center), radius: radius * scale })
            }
            SketchCurve::Spline { ref points } => {
                Some(SketchCurve::Spline { points: points.iter().map(|p| t(*p)).collect() })
            }
            SketchCurve::Ellipse { .. } => None,
        })
        .collect()
}

/// The circle that `curves` draw when they are arcs of it turning one way
/// all the way round from end to end (a circle written as trimmed halves).
fn whole_circle(curves: &[SketchCurve]) -> Option<SketchCurve> {
    if curves.len() < 2 {
        return None;
    }
    let tol = cc_core::sketch::JOIN_TOL;
    let mut first: Option<cc_core::arc::Arc> = None;
    let mut sweep = 0.0;
    let mut end: Option<DVec2> = None;
    for c in curves {
        let SketchCurve::Arc { a, b, bulge } = *c else { return None };
        let arc = cc_core::arc::Arc::from_bulge(a, b, bulge)?;
        if let Some(f) = &first
            && (f.center.distance(arc.center) > tol
                || (f.radius - arc.radius).abs() > tol
                || f.sweep * arc.sweep <= 0.0)
        {
            return None;
        }
        if end.is_some_and(|p| p.distance(a) > tol) {
            return None;
        }
        first.get_or_insert(arc);
        sweep += arc.sweep;
        end = Some(b);
    }
    let first = first?;
    let start = curves.first().and_then(SketchCurve::ends).map(|(a, _)| a)?;
    let closes = end.is_some_and(|p| p.distance(start) <= tol) && (sweep.abs() - TAU).abs() < 1e-6;
    let circle = SketchCurve::Circle { center: first.center, radius: first.radius };
    (closes && circle.is_valid()).then_some(circle)
}

/// Points of an arc from `a` through `b` to `c` (a straight segment when collinear),
/// `segments` chords for an arc of a given angle.
fn arc3(a: DVec3, b: DVec3, c: DVec3, segments: impl Fn(f64) -> usize) -> Vec<DVec3> {
    let (ab, ac) = (b - a, c - a);
    let n = ab.cross(ac);
    if n.length_squared() < 1e-18 {
        return vec![a, c];
    }
    let center =
        a + (n.cross(ab) * ac.length_squared() + ac.cross(n) * ab.length_squared()) / (2.0 * n.length_squared());
    let u = (a - center).normalize_or_zero();
    let v = n.normalize_or_zero().cross(u);
    let r = (a - center).length();
    let angle_of = |p: DVec3| {
        let d = p - center;
        d.dot(v).atan2(d.dot(u)).rem_euclid(TAU)
    };
    let end = angle_of(c);
    let end = if end < 1e-9 { TAU } else { end };
    let n = segments(end);
    (0..=n)
        .map(|i| {
            let t = end * i as f64 / n as f64;
            center + (u * t.cos() + v * t.sin()) * r
        })
        .collect()
}

/// Signed volume of a closed mesh.
fn signed_volume(mesh: &[Polygon]) -> f64 {
    mesh.iter().flat_map(|f| (1..f.len().saturating_sub(1)).map(move |i| f[0].dot(f[i].cross(f[i + 1])))).sum::<f64>()
        / 6.0
}

/// Turn a closed mesh outwards (positive volume).
fn orient_outwards(mut mesh: Vec<Polygon>) -> Vec<Polygon> {
    if signed_volume(&mesh) < 0.0 {
        mesh.iter_mut().for_each(|f| f.reverse());
    }
    mesh
}

/// Cap faces of loops in a plane: `to3` lifts plane coordinates; faces are
/// wound so their normal is `normal` (the plane's `u` x `v` when positive).
fn cap(loops: &[Vec<DVec2>], to3: &dyn Fn(DVec2) -> DVec3, flip: bool) -> Vec<Polygon> {
    let mut out = Vec::new();
    // Even nesting depth: outer loops; odd: holes of the loop just around them.
    let depth = |i: usize| {
        let Some(p) = loops[i].first() else { return 0 };
        loops.iter().enumerate().filter(|(j, l)| *j != i && polygon::contains(l, *p)).count()
    };
    let depths: Vec<usize> = (0..loops.len()).map(depth).collect();
    for (i, outer) in loops.iter().enumerate().filter(|(i, l)| depths[*i].is_multiple_of(2) && l.len() >= 3) {
        let outer = polygon::ccw(outer);
        let holes: Vec<Vec<DVec2>> = loops
            .iter()
            .enumerate()
            .filter(|(j, l)| depths[*j] == depths[i] + 1 && l.first().is_some_and(|p| polygon::contains(&outer, *p)))
            .map(|(_, l)| {
                let mut h = polygon::ccw(l);
                h.reverse();
                h
            })
            .collect();
        let mut push = |pts: Vec<DVec3>| {
            let mut pts = pts;
            if flip {
                pts.reverse();
            }
            out.push(pts);
        };
        if holes.is_empty() {
            push(outer.iter().map(|p| to3(*p)).collect());
        } else {
            for t in polygon::triangulate_with_holes(&outer, &holes) {
                push(t.iter().map(|p| to3(*p)).collect());
            }
        }
    }
    out
}

/// Faces between consecutive rings of the same loop (`closed` loops wrap).
fn bridge(a: &[DVec3], b: &[DVec3], closed: bool, out: &mut Vec<Polygon>) {
    let n = a.len().min(b.len());
    let edges = if closed { n } else { n.saturating_sub(1) };
    for i in 0..edges {
        let j = (i + 1) % n;
        out.push(vec![a[i], a[j], b[j], b[i]]);
    }
}

/// Keep the part of `mesh` on the side of the plane where `(p - point)·normal >= 0`,
/// closing the cut with cap faces.
pub fn clip_mesh(mesh: &[Polygon], point: DVec3, normal: DVec3) -> Vec<Polygon> {
    let Some(normal) = normal.try_normalize() else { return mesh.to_vec() };
    let mut out = Vec::with_capacity(mesh.len());
    let mut segments: Vec<(DVec3, DVec3)> = Vec::new();
    for f in mesh {
        let side: Vec<f64> = f.iter().map(|p| (*p - point).dot(normal)).collect();
        if side.iter().all(|d| *d >= 0.0) {
            out.push(f.clone());
            continue;
        }
        if side.iter().all(|d| *d < 0.0) {
            continue;
        }
        let clipped = cc_geom::clip_polygon(f, point, normal);
        // The new edge of the clipped face lies on the plane.
        let n = clipped.len();
        for i in 0..n {
            let (a, b) = (clipped[i], clipped[(i + 1) % n]);
            let on = |p: DVec3| (p - point).dot(normal).abs() < 1e-9;
            if on(a) && on(b) && a.distance(b) > 1e-9 {
                segments.push((a, b));
            }
        }
        if clipped.len() >= 3 {
            out.push(clipped);
        }
    }
    if segments.is_empty() || segments.len() > MAX_CAP_SEGMENTS {
        return out;
    }
    // Chain the cut edges into loops (each edge runs along its face's boundary).
    let mut loops: Vec<Vec<DVec3>> = Vec::new();
    let mut used = vec![false; segments.len()];
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut chain = vec![segments[start].0, segments[start].1];
        for _ in 0..segments.len() {
            let Some(&tail) = chain.last() else { break };
            let next = (0..segments.len())
                .filter(|i| !used[*i])
                .map(|i| (i, segments[i].0.distance(tail).min(segments[i].1.distance(tail))))
                .filter(|(_, d)| *d < 1e-6)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let Some((i, _)) = next else { break };
            used[i] = true;
            let (a, b) = segments[i];
            chain.push(if a.distance(tail) < b.distance(tail) { b } else { a });
        }
        let chain = clean3(chain);
        if chain.len() >= 3 {
            loops.push(chain);
        }
    }
    let basis = frame(Some(normal), None, point);
    let inv = basis.inverse();
    let flat: Vec<Vec<DVec2>> =
        loops.iter().map(|l| l.iter().map(|p| inv.transform_point3(*p).truncate()).collect()).collect();
    // The cap faces away from the kept side.
    out.extend(cap(&flat, &|p| basis.transform_point3(p.extend(0.0)), true));
    out
}

/// Reads geometry from a parsed IFC file.
pub struct Geometry<'a> {
    pub f: &'a StepFile,
    /// Meters per file length unit.
    pub scale: f64,
    /// Radians per file plane angle unit.
    pub angle: f64,
    /// Chords of a full circle while curves are sampled (finer for measured
    /// center lines, see [`Geometry::measured_curve`]).
    circle_segments: std::cell::Cell<usize>,
    /// The instances being read, outermost first ([`Geometry::enter`]).
    path: std::cell::Cell<Vec<u32>>,
    /// Instances the current walk (from an empty path) may still read.
    walk_left: std::cell::Cell<usize>,
    /// What `walk_left` starts at ([`MAX_WALK_VISITS`]; smaller in tests).
    max_walk_visits: usize,
    /// Instances the whole file may still read.
    file_left: std::cell::Cell<usize>,
}

/// Most instances one walk over items, curves and profiles reads: their
/// references may fan out (twenty mapped items of one representation,
/// each mapping it again) faster than the depth bound helps.
const MAX_WALK_VISITS: usize = 1_000_000;
/// Most instances the walks of one file read in all.
const MAX_FILE_VISITS: usize = 50_000_000;

/// An instance on the current path, taken off when dropped.
struct Visit<'g, 'a> {
    g: &'g Geometry<'a>,
}

impl Drop for Visit<'_, '_> {
    fn drop(&mut self) {
        let mut path = self.g.path.take();
        path.pop();
        self.g.path.set(path);
    }
}

impl<'a> Geometry<'a> {
    pub fn new(f: &'a StepFile, scale: f64, angle: f64) -> Self {
        Self {
            f,
            scale,
            angle,
            circle_segments: std::cell::Cell::new(CIRCLE_SEGMENTS),
            path: std::cell::Cell::new(Vec::new()),
            walk_left: std::cell::Cell::new(MAX_WALK_VISITS),
            max_walk_visits: MAX_WALK_VISITS,
            file_left: std::cell::Cell::new(MAX_FILE_VISITS),
        }
    }

    /// Start reading instance `param` within a walk: `None` when it is
    /// being read already (a cycle) or the walk or the file has read too
    /// many instances; else a guard that keeps it on the path.
    fn enter(&self, param: &Param) -> Option<Visit<'_, 'a>> {
        let id = param.as_ref()?;
        let mut path = self.path.take();
        if path.is_empty() {
            self.walk_left.set(self.max_walk_visits);
        }
        let (walk, file) = (self.walk_left.get(), self.file_left.get());
        if walk == 0 || file == 0 || path.contains(&id) {
            self.path.set(path);
            return None;
        }
        self.walk_left.set(walk - 1);
        self.file_left.set(file - 1);
        path.push(id);
        self.path.set(path);
        Some(Visit { g: self })
    }

    /// Chords for an arc of `angle` radians at the current sampling.
    fn arc_chords(&self, angle: f64) -> usize {
        segments_of(angle, self.circle_segments.get())
    }

    /// A curve whose length is measured (a bar's center line): its arcs are
    /// sampled at 2° rather than at the facets of meshes.
    pub fn measured_curve(&self, param: &Param) -> Vec<DVec3> {
        let before = self.circle_segments.replace(FINE_CIRCLE_SEGMENTS);
        let pts = self.curve(param, 0);
        self.circle_segments.set(before);
        pts
    }

    fn len(&self, p: &Param) -> Option<f64> {
        p.as_f64().map(|v| v * self.scale).filter(|v| v.is_finite())
    }

    fn positive(&self, p: &Param) -> Option<f64> {
        self.len(p).filter(|v| *v > 0.0)
    }

    pub fn point(&self, param: &Param) -> Option<DVec3> {
        let e = self.f.deref(param)?;
        if e.name != "IFCCARTESIANPOINT" {
            return None;
        }
        self.coords(e.arg(0))
    }

    /// A coordinate list `(x, y[, z])` in meters.
    fn coords(&self, list: &Param) -> Option<DVec3> {
        let c: Vec<f64> = list.as_list().iter().filter_map(Param::as_f64).collect();
        let p = match c.as_slice() {
            [x, y] => DVec3::new(*x, *y, 0.0),
            [x, y, z, ..] => DVec3::new(*x, *y, *z),
            _ => return None,
        } * self.scale;
        p.is_finite().then_some(p)
    }

    pub fn direction(&self, param: &Param) -> Option<DVec3> {
        let e = self.f.deref(param)?;
        if e.name != "IFCDIRECTION" {
            return None;
        }
        let c: Vec<f64> = e.arg(0).as_list().iter().filter_map(Param::as_f64).collect();
        let d = match c.as_slice() {
            [x, y] => DVec3::new(*x, *y, 0.0),
            [x, y, z, ..] => DVec3::new(*x, *y, *z),
            _ => return None,
        };
        d.try_normalize()
    }

    /// `IfcAxis2Placement3D` / `2D` / `IfcAxis1Placement` as a transform (identity if unset).
    pub fn axis_placement(&self, param: &Param) -> DAffine3 {
        let Some(e) = self.f.deref(param) else { return DAffine3::IDENTITY };
        let origin = self.point(e.arg(0)).unwrap_or(DVec3::ZERO);
        match e.name.as_str() {
            "IFCAXIS2PLACEMENT3D" => frame(self.direction(e.arg(1)), self.direction(e.arg(2)), origin),
            "IFCAXIS2PLACEMENT2D" => frame(None, self.direction(e.arg(1)), origin),
            "IFCAXIS1PLACEMENT" => frame(self.direction(e.arg(1)), None, origin),
            _ => DAffine3::from_translation(origin),
        }
    }

    /// World transform of an object placement chain.
    pub fn placement(&self, param: &Param) -> DAffine3 {
        self.placement_at(param, 0)
    }

    fn placement_at(&self, param: &Param, depth: usize) -> DAffine3 {
        if depth > MAX_PLACEMENT_DEPTH {
            return DAffine3::IDENTITY;
        }
        let Some(e) = self.f.deref(param) else { return DAffine3::IDENTITY };
        match e.name.as_str() {
            "IFCLOCALPLACEMENT" => {
                let parent = if e.arg(0).as_ref().is_some() {
                    self.placement_at(e.arg(0), depth + 1)
                } else {
                    DAffine3::IDENTITY
                };
                parent * self.axis_placement(e.arg(1))
            }
            // Grid placements put the object at a grid intersection; the
            // object's own direction is not resolved.
            "IFCGRIDPLACEMENT" => {
                let at = self.grid_intersection(e.arg(0)).unwrap_or(DVec3::ZERO);
                DAffine3::from_translation(at)
            }
            _ => DAffine3::IDENTITY,
        }
    }

    /// World point of an `IfcVirtualGridIntersection` (two grid axes and offsets).
    fn grid_intersection(&self, param: &Param) -> Option<DVec3> {
        let f = self.f;
        let v = f.deref(param)?;
        let axes: Vec<&Entity> = v.arg(0).as_list().iter().filter_map(|a| f.deref(a)).collect();
        let line = |axis: &Entity| -> Option<(DVec3, DVec3)> {
            let pts = self.curve(axis.arg(1), 0);
            let (a, b) = (*pts.first()?, *pts.last()?);
            // The axis curve lies in the grid's placement, found through its user.
            let grid = f.entities.values().find(|g| {
                g.name == "IFCGRID"
                    && (7..10)
                        .any(|i| g.arg(i).as_list().iter().any(|x| f.deref(x).is_some_and(|x| std::ptr::eq(x, axis))))
            });
            let xf = grid.map_or(DAffine3::IDENTITY, |g| self.placement_at(g.arg(5), 1));
            Some((xf.transform_point3(a), xf.transform_point3(b)))
        };
        let (a0, a1) = line(axes.first()?)?;
        let (b0, b1) = line(axes.get(1)?)?;
        let (d1, d2) = ((a1 - a0).truncate(), (b1 - b0).truncate());
        let t = polygon::line_intersect(a0.truncate(), d1, b0.truncate(), d2)?;
        Some((a0.truncate() + d1 * t).extend(a0.z))
    }

    /// `IfcCartesianTransformationOperator2D/3D` (uniform or not) as a transform.
    pub fn operator(&self, param: &Param) -> DAffine3 {
        let Some(e) = self.f.deref(param) else { return DAffine3::IDENTITY };
        let three = e.name.contains("3D");
        let x = self.direction(e.arg(0)).unwrap_or(DVec3::X);
        let y = self.direction(e.arg(1)).unwrap_or_else(|| DVec3::Z.cross(x).try_normalize().unwrap_or(DVec3::Y));
        let origin = self.point(e.arg(2)).unwrap_or(DVec3::ZERO);
        let ok = |s: Option<f64>| s.filter(|s| s.is_finite() && *s > 0.0);
        let scale = ok(e.arg(3).as_f64()).unwrap_or(1.0);
        let z = if three {
            self.direction(e.arg(4)).unwrap_or_else(|| x.cross(y).try_normalize().unwrap_or(DVec3::Z))
        } else {
            DVec3::Z
        };
        let (s2, s3) = if e.name.ends_with("NONUNIFORM") {
            let i = if three { 5 } else { 4 };
            (ok(e.arg(i).as_f64()).unwrap_or(scale), ok(e.arg(i + 1).as_f64()).unwrap_or(scale))
        } else {
            (scale, scale)
        };
        affine(x * scale, y * s2, z * if three { s3 } else { 1.0 }, origin)
    }

    // --- Curves ---

    /// Points of a curve in its own coordinates (2D curves at z = 0).
    pub fn curve(&self, param: &Param, depth: usize) -> Vec<DVec3> {
        let f = self.f;
        let Some(e) = f.deref(param) else { return Vec::new() };
        if depth > MAX_DEPTH {
            return Vec::new();
        }
        let Some(_visit) = self.enter(param) else { return Vec::new() };
        let mut pts = match e.name.as_str() {
            "IFCPOLYLINE" => e.arg(0).as_list().iter().filter_map(|p| self.point(p)).collect(),
            "IFCINDEXEDPOLYCURVE" => self.indexed_curve(e),
            "IFCCOMPOSITECURVE" | "IFCCOMPOSITECURVEONSURFACE" | "IFCBOUNDARYCURVE" | "IFCOUTERBOUNDARYCURVE" => {
                let mut out: Vec<DVec3> = Vec::new();
                for seg in e.arg(0).as_list().iter().filter_map(|s| f.deref(s)) {
                    let mut part = self.curve(seg.arg(2), depth + 1);
                    if seg.arg(1).as_bool() == Some(false) {
                        part.reverse();
                    }
                    out.extend(part);
                    if out.len() > MAX_CURVE_POINTS {
                        break;
                    }
                }
                out
            }
            "IFCTRIMMEDCURVE" => self.trimmed(e, depth),
            "IFCCIRCLE" | "IFCELLIPSE" => {
                let n = self.circle_segments.get();
                let mut pts: Vec<DVec3> =
                    (0..=n).filter_map(|i| self.conic_point(e, i as f64 / n as f64 * TAU)).collect();
                pts.pop();
                // Closed: repeat the first point so callers see a closed curve.
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
                pts
            }
            "IFCBSPLINECURVEWITHKNOTS" | "IFCRATIONALBSPLINECURVEWITHKNOTS" => self.bspline(e),
            "IFCLINE" => Vec::new(),
            "IFCOFFSETCURVE2D" | "IFCOFFSETCURVE3D" | "IFCOFFSETCURVE" => self.curve(e.arg(0), depth + 1),
            _ => Vec::new(),
        };
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        pts.truncate(MAX_CURVE_POINTS);
        pts
    }

    /// `IfcIndexedPolyCurve`: point list and optional line/arc segments.
    fn indexed_curve(&self, e: &Entity) -> Vec<DVec3> {
        let Some(list) = self.f.deref(e.arg(0)) else { return Vec::new() };
        let points: Vec<Option<DVec3>> = list.arg(0).as_list().iter().map(|c| self.coords(c)).collect();
        let at = |i: &Param| -> Option<DVec3> {
            let i = i.as_f64()? as usize;
            *points.get(i.checked_sub(1)?)?
        };
        let segments = e.arg(1).as_list();
        if segments.is_empty() {
            return points.into_iter().flatten().collect();
        }
        let mut out: Vec<DVec3> = Vec::new();
        for seg in segments {
            let Param::Typed(kind, inner) = seg else { continue };
            let idx: Vec<DVec3> = inner.as_list().iter().filter_map(at).collect();
            match (kind.as_str(), idx.as_slice()) {
                ("IFCARCINDEX", [a, b, c]) => out.extend(arc3(*a, *b, *c, |t| self.arc_chords(t))),
                _ => out.extend(idx),
            }
            if out.len() > MAX_CURVE_POINTS {
                break;
            }
        }
        out
    }

    /// A 2D curve as lines, arcs and circles in its own coordinates (polylines,
    /// indexed poly curves, composite curves of those, circles and trimmed
    /// circles and lines); `None` when any part is something else (ellipses,
    /// splines, offsets) or the curves are too many for a sketch.
    pub fn plan_curves(&self, param: &Param, depth: usize) -> Option<Vec<SketchCurve>> {
        let f = self.f;
        let e = f.deref(param)?;
        if depth > MAX_DEPTH {
            return None;
        }
        let _visit = self.enter(param)?;
        let lines = |pts: &[DVec3]| -> Vec<SketchCurve> {
            pts.windows(2)
                .filter(|w| w[0].truncate().distance(w[1].truncate()) > cc_core::sketch::JOIN_TOL)
                .map(|w| SketchCurve::Line { a: w[0].truncate(), b: w[1].truncate() })
                .collect()
        };
        // An arc through sampled points, in halves so each turns less than half a circle.
        let arcs = |pts: &[DVec3]| -> Option<Vec<SketchCurve>> {
            let n = pts.len().checked_sub(1).filter(|n| *n >= 2)?;
            let cuts: Vec<usize> = if n >= 4 { vec![0, n / 2, n] } else { vec![0, n] };
            cuts.windows(2)
                .map(|w| {
                    let (a, b) = (pts.get(w[0])?.truncate(), pts.get(w[1])?.truncate());
                    let m = pts.get((w[0] + w[1]) / 2)?.truncate();
                    let bulge = cc_core::arc::Arc::bulge_through(a, m, b)?;
                    Some(SketchCurve::Arc { a, b, bulge })
                })
                .collect()
        };
        let curves = match e.name.as_str() {
            "IFCPOLYLINE" => {
                let pts: Vec<DVec3> = e.arg(0).as_list().iter().filter_map(|p| self.point(p)).collect();
                lines(&pts)
            }
            "IFCINDEXEDPOLYCURVE" => {
                let list = f.deref(e.arg(0))?;
                let points: Vec<Option<DVec3>> = list.arg(0).as_list().iter().map(|c| self.coords(c)).collect();
                let at = |i: &Param| -> Option<DVec3> {
                    let i = i.as_f64()? as usize;
                    *points.get(i.checked_sub(1)?)?
                };
                let segments = e.arg(1).as_list();
                if segments.is_empty() {
                    lines(&points.iter().copied().collect::<Option<Vec<_>>>()?)
                } else {
                    let mut out = Vec::new();
                    for seg in segments {
                        let Param::Typed(kind, inner) = seg else { return None };
                        let idx: Vec<DVec3> = inner.as_list().iter().map(at).collect::<Option<_>>()?;
                        match (kind.as_str(), idx.as_slice()) {
                            ("IFCARCINDEX", [a, m, b]) => {
                                let (a, m, b) = (a.truncate(), m.truncate(), b.truncate());
                                match cc_core::arc::Arc::bulge_through(a, m, b) {
                                    Some(bulge) => out.push(SketchCurve::Arc { a, b, bulge }),
                                    None => out.extend(lines(&[a.extend(0.0), b.extend(0.0)])),
                                }
                            }
                            ("IFCLINEINDEX", _) => out.extend(lines(&idx)),
                            _ => return None,
                        }
                        if out.len() > cc_core::sketch::MAX_CURVES {
                            return None;
                        }
                    }
                    out
                }
            }
            "IFCCOMPOSITECURVE" | "IFCBOUNDARYCURVE" | "IFCOUTERBOUNDARYCURVE" => {
                let mut out: Vec<SketchCurve> = Vec::new();
                for seg in e.arg(0).as_list() {
                    let seg = f.deref(seg)?;
                    let mut part = self.plan_curves(seg.arg(2), depth + 1)?;
                    if seg.arg(1).as_bool() == Some(false) {
                        part = part.iter().rev().map(SketchCurve::reversed).collect();
                    }
                    out.extend(part);
                    if out.len() > cc_core::sketch::MAX_CURVES {
                        return None;
                    }
                }
                // Arcs of one circle that turn all the way round: the circle (#368).
                match whole_circle(&out) {
                    Some(circle) => vec![circle],
                    None => out,
                }
            }
            "IFCTRIMMEDCURVE" => match f.deref(e.arg(0))?.name.as_str() {
                // One arc, or the circle when it turns all the way round, else
                // arcs through its points.
                "IFCCIRCLE" => match self.circle_arc(e) {
                    Some(curve) => vec![curve],
                    None => arcs(&self.trimmed(e, depth))?,
                },
                "IFCLINE" => lines(&self.trimmed(e, depth)),
                _ => return None,
            },
            "IFCCIRCLE" => {
                let center = self.axis_placement(e.arg(0)).transform_point3(DVec3::ZERO).truncate();
                vec![SketchCurve::Circle { center, radius: self.positive(e.arg(1))? }]
            }
            _ => return None,
        };
        (curves.len() <= cc_core::sketch::MAX_CURVES && curves.iter().all(SketchCurve::is_valid)).then_some(curves)
    }

    /// Point of an `IfcCircle` / `IfcEllipse` at angle `t` (radians).
    fn conic_point(&self, e: &Entity, t: f64) -> Option<DVec3> {
        let xf = self.axis_placement(e.arg(0));
        let a = self.positive(e.arg(1))?;
        let b = if e.name == "IFCELLIPSE" { self.positive(e.arg(2))? } else { a };
        Some(xf.transform_point3(DVec3::new(a * t.cos(), b * t.sin(), 0.0)))
    }

    /// Parameter of `p` on a conic (its angle in the conic's frame).
    fn conic_parameter(&self, e: &Entity, p: DVec3) -> Option<f64> {
        let local = self.axis_placement(e.arg(0)).inverse().transform_point3(p);
        let a = self.positive(e.arg(1))?;
        let b = if e.name == "IFCELLIPSE" { self.positive(e.arg(2))? } else { a };
        Some((local.y / b).atan2(local.x / a))
    }

    /// A trimmed circle as one sketch arc in its own coordinates, or its
    /// circle when it turns all the way round; `None` when it does not trim
    /// a circle, its radius or trims are unusable, or the arc turns too far
    /// for one bulge.
    fn circle_arc(&self, e: &Entity) -> Option<SketchCurve> {
        let basis = self.f.deref(e.arg(0)).filter(|b| b.name == "IFCCIRCLE")?;
        let (t0, sweep) = self.conic_trim(e, basis)?;
        if sweep.abs() >= TAU - 1e-9 {
            let center = self.axis_placement(basis.arg(0)).transform_point3(DVec3::ZERO).truncate();
            let circle = SketchCurve::Circle { center, radius: self.positive(basis.arg(1))? };
            return circle.is_valid().then_some(circle);
        }
        let at = |t: f64| self.conic_point(basis, t).map(DVec3::truncate);
        let (a, m, b) = (at(t0)?, at(t0 + sweep * 0.5)?, at(t0 + sweep)?);
        let arc = SketchCurve::Arc { a, b, bulge: cc_core::arc::Arc::bulge_through(a, m, b)? };
        arc.is_valid().then_some(arc)
    }

    /// Where a trimmed circle or ellipse starts (its parameter, radians) and
    /// the turn to its end along its sense, within one full turn (equal
    /// ends make a full turn); `None` when a trim cannot be read.
    fn conic_trim(&self, e: &Entity, basis: &Entity) -> Option<(f64, f64)> {
        let sense = e.arg(3).as_bool() != Some(false);
        let prefer_points = e.arg(4).as_enum() == Some("CARTESIAN");
        // A trim select: a parameter value and/or a point.
        let trim = |p: &Param| -> Option<f64> {
            let mut value = None;
            let mut point = None;
            for x in p.as_list() {
                match x {
                    Param::Typed(t, v) if t == "IFCPARAMETERVALUE" => value = v.as_f64(),
                    Param::Ref(_) => point = self.point(x),
                    _ => {}
                }
            }
            let from_point = point.and_then(|p| self.conic_parameter(basis, p));
            let from_value = value.map(|v| v * self.angle);
            if prefer_points { from_point.or(from_value) } else { from_value.or(from_point) }
        };
        let (a, b) = (trim(e.arg(1))?, trim(e.arg(2))?);
        let turn = |d: f64| match d.rem_euclid(TAU) {
            d if d <= 1e-12 => TAU,
            d => d,
        };
        let sweep = if sense { turn(b - a) } else { -turn(a - b) };
        (a.is_finite() && b.is_finite() && sweep.is_finite()).then_some((a, sweep))
    }

    fn trimmed(&self, e: &Entity, depth: usize) -> Vec<DVec3> {
        let f = self.f;
        let Some(basis) = f.deref(e.arg(0)) else { return Vec::new() };
        let sense = e.arg(3).as_bool() != Some(false);
        let prefer_points = e.arg(4).as_enum() == Some("CARTESIAN");
        // A trim select: a parameter value and/or a point.
        let trim = |p: &Param| -> (Option<f64>, Option<DVec3>) {
            let mut value = None;
            let mut point = None;
            for x in p.as_list() {
                match x {
                    Param::Typed(t, v) if t == "IFCPARAMETERVALUE" => value = v.as_f64(),
                    Param::Ref(_) => point = self.point(x),
                    _ => {}
                }
            }
            (value, point)
        };
        let (t1, t2) = (trim(e.arg(1)), trim(e.arg(2)));
        match basis.name.as_str() {
            "IFCCIRCLE" | "IFCELLIPSE" => {
                let Some((a, sweep)) = self.conic_trim(e, basis) else { return Vec::new() };
                let b = a + sweep;
                let n = self.arc_chords(b - a);
                (0..=n).filter_map(|i| self.conic_point(basis, a + (b - a) * i as f64 / n as f64)).collect()
            }
            "IFCLINE" => {
                let Some(origin) = self.point(basis.arg(0)) else { return Vec::new() };
                let vector = f.deref(basis.arg(1));
                let dir = vector.and_then(|v| self.direction(v.arg(0))).unwrap_or(DVec3::X);
                let magnitude = vector.and_then(|v| self.len(v.arg(1))).unwrap_or(self.scale);
                let at = |(v, p): (Option<f64>, Option<DVec3>)| -> Option<DVec3> {
                    let from_value = v.map(|t| origin + dir * magnitude * t);
                    if prefer_points { p.or(from_value) } else { from_value.or(p) }
                };
                [at(t1), at(t2)].into_iter().flatten().collect()
            }
            _ => {
                // Other bases: the stretch between the trimming points, if given.
                let mut pts = self.curve(e.arg(0), depth + 1);
                if let (Some(a), Some(b)) = (t1.1, t2.1)
                    && pts.len() >= 2
                {
                    let nearest = |q: DVec3| {
                        (0..pts.len()).min_by(|i, j| pts[*i].distance(q).total_cmp(&pts[*j].distance(q))).unwrap_or(0)
                    };
                    let (i, j) = (nearest(a), nearest(b));
                    pts = if i <= j { pts[i..=j].to_vec() } else { pts[j..=i].iter().rev().copied().collect() };
                }
                if !sense {
                    pts.reverse();
                }
                pts
            }
        }
    }

    /// Samples of a (rational) B-spline curve with knots.
    fn bspline(&self, e: &Entity) -> Vec<DVec3> {
        let ctrl: Vec<DVec3> = e.arg(1).as_list().iter().filter_map(|p| self.point(p)).collect();
        let degree = e.arg(0).as_f64().unwrap_or(0.0) as usize;
        let mults: Vec<usize> = e
            .arg(5)
            .as_list()
            .iter()
            .filter_map(|m| m.as_f64())
            .map(|m| (m.max(0.0) as usize).min(MAX_KNOTS))
            .collect();
        let values: Vec<f64> = e.arg(6).as_list().iter().filter_map(Param::as_f64).collect();
        let weights: Vec<f64> = if e.name == "IFCRATIONALBSPLINECURVEWITHKNOTS" {
            e.arg(8).as_list().iter().filter_map(Param::as_f64).collect()
        } else {
            vec![1.0; ctrl.len()]
        };
        let n = ctrl.len();
        let total = mults.iter().fold(0usize, |a, m| a.saturating_add(*m));
        if degree == 0
            || degree > 10
            || n <= degree
            || mults.len() != values.len()
            || total != n + degree + 1
            || total > MAX_KNOTS
            || weights.len() != n
            || weights.iter().any(|w| !w.is_finite() || *w <= 0.0)
        {
            return ctrl;
        }
        let knots: Vec<f64> = mults.iter().zip(&values).flat_map(|(m, v)| std::iter::repeat_n(*v, *m)).collect();
        let (start, end) = (knots[degree], knots[n]);
        if end.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
            return ctrl;
        }
        let samples = (n * 8).clamp(16, 512);
        let homogeneous: Vec<(DVec3, f64)> = ctrl.iter().zip(&weights).map(|(p, w)| (*p * *w, *w)).collect();
        (0..=samples)
            .filter_map(|i| {
                let t = start + (end - start) * i as f64 / samples as f64;
                // Knot span k with knots[k] <= t < knots[k + 1] (the last span includes the end).
                let k = (degree..n).rev().find(|k| knots[*k] <= t)?;
                let mut d: Vec<(DVec3, f64)> = (0..=degree).map(|j| homogeneous[j + k - degree]).collect();
                for r in 1..=degree {
                    for j in (r..=degree).rev() {
                        let i = j + k - degree;
                        let denom = knots[i + degree + 1 - r] - knots[i];
                        let a = if denom.abs() < 1e-15 { 0.0 } else { (t - knots[i]) / denom };
                        d[j] = (d[j - 1].0 * (1.0 - a) + d[j].0 * a, d[j - 1].1 * (1.0 - a) + d[j].1 * a);
                    }
                }
                let (p, w) = d[degree];
                (w.abs() > 1e-15).then(|| p / w).filter(|p| p.is_finite())
            })
            .collect()
    }

    // --- Profiles ---

    /// The profiles of a profile definition (several for composite profiles).
    pub fn profiles(&self, param: &Param, depth: usize) -> Vec<Profile> {
        let f = self.f;
        let Some(e) = f.deref(param) else { return Vec::new() };
        if depth > MAX_DEPTH {
            return Vec::new();
        }
        let Some(_visit) = self.enter(param) else { return Vec::new() };
        match e.name.as_str() {
            "IFCCOMPOSITEPROFILEDEF" => e.arg(2).as_list().iter().flat_map(|p| self.profiles(p, depth + 1)).collect(),
            "IFCDERIVEDPROFILEDEF" => {
                let xf = self.operator(e.arg(3));
                self.profiles(e.arg(2), depth + 1).into_iter().map(|p| p.transformed(&xf)).collect()
            }
            "IFCMIRROREDPROFILEDEF" => {
                let xf = affine(-DVec3::X, DVec3::Y, DVec3::Z, DVec3::ZERO);
                self.profiles(e.arg(2), depth + 1).into_iter().map(|p| p.transformed(&xf)).collect()
            }
            _ => self.profile(e).into_iter().collect(),
        }
    }

    /// The structural profile shape of a parametric profile definition,
    /// when the shape holds it exactly (in meters, before its position);
    /// `None` for other definitions and for what the shapes lack (edge
    /// radii, slopes, a hollow rectangle's own inner radius).
    pub fn parametric_shape(&self, e: &Entity) -> Option<ProfileShape> {
        let len = |i: usize| self.positive(e.arg(i));
        let opt = |i: usize| self.len(e.arg(i)).filter(|v| *v > 0.0).unwrap_or(0.0);
        let unset = |i: usize| e.arg(i).as_f64().is_none_or(|v| v == 0.0);
        Some(match e.name.as_str() {
            "IFCISHAPEPROFILEDEF" if unset(8) && unset(9) => {
                ProfileShape::I { bf: len(3)?, d: len(4)?, tw: len(5)?, tf: len(6)?, r: opt(7) }
            }
            "IFCUSHAPEPROFILEDEF" if unset(8) && unset(9) => {
                ProfileShape::Channel { d: len(3)?, bf: len(4)?, tw: len(5)?, tf: len(6)?, r: opt(7) }
            }
            "IFCTSHAPEPROFILEDEF" if (8..=11).all(unset) => {
                ProfileShape::T { d: len(3)?, bf: len(4)?, tw: len(5)?, tf: len(6)?, r: opt(7) }
            }
            "IFCLSHAPEPROFILEDEF" if unset(7) && unset(8) => {
                let h = len(3)?;
                ProfileShape::Angle { h, b: len(4).unwrap_or(h), t: len(5)?, r: opt(6) }
            }
            // A girth no longer than the wall is no lip a lipped channel holds: drawn as its outline.
            "IFCCSHAPEPROFILEDEF" => {
                let (d, bf, t, lip, ri) = (len(3)?, len(4)?, len(5)?, opt(6), opt(7));
                if lip <= t {
                    return None;
                }
                ProfileShape::ColdLippedU { d, bf, lip, t, ri }
            }
            "IFCRECTANGLEHOLLOWPROFILEDEF" => {
                let (b, h, t, inner, ro) = (len(3)?, len(4)?, len(5)?, opt(6), opt(7));
                if (inner - (ro - t).max(0.0)).abs() > 1e-7 {
                    return None;
                }
                ProfileShape::RectTube { h, b, t, ro }
            }
            "IFCCIRCLEHOLLOWPROFILEDEF" => ProfileShape::RoundTube { d: 2.0 * len(3)?, t: len(4)? },
            "IFCRECTANGLEPROFILEDEF" => ProfileShape::Rect { b: len(3)?, h: len(4)?, chamfer: 0.0 },
            "IFCCIRCLEPROFILEDEF" => ProfileShape::Round { d: 2.0 * len(3)? },
            _ => return None,
        })
    }

    fn profile(&self, e: &Entity) -> Option<Profile> {
        let len = |i: usize| self.positive(e.arg(i));
        let opt = |i: usize| self.len(e.arg(i)).filter(|v| *v > 0.0).unwrap_or(0.0);
        let place = |p: Profile| -> Profile {
            let xf = self.axis_placement(e.arg(2));
            p.transformed(&xf)
        };
        let poly = |pts: &[(f64, f64)]| Profile::poly(pts.iter().map(|(x, y)| DVec2::new(*x, *y)).collect());
        let ellipse = |a: f64, b: f64| -> Vec<DVec2> {
            (0..CIRCLE_SEGMENTS)
                .map(|i| i as f64 / CIRCLE_SEGMENTS as f64 * TAU)
                .map(|t| DVec2::new(a * t.cos(), b * t.sin()))
                .collect()
        };
        let rect_pts = |w: f64, h: f64| {
            vec![
                DVec2::new(-w / 2.0, -h / 2.0),
                DVec2::new(w / 2.0, -h / 2.0),
                DVec2::new(w / 2.0, h / 2.0),
                DVec2::new(-w / 2.0, h / 2.0),
            ]
        };
        let profile = match e.name.as_str() {
            "IFCRECTANGLEPROFILEDEF" => {
                let (w, h) = (len(3)?, len(4)?);
                Profile {
                    pts: rect_pts(w, h),
                    holes: Vec::new(),
                    kind: ProfileKind::Rect { center: DVec2::ZERO, x: DVec2::X, w, h },
                    closed: true,
                    curves: None,
                }
            }
            "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                let (w, h) = (len(3)?, len(4)?);
                let r = opt(5).min(w / 2.0).min(h / 2.0);
                let mut pts = Vec::new();
                let corners = [
                    (w / 2.0 - r, -h / 2.0 + r, -0.25),
                    (w / 2.0 - r, h / 2.0 - r, 0.0),
                    (-w / 2.0 + r, h / 2.0 - r, 0.25),
                    (-w / 2.0 + r, -h / 2.0 + r, 0.5),
                ];
                for (cx, cy, start) in corners {
                    for k in 0..=6 {
                        let t = (start + k as f64 / 24.0) * TAU;
                        pts.push(DVec2::new(cx + r * t.cos(), cy + r * t.sin()));
                    }
                }
                Profile::poly(clean2(pts))
            }
            "IFCRECTANGLEHOLLOWPROFILEDEF" => {
                let (w, h, t) = (len(3)?, len(4)?, len(5)?);
                let mut p = Profile::poly(rect_pts(w, h));
                if t * 2.0 < w.min(h) {
                    p.holes.push(rect_pts(w - 2.0 * t, h - 2.0 * t).into_iter().rev().collect());
                }
                p
            }
            "IFCCIRCLEPROFILEDEF" => {
                let r = len(3)?;
                Profile {
                    pts: ellipse(r, r),
                    holes: Vec::new(),
                    kind: ProfileKind::Circle { center: DVec2::ZERO, r },
                    closed: true,
                    curves: Some(vec![SketchCurve::Circle { center: DVec2::ZERO, radius: r }]),
                }
            }
            "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let (r, t) = (len(3)?, len(4)?);
                let mut p = Profile::poly(ellipse(r, r));
                let mut curves = vec![SketchCurve::Circle { center: DVec2::ZERO, radius: r }];
                if t < r {
                    p.holes.push(ellipse(r - t, r - t).into_iter().rev().collect());
                    curves.push(SketchCurve::Circle { center: DVec2::ZERO, radius: r - t });
                }
                p.curves = Some(curves);
                p
            }
            "IFCELLIPSEPROFILEDEF" => Profile::poly(ellipse(len(3)?, len(4)?)),
            "IFCISHAPEPROFILEDEF" => {
                let (w, d, tw, tf) = (len(3)?, len(4)?, len(5)?, len(6)?);
                i_shape(w, w, d, tw, tf, tf)
            }
            "IFCASYMMETRICISHAPEPROFILEDEF" => {
                let (wb, d, tw, tfb) = (len(3)?, len(4)?, len(5)?, len(6)?);
                let wt = len(8).unwrap_or(wb);
                let tft = len(9).unwrap_or(tfb);
                i_shape(wb, wt, d, tw, tfb, tft)
            }
            "IFCLSHAPEPROFILEDEF" => {
                let d = len(3)?;
                let w = len(4).unwrap_or(d);
                let t = len(5)?;
                let (x, y) = (w / 2.0, d / 2.0);
                poly(&[(-x, -y), (x, -y), (x, -y + t), (-x + t, -y + t), (-x + t, y), (-x, y)])
            }
            "IFCTSHAPEPROFILEDEF" => {
                let (d, fw, tw, tf) = (len(3)?, len(4)?, len(5)?, len(6)?);
                let (x, y, w) = (fw / 2.0, d / 2.0, tw / 2.0);
                poly(&[(-w, -y), (w, -y), (w, y - tf), (x, y - tf), (x, y), (-x, y), (-x, y - tf), (-w, y - tf)])
            }
            "IFCUSHAPEPROFILEDEF" => {
                let (d, fw, tw, tf) = (len(3)?, len(4)?, len(5)?, len(6)?);
                let (x, y) = (fw / 2.0, d / 2.0);
                poly(&[
                    (-x, -y),
                    (x, -y),
                    (x, -y + tf),
                    (-x + tw, -y + tf),
                    (-x + tw, y - tf),
                    (x, y - tf),
                    (x, y),
                    (-x, y),
                ])
            }
            "IFCCSHAPEPROFILEDEF" => {
                let (d, w, t) = (len(3)?, len(4)?, len(5)?);
                let g = opt(6).min(d / 2.0);
                let (x, y) = (w / 2.0, d / 2.0);
                if g > t {
                    poly(&[
                        (-x, -y),
                        (x, -y),
                        (x, -y + g),
                        (x - t, -y + g),
                        (x - t, -y + t),
                        (-x + t, -y + t),
                        (-x + t, y - t),
                        (x - t, y - t),
                        (x - t, y - g),
                        (x, y - g),
                        (x, y),
                        (-x, y),
                    ])
                } else {
                    poly(&[
                        (-x, -y),
                        (x, -y),
                        (x, -y + t),
                        (-x + t, -y + t),
                        (-x + t, y - t),
                        (x, y - t),
                        (x, y),
                        (-x, y),
                    ])
                }
            }
            "IFCZSHAPEPROFILEDEF" => {
                let (d, fw, tw, tf) = (len(3)?, len(4)?, len(5)?, len(6)?);
                let (w, y) = (tw / 2.0, d / 2.0);
                poly(&[
                    (-fw + w, y),
                    (w, y),
                    (w, -y + tf),
                    (fw - w, -y + tf),
                    (fw - w, -y),
                    (-w, -y),
                    (-w, y - tf),
                    (-fw + w, y - tf),
                ])
            }
            "IFCTRAPEZIUMPROFILEDEF" => {
                let (b, t, h) = (len(3)?, len(4)?, len(5)?);
                let off = self.len(e.arg(6)).unwrap_or(0.0);
                poly(&[
                    (-b / 2.0, -h / 2.0),
                    (b / 2.0, -h / 2.0),
                    (-b / 2.0 + off + t, h / 2.0),
                    (-b / 2.0 + off, h / 2.0),
                ])
            }
            "IFCARBITRARYCLOSEDPROFILEDEF" | "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
                let outer = clean2(self.curve(e.arg(2), 0).into_iter().map(|p| p.truncate()).collect());
                if outer.len() < 3 {
                    return None;
                }
                let holes = e
                    .arg(3)
                    .as_list()
                    .iter()
                    .map(|c| clean2(self.curve(c, 0).into_iter().map(|p| p.truncate()).collect()))
                    .filter(|h| h.len() >= 3)
                    .collect();
                // The curves too, when every loop reads as lines, arcs and circles.
                let mut curves = self.plan_curves(e.arg(2), 0);
                for c in e.arg(3).as_list() {
                    curves = curves.zip(self.plan_curves(c, 0)).map(|(mut all, more)| {
                        all.extend(more);
                        all
                    });
                }
                let curves = curves.filter(|c| c.iter().any(SketchCurve::is_curved));
                // Arbitrary profiles are already in the profile plane: no position.
                let kind = rect_kind_of(&outer);
                return Some(Profile { pts: outer, holes, kind, closed: true, curves });
            }
            "IFCARBITRARYOPENPROFILEDEF" => {
                let pts: Vec<DVec2> = self.curve(e.arg(2), 0).into_iter().map(|p| p.truncate()).collect();
                return (pts.len() >= 2).then(|| Profile {
                    pts,
                    holes: Vec::new(),
                    kind: ProfileKind::Poly,
                    closed: false,
                    curves: None,
                });
            }
            "IFCCENTERLINEPROFILEDEF" => {
                let line: Vec<DVec2> = self.curve(e.arg(2), 0).into_iter().map(|p| p.truncate()).collect();
                let t = len(3)?;
                let pts = thicken(&line, t);
                return (pts.len() >= 3).then(|| Profile::poly(pts));
            }
            _ => return None,
        };
        let mut p = place(profile);
        p.pts = clean2(p.pts);
        (p.pts.len() >= 3 && p.pts.iter().all(|q| q.is_finite())).then_some(p)
    }

    // --- Solids ---

    /// Resolve a representation item into world solids (`xf`: item coordinates to world).
    pub fn items(&self, param: &Param, xf: DAffine3, out: &mut Vec<Solid>) {
        self.items_at(param, xf, 0, out);
    }

    fn items_at(&self, param: &Param, xf: DAffine3, depth: usize, out: &mut Vec<Solid>) {
        let f = self.f;
        let Some(id) = param.as_ref() else { return };
        let Some(e) = f.get(id) else { return };
        if depth > MAX_DEPTH || out.len() > MAX_SOLIDS {
            return;
        }
        let Some(_visit) = self.enter(param) else { return };
        let world = |mesh: Vec<Polygon>| -> Vec<Polygon> {
            let flip = xf.matrix3.determinant() < 0.0;
            mesh.into_iter()
                .map(|mut f| {
                    f.iter_mut().for_each(|p| *p = xf.transform_point3(*p));
                    if flip {
                        f.reverse();
                    }
                    f
                })
                .collect()
        };
        let push = |out: &mut Vec<Solid>, mesh: Vec<Polygon>| {
            let mesh = world(mesh);
            if !mesh.is_empty() {
                out.push(Solid::new(id, mesh, None));
            }
        };
        match e.name.as_str() {
            "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
                let Some(depth_len) = self.positive(e.arg(3)) else { return };
                let position = self.axis_placement(e.arg(1));
                let dir = self.direction(e.arg(2)).unwrap_or(DVec3::Z);
                let vector = position.transform_vector3(dir * depth_len);
                let profiles = self.profiles(e.arg(0), 0);
                let single = profiles.len() == 1;
                for profile in profiles {
                    let mesh = world(extrude(&profile, &position, vector));
                    if mesh.is_empty() {
                        continue;
                    }
                    let extrusion = (single && profile.closed).then(|| Extrusion {
                        xf: xf * position,
                        vector: xf.transform_vector3(vector),
                        source: e.arg(0).as_ref(),
                        profile: profile.clone(),
                    });
                    out.push(Solid::new(id, mesh, extrusion));
                }
            }
            "IFCREVOLVEDAREASOLID" | "IFCREVOLVEDAREASOLIDTAPERED" => {
                let position = self.axis_placement(e.arg(1));
                let axis = self.axis_placement(e.arg(2));
                let angle = e.arg(3).as_f64().map(|a| a * self.angle).filter(|a| a.is_finite() && a.abs() > 1e-9);
                let Some(angle) = angle else { return };
                let angle = angle.clamp(-TAU, TAU);
                let profiles = self.profiles(e.arg(0), 0);
                let single = profiles.len() == 1;
                for profile in profiles {
                    let mesh = revolve(&profile, axis.translation, axis.matrix3.z_axis, angle);
                    let before = out.len();
                    push(
                        out,
                        mesh.into_iter()
                            .map(|f| f.into_iter().map(|p| position.transform_point3(p)).collect())
                            .collect(),
                    );
                    // The revolution itself, in world coordinates (a mirror turns it the other way).
                    let to_world = xf * position;
                    let turn = if to_world.matrix3.determinant() < 0.0 { -angle } else { angle };
                    let revolution =
                        to_world.transform_vector3(axis.matrix3.z_axis).try_normalize().map(|dir| Revolution {
                            profile: profile.clone(),
                            xf: to_world,
                            origin: to_world.transform_point3(axis.translation),
                            axis: dir,
                            angle: turn,
                        });
                    // A tapered one changes its section on the way round: no plain revolution.
                    let plain = single && profile.closed && e.name == "IFCREVOLVEDAREASOLID";
                    if let Some(s) = out.get_mut(before).filter(|_| plain) {
                        s.revolution = revolution;
                    }
                }
            }
            "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => {
                let Some(r) = self.positive(e.arg(1)) else { return };
                let path = self.curve(e.arg(0), 0);
                let before = out.len();
                push(out, sweep_disk(&path, r));
                // The center line too, through a placement that keeps disks round.
                let (x, y, z) = (xf.matrix3.x_axis, xf.matrix3.y_axis, xf.matrix3.z_axis);
                let scale = x.length();
                let round = [y.length(), z.length()].iter().all(|l| (l - scale).abs() <= 1e-9 * scale.max(1.0))
                    && x.dot(y).abs() + y.dot(z).abs() + z.dot(x).abs() <= 1e-9 * scale * scale;
                // A hollow disk (inner radius) is no bar.
                let solid = e.arg(2).as_f64().is_none_or(|inner| inner <= 0.0);
                if let Some(s) = out.get_mut(before).filter(|_| round && solid && scale > 1e-12) {
                    // Sampled finer than the mesh: bar lengths and weights are read from it.
                    let line = self.measured_curve(e.arg(0));
                    s.disk = Some((line.iter().map(|p| xf.transform_point3(*p)).collect(), r * scale));
                }
            }
            "IFCSURFACECURVESWEPTAREASOLID" | "IFCFIXEDREFERENCESWEPTAREASOLID" | "IFCDIRECTRIXCURVESWEPTAREASOLID" => {
                let position = self.axis_placement(e.arg(1));
                let path = self.curve(e.arg(2), 0);
                let reference =
                    if e.name == "IFCFIXEDREFERENCESWEPTAREASOLID" { self.direction(e.arg(5)) } else { None };
                for profile in self.profiles(e.arg(0), 0) {
                    let mesh = sweep_profile(&profile, &path, reference);
                    push(
                        out,
                        mesh.into_iter()
                            .map(|f| f.into_iter().map(|p| position.transform_point3(p)).collect())
                            .collect(),
                    );
                }
            }
            "IFCBLOCK"
            | "IFCRECTANGULARPYRAMID"
            | "IFCRIGHTCIRCULARCYLINDER"
            | "IFCRIGHTCIRCULARCONE"
            | "IFCSPHERE" => {
                let position = self.axis_placement(e.arg(0));
                let mesh = self.primitive(e);
                push(
                    out,
                    mesh.into_iter().map(|f| f.into_iter().map(|p| position.transform_point3(p)).collect()).collect(),
                );
            }
            "IFCCSGSOLID" => self.items_at(e.arg(0), xf, depth + 1, out),
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
                let op = e.arg(0).as_enum().unwrap_or("DIFFERENCE");
                let mut first = Vec::new();
                self.items_at(e.arg(1), xf, depth + 1, &mut first);
                match op {
                    "UNION" => {
                        out.extend(first);
                        self.items_at(e.arg(2), xf, depth + 1, out);
                    }
                    "DIFFERENCE" => {
                        if let Some((point, keep)) = self.half_space(e.arg(2), xf, &first) {
                            // Half spaces clip by their plane.
                            for s in &mut first {
                                let mesh = clip_mesh(&s.mesh, point, keep);
                                s.replace_mesh(mesh);
                                s.other_cut |= s.clipped;
                            }
                        } else if let Some(cutter) = self.bounded_half_space(e.arg(2), xf, &first) {
                            // A bounded half space over part of the operand: its solid there, subtracted.
                            for s in &mut first {
                                if let Some(mesh) = subtract(&s.mesh, &cutter) {
                                    s.replace_mesh(mesh);
                                    s.other_cut |= s.clipped;
                                }
                            }
                        } else {
                            let mut second = Vec::new();
                            self.items_at(e.arg(2), xf, depth + 1, &mut second);
                            for s in &mut first {
                                for cutter in &second {
                                    if let Some(mesh) = subtract(&s.mesh, &cutter.mesh) {
                                        s.replace_mesh(mesh);
                                        match &cutter.extrusion {
                                            Some(ex) if !cutter.clipped => s.cuts.push(ex.clone()),
                                            _ => s.other_cut = true,
                                        }
                                    }
                                }
                            }
                        }
                        first.retain(|s| !s.mesh.is_empty());
                        out.extend(first);
                    }
                    "INTERSECTION" => {
                        let mut second = Vec::new();
                        self.items_at(e.arg(2), xf, depth + 1, &mut second);
                        let other: Vec<Polygon> = second.into_iter().flat_map(|s| s.mesh).collect();
                        for s in &mut first {
                            if let Some(mesh) = cc_geom::csg::intersect(&s.mesh, &other) {
                                s.replace_mesh(mesh);
                                s.other_cut = true;
                            }
                        }
                        first.retain(|s| !s.mesh.is_empty());
                        out.extend(first);
                    }
                    _ => out.extend(first),
                }
            }
            "IFCMAPPEDITEM" => {
                let Some(source) = f.deref(e.arg(0)) else { return };
                let origin = self.axis_placement(source.arg(0));
                let target = self.operator(e.arg(1));
                let Some(rep) = f.deref(source.arg(1)) else { return };
                for item in rep.arg(3).as_list() {
                    self.items_at(item, xf * target * origin, depth + 1, out);
                }
            }
            "IFCFACETEDBREP" | "IFCFACETEDBREPWITHVOIDS" | "IFCADVANCEDBREP" | "IFCADVANCEDBREPWITHVOIDS" => {
                push(out, self.shell(e.arg(0)));
            }
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
                let mesh = e.arg(0).as_list().iter().flat_map(|s| self.shell(s)).collect();
                push(out, mesh);
            }
            "IFCCLOSEDSHELL" | "IFCOPENSHELL" | "IFCCONNECTEDFACESET" => push(out, self.shell(param)),
            "IFCTRIANGULATEDFACESET" | "IFCTRIANGULATEDIRREGULARNETWORK" => {
                let points = self.point_list(e.arg(0));
                let map = index_map(e.arg(4));
                let mesh = e.arg(3).as_list().iter().filter_map(|t| indexed(&points, &map, t.as_list())).collect();
                push(out, mesh);
            }
            "IFCPOLYGONALFACESET" => {
                let points = self.point_list(e.arg(0));
                let map = index_map(e.arg(3));
                let mut mesh = Vec::new();
                for face in e.arg(2).as_list().iter().filter_map(|x| f.deref(x)) {
                    let Some(outer) = indexed(&points, &map, face.arg(0).as_list()) else { continue };
                    let holes: Vec<Polygon> =
                        face.arg(1).as_list().iter().filter_map(|h| indexed(&points, &map, h.as_list())).collect();
                    mesh.extend(face_with_holes(outer, holes));
                }
                push(out, mesh);
            }
            "IFCBOUNDINGBOX" => {
                let Some(corner) = self.point(e.arg(0)) else { return };
                let size = DVec3::new(
                    self.len(e.arg(1)).unwrap_or(0.0),
                    self.len(e.arg(2)).unwrap_or(0.0),
                    self.len(e.arg(3)).unwrap_or(0.0),
                );
                if size.min_element() > 0.0 {
                    push(out, block(corner, size));
                }
            }
            _ => {}
        }
    }

    /// Point and kept-side normal of a half space subtracted from `operand`
    /// (bounded half spaces only when they enclose the whole operand).
    fn half_space(&self, param: &Param, xf: DAffine3, operand: &[Solid]) -> Option<(DVec3, DVec3)> {
        let e = self.f.deref(param)?;
        if !e.name.contains("HALFSPACESOLID") {
            return None;
        }
        let plane = self.f.deref(e.arg(0)).filter(|p| p.name == "IFCPLANE")?;
        let frame = xf * self.axis_placement(plane.arg(0));
        let normal = frame.matrix3.z_axis.try_normalize()?;
        // AgreementFlag true: the normal points away from the half space's material.
        let keep = if e.arg(1).as_bool() == Some(false) { -normal } else { normal };
        if e.name == "IFCPOLYGONALBOUNDEDHALFSPACE" {
            let position = xf * self.axis_placement(e.arg(2));
            let boundary: Vec<DVec2> = self.curve(e.arg(3), 0).into_iter().map(|p| p.truncate()).collect();
            let inv = position.inverse();
            let inside = operand
                .iter()
                .flat_map(|s| s.points.iter())
                .all(|p| polygon::contains(&boundary, inv.transform_point3(*p).truncate()));
            if boundary.len() < 3 || !inside {
                return None;
            }
        }
        Some((frame.translation, keep))
    }

    /// The solid of a polygonal bounded half space where `operand` lies (its
    /// boundary's prism from below the operand to above it, on the side of
    /// its plane its material is), as a closed mesh; `None` for other
    /// items or a boundary that is not a simple loop.
    fn bounded_half_space(&self, param: &Param, xf: DAffine3, operand: &[Solid]) -> Option<Vec<Polygon>> {
        let e = self.f.deref(param).filter(|e| e.name == "IFCPOLYGONALBOUNDEDHALFSPACE")?;
        let plane = self.f.deref(e.arg(0)).filter(|p| p.name == "IFCPLANE")?;
        let frame = xf * self.axis_placement(plane.arg(0));
        let normal = frame.matrix3.z_axis.try_normalize()?;
        // AgreementFlag true: the normal points away from the half space's material.
        let material = if e.arg(1).as_bool() == Some(false) { normal } else { -normal };
        let position = xf * self.axis_placement(e.arg(2));
        let mut boundary: Vec<DVec2> = clean2(self.curve(e.arg(3), 0).into_iter().map(|p| p.truncate()).collect());
        if boundary.first().zip(boundary.last()).is_some_and(|(a, b)| a.distance(*b) < 1e-9) {
            boundary.pop();
        }
        if boundary.len() < 3 || boundary.len() > MAX_CAP_SEGMENTS || polygon::area(&boundary) < 1e-12 {
            return None;
        }
        let inv = position.inverse();
        let (lo, hi) = operand
            .iter()
            .flat_map(|s| s.points.iter())
            .map(|p| inv.transform_point3(*p).z)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z)));
        if !(lo.is_finite() && hi.is_finite()) {
            return None;
        }
        let boundary = polygon::ccw(&boundary);
        let ring = |z: f64| -> Vec<DVec3> {
            boundary.iter().map(|p| position.transform_point3(DVec3::new(p.x, p.y, z))).collect()
        };
        let (bottom, top) = (ring(lo - 1.0), ring(hi + 1.0));
        let mut prism = Vec::with_capacity(boundary.len() + 2);
        prism.push(bottom.iter().rev().copied().collect());
        prism.push(top.clone());
        bridge(&bottom, &top, true, &mut prism);
        let prism = orient_outwards(prism);
        let cutter = clip_mesh(&prism, frame.translation, material);
        (!cutter.is_empty()).then_some(cutter)
    }

    fn primitive(&self, e: &Entity) -> Vec<Polygon> {
        let len = |i: usize| self.positive(e.arg(i));
        match e.name.as_str() {
            "IFCBLOCK" => match (len(1), len(2), len(3)) {
                (Some(x), Some(y), Some(z)) => block(DVec3::ZERO, DVec3::new(x, y, z)),
                _ => Vec::new(),
            },
            "IFCRECTANGULARPYRAMID" => {
                let (Some(x), Some(y), Some(h)) = (len(1), len(2), len(3)) else { return Vec::new() };
                let base = [DVec3::ZERO, DVec3::new(x, 0.0, 0.0), DVec3::new(x, y, 0.0), DVec3::new(0.0, y, 0.0)];
                cone(&base, DVec3::new(x / 2.0, y / 2.0, h))
            }
            "IFCRIGHTCIRCULARCYLINDER" => {
                let (Some(h), Some(r)) = (len(1), len(2)) else { return Vec::new() };
                let circle: Vec<DVec2> = (0..CIRCLE_SEGMENTS)
                    .map(|i| DVec2::from_angle(i as f64 / CIRCLE_SEGMENTS as f64 * TAU) * r)
                    .collect();
                extrude(&Profile::poly(circle), &DAffine3::IDENTITY, DVec3::Z * h)
            }
            "IFCRIGHTCIRCULARCONE" => {
                let (Some(h), Some(r)) = (len(1), len(2)) else { return Vec::new() };
                let base: Vec<DVec3> = (0..CIRCLE_SEGMENTS)
                    .map(|i| (DVec2::from_angle(i as f64 / CIRCLE_SEGMENTS as f64 * TAU) * r).extend(0.0))
                    .collect();
                cone(&base, DVec3::Z * h)
            }
            "IFCSPHERE" => {
                let Some(r) = len(1) else { return Vec::new() };
                let (rings, n) = (CIRCLE_SEGMENTS / 2, CIRCLE_SEGMENTS);
                let at = |i: usize, j: usize| {
                    let phi = std::f64::consts::PI * i as f64 / rings as f64;
                    let theta = TAU * j as f64 / n as f64;
                    DVec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), -phi.cos()) * r
                };
                let mut mesh = Vec::new();
                for i in 0..rings {
                    for j in 0..n {
                        let quad = vec![at(i, j), at(i, j + 1), at(i + 1, j + 1), at(i + 1, j)];
                        mesh.push(clean3(quad));
                    }
                }
                orient_outwards(mesh.into_iter().filter(|f| f.len() >= 3).collect())
            }
            _ => Vec::new(),
        }
    }

    /// Points of an `IfcCartesianPointList2D/3D`.
    fn point_list(&self, param: &Param) -> Vec<Option<DVec3>> {
        let Some(list) = self.f.deref(param) else { return Vec::new() };
        list.arg(0).as_list().iter().map(|c| self.coords(c)).collect()
    }

    /// Faces of an `IfcClosedShell` / `IfcOpenShell` / `IfcConnectedFaceSet`.
    fn shell(&self, param: &Param) -> Vec<Polygon> {
        let f = self.f;
        let Some(shell) = f.deref(param) else { return Vec::new() };
        let mut mesh = Vec::new();
        for face in shell.arg(0).as_list().iter().filter_map(|x| f.deref(x)) {
            let mut outer: Option<Vec<Vec<DVec3>>> = None;
            let mut inner: Vec<Polygon> = Vec::new();
            for bound in face.arg(0).as_list().iter().filter_map(|x| f.deref(x)) {
                let mut edges = self.face_loop(bound.arg(0));
                if bound.arg(1).as_bool() == Some(false) {
                    edges.reverse();
                    edges.iter_mut().for_each(|e| e.reverse());
                }
                if edges.is_empty() {
                    continue;
                }
                if bound.name == "IFCFACEOUTERBOUND" || outer.is_none() {
                    if let Some(previous) = outer.replace(edges) {
                        inner.push(previous.concat());
                    }
                } else {
                    inner.push(edges.concat());
                }
            }
            let Some(outer) = outer else { continue };
            // A reversed surface (SameSense false) flips the face.
            let flip = face.name == "IFCADVANCEDFACE" || face.name == "IFCFACESURFACE";
            let flip = flip && face.arg(2).as_bool() == Some(false);
            let mut faces = curved_face(outer, inner);
            if flip {
                faces.iter_mut().for_each(|f| f.reverse());
            }
            mesh.extend(faces);
            if mesh.len() > MAX_CURVE_POINTS * 10 {
                break;
            }
        }
        mesh
    }

    /// The edges of a loop, each as its points from start to end.
    fn face_loop(&self, param: &Param) -> Vec<Vec<DVec3>> {
        let f = self.f;
        let Some(lp) = f.deref(param) else { return Vec::new() };
        match lp.name.as_str() {
            "IFCPOLYLOOP" => {
                let pts = clean3(lp.arg(0).as_list().iter().filter_map(|p| self.point(p)).collect());
                let n = pts.len();
                (0..n).map(|i| vec![pts[i], pts[(i + 1) % n]]).collect()
            }
            "IFCEDGELOOP" => lp
                .arg(0)
                .as_list()
                .iter()
                .filter_map(|x| f.deref(x))
                .filter_map(|oriented| {
                    let edge = f.deref(oriented.arg(2))?;
                    let mut pts = self.edge(edge)?;
                    if oriented.arg(3).as_bool() == Some(false) {
                        pts.reverse();
                    }
                    Some(pts)
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Points of an `IfcEdgeCurve` (or plain edge) from its start to its end vertex.
    fn edge(&self, edge: &Entity) -> Option<Vec<DVec3>> {
        let f = self.f;
        let vertex = |p: &Param| f.deref(p).and_then(|v| self.point(v.arg(0)));
        let (a, b) = (vertex(edge.arg(0))?, vertex(edge.arg(1))?);
        if edge.name != "IFCEDGECURVE" {
            return Some(vec![a, b]);
        }
        let same_sense = edge.arg(3).as_bool() != Some(false);
        let geometry = f.deref(edge.arg(2));
        let conic = geometry.filter(|g| g.name == "IFCCIRCLE" || g.name == "IFCELLIPSE");
        let pts = if let Some(g) = conic {
            // The arc from start to end, counter-clockwise in the conic's frame when same sense.
            let (Some(t0), Some(t1)) = (self.conic_parameter(g, a), self.conic_parameter(g, b)) else {
                return Some(vec![a, b]);
            };
            let (from, to) = if same_sense { (t0, t1) } else { (t1, t0) };
            let mut to = to;
            while to <= from + 1e-9 {
                to += TAU;
            }
            let n = arc_segments(to - from);
            let mut pts: Vec<DVec3> =
                (0..=n).filter_map(|i| self.conic_point(g, from + (to - from) * i as f64 / n as f64)).collect();
            if !same_sense {
                pts.reverse();
            }
            pts
        } else {
            let mut pts = edge.arg(2).as_ref().map(|_| self.curve(edge.arg(2), 0)).unwrap_or_default();
            if pts.len() < 2 {
                return Some(vec![a, b]);
            }
            if pts.first().is_some_and(|p| p.distance(b) < p.distance(a)) {
                pts.reverse();
            }
            pts
        };
        let mut out = vec![a];
        out.extend(pts.into_iter().filter(|p| p.distance(a) > 1e-9 && p.distance(b) > 1e-9));
        out.push(b);
        Some(out)
    }
}

/// Map of `PnIndex` (1-based point indices), empty when absent.
fn index_map(param: &Param) -> Vec<usize> {
    param.as_list().iter().filter_map(|i| i.as_f64()).map(|i| i as usize).collect()
}

/// Polygon of 1-based `indices` into `points` (through `map` when given).
fn indexed(points: &[Option<DVec3>], map: &[usize], indices: &[Param]) -> Option<Polygon> {
    let pts: Option<Vec<DVec3>> = indices
        .iter()
        .map(|i| {
            let i = i.as_f64()? as usize;
            let i = if map.is_empty() { i } else { *map.get(i.checked_sub(1)?)? };
            *points.get(i.checked_sub(1)?)?
        })
        .collect();
    let pts = clean3(pts?);
    (pts.len() >= 3).then_some(pts)
}

/// Plane basis `(origin, u, v, normal)` of a polygon (Newell normal).
fn plane_of(pts: &[DVec3]) -> Option<(DVec3, DVec3, DVec3, DVec3)> {
    let n = pts.len();
    let normal: DVec3 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            DVec3::new((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y))
        })
        .sum::<DVec3>()
        .try_normalize()?;
    let origin = pts.iter().copied().sum::<DVec3>() / n as f64;
    let helper = if normal.z.abs() < 0.9 { DVec3::Z } else { DVec3::X };
    let u = normal.cross(helper).normalize();
    Some((origin, u, normal.cross(u), normal))
}

/// A planar face with holes: the face itself, or triangles when it has holes.
fn face_with_holes(outer: Polygon, holes: Vec<Polygon>) -> Vec<Polygon> {
    if holes.is_empty() {
        return vec![outer];
    }
    let Some((o, u, v, _)) = plane_of(&outer) else { return Vec::new() };
    let flat = |l: &Polygon| l.iter().map(|p| DVec2::new((*p - o).dot(u), (*p - o).dot(v))).collect::<Vec<_>>();
    let holes2: Vec<Vec<DVec2>> = holes.iter().map(flat).collect();
    polygon::triangulate_with_holes(&flat(&outer), &holes2)
        .into_iter()
        .map(|t| t.iter().map(|p| o + u * p.x + v * p.y).collect())
        .collect()
}

/// A face from its boundary edges: planar faces as polygons; curved ones
/// (cylinders, B-spline patches) as a ruled strip between opposite edges
/// when bounded by four edges, else a fan around the centroid.
fn curved_face(edges: Vec<Vec<DVec3>>, holes: Vec<Polygon>) -> Vec<Polygon> {
    let outer = clean3(edges.iter().flat_map(|e| e.iter().copied()).collect());
    if outer.len() < 3 {
        return Vec::new();
    }
    let Some((o, _, _, normal)) = plane_of(&outer) else { return Vec::new() };
    let size = outer.iter().map(|p| p.distance(o)).fold(0.0, f64::max);
    let planar = outer.iter().all(|p| (*p - o).dot(normal).abs() <= 1e-3 * size.max(1e-3));
    if planar {
        return face_with_holes(outer, holes.into_iter().map(clean3).filter(|h| h.len() >= 3).collect());
    }
    if edges.len() == 4 && edges.iter().all(|e| e.len() >= 2) {
        // The pair of opposite edges with more points carries the curvature.
        let (i, j) = if edges[0].len() + edges[2].len() >= edges[1].len() + edges[3].len() { (0, 2) } else { (1, 3) };
        let a = &edges[i];
        let b: Vec<DVec3> = edges[j].iter().rev().copied().collect();
        let n = a.len().max(b.len()).max(2);
        let resample = |c: &[DVec3]| -> Vec<DVec3> {
            (0..n)
                .map(|k| {
                    let t = k as f64 / (n - 1) as f64 * (c.len() - 1) as f64;
                    let (lo, hi) = (t.floor() as usize, (t.ceil() as usize).min(c.len() - 1));
                    c[lo].lerp(c[hi], t - lo as f64)
                })
                .collect()
        };
        let (a, b) = (resample(a), resample(&b));
        let mut out = Vec::new();
        for k in 0..n - 1 {
            for tri in [vec![a[k], a[k + 1], b[k + 1]], vec![a[k], b[k + 1], b[k]]] {
                let tri = clean3(tri);
                if tri.len() == 3 {
                    out.push(tri);
                }
            }
        }
        return out;
    }
    let n = outer.len();
    (0..n).map(|k| vec![o, outer[k], outer[(k + 1) % n]]).collect()
}

/// The closed outline of a polyline thickened by `t` (centered).
fn thicken(line: &[DVec2], t: f64) -> Vec<DVec2> {
    let n = line.len();
    if n < 2 {
        return Vec::new();
    }
    let normal_at = |i: usize| {
        let prev = if i > 0 { (line[i] - line[i - 1]).try_normalize() } else { None };
        let next = if i + 1 < n { (line[i + 1] - line[i]).try_normalize() } else { None };
        let d = match (prev, next) {
            (Some(a), Some(b)) => (a + b).try_normalize().unwrap_or(a),
            (Some(a), None) | (None, Some(a)) => a,
            _ => DVec2::X,
        };
        // Miter: stretch the offset so straight parts keep the thickness.
        let scale = prev.zip(next).map_or(1.0, |(a, _)| 1.0 / a.dot(d).abs().max(0.25));
        d.perp() * (t / 2.0) * scale
    };
    let left: Vec<DVec2> = (0..n).map(|i| line[i] + normal_at(i)).collect();
    let right: Vec<DVec2> = (0..n).rev().map(|i| line[i] - normal_at(i)).collect();
    left.into_iter().chain(right).collect()
}

fn i_shape(wb: f64, wt: f64, d: f64, tw: f64, tfb: f64, tft: f64) -> Profile {
    let (y, w) = (d / 2.0, tw / 2.0);
    let (b, t) = (wb / 2.0, wt / 2.0);
    let pts = [
        (-b, -y),
        (b, -y),
        (b, -y + tfb),
        (w, -y + tfb),
        (w, y - tft),
        (t, y - tft),
        (t, y),
        (-t, y),
        (-t, y - tft),
        (-w, y - tft),
        (-w, -y + tfb),
        (-b, -y + tfb),
    ];
    Profile::poly(pts.iter().map(|(x, y)| DVec2::new(*x, *y)).collect())
}

/// A rectangle's exact parameters when `pts` form one.
pub(crate) fn rect_kind_of(pts: &[DVec2]) -> ProfileKind {
    if let [a, b, c, d] = pts {
        let (ab, bc, cd, da) = (*b - *a, *c - *b, *d - *c, *a - *d);
        let square = |u: DVec2, v: DVec2| u.dot(v).abs() < 1e-6 * u.length().max(1e-9) * v.length().max(1e-9);
        if square(ab, bc) && square(bc, cd) && square(cd, da) && ab.length() > 1e-9 && bc.length() > 1e-9 {
            let x = ab.normalize();
            return ProfileKind::Rect { center: (*a + *c) * 0.5, x, w: ab.length(), h: bc.length() };
        }
    }
    ProfileKind::Poly
}

/// An axis-aligned box from `corner` with `size`.
fn block(corner: DVec3, size: DVec3) -> Vec<Polygon> {
    let base = Profile::poly(vec![
        corner.truncate(),
        corner.truncate() + DVec2::new(size.x, 0.0),
        corner.truncate() + size.truncate(),
        corner.truncate() + DVec2::new(0.0, size.y),
    ]);
    extrude(&base, &DAffine3::from_translation(DVec3::Z * corner.z), DVec3::Z * size.z)
}

/// A cone or pyramid from a base polygon (in a plane) to `apex`.
fn cone(base: &[DVec3], apex: DVec3) -> Vec<Polygon> {
    let n = base.len();
    let mut mesh = vec![base.iter().rev().copied().collect::<Vec<_>>()];
    mesh.extend((0..n).map(|i| vec![base[i], base[(i + 1) % n], apex]));
    orient_outwards(mesh)
}

/// The loops of a profile: outer counter-clockwise, holes clockwise.
fn loops(profile: &Profile) -> Vec<(Vec<DVec2>, bool)> {
    if !profile.closed {
        return vec![(profile.pts.clone(), false)];
    }
    let mut out = vec![(polygon::ccw(&profile.pts), true)];
    for h in &profile.holes {
        let mut h = polygon::ccw(h);
        h.reverse();
        out.push((h, true));
    }
    out
}

/// Faces of `profile` (in the plane of `position`) extruded by `vector`
/// (in `position`'s parent coordinates).
fn extrude(profile: &Profile, position: &DAffine3, vector: DVec3) -> Vec<Polygon> {
    let base = |p: DVec2| position.transform_point3(p.extend(0.0));
    let mut mesh = Vec::new();
    for (lp, closed) in loops(profile) {
        let bottom: Vec<DVec3> = lp.iter().map(|p| base(*p)).collect();
        let top: Vec<DVec3> = bottom.iter().map(|p| *p + vector).collect();
        bridge(&bottom, &top, closed, &mut mesh);
    }
    if profile.closed {
        let outer = polygon::ccw(&profile.pts);
        let holes: Vec<Vec<DVec2>> = profile.holes.iter().map(|h| polygon::ccw(h)).collect();
        let lifted = |p: DVec2| base(p) + vector;
        mesh.extend(cap(&[vec![outer], holes].concat(), &lifted, false));
        mesh.extend(cap(
            &[vec![polygon::ccw(&profile.pts)], profile.holes.iter().map(|h| polygon::ccw(h)).collect()].concat(),
            &base,
            true,
        ));
        return orient_outwards(mesh);
    }
    mesh
}

/// Faces of `profile` revolved by `angle` about the axis through `origin`
/// along `axis` (all in the profile's plane coordinates).
fn revolve(profile: &Profile, origin: DVec3, axis: DVec3, angle: f64) -> Vec<Polygon> {
    let Some(axis) = axis.try_normalize() else { return Vec::new() };
    let n = arc_segments(angle);
    let rotate = |p: DVec3, t: f64| {
        let v = p - origin;
        let (s, c) = t.sin_cos();
        origin + v * c + axis.cross(v) * s + axis * axis.dot(v) * (1.0 - c)
    };
    let full = (angle.abs() - TAU).abs() < 1e-6;
    let mut mesh = Vec::new();
    for (lp, closed) in loops(profile) {
        let rings: Vec<Vec<DVec3>> =
            (0..=n).map(|k| lp.iter().map(|p| rotate(p.extend(0.0), angle * k as f64 / n as f64)).collect()).collect();
        for k in 0..n {
            bridge(&rings[k], &rings[k + 1], closed, &mut mesh);
        }
    }
    if profile.closed && !full {
        let all: Vec<Vec<DVec2>> = [vec![profile.pts.clone()], profile.holes.clone()].concat();
        mesh.extend(cap(&all, &|p| p.extend(0.0), true));
        mesh.extend(cap(&all, &|p| rotate(p.extend(0.0), angle), false));
    }
    let mesh: Vec<Polygon> = mesh.into_iter().map(clean3).filter(|f| f.len() >= 3).collect();
    if profile.closed { orient_outwards(mesh) } else { mesh }
}

/// Unit frames along a path (parallel transport): `(point, tangent, x)`.
fn path_frames(path: &[DVec3], reference: Option<DVec3>) -> Vec<(DVec3, DVec3, DVec3, f64)> {
    let n = path.len();
    let mut out = Vec::with_capacity(n);
    let mut x_prev: Option<DVec3> = None;
    for i in 0..n {
        let prev = if i > 0 { (path[i] - path[i - 1]).try_normalize() } else { None };
        let next = if i + 1 < n { (path[i + 1] - path[i]).try_normalize() } else { None };
        let Some(t) = (match (prev, next) {
            (Some(a), Some(b)) => (a + b).try_normalize().or(Some(b)),
            (a, b) => a.or(b),
        }) else {
            continue;
        };
        // Miter scale so the section keeps its size across corners.
        let miter = prev.zip(next).map_or(1.0, |(a, _)| 1.0 / a.dot(t).abs().max(0.3));
        let guess = reference.or(x_prev).unwrap_or_else(|| if t.z.abs() < 0.9 { DVec3::Z.cross(t) } else { DVec3::X });
        let x = (guess - t * guess.dot(t)).try_normalize().unwrap_or_else(|| t.any_orthonormal_vector());
        x_prev = Some(x);
        out.push((path[i], t, x, miter));
    }
    out
}

/// Faces of a disk of radius `r` swept along `path`.
fn sweep_disk(path: &[DVec3], r: f64) -> Vec<Polygon> {
    let circle: Vec<DVec2> =
        (0..CIRCLE_SEGMENTS).map(|i| DVec2::from_angle(i as f64 / CIRCLE_SEGMENTS as f64 * TAU) * r).collect();
    sweep_profile(&Profile::poly(circle), path, None)
}

/// Faces of `profile` swept along `path`, its plane normal to the path.
fn sweep_profile(profile: &Profile, path: &[DVec3], reference: Option<DVec3>) -> Vec<Polygon> {
    let path = clean_loop(path.to_vec(), |a, b| a.distance(*b) < 1e-9);
    let frames = path_frames(&path, reference);
    if frames.len() < 2 {
        return Vec::new();
    }
    let place = |(o, t, x, miter): &(DVec3, DVec3, DVec3, f64), p: DVec2| {
        let y = t.cross(*x);
        // Corners: stretch along the bend only, keeping the section width.
        *o + *x * p.x * *miter + y * p.y
    };
    let mut mesh = Vec::new();
    for (lp, closed) in loops(profile) {
        let rings: Vec<Vec<DVec3>> = frames.iter().map(|f| lp.iter().map(|p| place(f, *p)).collect()).collect();
        for k in 0..rings.len() - 1 {
            bridge(&rings[k], &rings[k + 1], closed, &mut mesh);
        }
    }
    if profile.closed {
        let all: Vec<Vec<DVec2>> = [vec![profile.pts.clone()], profile.holes.clone()].concat();
        if let (Some(first), Some(last)) = (frames.first(), frames.last()) {
            let start = (first.0, first.1, first.2, 1.0);
            let end = (last.0, last.1, last.2, 1.0);
            mesh.extend(cap(&all, &|p| place(&start, p), true));
            mesh.extend(cap(&all, &|p| place(&end, p), false));
        }
        return orient_outwards(mesh);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(mesh: &[Polygon]) -> f64 {
        signed_volume(mesh)
    }

    #[test]
    fn extrusion_with_hole_has_exact_volume() {
        let sq = |h: f64| vec![DVec2::new(-h, -h), DVec2::new(h, -h), DVec2::new(h, h), DVec2::new(-h, h)];
        let p = Profile { pts: sq(1.0), holes: vec![sq(0.5)], kind: ProfileKind::Poly, closed: true, curves: None };
        let mesh = extrude(&p, &DAffine3::IDENTITY, DVec3::Z * 3.0);
        assert!((volume(&mesh) - (4.0 - 1.0) * 3.0).abs() < 1e-9, "{}", volume(&mesh));
        // Extruding downwards still yields an outward mesh.
        let mesh = extrude(&p, &DAffine3::IDENTITY, -DVec3::Z * 3.0);
        assert!((volume(&mesh) - 9.0).abs() < 1e-9);
    }

    #[test]
    fn arcs_pass_through_their_points() {
        let pts = arc3(DVec3::X, DVec3::Y, -DVec3::X, arc_segments);
        assert!(pts.iter().all(|p| (p.length() - 1.0).abs() < 1e-9));
        assert!(pts.last().is_some_and(|p| p.distance(-DVec3::X) < 1e-9));
        assert!(pts.iter().all(|p| p.y >= -1e-9));
    }

    #[test]
    fn clipping_a_cube_caps_the_cut() {
        let cube = block(DVec3::ZERO, DVec3::ONE);
        let half = clip_mesh(&cube, DVec3::new(0.0, 0.0, 0.25), -DVec3::Z);
        assert!((volume(&half) - 0.25).abs() < 1e-9, "{}", volume(&half));
        // A sloped cut through the box.
        let n = DVec3::new(0.0, -1.0, -1.0);
        let wedge = clip_mesh(&cube, DVec3::new(0.0, 0.0, 1.0), n);
        assert!((volume(&wedge) - 0.5).abs() < 1e-9, "{}", volume(&wedge));
    }

    #[test]
    fn revolved_square_makes_a_ring() {
        // Square [1,2]x[0,1] revolved fully about the Y axis: a ring of volume 2π·1.5·1.
        let sq = vec![DVec2::new(1.0, 0.0), DVec2::new(2.0, 0.0), DVec2::new(2.0, 1.0), DVec2::new(1.0, 1.0)];
        let mesh = revolve(&Profile::poly(sq), DVec3::ZERO, DVec3::Y, TAU);
        let exact = TAU * 1.5;
        let v = volume(&mesh);
        // Facets of 24 segments: within 2 % of the exact volume.
        assert!((v - exact).abs() / exact < 0.02, "{v} vs {exact}");
    }

    /// `levels` representations of `fan` mapped items each, every item of
    /// one mapping the next representation; the last holds `last` (STEP).
    fn mapped_fan(levels: u32, fan: usize, last: &str) -> String {
        let mut s = String::from("#1=IFCCARTESIANPOINT((0.,0.,0.));#2=IFCAXIS2PLACEMENT3D(#1,$,$);");
        s += "#3=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#1,$,$);";
        // Level k: representation #(10k), map #(10k+1), mapped item #(10k+2) mapping it.
        for k in 1..=levels {
            let items =
                if k == levels { last.to_string() } else { vec![format!("#{}", 10 * (k + 1) + 2); fan].join(",") };
            s += &format!("#{}=IFCSHAPEREPRESENTATION($,'Body','MappedRepresentation',({items}));", 10 * k);
            s += &format!("#{}=IFCREPRESENTATIONMAP(#2,#{});", 10 * k + 1, 10 * k);
            s += &format!("#{}=IFCMAPPEDITEM(#{},#3);", 10 * k + 2, 10 * k + 1);
        }
        s
    }

    #[test]
    fn mapped_items_that_fan_out_or_cycle_are_bounded() {
        // 20 items at each of 17 levels: 20^16 visits unbounded, none with geometry.
        let f = step(&mapped_fan(17, 20, "#1"));
        let mut g = Geometry::new(&f, 1.0, 1.0);
        g.max_walk_visits = 20_000;
        let mut out = Vec::new();
        g.items(&Param::Ref(12), DAffine3::IDENTITY, &mut out);
        assert!(MAX_FILE_VISITS - g.file_left.get() <= 20_000);
        assert!(g.path.take().is_empty());
        // A representation that maps itself twenty times is read once.
        let items = format!("{},#5", vec!["#12"; 20].join(","));
        let f = step(&format!("{}#5=IFCBLOCK(#2,1.,1.,1.);", mapped_fan(1, 1, &items)));
        let g = Geometry::new(&f, 1.0, 1.0);
        let mut out = Vec::new();
        g.items(&Param::Ref(12), DAffine3::IDENTITY, &mut out);
        assert_eq!(out.len(), 1);
        assert!(MAX_FILE_VISITS - g.file_left.get() <= 25);
    }

    #[test]
    fn composite_curves_and_profiles_that_fan_out_are_bounded() {
        // Composite curves of 20 segments, each the next composite, 17 deep;
        // composite profiles likewise.
        let mut s = String::new();
        for k in 1..=17 {
            let next = if k == 17 { "#999".to_string() } else { format!("#{}", 10 * (k + 1)) };
            s += &format!("#{}=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,{next});", 10 * k + 1);
            let segments = vec![format!("#{}", 10 * k + 1); 20].join(",");
            s += &format!("#{}=IFCCOMPOSITECURVE(({segments}),.F.);", 10 * k);
            let profiles =
                if k == 17 { "#999".to_string() } else { vec![format!("#{}", 10 * (k + 1) + 5); 20].join(",") };
            s += &format!("#{}=IFCCOMPOSITEPROFILEDEF(.AREA.,$,({profiles}),$);", 10 * k + 5);
        }
        let f = step(&s);
        let mut g = Geometry::new(&f, 1.0, 1.0);
        g.max_walk_visits = 20_000;
        assert!(g.curve(&Param::Ref(10), 0).is_empty());
        assert!(g.plan_curves(&Param::Ref(10), 0).is_none_or(|c| c.is_empty()));
        assert!(g.profiles(&Param::Ref(15), 0).is_empty());
        assert!(MAX_FILE_VISITS - g.file_left.get() <= 3 * 20_000);
    }

    #[test]
    fn swept_disk_follows_its_path() {
        let path = [DVec3::ZERO, DVec3::new(4.0, 0.0, 0.0), DVec3::new(4.0, 3.0, 0.0)];
        let mesh = sweep_disk(&path, 0.1);
        let v = volume(&mesh);
        let exact = std::f64::consts::PI * 0.01 * 7.0;
        assert!((v - exact).abs() / exact < 0.05, "{v} vs {exact}");
    }

    fn step(data: &str) -> crate::parse::StepFile {
        crate::parse::parse(&format!(
            "ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
        ))
        .unwrap()
    }

    #[test]
    fn trimmed_circles_with_huge_or_infinite_parameters_end() {
        let base = "#1=IFCCARTESIANPOINT((0.,0.));#2=IFCAXIS2PLACEMENT2D(#1,$);#3=IFCCIRCLE(#2,1.);";
        for (t1, t2, sense) in
            [("0.", "1.E30", ".F."), ("1.E30", "0.", ".T."), ("0.", "1.E999", ".T."), ("-1.E999", "0.", ".F.")]
        {
            let f = step(&format!(
                "{base}#4=IFCTRIMMEDCURVE(#3,(IFCPARAMETERVALUE({t1})),(IFCPARAMETERVALUE({t2})),{sense},.PARAMETER.);"
            ));
            let g = Geometry::new(&f, 1.0, 1.0);
            let pts = g.curve(&Param::Ref(4), 0);
            assert!(pts.len() <= 4 * CIRCLE_SEGMENTS + 1 && pts.iter().all(|p| p.is_finite()), "{t1} {t2}");
        }
        // A quarter turn either way, as before.
        let f = step(&format!(
            "{base}#4=IFCTRIMMEDCURVE(#3,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5707963267948966)),.F.,.PARAMETER.);"
        ));
        let pts = Geometry::new(&f, 1.0, 1.0).curve(&Param::Ref(4), 0);
        // Clockwise from (1, 0) the long way round to (0, 1), through (0, -1).
        assert!(pts.first().is_some_and(|p| p.distance(DVec3::X) < 1e-9));
        assert!(pts.last().is_some_and(|p| p.distance(DVec3::Y) < 1e-9));
        assert!(pts.iter().any(|p| p.distance(-DVec3::Y) < 1e-9));
    }

    #[test]
    fn rounded_and_thick_lines() {
        let line = [DVec2::ZERO, DVec2::new(2.0, 0.0)];
        let pts = thicken(&line, 0.2);
        assert!((polygon::area(&pts) - 0.4).abs() < 1e-9);
        let poly = i_shape(0.2, 0.2, 0.4, 0.01, 0.02, 0.02);
        let area = 2.0 * 0.2 * 0.02 + 0.01 * 0.36;
        assert!((polygon::area(&poly.pts) - area).abs() < 1e-12);
    }
}
