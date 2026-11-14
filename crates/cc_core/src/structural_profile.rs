//! Structural profiles: the cross-sections of columns, beams and framing
//! members as named project objects (`Project::profiles`). A profile is a
//! standard parametric shape (rolled, welded, cold-formed, timber or
//! concrete) or a parametric sketch ([`ParamSketch`]). Lengths are meters.
//! Outlines are laid out as in steel tables: the depth along +y, the web of
//! an I or channel vertical, the flanges of a channel and the legs of an
//! angle pointing to +x; centered on the bounding box.

use crate::component::{ComponentParam, ParamKind};
use crate::profile::{self, ProfilePoint};
use crate::{ProfileId, Uid};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Segments of an arc that turns a quarter circle (fillets use this many).
pub const ARC_SEGMENTS: usize = 8;
/// Most points in an outline, holes included.
pub const MAX_POINTS: usize = 4096;
/// Most holes in an outline.
pub const MAX_OPENINGS: usize = 64;
/// Smallest dimension, meters.
pub const MIN_THICKNESS: f64 = 0.0003;
/// Largest dimension, meters.
pub const MAX_SIZE: f64 = 10.0;
/// Longest profile name, characters.
pub const MAX_NAME: usize = 100;
/// Longest description, characters.
pub const MAX_DESCRIPTION: usize = 200;
/// Most profiles in a project (a crafted file).
pub const MAX_PROFILES: usize = 5000;

/// What a profile is made of; decides the material a type made for it gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileMaterial {
    #[default]
    Steel,
    ColdFormed,
    Timber,
    Concrete,
}

impl ProfileMaterial {
    pub const ALL: [ProfileMaterial; 4] = [Self::Steel, Self::ColdFormed, Self::Timber, Self::Concrete];

    pub fn code(self) -> &'static str {
        match self {
            Self::Steel => "steel",
            Self::ColdFormed => "cold_formed",
            Self::Timber => "timber",
            Self::Concrete => "concrete",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.code().eq_ignore_ascii_case(c.trim()))
    }

    /// The i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Self::Steel => "profile-material-steel",
            Self::ColdFormed => "profile-material-cold-formed",
            Self::Timber => "profile-material-timber",
            Self::Concrete => "profile-material-concrete",
        }
    }

    /// The `MaterialClass` a type made for this profile gets its material from.
    pub fn material_class(self) -> crate::MaterialClass {
        match self {
            Self::Steel | Self::ColdFormed => crate::MaterialClass::Metal,
            Self::Timber => crate::MaterialClass::Wood,
            Self::Concrete => crate::MaterialClass::Concrete,
        }
    }
}

/// A standard parametric cross-section; every field is a length in meters
/// except `ColdZ::angle` (degrees).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProfileShape {
    /// Rolled or welded I/H: depth, flange width, web and flange thickness, root radius.
    I {
        d: f64,
        bf: f64,
        tw: f64,
        tf: f64,
        r: f64,
    },
    /// I with different flanges.
    IAsym {
        d: f64,
        bf_top: f64,
        tf_top: f64,
        bf_bot: f64,
        tf_bot: f64,
        tw: f64,
    },
    /// Channel, web on the left, flanges to +x.
    Channel {
        d: f64,
        bf: f64,
        tw: f64,
        tf: f64,
        r: f64,
    },
    /// T with the flange on top.
    T {
        d: f64,
        bf: f64,
        tw: f64,
        tf: f64,
        r: f64,
    },
    /// Angle, legs to +x (`b`) and +y (`h`).
    Angle {
        h: f64,
        b: f64,
        t: f64,
        r: f64,
    },
    /// Cold-formed channel without lips; `ri` is the inner bend radius.
    ColdU {
        d: f64,
        bf: f64,
        t: f64,
        ri: f64,
    },
    /// Cold-formed channel with lips.
    ColdLippedU {
        d: f64,
        bf: f64,
        lip: f64,
        t: f64,
        ri: f64,
    },
    /// Cold-formed Z; `angle` turns the lips from the flange (90 = square).
    ColdZ {
        d: f64,
        bf: f64,
        lip: f64,
        t: f64,
        ri: f64,
        angle: f64,
    },
    /// Cold-formed hat (omega): crown width, brims of length `lip`.
    ColdHat {
        d: f64,
        bf: f64,
        lip: f64,
        t: f64,
        ri: f64,
    },
    RectTube {
        h: f64,
        b: f64,
        t: f64,
        ro: f64,
    },
    RoundTube {
        d: f64,
        t: f64,
    },
    /// Solid rectangle (timber or concrete), optionally chamfered.
    Rect {
        b: f64,
        h: f64,
        chamfer: f64,
    },
    Round {
        d: f64,
    },
    /// Concrete T beam, flange on top.
    ConcreteT {
        h: f64,
        bf: f64,
        hf: f64,
        bw: f64,
    },
    /// Concrete L beam, flange on top to +x.
    ConcreteL {
        h: f64,
        bf: f64,
        hf: f64,
        bw: f64,
    },
    /// An annular sector: the part of a ring between the radii `r_in` (0
    /// for a circular sector) and `r_out` turning by `sweep` degrees,
    /// symmetric about the Y axis with its outer face up, as the piece of a
    /// cylindrical wall a column continues.
    AnnularSector {
        r_in: f64,
        r_out: f64,
        sweep: f64,
    },
    Sketch(ParamSketch),
}

impl ProfileShape {
    /// Largest turn of an annular sector, degrees.
    pub const MAX_SECTOR_SWEEP: f64 = 180.0;

    /// The center of an annular sector's ring in the section's own frame
    /// (the outline is centered on its bounding box): below the origin on
    /// the Y axis. `None` for other shapes and invalid values.
    pub fn sector_center(&self) -> Option<DVec2> {
        let Self::AnnularSector { r_in, r_out, sweep } = *self else { return None };
        self.outline().ok()?;
        // The lowest points are the inner arc's ends (or the center of a
        // circular sector); the highest the middle of the outer arc.
        let half = sweep.to_radians() / 2.0;
        let low = if r_in > 0.0 { r_in * half.cos() } else { 0.0 };
        Some(DVec2::new(0.0, -(low + r_out) / 2.0))
    }
}

/// Most parameters of a sketch.
pub const MAX_SKETCH_PARAMS: usize = 64;

/// A parametric sketch: an outline and openings whose points are
/// expressions over named numeric parameters (meters).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParamSketch {
    pub params: Vec<ComponentParam>,
    pub outline: Vec<ProfilePoint>,
    #[serde(default)]
    pub openings: Vec<Vec<ProfilePoint>>,
}

impl ParamSketch {
    /// A centered `b` by `h` rectangle with parameters `b` and `h`.
    pub fn square(b: f64, h: f64) -> ParamSketch {
        ParamSketch {
            params: vec![
                ComponentParam::number("b", b, ParamKind::Length),
                ComponentParam::number("h", h, ParamKind::Length),
            ],
            outline: vec![
                ProfilePoint::new("-b/2", "-h/2"),
                ProfilePoint::new("b/2", "-h/2"),
                ProfilePoint::new("b/2", "h/2"),
                ProfilePoint::new("-b/2", "h/2"),
            ],
            openings: Vec::new(),
        }
    }

    /// The outline, wound and centered on its bounding box.
    pub fn resolve(&self) -> Result<SectionOutline, ProfileError> {
        let err = |e: profile::ProfileError| ProfileError::Sketch(e.to_string());
        if self.params.len() > MAX_SKETCH_PARAMS {
            return Err(ProfileError::Sketch(format!("more than {MAX_SKETCH_PARAMS} parameters")));
        }
        if self.openings.len() > MAX_OPENINGS {
            return Err(ProfileError::TooBig);
        }
        let total = self.outline.len() + self.openings.iter().map(Vec::len).sum::<usize>();
        if total > MAX_POINTS {
            return Err(ProfileError::TooBig);
        }
        if self.outline.len() < 3 || self.openings.iter().any(|o| o.len() < 3) {
            return Err(ProfileError::Sketch("a loop needs at least 3 points".into()));
        }
        let lookup = |name: &str| {
            self.params
                .iter()
                .find(|p| {
                    p.name == name && !matches!(p.kind, ParamKind::Text | ParamKind::Choice | ParamKind::Material)
                })
                .map(|p| p.value)
        };
        let outer = profile::outline(&self.outline, &lookup).map_err(err)?;
        let mut holes = Vec::with_capacity(self.openings.len());
        for o in &self.openings {
            let h = profile::outline(o, &lookup).map_err(err)?;
            if !h.iter().all(|&p| inside(&outer, p)) {
                return Err(ProfileError::Sketch("an opening lies outside the outline".into()));
            }
            holes.push(h);
        }
        if outer.iter().chain(holes.iter().flatten()).any(|p| !p.is_finite() || p.abs().max_element() > MAX_SIZE) {
            return Err(ProfileError::Sketch("a point is out of range".into()));
        }
        if outer.len() + holes.iter().map(Vec::len).sum::<usize>() > MAX_POINTS {
            return Err(ProfileError::TooBig);
        }
        if let Some(e) = crossing(&outer, &holes) {
            return Err(ProfileError::Sketch(e.into()));
        }
        finish(outer, holes)
    }
}

/// `poly` without points repeating the one before them (closing included).
fn distinct(poly: &[DVec2]) -> Vec<DVec2> {
    let mut out: Vec<DVec2> = Vec::with_capacity(poly.len());
    for &p in poly {
        if out.last().is_none_or(|q| q.distance_squared(p) > 1e-24) {
            out.push(p);
        }
    }
    while out.len() > 1 && out.first().zip(out.last()).is_some_and(|(a, b)| a.distance_squared(*b) <= 1e-24) {
        out.pop();
    }
    out
}

/// Whether segments `a`-`b` and `c`-`d` touch or cross.
fn segments_meet(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    let scale = (b - a).length().max((d - c).length()).max(1e-9);
    let eps = 1e-12 * scale * scale;
    let orient = |p: DVec2, q: DVec2, r: DVec2| {
        let o = (q - p).perp_dot(r - p);
        if o.abs() <= eps { 0.0 } else { o.signum() }
    };
    // `r`, collinear with `p`-`q`, lies within it.
    let within = |p: DVec2, q: DVec2, r: DVec2| {
        let (lo, hi) = (p.min(q) - DVec2::splat(1e-12), p.max(q) + DVec2::splat(1e-12));
        r.cmpge(lo).all() && r.cmple(hi).all()
    };
    let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    (o1 == 0.0 && within(a, b, c))
        || (o2 == 0.0 && within(a, b, d))
        || (o3 == 0.0 && within(c, d, a))
        || (o4 == 0.0 && within(c, d, b))
}

/// Why the loops of a sketch do not make a section: an outline or opening
/// that crosses itself, an opening that crosses or touches the outline, or
/// openings that cross, touch or lie in one another. The caller bounds the
/// points by `MAX_POINTS`, so this is at most `MAX_POINTS`² segment pairs.
fn crossing(outer: &[DVec2], holes: &[Vec<DVec2>]) -> Option<&'static str> {
    let loops: Vec<Vec<DVec2>> = std::iter::once(outer).chain(holes.iter().map(Vec::as_slice)).map(distinct).collect();
    if loops.iter().any(|l| l.len() < 3) {
        return Some("a loop needs at least 3 distinct points");
    }
    let seg = |l: &[DVec2], i: usize| (l[i], l[(i + 1) % l.len()]);
    for (li, l) in loops.iter().enumerate() {
        let n = l.len();
        // The loop against itself, neighbouring segments left out.
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let ((a, b), (c, d)) = (seg(l, i), seg(l, j));
                if segments_meet(a, b, c, d) {
                    return Some(if li == 0 { "the outline crosses itself" } else { "an opening crosses itself" });
                }
            }
        }
        // Against the later loops.
        for m in loops.iter().skip(li + 1) {
            for i in 0..n {
                for j in 0..m.len() {
                    let ((a, b), (c, d)) = (seg(l, i), seg(m, j));
                    if segments_meet(a, b, c, d) {
                        return Some(if li == 0 { "an opening crosses the outline" } else { "two openings overlap" });
                    }
                }
            }
            if li > 0 && (inside(l, m[0]) || inside(m, l[0])) {
                return Some("two openings overlap");
            }
        }
    }
    None
}

/// Even-odd test of a point against a loop.
fn inside(poly: &[DVec2], p: DVec2) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

/// An outline: `outer` runs counter-clockwise, `holes` clockwise.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SectionOutline {
    pub outer: Vec<DVec2>,
    pub holes: Vec<Vec<DVec2>>,
}

impl SectionOutline {
    /// Corners of the bounding box of the outer loop.
    pub fn bbox(&self) -> (DVec2, DVec2) {
        let mut it = self.outer.iter();
        let Some(&first) = it.next() else {
            return (DVec2::ZERO, DVec2::ZERO);
        };
        it.fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)))
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    #[error("{0} must be a number between {MIN_THICKNESS} and {MAX_SIZE} m")]
    Value(&'static str),
    #[error("{0} must be a number between 0 and {MAX_SIZE} m")]
    Range(&'static str),
    #[error("{0}")]
    DoesNotFit(String),
    #[error("sketch: {0}")]
    Sketch(String),
    #[error("the outline has more than {MAX_POINTS} points")]
    TooBig,
}

/// Where a profile comes from in a manufacturer's or standard's table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogRef {
    pub series: String,
    pub designation: String,
}

/// A named profile of the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileDef {
    pub id: ProfileId,
    #[serde(default)]
    pub uid: Uid,
    pub name: String,
    pub material: ProfileMaterial,
    pub shape: ProfileShape,
    #[serde(default)]
    pub catalog: Option<CatalogRef>,
    #[serde(default)]
    pub description: String,
}

impl ProfileDef {
    pub fn outline(&self) -> Result<SectionOutline, ProfileError> {
        self.shape.outline()
    }
}

fn size(name: &'static str, v: f64) -> Result<(), ProfileError> {
    if v.is_finite() && (MIN_THICKNESS..=MAX_SIZE).contains(&v) { Ok(()) } else { Err(ProfileError::Value(name)) }
}

fn radius(name: &'static str, v: f64) -> Result<(), ProfileError> {
    if v.is_finite() && (0.0..=MAX_SIZE).contains(&v) { Ok(()) } else { Err(ProfileError::Range(name)) }
}

fn fit(ok: bool, msg: &str) -> Result<(), ProfileError> {
    if ok { Ok(()) } else { Err(ProfileError::DoesNotFit(msg.to_string())) }
}

fn v2(x: f64, y: f64) -> DVec2 {
    DVec2::new(x, y)
}

/// Points of the arc tangent to the two edges that leave `corner` along the
/// unit directions `a` and `b`, with radius `r`. With `r = 0` (or straight
/// edges) the corner itself.
fn fillet(corner: DVec2, a: DVec2, b: DVec2, r: f64, segs: usize) -> Vec<DVec2> {
    let theta = a.dot(b).clamp(-1.0, 1.0).acos();
    if r <= 0.0 || theta < 1e-6 || theta > PI - 1e-6 {
        return vec![corner];
    }
    let t = r / (theta / 2.0).tan();
    let (p0, p1) = (corner + a * t, corner + b * t);
    let center = corner + (a + b).normalize_or_zero() * (r / (theta / 2.0).sin());
    let (s, e) = (p0 - center, p1 - center);
    let a0 = s.y.atan2(s.x);
    let mut d = e.y.atan2(e.x) - a0;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    (0..=segs)
        .map(|i| {
            let ang = a0 + d * i as f64 / segs as f64;
            center + v2(ang.cos(), ang.sin()) * r
        })
        .collect()
}

/// A polygon whose corners carry a radius: each is replaced by its fillet.
fn rounded(pts: &[(DVec2, f64)], segs: usize) -> Result<Vec<DVec2>, ProfileError> {
    let n = pts.len();
    let bad = || ProfileError::DoesNotFit("the corner radii do not fit the edges of the section".into());
    // Directions to the neighbors and how far each fillet eats into its edges.
    let mut dirs = Vec::with_capacity(n);
    let mut cut = Vec::with_capacity(n);
    for i in 0..n {
        let (v, r) = pts[i];
        let (pa, pb) = (pts[(i + n - 1) % n].0 - v, pts[(i + 1) % n].0 - v);
        let (la, lb) = (pa.length(), pb.length());
        if la < 1e-12 || lb < 1e-12 {
            return Err(bad());
        }
        let (a, b) = (pa / la, pb / lb);
        let theta = a.dot(b).clamp(-1.0, 1.0).acos();
        let t = if r > 0.0 && theta > 1e-6 && theta < PI - 1e-6 { r / (theta / 2.0).tan() } else { 0.0 };
        dirs.push((a, b));
        cut.push(t);
    }
    for i in 0..n {
        let len = (pts[(i + 1) % n].0 - pts[i].0).length();
        if cut[i] + cut[(i + 1) % n] > len + 1e-9 {
            return Err(bad());
        }
    }
    let mut out = Vec::new();
    for (i, (a, b)) in dirs.into_iter().enumerate() {
        out.extend(fillet(pts[i].0, a, b, pts[i].1, segs));
    }
    Ok(out)
}

fn signed_area(p: &[DVec2]) -> f64 {
    0.5 * (0..p.len()).map(|i| p[i].perp_dot(p[(i + 1) % p.len()])).sum::<f64>()
}

/// A closed strip along an open centerline, `t` thick. Bends turn on the
/// inner radius `ri` and the outer `ri + t` (square corners when `ri` is 0).
fn strip(center: &[DVec2], t: f64, ri: f64, segs: usize) -> Result<Vec<DVec2>, ProfileError> {
    let n = center.len();
    if n < 2 {
        return Err(ProfileError::DoesNotFit("the strip needs at least two points".into()));
    }
    let miss = || ProfileError::DoesNotFit("the strip folds back on itself".into());
    let dir = |i: usize| (center[i + 1] - center[i]).normalize_or_zero();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    let mut rl = Vec::with_capacity(n);
    let mut rr = Vec::with_capacity(n);
    let outer = if ri > 0.0 { ri + t } else { 0.0 };
    for (i, &p) in center.iter().enumerate() {
        let h = t / 2.0;
        if i == 0 || i == n - 1 {
            let d = if i == 0 { dir(0) } else { dir(n - 2) };
            let nl = d.perp();
            left.push(p + nl * h);
            right.push(p - nl * h);
            rl.push(0.0);
            rr.push(0.0);
        } else {
            let (d0, d1) = (dir(i - 1), dir(i));
            let (n0, n1) = (d0.perp(), d1.perp());
            let k = 1.0 + n0.dot(n1);
            if k < 1e-9 {
                return Err(miss());
            }
            let m = (n0 + n1) / k * h;
            left.push(p + m);
            right.push(p - m);
            let cross = d0.perp_dot(d1);
            if cross.abs() < 1e-12 {
                rl.push(0.0);
                rr.push(0.0);
            } else if cross > 0.0 {
                rl.push(ri);
                rr.push(outer);
            } else {
                rl.push(outer);
                rr.push(ri);
            }
        }
    }
    let mut poly: Vec<(DVec2, f64)> = left.into_iter().zip(rl).collect();
    poly.extend(right.into_iter().zip(rr).rev());
    rounded(&poly, segs)
}

fn rect_loop(w: f64, h: f64, r: f64, segs: usize) -> Result<Vec<DVec2>, ProfileError> {
    let (x, y) = (w / 2.0, h / 2.0);
    rounded(&[(v2(-x, -y), r), (v2(x, -y), r), (v2(x, y), r), (v2(-x, y), r)], segs)
}

fn circle(d: f64, segs: usize) -> Vec<DVec2> {
    let n = 4 * segs;
    (0..n)
        .map(|i| {
            let a = TAU * i as f64 / n as f64;
            v2(a.cos(), a.sin()) * (d / 2.0)
        })
        .collect()
}

/// Winds the loops (outer counter-clockwise, holes clockwise), centers them on
/// the bounding box of the outer one and checks the sizes.
fn finish(mut outer: Vec<DVec2>, mut holes: Vec<Vec<DVec2>>) -> Result<SectionOutline, ProfileError> {
    if holes.len() > MAX_OPENINGS || outer.len() + holes.iter().map(Vec::len).sum::<usize>() > MAX_POINTS {
        return Err(ProfileError::TooBig);
    }
    if signed_area(&outer) < 0.0 {
        outer.reverse();
    }
    for h in &mut holes {
        if signed_area(h) > 0.0 {
            h.reverse();
        }
    }
    let mut o = SectionOutline { outer, holes };
    let (lo, hi) = o.bbox();
    let c = (lo + hi) / 2.0;
    for p in o.outer.iter_mut().chain(o.holes.iter_mut().flatten()) {
        *p -= c;
    }
    Ok(o)
}

impl ProfileShape {
    /// Codes of the shapes, in the order a picker lists them.
    pub const CODES: &'static [&'static str] = &[
        "i",
        "i_asym",
        "channel",
        "t",
        "angle",
        "cold_u",
        "cold_lipped_u",
        "cold_z",
        "cold_hat",
        "rect_tube",
        "round_tube",
        "rect",
        "round",
        "concrete_t",
        "concrete_l",
        "annular_sector",
        "sketch",
    ];

    pub fn code(&self) -> &'static str {
        match self {
            Self::I { .. } => "i",
            Self::IAsym { .. } => "i_asym",
            Self::Channel { .. } => "channel",
            Self::T { .. } => "t",
            Self::Angle { .. } => "angle",
            Self::ColdU { .. } => "cold_u",
            Self::ColdLippedU { .. } => "cold_lipped_u",
            Self::ColdZ { .. } => "cold_z",
            Self::ColdHat { .. } => "cold_hat",
            Self::RectTube { .. } => "rect_tube",
            Self::RoundTube { .. } => "round_tube",
            Self::Rect { .. } => "rect",
            Self::Round { .. } => "round",
            Self::ConcreteT { .. } => "concrete_t",
            Self::ConcreteL { .. } => "concrete_l",
            Self::AnnularSector { .. } => "annular_sector",
            Self::Sketch(_) => "sketch",
        }
    }

    /// A shape with sensible values for `code`.
    pub fn default_for(code: &str) -> Option<ProfileShape> {
        Some(match code {
            "i" => Self::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.012 },
            "i_asym" => Self::IAsym { d: 0.3, bf_top: 0.15, tf_top: 0.012, bf_bot: 0.1, tf_bot: 0.01, tw: 0.007 },
            "channel" => Self::Channel { d: 0.1, bf: 0.05, tw: 0.006, tf: 0.0085, r: 0.0085 },
            "t" => Self::T { d: 0.1, bf: 0.1, tw: 0.0068, tf: 0.0103, r: 0.01 },
            "angle" => Self::Angle { h: 0.1, b: 0.1, t: 0.01, r: 0.012 },
            "cold_u" => Self::ColdU { d: 0.1, bf: 0.05, t: 0.002, ri: 0.003 },
            "cold_lipped_u" => Self::ColdLippedU { d: 0.09, bf: 0.04, lip: 0.012, t: 0.00095, ri: 0.002 },
            "cold_z" => Self::ColdZ { d: 0.2, bf: 0.075, lip: 0.02, t: 0.002, ri: 0.003, angle: 90.0 },
            "cold_hat" => Self::ColdHat { d: 0.04, bf: 0.04, lip: 0.02, t: 0.001, ri: 0.002 },
            "rect_tube" => Self::RectTube { h: 0.1, b: 0.05, t: 0.004, ro: 0.006 },
            "round_tube" => Self::RoundTube { d: 0.1, t: 0.005 },
            "rect" => Self::Rect { b: 0.15, h: 0.3, chamfer: 0.0 },
            "round" => Self::Round { d: 0.3 },
            "concrete_t" => Self::ConcreteT { h: 0.5, bf: 0.6, hf: 0.12, bw: 0.2 },
            "concrete_l" => Self::ConcreteL { h: 0.5, bf: 0.4, hf: 0.12, bw: 0.2 },
            "annular_sector" => Self::AnnularSector { r_in: 2.0, r_out: 2.2, sweep: 15.0 },
            "sketch" => Self::Sketch(ParamSketch::square(0.1, 0.1)),
            _ => return None,
        })
    }

    fn fields_mut(&mut self) -> Vec<(&'static str, &mut f64)> {
        match self {
            Self::I { d, bf, tw, tf, r } | Self::Channel { d, bf, tw, tf, r } | Self::T { d, bf, tw, tf, r } => {
                vec![("d", d), ("bf", bf), ("tw", tw), ("tf", tf), ("r", r)]
            }
            Self::IAsym { d, bf_top, tf_top, bf_bot, tf_bot, tw } => vec![
                ("d", d),
                ("bf_top", bf_top),
                ("tf_top", tf_top),
                ("bf_bot", bf_bot),
                ("tf_bot", tf_bot),
                ("tw", tw),
            ],
            Self::Angle { h, b, t, r } => vec![("h", h), ("b", b), ("t", t), ("r", r)],
            Self::ColdU { d, bf, t, ri } => vec![("d", d), ("bf", bf), ("t", t), ("ri", ri)],
            Self::ColdLippedU { d, bf, lip, t, ri } | Self::ColdHat { d, bf, lip, t, ri } => {
                vec![("d", d), ("bf", bf), ("lip", lip), ("t", t), ("ri", ri)]
            }
            Self::ColdZ { d, bf, lip, t, ri, angle } => {
                vec![("d", d), ("bf", bf), ("lip", lip), ("t", t), ("ri", ri), ("angle", angle)]
            }
            Self::RectTube { h, b, t, ro } => vec![("h", h), ("b", b), ("t", t), ("ro", ro)],
            Self::RoundTube { d, t } => vec![("d", d), ("t", t)],
            Self::Rect { b, h, chamfer } => vec![("b", b), ("h", h), ("chamfer", chamfer)],
            Self::Round { d } => vec![("d", d)],
            Self::ConcreteT { h, bf, hf, bw } | Self::ConcreteL { h, bf, hf, bw } => {
                vec![("h", h), ("bf", bf), ("hf", hf), ("bw", bw)]
            }
            Self::AnnularSector { r_in, r_out, sweep } => vec![("r_in", r_in), ("r_out", r_out), ("sweep", sweep)],
            Self::Sketch(_) => Vec::new(),
        }
    }

    /// The named values of the shape, in declaration order.
    pub fn params(&self) -> Vec<(String, f64)> {
        if let Self::Sketch(s) = self {
            return s.params.iter().map(|p| (p.name.clone(), p.value)).collect();
        }
        self.clone().fields_mut().into_iter().map(|(n, v)| (n.to_string(), *v)).collect()
    }

    /// Sets a value by name; false when the shape has none of that name. The
    /// value is checked when the outline is built.
    pub fn set_param(&mut self, name: &str, v: f64) -> bool {
        if let Self::Sketch(s) = self {
            return match s.params.iter_mut().find(|p| p.name == name) {
                Some(p) => {
                    p.value = v;
                    true
                }
                None => false,
            };
        }
        match self.fields_mut().into_iter().find(|(n, _)| *n == name) {
            Some((_, slot)) => {
                *slot = v;
                true
            }
            None => false,
        }
    }

    /// The outline, centered on its bounding box.
    pub fn outline(&self) -> Result<SectionOutline, ProfileError> {
        self.outline_at(ARC_SEGMENTS)
    }

    /// The section's area, exact for its arcs: the areas of outlines with
    /// arcs of [`ARC_SEGMENTS`] and twice as many chords, extrapolated (the
    /// chords' error falls with the square of their number), so a circle or a
    /// root radius is not counted short. Sketches give their outline's area.
    pub fn area(&self) -> Result<f64, ProfileError> {
        // A piece of a ring has its own formula.
        if let Self::AnnularSector { r_in, r_out, sweep } = *self {
            self.outline()?;
            return Ok(sweep.to_radians() / 2.0 * (r_out * r_out - r_in * r_in));
        }
        let area = |segs| -> Result<f64, ProfileError> {
            let o = self.outline_at(segs)?;
            Ok(signed_area(&o.outer).abs() - o.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>())
        };
        let (coarse, fine) = (area(ARC_SEGMENTS)?, area(2 * ARC_SEGMENTS)?);
        Ok((4.0 * fine - coarse) / 3.0)
    }

    /// [`Self::outline`] with arcs of `segs` chords per quarter turn.
    fn outline_at(&self, segs: usize) -> Result<SectionOutline, ProfileError> {
        match *self {
            Self::I { d, bf, tw, tf, r } => {
                size("d", d)?;
                size("bf", bf)?;
                size("tw", tw)?;
                size("tf", tf)?;
                radius("r", r)?;
                fit(2.0 * tf < d, "the flanges (tf) must be thinner than half the depth (d)")?;
                fit(tw < bf, "the web (tw) must be thinner than the flange width (bf)")?;
                fit(r <= (bf - tw) / 2.0 && r <= (d - 2.0 * tf) / 2.0, "the root radius (r) does not fit")?;
                Self::i_loop(d, (bf, tf), (bf, tf), tw, r, segs)
            }
            Self::IAsym { d, bf_top, tf_top, bf_bot, tf_bot, tw } => {
                size("d", d)?;
                size("bf_top", bf_top)?;
                size("tf_top", tf_top)?;
                size("bf_bot", bf_bot)?;
                size("tf_bot", tf_bot)?;
                size("tw", tw)?;
                fit(tf_top + tf_bot < d, "the flanges (tf_top, tf_bot) must leave room for the web")?;
                fit(tw < bf_top.min(bf_bot), "the web (tw) must be thinner than the flanges are wide")?;
                Self::i_loop(d, (bf_top, tf_top), (bf_bot, tf_bot), tw, 0.0, segs)
            }
            Self::Channel { d, bf, tw, tf, r } => {
                size("d", d)?;
                size("bf", bf)?;
                size("tw", tw)?;
                size("tf", tf)?;
                radius("r", r)?;
                fit(2.0 * tf < d, "the flanges (tf) must be thinner than half the depth (d)")?;
                fit(tw < bf, "the web (tw) must be thinner than the flange width (bf)")?;
                fit(r <= bf - tw && r <= (d - 2.0 * tf) / 2.0, "the root radius (r) does not fit")?;
                let hd = d / 2.0;
                let pts = [
                    (v2(0.0, -hd), 0.0),
                    (v2(bf, -hd), 0.0),
                    (v2(bf, -hd + tf), 0.0),
                    (v2(tw, -hd + tf), r),
                    (v2(tw, hd - tf), r),
                    (v2(bf, hd - tf), 0.0),
                    (v2(bf, hd), 0.0),
                    (v2(0.0, hd), 0.0),
                ];
                finish(rounded(&pts, segs)?, vec![])
            }
            Self::T { d, bf, tw, tf, r } => {
                size("d", d)?;
                size("bf", bf)?;
                size("tw", tw)?;
                size("tf", tf)?;
                radius("r", r)?;
                fit(tf < d, "the flange (tf) must be thinner than the depth (d)")?;
                fit(tw < bf, "the web (tw) must be thinner than the flange width (bf)")?;
                fit(r <= (bf - tw) / 2.0 && r <= d - tf, "the root radius (r) does not fit")?;
                let (hd, hw, hb) = (d / 2.0, bf / 2.0, tw / 2.0);
                let pts = [
                    (v2(-hb, -hd), 0.0),
                    (v2(hb, -hd), 0.0),
                    (v2(hb, hd - tf), r),
                    (v2(hw, hd - tf), 0.0),
                    (v2(hw, hd), 0.0),
                    (v2(-hw, hd), 0.0),
                    (v2(-hw, hd - tf), 0.0),
                    (v2(-hb, hd - tf), r),
                ];
                finish(rounded(&pts, segs)?, vec![])
            }
            Self::Angle { h, b, t, r } => {
                size("h", h)?;
                size("b", b)?;
                size("t", t)?;
                radius("r", r)?;
                fit(t < h && t < b, "the thickness (t) must be less than both legs (h, b)")?;
                fit(r <= (b - t).min(h - t), "the root radius (r) does not fit")?;
                let pts = [
                    (v2(0.0, 0.0), 0.0),
                    (v2(b, 0.0), 0.0),
                    (v2(b, t), 0.0),
                    (v2(t, t), r),
                    (v2(t, h), 0.0),
                    (v2(0.0, h), 0.0),
                ];
                finish(rounded(&pts, segs)?, vec![])
            }
            Self::ColdU { d, bf, t, ri } => {
                Self::cold_check(d, bf, t, ri)?;
                let yt = d / 2.0 - t / 2.0;
                let c = [v2(bf, yt), v2(t / 2.0, yt), v2(t / 2.0, -yt), v2(bf, -yt)];
                finish(strip(&c, t, ri, segs)?, vec![])
            }
            Self::ColdLippedU { d, bf, lip, t, ri } => {
                Self::cold_check(d, bf, t, ri)?;
                size("lip", lip)?;
                fit(lip > t, "the lip (lip) must be longer than the thickness (t)")?;
                fit(2.0 * lip < d - t, "the lips (lip) must not reach each other across the depth (d)")?;
                let (yt, xf) = (d / 2.0 - t / 2.0, bf - t / 2.0);
                let c = [
                    v2(xf, d / 2.0 - lip),
                    v2(xf, yt),
                    v2(t / 2.0, yt),
                    v2(t / 2.0, -yt),
                    v2(xf, -yt),
                    v2(xf, lip - d / 2.0),
                ];
                finish(strip(&c, t, ri, segs)?, vec![])
            }
            Self::ColdZ { d, bf, lip, t, ri, angle } => {
                Self::cold_check(d, bf, t, ri)?;
                size("lip", lip)?;
                fit(lip > t, "the lip (lip) must be longer than the thickness (t)")?;
                fit(
                    angle.is_finite() && (0.0..=90.0).contains(&angle),
                    "the lip angle (angle) must be between 0 and 90 degrees",
                )?;
                fit(lip * angle.to_radians().sin() < d - t, "the lips (lip) must not pass the opposite flange")?;
                let (yt, xf, l) = (d / 2.0 - t / 2.0, bf - t / 2.0, lip - t / 2.0);
                let dir = v2(angle.to_radians().cos(), -angle.to_radians().sin());
                let c =
                    [v2(-xf, -yt) - dir * l, v2(-xf, -yt), v2(0.0, -yt), v2(0.0, yt), v2(xf, yt), v2(xf, yt) + dir * l];
                finish(strip(&c, t, ri, segs)?, vec![])
            }
            Self::ColdHat { d, bf, lip, t, ri } => {
                Self::cold_check(d, bf, t, ri)?;
                size("lip", lip)?;
                fit(lip > t, "the brim (lip) must be longer than the thickness (t)")?;
                let (yt, xc, xb) = (d / 2.0 - t / 2.0, bf / 2.0 - t / 2.0, bf / 2.0 + lip);
                let c = [v2(-xb, -yt), v2(-xc, -yt), v2(-xc, yt), v2(xc, yt), v2(xc, -yt), v2(xb, -yt)];
                finish(strip(&c, t, ri, segs)?, vec![])
            }
            Self::RectTube { h, b, t, ro } => {
                size("h", h)?;
                size("b", b)?;
                size("t", t)?;
                radius("ro", ro)?;
                fit(2.0 * t < h.min(b), "the wall (t) must be thinner than half the sides (h, b)")?;
                let outer = rect_loop(b, h, ro, segs)?;
                let hole = rect_loop(b - 2.0 * t, h - 2.0 * t, (ro - t).max(0.0), segs)?;
                finish(outer, vec![hole])
            }
            Self::RoundTube { d, t } => {
                size("d", d)?;
                size("t", t)?;
                fit(2.0 * t < d, "the wall (t) must be thinner than the radius")?;
                finish(circle(d, segs), vec![circle(d - 2.0 * t, segs)])
            }
            Self::Rect { b, h, chamfer } => {
                size("b", b)?;
                size("h", h)?;
                radius("chamfer", chamfer)?;
                fit(2.0 * chamfer < b.min(h), "the chamfer must be smaller than half the sides")?;
                if chamfer == 0.0 {
                    return finish(rect_loop(b, h, 0.0, segs)?, vec![]);
                }
                let (x, y, c) = (b / 2.0, h / 2.0, chamfer);
                finish(
                    vec![
                        v2(-x + c, -y),
                        v2(x - c, -y),
                        v2(x, -y + c),
                        v2(x, y - c),
                        v2(x - c, y),
                        v2(-x + c, y),
                        v2(-x, y - c),
                        v2(-x, -y + c),
                    ],
                    vec![],
                )
            }
            Self::Round { d } => {
                size("d", d)?;
                finish(circle(d, segs), vec![])
            }
            Self::ConcreteT { h, bf, hf, bw } => {
                Self::concrete_check(h, bf, hf, bw)?;
                let (y, f, w) = (h / 2.0, bf / 2.0, bw / 2.0);
                let pts = [
                    v2(-w, -y),
                    v2(w, -y),
                    v2(w, y - hf),
                    v2(f, y - hf),
                    v2(f, y),
                    v2(-f, y),
                    v2(-f, y - hf),
                    v2(-w, y - hf),
                ];
                finish(pts.to_vec(), vec![])
            }
            Self::ConcreteL { h, bf, hf, bw } => {
                Self::concrete_check(h, bf, hf, bw)?;
                let y = h / 2.0;
                let pts = [v2(0.0, -y), v2(bw, -y), v2(bw, y - hf), v2(bf, y - hf), v2(bf, y), v2(0.0, y)];
                finish(pts.to_vec(), vec![])
            }
            Self::AnnularSector { r_in, r_out, sweep } => {
                radius("r_in", r_in)?;
                size("r_out", r_out)?;
                fit(
                    r_out - r_in >= MIN_THICKNESS,
                    "the outer radius (r_out) must be larger than the inner one (r_in)",
                )?;
                if !(sweep.is_finite() && sweep > 0.0 && sweep <= Self::MAX_SECTOR_SWEEP) {
                    return Err(ProfileError::Range("sweep"));
                }
                let half = sweep.to_radians() / 2.0;
                fit(2.0 * r_out * half.sin() >= MIN_THICKNESS, "the sweep is too small for the radius")?;
                // Chords as fine as a circle's, `segs` to the quarter turn, and no
                // wider than twice the step arcs are drawn with (the radii are
                // those of a ring, meters across); an even number, so the middle
                // of the outer arc is a point.
                let fine = (sweep.to_radians() / (4.0 * crate::arc::ARC_STEP)).ceil();
                let n = 2 * ((sweep / 180.0) * segs as f64).ceil().max(fine).max(1.0) as usize;
                let at = |r: f64, i: usize| {
                    let a = FRAC_PI_2 - half + 2.0 * half * i as f64 / n as f64;
                    v2(a.cos(), a.sin()) * r
                };
                let mut pts: Vec<DVec2> = (0..=n).map(|i| at(r_out, i)).collect();
                if r_in > 0.0 {
                    pts.extend((0..=n).rev().map(|i| at(r_in, i)));
                } else {
                    pts.push(DVec2::ZERO);
                }
                finish(pts, vec![])
            }
            Self::Sketch(ref s) => s.resolve(),
        }
    }

    /// An I (or H) with the given flanges and web fillets of radius `r`.
    fn i_loop(
        d: f64,
        (bt, tt): (f64, f64),
        (bb, tb): (f64, f64),
        tw: f64,
        r: f64,
        segs: usize,
    ) -> Result<SectionOutline, ProfileError> {
        let (hd, hb, wt, wb) = (d / 2.0, tw / 2.0, bt / 2.0, bb / 2.0);
        let pts = [
            (v2(-wb, -hd), 0.0),
            (v2(wb, -hd), 0.0),
            (v2(wb, -hd + tb), 0.0),
            (v2(hb, -hd + tb), r),
            (v2(hb, hd - tt), r),
            (v2(wt, hd - tt), 0.0),
            (v2(wt, hd), 0.0),
            (v2(-wt, hd), 0.0),
            (v2(-wt, hd - tt), 0.0),
            (v2(-hb, hd - tt), r),
            (v2(-hb, -hd + tb), r),
            (v2(-wb, -hd + tb), 0.0),
        ];
        finish(rounded(&pts, segs)?, vec![])
    }

    fn cold_check(d: f64, bf: f64, t: f64, ri: f64) -> Result<(), ProfileError> {
        size("d", d)?;
        size("bf", bf)?;
        size("t", t)?;
        radius("ri", ri)?;
        fit(2.0 * t < d, "the thickness (t) must be less than half the depth (d)")?;
        fit(2.0 * t < bf, "the thickness (t) must be less than half the flange width (bf)")
    }

    fn concrete_check(h: f64, bf: f64, hf: f64, bw: f64) -> Result<(), ProfileError> {
        size("h", h)?;
        size("bf", bf)?;
        size("hf", hf)?;
        size("bw", bw)?;
        fit(hf < h, "the flange (hf) must be thinner than the depth (h)")?;
        fit(bw < bf, "the web (bw) must be narrower than the flange (bf)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(o: &SectionOutline) -> f64 {
        let a = |p: &[DVec2]| 0.5 * (0..p.len()).map(|i| p[i].perp_dot(p[(i + 1) % p.len()])).sum::<f64>();
        a(&o.outer) + o.holes.iter().map(|h| a(h)).sum::<f64>()
    }

    #[test]
    fn rect_and_round_areas() {
        let r = ProfileShape::Rect { b: 0.2, h: 0.4, chamfer: 0.0 }.outline().unwrap();
        assert!((area(&r) - 0.08).abs() < 1e-12);
        let c = ProfileShape::Round { d: 0.3 }.outline().unwrap();
        let exact = std::f64::consts::PI * 0.15 * 0.15;
        assert!((area(&c) - exact).abs() / exact < 0.01);
    }

    #[test]
    fn i_without_root_radius_is_exact() {
        // IPE-like: d 200, bf 100, tw 5.6, tf 8.5, r 0
        let o = ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.0 }.outline().unwrap();
        let exact = 2.0 * 0.1 * 0.0085 + (0.2 - 2.0 * 0.0085) * 0.0056;
        assert!((area(&o) - exact).abs() < 1e-12);
        let (lo, hi) = o.bbox();
        assert!((hi - lo - DVec2::new(0.1, 0.2)).length() < 1e-12);
    }

    #[test]
    fn area_counts_arcs_whole() {
        let (d, bf, tw, tf, r) = (0.2, 0.1, 0.0056, 0.0085, 0.012);
        let i = ProfileShape::I { d, bf, tw, tf, r }.area().unwrap();
        let exact = 2.0 * bf * tf + (d - 2.0 * tf) * tw + 4.0 * r * r * (1.0 - PI / 4.0);
        assert!((i - exact).abs() / exact < 1e-6, "{i} vs {exact}");
        // The chorded outline cuts the fillets' arcs short.
        assert!(area(&ProfileShape::I { d, bf, tw, tf, r }.outline().unwrap()) > exact + 1e-7);
        let round = ProfileShape::Round { d: 0.3 }.area().unwrap();
        assert!((round - PI * 0.0225).abs() / round < 1e-5, "{round}");
        let tube = ProfileShape::RoundTube { d: 0.2, t: 0.008 }.area().unwrap();
        let exact = PI / 4.0 * (0.04 - 0.184 * 0.184);
        assert!((tube - exact).abs() / exact < 1e-5, "{tube} vs {exact}");
        let rect = ProfileShape::Rect { b: 0.2, h: 0.4, chamfer: 0.0 }.area().unwrap();
        assert!((rect - 0.08).abs() < 1e-15);
    }

    #[test]
    fn root_radius_adds_four_fillets() {
        let (d, bf, tw, tf, r) = (0.2, 0.1, 0.0056, 0.0085, 0.012);
        let o = ProfileShape::I { d, bf, tw, tf, r }.outline().unwrap();
        let plain = 2.0 * bf * tf + (d - 2.0 * tf) * tw;
        let fillets = 4.0 * (1.0 - std::f64::consts::FRAC_PI_4) * r * r;
        assert!((area(&o) - (plain + fillets)).abs() / (plain + fillets) < 0.002);
    }

    #[test]
    fn tubes_have_holes() {
        let o = ProfileShape::RectTube { h: 0.1, b: 0.05, t: 0.004, ro: 0.0 }.outline().unwrap();
        assert_eq!(o.holes.len(), 1);
        assert!((area(&o) - (0.1 * 0.05 - 0.092 * 0.042)).abs() < 1e-12);
        let c = ProfileShape::RoundTube { d: 0.1, t: 0.005 }.outline().unwrap();
        assert_eq!(c.holes.len(), 1);
    }

    #[test]
    fn lipped_channel_area_matches_centerline_formula() {
        // Ue 90x40x12x0.95 with ri 0: area = t * (d + 2 bf + 2 lip - 4 t) for square corners
        let (d, bf, lip, t) = (0.09, 0.04, 0.012, 0.00095);
        let o = ProfileShape::ColdLippedU { d, bf, lip, t, ri: 0.0 }.outline().unwrap();
        let exact = t * (d + 2.0 * bf + 2.0 * lip - 4.0 * t);
        assert!((area(&o) - exact).abs() / exact < 1e-9);
    }

    #[test]
    fn every_default_shape_resolves() {
        for code in ProfileShape::CODES {
            let s = ProfileShape::default_for(code).unwrap();
            assert!(s.outline().is_ok(), "{code}");
            assert_eq!(s.code(), *code);
        }
    }

    #[test]
    fn bad_values_are_errors_not_panics() {
        assert!(ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.2, tf: 0.0085, r: 0.0 }.outline().is_err());
        assert!(ProfileShape::Rect { b: f64::NAN, h: 0.4, chamfer: 0.0 }.outline().is_err());
        assert!(ProfileShape::RoundTube { d: 0.1, t: 0.06 }.outline().is_err());
        assert!(ProfileShape::Rect { b: 50.0, h: 0.4, chamfer: 0.0 }.outline().is_err());
    }

    #[test]
    fn outlines_are_wound_and_centered() {
        let signed = |p: &[DVec2]| 0.5 * (0..p.len()).map(|i| p[i].perp_dot(p[(i + 1) % p.len()])).sum::<f64>();
        for code in ProfileShape::CODES.iter() {
            let o = ProfileShape::default_for(code).unwrap().outline().unwrap();
            assert!(signed(&o.outer) > 0.0, "{code} outer");
            assert!(o.holes.iter().all(|h| signed(h) < 0.0), "{code} holes");
            let (lo, hi) = o.bbox();
            assert!((lo + hi).length() < 1e-12, "{code} centered");
        }
    }

    fn close(o: &SectionOutline, exact: f64) {
        assert!((area(o) - exact).abs() / exact < 1e-9, "{} vs {exact}", area(o));
    }

    fn centroid(o: &SectionOutline) -> DVec2 {
        let p = &o.outer;
        let mut c = DVec2::ZERO;
        let mut a = 0.0;
        for i in 0..p.len() {
            let (u, v) = (p[i], p[(i + 1) % p.len()]);
            let w = u.perp_dot(v);
            a += w;
            c += (u + v) * w;
        }
        c / (3.0 * a)
    }

    #[test]
    fn square_corner_areas_match_analytic_values() {
        let o = ProfileShape::Channel { d: 0.1, bf: 0.05, tw: 0.006, tf: 0.0085, r: 0.0 }.outline().unwrap();
        close(&o, 2.0 * 0.05 * 0.0085 + (0.1 - 2.0 * 0.0085) * 0.006);
        let o = ProfileShape::Angle { h: 0.1, b: 0.08, t: 0.01, r: 0.0 }.outline().unwrap();
        close(&o, 0.01 * (0.1 + 0.08 - 0.01));
        let o = ProfileShape::T { d: 0.1, bf: 0.1, tw: 0.007, tf: 0.01, r: 0.0 }.outline().unwrap();
        close(&o, 0.1 * 0.01 + (0.1 - 0.01) * 0.007);
        let o = ProfileShape::IAsym { d: 0.3, bf_top: 0.15, tf_top: 0.012, bf_bot: 0.1, tf_bot: 0.01, tw: 0.007 }
            .outline()
            .unwrap();
        close(&o, 0.15 * 0.012 + 0.1 * 0.01 + 0.007 * (0.3 - 0.022));
        let (d, bf, lip, t) = (0.2, 0.075, 0.02, 0.002);
        let o = ProfileShape::ColdU { d, bf, t, ri: 0.0 }.outline().unwrap();
        close(&o, t * (d + 2.0 * bf - 2.0 * t));
        let o = ProfileShape::ColdZ { d, bf, lip, t, ri: 0.0, angle: 90.0 }.outline().unwrap();
        close(&o, t * (d + 2.0 * bf + 2.0 * lip - 3.0 * t));
        let o = ProfileShape::ColdHat { d: 0.04, bf: 0.04, lip, t, ri: 0.0 }.outline().unwrap();
        close(&o, t * (0.04 + 2.0 * 0.04 + 2.0 * lip - 2.0 * t));
        let o = ProfileShape::ConcreteT { h: 0.5, bf: 0.6, hf: 0.12, bw: 0.2 }.outline().unwrap();
        close(&o, 0.6 * 0.12 + 0.2 * 0.38);
        let o = ProfileShape::ConcreteL { h: 0.5, bf: 0.4, hf: 0.12, bw: 0.2 }.outline().unwrap();
        close(&o, 0.4 * 0.12 + 0.2 * 0.38);
        let o = ProfileShape::Rect { b: 0.2, h: 0.4, chamfer: 0.03 }.outline().unwrap();
        close(&o, 0.08 - 2.0 * 0.03 * 0.03);
    }

    #[test]
    fn annular_sector_is_a_piece_of_a_ring() {
        let shape = ProfileShape::AnnularSector { r_in: 2.0, r_out: 2.2, sweep: 30.0 };
        let o = shape.outline().unwrap();
        // Counter-clockwise, centered on its box, symmetric about the Y axis.
        assert!(signed_area(&o.outer) > 0.0);
        let (lo, hi) = o.bbox();
        assert!((lo + hi).length() < 1e-12);
        assert!((hi.x - 2.2 * 15f64.to_radians().sin()).abs() < 1e-12);
        assert!((hi.y - lo.y - (2.2 - 2.0 * 15f64.to_radians().cos())).abs() < 1e-12);
        // Every point lies on one of the two circles around the ring's center.
        let c = shape.sector_center().unwrap();
        assert!(c.x == 0.0 && (c.y + (2.0 * 15f64.to_radians().cos() + 2.2) / 2.0).abs() < 1e-12);
        assert!(o.outer.iter().all(|p| {
            let r = p.distance(c);
            (r - 2.0).abs() < 1e-9 || (r - 2.2).abs() < 1e-9
        }));
        // The exact area: half the sweep times the difference of the squared radii.
        let exact = 30f64.to_radians() / 2.0 * (2.2f64.powi(2) - 2.0f64.powi(2));
        assert!((shape.area().unwrap() - exact).abs() < 1e-9, "{}", shape.area().unwrap());
        // Without an inner radius it is a circular sector, to its center.
        let pie = ProfileShape::AnnularSector { r_in: 0.0, r_out: 0.5, sweep: 90.0 };
        let o = pie.outline().unwrap();
        assert!(o.outer.iter().any(|p| p.distance(pie.sector_center().unwrap()) < 1e-12));
        assert!((pie.area().unwrap() - FRAC_PI_2 / 2.0 * 0.25).abs() < 1e-9);
        // Out of range values are errors, not shapes.
        for bad in [
            ProfileShape::AnnularSector { r_in: 2.2, r_out: 2.2, sweep: 30.0 },
            ProfileShape::AnnularSector { r_in: 2.0, r_out: 2.2, sweep: 0.0 },
            ProfileShape::AnnularSector { r_in: 2.0, r_out: 2.2, sweep: 200.0 },
            ProfileShape::AnnularSector { r_in: -1.0, r_out: 2.2, sweep: 30.0 },
            ProfileShape::AnnularSector { r_in: 2.0, r_out: f64::NAN, sweep: 30.0 },
        ] {
            assert!(bad.outline().is_err() && bad.sector_center().is_none(), "{bad:?}");
        }
        // Parameters by name, the sweep in degrees.
        let mut s = ProfileShape::default_for("annular_sector").unwrap();
        assert_eq!(s.code(), "annular_sector");
        assert!(s.set_param("sweep", 45.0) && s.set_param("r_in", 1.0) && !s.set_param("angle", 1.0));
        assert_eq!(s, ProfileShape::AnnularSector { r_in: 1.0, r_out: 2.2, sweep: 45.0 });
    }

    #[test]
    fn orientation_follows_steel_tables() {
        // The web of a channel is on -x: the material leans that way.
        let c = ProfileShape::Channel { d: 0.1, bf: 0.05, tw: 0.006, tf: 0.0085, r: 0.0 }.outline().unwrap();
        assert!(centroid(&c).x < 0.0);
        // The angle's vertical leg is at -x, its horizontal leg at -y.
        let a = ProfileShape::Angle { h: 0.1, b: 0.1, t: 0.01, r: 0.0 }.outline().unwrap();
        let has = |p: DVec2| a.outer.iter().any(|q| (*q - p).length() < 1e-12);
        assert!(has(DVec2::new(-0.05, 0.05)) && has(DVec2::new(0.05, -0.05)) && has(DVec2::new(-0.05, -0.05)));
        assert!(!has(DVec2::new(0.05, 0.05)));
        let (cx, cy) = (centroid(&a).x, centroid(&a).y);
        assert!(cx < 0.0 && cy < 0.0);
    }

    #[test]
    fn pieces_that_do_not_fit_are_errors() {
        // Lips that meet or pass each other.
        for lip in [0.045, 0.08] {
            assert!(ProfileShape::ColdLippedU { d: 0.09, bf: 0.04, lip, t: 0.00095, ri: 0.0 }.outline().is_err());
        }
        assert!(
            ProfileShape::ColdZ { d: 0.05, bf: 0.04, lip: 0.06, t: 0.001, ri: 0.0, angle: 90.0 }.outline().is_err()
        );
        assert!(
            ProfileShape::ColdZ { d: 0.05, bf: 0.04, lip: 0.02, t: 0.001, ri: 0.0, angle: 120.0 }.outline().is_err()
        );
        // Root radius too big for the flange or the web.
        assert!(ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.05 }.outline().is_err());
        assert!(ProfileShape::Channel { d: 0.1, bf: 0.05, tw: 0.006, tf: 0.0085, r: 0.06 }.outline().is_err());
        assert!(ProfileShape::Angle { h: 0.1, b: 0.1, t: 0.01, r: 0.1 }.outline().is_err());
        // Bend radius that does not fit the legs.
        assert!(ProfileShape::ColdU { d: 0.1, bf: 0.05, t: 0.002, ri: 0.05 }.outline().is_err());
        assert!(ProfileShape::ColdLippedU { d: 0.09, bf: 0.04, lip: 0.012, t: 0.00095, ri: 0.02 }.outline().is_err());
        // Radii accept zero but not negative values, with their own message.
        let e = ProfileShape::Angle { h: 0.1, b: 0.1, t: 0.01, r: -1.0 }.outline().unwrap_err();
        assert_eq!(e, ProfileError::Range("r"));
        assert!(e.to_string().contains("between 0 and"));
    }

    #[test]
    fn strip_with_too_few_points_is_an_error() {
        assert!(strip(&[], 0.001, 0.0, ARC_SEGMENTS).is_err());
        assert!(strip(&[DVec2::ZERO], 0.001, 0.0, ARC_SEGMENTS).is_err());
    }

    #[test]
    fn set_param_round_trips() {
        let mut s = ProfileShape::default_for("i").unwrap();
        assert!(s.set_param("tw", 0.007));
        assert!(!s.set_param("nope", 1.0));
        assert_eq!(s.params().iter().find(|(n, _)| n == "tw").unwrap().1, 0.007);
    }

    #[test]
    fn sketch_reads_its_parameters() {
        let mut s = ParamSketch::square(0.1, 0.2);
        let o = ProfileShape::Sketch(s.clone()).outline().unwrap();
        let (lo, hi) = o.bbox();
        assert!((hi - lo - DVec2::new(0.1, 0.2)).length() < 1e-12);
        s.params.iter_mut().find(|p| p.name == "b").unwrap().value = 0.3;
        let (lo, hi) = ProfileShape::Sketch(s).outline().unwrap().bbox();
        assert!((hi.x - lo.x - 0.3).abs() < 1e-12);
    }

    #[test]
    fn sketch_with_opening_is_a_hollow_section() {
        let mut s = ParamSketch::square(0.2, 0.2);
        s.params.push(ComponentParam::number("t", 0.01, ParamKind::Length));
        s.openings.push(vec![
            ProfilePoint::new("-b/2+t", "-h/2+t"),
            ProfilePoint::new("b/2-t", "-h/2+t"),
            ProfilePoint::new("b/2-t", "h/2-t"),
            ProfilePoint::new("-b/2+t", "h/2-t"),
        ]);
        let o = ProfileShape::Sketch(s).outline().unwrap();
        assert_eq!(o.holes.len(), 1);
    }

    #[test]
    fn bad_sketch_is_an_error() {
        let mut s = ParamSketch::square(0.1, 0.1);
        s.outline[0].u = "nope + 1".into();
        assert!(matches!(ProfileShape::Sketch(s).outline(), Err(ProfileError::Sketch(_))));
        let mut s = ParamSketch::square(0.1, 0.1);
        s.outline.truncate(2);
        assert!(ProfileShape::Sketch(s).outline().is_err());
    }

    #[test]
    fn sketch_opening_outside_is_an_error_and_params_are_named() {
        let mut s = ParamSketch::square(0.1, 0.1);
        s.openings.push(vec![ProfilePoint::new("1", "1"), ProfilePoint::new("2", "1"), ProfilePoint::new("2", "2")]);
        assert!(ProfileShape::Sketch(s).outline().is_err());
        let mut sh = ProfileShape::default_for("sketch").unwrap();
        assert!(sh.set_param("b", 0.25));
        assert_eq!(sh.params().iter().find(|(n, _)| n == "b").unwrap().1, 0.25);
        assert!(!sh.set_param("zz", 1.0));
    }
}
