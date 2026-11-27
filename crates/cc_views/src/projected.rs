//! Orthographic projected views: elevations, sections and axonometrics,
//! with painter's-algorithm hidden surface removal.
//!
//! Faces are painted back to front. The order comes from comparing the depth
//! of every pair of faces that overlap on screen, at a point inside their
//! overlap; sorting by centroid depth alone fails for large faces such as roofs.

use crate::shadow::{self, Catch, Shadows};
use crate::{Drawing2D, LineStyle, cut::POCHE};
use cc_core::{CameraState, DetailLevel, Direction, ElementId, Project, ShadingSettings, TypeData, VisualStyle};
use cc_geom::polygon::signed_area;
use cc_geom::{ModelGeometry, clip_polygon, face_plane_segments};
use glam::{DVec2, DVec3};

/// Orthographic view frame with optional clip planes.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub origin: DVec3,
    pub right: DVec3,
    pub up: DVec3,
    /// View direction (into the scene).
    pub forward: DVec3,
    /// Keep depth in `near..far` (relative to `origin`).
    pub near: Option<f64>,
    pub far: Option<f64>,
    /// Keep screen x in this range.
    pub crop_x: Option<(f64, f64)>,
}

impl Frame {
    pub fn looking(origin: DVec3, forward: DVec3) -> Self {
        let forward = forward.normalize();
        let right = forward.cross(DVec3::Z).try_normalize().unwrap_or(DVec3::X);
        let up = right.cross(forward);
        Self { origin, right, up, forward, near: None, far: None, crop_x: None }
    }

    pub fn project(&self, p: DVec3) -> DVec2 {
        let r = p - self.origin;
        DVec2::new(r.dot(self.right), r.dot(self.up))
    }

    /// The model point drawn at `p` that lies `depth` along the view
    /// direction (the inverse of [`Frame::project`] and [`Frame::depth`]).
    pub fn unproject(&self, p: DVec2, depth: f64) -> DVec3 {
        self.origin + self.right * p.x + self.up * p.y + self.forward * depth
    }

    pub fn depth(&self, p: DVec3) -> f64 {
        (p - self.origin).dot(self.forward)
    }

    /// Clip planes as (point, normal) keeping the positive side.
    pub(crate) fn planes(&self) -> Vec<(DVec3, DVec3)> {
        let mut planes = Vec::new();
        if let Some(n) = self.near {
            planes.push((self.origin + self.forward * n, self.forward));
        }
        if let Some(f) = self.far {
            planes.push((self.origin + self.forward * f, -self.forward));
        }
        if let Some((x0, x1)) = self.crop_x {
            planes.push((self.origin + self.right * x0, self.right));
            planes.push((self.origin + self.right * x1, -self.right));
        }
        planes
    }
}

pub(crate) fn clip_segment(a: DVec3, b: DVec3, point: DVec3, normal: DVec3) -> Option<(DVec3, DVec3)> {
    let (da, db) = ((a - point).dot(normal), (b - point).dot(normal));
    match (da >= 0.0, db >= 0.0) {
        (true, true) => Some((a, b)),
        (false, false) => None,
        (true, false) => Some((a, a.lerp(b, da / (da - db)))),
        (false, true) => Some((a.lerp(b, da / (da - db)), b)),
    }
}

/// The cosine of the angle between a face's unit `normal` and the light:
/// the default light fixed to the view (from behind the viewer, above and to
/// the left) when `sun` is `None`, else the sun's direction (towards it; the
/// zero vector, a sun below the horizon, lights no face).
fn facing(normal: DVec3, frame: &Frame, sun: Option<DVec3>) -> f64 {
    match sun {
        None => {
            let light = (frame.forward * 0.6 + frame.right * 0.3 - frame.up * 0.5).normalize();
            -normal.dot(light)
        }
        Some(towards) => normal.dot(towards),
    }
}

/// A face's fill in the shaded style: white, greyed by the light (see
/// [`ShadingSettings::grey`]).
fn grey(normal: DVec3, frame: &Frame, sun: Option<DVec3>, shading: &ShadingSettings) -> [u8; 4] {
    let g = (shading.grey(facing(normal, frame, sun)) * 255.0).round().clamp(0.0, 255.0) as u8;
    [g, g, g, 255]
}

/// A face's fill in `color` shaded by the light (the textures style).
fn shade(color: [u8; 3], normal: DVec3, frame: &Frame, sun: Option<DVec3>, shading: &ShadingSettings) -> [u8; 4] {
    let k = shadow::textures_light(shading, facing(normal, frame, sun));
    // Wash colors towards white so linework stays legible.
    let c = |v: u8| ((v as f64 * 0.55 + 255.0 * 0.45) * k).min(255.0) as u8;
    [c(color[0]), c(color[1]), c(color[2]), 255]
}

/// How a view draws the model: its detail level, its visual style, the
/// spacing of patterns (cut and surface hatches) in drawing units, the
/// light that shades its faces and the shadows it casts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look<'a> {
    pub detail: DetailLevel,
    pub style: VisualStyle,
    pub spacing: f64,
    /// The sun lighting the faces (shaded and textures styles): the unit
    /// vector towards it in project coordinates, or the zero vector when it
    /// is below the horizon (ambient light only). `None`: the default light,
    /// fixed to the view.
    pub light: Option<DVec3>,
    /// The shadows the sun casts while it is above the horizon, in the
    /// styles that fill faces with more than white (see `shadow`); `None`: none.
    pub shadows: Option<Shadows<'a>>,
    /// How light faces turned away from the light are and how dark shadows
    /// are ([`cc_core::View::shading`]).
    pub shading: ShadingSettings,
}

impl<'a> Look<'a> {
    /// The look of `view` in `project`: lit by the view's sun when it has
    /// one ([`cc_core::View::sun`]), which casts shadows when it is set to
    /// ([`cc_core::sun::SunSettings::shadows`]): on the model `geom` and on
    /// the ground plane ([`Project::sun_ground_height`]), `skip` being the
    /// elements the view's graphics leave out of them. What is `indoors`
    /// of a floor plan's or a section's faces catches none (without it,
    /// every face catches them; see [`shadow::Catch`]).
    pub fn of(
        project: &Project,
        geom: &ModelGeometry,
        view: &cc_core::View,
        skip: Option<&'a shadow::Skip>,
        indoors: Option<&'a shadow::Indoors>,
    ) -> Self {
        let light = project
            .sun_state(view)
            .filter(|_| view.kind.takes_sun())
            .map(|s| if s.above_horizon && s.direction.is_finite() { s.direction } else { DVec3::ZERO });
        let shadows = Self::casts_shadows(view).then(|| {
            let bottom = geom.bounds().map(|(lo, _)| lo.z);
            let catch = match (&view.kind, indoors) {
                (cc_core::ViewKind::FloorPlan { .. }, Some(indoors)) => Catch::Site(indoors),
                (cc_core::ViewKind::Section { .. } | cc_core::ViewKind::InteriorElevation { .. }, Some(indoors)) => {
                    Catch::Outdoors(indoors)
                }
                _ => Catch::All,
            };
            Shadows { ground: Some(project.sun_ground_height(view, bottom)), skip, catch }
        });
        let shading = if view.kind.takes_sun() { view.shading.clamped() } else { ShadingSettings::default() };
        Self {
            detail: view.detail,
            style: view.style(),
            spacing: crate::cut::spacing(view.scale),
            light,
            shadows,
            shading,
        }
    }

    /// Whether `view` is set to draw its sun's shadows: it has a sun that
    /// casts them, in a visual style that shows them (wherever the sun
    /// stands). Ceiling plans, which look up at what the sun does not
    /// reach, draw none.
    pub fn casts_shadows(view: &cc_core::View) -> bool {
        let looks_up = matches!(view.kind, cc_core::ViewKind::CeilingPlan { .. });
        view.kind.takes_sun() && !looks_up && view.sun.as_ref().is_some_and(|s| s.shadows) && view.style().colored()
    }

    /// Shaded, at the medium detail level, patterns as at 1:100, the default light, no shadows.
    pub fn plain() -> Self {
        Self {
            detail: DetailLevel::Medium,
            style: VisualStyle::Shaded,
            spacing: crate::cut::spacing(100),
            light: None,
            shadows: None,
            shading: ShadingSettings::default(),
        }
    }
}

/// The fill of a face of `solid` in `style` (none for the wireframe).
fn face_color(solid: &cc_geom::Solid, face: usize, normal: DVec3, frame: &Frame, look: Look<'_>) -> Option<[u8; 4]> {
    // A painted face shows its paint.
    let paint = solid.paint_of(face);
    let color = paint.map_or(solid.color, |p| p.color);
    let appearance = paint.map_or(solid.appearance, |p| p.appearance);
    let transparency = paint.map_or(solid.transparency, |p| p.transparency);
    let mut c = match look.style {
        VisualStyle::Wireframe => return None,
        VisualStyle::HiddenLine => [255, 255, 255, 255],
        VisualStyle::Shaded => grey(normal, frame, look.light, &look.shading),
        VisualStyle::ConsistentColors => [color[0], color[1], color[2], 255],
        VisualStyle::Textures => shade(appearance, normal, frame, look.light, &look.shading),
    };
    // Transparent materials let what is behind show through (never fully: alpha 0 means "no hit").
    c[3] = ((1.0 - transparency.clamp(0.0, 1.0)) * 255.0).round().clamp(40.0, 255.0) as u8;
    Some(c)
}

/// The surface pattern of face `face` of `solid` (its paint's, when painted)
/// over its outline `pts` (the textures style), each line with the pattern's color.
fn surface_lines(
    d: &mut Drawing2D,
    patterns: &cc_core::fill_pattern::FillPatterns,
    solid: &cc_geom::Solid,
    face: usize,
    pts: &[DVec2],
    spacing: f64,
) {
    let fill = solid.paint_of(face).map_or(solid.surface, |p| p.surface);
    for (pattern, color) in [(fill.background, fill.background_color), (fill.foreground, fill.foreground_color)] {
        if matches!(pattern, cc_core::Pattern::Solid | cc_core::Pattern::None) {
            continue;
        }
        let pen = crate::Pen { color: Some(color), ..Default::default() };
        for (a, b) in crate::hatch::hatch_in(patterns, pattern, pts, spacing) {
            d.prims.push(crate::Prim::Line {
                a,
                b,
                style: LineStyle::Thin,
                element: Some(solid.element),
                pen: Some(pen),
            });
        }
    }
}

struct ProjectedFace {
    /// Mean depth of the vertices, used to order unrelated faces and break cycles.
    depth: f64,
    /// Plane of the face in 3D.
    normal: DVec3,
    point: DVec3,
    pts: Vec<DVec2>,
    /// Its outline in the model, as `pts` shows it (when shadows are cast).
    verts: Vec<DVec3>,
    color: Option<[u8; 4]>,
    element: ElementId,
    /// Index of its solid in the geometry and of the face in it (its surface pattern).
    solid: usize,
    face: usize,
    edges: Vec<(DVec2, DVec2)>,
    /// Where its curved surface turns away: lines drawn once the faces in
    /// `after` (indices of the faces beside them, see [`beside`]) are painted.
    silhouettes: Vec<(DVec2, DVec2)>,
    after: Vec<usize>,
    /// Symbol lines drawn on the face (a window's opening), in their styles.
    marks: Vec<(DVec2, DVec2, LineStyle)>,
}

/// The silhouettes of a face, held back while faces beside them are unpainted.
struct Late {
    lines: Vec<(DVec2, DVec2)>,
    /// The faces beside them, and how many of those are still to paint.
    after: Vec<usize>,
    left: usize,
}

/// How far from a silhouette (paper millimeters) the facets of its surface
/// are painted before it: about a pen's width, so the facets a line of that
/// width lies over do not cover it.
const SILHOUETTE_REACH_MM: f64 = 0.35;
/// The most faces a silhouette waits for.
const SILHOUETTE_FACES: usize = 64;

/// The faces painted before the silhouettes of face `start` of `faces`: the
/// facets of its curved surface reached across smooth edges (`neighbors`, by
/// face of the solid, `slots` giving each one's place in `faces`) that come
/// within `reach` of a silhouette. Painted after it, a facet thinner than
/// the line's width would cover the line: a rounded edge or a thin bar then
/// lost its outline on screen.
fn beside(
    faces: &[ProjectedFace],
    start: usize,
    slots: &[usize],
    neighbors: &[Vec<(usize, DVec3)>],
    reach: f64,
) -> Vec<usize> {
    let Some(face) = faces.get(start).filter(|f| !f.silhouettes.is_empty()) else { return Vec::new() };
    let off = |p: DVec2, (a, b): (DVec2, DVec2)| cc_geom::polygon::closest_on_segment(a, b, p).0.distance(p);
    let near = |f: &ProjectedFace| f.pts.iter().any(|p| face.silhouettes.iter().any(|s| off(*p, *s) <= reach));
    let mut found = Vec::new();
    let mut queue = vec![start];
    while let Some(i) = queue.pop() {
        let Some(f) = faces.get(i) else { continue };
        for &(n, _) in neighbors.get(f.face).into_iter().flatten() {
            let Some(&j) = slots.get(n) else { continue };
            if j == start || found.contains(&j) || found.len() >= SILHOUETTE_FACES {
                continue;
            }
            if faces.get(j).is_some_and(near) {
                found.push(j);
                queue.push(j);
            }
        }
    }
    found
}

fn symbol_style(line: cc_core::SymbolLine) -> LineStyle {
    match line {
        cc_core::SymbolLine::Opening => LineStyle::Hidden,
        cc_core::SymbolLine::Axis => LineStyle::Center,
        cc_core::SymbolLine::Plain => LineStyle::Thin,
    }
}

/// How section cuts are filled: the project's materials at a hatch spacing.
#[derive(Clone, Copy)]
pub struct CutStyle<'a> {
    pub project: &'a Project,
    pub spacing: f64,
}

/// Render the solids a view shows through `frame`, at its detail level and
/// in its visual style; section cuts use `cut` (else, and at the coarse
/// level, the neutral poche; in the wireframe, only their outline).
pub fn render(
    geom: &ModelGeometry,
    frame: &Frame,
    cut: Option<CutStyle>,
    look: Look<'_>,
    patterns: &cc_core::fill_pattern::FillPatterns,
) -> Drawing2D {
    let (detail, style) = (look.detail, look.style);
    let planes = frame.planes();
    let mut faces = Vec::new();
    let shown = |s: &&cc_geom::Solid| s.detail.shows(detail);
    let cut = cut.filter(|_| detail.shows_layers());
    // The sun's shadows: what casts them (the model beyond the cut plane;
    // in a floor plan the whole model, which reads the site) and, where
    // the view has spaces indoors, what covers them from the sky. Either
    // past a limit: no shadows.
    let sun = look.light.filter(|l| style.colored() && l.length_squared() > 0.5);
    let skip = look.shadows.and_then(|s| s.skip);
    let (site, indoors) = match look.shadows.map_or(Catch::All, |s| s.catch) {
        Catch::All => (false, None),
        Catch::Site(indoors) => (true, Some(indoors)),
        Catch::Outdoors(indoors) => (false, Some(indoors)),
    };
    let seen = |s: &cc_geom::Solid| s.detail.shows(detail);
    let mut cover = None;
    let mut scene = look.shadows.zip(sun).and_then(|(_, sun)| {
        if indoors.is_some_and(|i| i.failed) {
            return None;
        }
        if indoors.is_some_and(|i| !i.is_empty()) {
            cover = Some(shadow::Scene::covers(geom, skip, &seen, None)?);
        }
        let near = frame.near.filter(|_| !site).map(|n| (frame.origin + frame.forward * n, frame.forward));
        shadow::Scene::new(geom, sun, skip, &seen, near)
    });
    for (index, solid) in geom.solids.iter().enumerate().filter(|(_, s)| shown(s)) {
        let edges = solid.feature_edges();
        let seams = solid.smooth_edges();
        let first = faces.len();
        // Where each face of the solid is in `faces` (those the view shows).
        let mut slots = vec![usize::MAX; solid.faces.len()];
        for (face_index, ((face, face_edges), seams)) in solid.faces.iter().zip(edges).zip(seams).enumerate() {
            // The wireframe sees through: faces turned away show their edges too.
            if face.normal.dot(frame.forward) > -1e-6 && style.fills() {
                continue;
            }
            // Where a curved surface turns away, the edge to the hidden facet is its silhouette.
            let turned = seams.into_iter().filter(|s| s.2.dot(frame.forward) > -1e-6).map(|(a, b, _)| (a, b));
            let shown = |edges: Vec<(DVec3, DVec3)>| -> Vec<(DVec2, DVec2)> {
                edges
                    .into_iter()
                    .filter_map(|(a, b)| planes.iter().try_fold((a, b), |(a, b), &(p, n)| clip_segment(a, b, p, n)))
                    .map(|(a, b)| (frame.project(a), frame.project(b)))
                    .collect()
            };
            let mut verts = face.verts.clone();
            for &(p, n) in &planes {
                verts = clip_polygon(&verts, p, n);
            }
            if verts.len() < 3 {
                continue;
            }
            let (edges, silhouettes) = (shown(face_edges), shown(turned.collect()));
            if let Some(slot) = slots.get_mut(face_index) {
                *slot = faces.len();
            }
            let depth = verts.iter().map(|v| frame.depth(*v)).sum::<f64>() / verts.len() as f64;
            faces.push(ProjectedFace {
                depth,
                normal: face.normal,
                point: verts[0],
                pts: verts.iter().map(|v| frame.project(*v)).collect(),
                verts: if scene.is_some() { verts } else { Vec::new() },
                color: face_color(solid, face_index, face.normal, frame, look),
                element: solid.element,
                solid: index,
                face: face_index,
                edges,
                silhouettes,
                after: Vec::new(),
                marks: Vec::new(),
            });
        }
        // The facets beside a silhouette are painted before it (a wireframe paints none).
        if style.fills() && faces.get(first..).is_some_and(|f| f.iter().any(|f| !f.silhouettes.is_empty())) {
            let neighbors = solid.smooth_neighbors();
            let reach = SILHOUETTE_REACH_MM * crate::hatch::paper_unit(look.spacing);
            for i in first..faces.len() {
                let after = beside(&faces, i, &slots, &neighbors, reach);
                if let Some(face) = faces.get_mut(i) {
                    face.after = after;
                }
            }
        }
    }
    // Windows' opening symbols: unfilled faces just off their walls, so
    // whatever stands in front hides them like any face.
    for symbol in &geom.opening_symbols {
        if symbol.normal.dot(frame.forward) > -1e-6 && style.fills() {
            continue;
        }
        let mut verts = symbol.outline.to_vec();
        for &(p, n) in &planes {
            verts = clip_polygon(&verts, p, n);
        }
        if verts.len() < 3 {
            continue;
        }
        let marks = symbol
            .lines
            .iter()
            .filter_map(|&(a, b, line)| {
                let (a, b) = planes.iter().try_fold((a, b), |(a, b), &(p, n)| clip_segment(a, b, p, n))?;
                Some((frame.project(a), frame.project(b), symbol_style(line)))
            })
            .collect();
        faces.push(ProjectedFace {
            depth: verts.iter().map(|v| frame.depth(*v)).sum::<f64>() / verts.len() as f64,
            normal: symbol.normal,
            point: verts[0],
            pts: verts.iter().map(|v| frame.project(*v)).collect(),
            verts: Vec::new(),
            color: None,
            element: symbol.element,
            solid: usize::MAX,
            face: 0,
            edges: Vec::new(),
            silhouettes: Vec::new(),
            after: Vec::new(),
            marks,
        });
    }
    let mut d = Drawing2D::default();
    // The shadows on each face, and on the ground seen from above: all of
    // them, or none when a limit was passed.
    let mut shadows: Vec<Vec<Vec<DVec2>>> = Vec::new();
    if let Some(scene) = scene.as_mut() {
        let ground = look.shadows.and_then(|s| s.ground).filter(|_| frame.forward.z < -1e-3);
        let mut under: Vec<Vec<DVec2>> = Vec::new();
        if let Some((z, bounds)) = ground.zip(geom.bounds()) {
            // A floor plan's ground catches none where it is indoors.
            let inside = cover.as_mut().zip(indoors).filter(|_| site);
            for ring in scene.on_ground(z, bounds, inside) {
                let ring = planes.iter().fold(ring, |r, &(p, n)| clip_polygon(&r, p, n));
                if ring.len() >= 3 {
                    under.push(ring.iter().map(|p| frame.project(*p)).collect());
                }
            }
        }
        shadows = faces
            .iter()
            .map(|f| {
                // Faces that show through, and those the view hides, catch none.
                let catches = f.color.is_some_and(|c| c[3] == 255)
                    && !skip.is_some_and(|k| k.hidden.contains(&f.element) || k.clear.contains(&f.element));
                if !catches {
                    return Vec::new();
                }
                // What is indoors catches none: a section's face whole, by
                // the point in front of it; a floor plan's by its part indoors.
                let mut inside = cover.as_mut().zip(indoors).map(|(c, i)| (c, i, f.element));
                if !site
                    && let Some((cover, indoors, element)) = inside.take()
                    && let Some(p) = shadow::point_within(&f.verts, f.normal)
                    && indoors.holds(p + f.normal * shadow::FACE_PROBE)
                    && cover.above(p + f.normal * shadow::FACE_PROBE, Some(element))
                {
                    return Vec::new();
                }
                let rings = scene.on_face(&f.verts, f.normal, (f.solid, f.face), inside);
                rings.into_iter().map(|r| r.into_iter().map(|p| frame.project(p)).collect()).collect()
            })
            .collect();
        if scene.failed() {
            shadows.clear();
        } else {
            for ring in under {
                d.fill(ring, [0, 0, 0, shadow::ground_veil(&look.shading)], None);
            }
        }
    }
    if style.fills() {
        let order = paint_order(frame, &faces);
        // Silhouettes are held until the faces beside them are painted: a
        // facet thinner than the line would cover it.
        let mut late: Vec<Late> = faces
            .iter_mut()
            .map(|f| Late { lines: std::mem::take(&mut f.silhouettes), after: std::mem::take(&mut f.after), left: 0 })
            .collect();
        let mut waiting: Vec<Vec<usize>> = vec![Vec::new(); late.len()];
        for (i, l) in late.iter_mut().enumerate() {
            l.left = l.after.len();
            for j in &l.after {
                if let Some(list) = waiting.get_mut(*j) {
                    list.push(i);
                }
            }
        }
        let elements: Vec<ElementId> = faces.iter().map(|f| f.element).collect();
        let mut held: Vec<usize> = Vec::new();
        let mut slots: Vec<Option<ProjectedFace>> = faces.into_iter().map(Some).collect();
        for (i, f) in order.into_iter().filter_map(|i| Some((i, slots[i].take()?))) {
            // A face over a held silhouette, and not beside it, hides it: the silhouette goes first.
            held.retain(|&h| {
                let Some((l, element)) = late.get(h).zip(elements.get(h)) else { return false };
                let hidden = !l.after.contains(&i)
                    && l.lines.iter().any(|&(a, b)| cc_geom::polygon::contains(&f.pts, (a + b) * 0.5));
                if hidden {
                    for &(a, b) in &l.lines {
                        d.line(a, b, LineStyle::Projection, Some(*element));
                    }
                }
                !hidden
            });
            if let Some(color) = f.color {
                // A shadow is a veil over the face's fill, under its lines; it
                // belongs to no element (it is neither picked nor overridden).
                let veil = sun.map_or(0, |sun| shadow::veil(style, f.normal.dot(sun), &look.shading));
                let cast = shadows.get_mut(i).map(std::mem::take).unwrap_or_default();
                let veils = |d: &mut Drawing2D| {
                    for ring in cast.iter().filter(|_| veil > 0) {
                        d.fill(ring.clone(), [0, 0, 0, veil], None);
                    }
                };
                if style == VisualStyle::Textures
                    && let Some(solid) = geom.solids.get(f.solid)
                {
                    d.fill(f.pts.clone(), color, Some(f.element));
                    veils(&mut d);
                    surface_lines(&mut d, patterns, solid, f.face, &f.pts, look.spacing);
                } else {
                    d.fill(f.pts, color, Some(f.element));
                    veils(&mut d);
                }
            }
            for (a, b) in f.edges {
                d.line(a, b, LineStyle::Projection, Some(f.element));
            }
            for (a, b, style) in f.marks {
                d.line(a, b, style, Some(f.element));
            }
            // Its own silhouettes, unless faces beside them are still to come,
            // and those that waited for this face alone.
            if late.get(i).is_some_and(|l| !l.lines.is_empty()) {
                held.push(i);
            }
            for &h in waiting.get(i).into_iter().flatten() {
                if let Some(l) = late.get_mut(h) {
                    l.left = l.left.saturating_sub(1);
                }
            }
            held.retain(|&h| {
                let Some((l, element)) = late.get(h).zip(elements.get(h)) else { return false };
                if l.left == 0 {
                    for &(a, b) in &l.lines {
                        d.line(a, b, LineStyle::Projection, Some(*element));
                    }
                }
                l.left > 0
            });
        }
        // A cycle in the order may leave some waiting.
        for (l, element) in held.into_iter().filter_map(|h| late.get(h).zip(elements.get(h))) {
            for &(a, b) in &l.lines {
                d.line(a, b, LineStyle::Projection, Some(*element));
            }
        }
    } else {
        // Every edge, in no particular order: nothing hides anything.
        for f in faces {
            for (a, b) in f.edges.into_iter().chain(f.silhouettes) {
                d.line(a, b, LineStyle::Projection, Some(f.element));
            }
            for (a, b, style) in f.marks {
                d.line(a, b, style, Some(f.element));
            }
        }
    }

    // Section cut: poche where loops close, heavy outline always.
    if let Some(near) = frame.near {
        let point = frame.origin + frame.forward * near;
        for solid in geom.solids.iter().filter(shown) {
            let segs: Vec<(DVec2, DVec2)> = solid
                .faces
                .iter()
                .flat_map(|f| face_plane_segments(f, point, frame.forward))
                .filter_map(|(a, b)| planes.iter().skip(1).try_fold((a, b), |(a, b), &(p, n)| clip_segment(a, b, p, n)))
                .map(|(a, b)| (frame.project(a), frame.project(b)))
                .collect();
            // A layered wall's cut shows its layers (at the medium and fine levels, as in plan).
            let wall = geom
                .walls
                .get(&solid.element)
                .filter(|g| cut.is_some() && !solid.secondary && g.curtain.is_none() && g.layers.len() > 1);
            // So does a layered floor's or roof's, its thickness measured
            // vertically (as their bodies are built), seen in an upright view.
            let slab = cut
                .filter(|_| !solid.secondary && frame.up.z > 0.99)
                .and_then(|c| c.project.element(solid.element).and_then(|e| c.project.element_type(e.type_id)))
                .and_then(|t| match &t.data {
                    TypeData::Floor { layers } | TypeData::Roof { layers, .. } => Some(layers),
                    _ => None,
                })
                .filter(|l| l.len() > 1 && l.iter().map(|l| l.thickness).sum::<f64>() > 1e-9);
            for pts in chain_loops(&segs).into_iter().filter(|_| style.fills()) {
                match cut {
                    Some(c) if let Some(layers) = slab => {
                        let total: f64 = layers.iter().map(|l| l.thickness).sum();
                        let mut fractions = vec![0.0];
                        let mut sum = 0.0;
                        for l in layers {
                            sum += l.thickness;
                            fractions.push((sum / total).min(1.0));
                        }
                        let (pieces, lines) = slab_layers(&pts, &fractions);
                        for (layer, pieces) in layers.iter().zip(pieces) {
                            for piece in pieces {
                                let element = Some(solid.element);
                                crate::cut::cut_fill(&mut d, c.project, &layer.material, piece, c.spacing, element);
                            }
                        }
                        let functions: Vec<_> = layers.iter().map(|l| l.function).collect();
                        for (k, lines) in lines.into_iter().enumerate() {
                            let pen = crate::plan::layer_pen(crate::plan::boundary_function(&functions, k));
                            for (a, b) in lines {
                                d.polyline_pen(&[a, b], false, LineStyle::Thin, pen, Some(solid.element));
                            }
                        }
                    }
                    Some(c) if let Some(g) = wall => {
                        let v_of = |s: DVec2| g.local((point + frame.right * s.x + frame.up * s.y).truncate()).1;
                        let v: Vec<f64> = pts.iter().map(|&s| v_of(s)).collect();
                        for (lo, hi, material) in &g.layers {
                            let (lo, hi) = (lo.min(*hi), lo.max(*hi));
                            let piece = clip_by(&pts, |s| v_of(s) - lo);
                            let piece = clip_by(&piece, |s| hi - v_of(s));
                            if piece.len() >= 3 && signed_area(&piece).abs() > 1e-12 {
                                crate::cut::cut_fill(
                                    &mut d,
                                    c.project,
                                    material,
                                    piece,
                                    c.spacing,
                                    Some(solid.element),
                                );
                            }
                        }
                        // The lines between layers, pens by their function as in plan.
                        let functions = crate::plan::layer_functions(c.project, g);
                        for (k, &(_, at, _)) in g.layers.iter().enumerate().take(g.layers.len() - 1) {
                            let pen = crate::plan::layer_pen(crate::plan::boundary_function(&functions, k));
                            for (a, b) in level_crossings(&pts, &v, at) {
                                d.polyline_pen(&[a, b], false, LineStyle::Thin, pen, Some(solid.element));
                            }
                        }
                    }
                    Some(c) => {
                        let ty = c.project.element(solid.element).and_then(|e| c.project.element_type(e.type_id));
                        let material = ty.map_or("", |t| match &t.data {
                            TypeData::Roof { framing, .. } if solid.secondary => framing.material.as_str(),
                            data => data.cut_material(),
                        });
                        crate::cut::cut_fill(&mut d, c.project, material, pts, c.spacing, Some(solid.element));
                    }
                    None => d.cut_fill(pts, POCHE, Some(solid.element)),
                }
            }
            for (a, b) in segs {
                d.line(a, b, LineStyle::Cut, Some(solid.element));
            }
        }
    }
    d
}

/// The layers of the cut `poly` of a slab whose layers run down from its
/// top, `fractions` (from 0 to 1) their boundaries as parts of its height:
/// the pieces of each layer and the lines between each two. The cut is split
/// into vertical strips at its corners, each column between an edge below
/// and one above shared out by height; a layer's pieces in neighbouring
/// strips that meet become one.
type SlabLayers = (Vec<Vec<Vec<DVec2>>>, Vec<Vec<(DVec2, DVec2)>>);
fn slab_layers(poly: &[DVec2], fractions: &[f64]) -> SlabLayers {
    let count = fractions.len().saturating_sub(1);
    // Each layer's runs: points along its bottom and along its top, left to right.
    let mut runs: Vec<Vec<(Vec<DVec2>, Vec<DVec2>)>> = vec![Vec::new(); count];
    let mut lines: Vec<Vec<(DVec2, DVec2)>> = vec![Vec::new(); count.saturating_sub(1)];
    let mut xs: Vec<f64> = poly.iter().map(|p| p.x).filter(|x| x.is_finite()).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let edges: Vec<(DVec2, DVec2)> =
        (0..poly.len()).filter_map(|i| Some((*poly.get(i)?, *poly.get((i + 1) % poly.len())?))).collect();
    let close = |p: DVec2, q: DVec2| p.distance_squared(q) < 1e-14;
    for w in xs.windows(2) {
        let (x0, x1) = (w[0], w[1]);
        let mut spans: Vec<(f64, f64)> = edges
            .iter()
            .filter(|(a, b)| a.x.min(b.x) <= x0 + 1e-9 && a.x.max(b.x) >= x1 - 1e-9 && (a.x - b.x).abs() > 1e-9)
            .map(|(a, b)| {
                let y = |x: f64| a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
                (y(x0), y(x1))
            })
            .collect();
        spans.sort_by(|p, q| (p.0 + p.1).total_cmp(&(q.0 + q.1)));
        for pair in spans.as_chunks::<2>().0 {
            let [(b0, b1), (t0, t1)] = *pair;
            if t0 - b0 < 1e-12 && t1 - b1 < 1e-12 {
                continue;
            }
            // The points at a part `f` of the height down from the top, at both sides.
            let at = |f: f64| (DVec2::new(x0, t0 - f * (t0 - b0)), DVec2::new(x1, t1 - f * (t1 - b1)));
            for (k, runs) in runs.iter_mut().enumerate() {
                let (Some(&fa), Some(&fb)) = (fractions.get(k), fractions.get(k + 1)) else { continue };
                let ((top0, top1), (low0, low1)) = (at(fa), at(fb));
                let open = runs.iter_mut().find(|(low, top)| {
                    low.last().is_some_and(|&p| close(p, low0)) && top.last().is_some_and(|&p| close(p, top0))
                });
                match open {
                    Some((low, top)) => {
                        low.push(low1);
                        top.push(top1);
                    }
                    None => runs.push((vec![low0, low1], vec![top0, top1])),
                }
            }
            for (k, lines) in lines.iter_mut().enumerate() {
                let Some(&f) = fractions.get(k + 1) else { continue };
                let (a, b) = at(f);
                lines.push((a, b));
            }
        }
    }
    let pieces = runs
        .into_iter()
        .map(|runs| {
            runs.into_iter()
                .map(|(mut low, top)| {
                    low.extend(top.into_iter().rev());
                    low
                })
                .filter(|p| signed_area(p).abs() > 1e-12)
                .collect()
        })
        .collect();
    (pieces, lines)
}

/// The part of `poly` where `side` is not negative (Sutherland-Hodgman, the
/// side taken linear along each edge; a polygon that is not convex may come
/// out with zero-width bridges along the boundary, which fill nothing).
fn clip_by(poly: &[DVec2], side: impl Fn(DVec2) -> f64) -> Vec<DVec2> {
    let sides: Vec<f64> = poly.iter().map(|&p| side(p)).collect();
    let mut out = Vec::with_capacity(poly.len() + 2);
    for (i, (&p, &sp)) in poly.iter().zip(&sides).enumerate() {
        let j = (i + 1) % poly.len();
        let (Some(&q), Some(&sq)) = (poly.get(j), sides.get(j)) else { continue };
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) && (sp - sq).abs() > 1e-15 {
            out.push(p + (q - p) * (sp / (sp - sq)));
        }
    }
    out
}

/// The stretches of the line where `values` (one per vertex of `poly`, taken
/// linear along its edges) equal `at` that lie inside `poly`: its crossings,
/// sorted along the line and paired (even-odd).
fn level_crossings(poly: &[DVec2], values: &[f64], at: f64) -> Vec<(DVec2, DVec2)> {
    let mut hits = Vec::new();
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        let (Some(&p), Some(&q), Some(&a), Some(&b)) = (poly.get(i), poly.get(j), values.get(i), values.get(j)) else {
            continue;
        };
        // Half-open, so a vertex on the level counts once.
        if (a >= at) != (b >= at) && (a - b).abs() > 1e-15 {
            hits.push(p + (q - p) * ((a - at) / (a - b)));
        }
    }
    let (Some(&first), Some(&last)) = (hits.first(), hits.last()) else { return Vec::new() };
    let along = (last - first).normalize_or_zero();
    hits.sort_by(|p, q| p.dot(along).total_cmp(&q.dot(along)));
    hits.as_chunks::<2>().0.iter().map(|&[a, b]| (a, b)).collect()
}

/// Depth (along the view direction) of the plane of `f` under screen point `s`.
fn depth_at(frame: &Frame, f: &ProjectedFace, s: DVec2) -> f64 {
    let base = frame.origin + frame.right * s.x + frame.up * s.y;
    f.normal.dot(f.point - base) / f.normal.dot(frame.forward)
}

/// Clip a polygon by a counter-clockwise convex polygon (Sutherland-Hodgman;
/// a polygon that is not convex may come out with sides along the clip's).
pub(crate) fn clip_convex(subject: &[DVec2], clipper: &[DVec2]) -> Vec<DVec2> {
    let mut out = subject.to_vec();
    for i in 0..clipper.len() {
        if out.is_empty() {
            break;
        }
        let (a, b) = (clipper[i], clipper[(i + 1) % clipper.len()]);
        let side = |p: DVec2| (b - a).perp_dot(p - a);
        let input = std::mem::take(&mut out);
        for j in 0..input.len() {
            let (p, q) = (input[j], input[(j + 1) % input.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                out.push(p + (q - p) * (sp / (sp - sq)));
            }
        }
    }
    out
}

/// Counter-clockwise triangles of a projected face.
fn triangles(pts: &[DVec2]) -> Vec<[DVec2; 3]> {
    cc_geom::polygon::triangulate(pts)
        .into_iter()
        .map(|[a, b, c]| {
            let t = [pts[a], pts[b], pts[c]];
            if signed_area(&t) < 0.0 { [t[0], t[2], t[1]] } else { t }
        })
        .collect()
}

/// A point strictly inside the overlap of two faces on screen: the centroid of
/// the largest overlap between their triangles.
fn overlap_point(a: &[[DVec2; 3]], b: &[[DVec2; 3]]) -> Option<DVec2> {
    const MIN_AREA: f64 = 1e-8;
    let mut best: Option<(f64, DVec2)> = None;
    for ta in a {
        for tb in b {
            let clipped = clip_convex(ta, tb);
            if clipped.len() < 3 {
                continue;
            }
            let area = signed_area(&clipped).abs();
            if area > MIN_AREA && best.is_none_or(|(a, _)| area > a) {
                let centroid = clipped.iter().copied().sum::<DVec2>() / clipped.len() as f64;
                best = Some((area, centroid));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Back-to-front painting order: a face is painted after every face it hides.
fn paint_order(frame: &Frame, faces: &[ProjectedFace]) -> Vec<usize> {
    const DEPTH_TOL: f64 = 1e-6;
    let n = faces.len();
    let boxes: Vec<(DVec2, DVec2)> =
        faces.iter().map(|f| f.pts.iter().fold((f.pts[0], f.pts[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p)))).collect();
    let tris: Vec<Vec<[DVec2; 3]>> = faces.iter().map(|f| triangles(&f.pts)).collect();
    // `before[i]` lists the faces that must be painted after face `i`.
    let mut before: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pending = vec![0usize; n];
    let mut compare = |i: usize, j: usize| {
        let (a, b) = (boxes[i], boxes[j]);
        if a.0.x >= b.1.x || b.0.x >= a.1.x || a.0.y >= b.1.y || b.0.y >= a.1.y {
            return;
        }
        let Some(s) = overlap_point(&tris[i], &tris[j]) else { return };
        let (di, dj) = (depth_at(frame, &faces[i], s), depth_at(frame, &faces[j], s));
        if (di - dj).abs() < DEPTH_TOL {
            return;
        }
        let (back, front) = if di > dj { (i, j) } else { (j, i) };
        before[back].push(front);
        pending[front] += 1;
    };
    // Each pair whose boxes may overlap is compared once, the lower index
    // first: a sweep along X over the boxes sorted by their left side meets
    // only faces whose X ranges overlap. Boxes that are not finite are
    // compared with every face, as their tests may not fail.
    let finite = |k: usize| boxes[k].0.is_finite() && boxes[k].1.is_finite();
    let (mut sweep, odd): (Vec<usize>, Vec<usize>) = (0..n).partition(|&k| finite(k));
    sweep.sort_by(|&a, &b| boxes[a].0.x.total_cmp(&boxes[b].0.x).then(a.cmp(&b)));
    for (p, &i) in sweep.iter().enumerate() {
        for &j in &sweep[p + 1..] {
            // This box and every later one start where this one ends or after.
            if boxes[j].0.x >= boxes[i].1.x {
                break;
            }
            compare(i.min(j), i.max(j));
        }
    }
    for &i in &odd {
        for j in (0..n).filter(|&j| j != i && (finite(j) || j > i)) {
            compare(i.min(j), i.max(j));
        }
    }
    // Kahn's algorithm, taking the farthest ready face first (the last of
    // equally far ones). On a cycle the face waiting on the fewest others is
    // forced (the farthest among them): forcing the farthest instead lets a
    // large face, such as a roof with many members under it, be painted
    // before most of what it hides.
    let mut ready: std::collections::BinaryHeap<(Depth, usize)> =
        (0..n).filter(|&k| pending[k] == 0).map(|k| (Depth(faces[k].depth), k)).collect();
    let mut done = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for _ in 0..n {
        let mut next = None;
        while let Some((_, k)) = ready.pop() {
            if !done[k] {
                next = Some(k);
                break;
            }
        }
        let forced = || {
            (0..n)
                .filter(|&k| !done[k])
                .min_by(|&x, &y| pending[x].cmp(&pending[y]).then(faces[y].depth.total_cmp(&faces[x].depth)))
        };
        let Some(k) = next.or_else(forced) else { break };
        done[k] = true;
        order.push(k);
        for &f in &before[k] {
            if pending[f] > 0 {
                pending[f] -= 1;
                if pending[f] == 0 && !done[f] {
                    ready.push((Depth(faces[f].depth), f));
                }
            }
        }
    }
    order
}

/// A depth ordered by [`f64::total_cmp`], for the queue of faces ready to paint.
#[derive(Debug, Clone, Copy)]
struct Depth(f64);

impl PartialEq for Depth {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0).is_eq()
    }
}

impl Eq for Depth {}

impl PartialOrd for Depth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Depth {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Join segments end-to-end into closed loops.
pub(crate) fn chain_loops(segs: &[(DVec2, DVec2)]) -> Vec<Vec<DVec2>> {
    const TOL: f64 = 1e-6;
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut pts = vec![segs[start].0, segs[start].1];
        while let Some(&tail) = pts.last() {
            if pts.len() > 2 && tail.distance(pts[0]) < TOL {
                pts.pop();
                loops.push(pts);
                break;
            }
            let next = (0..segs.len())
                .find(|&i| !used[i] && (segs[i].0.distance(tail) < TOL || segs[i].1.distance(tail) < TOL));
            let Some(i) = next else { break };
            used[i] = true;
            let (a, b) = segs[i];
            pts.push(if a.distance(tail) < TOL { b } else { a });
        }
    }
    loops
}

/// The frame of an elevation looking `direction`.
fn elevation_frame(direction: Direction) -> Frame {
    let toward = direction.look();
    Frame::looking(DVec3::ZERO, DVec3::new(toward.x, toward.y, 0.0))
}

/// The frame of a section along `start`..`end` (looking to the right of
/// it), clipped to `depth` and to the line's ends.
fn section_frame(start: DVec2, end: DVec2, depth: f64) -> Frame {
    let dir = (end - start).normalize_or_zero();
    let toward = -dir.perp();
    let mut frame = Frame::looking(DVec3::new(start.x, start.y, 0.0), DVec3::new(toward.x, toward.y, 0.0));
    let xe = frame.project(DVec3::new(end.x, end.y, 0.0)).x;
    frame.near = Some(0.0);
    frame.far = Some(depth.max(0.01));
    frame.crop_x = Some((xe.min(0.0), xe.max(0.0)));
    frame
}

/// The frame of a 3D view's 2D form, looking along its camera.
fn camera_frame(camera: &CameraState) -> Frame {
    Frame::looking(DVec3::ZERO, camera.forward())
}

/// The frame through which a projected view (elevation, section, the 2D
/// form of a 3D view) maps the model onto its drawing, as its generator
/// builds it; `None` for other views (plans draw plan coordinates as they are).
pub fn view_frame(view: &cc_core::View) -> Option<Frame> {
    match &view.kind {
        cc_core::ViewKind::Elevation { direction } => Some(elevation_frame(*direction)),
        cc_core::ViewKind::Section { start, end, depth, .. } => Some(section_frame(*start, *end, *depth)),
        cc_core::ViewKind::ThreeD { camera, .. } => Some(camera_frame(camera)),
        cc_core::ViewKind::InteriorElevation { .. } => crate::interior::frame(view),
        _ => None,
    }
}

pub fn elevation(project: &Project, geom: &ModelGeometry, direction: Direction, look: Look<'_>) -> Drawing2D {
    let frame = elevation_frame(direction);
    render(geom, &frame, None, look, &project.fill_patterns)
}

pub fn section(
    project: &Project,
    geom: &ModelGeometry,
    (start, end): (DVec2, DVec2),
    depth: f64,
    scale: u32,
    look: Look<'_>,
) -> Drawing2D {
    section_in(project, geom, &section_frame(start, end, depth), scale, look)
}

/// A section through `frame` (its near plane cuts; see [`section`]).
pub fn section_in(project: &Project, geom: &ModelGeometry, frame: &Frame, scale: u32, look: Look<'_>) -> Drawing2D {
    let cut = CutStyle { project, spacing: crate::cut::spacing(scale) };
    let mut d = render(geom, frame, Some(cut), look, &project.fill_patterns);
    existing_ground(&mut d, geom, frame);
    d
}

/// Where the section plane crosses the existing ground that pads changed, dashed.
fn existing_ground(d: &mut Drawing2D, geom: &ModelGeometry, frame: &Frame) {
    let Some(near) = frame.near else { return };
    let point = frame.origin + frame.forward * near;
    let planes = frame.planes();
    for (id, g) in &geom.topographies {
        for t in &g.existing {
            let side = t.map(|p| (p - point).dot(frame.forward));
            let mut hits = Vec::with_capacity(2);
            for i in 0..3 {
                let (a, b, da, db) = (t[i], t[(i + 1) % 3], side[i], side[(i + 1) % 3]);
                if (da >= 0.0) != (db >= 0.0) {
                    hits.push(a.lerp(b, da / (da - db)));
                }
            }
            let [a, b] = hits[..] else { continue };
            let Some((a, b)) = planes.iter().skip(1).try_fold((a, b), |(a, b), &(p, n)| clip_segment(a, b, p, n))
            else {
                continue;
            };
            let (a, b) = (frame.project(a), frame.project(b));
            if a.distance(b) > 1e-9 {
                d.line(a, b, LineStyle::Hidden, Some(*id));
            }
        }
    }
}

pub fn axonometric(project: &Project, geom: &ModelGeometry, camera: &CameraState, look: Look<'_>) -> Drawing2D {
    render(geom, &camera_frame(camera), None, look, &project.fill_patterns)
}

/// The analytical model seen along the camera: member lines (columns and
/// beams) and nodes, supports marked with a triangle.
pub fn analytical(project: &Project, geom: &ModelGeometry, camera: &CameraState) -> Drawing2D {
    let frame = camera_frame(camera);
    let model = cc_geom::analytical::analytical_model(project, geom);
    let mut d = Drawing2D::default();
    for m in &model.members {
        let style = match m.kind {
            cc_geom::analytical::MemberKind::Column => crate::LineStyle::Cut,
            cc_geom::analytical::MemberKind::Beam => crate::LineStyle::Projection,
        };
        // An arc member in short chords.
        let pts: Vec<DVec2> = model.points(m, cc_core::arc::ARC_STEP).iter().map(|p| frame.project(*p)).collect();
        if pts.len() >= 2 {
            d.polyline(&pts, false, style, Some(m.element));
        }
    }
    for n in &model.nodes {
        let p = frame.project(n.position);
        let r = 0.08;
        let marker: Vec<DVec2> = if n.support.is_some() {
            vec![p, p + DVec2::new(r, -1.6 * r), p + DVec2::new(-r, -1.6 * r)]
        } else {
            (0..8).map(|i| p + DVec2::from_angle(i as f64 * std::f64::consts::FRAC_PI_4) * (r * 0.5)).collect()
        };
        d.polyline(&marker, true, crate::LineStyle::Thin, None);
    }
    d
}

/// The reinforcing bars seen along the camera, tagged with their sets, at
/// `scale` (1:`scale`); a leg of an open bar seen end-on (the vertical bars
/// of a column seen from above, a starter bar's leg above its foot) is a dot
/// as wide as the bar, at least [`crate::rebar_view::MIN_DOT`] on paper, as
/// plans and sections draw it ([`crate::rebar_view::dot`]).
pub fn rebar(geom: &ModelGeometry, camera: &CameraState, scale: u32) -> Drawing2D {
    use crate::rebar_view::{END_ON, dot, dot_radius};
    let frame = camera_frame(camera);
    let k = scale.max(1) as f64 / 1000.0;
    let mut d = Drawing2D::default();
    for set in geom.rebar.values() {
        let r = dot_radius(set.diameter, k);
        for bar in &set.bars {
            let pts: Vec<DVec2> = bar.points.iter().map(|p| frame.project(*p)).collect();
            let mut dots: Vec<DVec2> = Vec::new();
            if !bar.closed {
                for (w, p) in pts.windows(2).zip(bar.points.windows(2)) {
                    if w[0].distance(w[1]) < END_ON
                        && p[0].distance(p[1]) > END_ON
                        && !dots.iter().any(|q| q.distance(w[0]) < r)
                    {
                        dots.push(w[0]);
                    }
                }
            }
            for at in dots {
                dot(&mut d, at, r, set.element);
            }
            let run: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
            if run >= END_ON {
                d.polyline(&pts, bar.closed, crate::LineStyle::Projection, Some(set.element));
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prim;
    use cc_core::{Category, Document, Element, ElementKind, Wall};

    fn one_wall() -> (Document, ElementId) {
        let mut doc = Document::new(Project::from_template());
        let id = doc.edit("t", |e| {
            let level = e.project().levels[0].id;
            let type_id = e.project().default_type(Category::Wall).unwrap();
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Wall(Wall {
                    start: DVec2::ZERO,
                    end: DVec2::new(4.0, 0.0),
                    base_offset: 0.0,
                    height: 3.0,
                    ..Default::default()
                }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        (doc, id)
    }

    /// Each visual style draws the same wall differently: the wireframe
    /// only lines (the far face's edges too), hidden line white faces,
    /// shaded the same faces and edges greyed by the light, consistent
    /// colors the material's color unlit, textures the
    /// material's appearance with its surface pattern in the pattern's color.
    #[test]
    fn visual_styles_fill_faces_their_own_way() {
        use cc_core::VisualStyle as V;
        let (mut doc, id) = one_wall();
        let material = doc
            .project
            .element(id)
            .and_then(|e| doc.project.element_type(e.type_id))
            .map(|t| t.main_material().to_string())
            .unwrap();
        let m = doc.project.materials.iter_mut().find(|m| m.name == material).unwrap();
        m.appearance.color = [180, 60, 40];
        m.graphics.surface.foreground = cc_core::Pattern::Brick;
        m.graphics.surface.foreground_color = [90, 20, 10];
        let geom = ModelGeometry::build(&doc.project);
        let draw = |style| elevation(&doc.project, &geom, Direction::South, Look { style, ..Look::plain() });
        let fills = |d: &Drawing2D| -> Vec<[u8; 4]> {
            d.prims
                .iter()
                .filter_map(|p| match p {
                    Prim::Fill { color, cut: false, .. } => Some(*color),
                    _ => None,
                })
                .collect()
        };
        let lines = |d: &Drawing2D| d.prims.iter().filter(|p| matches!(p, Prim::Line { .. })).count();
        let (wire, hidden) = (draw(V::Wireframe), draw(V::HiddenLine));
        assert!(fills(&wire).is_empty());
        assert!(lines(&wire) > lines(&hidden), "the far face's edges show through");
        assert!(!fills(&hidden).is_empty() && fills(&hidden).iter().all(|c| c[..3] == [255, 255, 255]));
        let color = doc.project.material_color(&material);
        let consistent = fills(&draw(V::ConsistentColors));
        assert!(consistent.iter().all(|c| c[..3] == color), "{consistent:?} vs {color:?}");
        // Shaded: hidden line's edges over white faces greyed by the light, not the material's color.
        let shaded = draw(V::Shaded);
        assert_eq!(lines(&shaded), lines(&hidden));
        let greys = fills(&shaded);
        assert!(!greys.is_empty() && greys.iter().all(|c| c[0] == c[1] && c[1] == c[2]), "{greys:?}");
        assert!(greys.iter().all(|c| f64::from(c[0]) >= 255.0 * V::SHADED_AMBIENT - 1.0), "never dark: {greys:?}");
        assert!(greys.iter().any(|c| c[0] > 200), "the face turned to the light is near white: {greys:?}");
        let textures = draw(V::Textures);
        let bricks = textures
            .prims
            .iter()
            .filter(|p| matches!(p, Prim::Line { pen: Some(pen), .. } if pen.color == Some([90, 20, 10])))
            .count();
        assert!(bricks > 10, "{bricks} brick lines");
        assert!(fills(&textures).iter().all(|c| c[0] > c[1]), "the reddish appearance color");
        // The wireframe is still picked by its lines.
        assert_eq!(wire.hit_test(DVec2::new(0.0, 1.5), 0.05), Some(id));
    }

    /// A sun lights the faces turned to it: the wall's south face is brighter
    /// with the sun to the south than to the north, a sun below the horizon
    /// leaves the ambient light only, and no sun keeps the default light.
    #[test]
    fn the_sun_lights_the_faces_turned_to_it() {
        use cc_core::sun::{SunMode, SunSettings};
        let (doc, _) = one_wall();
        let geom = ModelGeometry::build(&doc.project);
        let fills = |d: &Drawing2D| -> Vec<[u8; 4]> {
            d.prims
                .iter()
                .filter_map(|p| match p {
                    Prim::Fill { color, .. } => Some(*color),
                    _ => None,
                })
                .collect()
        };
        let lit = |light| fills(&elevation(&doc.project, &geom, Direction::South, Look { light, ..Look::plain() }));
        let brightness =
            |c: &[[u8; 4]]| c.iter().map(|c| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2])).sum::<u32>();
        let south = lit(Some(DVec3::new(0.0, -1.0, 1.0).normalize()));
        let north = lit(Some(DVec3::new(0.0, 1.0, 1.0).normalize()));
        let night = lit(Some(DVec3::ZERO));
        assert!(!south.is_empty() && south.len() == north.len());
        assert!(brightness(&south) > brightness(&north), "{south:?} vs {north:?}");
        assert_eq!(north, night, "the face turned away gets the ambient light only");
        // White greyed by the light: a mid grey turned away, near white facing a sun 45° up.
        assert!(north.iter().all(|c| c[..3] == [153; 3]), "{north:?}");
        assert!(south.iter().all(|c| c[..3] == [225; 3]), "{south:?}");
        assert_eq!(cc_core::VisualStyle::shaded_grey(1.0), 1.0);
        assert_eq!(cc_core::VisualStyle::shaded_grey(f64::NAN), cc_core::VisualStyle::SHADED_AMBIENT);
        assert_ne!(lit(None), south);
        // No sun: the default light; a view's sun comes from the project.
        let mut view = cc_core::View::new(
            cc_core::ViewId(9999),
            "E",
            100,
            cc_core::ViewKind::Elevation { direction: Direction::South },
        );
        assert_eq!(Look::of(&doc.project, &geom, &view, None, None).light, None);
        assert_eq!(
            Look::of(&doc.project, &geom, &view, None, None),
            Look { light: None, ..Look::of(&doc.project, &geom, &view, None, None) }
        );
        view.sun = Some(SunSettings::default());
        let state = doc.project.sun_state(&view).unwrap();
        assert!(state.above_horizon);
        assert_eq!(Look::of(&doc.project, &geom, &view, None, None).light, Some(state.direction));
        view.sun =
            Some(SunSettings { mode: SunMode::Still { date: Default::default(), time: 0 }, ..Default::default() });
        assert_eq!(
            Look::of(&doc.project, &geom, &view, None, None).light,
            Some(DVec3::ZERO),
            "midnight at the equator"
        );
    }

    #[test]
    fn south_elevation_shows_wall_face() {
        let (doc, id) = one_wall();
        let geom = ModelGeometry::build(&doc.project);
        let d = elevation(&doc.project, &geom, Direction::South, Look::plain());
        assert_eq!(d.hit_test(DVec2::new(2.0, 1.5), 0.0), Some(id));
        assert_eq!(d.hit_test(DVec2::new(2.0, 3.5), 0.0), None);
    }

    /// A roof slab overhanging a wall, seen from above: every wall face lies
    /// under the roof, so the roof's top face must be painted after all of them.
    /// Sorting faces by their centroid depth alone paints the near end of the
    /// wall over the roof.
    #[test]
    fn large_faces_hide_what_is_behind_them() {
        use cc_geom::Solid;
        let (roof_id, wall_id) = (ElementId(1), ElementId(2));
        let mut roof = Solid::new(roof_id, Category::Roof, [200, 80, 60]);
        roof.add_box(DVec2::ZERO, DVec2::X, DVec2::Y, 0.0, 10.0, -2.0, 12.0, 3.0, 3.2);
        let mut wall = Solid::new(wall_id, Category::Wall, [200, 200, 200]);
        wall.add_box(DVec2::ZERO, DVec2::X, DVec2::Y, 4.0, 4.2, 0.0, 10.0, 0.0, 3.0);
        let geom = ModelGeometry { solids: vec![roof, wall], ..Default::default() };
        let frame = Frame::looking(DVec3::ZERO, DVec3::new(1.0, 1.0, -1.0));
        let d = render(&geom, &frame, None, Look::plain(), &Default::default());
        let fills: Vec<(usize, ElementId, f64)> = d
            .prims
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                Prim::Fill { pts, element: Some(e), .. } => Some((i, *e, cc_geom::polygon::area(pts))),
                _ => None,
            })
            .collect();
        let roof_top = fills.iter().filter(|f| f.1 == roof_id).max_by(|a, b| a.2.total_cmp(&b.2)).unwrap().0;
        let last_wall = fills.iter().filter(|f| f.1 == wall_id).map(|f| f.0).max().unwrap();
        assert!(roof_top > last_wall, "roof top painted at {roof_top}, a wall face at {last_wall}");
    }

    /// A thin round bar seen from the side (an oven's handle): its facets
    /// are thinner than a line on screen, so the facets painted after a
    /// silhouette covered it and the bar lost its outline. Both silhouettes
    /// are drawn after every facet of the bar.
    #[test]
    fn silhouettes_are_drawn_after_the_facets_beside_them() {
        use cc_geom::Solid;
        let (r, n) = (0.011, 24);
        let at = |x: f64, k: usize| {
            let t = std::f64::consts::TAU * k as f64 / n as f64;
            DVec3::new(x, r * t.cos(), r * t.sin())
        };
        let mut bar = Solid::new(ElementId(1), Category::Component, [200, 200, 200]);
        bar.add_polygons((0..n).map(|k| vec![at(0.0, k), at(0.0, k + 1), at(0.4, k + 1), at(0.4, k)]));
        bar.smooth_curved(0);
        assert!(bar.faces.iter().all(|f| f.smooth));
        let geom = ModelGeometry { solids: vec![bar], ..Default::default() };
        let frame = Frame::looking(DVec3::new(0.0, -5.0, 0.0), DVec3::Y);
        let d = render(&geom, &frame, None, Look::plain(), &Default::default());
        let last_fill = d.prims.iter().rposition(|p| matches!(p, Prim::Fill { .. })).unwrap();
        let long = |y: f64| {
            d.prims.iter().position(|p| {
                matches!(p, Prim::Line { a, b, .. } if (a.y - y).abs() < 1e-9 && (b.y - y).abs() < 1e-9 && (a.x - b.x).abs() > 0.39)
            })
        };
        for y in [r, -r] {
            let line = long(y).unwrap_or_else(|| panic!("no silhouette at {y}"));
            assert!(line > last_fill, "the silhouette at {y} is prim {line}, a facet is painted at {last_fill}");
        }
        // The wireframe draws them too, once.
        let wire = Look { style: VisualStyle::Wireframe, ..Look::plain() };
        let d = render(&geom, &frame, None, wire, &Default::default());
        assert!(d.prims.iter().all(|p| matches!(p, Prim::Line { .. })));
    }

    /// Three members lying over one another in a ring (a cycle) under a
    /// large tilted face whose mean depth is the farthest: breaking the cycle
    /// must not paint the large face before the members it hides.
    #[test]
    fn cycles_do_not_paint_a_large_face_early() {
        let frame = Frame::looking(DVec3::new(0.0, 0.0, 10.0), -DVec3::Z);
        let face = |verts: Vec<DVec3>| {
            let normal = (verts[1] - verts[0]).cross(verts[2] - verts[0]).normalize();
            ProjectedFace {
                depth: verts.iter().map(|v| frame.depth(*v)).sum::<f64>() / verts.len() as f64,
                normal,
                point: verts[0],
                pts: verts.iter().map(|v| frame.project(*v)).collect(),
                verts: Vec::new(),
                color: Some([0; 4]),
                element: ElementId(1),
                solid: 0,
                face: 0,
                edges: Vec::new(),
                silhouettes: Vec::new(),
                after: Vec::new(),
                marks: Vec::new(),
            }
        };
        // The large face rises along x: 4.25 to 5.75 m high over the members.
        let z = |x: f64| 1.0 + 0.5 * x;
        let big = face(vec![
            DVec3::new(-10.0, -10.0, z(-10.0)),
            DVec3::new(10.0, -10.0, z(10.0)),
            DVec3::new(10.0, 10.0, z(10.0)),
            DVec3::new(-10.0, 10.0, z(-10.0)),
        ]);
        // Strips along the sides of a triangle around (8, 0), each rising from
        // 3 to 3.5 m in turn, so each lies over the next where they cross.
        let center = DVec2::new(8.0, 0.0);
        let corner = |k: usize| {
            let a = std::f64::consts::TAU * k as f64 / 3.0;
            center + DVec2::new(a.cos(), a.sin()) * 1.5
        };
        let mut faces = vec![big];
        for k in 0..3 {
            let (p, q) = (corner(k), corner(k + 1));
            let d = (q - p).normalize();
            let (a, b) = (p - d * 0.5, q + d * 0.5);
            let side = d.perp() * 0.15;
            let at = |p: DVec2, h: f64| DVec3::new(p.x, p.y, h);
            faces.push(face(vec![at(a - side, 3.0), at(b - side, 3.5), at(b + side, 3.5), at(a + side, 3.0)]));
        }
        let order = paint_order(&frame, &faces);
        assert_eq!(order.len(), 4);
        assert_eq!(order.last(), Some(&0), "the large face hides the members: painted last, {order:?}");
    }

    /// The order found with a queue of ready faces and a sweep over their
    /// boxes is the one comparing every pair and scanning every face gives,
    /// with overlaps, ties in depth and cycles among many tilted faces.
    #[test]
    fn paint_order_matches_comparing_every_pair() {
        fn reference(frame: &Frame, faces: &[ProjectedFace]) -> Vec<usize> {
            let n = faces.len();
            let boxes: Vec<(DVec2, DVec2)> = faces
                .iter()
                .map(|f| f.pts.iter().fold((f.pts[0], f.pts[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p))))
                .collect();
            let tris: Vec<Vec<[DVec2; 3]>> = faces.iter().map(|f| triangles(&f.pts)).collect();
            let mut before: Vec<Vec<usize>> = vec![Vec::new(); n];
            let mut pending = vec![0usize; n];
            for i in 0..n {
                for j in i + 1..n {
                    let (a, b) = (boxes[i], boxes[j]);
                    if a.0.x >= b.1.x || b.0.x >= a.1.x || a.0.y >= b.1.y || b.0.y >= a.1.y {
                        continue;
                    }
                    let Some(s) = overlap_point(&tris[i], &tris[j]) else { continue };
                    let (di, dj) = (depth_at(frame, &faces[i], s), depth_at(frame, &faces[j], s));
                    if (di - dj).abs() < 1e-6 {
                        continue;
                    }
                    let (back, front) = if di > dj { (i, j) } else { (j, i) };
                    before[back].push(front);
                    pending[front] += 1;
                }
            }
            let mut done = vec![false; n];
            let mut order = Vec::with_capacity(n);
            for _ in 0..n {
                let ready = (0..n)
                    .filter(|&k| !done[k] && pending[k] == 0)
                    .max_by(|&x, &y| faces[x].depth.total_cmp(&faces[y].depth));
                let forced = || {
                    (0..n)
                        .filter(|&k| !done[k])
                        .min_by(|&x, &y| pending[x].cmp(&pending[y]).then(faces[y].depth.total_cmp(&faces[x].depth)))
                };
                let Some(k) = ready.or_else(forced) else { break };
                done[k] = true;
                order.push(k);
                for &f in &before[k] {
                    pending[f] = pending[f].saturating_sub(1);
                }
            }
            order
        }
        let frame = Frame::looking(DVec3::new(0.0, 0.0, 50.0), -DVec3::Z);
        let mut seed = 12345u64;
        let mut rand = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut faces = Vec::new();
        for k in 0..300 {
            let (x, y) = (rand() * 20.0, rand() * 20.0);
            let (w, h) = (0.5 + rand() * 4.0, 0.5 + rand() * 4.0);
            // Some faces share a depth; tilted faces cross one another.
            let z = if k % 7 == 0 { 1.0 } else { rand() * 5.0 };
            let tilt = (rand() - 0.5) * 0.8;
            let at = |px: f64, py: f64| DVec3::new(px, py, z + tilt * (px - x));
            let verts = [at(x, y), at(x + w, y), at(x + w, y + h), at(x, y + h)];
            let normal = (verts[1] - verts[0]).cross(verts[2] - verts[0]).normalize();
            faces.push(ProjectedFace {
                depth: verts.iter().map(|v| frame.depth(*v)).sum::<f64>() / verts.len() as f64,
                normal,
                point: verts[0],
                pts: verts.iter().map(|v| frame.project(*v)).collect(),
                verts: Vec::new(),
                color: Some([0; 4]),
                element: ElementId(1),
                solid: 0,
                face: 0,
                edges: Vec::new(),
                silhouettes: Vec::new(),
                after: Vec::new(),
                marks: Vec::new(),
            });
        }
        assert_eq!(paint_order(&frame, &faces), reference(&frame, &faces));
    }

    #[test]
    fn section_cuts_wall_with_poche() {
        let (doc, id) = one_wall();
        let geom = ModelGeometry::build(&doc.project);
        // Section line along X at x = 2, looking -Y... crossing the wall north to south.
        let d = section(&doc.project, &geom, (DVec2::new(2.0, 2.0), DVec2::new(2.0, -2.0)), 5.0, 50, Look::plain());
        let cut = d.prims.iter().filter(|p| matches!(p, Prim::Line { style: LineStyle::Cut, .. })).count();
        assert_eq!(cut, 4);
        // The cut is filled with the core material's pattern (the template's masonry hatch).
        let fills = d.prims.iter().filter(|p| matches!(p, Prim::Fill { element: Some(e), .. } if *e == id)).count();
        assert!(fills >= 1);
        let hatch = d
            .prims
            .iter()
            .filter(|p| matches!(p, Prim::Line { style: LineStyle::Hatch, element: Some(e), .. } if *e == id))
            .count();
        assert!(hatch > 0);
        assert!(d.elements_in(DVec2::splat(-100.0), DVec2::splat(100.0), false).contains(&id));
    }

    #[test]
    fn section_shows_a_layered_walls_layers_above_the_coarse_level() {
        let (doc, id) = one_wall();
        let geom = ModelGeometry::build(&doc.project);
        let layers = geom.walls.get(&id).map_or(0, |g| g.layers.len());
        assert!(layers > 1, "the template's wall has finish layers");
        let line = (DVec2::new(2.0, 2.0), DVec2::new(2.0, -2.0));
        let between = |d: &Drawing2D| {
            d.prims
                .iter()
                .filter(|p| matches!(p, Prim::Line { style: LineStyle::Thin, element: Some(e), .. } if *e == id))
                .count()
        };
        let fills = |d: &Drawing2D| {
            d.prims.iter().filter(|p| matches!(p, Prim::Fill { cut: true, element: Some(e), .. } if *e == id)).count()
        };
        let medium = section(&doc.project, &geom, line, 5.0, 50, Look::plain());
        // One fill per layer and one line between each two, across the whole height.
        assert_eq!(fills(&medium), layers);
        assert_eq!(between(&medium), layers - 1);
        for p in &medium.prims {
            if let Prim::Line { style: LineStyle::Thin, element: Some(e), a, b, .. } = p
                && *e == id
            {
                assert!((a.x - b.x).abs() < 1e-9 && ((a.y - b.y).abs() - 3.0).abs() < 1e-6, "{a} {b}");
            }
        }
        let coarse = section(&doc.project, &geom, line, 5.0, 50, Look { detail: DetailLevel::Coarse, ..Look::plain() });
        assert_eq!(fills(&coarse), 1);
        assert_eq!(between(&coarse), 0);
    }
}

#[cfg(test)]
mod slab_layer_tests {
    use super::*;
    use crate::Prim;
    use cc_core::{Category, Document, Element, ElementKind, Floor, Roof, RoofShape};

    fn place(doc: &mut Document, category: Category, kind: ElementKind) -> (ElementId, usize) {
        doc.edit("t", |e| {
            let level = e.project().levels[0].id;
            let type_id = e.project().default_type(category).unwrap();
            let layers = e.project().element_type(type_id).unwrap().layers().len();
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind,
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            (id, layers)
        })
    }

    fn square() -> Vec<DVec2> {
        vec![DVec2::new(0.0, -2.0), DVec2::new(4.0, -2.0), DVec2::new(4.0, 2.0), DVec2::new(0.0, 2.0)]
    }

    fn layer_prims(d: &Drawing2D, id: ElementId) -> (usize, Vec<(DVec2, DVec2)>) {
        let fills =
            d.prims.iter().filter(|p| matches!(p, Prim::Fill { cut: true, element: Some(e), .. } if *e == id)).count();
        let lines = d
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { style: LineStyle::Thin, element: Some(e), a, b, .. } if *e == id => Some((*a, *b)),
                _ => None,
            })
            .collect();
        (fills, lines)
    }

    #[test]
    fn a_floor_cut_in_section_shows_its_layers() {
        let mut doc = Document::new(Project::from_template());
        let (id, layers) = place(&mut doc, Category::Floor, ElementKind::Floor(Floor::new(square(), 0.0)));
        assert!(layers > 1, "the template's floor has a screed over its slab");
        let geom = ModelGeometry::build(&doc.project);
        let line = (DVec2::new(2.0, 3.0), DVec2::new(2.0, -3.0));
        let (fills, lines) = layer_prims(&section(&doc.project, &geom, line, 5.0, 20, Look::plain()), id);
        assert_eq!(fills, layers);
        assert_eq!(lines.len(), layers - 1);
        // The line under the top layer runs level across the whole floor, its thickness below the top.
        let ty = doc.project.element_type(doc.project.element(id).unwrap().type_id).unwrap();
        let top = ty.layers()[0].thickness;
        let (a, b) = lines[0];
        assert!((a.y - b.y).abs() < 1e-9 && ((a.x - b.x).abs() - 4.0).abs() < 1e-9, "{a} {b}");
        let floor_top = d_top(&doc, &geom, line, id);
        assert!((floor_top - a.y - top).abs() < 1e-9, "{floor_top} {a}");
        let coarse = Look { detail: DetailLevel::Coarse, ..Look::plain() };
        let (fills, lines) = layer_prims(&section(&doc.project, &geom, line, 5.0, 20, coarse), id);
        assert_eq!((fills, lines.len()), (1, 0));
    }

    /// The highest point of the element's cut outline.
    fn d_top(doc: &Document, geom: &ModelGeometry, line: (DVec2, DVec2), id: ElementId) -> f64 {
        let d = section(&doc.project, geom, line, 5.0, 20, Look::plain());
        d.prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { style: LineStyle::Cut, element: Some(e), a, b, .. } if *e == id => Some(a.y.max(b.y)),
                _ => None,
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    #[test]
    fn a_pitched_roof_cut_in_section_shows_its_layers_along_its_slopes() {
        let mut doc = Document::new(Project::from_template());
        let shape = RoofShape::Gable { slope: 0.5, ridge_deg: 0.0 };
        let (id, layers) = place(&mut doc, Category::Roof, ElementKind::Roof(Roof::new(square(), 3.0, 0.5, shape)));
        assert!(layers > 1, "the template's roof has a covering over its deck");
        let geom = ModelGeometry::build(&doc.project);
        let line = (DVec2::new(2.0, 4.0), DVec2::new(2.0, -4.0));
        let (fills, lines) = layer_prims(&section(&doc.project, &geom, line, 5.0, 20, Look::plain()), id);
        // Both slopes of a layer meet at the ridge: one piece each.
        assert_eq!(fills, layers);
        assert!(!lines.is_empty());
        for (a, b) in lines {
            let rise = ((b.y - a.y) / (b.x - a.x)).abs();
            assert!((rise - 0.5).abs() < 1e-6, "{a} {b}");
        }
    }
}
