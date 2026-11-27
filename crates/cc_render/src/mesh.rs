use crate::textures::TextureSet;
use cc_core::ElementId;
use cc_geom::ModelGeometry;
use glam::DVec3;
use std::collections::BTreeSet;
use std::ops::Range;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    /// The surface pattern drawn over a face (textures style): its color
    /// (linear) and, in `w`, its kind (the order of `cc_core::Pattern`, 0 for none).
    pub fg: [f32; 4],
    pub bg: [f32; 4],
    /// The material's texture (textures style): coordinates in repeats in
    /// `xy` and, in `z`, its layer plus one (0 for none).
    pub tex: [f32; 4],
    /// Where the vertex is on its face, in meters (see [`plane_axes`]),
    /// running on across the facets of a curved surface: what the surface
    /// patterns are laid along.
    pub plane: [f32; 2],
}

impl Vertex {
    /// A vertex with no surface pattern or texture.
    pub fn plain(pos: [f32; 3], normal: [f32; 3], color: [f32; 4]) -> Self {
        Self { pos, normal, color, fg: [0.0; 4], bg: [0.0; 4], tex: [0.0; 4], plane: [0.0; 2] }
    }

    /// An end of an edge between two facets of a curved surface, whose
    /// normals are `normal` and `other`: the edge shader (`vs_edge`) draws
    /// it only where it is a silhouette, one facet turned to the eye and the
    /// other away, as seen from the edge's `middle` (so both ends agree).
    /// It reads `other` in `fg`, `middle` in `bg` and the mark in `tex.w`.
    pub fn seam(pos: [f32; 3], middle: [f32; 3], normal: [f32; 3], other: [f32; 3], color: [f32; 4]) -> Self {
        let wide = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
        Self { fg: wide(other), bg: wide(middle), tex: [0.0, 0.0, 0.0, 1.0], ..Self::plain(pos, normal, color) }
    }
}

/// GPU-ready triangles and edge lines.
#[derive(Debug, Clone, Default)]
pub struct SceneMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Triangles of transparent solids, drawn last with blending.
    pub transparent_indices: Vec<u32>,
    pub edges: Vec<Vertex>,
    /// Faces drawn without light (hidden line, consistent colors).
    pub unlit: bool,
    /// Faces shaded grey by the light, as displayed (the shaded style: see
    /// [`cc_core::VisualStyle::shaded_grey`]), a cast shadow's grey where
    /// the sun does not reach them; else lit in their colors.
    pub grey: bool,
    /// Where each solid part of the model sits in the buffers, in the order
    /// it was added (see [`SceneMesh::highlight`]); empty for meshes whose
    /// solids the selection does not change (analytical, rebar).
    pub spans: Vec<Span>,
}

/// The ranges of one solid part (see [`Tessellation`]) in a [`SceneMesh`]'s
/// buffers. A mesh's spans follow one another: each range starts where the
/// previous span's ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub element: ElementId,
    /// The part's index in the tessellation the mesh was built from.
    part: usize,
    pub vertices: Range<u32>,
    pub indices: Range<u32>,
    pub transparent: Range<u32>,
    pub edges: Range<u32>,
}

/// The ranges of a base mesh's buffers to draw (the others are covered by a
/// [`Highlight`]): index ranges of its opaque and transparent triangles and
/// vertex ranges of its edges. Ranges may run past the buffers' ends.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shown {
    pub indices: Vec<Range<u32>>,
    pub transparent: Vec<Range<u32>>,
    pub edges: Vec<Range<u32>>,
}

/// The selection drawn over a base mesh built without it: the selected
/// solids' faces (opaque, untextured, in the selection color) and edges, or
/// the selected analytical members or bars, plus the parts of the base mesh
/// to keep (`None`: all of it).
#[derive(Debug, Clone, Default)]
pub struct Highlight {
    /// Opaque triangles (`indices`) and edges; nothing transparent.
    pub mesh: SceneMesh,
    pub shown: Option<Shown>,
}

/// The most runs of selected spans [`SceneMesh::highlight`] takes: each run
/// splits the base mesh's draws in two, so a selection scattered wider than
/// this is better built into the base mesh (see [`hidden_runs`]).
pub const MAX_HIDDEN_RUNS: usize = 512;

/// How many runs of consecutive spans of selected elements `spans` holds:
/// the draws [`SceneMesh::highlight`] would add to each of the base mesh's buffers.
pub fn hidden_runs(spans: &[Span], selected: &BTreeSet<ElementId>) -> usize {
    let mut runs = 0;
    let mut inside = false;
    for span in spans {
        let sel = selected.contains(&span.element);
        if sel && !inside {
            runs += 1;
        }
        inside = sel;
    }
    runs
}

/// The parts of `0..` left once `hidden` (increasing, not overlapping) are taken out.
fn complement(hidden: impl Iterator<Item = Range<u32>>) -> Vec<Range<u32>> {
    let mut shown = Vec::new();
    let mut at = 0;
    for r in hidden {
        if r.start > at {
            shown.push(at..r.start);
        }
        at = at.max(r.end);
    }
    shown.push(at..u32::MAX);
    shown
}

/// The triangles and outline edges of a model's solids, without colors:
/// the costly part of a [`SceneMesh`], kept while the geometry stays, so a
/// new selection or view graphics only repaint it.
#[derive(Debug, Clone, Default)]
pub struct Tessellation {
    solids: Vec<SolidTess>,
}

/// The faces of a solid that show one material: all of them, or those
/// painted with one paint (a painted solid makes several).
#[derive(Debug, Clone)]
struct SolidTess {
    element: ElementId,
    /// The material's colors, surface pattern and transparency.
    look: cc_geom::Look,
    /// The detail levels of the views that show it (a component's piece).
    detail: cc_core::component::DetailLevels,
    /// Position, normal and plane coordinates (meters, see [`plane_axes`]) of each vertex.
    vertices: Vec<([f32; 3], [f32; 3], [f64; 2])>,
    /// Where each face's vertices start in `vertices`.
    faces: Vec<u32>,
    /// Triangles, as indices into `vertices`.
    triangles: Vec<u32>,
    /// Outline segments (pairs of points) with their face's normal.
    edges: Vec<([f32; 3], [f32; 3])>,
    /// The edges between the facets of its curved surfaces, each once: its
    /// ends and the normals of the two facets. One is drawn only as a
    /// silhouette (see [`Vertex::seam`]).
    seams: Vec<[[f32; 3]; 4]>,
}

impl SolidTess {
    /// The vertices of its edge lines in `color`: its outlines, then the
    /// edges that show as silhouettes.
    fn edge_vertices(&self, color: [f32; 4]) -> impl Iterator<Item = Vertex> + '_ {
        let outlines = self.edges.iter().map(move |&(pos, normal)| Vertex::plain(pos, normal, color));
        let seams = self.seams.iter().flat_map(move |&[a, b, normal, other]| {
            let middle = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5];
            [a, b].map(|pos| Vertex::seam(pos, middle, normal, other, color))
        });
        outlines.chain(seams)
    }
}

/// The axes a texture is laid along on a face (moved to `cc_geom`, which the
/// path tracer shares).
pub use cc_geom::plane_axes;

/// What to add to the plane coordinates of each face of `solid` so they run
/// on across the facets of a curved surface (see [`cc_geom::Solid::facet_shifts`]).
fn facet_shifts(solid: &cc_geom::Solid) -> Vec<[f64; 2]> {
    solid.facet_shifts()
}

impl Tessellation {
    pub fn new(geom: &ModelGeometry) -> Self {
        let mut solids = Vec::new();
        for solid in &geom.solids {
            let part = |look: cc_geom::Look| SolidTess {
                element: solid.element,
                look,
                detail: solid.detail,
                vertices: Vec::new(),
                faces: Vec::new(),
                triangles: Vec::new(),
                edges: Vec::new(),
                seams: Vec::new(),
            };
            // One part for the solid's own material, one per paint.
            let mut parts: Vec<SolidTess> = vec![part(solid.look())];
            let mut part_of = vec![0usize; solid.faces.len()];
            for (fi, look) in &solid.painted {
                let at = match parts.iter().position(|p| p.look == *look) {
                    Some(i) => i,
                    None => {
                        parts.push(part(look.clone()));
                        parts.len() - 1
                    }
                };
                if let Some(slot) = part_of.get_mut(*fi) {
                    *slot = at;
                }
            }
            // Curved surfaces shade smoothly across their facets, and their
            // plane coordinates run on from one facet to the next.
            let shifts = facet_shifts(solid);
            // Faces between the slices of one body are inside it.
            let inner = solid.inner_faces();
            for (fi, (face, normals)) in solid.faces.iter().zip(solid.vertex_normals()).enumerate() {
                let Some(s) = part_of.get(fi).and_then(|i| parts.get_mut(*i)) else { continue };
                let (u, v) = plane_axes(face.normal);
                let [du, dv] = shifts.get(fi).copied().unwrap_or_default();
                let start = s.vertices.len() as u32;
                s.faces.push(start);
                s.vertices.extend(
                    face.verts
                        .iter()
                        .zip(normals)
                        .map(|(p, n)| (p.as_vec3().to_array(), n.as_vec3().to_array(), [p.dot(u) + du, p.dot(v) + dv])),
                );
                if inner.get(fi) == Some(&true) {
                    continue;
                }
                for [a, b, c] in face.triangulate() {
                    s.triangles.extend([start + a as u32, start + b as u32, start + c as u32]);
                }
            }
            if let Some(first) = parts.first_mut() {
                for (face, edges) in solid.faces.iter().zip(solid.feature_edges()) {
                    let normal = face.normal.as_vec3().to_array();
                    for (a, b) in edges {
                        first.edges.push((a.as_vec3().to_array(), normal));
                        first.edges.push((b.as_vec3().to_array(), normal));
                    }
                }
                // Each edge between smooth facets is listed by both: keep one.
                for (face, seams) in solid.faces.iter().zip(solid.smooth_edges()) {
                    let n = face.normal;
                    for (a, b, other) in seams {
                        if (n.x, n.y, n.z) < (other.x, other.y, other.z) {
                            let p = |v: glam::DVec3| v.as_vec3().to_array();
                            first.seams.push([p(a), p(b), p(n), p(other)]);
                        }
                    }
                }
            }
            solids.extend(parts.into_iter().filter(|p| !p.triangles.is_empty() || !p.edges.is_empty()));
        }
        Self { solids }
    }
}

impl SceneMesh {
    /// Append `solid`'s triangles in `color` (with its surface pattern and
    /// its material's texture in `textures` when `textured`), to the
    /// blended pass when transparent.
    fn add_faces(&mut self, solid: &SolidTess, color: [f32; 4], textured: bool, textures: &TextureSet) {
        let start = self.vertices.len() as u32;
        let layer = |pattern: cc_core::Pattern, c: [u8; 3]| -> [f32; 4] {
            let kind = cc_core::Pattern::ALL.iter().position(|p| *p == pattern).unwrap_or(0);
            if !textured || kind == 0 {
                return [0.0; 4];
            }
            [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), kind as f32]
        };
        let surface = solid.look.surface;
        let fg = layer(surface.foreground, surface.foreground_color);
        let bg = layer(surface.background, surface.background_color);
        let texture = if textured { solid.look.material.as_deref().and_then(|m| textures.of(m)) } else { None };
        match texture {
            Some((index, t)) => {
                // Per face, whole repeats are taken off so the coordinates stay precise as f32.
                let mut shift = [0.0, 0.0];
                let mut next_face = solid.faces.iter().peekable();
                for (i, &(pos, normal, uv)) in solid.vertices.iter().enumerate() {
                    // Image rows run down from the top: the texture's up is the face's up.
                    let [x, y] = t.map(uv);
                    let y = -y;
                    if next_face.next_if(|s| **s as usize == i).is_some() {
                        shift = [x.floor(), y.floor()];
                    }
                    let tex = [(x - shift[0]) as f32, (y - shift[1]) as f32, index as f32 + 1.0, 0.0];
                    let plane = [uv[0] as f32, uv[1] as f32];
                    self.vertices.push(Vertex { pos, normal, color, fg, bg, tex, plane });
                }
            }
            None => self.vertices.extend(solid.vertices.iter().map(|&(pos, normal, uv)| Vertex {
                pos,
                normal,
                color,
                fg,
                bg,
                tex: [0.0; 4],
                plane: [uv[0] as f32, uv[1] as f32],
            })),
        }
        let target = if color[3] < 0.999 { &mut self.transparent_indices } else { &mut self.indices };
        target.extend(solid.triangles.iter().map(|i| start + i));
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

const SELECTED: [u8; 3] = [70, 150, 255];
/// Edges of selected solids, and selected bars and members.
const SELECTED_EDGE: [f32; 4] = [0.02, 0.15, 0.6, 1.0];

/// View graphics of an element's solids in 3D; `None` keeps the material's.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SolidStyle {
    pub color: Option<[u8; 3]>,
    /// 0 (opaque) to 1.
    pub transparency: Option<f64>,
    /// Faces and edges mixed with white.
    pub halftone: bool,
    /// Edge color.
    pub edges: Option<[u8; 3]>,
}

impl SceneMesh {
    /// The model's solids in their materials' colors, lit (outside any
    /// view's style: thumbnails); `style` gives each element's view
    /// graphics (`None` hides it).
    pub fn build(
        tess: &Tessellation,
        selected: &BTreeSet<ElementId>,
        style: &mut dyn FnMut(ElementId) -> Option<SolidStyle>,
    ) -> Self {
        Self::build_at(tess, selected, style, cc_core::DetailLevel::Medium)
    }

    /// [`SceneMesh::build`] at `detail`: component pieces show at the
    /// levels they are set for.
    pub fn build_at(
        tess: &Tessellation,
        selected: &BTreeSet<ElementId>,
        style: &mut dyn FnMut(ElementId) -> Option<SolidStyle>,
        detail: cc_core::DetailLevel,
    ) -> Self {
        // The consistent colors' faces, lit.
        let mut mesh = Self::build_styled(tess, selected, style, detail, cc_core::VisualStyle::ConsistentColors);
        mesh.unlit = false;
        mesh
    }

    /// The model's solids in a visual style, at `detail`: the wireframe has
    /// only edges, hidden line white unlit faces, shaded white faces
    /// greyed by the light (see [`SceneMesh::grey`]), consistent colors the
    /// faces' colors unlit, textures the materials' appearance colors with
    /// their surface patterns, lit (view graphics' color overrides win over
    /// all of them).
    pub fn build_styled(
        tess: &Tessellation,
        selected: &BTreeSet<ElementId>,
        style: &mut dyn FnMut(ElementId) -> Option<SolidStyle>,
        detail: cc_core::DetailLevel,
        visual: cc_core::VisualStyle,
    ) -> Self {
        Self::build_textured(tess, selected, style, detail, visual, &TextureSet::default())
    }

    /// [`SceneMesh::build_styled`] with the materials' textures: in the textures
    /// style, faces of a material with a texture in `textures` show it (not
    /// where view graphics override the color, under halftone or selected).
    pub fn build_textured(
        tess: &Tessellation,
        selected: &BTreeSet<ElementId>,
        style: &mut dyn FnMut(ElementId) -> Option<SolidStyle>,
        detail: cc_core::DetailLevel,
        visual: cc_core::VisualStyle,
        textures: &TextureSet,
    ) -> Self {
        use cc_core::VisualStyle as V;
        let mut mesh = Self {
            unlit: matches!(visual, V::HiddenLine | V::ConsistentColors),
            grey: visual == V::Shaded,
            ..Self::default()
        };
        let mut styles: std::collections::HashMap<ElementId, Option<SolidStyle>> = Default::default();
        for (part, solid) in tess.solids.iter().enumerate().filter(|(_, s)| s.detail.shows(detail)) {
            let Some(st) = *styles.entry(solid.element).or_insert_with(|| style(solid.element)) else { continue };
            let sel = selected.contains(&solid.element);
            let own = match visual {
                V::HiddenLine | V::Shaded => [255, 255, 255],
                V::Textures => solid.look.appearance,
                _ => solid.look.color,
            };
            let mut base = st.color.unwrap_or(own);
            if st.halftone {
                base = base.map(|v| (f32::from(v) + (255.0 - f32::from(v)) * 0.66).round() as u8);
            }
            let base = if sel { SELECTED } else { base };
            let transparency = st.transparency.unwrap_or(solid.look.transparency).clamp(0.0, 1.0) as f32;
            // Selected solids stay opaque so they read clearly.
            let alpha = if sel { 1.0 } else { (1.0 - transparency).clamp(0.08, 1.0) };
            let color = [srgb_to_linear(base[0]), srgb_to_linear(base[1]), srgb_to_linear(base[2]), alpha];
            let edge_color = match (sel, st.edges) {
                (true, _) => SELECTED_EDGE,
                (false, Some(c)) => [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), 1.0],
                (false, None) if st.halftone => [0.45, 0.45, 0.46, 1.0],
                (false, None) => [0.02, 0.02, 0.025, 1.0],
            };
            let len = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
            let start = (len(mesh.vertices.len()), len(mesh.indices.len()), len(mesh.transparent_indices.len()));
            let edges_start = len(mesh.edges.len());
            if visual.fills() {
                let textured = visual == V::Textures && st.color.is_none() && !st.halftone && !sel;
                mesh.add_faces(solid, color, textured, textures);
            }
            mesh.edges.extend(solid.edge_vertices(edge_color));
            mesh.spans.push(Span {
                element: solid.element,
                part,
                vertices: start.0..len(mesh.vertices.len()),
                indices: start.1..len(mesh.indices.len()),
                transparent: start.2..len(mesh.transparent_indices.len()),
                edges: edges_start..len(mesh.edges.len()),
            });
        }
        mesh
    }

    /// The selection over a mesh built from `tess` with no selection, whose
    /// spans are `spans`: the selected elements' parts as
    /// [`SceneMesh::build_textured`] shows them selected, and the rest of
    /// the base mesh to draw. Together they draw the triangles and edges of
    /// a mesh built with the selection.
    pub fn highlight(tess: &Tessellation, spans: &[Span], selected: &BTreeSet<ElementId>) -> Highlight {
        let mut mesh = Self::default();
        let color = [srgb_to_linear(SELECTED[0]), srgb_to_linear(SELECTED[1]), srgb_to_linear(SELECTED[2]), 1.0];
        let hidden: Vec<&Span> = spans.iter().filter(|s| selected.contains(&s.element)).collect();
        for span in &hidden {
            let Some(solid) = tess.solids.get(span.part) else { continue };
            // Faces only where the base mesh has them (not in the wireframe).
            if !span.indices.is_empty() || !span.transparent.is_empty() {
                mesh.add_faces(solid, color, false, &TextureSet::default());
            }
            mesh.edges.extend(solid.edge_vertices(SELECTED_EDGE));
        }
        let shown = Shown {
            indices: complement(hidden.iter().map(|s| s.indices.clone())),
            transparent: complement(hidden.iter().map(|s| s.transparent.clone())),
            edges: complement(hidden.iter().map(|s| s.edges.clone())),
        };
        Highlight { mesh, shown: Some(shown) }
    }

    /// Every solid as its material shows it.
    pub fn build_plain(tess: &Tessellation, selected: &BTreeSet<ElementId>) -> Self {
        Self::build(tess, selected, &mut |_| Some(SolidStyle::default()))
    }

    /// The reinforcing bars (those of the selected hosts or sets highlighted)
    /// over the model's solids drawn faint.
    pub fn build_rebar(geom: &ModelGeometry, tess: &Tessellation, selected: &BTreeSet<ElementId>) -> Self {
        let mut mesh = Self::faint(tess);
        rebar_lines(geom, selected, false, &mut mesh.edges);
        mesh
    }

    /// The bars [`SceneMesh::build_rebar`] highlights, drawn over a mesh
    /// it built with no selection (the same lines, so they win the depth test).
    pub fn rebar_highlight(geom: &ModelGeometry, selected: &BTreeSet<ElementId>) -> Highlight {
        let mut mesh = Self::default();
        rebar_lines(geom, selected, true, &mut mesh.edges);
        Highlight { mesh, shown: None }
    }

    /// The model's solids drawn faint, with no edges.
    fn faint(tess: &Tessellation) -> Self {
        let mut mesh = Self::default();
        for solid in &tess.solids {
            let base = solid.look.color;
            mesh.add_faces(
                solid,
                [srgb_to_linear(base[0]), srgb_to_linear(base[1]), srgb_to_linear(base[2]), 0.12],
                false,
                &TextureSet::default(),
            );
        }
        mesh
    }

    /// The analytical model: member lines (columns blue, beams red, the
    /// selection highlighted) and node crosses (supports green) over the
    /// model's solids drawn faint.
    pub fn build_analytical(
        tess: &Tessellation,
        model: &cc_geom::analytical::AnalyticalModel,
        selected: &BTreeSet<ElementId>,
    ) -> Self {
        let mut mesh = Self::faint(tess);
        member_lines(model, selected, false, &mut mesh.edges);
        for n in &model.nodes {
            let color = if n.support.is_some() { [0.05, 0.5, 0.1, 1.0] } else { [0.02, 0.02, 0.025, 1.0] };
            let r = 0.08;
            for d in [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z] {
                line(&mut mesh.edges, n.position - d * r, n.position + d * r, color);
            }
        }
        mesh
    }

    /// The members [`SceneMesh::build_analytical`] highlights, drawn over a
    /// mesh it built with no selection (the same lines, so they win the depth test).
    pub fn analytical_highlight(
        model: &cc_geom::analytical::AnalyticalModel,
        selected: &BTreeSet<ElementId>,
    ) -> Highlight {
        let mut mesh = Self::default();
        member_lines(model, selected, true, &mut mesh.edges);
        Highlight { mesh, shown: None }
    }
}

/// A line from `a` to `b` in `color`.
fn line(edges: &mut Vec<Vertex>, a: DVec3, b: DVec3, color: [f32; 4]) {
    let normal = [0.0, 0.0, 1.0];
    edges.push(Vertex::plain(a.as_vec3().to_array(), normal, color));
    edges.push(Vertex::plain(b.as_vec3().to_array(), normal, color));
}

/// The bars of the rebar sets (those of the selected hosts or sets
/// highlighted), or only the highlighted ones.
fn rebar_lines(geom: &ModelGeometry, selected: &BTreeSet<ElementId>, only_selected: bool, edges: &mut Vec<Vertex>) {
    for set in geom.rebar.values() {
        let sel = selected.contains(&set.element) || set.host.is_some_and(|h| selected.contains(&h));
        if only_selected && !sel {
            continue;
        }
        let color = if sel { SELECTED_EDGE } else { [0.55, 0.06, 0.04, 1.0] };
        for bar in &set.bars {
            let n = bar.points.len();
            let segments = if bar.closed { n } else { n.saturating_sub(1) };
            for i in 0..segments {
                let (Some(a), Some(b)) = (bar.points.get(i), bar.points.get((i + 1) % n)) else { continue };
                line(edges, *a, *b, color);
            }
        }
    }
}

/// The analytical members (columns blue, beams red, the selection
/// highlighted), or only the selected ones.
fn member_lines(
    model: &cc_geom::analytical::AnalyticalModel,
    selected: &BTreeSet<ElementId>,
    only_selected: bool,
    edges: &mut Vec<Vertex>,
) {
    for m in &model.members {
        let sel = selected.contains(&m.element);
        if only_selected && !sel {
            continue;
        }
        let color = if sel {
            SELECTED_EDGE
        } else if m.kind == cc_geom::analytical::MemberKind::Column {
            [0.05, 0.25, 0.75, 1.0]
        } else {
            [0.75, 0.08, 0.05, 1.0]
        };
        // An arc member in short chords.
        for w in model.points(m, cc_core::arc::ARC_STEP).windows(2) {
            line(edges, w[0], w[1], color);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A round column, 1 m in radius and 4 m tall, of 24 smooth facets (no ends).
    pub(crate) fn column() -> ModelGeometry {
        use glam::DVec3;
        let at = |k: usize, z: f64| {
            let t = std::f64::consts::TAU * (k as f64 + 0.25) / 24.0;
            DVec3::new(t.cos(), t.sin(), z)
        };
        let mut solid = cc_geom::Solid::new(ElementId(1), cc_core::Category::Column, [200, 200, 200]);
        solid.add_polygons((0..24).map(|k| vec![at(k, 0.0), at(k + 1, 0.0), at(k + 1, 4.0), at(k, 4.0)]));
        solid.smooth_curved(0);
        ModelGeometry { solids: vec![solid], ..Default::default() }
    }

    /// The edges between the facets of a curved surface go to the edge
    /// lines once each, marked as seams (drawn only as silhouettes), after
    /// the outlines; a selection draws them too.
    #[test]
    fn curved_surfaces_carry_their_seams() {
        let tess = Tessellation::new(&column());
        let mesh = SceneMesh::build_plain(&tess, &BTreeSet::new());
        let seams: Vec<&Vertex> = mesh.edges.iter().filter(|v| v.tex[3] > 0.5).collect();
        assert_eq!(seams.len(), 24 * 2);
        // The two rims are outlines.
        assert_eq!(mesh.edges.len() - seams.len(), 24 * 2 * 2);
        for pair in seams.chunks(2) {
            let (a, b) = (pair[0], pair[1]);
            // Both ends decide alike: the same middle and normals, which differ.
            assert_eq!((a.bg, a.fg, a.normal), (b.bg, b.fg, b.normal));
            assert_ne!([a.fg[0], a.fg[1], a.fg[2]], a.normal);
            assert!((a.bg[2] - 2.0).abs() < 1e-6 && a.pos[2] != b.pos[2]);
        }
        let all: BTreeSet<ElementId> = [ElementId(1)].into();
        let highlight = SceneMesh::highlight(&tess, &mesh.spans, &all);
        assert_eq!(highlight.mesh.edges.len(), mesh.edges.len());
    }

    #[test]
    fn transparent_materials_go_to_the_blended_pass() {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let tess = Tessellation::new(&ModelGeometry::build(&project));
        let mesh = SceneMesh::build_plain(&tess, &BTreeSet::new());
        // Window glass (60 % transparent in the template) is blended; walls are not.
        assert!(!mesh.transparent_indices.is_empty());
        assert!(!mesh.indices.is_empty());
        let glass: Vec<&Vertex> =
            mesh.transparent_indices.iter().filter_map(|i| mesh.vertices.get(*i as usize)).collect();
        assert!(glass.iter().all(|v| (v.color[3] - 0.4).abs() < 1e-6));
        // A selected window is drawn opaque.
        let window = project.elements.values().find(|e| matches!(e.kind, cc_core::ElementKind::Window(_))).unwrap().id;
        let sel = SceneMesh::build_plain(&tess, &BTreeSet::from([window]));
        assert!(sel.transparent_indices.len() < mesh.transparent_indices.len());
        // View graphics: a hidden window leaves the mesh, an opaque override takes it out of the blended pass.
        let hidden = SceneMesh::build(&tess, &BTreeSet::new(), &mut |e| (e != window).then(SolidStyle::default));
        assert!(hidden.vertices.len() < mesh.vertices.len());
        let opaque = SolidStyle { transparency: Some(0.0), color: Some([255, 0, 0]), ..Default::default() };
        let solid = SceneMesh::build(&tess, &BTreeSet::new(), &mut |e| {
            Some(if e == window { opaque } else { SolidStyle::default() })
        });
        assert!(solid.transparent_indices.len() < mesh.transparent_indices.len());
        assert!(solid.vertices.iter().any(|v| v.color == [1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn visual_styles_shape_the_mesh() {
        use cc_core::VisualStyle as V;
        let mut project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        for m in &mut project.materials {
            m.graphics.surface.foreground = cc_core::Pattern::Brick;
            m.graphics.surface.foreground_color = [90, 20, 10];
        }
        let tess = Tessellation::new(&ModelGeometry::build(&project));
        let build = |visual| {
            SceneMesh::build_styled(
                &tess,
                &BTreeSet::new(),
                &mut |_| Some(SolidStyle::default()),
                cc_core::DetailLevel::Medium,
                visual,
            )
        };
        // Shaded: white faces greyed by the light, no patterns.
        let shaded = build(V::Shaded);
        assert!(!shaded.unlit && shaded.grey && shaded.vertices.iter().all(|v| v.fg[3] == 0.0));
        assert!(shaded.vertices.iter().all(|v| v.color[..3] == [1.0, 1.0, 1.0]));
        // The wireframe: the same edges, no faces.
        let wire = build(V::Wireframe);
        assert!(wire.indices.is_empty() && wire.transparent_indices.is_empty());
        assert_eq!(wire.edges.len(), shaded.edges.len());
        // Hidden line: white, unlit faces.
        let hidden = build(V::HiddenLine);
        assert!(hidden.unlit && !hidden.grey && hidden.vertices.iter().all(|v| v.color[..3] == [1.0, 1.0, 1.0]));
        assert_eq!(hidden.edges.len(), shaded.edges.len());
        let consistent = build(V::ConsistentColors);
        assert!(consistent.unlit && !consistent.grey);
        // Outside a view: the consistent colors, lit.
        let plain = SceneMesh::build_plain(&tess, &BTreeSet::new());
        assert!(!plain.unlit && !plain.grey);
        assert_eq!(
            drawn(&plain.vertices, plain.indices.iter().copied()),
            drawn(&consistent.vertices, consistent.indices.iter().copied())
        );
        // Textures: lit, the brick pattern (kind 8) in its color on the faces of
        // solids made of a material (most of them; door frames are not).
        let textured = build(V::Textures);
        let brick = cc_core::Pattern::ALL.iter().position(|p| *p == cc_core::Pattern::Brick).unwrap() as f32;
        let bricked: Vec<&Vertex> = textured.vertices.iter().filter(|v| v.fg[3] == brick).collect();
        assert!(!textured.unlit && !textured.grey && bricked.len() * 2 > textured.vertices.len(), "{}", bricked.len());
        assert!(bricked.iter().all(|v| (v.fg[0] - srgb_to_linear(90)).abs() < 1e-6));
    }

    /// A 2 x 2 PNG.
    fn png() -> cc_core::raster::RasterImage {
        let mut bytes = Vec::new();
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 200, 10, 255]));
        img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png).unwrap();
        cc_core::raster::RasterImage { format: cc_core::raster::RasterFormat::Png, width: 2, height: 2, data: bytes }
    }

    #[test]
    fn textures_and_paints_reach_the_vertices() {
        use cc_core::VisualStyle as V;
        let mut project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        // The material of the first wall gets a texture 0.5 m wide.
        let wall = geom.solids.iter().find(|s| s.category == cc_core::Category::Wall).unwrap();
        let material = wall.material.clone().unwrap();
        let m = project.materials.iter_mut().find(|m| m.name == material).unwrap();
        m.appearance.texture = Some(cc_core::material::MaterialTexture::new(png(), 0.5, 0.5));
        // A face of another wall is painted with a second material.
        let paint = project.materials.iter().find(|m| m.name != material).unwrap().clone();
        let (id, face) = geom
            .solids
            .iter()
            .filter(|s| s.category == cc_core::Category::Wall)
            .find_map(|s| Some((s.element, s.faces.iter().find(|f| f.normal.z.abs() < 0.1)?.clone())))
            .unwrap();
        project.elements.get_mut(&id).unwrap().paints.push(cc_core::paint::FacePaint {
            point: face.centroid(),
            normal: face.normal,
            material: paint.name.clone(),
        });
        let geom = ModelGeometry::build(&project);
        let tess = Tessellation::new(&geom);
        let mut textures = TextureSet::default();
        textures.update(&project);
        assert_eq!(textures.len(), 1);
        let build = |visual| {
            SceneMesh::build_textured(
                &tess,
                &BTreeSet::new(),
                &mut |_| Some(SolidStyle::default()),
                cc_core::DetailLevel::Medium,
                visual,
                &textures,
            )
        };
        let textured = build(V::Textures);
        let mapped: Vec<&Vertex> = textured.vertices.iter().filter(|v| v.tex[2] == 1.0).collect();
        assert!(!mapped.is_empty());
        // Coordinates in repeats of 0.5 m, kept near zero per face.
        assert!(mapped.iter().all(|v| v.tex[0].abs() < 1e4 && v.tex[1].abs() < 1e4));
        assert!(mapped.iter().any(|v| v.tex[0].fract().abs() > 1e-3 || v.tex[0].abs() > 1.5));
        // Shaded views do not texture, nor show paints (white faces); in consistent
        // colors the painted face shows its paint's color.
        assert!(build(V::Shaded).vertices.iter().all(|v| v.tex[2] == 0.0 && v.color[..3] == [1.0; 3]));
        let c = paint.shading_color();
        let want = [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2])];
        assert!(build(V::ConsistentColors).vertices.iter().any(|v| v.color[..3] == want));
    }

    /// The vertices of triangles or lines, sorted, to compare what two meshes draw.
    fn drawn(vertices: &[Vertex], indices: impl Iterator<Item = u32>) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> =
            indices.map(|i| bytemuck::bytes_of(&vertices[i as usize]).to_vec()).collect::<Vec<_>>();
        out.sort();
        out
    }

    fn indices_in(all: &[u32], ranges: &[Range<u32>]) -> Vec<u32> {
        let len = all.len() as u32;
        ranges.iter().flat_map(|r| all[r.start.min(len) as usize..r.end.min(len) as usize].iter().copied()).collect()
    }

    #[test]
    fn spans_cover_each_element_exactly() {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let tess = Tessellation::new(&ModelGeometry::build(&project));
        let mesh = SceneMesh::build_plain(&tess, &BTreeSet::new());
        // The spans follow one another and end with the buffers.
        let mut at = (0, 0, 0, 0);
        for s in &mesh.spans {
            assert_eq!((s.vertices.start, s.indices.start, s.transparent.start, s.edges.start), at);
            at = (s.vertices.end, s.indices.end, s.transparent.end, s.edges.end);
            // A span's triangles use its own vertices.
            let own = mesh.indices[s.indices.start as usize..s.indices.end as usize]
                .iter()
                .chain(&mesh.transparent_indices[s.transparent.start as usize..s.transparent.end as usize]);
            assert!(own.into_iter().all(|i| s.vertices.contains(i)));
        }
        let lens = (mesh.vertices.len(), mesh.indices.len(), mesh.transparent_indices.len(), mesh.edges.len());
        assert_eq!((at.0 as usize, at.1 as usize, at.2 as usize, at.3 as usize), lens);
        // Each element's spans hold what a mesh of that element alone holds.
        let elements: BTreeSet<ElementId> = mesh.spans.iter().map(|s| s.element).collect();
        assert!(elements.len() > 5);
        for e in elements {
            let alone = SceneMesh::build(&tess, &BTreeSet::new(), &mut |x| (x == e).then(SolidStyle::default));
            let spans = mesh.spans.iter().filter(|s| s.element == e);
            let count = |f: fn(&Span) -> usize| spans.clone().map(f).sum::<usize>();
            assert_eq!(count(|s| s.indices.len()), alone.indices.len());
            assert_eq!(count(|s| s.transparent.len()), alone.transparent_indices.len());
            assert_eq!(count(|s| s.edges.len()), alone.edges.len());
        }
    }

    #[test]
    fn a_highlight_over_the_base_draws_what_a_selected_mesh_draws() {
        use cc_core::VisualStyle as V;
        let mut project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        let wall = geom.solids.iter().find(|s| s.category == cc_core::Category::Wall).unwrap();
        let material = wall.material.clone().unwrap();
        let m = project.materials.iter_mut().find(|m| m.name == material).unwrap();
        m.appearance.texture = Some(cc_core::material::MaterialTexture::new(png(), 0.5, 0.5));
        m.graphics.surface.foreground = cc_core::Pattern::Brick;
        let geom = ModelGeometry::build(&project);
        let tess = Tessellation::new(&geom);
        let mut textures = TextureSet::default();
        textures.update(&project);
        // A textured wall, a transparent window and a hidden door.
        let first =
            |kind: fn(&cc_core::ElementKind) -> bool| project.elements.values().find(|e| kind(&e.kind)).unwrap().id;
        let window = first(|k| matches!(k, cc_core::ElementKind::Window(_)));
        let door = first(|k| matches!(k, cc_core::ElementKind::Door(_)));
        let selected = BTreeSet::from([wall.element, window, door]);
        for visual in [V::Shaded, V::Textures, V::Wireframe, V::HiddenLine] {
            let build = |sel: &BTreeSet<ElementId>| {
                SceneMesh::build_textured(
                    &tess,
                    sel,
                    &mut |e| (e != door).then(SolidStyle::default),
                    cc_core::DetailLevel::Medium,
                    visual,
                    &textures,
                )
            };
            let (baked, base) = (build(&selected), build(&BTreeSet::new()));
            let before = base.clone();
            let h = SceneMesh::highlight(&tess, &base.spans, &selected);
            let shown = h.shown.clone().unwrap();
            assert!(h.mesh.transparent_indices.is_empty());
            // Opaque triangles: the base's shown ones plus the highlight's.
            let mut opaque = drawn(&base.vertices, indices_in(&base.indices, &shown.indices).into_iter());
            opaque.extend(drawn(&h.mesh.vertices, h.mesh.indices.iter().copied()));
            opaque.sort();
            assert_eq!(opaque, drawn(&baked.vertices, baked.indices.iter().copied()), "{visual:?}");
            let transparent =
                drawn(&base.vertices, indices_in(&base.transparent_indices, &shown.transparent).into_iter());
            assert_eq!(transparent, drawn(&baked.vertices, baked.transparent_indices.iter().copied()), "{visual:?}");
            let mut edges = drawn(
                &base.edges,
                indices_in(&(0..base.edges.len() as u32).collect::<Vec<_>>(), &shown.edges).into_iter(),
            );
            edges.extend(drawn(&h.mesh.edges, 0..h.mesh.edges.len() as u32));
            edges.sort();
            assert_eq!(edges, drawn(&baked.edges, 0..baked.edges.len() as u32), "{visual:?}");
            assert!(!h.mesh.edges.is_empty());
            // The base mesh is left as it was.
            assert_eq!(base.spans, before.spans);
            assert_eq!(
                drawn(&base.vertices, base.indices.iter().copied()),
                drawn(&before.vertices, before.indices.iter().copied())
            );
        }
        // No selection: the whole base, nothing over it.
        let base = SceneMesh::build_plain(&tess, &BTreeSet::new());
        let h = SceneMesh::highlight(&tess, &base.spans, &BTreeSet::new());
        assert!(h.mesh.vertices.is_empty() && h.mesh.edges.is_empty());
        assert_eq!(h.shown.unwrap().indices, vec![0..u32::MAX]);
    }

    #[test]
    fn line_highlights_are_the_lines_a_selected_mesh_highlights() {
        let project = cc_core::from_ron(include_str!("../../cc_ifc/tests/fixtures/structure.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        let tess = Tessellation::new(&geom);
        let model = cc_geom::analytical::analytical_model(&project, &geom);
        let highlighted = |edges: &[Vertex]| -> Vec<Vec<u8>> {
            let n = edges.len() as u32;
            drawn(edges, (0..n).filter(|i| edges[*i as usize].color == SELECTED_EDGE))
        };
        let member = model.members.first().unwrap().element;
        let sel = BTreeSet::from([member]);
        let h = SceneMesh::analytical_highlight(&model, &sel);
        assert!(h.shown.is_none() && !h.mesh.edges.is_empty());
        assert_eq!(
            drawn(&h.mesh.edges, 0..h.mesh.edges.len() as u32),
            highlighted(&SceneMesh::build_analytical(&tess, &model, &sel).edges)
        );
        let set = geom.rebar.values().next().unwrap().element;
        let sel = BTreeSet::from([set]);
        let h = SceneMesh::rebar_highlight(&geom, &sel);
        assert!(h.shown.is_none() && !h.mesh.edges.is_empty());
        assert_eq!(
            drawn(&h.mesh.edges, 0..h.mesh.edges.len() as u32),
            highlighted(&SceneMesh::build_rebar(&geom, &tess, &sel).edges)
        );
    }

    #[test]
    fn scattered_selections_count_their_runs() {
        let span = |e: u64| Span {
            element: ElementId(e),
            part: 0,
            vertices: 0..0,
            indices: 0..0,
            transparent: 0..0,
            edges: 0..0,
        };
        let spans: Vec<Span> = [1, 1, 2, 3, 1, 4].into_iter().map(span).collect();
        assert_eq!(hidden_runs(&spans, &BTreeSet::new()), 0);
        assert_eq!(hidden_runs(&spans, &BTreeSet::from([ElementId(1)])), 2);
        assert_eq!(hidden_runs(&spans, &BTreeSet::from([ElementId(2), ElementId(3)])), 1);
        assert_eq!(complement([2..4, 4..6, 8..9].into_iter()), vec![0..2, 6..8, 9..u32::MAX]);
    }

    /// Patterns and textures run on across the facets of a curved surface:
    /// two facets give the points of their shared edge the same coordinates,
    /// which follow the surface (not a world axis the normal picks).
    #[test]
    fn plane_coordinates_run_on_across_smooth_facets() {
        use glam::DVec2;
        // Facets of a cylinder of radius 2, 10° apart, across the 45° where the world axes swap.
        let at = |deg: f64, z: f64| DVec3::new(2.0 * deg.to_radians().cos(), 2.0 * deg.to_radians().sin(), z);
        let facet = |a: f64, b: f64, smooth: bool| {
            let mid = ((a + b) / 2.0).to_radians();
            let verts = vec![at(a, 0.0), at(b, 0.0), at(b, 3.0), at(a, 3.0)];
            let mut f = cc_geom::Face::new(verts, DVec3::new(mid.cos(), mid.sin(), 0.0));
            f.smooth = smooth;
            f
        };
        let mut solid = cc_geom::Solid::new(ElementId(1), cc_core::Category::Wall, [200; 3]);
        solid.faces = (0..6).map(|i| facet(20.0 + 10.0 * i as f64, 30.0 + 10.0 * i as f64, true)).collect();
        let coords = |solid: &cc_geom::Solid, fi: usize, vi: usize| {
            let (face, shift) = (&solid.faces[fi], facet_shifts(solid)[fi]);
            let (u, v) = plane_axes(face.normal);
            DVec2::new(face.verts[vi].dot(u) + shift[0], face.verts[vi].dot(v) + shift[1])
        };
        let chord = at(20.0, 0.0).distance(at(30.0, 0.0));
        for fi in 0..5 {
            // The end of one facet is the start of the next, bottom and top.
            assert!(coords(&solid, fi, 1).distance(coords(&solid, fi + 1, 0)) < 1e-9, "facet {fi}");
            assert!(coords(&solid, fi, 2).distance(coords(&solid, fi + 1, 3)) < 1e-9, "facet {fi}");
            // Along the surface at true length, level and up.
            let step = coords(&solid, fi + 1, 1) - coords(&solid, fi, 1);
            assert!((step.x - chord).abs() < 1e-9 && step.y.abs() < 1e-9, "{step}");
        }
        // The vertices carry them.
        let mut geom = ModelGeometry::default();
        geom.solids.push(solid.clone());
        let tess = Tessellation::new(&geom);
        let mesh = SceneMesh::build(&tess, &BTreeSet::new(), &mut |_| Some(SolidStyle::default()));
        let last = mesh.vertices[4 * 5 + 1].plane;
        assert!((last[0] - mesh.vertices[0].plane[0] - 6.0 * chord as f32).abs() < 1e-4, "{last:?}");
        // Flat faces and facets across a sharp corner keep their own coordinates.
        solid.faces[1].smooth = false;
        solid.faces[4] = facet(60.0, 140.0, true);
        let shifts = facet_shifts(&solid);
        assert_eq!((shifts[0], shifts[1], shifts[4], shifts[5]), ([0.0; 2], [0.0; 2], [0.0; 2], [0.0; 2]));
        assert!(shifts[3][0].abs() > 1e-3);
    }

    #[test]
    fn walls_and_floors_lay_textures_level_and_up() {
        // A wall facing south: across to the east (the right seen from the south), up the wall.
        let (u, v) = plane_axes(-DVec3::Y);
        assert!(u.distance(DVec3::X) < 1e-9 && v.distance(DVec3::Z) < 1e-9, "{u} {v}");
        let (u, v) = plane_axes(DVec3::Z);
        assert_eq!((u, v), (DVec3::X, DVec3::Y));
        // A roof slope: level along the eave, up the slope.
        let (u, v) = plane_axes(DVec3::new(0.0, -1.0, 1.0).normalize());
        assert!(u.z.abs() < 1e-9 && v.z > 0.5);
    }
}
