use crate::{Drawing2D, LineStyle};
use cc_core::tag::TagSymbol;
use cc_core::view_mark::{MarkArrow, MarkStyle};
use cc_core::{
    DetailLevel, ElementId, ElementKind, LayerFunction, LevelId, Project, TypeData, ViewId, ViewKind, VisualStyle,
};
use cc_geom::{Column, MeshPlan, ModelGeometry, PlanCover, WallGeom, polygon};
use glam::{DVec2, DVec3};
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

pub(crate) use crate::cut::POCHE;
const CEILING_GRID: f64 = 0.625;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnCut {
    Cut,
    Below,
    None,
}

fn column_state(c: &Column, cut_z: f64) -> ColumnCut {
    if c.is_solid_at(cut_z) {
        ColumnCut::Cut
    } else if c.solid.first().is_some_and(|s| s.0 < cut_z) {
        ColumnCut::Below
    } else {
        ColumnCut::None
    }
}

/// The functions of a wall's layers, in the order of [`WallGeom::layers`]
/// (empty when its type's layers do not match them).
fn layer_functions(project: &Project, g: &WallGeom) -> Vec<LayerFunction> {
    let ty = project.element(g.element).and_then(|e| project.element_type(e.type_id));
    let functions: Vec<LayerFunction> = ty.map(|t| t.layers().iter().map(|l| l.function).collect()).unwrap_or_default();
    if functions.len() == g.layers.len() { functions } else { Vec::new() }
}

/// The subcategory of the line between layers `k` and `k + 1` (#452): the
/// core's when either is structure, else that of the one nearer the core
/// (nearer the middle of the wall when it has none).
fn boundary_function(functions: &[LayerFunction], k: usize) -> Option<LayerFunction> {
    let (a, b) = (*functions.get(k)?, *functions.get(k + 1)?);
    if a == b || a == LayerFunction::Structure {
        return Some(a);
    }
    if b == LayerFunction::Structure {
        return Some(b);
    }
    let core: Vec<usize> = (0..functions.len()).filter(|i| functions[*i] == LayerFunction::Structure).collect();
    let last = functions.len().saturating_sub(1);
    let distance = |i: usize| match core.iter().map(|c| c.abs_diff(i)).min() {
        Some(d) => 2 * d,
        None => (2 * i).abs_diff(last),
    };
    Some(if distance(k + 1) < distance(k) { b } else { a })
}

/// A pen that only says which layer function a line belongs to.
fn layer_pen(function: Option<LayerFunction>) -> Option<crate::Pen> {
    function.map(|f| crate::Pen { layer: Some(f), ..Default::default() })
}

fn style_of(state: ColumnCut) -> Option<LineStyle> {
    match state {
        ColumnCut::Cut => Some(LineStyle::Cut),
        ColumnCut::Below => Some(LineStyle::Projection),
        ColumnCut::None => None,
    }
}

/// Floor plan (`ceiling == false`) or reflected ceiling plan of `level`.
pub fn plan(
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    cut_height: f64,
    ceiling: bool,
    scale: u32,
) -> Drawing2D {
    plan_of(project, geom, level, cut_height, (ceiling, false), scale, (DetailLevel::Medium, VisualStyle::HiddenLine))
}

/// Whether a structural plan shows the element: slabs, the load-bearing
/// walls and columns, beams and foundations.
pub fn is_structural(e: &cc_core::Element) -> bool {
    match &e.kind {
        ElementKind::Wall(w) => w.structural,
        ElementKind::Column(c) => c.structural,
        ElementKind::Floor(_)
        | ElementKind::Beam(_)
        | ElementKind::Foundation(_)
        | ElementKind::Pile(_)
        | ElementKind::Grid(_)
        | ElementKind::SteelBuilding(_)
        | ElementKind::Truss(_)
        | ElementKind::Bracing(_)
        | ElementKind::Connection(_) => true,
        _ => false,
    }
}

/// A plan as [`plan`] draws it, at `detail`; a `structural` plan shows only
/// the structure, with the beams above the cut drawn as seen from below. A
/// floor plan in a visual `style` that fills faces draws the pieces of
/// components (and of swept profiles and connections) as seen from above:
/// their edges only where no face of them lies over them (see
/// [`PlanCover`]); the wireframe sees through them.
pub fn plan_of(
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    cut_height: f64,
    (ceiling, structural): (bool, bool),
    scale: u32,
    drawn: (DetailLevel, VisualStyle),
) -> Drawing2D {
    plan_drawing(project, geom, level, cut_height, (ceiling, structural), (scale, &BTreeMap::new()), drawn)
}

/// [`plan_of`] with the elevation markers moved by `marks` (a plan's
/// [`cc_core::View::mark_offsets`]).
pub(crate) fn plan_drawing(
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    cut_height: f64,
    (ceiling, structural): (bool, bool),
    (scale, marks): (u32, &BTreeMap<ViewId, DVec2>),
    (detail, style): (DetailLevel, VisualStyle),
) -> Drawing2D {
    let shown = |e: &cc_core::Element| !structural || is_structural(e);
    let spacing = crate::cut::spacing(scale);
    // Zoom to fit leaves the elevation and section markers out.
    let mut d = Drawing2D { fit_elements: true, ..Default::default() };
    let level_z = project.level_elevation(level);
    let cut_z = level_z + cut_height;
    // Every piece is known before any is drawn: each may hide the others. A
    // line within a projection line's width (on paper) of the outline of
    // what lies over it merges with that outline.
    let merge = LineStyle::Projection.pen().0 * f64::from(scale) / 1000.0;
    let mut cover = (!ceiling && style.fills()).then(|| PlanCover::new(cut_z).with_margin(merge));
    let connections = match ceiling {
        true => Vec::new(),
        false => connection_pieces(project, geom, level, cut_z, cover.as_mut()),
    };
    let components = match structural {
        true => Vec::new(),
        false => component_pieces(project, geom, level, (cut_z, ceiling, detail), cover.as_mut()),
    };

    for e in project.elements.values().filter(|e| e.level == level && shown(e)) {
        let id = Some(e.id);
        match &e.kind {
            ElementKind::Floor(f) if !ceiling => {
                d.polyline(&f.boundary, true, LineStyle::Projection, id);
                for o in f.all_openings() {
                    d.polyline(o, true, LineStyle::Projection, id);
                }
            }
            ElementKind::Ceiling(c) if ceiling => {
                d.polyline(&c.boundary, true, LineStyle::Projection, id);
                ceiling_grid(&mut d, &c.boundary, e.id);
                for cut in &c.cuts {
                    d.polyline(cut, true, LineStyle::Projection, id);
                }
            }
            ElementKind::Roof(r) => {
                let joined = geom.roof_joins.get(&e.id).filter(|j| j.active());
                // Joined to another roof, or cut by one: its outline as the joins left it
                // (the valleys are drawn with the ridges).
                for &(a, b) in joined.iter().flat_map(|j| &j.outline_edges) {
                    d.line(a, b, LineStyle::Projection, id);
                }
                if joined.is_none() {
                    d.polyline(&cc_geom::roofs::outline(r).points, true, LineStyle::Projection, id);
                }
                if r.overhang > 0.0 || r.edge_overhangs.iter().flatten().any(|o| *o > 0.0) {
                    d.polyline(&r.footprint, true, LineStyle::Hidden, id);
                }
                // Courtyards: the eaves reach over them by the overhang.
                for hole in &r.holes {
                    if joined.is_none() {
                        d.polyline(&polygon::offset(hole, -r.overhang), true, LineStyle::Projection, id);
                    }
                    if r.overhang > 0.0 {
                        d.polyline(hole, true, LineStyle::Hidden, id);
                    }
                }
                for cut in r.cuts.iter().filter(|_| joined.is_none()) {
                    d.polyline(cut, true, LineStyle::Projection, id);
                }
                for &(a, b) in geom.roof_ridges.get(&e.id).into_iter().flatten() {
                    d.line(a, b, LineStyle::Projection, id);
                }
            }
            _ => {}
        }
    }
    if !ceiling && !structural {
        crate::topography::draw(&mut d, project, geom, level, cut_z, scale);
    }
    if !ceiling {
        draw_structure(&mut d, project, geom, level, cut_z, spacing, structural);
        draw_pieces(&mut d, &connections, cut_z, false, cover.as_mut());
    }
    if !structural {
        draw_pieces(&mut d, &components, cut_z, ceiling, cover.as_mut());
    }
    if !structural && !ceiling {
        draw_masses(&mut d, project, geom, level, cut_z);
    }
    draw_shafts(&mut d, project, level, cut_z);
    if !ceiling {
        draw_steel_buildings(&mut d, project, level, &shown);
        draw_trusses(&mut d, project, level, &shown);
    }

    for (e, _) in project.walls().filter(|(e, _)| e.level == level && shown(e)) {
        if let Some(g) = geom.walls.get(&e.id) {
            draw_wall(&mut d, project, g, cut_z, spacing, detail);
            draw_recesses(&mut d, g, (cut_z, ceiling), &|id| project.element(id).is_some_and(shown));
            if !structural {
                draw_openings(&mut d, project, geom, g, (ceiling, cut_z), (detail, scale));
            }
        }
    }
    // Columns over the walls they stand in (issue 276).
    if !ceiling {
        draw_columns(&mut d, project, level, (cut_z, spacing, detail), structural);
    }

    if !ceiling && let Some((lo, hi)) = geom.bounds() {
        // Markers stay outside the grid lines too, which usually extend past the model.
        let grid_ends = project.elements.values().filter_map(|e| match &e.kind {
            ElementKind::Grid(g) => Some([g.start, g.end]),
            _ => None,
        });
        let (lo, hi) = grid_ends
            .flatten()
            .fold((DVec2::new(lo.x, lo.y), DVec2::new(hi.x, hi.y)), |(lo, hi), p| (lo.min(p), hi.max(p)));
        for v in project.views.values() {
            // A detail view of an elevation is drawn as a callout in it, not here.
            if let ViewKind::Elevation { direction } = v.kind
                && !v.is_detail()
            {
                let offset = marks.get(&v.id).copied().filter(|o| o.is_finite()).unwrap_or(DVec2::ZERO);
                elevation_mark(&mut d, project, v, (lo, hi), direction.look(), (scale, offset));
            }
        }
    }
    if !ceiling {
        crate::interior::marks(&mut d, project, level, scale);
    }
    d
}

/// Distance from the model extents to elevation markers, in meters.
const ELEVATION_MARK_OFFSET: f64 = 1.2;
/// Radius of a head drawn at model size (a style's head of 0), meters.
const MODEL_HEAD_RADIUS: f64 = 0.25;

/// Elevation marker outside the model, `offset` from where it sits: as its
/// mark type says (by default a circle with a filled arrow pointing along
/// `look`, towards the model, labeled with the view name), listed in
/// `d.marks` to be picked and moved.
fn elevation_mark(
    d: &mut Drawing2D,
    project: &Project,
    view: &cc_core::View,
    (lo, hi): (DVec2, DVec2),
    look: DVec2,
    (scale, offset): (u32, DVec2),
) {
    let Some(style) = project.mark_style(view) else { return };
    let center = (lo + hi) * 0.5;
    let half = (hi - lo) * 0.5;
    let reach = (half.x * look.x.abs() + half.y * look.y.abs()) + ELEVATION_MARK_OFFSET;
    let c = center - look * reach + offset;
    let r = head_radius(&style, scale);
    let fields = MarkFields::of(project, view);
    draw_head(d, project, &style, &fields, (c, look, r), (24, scale));
    d.marks.push(crate::MarkHandle { view: view.id, center: c, half: DVec2::splat(1.6 * r), offset });
}

/// Radius of a mark's head at 1:`scale`, meters.
fn head_radius(style: &MarkStyle, scale: u32) -> f64 {
    let k = f64::from(scale.max(1)) / 1000.0;
    if style.head > 0.0 { style.head * 0.5 * k } else { MODEL_HEAD_RADIUS }
}

/// What a view mark's `{Field}`s write: `Number`, `Sheet`, `Name`, `Letter`.
pub(crate) struct MarkFields {
    number: String,
    sheet: String,
    name: String,
}

impl MarkFields {
    pub(crate) fn of(project: &Project, view: &cc_core::View) -> Self {
        let (number, sheet) = section_labels(project, view);
        Self { number, sheet, name: view.name.clone() }
    }

    fn value(&self, key: &str) -> Option<String> {
        Some(match key {
            "Number" => self.number.clone(),
            "Sheet" => self.sheet.clone(),
            "Name" => self.name.clone(),
            "Letter" => self.name.split_whitespace().last().unwrap_or_default().chars().take(4).collect(),
            _ => return None,
        })
    }

    /// `template` with its fields filled in.
    pub(crate) fn fill(&self, template: &str) -> String {
        crate::tag::fill_fields(template, &|k| self.value(k))
    }
}

/// A mark's head at `c` of radius `r` with its arrow pointing along `look`
/// and its texts, as `style` says: the arrow under the head (filled white)
/// or, a narrow one, over it; the outline drawn with `segments` chords
/// when a circle; at 1:`scale`.
fn draw_head(
    d: &mut Drawing2D,
    project: &Project,
    style: &MarkStyle,
    fields: &MarkFields,
    (c, look, r): (DVec2, DVec2, f64),
    (segments, scale): (u32, u32),
) {
    let [ink_r, ink_g, ink_b] = style.color.unwrap_or(LineStyle::Annotation.pen().1);
    let [ar, ag, ab] = style.arrow_color.or(style.color).unwrap_or([ink_r, ink_g, ink_b]);
    let pen = style.color.map(|color| crate::Pen { color: Some(color), ..Default::default() });
    let side = look.perp();
    let wide = vec![c + side * (2.0 * r), c + look * 1.4 * r, c - side * (2.0 * r)];
    match style.arrow {
        MarkArrow::Filled => d.fill(wide, [ar, ag, ab, 255], None),
        MarkArrow::Open => {
            let open = crate::Pen { color: Some([ar, ag, ab]), ..Default::default() };
            d.polyline_pen(&wide, true, LineStyle::Annotation, Some(open), None);
        }
        MarkArrow::Narrow | MarkArrow::None => {}
    }
    let under = matches!(style.arrow, MarkArrow::Filled | MarkArrow::Open);
    let outline: Vec<DVec2> = match style.symbol {
        TagSymbol::Circle => (0..segments)
            .map(|i| {
                let a = f64::from(i) / f64::from(segments) * std::f64::consts::TAU;
                c + DVec2::new(a.cos(), a.sin()) * r
            })
            .collect(),
        TagSymbol::None | TagSymbol::Component(_) => Vec::new(),
        symbol => crate::tag::symbol_outline(symbol, c - DVec2::splat(r), c + DVec2::splat(r)),
    };
    if under {
        d.fill(outline.clone(), [255, 255, 255, 255], None);
    }
    d.polyline_pen(&outline, true, LineStyle::Annotation, pen, None);
    if let TagSymbol::Component(t) = style.symbol {
        crate::tag::draw_symbol_component(d, project, t, (c, 0.0, scale), None, &|text| fields.fill(text));
    }
    if style.split {
        d.line_pen(c - DVec2::X * r, c + DVec2::X * r, LineStyle::Annotation, pen, None);
    }
    let texts = [(&style.top, c + DVec2::Y * r * 0.48), (&style.bottom, c - DVec2::Y * r * 0.5)];
    for (t, at) in texts {
        let text = fields.fill(&t.text);
        if !text.is_empty() {
            d.colored_text(at, text, t.size, (crate::Anchor::Center, 0.0), None, style.color);
        }
    }
    if style.arrow == MarkArrow::Narrow {
        let arrow = vec![c + look * (r * 1.6), c + side * (r * 0.8), c - side * (r * 0.8)];
        d.fill(arrow, [ar, ag, ab, 255], None);
    }
    // The label below and to the right of the head, clear of the arrow.
    let label = fields.fill(&style.label.text);
    if !label.is_empty() {
        let at = c + DVec2::new(r + r * 0.32, -r - r * 0.2);
        d.colored_text(at, label, style.label.size, (crate::Anchor::LeftBottom, 0.0), None, style.color);
    }
}

/// A piece a plan draws: a mesh with what it shows in plan, or the outline
/// of a prism, with its number in the plan's cover when it is in one (see
/// [`PlanCover::begin_piece`]); a 2D symbol (a piece drawn in plans only)
/// is in none: it neither hides nor is hidden.
enum PlanPiece<'a> {
    Mesh { element: ElementId, plan: MeshPlan, piece: Option<usize> },
    Outline { element: ElementId, footprint: &'a [DVec2], z0: f64, z1: f64, piece: Option<usize> },
}

/// A mesh piece and what it shows in plan, its faces and section added to
/// `cover` unless it is a `symbol`.
fn mesh_piece<'a>(
    element: ElementId,
    faces: &[Vec<DVec3>],
    smooth: &[bool],
    (cut_z, symbol): (f64, bool),
    cover: Option<&mut PlanCover>,
) -> PlanPiece<'a> {
    let plan = cc_geom::mesh_plan(faces, smooth, cut_z);
    let piece = cover.filter(|_| !symbol).map(|cover| {
        let piece = cover.begin_piece();
        cover.add_faces(faces);
        cover.add_section(&plan.cut);
        piece
    });
    PlanPiece::Mesh { element, plan, piece }
}

/// Outlines of component pieces marked for plans: in floor plans the pieces
/// starting below the cut (pieces above it dashed), in ceiling plans only
/// the pieces above the cut (light fixtures and the like). Swept profiles
/// are drawn the same way. Those seen from above are added to `cover`.
fn component_pieces<'a>(
    project: &Project,
    geom: &'a ModelGeometry,
    level: LevelId,
    (cut_z, ceiling, detail): (f64, bool, DetailLevel),
    mut cover: Option<&mut PlanCover>,
) -> Vec<PlanPiece<'a>> {
    let mut out = Vec::new();
    for e in project.elements.values().filter(|e| e.level == level) {
        // Swept profiles (baseboards, crown moldings) cut and show like mesh pieces.
        if let ElementKind::Sweep(sw) = &e.kind {
            // Along arcs the curved sides' facets are known (no lines across them); a
            // straight sweep's are found from the faces, as before.
            if let Some(mesh) = cc_geom::profile_sweep::sweep_mesh(project, e, sw) {
                let smooth: &[bool] = if sw.is_curved() { &mesh.smooth } else { &[] };
                out.push(mesh_piece(e.id, &mesh.faces, smooth, (cut_z, false), cover.as_deref_mut()));
            }
            continue;
        }
        if !matches!(e.kind, ElementKind::Component(_)) {
            continue;
        }
        // Floor plans leave electrical devices out: electrical plans draw their symbols.
        if !ceiling && matches!(project.mep_data(e), Some(cc_core::MepData::Device { .. })) {
            continue;
        }
        for part in geom.components.get(&e.id).into_iter().flatten().filter(|p| p.in_plan && p.detail.shows(detail)) {
            let symbol = part.plan_only;
            if !part.faces.is_empty() {
                out.push(mesh_piece(e.id, &part.faces, &part.smooth, (cut_z, symbol), cover.as_deref_mut()));
                continue;
            }
            let piece = cover.as_deref_mut().filter(|_| !symbol && part.z0 < cut_z).map(|cover| {
                let piece = cover.begin_piece();
                cover.add_outline(&part.footprint, part.z1);
                piece
            });
            let (footprint, z0, z1) = (&part.footprint[..], part.z0, part.z1);
            out.push(PlanPiece::Outline { element: e.id, footprint, z0, z1, piece });
        }
    }
    out
}

/// Draw `pieces` as they show in a plan cut at `cut_z`: with a `cover`
/// (floor plans), the edges seen below the cut plane only where no face
/// lies over them.
fn draw_pieces(d: &mut Drawing2D, pieces: &[PlanPiece], cut_z: f64, ceiling: bool, mut cover: Option<&mut PlanCover>) {
    for piece in pieces {
        match piece {
            PlanPiece::Mesh { element, plan, piece } => {
                draw_plan(d, plan, ceiling, *element, cover.as_deref_mut().zip(*piece));
            }
            PlanPiece::Outline { element, footprint, z0, z1, piece } => {
                let above = *z0 >= cut_z;
                let style = match (ceiling, above) {
                    (false, false) | (true, true) => LineStyle::Projection,
                    (false, true) => LineStyle::Hidden,
                    (true, false) => continue,
                };
                let id = Some(*element);
                match cover.as_deref_mut().zip(*piece).filter(|_| !ceiling && !above) {
                    Some((cover, piece)) => {
                        // Its top outline, at the cut plane when it rises through it.
                        let z = z1.min(cut_z);
                        let n = footprint.len();
                        for (i, a) in footprint.iter().enumerate() {
                            let Some(b) = footprint.get((i + 1) % n) else { continue };
                            for (p, q) in cover.visible(a.extend(z), b.extend(z), Some(piece)) {
                                d.line(p, q, style, id);
                            }
                        }
                    }
                    None => d.polyline(footprint, true, style, id),
                }
            }
        }
    }
}

/// Conceptual masses in a floor plan: those on its level or cut by its cut
/// plane, as their body cuts and shows (see [`draw_mesh`]), with the mass
/// floor of the level and its area.
fn draw_masses(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, level: LevelId, cut_z: f64) {
    for (id, g) in &geom.masses {
        let own = project.element(*id).is_some_and(|e| e.level == level);
        if !own && !(g.z_range.0 <= cut_z && cut_z <= g.z_range.1) {
            continue;
        }
        draw_mesh(d, &g.faces, &[], cut_z, false, *id);
        if let Some(floor) = g.floors.iter().find(|f| f.level == level) {
            d.polyline(&floor.outline, true, LineStyle::Thin, Some(*id));
            let n = floor.outline.len().max(1) as f64;
            let center = floor.outline.iter().copied().sum::<DVec2>() / n;
            d.prims.push(crate::Prim::Text {
                pos: center,
                text: crate::format::area(floor.area),
                size: 2.5,
                anchor: crate::Anchor::Center,
                angle: 0.0,
                element: Some(*id),
                color: None,
            });
        }
    }
}

/// Connections whose member stands on `level`: their plates and bolts as
/// they cut and show (see [`draw_plan`]), added to `cover`.
fn connection_pieces(
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    cut_z: f64,
    mut cover: Option<&mut PlanCover>,
) -> Vec<PlanPiece<'static>> {
    let mut out = Vec::new();
    for g in geom.connections.values() {
        if project.element(g.member).is_some_and(|m| m.level == level) {
            out.push(mesh_piece(g.element, &g.faces(), &[], (cut_z, false), cover.as_deref_mut()));
        }
    }
    out
}

/// A mesh as it cuts and shows (see [`draw_plan`]), hiding nothing.
fn draw_mesh(d: &mut Drawing2D, faces: &[Vec<DVec3>], smooth: &[bool], cut_z: f64, ceiling: bool, id: ElementId) {
    draw_plan(d, &cc_geom::mesh_plan(faces, smooth, cut_z), ceiling, id, None);
}

/// A mesh piece: its section at the cut plane, the edges seen below it (with
/// a `cover` it is a piece of, where nothing lies over them) and, dashed,
/// those above (in ceiling plans only the edges above).
fn draw_plan(d: &mut Drawing2D, plan: &MeshPlan, ceiling: bool, id: ElementId, cover: Option<(&mut PlanCover, usize)>) {
    let id = Some(id);
    if ceiling {
        plan.above.iter().for_each(|&(a, b)| d.line(a, b, LineStyle::Projection, id));
        return;
    }
    plan.cut.iter().for_each(|&(a, b)| d.line(a, b, LineStyle::Cut, id));
    match cover {
        Some((cover, piece)) => {
            for &(a, b) in &plan.below {
                for (p, q) in cover.visible(a, b, Some(piece)) {
                    d.line(p, q, LineStyle::Projection, id);
                }
            }
        }
        None => plan.below.iter().for_each(|&(a, b)| d.line(a.truncate(), b.truncate(), LineStyle::Projection, id)),
    }
    plan.above.iter().for_each(|&(a, b)| d.line(a, b, LineStyle::Hidden, id));
}

/// Foundations, beams, stairs, ramps and railings of `level` as seen from the
/// cut plane (columns: [`draw_columns`]).
fn draw_structure(
    d: &mut Drawing2D,
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    cut_z: f64,
    spacing: f64,
    structural: bool,
) {
    for e in project.elements.values().filter(|e| e.level == level && (!structural || is_structural(e))) {
        let id = Some(e.id);
        let data = project.element_type(e.type_id).map(|t| &t.data);
        match (&e.kind, data) {
            (ElementKind::Foundation(_), Some(TypeData::Foundation { .. })) => {
                // Foundations lie under the floor: dashed, unless the cut plane passes through them.
                let Some(g) = geom.foundations.get(&e.id) else { continue };
                let (bottom, top) = g.z_range;
                if bottom <= cut_z && cut_z <= top {
                    let material = project.element_type(e.type_id).map_or("", |t| t.data.cut_material());
                    // A ring pile cap fills around its hole.
                    let region = match g.holes.is_empty() {
                        true => Some(g.footprint.clone()),
                        false => cc_geom::polygon::keyhole(&g.footprint, &g.holes),
                    };
                    if let Some(region) = region {
                        crate::cut::cut_fill(d, project, material, region, spacing, id);
                    }
                    d.polyline(&g.footprint, true, LineStyle::Cut, id);
                    g.holes.iter().for_each(|h| d.polyline(h, true, LineStyle::Cut, id));
                } else {
                    d.polyline(&g.footprint, true, LineStyle::Hidden, id);
                    g.holes.iter().for_each(|h| d.polyline(h, true, LineStyle::Hidden, id));
                }
                // The beam on top of a haunched strip: its two faces.
                if let (Some(body), Some(h)) = (g.body(), data.and_then(|t| t.strip_haunch())) {
                    let style = if bottom <= cut_z && cut_z <= top { LineStyle::Projection } else { LineStyle::Hidden };
                    for v in [-h.beam_width / 2.0, h.beam_width / 2.0] {
                        d.polyline(&body.line(v), false, style, id);
                    }
                }
            }
            (ElementKind::Pile(_), Some(TypeData::Pile { .. })) => {
                // Piles lie under the floor: dashed, unless the cut plane passes
                // through them; a center mark on their axis.
                let Some(g) = geom.piles.get(&e.id) else { continue };
                let (bottom, top) = g.z_range;
                let style = if bottom <= cut_z && cut_z <= top { LineStyle::Cut } else { LineStyle::Hidden };
                d.polyline(&g.outline(), true, style, id);
                let (u, arm) = (DVec2::from_angle(g.rotation_deg.to_radians()), g.section.size() * 0.75);
                for axis in [u, u.perp()] {
                    d.line(g.position - axis * arm, g.position + axis * arm, LineStyle::Center, id);
                }
            }
            (ElementKind::Beam(_), Some(TypeData::Beam { .. })) => {
                // The body as framed into columns and other beams.
                let Some(g) = geom.beams.get(&e.id) else { continue };
                let (bottom, top) = g.z_bounds();
                // Beams above the cut plane are dashed (in structural plans, seen
                // from below, they are drawn); cut beams are drawn heavy.
                let style = if bottom > cut_z && structural {
                    LineStyle::Projection
                } else if bottom > cut_z {
                    LineStyle::Hidden
                } else if top >= cut_z {
                    LineStyle::Cut
                } else {
                    LineStyle::Projection
                };
                d.polyline(&g.outline(), true, style, id);
            }
            (ElementKind::Stair(st), Some(TypeData::Stair { .. })) => {
                if let Some(g) = cc_geom::stair_geom(project, e, st) {
                    draw_stair(d, &g, cut_z, e.id);
                }
            }
            (ElementKind::Ramp(r), Some(TypeData::Ramp { .. })) => {
                if let Some(g) = cc_geom::ramp_geom(project, e, r) {
                    draw_ramp(d, &g, cut_z, e.id);
                }
            }
            (ElementKind::Railing(r), Some(TypeData::Railing { .. })) => {
                let path: Vec<DVec2> = cc_geom::railing_path(project, e, r).iter().map(|p| p.truncate()).collect();
                d.polyline(&path, false, LineStyle::Projection, id);
            }
            _ => {}
        }
    }
}

/// Columns of `level` as seen from the cut plane. They are drawn after the
/// walls, so a column standing in or crossing a wall shows whole over the
/// wall's cut; a wall joined with an architectural column still cuts it, as
/// [`crate::joins::merge`] paints the cutting element over the one it cuts.
fn draw_columns(
    d: &mut Drawing2D,
    project: &Project,
    level: LevelId,
    (cut_z, spacing, detail): (f64, f64, DetailLevel),
    structural: bool,
) {
    for e in project.elements.values().filter(|e| e.level == level && (!structural || is_structural(e))) {
        let id = Some(e.id);
        let (ElementKind::Column(c), Some(data @ TypeData::Column { .. })) =
            (&e.kind, project.element_type(e.type_id).map(|t| &t.data))
        else {
            continue;
        };
        let Some(section) = cc_geom::section::detailed(project, data, detail) else { continue };
        let (outer, holes) = cc_geom::section::column_body(&section, c.position, c.rotation_deg);
        let (base, top) = project.column_span(e, c);
        let style = if base <= cut_z && cut_z <= top {
            let material = data.cut_material();
            crate::cut::cut_fill_with_holes(d, project, material, &outer, &holes, spacing, id);
            LineStyle::Cut
        } else if base > cut_z {
            LineStyle::Hidden
        } else {
            LineStyle::Projection
        };
        for outline in std::iter::once(&outer).chain(&holes) {
            d.polyline(outline, true, style, id);
        }
    }
}

/// Height of the step numbers of a stair, paper millimeters.
const STAIR_NUMBER_MM: f64 = 2.0;

/// Stair symbol: the treads and landings (dashed beyond the cut), a break
/// line across the first step above the cut plane, the step numbers when
/// the stair asks for them and an arrow along the walking line pointing up,
/// to the cut (or the top).
fn draw_stair(d: &mut Drawing2D, g: &cc_geom::StairGeom, cut_z: f64, element: cc_core::ElementId) {
    let id = Some(element);
    let below = |z: f64| z <= cut_z + 1e-9;
    for st in &g.steps {
        let style = if below(st.z) { LineStyle::Projection } else { LineStyle::Hidden };
        d.polyline(&st.outline, true, style, id);
    }
    if let Some(st) = g.steps.iter().find(|st| !below(st.z)) {
        // Break line: a diagonal with a small zigzag across the step's front.
        let [a, b] = st.front;
        let n = st.outline.len().max(1) as f64;
        let centroid = st.outline.iter().copied().sum::<DVec2>() / n;
        let mut inward = (b - a).perp().normalize_or_zero();
        if inward.dot(centroid - a) < 0.0 {
            inward = -inward;
        }
        let (a, b) = (a - inward * 0.15, b + inward * 0.15);
        let m = (a + b) * 0.5;
        let z = inward * 0.08;
        d.polyline(&[a, m - z, m + z, b], false, LineStyle::Projection, id);
    }
    if g.layout.numbers {
        for (at, n) in g.number_spots() {
            d.tag_text(at, n.to_string(), STAIR_NUMBER_MM, crate::Anchor::Center, 0.0, id);
        }
    }
    // Arrow along the walking line, to the cut plane or the top.
    let min_z = g.base + g.riser * 1.5;
    let mut arrow: Vec<DVec2> = Vec::new();
    for w in g.walkline.windows(2) {
        let (a, b) = (w[0], w[1]);
        if arrow.is_empty() {
            arrow.push(a.truncate());
        }
        let limit = cut_z.max(min_z);
        if b.z <= limit + 1e-9 || (b.z - a.z).abs() < 1e-12 {
            arrow.push(b.truncate());
            continue;
        }
        if a.z < limit {
            let t = (limit - a.z) / (b.z - a.z);
            arrow.push(a.truncate().lerp(b.truncate(), t));
        }
        break;
    }
    arrow.dedup_by(|a, b| a.distance(*b) < 1e-6);
    let (Some(&first), [.., before, end]) = (arrow.first(), arrow.as_slice()) else { return };
    let end = *end;
    d.polyline(&arrow, false, LineStyle::Annotation, id);
    let dir = (end - *before).normalize_or(g.u);
    let head = 0.12;
    d.line(end, end - dir * head + dir.perp() * head * 0.6, LineStyle::Annotation, id);
    d.line(end, end - dir * head - dir.perp() * head * 0.6, LineStyle::Annotation, id);
    let start_dir = arrow.get(1).map_or(g.u, |p| (*p - first).normalize_or(g.u));
    let foot = start_dir.perp() * 0.06;
    d.line(first - foot, first + foot, LineStyle::Annotation, id);
}

/// The part of a polygon with an elevation per vertex at or below `cut`
/// (`below`) or above it, with the elevations interpolated along its edges.
fn clip_at(outline: &[DVec2], z: &[f64], cut: f64, below: bool) -> Vec<DVec2> {
    let inside = |v: f64| if below { v <= cut + 1e-9 } else { v > cut + 1e-9 };
    let n = outline.len().min(z.len());
    let mut out = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b, za, zb) = (outline[i], outline[j], z[i], z[j]);
        if inside(za) {
            out.push(a);
        }
        if inside(za) != inside(zb) && (zb - za).abs() > 1e-12 {
            out.push(a.lerp(b, (cut - za) / (zb - za)));
        }
    }
    out
}

/// Ramp symbol: the flights and landings (dashed above the cut plane, with a
/// break line where a flight passes it), an arrow along the walking line
/// pointing up and the slope written along the first flight.
fn draw_ramp(d: &mut Drawing2D, g: &cc_geom::RampGeom, cut_z: f64, element: cc_core::ElementId) {
    use crate::drawing::Anchor;
    let id = Some(element);
    // Edges of the parts below and above the cut; those two flight parts
    // share (the facets of a helix, the cut itself) are not drawn.
    let mut edges: Vec<(DVec2, DVec2, LineStyle, bool)> = Vec::new();
    for p in &g.pieces {
        let below = clip_at(&p.outline, &p.z, cut_z, true);
        let above = clip_at(&p.outline, &p.z, cut_z, false);
        for (part, style) in [(&below, LineStyle::Projection), (&above, LineStyle::Hidden)] {
            if part.len() >= 3 {
                let n = part.len();
                edges.extend((0..n).map(|i| (part[i], part[(i + 1) % n], style, !p.landing)));
            }
        }
        if below.len() >= 3 && above.len() >= 3 && !p.landing {
            // The cut runs between the two points the parts share.
            let shared: Vec<DVec2> =
                below.iter().copied().filter(|q| above.iter().any(|r| r.distance(*q) < 1e-9)).collect();
            if let [a, b] = shared.as_slice() {
                let m = (*a + *b) * 0.5;
                let z = (*b - *a).perp().normalize_or_zero() * 0.08;
                d.polyline(&[*a, m - z, m + z, *b], false, LineStyle::Projection, id);
            }
        }
    }
    // Edges keyed by their ends rounded to a micrometer, in either direction.
    let point = |p: DVec2| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let key = |a: DVec2, b: DVec2| {
        let (a, b) = (point(a), point(b));
        if a <= b { (a, b) } else { (b, a) }
    };
    let mut flight_edges: std::collections::HashMap<_, usize> = std::collections::HashMap::new();
    for &(a, b, _, flight) in &edges {
        if flight {
            *flight_edges.entry(key(a, b)).or_default() += 1;
        }
    }
    for &(a, b, style, flight) in &edges {
        let shared = flight && flight_edges.get(&key(a, b)).is_some_and(|n| *n > 1);
        if !shared && a.distance(b) > 1e-9 {
            d.line(a, b, style, id);
        }
    }
    // Arrow along the walking line, to the cut plane (or the top).
    let limit = cut_z.max(g.base + 0.05);
    let mut arrow: Vec<DVec2> = Vec::new();
    for w in g.walkline.windows(2) {
        let (a, b) = (w[0], w[1]);
        if arrow.is_empty() {
            arrow.push(a.truncate());
        }
        if b.z <= limit + 1e-9 || (b.z - a.z).abs() < 1e-12 {
            arrow.push(b.truncate());
            continue;
        }
        if a.z < limit {
            arrow.push(a.truncate().lerp(b.truncate(), (limit - a.z) / (b.z - a.z)));
        }
        break;
    }
    arrow.dedup_by(|a, b| a.distance(*b) < 1e-6);
    if let [.., before, end] = arrow.as_slice() {
        let (before, end) = (*before, *end);
        d.polyline(&arrow, false, LineStyle::Annotation, id);
        let dir = (end - before).normalize_or(g.u);
        let head = 0.15;
        d.line(end, end - dir * head + dir.perp() * head * 0.6, LineStyle::Annotation, id);
        d.line(end, end - dir * head - dir.perp() * head * 0.6, LineStyle::Annotation, id);
    }
    // The slope beside the walking line of the first flight, kept upright.
    if let [a, b, ..] = g.walkline.as_slice() {
        let (a, b) = (a.truncate(), b.truncate());
        let dir = (b - a).normalize_or(g.u);
        let mut angle = dir.y.atan2(dir.x);
        if angle > PI / 2.0 + 1e-9 || angle <= -PI / 2.0 {
            angle += if angle > 0.0 { -PI } else { PI };
        }
        let at = a.lerp(b, 0.5) + dir.perp() * (g.width * 0.2);
        d.tag_text(at, crate::format::slope(g.check.slope), 2.5, Anchor::Center, angle, id);
    }
}

/// A wall in plan: cut, its layers with their materials' patterns (at the
/// coarse level one plain region), or seen below or above the cut. Where a
/// layered wall meets another in a T, its core runs on through the other
/// wall's outer layers to its core (layer cleanup).
fn draw_wall(d: &mut Drawing2D, project: &Project, g: &WallGeom, cut_z: f64, spacing: f64, detail: DetailLevel) {
    if let Some(c) = &g.curtain {
        draw_curtain(d, project, g, c, cut_z, spacing, detail);
        return;
    }
    if g.is_banded() {
        draw_banded_wall(d, project, g, cut_z, spacing, detail);
        return;
    }
    let id = Some(g.element);
    let h = g.half;
    let functions = layer_functions(project, g);
    // A wall with an elevation profile is cut only where the plane passes through it.
    let columns = g.cut_columns(cut_z);
    let states: Vec<ColumnCut> = columns.iter().map(|c| column_state(c, cut_z)).collect();
    let count = columns.len();
    for (i, c) in columns.iter().enumerate() {
        let poly = g.column_polygon(c);
        let state = states[i];
        // Each layer with its material's cut pattern; coarse, one plain region.
        let layers: &[_] = if state == ColumnCut::Cut && detail.shows_layers() { &g.layers } else { &[] };
        if state == ColumnCut::Cut {
            if layers.is_empty() {
                d.cut_fill(poly.to_vec(), POCHE, id);
            }
            for (v0, v1, material) in layers {
                let (a, b) = (g.layer_span(c, i, count, *v0), g.layer_span(c, i, count, *v1));
                for ((a0, a1), (b0, b1)) in g.layer_pieces(*v0, *v1, a, b) {
                    let mut strip = g.path(a0, a1, *v0);
                    strip.extend(g.path(b1, b0, *v1));
                    crate::cut::cut_fill(d, project, material, strip, spacing, id);
                }
            }
            // Interior layer boundaries; the outer faces are drawn below.
            for (k, &(_, v, _)) in layers.iter().take(layers.len().saturating_sub(1)).enumerate() {
                let (u0, u1) = g.layer_span(c, i, count, v);
                let pen = layer_pen(boundary_function(&functions, k));
                for (a, b) in g.boundary_segments(v, u0, u1) {
                    d.polyline_pen(&g.path(a, b, v), false, LineStyle::Thin, pen, id);
                }
            }
        }
        if let Some(style) = style_of(state) {
            // Faces, opened where other walls meet them in a T (and carried on
            // with a core that reaches them).
            let face = |v: f64, plain: (f64, f64)| if layers.is_empty() { plain } else { g.layer_span(c, i, count, v) };
            for (side, (u0, u1)) in [(-1.0, face(-h, c.right)), (1.0, face(h, c.left))] {
                // A cut wall's face is open where a niche is cut too.
                let cut_at = if state == ColumnCut::Cut { cut_z } else { f64::NAN };
                // A cut layered wall's face is its outer layer's.
                let outer = if side > 0.0 { functions.first() } else { functions.last() };
                let pen = layer_pen(outer.copied().filter(|_| !layers.is_empty()));
                for (a, b) in g.face_segments_at(side, u0, u1, cut_at) {
                    d.polyline_pen(&g.path(a, b, side * h), false, style, pen, id);
                }
            }
        }
        if let Some(&next) = states.get(i + 1)
            && next != state
        {
            let style =
                if state == ColumnCut::Cut || next == ColumnCut::Cut { LineStyle::Cut } else { LineStyle::Projection };
            d.line(g.world(c.left.1, -h), g.world(c.left.1, h), style, id);
        }
    }
    let caps = [
        (g.start_joined, columns.first(), states.first(), g.start_ext),
        (g.end_joined, columns.last(), states.last(), g.end_ext),
    ];
    for (joined, col, state, (ul, ur)) in caps {
        if let (false, Some(_), Some(&state)) = (joined, col, state)
            && let Some(style) = style_of(state)
        {
            d.line(g.world(ur, -h), g.world(ul, h), style, id);
        }
    }
}

/// A wall whose layers have extents of their own (see
/// [`cc_core::layer_extent`]) in plan: each band of layers is cut, seen
/// below or left out on its own, as the cut height finds it. A cut band's
/// layers are drawn as a plain wall's (at the coarse level one plain region
/// per band); one wholly below the cut shows its outline; one above it
/// nothing. Where neighbouring bands differ, the line between them takes
/// the stronger state. The layer cleanup at Ts is drawn as for a plain wall:
/// it takes the layers to stand at the cut.
fn draw_banded_wall(d: &mut Drawing2D, project: &Project, g: &WallGeom, cut_z: f64, spacing: f64, detail: DetailLevel) {
    let id = Some(g.element);
    let h = g.half;
    let functions = layer_functions(project, g);
    let bands = g.bands();
    let last = bands.len().saturating_sub(1);
    let columns = g.cut_columns(cut_z);
    let count = columns.len();
    let state_of = |c: &Column, band: &cc_geom::Band| {
        let mid = (c.left.0.max(c.right.0) + c.left.1.min(c.right.1)) * 0.5;
        let heights: Vec<(f64, f64)> =
            g.extent_lines(c, band.extent).iter().map(|(a, b)| (a.at(mid), b.at(mid))).collect();
        if heights.iter().any(|&(a, b)| a <= cut_z && cut_z <= b) {
            ColumnCut::Cut
        } else if heights.first().is_some_and(|s| s.0 < cut_z) {
            ColumnCut::Below
        } else {
            ColumnCut::None
        }
    };
    let states: Vec<Vec<ColumnCut>> = columns.iter().map(|c| bands.iter().map(|b| state_of(c, b)).collect()).collect();
    let stronger = |a: ColumnCut, b: ColumnCut| {
        if a == ColumnCut::Cut || b == ColumnCut::Cut {
            Some(LineStyle::Cut)
        } else if a == ColumnCut::Below || b == ColumnCut::Below {
            Some(LineStyle::Projection)
        } else {
            None
        }
    };
    let layered = detail.shows_layers();
    for (i, c) in columns.iter().enumerate() {
        let Some(state) = states.get(i) else { continue };
        let at = |j: usize| state.get(j).copied().unwrap_or(ColumnCut::None);
        // Along the wall, the line `v` across it: carried on with a core reaching a T-joined wall where cut layers are drawn.
        let span = |v: f64, cut: bool| if cut && layered { g.layer_span(c, i, count, v) } else { g.span_at(c, v) };
        for (j, band) in bands.iter().enumerate() {
            if at(j) != ColumnCut::Cut {
                continue;
            }
            if !layered {
                let (a, b) = (g.span_at(c, band.v_hi), g.span_at(c, band.v_lo));
                let mut strip = g.path(a.0, a.1, band.v_hi);
                strip.extend(g.path(b.1, b.0, band.v_lo));
                d.cut_fill(strip, POCHE, id);
                continue;
            }
            for k in band.layers.0..=band.layers.1 {
                let Some((v0, v1, material)) = g.layers.get(k) else { continue };
                let (a, b) = (g.layer_span(c, i, count, *v0), g.layer_span(c, i, count, *v1));
                for ((a0, a1), (b0, b1)) in g.layer_pieces(*v0, *v1, a, b) {
                    let mut strip = g.path(a0, a1, *v0);
                    strip.extend(g.path(b1, b0, *v1));
                    crate::cut::cut_fill(d, project, material, strip, spacing, id);
                }
            }
            // The lines between its own layers.
            for k in band.layers.0..band.layers.1 {
                let Some(&(_, v, _)) = g.layers.get(k) else { continue };
                let (u0, u1) = g.layer_span(c, i, count, v);
                let pen = layer_pen(boundary_function(&functions, k));
                for (a, b) in g.boundary_segments(v, u0, u1) {
                    d.polyline_pen(&g.path(a, b, v), false, LineStyle::Thin, pen, id);
                }
            }
        }
        // Between neighbouring bands: a layer line where both are cut, else the stronger state's.
        for (j, band) in bands.iter().enumerate().take(last) {
            let (a, b) = (at(j), at(j + 1));
            let style = match (a, b) {
                (ColumnCut::Cut, ColumnCut::Cut) => layered.then_some(LineStyle::Thin),
                _ => stronger(a, b),
            };
            let Some(style) = style else { continue };
            let v = band.v_lo;
            let (u0, u1) = span(v, a == ColumnCut::Cut || b == ColumnCut::Cut);
            // Between two cut layered bands: the line between their layers.
            let both = a == ColumnCut::Cut && b == ColumnCut::Cut && layered;
            let pen = layer_pen(boundary_function(&functions, band.layers.1).filter(|_| both));
            for (p, q) in g.boundary_segments(v, u0, u1) {
                d.polyline_pen(&g.path(p, q, v), false, style, pen, id);
            }
        }
        // The wall's faces, opened where other walls meet them in a T and where a cut niche is.
        for (side, j) in [(-1.0, last), (1.0, 0)] {
            let Some(style) = style_of(at(j)) else { continue };
            let cut = at(j) == ColumnCut::Cut;
            let (u0, u1) = span(side * h, cut);
            let cut_at = if cut { cut_z } else { f64::NAN };
            // A cut layered face is its outer layer's.
            let outer = if side > 0.0 { functions.first() } else { functions.last() };
            let pen = layer_pen(outer.copied().filter(|_| cut && layered));
            for (a, b) in g.face_segments_at(side, u0, u1, cut_at) {
                d.polyline_pen(&g.path(a, b, side * h), false, style, pen, id);
            }
        }
        // Across the wall where a band's state changes from this column to the next.
        if let Some(next) = states.get(i + 1) {
            for (j, band) in bands.iter().enumerate() {
                let theirs = next.get(j).copied().unwrap_or(ColumnCut::None);
                if theirs == at(j) {
                    continue;
                }
                if let Some(style) = stronger(at(j), theirs) {
                    d.line(g.world(c.left.1, band.v_lo), g.world(c.left.1, band.v_hi), style, id);
                }
            }
        }
    }
    // Each band's piece of a free end.
    let caps = [(g.start_joined, states.first(), g.start_ext), (g.end_joined, states.last(), g.end_ext)];
    let across = |(ul, ur): (f64, f64), v: f64| if h > 1e-12 { ur + (ul - ur) * (v + h) / (2.0 * h) } else { ul };
    for (joined, state, ends) in caps {
        let (false, Some(state)) = (joined, state) else { continue };
        for (j, band) in bands.iter().enumerate() {
            let Some(style) = state.get(j).copied().and_then(style_of) else { continue };
            let (lo, hi) = (across(ends, band.v_lo), across(ends, band.v_hi));
            d.line(g.world(lo, band.v_lo), g.world(hi, band.v_hi), style, id);
        }
    }
}

/// The niches of a wall in plan. One the cut passes through opens the wall's
/// face: the wall's fill is covered from the face to the recess's back, and
/// its sides and back are cut lines; when the wall stands under it, a
/// projection line along the face is its sill seen below the cut. One wholly
/// below the cut is drawn hidden, one wholly above it hidden with longer
/// dashes. A ceiling plan dashes both alike and draws no sill. The lines and
/// the cover belong to the niche, so hiding niches shows the wall whole.
fn draw_recesses(
    d: &mut Drawing2D,
    g: &WallGeom,
    (cut_z, ceiling): (f64, bool),
    shown: &dyn Fn(cc_core::ElementId) -> bool,
) {
    for r in g.recesses.iter().filter(|r| shown(r.element)) {
        let id = Some(r.element);
        let (face, back) = r.across(g.half);
        let cut = r.z0 <= cut_z && cut_z <= r.z1;
        if cut {
            let mut area = g.path(r.u0, r.u1, face);
            area.extend(g.path(r.u1, r.u0, back));
            d.fill(area, PAPER, id);
            // The wall under the niche: its sill's edge is seen below the cut.
            if !ceiling && r.z0 > g.base_z + 1e-6 {
                d.polyline(&g.path(r.u0, r.u1, face), false, LineStyle::Projection, id);
            }
        }
        let style = if cut { LineStyle::Cut } else { LineStyle::Hidden };
        let pen = (!cut && !ceiling && r.z0 > cut_z)
            .then(|| crate::Pen { pattern: Some(cc_core::LinePattern::Dash), ..Default::default() });
        d.line_pen(g.world(r.u0, face), g.world(r.u0, back), style, pen, id);
        d.polyline_pen(&g.path(r.u0, r.u1, back), false, style, pen, id);
        d.line_pen(g.world(r.u1, back), g.world(r.u1, face), style, pen, id);
    }
}

/// The paper showing through a cut niche, over its wall's fill.
const PAPER: [u8; 4] = [255, 255, 255, 255];

/// A curtain wall in plan: its mullions (cut with their material's pattern,
/// or seen below the cut), a line along each glass panel and the outline of
/// solid ones, and the pieces of component panels.
fn draw_curtain(
    d: &mut Drawing2D,
    project: &Project,
    g: &WallGeom,
    c: &cc_geom::curtain::CurtainGeom,
    cut_z: f64,
    spacing: f64,
    detail: DetailLevel,
) {
    use cc_core::curtain::CurtainPanel;
    let id = Some(g.element);
    let state = |z: (f64, f64)| {
        if z.0 <= cut_z && cut_z <= z.1 {
            Some(LineStyle::Cut)
        } else if z.1 < cut_z {
            Some(LineStyle::Projection)
        } else {
            None
        }
    };
    for p in &c.panels {
        let Some(cell) = c.layout.panels.get(p.cell) else { continue };
        match &cell.panel {
            CurtainPanel::Glass { .. } => {
                if let Some(style) = state(p.z) {
                    d.line(p.start, p.end, style, id);
                }
            }
            CurtainPanel::Solid { thickness, material } => {
                let (_, n) = p.frame();
                let t = n * (thickness * 0.5);
                let poly = vec![p.start - t, p.end - t, p.end + t, p.start + t];
                match state(p.z) {
                    Some(LineStyle::Cut) => {
                        crate::cut::cut_fill(d, project, material, poly.clone(), spacing, id);
                        d.polyline(&poly, true, LineStyle::Cut, id);
                    }
                    Some(style) => d.polyline(&poly, true, style, id),
                    None => {}
                }
            }
            CurtainPanel::Empty => {}
            CurtainPanel::Component { .. } => {
                for part in p.parts.iter().filter(|q| q.in_plan && !q.void && q.detail.shows(detail)) {
                    if !part.faces.is_empty() {
                        draw_mesh(d, &part.faces, &part.smooth, cut_z, false, g.element);
                    } else if let Some(style) = state((part.z0, part.z1)) {
                        d.polyline(&part.footprint, true, style, id);
                    }
                }
            }
        }
    }
    for m in &c.mullions {
        let Some(piece) = c.layout.mullions.get(m.piece) else { continue };
        let poly = m.footprint.to_vec();
        match state(m.z) {
            Some(LineStyle::Cut) => {
                crate::cut::cut_fill(d, project, &piece.mullion.material, poly.clone(), spacing, id);
                d.polyline(&poly, true, LineStyle::Cut, id);
            }
            Some(style) => d.polyline(&poly, true, style, id),
            None => {}
        }
    }
}

fn draw_openings(
    d: &mut Drawing2D,
    project: &Project,
    geom: &ModelGeometry,
    g: &WallGeom,
    (ceiling, cut_z): (bool, f64),
    (detail, scale): (DetailLevel, u32),
) {
    for o in &g.openings {
        let Some(e) = project.element(o.element) else { continue };
        let Some(host) = e.as_hosted() else { continue };
        let id = Some(e.id);
        // A door or window component may draw its own symbol: its pieces marked for plans.
        let opening = match project.element_type(e.type_id).map(|t| &t.data) {
            Some(cc_core::TypeData::Component { def, .. }) => def.opening.as_deref(),
            _ => None,
        };
        let own_symbol = opening.is_some_and(|o| o.own_symbol);
        if own_symbol {
            if !ceiling {
                let shown = |p: &&cc_core::component::ComponentPart| p.in_plan && !p.void && p.detail.shows(detail);
                for p in geom.opening_fills.get(&e.id).into_iter().flatten().filter(shown) {
                    d.polyline(&p.footprint, true, LineStyle::Projection, id);
                }
            }
            continue;
        }
        let width = o.u1 - o.u0;
        let side = if host.flip_facing { -1.0 } else { 1.0 };
        match e.kind {
            ElementKind::Door(_) if !ceiling => {
                // A swing per leaf: a double door's two meet in the middle.
                let hinges = opening.map(|o| o.door_operation).unwrap_or_default().hinges();
                let leaf = width / hinges.len().max(1) as f64;
                for &(at, toward) in hinges {
                    let (at, toward) = if host.flip_hand { (1.0 - at, -toward) } else { (at, toward) };
                    let hinge = (o.u0 + at * width, toward);
                    swing(d, g, (o.u0 + o.u1) * 0.5, hinge, (leaf, LineStyle::Thin, scale), side, id);
                }
            }
            ElementKind::Window(_) => {
                let operation = opening.map(|o| o.operation).unwrap_or_default();
                // A window above the cut is seen from below through the wall: dashed, jambs too.
                if !ceiling && o.z0 > cut_z {
                    for v in [-0.01, 0.01] {
                        d.polyline(&g.path(o.u0, o.u1, v), false, LineStyle::Hidden, id);
                    }
                    for u in [o.u0, o.u1] {
                        d.line(g.world(u, -g.half), g.world(u, g.half), LineStyle::Hidden, id);
                    }
                    continue;
                }
                if operation == cc_core::WindowOperation::Sliding {
                    // Two panels on their tracks, overlapping in the middle.
                    let (mid, lap) = ((o.u0 + o.u1) * 0.5, SLIDING_LAP.min(width / 4.0));
                    let (first, second) = if host.flip_hand { (0.02, -0.02) } else { (-0.02, 0.02) };
                    d.polyline(&g.path(o.u0, mid + lap, first), false, LineStyle::Thin, id);
                    d.polyline(&g.path(mid - lap, o.u1, second), false, LineStyle::Thin, id);
                } else {
                    for v in [-0.01, 0.01] {
                        d.polyline(&g.path(o.u0, o.u1, v), false, LineStyle::Thin, id);
                    }
                }
                if ceiling {
                    continue;
                }
                // Side-hung panels, open square to the wall on the side the window opens to.
                let hinges = operation.hinges();
                let leaf = width / hinges.len().max(1) as f64;
                for &(at, toward) in hinges {
                    let (at, toward) = if host.flip_hand { (1.0 - at, -toward) } else { (at, toward) };
                    let hinge = (o.u0 + at * width, toward);
                    swing(d, g, (o.u0 + o.u1) * 0.5, hinge, (leaf, LineStyle::Hidden, scale), side, id);
                }
            }
            _ => {}
        }
    }
}

/// How far the panels of a sliding window overlap past the middle in plan, meters.
const SLIDING_LAP: f64 = 0.05;

/// A swing in plan: a leaf `width` long from the hinge at `hinge_u` on the
/// wall's face on `side`, square to the wall at `center_u` (an arc wall's
/// chord), and the arc (in style `arc`: windows dash it) its free end runs
/// back to the wall `toward` the opening's other end (+1 along the wall).
fn swing(
    d: &mut Drawing2D,
    g: &WallGeom,
    center_u: f64,
    (hinge_u, toward): (f64, f64),
    (width, arc, scale): (f64, LineStyle, u32),
    side: f64,
    id: Option<cc_core::ElementId>,
) {
    let hinge = g.world(hinge_u, side * g.half);
    let (_, dir, normal) = g.frame_at(center_u);
    let leaf_dir = normal * side;
    d.line(hinge, hinge + leaf_dir * width, LineStyle::Projection, id);
    let a0 = leaf_dir.to_angle();
    let mut a1 = (dir * toward).to_angle();
    if a1 - a0 > PI {
        a1 -= TAU;
    } else if a0 - a1 > PI {
        a1 += TAU;
    }
    if arc.dashes().is_empty() {
        d.arc(hinge, width, a0, a1, arc, id);
    } else {
        d.dashed_arc(hinge, width, (a0, a1), arc, id, scale);
    }
}

fn ceiling_grid(d: &mut Drawing2D, boundary: &[DVec2], id: cc_core::ElementId) {
    let Some((lo, hi)) = boundary
        .iter()
        .fold(None, |acc: Option<(DVec2, DVec2)>, p| Some(acc.map_or((*p, *p), |(lo, hi)| (lo.min(*p), hi.max(*p)))))
    else {
        return;
    };
    for (axis, dir, start, end) in [(0, DVec2::Y, lo.x, hi.x), (1, DVec2::X, lo.y, hi.y)] {
        let mut t = start + CEILING_GRID;
        while t < end - 1e-6 {
            let origin = if axis == 0 { DVec2::new(t, lo.y - 1.0) } else { DVec2::new(lo.x - 1.0, t) };
            for (a, b) in polygon::clip_line(boundary, origin, dir) {
                d.line(origin + dir * a, origin + dir * b, LineStyle::Thin, Some(id));
            }
            t += CEILING_GRID;
        }
    }
}

/// The marks of every section in a plan at 1:`scale`, as their mark types
/// say, each listed in `d.sections` to be picked, moved and flipped.
pub(crate) fn section_marks(d: &mut Drawing2D, project: &Project, scale: u32) {
    for v in project.views.values() {
        // A detail view of a section is drawn as a callout in it, not here.
        if let ViewKind::Section { start, end, depth, segments } = v.kind
            && !v.is_detail()
            && let Some(style) = project.mark_style(v)
        {
            let fields = MarkFields::of(project, v);
            let handle = section_mark(d, project, (start, end), (depth, segments), scale, (&style, &fields));
            d.sections.push(crate::SectionHandle { view: v.id, ..handle });
        }
    }
}

/// What a section head shows: the drawing's number on its sheet and the
/// sheet's number, or, while the section is on no sheet, the last word of
/// its name (its letter, as in "Section A") over a dash.
pub(crate) fn section_labels(project: &Project, view: &cc_core::View) -> (String, String) {
    for sheet in project.views.values() {
        if let ViewKind::Sheet { number, viewports, .. } = &sheet.kind
            && let Some(i) = viewports.iter().position(|vp| vp.view == view.id)
        {
            return ((i + 1).to_string(), number.clone());
        }
    }
    let short = view.name.split_whitespace().last().unwrap_or_default();
    (short.chars().take(4).collect(), "-".into())
}

/// The pieces of a section line drawn between its heads: the whole line, or
/// with `segments` only that much of it next to each head (the whole line
/// when they would meet). The line starts at the base of each head's
/// triangle, `head` from its end.
pub fn section_pieces(start: DVec2, end: DVec2, segments: Option<[f64; 2]>, head: f64) -> Vec<(DVec2, DVec2)> {
    let length = start.distance(end);
    let dir = (end - start).normalize_or_zero();
    if length <= 2.0 * head || dir == DVec2::ZERO {
        return Vec::new();
    }
    let (a, b) = (start + dir * head, end - dir * head);
    match segments {
        Some([s0, s1]) if s0 + s1 < length => {
            let mut out = Vec::new();
            if s0 > head {
                out.push((a, start + dir * s0));
            }
            if s1 > head {
                out.push((end - dir * s1, b));
            }
            out
        }
        _ => vec![(a, b)],
    }
}

/// A section mark (NBR 6492, figure A.19): at each end a 12 mm circle split
/// by a horizontal line, the drawing number above it and the sheet number
/// below, on a filled triangle pointing where the section looks; between
/// the heads an extra-wide dash-dot line (split with `segments`), and the
/// far extent dashed. Sizes are on paper at 1:`scale`. That is the default
/// style ([`cc_core::view_mark::MarkKind::default_style`]); a mark type's
/// `style` changes the heads, their texts (`fields`), the ends with one,
/// the arrow, the line's weight and the color.
fn section_mark(
    d: &mut Drawing2D,
    project: &Project,
    (start, end): (DVec2, DVec2),
    (depth, segments): (f64, Option<[f64; 2]>),
    scale: u32,
    (style, fields): (&MarkStyle, &MarkFields),
) -> crate::SectionHandle {
    let r = head_radius(style, scale);
    let dir = (end - start).normalize_or_zero();
    let look = -dir.perp();
    // The triangle's base runs along the line, twice the circle wide.
    let half_base = 2.0 * r;
    let pieces = section_pieces(start, end, segments, half_base);
    let pen = crate::Pen { weight: Some(style.line_weight), color: style.color, ..Default::default() };
    for &(a, b) in &pieces {
        d.prims.push(crate::Prim::Line { a, b, style: LineStyle::Center, element: None, pen: Some(pen) });
    }
    for (c, shown) in [(start, style.heads.at_start()), (end, style.heads.at_end())] {
        if shown {
            draw_head(d, project, style, fields, (c, look, r), (32, scale));
        }
    }
    // Far clip extent, dashed.
    d.line(start + look * depth, end + look * depth, LineStyle::Hidden, None);
    crate::SectionHandle { view: cc_core::ViewId(0), start, end, segments, pieces, radius: r, head: half_base }
}

/// Steel buildings of a plan's level: their column lines' rectangle, thin
/// and dashed, as the building's own lines so it can be picked (its
/// columns and beams draw themselves).
fn draw_steel_buildings(
    d: &mut Drawing2D,
    project: &Project,
    level: LevelId,
    shown: &dyn Fn(&cc_core::Element) -> bool,
) {
    for e in project.elements.values().filter(|e| e.level == level && shown(e)) {
        let ElementKind::SteelBuilding(b) = &e.kind else { continue };
        d.polyline(&b.footprint(), true, LineStyle::Hidden, Some(e.id));
    }
}

/// Trusses of a plan's level: each copy's span line, thin and dashed, as
/// the truss's own line so it can be picked (its members draw themselves).
/// A bracing draws its bay line the same way.
fn draw_trusses(d: &mut Drawing2D, project: &Project, level: LevelId, shown: &dyn Fn(&cc_core::Element) -> bool) {
    for e in project.elements.values().filter(|e| e.level == level && shown(e)) {
        if let ElementKind::Bracing(b) = &e.kind {
            d.line(b.start, b.end, LineStyle::Hidden, Some(e.id));
        }
        let ElementKind::Truss(t) = &e.kind else { continue };
        for [a, b] in t.span_lines() {
            d.line(a, b, LineStyle::Hidden, Some(e.id));
        }
    }
}

/// Shafts in a plan: those on its level and those its cut plane passes
/// through, as their outline crossed by its two diagonals (the usual
/// symbol of an opening through the floors).
fn draw_shafts(d: &mut Drawing2D, project: &Project, level: LevelId, cut_z: f64) {
    for e in project.elements.values() {
        let ElementKind::Shaft(s) = &e.kind else { continue };
        let (base, top) = project.shaft_span(e, s);
        if e.level != level && !(base <= cut_z && cut_z <= top) {
            continue;
        }
        let id = Some(e.id);
        d.polyline(&s.outline, true, LineStyle::Projection, id);
        // Corner to opposite corner: the points farthest apart, then the
        // two farthest from that line on either side.
        let n = s.outline.len();
        let pairs = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j)));
        let Some((i, j)) = pairs.max_by(|a, b| {
            let da = s.outline[a.0].distance(s.outline[a.1]);
            let db = s.outline[b.0].distance(s.outline[b.1]);
            da.total_cmp(&db)
        }) else {
            continue;
        };
        let (a, b) = (s.outline[i], s.outline[j]);
        d.line(a, b, LineStyle::Thin, id);
        let side = |p: DVec2| (b - a).perp_dot(p - a);
        let left = s.outline.iter().copied().max_by(|p, q| side(*p).total_cmp(&side(*q)));
        let right = s.outline.iter().copied().min_by(|p, q| side(*p).total_cmp(&side(*q)));
        if let (Some(l), Some(r)) = (left, right)
            && side(l) > 1e-9
            && side(r) < -1e-9
        {
            d.line(l, r, LineStyle::Thin, id);
        }
    }
}
/// The faces a plan sees beyond its cut plane at `cut_z`, filled in the
/// view's visual style, to lie under the plan's linework: what is below the
/// cut in floor plans, above it in ceiling plans (the shaded, consistent
/// colors and textures styles). Only fills and surface patterns: the plan
/// draws its own edges and cuts.
///
/// The sun's shadows of a floor plan read the site
/// ([`crate::shadow::Catch::Site`]): the whole model casts them, what
/// stands above the cut plane too, on the faces and on the ground, but for
/// what is indoors of them (the spaces the look's shadows carry; none
/// when it carries none). A ceiling plan draws none.
pub fn style_underlay(
    geom: &ModelGeometry,
    cut_z: f64,
    ceiling: bool,
    mut look: crate::Look<'_>,
    patterns: &cc_core::fill_pattern::FillPatterns,
) -> Vec<crate::Prim> {
    look.shadows = look.shadows.filter(|_| !ceiling);
    if let Some(shadows) = look.shadows.as_mut() {
        use crate::shadow::{Catch, OPEN_AIR};
        shadows.catch = match shadows.catch {
            Catch::Site(indoors) | Catch::Outdoors(indoors) => Catch::Site(indoors),
            Catch::All => Catch::Site(&OPEN_AIR),
        };
    }
    let forward = if ceiling { glam::DVec3::Z } else { -glam::DVec3::Z };
    let mut frame = crate::projected::Frame::looking(glam::DVec3::new(0.0, 0.0, cut_z), forward);
    frame.near = Some(0.0);
    let d = crate::projected::render(geom, &frame, None, look, patterns);
    // Looking up, the frame's up is -y: back to plan coordinates.
    let flip = |p: DVec2| if ceiling { DVec2::new(p.x, -p.y) } else { p };
    d.prims
        .into_iter()
        .filter(|p| match p {
            crate::Prim::Fill { cut, .. } => !cut,
            crate::Prim::Line { style, .. } => *style == LineStyle::Thin,
            _ => false,
        })
        .map(|p| p.mapped(flip))
        .collect()
}

/// The wireframe style of a plan: no fills (but the invisible room areas
/// that make rooms pickable) and no hatches, only lines.
pub fn strip_fills(d: &mut Drawing2D) {
    d.prims.retain(|p| match p {
        crate::Prim::Fill { color, .. } => color[3] == 0,
        crate::Prim::Line { style, .. } => *style != LineStyle::Hatch,
        _ => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prim;
    use cc_core::{Category, Document, Element, ElementId, Hosted, Wall};

    /// A steel building draws its footprint dashed as its own lines, so it
    /// can be picked; its members draw themselves.
    #[test]
    fn steel_buildings_draw_their_footprint() {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let host = doc.edit("t", |e| {
            let column = e.project().default_type(Category::Column).unwrap();
            let beam = e.project().default_type(Category::Beam).unwrap();
            let ty = cc_core::ElementTypeId(e.alloc_id());
            e.add_type(cc_core::steel_building::host_type(ty, "Steel building".into()));
            let b = cc_core::steel_building::SteelBuilding::new(column, beam, None);
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id: ty,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::SteelBuilding(Box::new(b)),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        let geom = ModelGeometry::build(&doc.project);
        let d = plan(&doc.project, &geom, level, 1.2, false, 100);
        let lines = d
            .prims
            .iter()
            .filter(|p| matches!(p, Prim::Line { style: LineStyle::Hidden, element: Some(e), .. } if *e == host))
            .count();
        assert_eq!(lines, 4);
        // Mid-bay on a long side: nothing else is there.
        assert_eq!(d.hit_test(DVec2::new(0.0, 3.0), 0.01), Some(host));
    }
    /// A truss draws each copy's span line dashed as its own line, so it can
    /// be picked; its members draw themselves.
    #[test]
    fn trusses_draw_their_span_lines() {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let host = doc.edit("t", |e| {
            let column = e.project().default_type(Category::Column).unwrap();
            let beam = e.project().default_type(Category::Beam).unwrap();
            let ty = cc_core::ElementTypeId(e.alloc_id());
            e.add_type(cc_core::truss::host_type(ty, "Truss".into()));
            let mut t = cc_core::truss::Truss::new(beam, beam, column);
            (t.copies, t.spacing, t.base_offset) = (2, 3.0, 2.0);
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id: ty,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Truss(Box::new(t)),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        let geom = ModelGeometry::build(&doc.project);
        let d = plan(&doc.project, &geom, level, 1.2, false, 100);
        let lines = d
            .prims
            .iter()
            .filter(|p| matches!(p, Prim::Line { style: LineStyle::Hidden, element: Some(e), .. } if *e == host))
            .count();
        assert_eq!(lines, 2);
    }

    /// A connection draws its plate and bolts where its member stands.
    #[test]
    fn connections_draw_on_their_members_level() {
        use cc_core::connection::{Connection, ConnectionKind, MemberEnd};
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let element = move |id, type_id, kind| Element {
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
        };
        let conn = doc.edit("t", |e| {
            let column_t = e.project().default_type(Category::Column).unwrap();
            let ty = cc_core::ElementTypeId(e.alloc_id());
            e.add_type(cc_core::connection::host_type(ty, "Connection".into()));
            let col = ElementId(e.alloc_id());
            let column = cc_core::Column {
                position: glam::DVec2::new(1.0, 1.0),
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: cc_core::WallTop::Unconnected,
                structural: true,
            };
            e.add_element(element(col, column_t, ElementKind::Column(column)));
            let id = ElementId(e.alloc_id());
            let c = Connection::new(col, MemberEnd::Base, ConnectionKind::BasePlate);
            e.add_element(element(id, ty, ElementKind::Connection(c)))
        });
        let geom = ModelGeometry::build(&doc.project);
        let d = plan(&doc.project, &geom, level, 1.2, false, 100);
        let lines = d.prims.iter().filter(|p| matches!(p, Prim::Line { element: Some(e), .. } if *e == conn)).count();
        assert!(lines >= 4, "the plate's outline at least: {lines}");
    }

    /// Layers with extents of their own are cut on their own: an outer layer
    /// that starts 1 m up is not cut below it (no cut fill over it, no cut
    /// line on its face: the core's face beside it is), and an inner one
    /// 30 cm high is seen below a cut above it.
    #[test]
    fn layers_with_extents_are_cut_on_their_own() {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let (wall, half, t0, t2) = doc.edit("t", |e| {
            let base = e.project().default_type(Category::Wall).unwrap();
            let mut ty = e.project().element_type(base).unwrap().clone();
            let TypeData::Wall { layers, .. } = &mut ty.data else { panic!("a wall type") };
            layers[0].base_offset = 1.0;
            layers[2].height = Some(0.3);
            let (t0, t2) = (layers[0].thickness, layers[2].thickness);
            let half = ty.thickness() * 0.5;
            (ty.id, ty.uid, ty.name) = (cc_core::ElementTypeId(e.alloc_id()), Default::default(), "extents".into());
            let type_id = e.add_type(ty);
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Wall(Wall { end: DVec2::new(6.0, 0.0), height: 3.0, ..Default::default() }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            (id, half, t0, t2)
        });
        let geom = ModelGeometry::build(&doc.project);
        let lines_at = |d: &Drawing2D, y: f64, style: LineStyle| {
            d.prims
                .iter()
                .filter(|p| {
                    matches!(p, Prim::Line { a, b, style: s, element: Some(e), .. }
                        if *e == wall && *s == style && (a.y - y).abs() < 1e-9 && (b.y - y).abs() < 1e-9)
                })
                .count()
        };
        let cut_over = |d: &Drawing2D, y: f64| {
            d.prims.iter().any(|p| match p {
                Prim::Fill { pts, cut: true, element: Some(e), .. } if *e == wall => {
                    let (lo, hi) = pts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
                    lo < y && y < hi
                }
                _ => false,
            })
        };
        // Cut at 0.5 m: the outer layer stands above, the inner one below.
        let low = plan(&doc.project, &geom, level, 0.5, false, 100);
        let outer = half - t0 * 0.5;
        assert!(!cut_over(&low, outer), "the outer layer is not cut");
        assert_eq!(lines_at(&low, half, LineStyle::Cut) + lines_at(&low, half, LineStyle::Projection), 0);
        assert!(lines_at(&low, half - t0, LineStyle::Cut) > 0, "the core's face is the cut line");
        assert!(cut_over(&low, 0.0), "the core is cut");
        assert!(!cut_over(&low, -half + t2 * 0.5), "the inner layer is not cut");
        assert!(lines_at(&low, -half, LineStyle::Projection) > 0, "it is seen below the cut");
        // Cut at 1.5 m: the outer layer is cut too.
        let high = plan(&doc.project, &geom, level, 1.5, false, 100);
        assert!(cut_over(&high, outer));
        assert!(lines_at(&high, half, LineStyle::Cut) > 0);
        assert!(lines_at(&high, half - t0, LineStyle::Thin) > 0, "a layer line between cut layers");
        // Coarse: one plain region per cut band.
        let mut coarse = plan_of(
            &doc.project,
            &geom,
            level,
            0.5,
            (false, false),
            100,
            (DetailLevel::Coarse, VisualStyle::HiddenLine),
        );
        coarse.prims.retain(|p| matches!(p, Prim::Fill { cut: true, element: Some(e), .. } if *e == wall));
        assert!(cut_over(&coarse, 0.0) && !cut_over(&coarse, outer));
    }

    /// A wall with a column standing in it, cut by the plan; the column is
    /// joined with the wall when `joined` (the wall cutting an architectural one).
    fn wall_and_column(structural: bool, joined: bool) -> (Drawing2D, ElementId, ElementId) {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let element = |id, type_id, kind| Element {
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
        };
        let (wall, column) = doc.edit("t", |e| {
            let wall_t = e.project().default_type(Category::Wall).unwrap();
            let column_t = e.project().default_type(Category::Column).unwrap();
            let w = ElementId(e.alloc_id());
            let kind = ElementKind::Wall(Wall { end: DVec2::new(5.0, 0.0), height: 3.0, ..Default::default() });
            e.add_element(element(w, wall_t, kind));
            let c = ElementId(e.alloc_id());
            let kind = ElementKind::Column(cc_core::Column {
                position: DVec2::new(2.5, 0.0),
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: Default::default(),
                structural,
            });
            e.add_element(element(c, column_t, kind));
            if joined {
                e.set_joined(w, c, true);
            }
            (w, c)
        });
        let geom = ModelGeometry::build(&doc.project);
        let mut d = plan(&doc.project, &geom, level, 1.2, false, 50);
        crate::joins::merge(&doc.project, &mut d);
        (d, wall, column)
    }

    /// Indices of the cut fills of `id`.
    fn fills_of(d: &Drawing2D, id: ElementId) -> Vec<usize> {
        let fill = |p: &Prim| matches!(p, Prim::Fill { element: Some(e), cut: true, .. } if *e == id);
        d.prims.iter().enumerate().filter(|(_, p)| fill(p)).map(|(i, _)| i).collect()
    }

    /// Issue 276: a column standing in a wall is drawn over the wall's cut,
    /// fills and outline, unless it is an architectural column the wall cuts.
    #[test]
    fn columns_show_over_the_walls_they_stand_in() {
        for structural in [true, false] {
            let (d, wall, column) = wall_and_column(structural, false);
            let wall_fills = fills_of(&d, wall);
            let last_wall_fill = *wall_fills.iter().max().unwrap();
            let cut = |p: &Prim| match p {
                Prim::Fill { element: Some(e), cut: true, .. } => *e == column,
                Prim::Line { style: LineStyle::Cut, element: Some(e), .. } => *e == column,
                _ => false,
            };
            let column_cut: Vec<usize> = d.prims.iter().enumerate().filter(|(_, p)| cut(p)).map(|(i, _)| i).collect();
            assert!(!column_cut.is_empty());
            assert!(
                column_cut.iter().all(|&i| i > last_wall_fill),
                "the column's cut is painted after the wall's (structural: {structural})"
            );
            assert_eq!(d.hit_test(DVec2::new(2.5, 0.0), 0.001), Some(column));
        }
        // A wall joined with an architectural column cuts it: the wall stays on top.
        let (d, wall, column) = wall_and_column(false, true);
        let first_wall_fill = *fills_of(&d, wall).iter().min().unwrap();
        assert!(fills_of(&d, column).iter().all(|&i| i < first_wall_fill));
    }

    #[test]
    fn plan_cuts_wall_and_gaps_at_door() {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let (wall_id, door_id) = doc.edit("t", |e| {
            let wall_t = e.project().default_type(Category::Wall).unwrap();
            let door_t = e.project().default_type(Category::Door).unwrap();
            let w = ElementId(e.alloc_id());
            e.add_element(Element {
                id: w,
                level,
                type_id: wall_t,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Wall(Wall {
                    start: DVec2::ZERO,
                    end: DVec2::new(5.0, 0.0),
                    base_offset: 0.0,
                    height: 3.0,
                    ..Default::default()
                }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            let dr = ElementId(e.alloc_id());
            e.add_element(Element {
                id: dr,
                level,
                type_id: door_t,
                mark: "1".into(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Door(Hosted {
                    host: w,
                    offset: 2.5,
                    sill_height: 0.0,
                    flip_facing: false,
                    flip_hand: false,
                }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            (w, dr)
        });
        let geom = ModelGeometry::build(&doc.project);
        let d = plan(&doc.project, &geom, level, 1.2, false, 50);
        let fills =
            d.prims.iter().filter(|p| matches!(p, Prim::Fill { element: Some(id), .. } if *id == wall_id)).count();
        // One fill per layer on each side of the door.
        let layers = geom.walls[&wall_id].layers.len();
        assert_eq!(fills, 2 * layers, "wall cut split by the door");
        // The template's masonry core is hatched (the hatch belongs to the wall).
        let hatched =
            |p: &Prim| matches!(p, Prim::Line { style: LineStyle::Hatch, element: Some(e), .. } if *e == wall_id);
        assert!(d.prims.iter().any(hatched));
        assert_eq!(d.hit_test(DVec2::new(1.0, 0.0), 0.01), Some(wall_id));
        // On the open leaf, drawn from the hinge at u = 2.1 on the left face.
        assert_eq!(d.hit_test(DVec2::new(2.12, 0.5), 0.05), Some(door_id));
    }

    /// A layered wall meeting another in a T (issue 165): its core runs on
    /// through the other wall's plaster to its core, the plaster opens there
    /// and, the cores being of one masonry, no line parts them. A wall with
    /// no core besides its one layer still stops at the face.
    #[test]
    fn t_joined_cores_run_through_the_plaster() {
        use cc_core::ElementTypeId;
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let wall_t = doc.project.default_type(Category::Wall).unwrap();
        let partition = doc
            .project
            .types
            .values()
            .find(|t| t.category() == Category::Wall && t.layers().len() == 1)
            .map(|t| t.id)
            .unwrap();
        let add = |doc: &mut Document, ty: ElementTypeId, a: (f64, f64), b: (f64, f64)| {
            doc.edit("t", |e| {
                let id = ElementId(e.alloc_id());
                e.add_element(Element {
                    id,
                    level,
                    type_id: ty,
                    mark: String::new(),
                    comments: String::new(),
                    uid: Default::default(),
                    kind: ElementKind::Wall(Wall {
                        start: DVec2::new(a.0, a.1),
                        end: DVec2::new(b.0, b.1),
                        height: 3.0,
                        ..Default::default()
                    }),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                });
                id
            })
        };
        let host = add(&mut doc, wall_t, (0.0, 0.0), (6.0, 0.0));
        let branch = add(&mut doc, wall_t, (3.0, -4.0), (3.0, 0.0));
        let geom = ModelGeometry::build(&doc.project);
        let (h, b) = (&geom.walls[&host], &geom.walls[&branch]);
        // The template's 15 cm wall: 2 cm plaster, 11 cm masonry, 2 cm plaster.
        let (half, core) = (h.half, h.core.unwrap().0);
        assert!((half - 0.075).abs() < 1e-9 && (core - 0.055).abs() < 1e-9, "{half} {core}");
        // The branch's faces stop at the host's face, its core at the host's core.
        assert!((b.end_ext.0 - (4.0 - half)).abs() < 1e-9, "{:?}", b.end_ext);
        let (ul, ur) = b.core_ends[1].unwrap();
        assert!((ul - (4.0 - core)).abs() < 1e-9 && (ur - (4.0 - core)).abs() < 1e-9, "{ul} {ur}");
        let [gap] = h.layer_gaps.as_slice() else { panic!("{:?}", h.layer_gaps) };
        assert!(gap.merge && (gap.v_face + half).abs() < 1e-9 && (gap.v_core + core).abs() < 1e-9, "{gap:?}");
        assert!((gap.at_face.0 - (3.0 - core)).abs() < 1e-9 && (gap.at_face.1 - (3.0 + core)).abs() < 1e-9);

        let d = plan(&doc.project, &geom, level, 1.2, false, 50);
        let fills_at = |p: DVec2| -> Vec<ElementId> {
            d.prims
                .iter()
                .filter_map(|prim| match prim {
                    Prim::Fill { pts, element: Some(e), .. } if cc_geom::polygon::contains(pts, p) => Some(*e),
                    _ => None,
                })
                .collect()
        };
        // In the host's plaster under the branch: the branch's masonry only.
        assert_eq!(fills_at(DVec2::new(3.0, -0.065)), [branch]);
        // Beside the branch's core the host's plaster runs on.
        assert_eq!(fills_at(DVec2::new(3.065, -0.065)), [host]);
        let crosses = |e: ElementId, p: DVec2| {
            d.prims.iter().any(|prim| match prim {
                Prim::Line { a, b, element: Some(id), .. } if *id == e => {
                    let (ab, ap) = (*b - *a, p - *a);
                    let t = ap.dot(ab) / ab.length_squared().max(1e-12);
                    (0.0..=1.0).contains(&t) && (*a + ab * t).distance(p) < 1e-6
                }
                _ => false,
            })
        };
        // No line along the host's core face where the cores meet; the
        // branch's core faces run on to it.
        assert!(!crosses(host, DVec2::new(3.0, -core)));
        assert!(crosses(host, DVec2::new(1.0, -core)));
        assert!(crosses(branch, DVec2::new(3.0 - core, -0.065)));

        // A one-layer partition has no core to run through: it stops at the face.
        let other = add(&mut doc, partition, (1.0, 4.0), (1.0, 0.0));
        let geom = ModelGeometry::build(&doc.project);
        assert!(geom.walls[&other].core.is_none() && geom.walls[&other].core_ends == [None, None]);
        assert_eq!(geom.walls[&host].layer_gaps.len(), 1);
    }

    /// A half-circle wall: plans draw its faces as arcs (pickable as arcs),
    /// elevations draw no seams between the facets of its curved faces.
    #[test]
    fn arc_walls_draw_arcs_without_seams() {
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let wall_id = doc.edit("t", |e| {
            let wall_t = e.project().default_type(Category::Wall).unwrap();
            let w = ElementId(e.alloc_id());
            let wall =
                Wall { start: DVec2::new(4.0, 0.0), end: DVec2::new(-4.0, 0.0), bulge: 1.0, ..Default::default() };
            e.add_element(Element {
                id: w,
                level,
                type_id: wall_t,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Wall(wall),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            w
        });
        let geom = ModelGeometry::build(&doc.project);
        let half = geom.walls[&wall_id].half;
        let d = plan(&doc.project, &geom, level, 1.2, false, 50);
        let cut: Vec<(DVec2, DVec2)> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, style: LineStyle::Cut, element: Some(e), .. } if *e == wall_id => Some((*a, *b)),
                _ => None,
            })
            .collect();
        // Many short chords, each on the inner or outer circle (the end caps join the two).
        let on_circle = |p: DVec2| (p.length() - (4.0 - half)).abs() < 1e-9 || (p.length() - (4.0 + half)).abs() < 1e-9;
        assert!(cut.len() > 40, "{}", cut.len());
        assert!(cut.iter().all(|(a, b)| on_circle(*a) && on_circle(*b) && a.distance(*b) < 0.25));
        assert_eq!(d.hit_test(DVec2::new(0.0, 4.0 + half), 0.01), Some(wall_id));
        // Seen from the north, the outer face shows only its outline.
        let view = doc
            .project
            .views
            .values()
            .find(|v| matches!(v.kind, cc_core::ViewKind::Elevation { direction: cc_core::Direction::North }))
            .unwrap();
        let d = crate::generate(&doc.project, &geom, view).unwrap();
        let vertical = d
            .prims
            .iter()
            .filter(|p| match p {
                Prim::Line { a, b, element: Some(e), .. } if *e == wall_id => (a.x - b.x).abs() < 1e-9,
                _ => false,
            })
            .count();
        assert!(vertical <= 4, "{vertical} vertical lines");
    }

    /// A mesh piece (as imported from IFC) is cut at the plan's cut plane.
    #[test]
    fn plan_cuts_mesh_pieces() {
        use cc_core::{Component, ComponentDef, ComponentItem, ComponentShape, ElementType, ElementTypeId, TypeData};
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        // A 1 m cube standing on the level, rotated 90° and moved to (5, 0).
        let v = |i: u32| [f64::from(i & 1), f64::from((i >> 1) & 1), f64::from((i >> 2) & 1)];
        let faces = vec![
            vec![0, 2, 3, 1],
            vec![4, 5, 7, 6],
            vec![0, 1, 5, 4],
            vec![1, 3, 7, 5],
            vec![3, 2, 6, 7],
            vec![2, 0, 4, 6],
        ];
        let shape = ComponentShape::Mesh { vertices: (0..8).map(v).collect::<Vec<_>>().into(), faces: faces.into() };
        let item = ComponentItem {
            name: "m".into(),
            shape,
            material: String::new(),
            in_plan: true,
            void: false,
            plan_only: false,
            detail: Default::default(),
        };
        let id = doc.edit("t", |e| {
            let type_id = ElementTypeId(e.alloc_id());
            let def = ComponentDef { items: vec![item], ..Default::default() };
            let data = TypeData::Component { component: "Mesh".into(), def };
            e.add_type(ElementType {
                id: type_id,
                name: "Mesh".into(),
                data,
                info: Default::default(),
                uid: Default::default(),
            });
            let id = ElementId(e.alloc_id());
            let kind = ElementKind::Component(Component {
                position: DVec2::new(5.0, 0.0),
                rotation_deg: 90.0,
                base_offset: 0.0,
                overrides: Default::default(),
                properties: Vec::new(),
                system: None,
                ..Default::default()
            });
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
            id
        });
        let geom = ModelGeometry::build(&doc.project);
        assert!((geom.volume(id) - 1.0).abs() < 1e-9);
        let d = plan(&doc.project, &geom, level, 0.5, false, 50);
        let cut: Vec<_> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, style: LineStyle::Cut, element: Some(e), .. } if *e == id => Some((*a, *b)),
                _ => None,
            })
            .collect();
        assert_eq!(cut.len(), 4);
        // Rotated about the insertion point: the square spans x 4..5, y 0..1.
        assert!(cut.iter().all(|(a, b)| {
            [a, b].iter().all(|p| (4.0 - 1e-9..=5.0 + 1e-9).contains(&p.x) && (-1e-9..=1.0 + 1e-9).contains(&p.y))
        }));
    }

    /// A piece of a component for the hidden line tests.
    fn piece(name: &str, shape: cc_core::ComponentShape, plan_only: bool) -> cc_core::ComponentItem {
        cc_core::ComponentItem {
            name: name.into(),
            shape,
            material: String::new(),
            in_plan: true,
            void: false,
            plan_only,
            detail: Default::default(),
        }
    }

    /// A box piece (a prism) from `lo`, `size` big.
    fn box_piece(name: &str, lo: [f64; 3], size: [f64; 3], plan_only: bool) -> cc_core::ComponentItem {
        let s = |v: f64| v.to_string();
        let shape = cc_core::ComponentShape::Box {
            x: s(lo[0]),
            y: s(lo[1]),
            z: s(lo[2]),
            width: s(size[0]),
            depth: s(size[1]),
            height: s(size[2]),
        };
        piece(name, shape, plan_only)
    }

    /// A box piece as a fixed mesh (as imported), between `lo` and `hi`.
    fn mesh_box(name: &str, lo: [f64; 3], hi: [f64; 3]) -> cc_core::ComponentItem {
        let v = |i: u32| {
            let pick = |k: usize, bit: u32| if i & bit == 0 { lo[k] } else { hi[k] };
            [pick(0, 1), pick(1, 2), pick(2, 4)]
        };
        let faces = vec![
            vec![0, 2, 3, 1],
            vec![4, 5, 7, 6],
            vec![0, 1, 5, 4],
            vec![1, 3, 7, 5],
            vec![3, 2, 6, 7],
            vec![2, 0, 4, 6],
        ];
        let shape =
            cc_core::ComponentShape::Mesh { vertices: (0..8).map(v).collect::<Vec<_>>().into(), faces: faces.into() };
        piece(name, shape, false)
    }

    /// A project with one component of `items` placed at the origin of its first level.
    fn placed(items: Vec<cc_core::ComponentItem>) -> (Document, LevelId, ElementId) {
        use cc_core::{ComponentDef, ElementType, ElementTypeId};
        let mut doc = Document::new(Project::from_template());
        let level = doc.project.levels[0].id;
        let id = doc.edit("t", |e| {
            let type_id = ElementTypeId(e.alloc_id());
            let def = ComponentDef { items, ..Default::default() };
            e.add_type(ElementType {
                id: type_id,
                name: "Pieces".into(),
                data: TypeData::Component { component: "Pieces".into(), def },
                info: Default::default(),
                uid: Default::default(),
            });
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Component(Default::default()),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            id
        });
        (doc, level, id)
    }

    /// The lines of `id` in `style` drawn in a plan.
    fn lines_of(d: &Drawing2D, id: ElementId, style: LineStyle) -> Vec<(DVec2, DVec2)> {
        d.prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, style: s, element: Some(e), .. } if *e == id && *s == style => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    fn total(lines: &[(DVec2, DVec2)]) -> f64 {
        lines.iter().map(|(a, b)| a.distance(*b)).sum()
    }

    /// Hidden line plans see the pieces of a component from above: a slab
    /// hides the box under it and the part of another box it lies over;
    /// the wireframe sees through it. A 2D symbol under the slab shows.
    #[test]
    fn hidden_line_plans_hide_what_pieces_cover() {
        let (doc, level, id) = placed(vec![
            // A 2 x 2 m slab from 0.5 to 0.8 m (a prism).
            box_piece("slab", [0.0, 0.0, 0.5], [2.0, 2.0, 0.3], false),
            // Under it: a 0.5 m box, and a box half out of it (meshes).
            mesh_box("under", [0.5, 0.5, 0.0], [1.0, 1.0, 0.3]),
            mesh_box("half", [1.5, 0.5, 0.0], [3.0, 1.0, 0.3]),
            // A symbol under the slab.
            box_piece("symbol", [0.2, 1.5, 0.0], [0.2, 0.2, 0.01], true),
        ]);
        let geom = ModelGeometry::build(&doc.project);
        let draw = |style| plan_of(&doc.project, &geom, level, 1.2, (false, false), 50, (DetailLevel::Medium, style));
        let hidden = lines_of(&draw(VisualStyle::HiddenLine), id, LineStyle::Projection);
        let wire = lines_of(&draw(VisualStyle::Wireframe), id, LineStyle::Projection);
        // The slab's outline (8), the symbol's (0.8) and what shows of the
        // half-covered box (2.5 of its 4 m); the wireframe sees all (14.8).
        assert!((total(&hidden) - 11.3).abs() < 1e-6, "{}: {hidden:?}", total(&hidden));
        assert!((total(&wire) - 14.8).abs() < 1e-6, "{}", total(&wire));
        let inside = |p: DVec2| p.x > 1e-6 && p.x < 2.0 - 1e-6 && p.y > 1e-6 && p.y < 2.0 - 1e-6;
        let symbol = |p: DVec2| (0.2 - 1e-9..=0.4 + 1e-9).contains(&p.x) && (1.5 - 1e-9..=1.7 + 1e-9).contains(&p.y);
        assert!(
            hidden.iter().all(|(a, b)| !inside((*a + *b) * 0.5) || symbol(*a) && symbol(*b)),
            "only the symbol shows under the slab"
        );
        assert!(wire.iter().any(|(a, b)| inside((*a + *b) * 0.5) && !symbol(*a)), "the wireframe sees under it");
        // Shaded plans hide the same lines.
        let shaded = lines_of(&draw(VisualStyle::Shaded), id, LineStyle::Projection);
        assert!((total(&shaded) - total(&hidden)).abs() < 1e-9);
    }

    /// A piece rising through the cut plane hides what is under its
    /// section: a plate through a tall box shows only outside it.
    #[test]
    fn sections_at_the_cut_plane_hide_what_is_under_them() {
        let (doc, level, id) = placed(vec![
            mesh_box("tall", [4.0, 0.0, 0.0], [5.0, 1.0, 2.0]),
            box_piece("plate", [3.5, 0.25, 0.1], [2.0, 0.5, 0.1], false),
        ]);
        let geom = ModelGeometry::build(&doc.project);
        let d = plan(&doc.project, &geom, level, 1.2, false, 50);
        assert!((total(&lines_of(&d, id, LineStyle::Cut)) - 4.0).abs() < 1e-9, "the tall box's section");
        // The plate's 5 m outline less its 2 x 1 m inside the box.
        let seen = lines_of(&d, id, LineStyle::Projection);
        assert!((total(&seen) - 3.0).abs() < 1e-6, "{}: {seen:?}", total(&seen));
    }

    /// Curved surfaces draw their outlines and silhouettes only, never the
    /// edges between their facets, also where a void cut them into pieces:
    /// a dome with a slot through it shows its rim circles and the slot.
    #[test]
    fn curved_pieces_draw_no_facet_edges() {
        use cc_core::{ComponentShape, ProfilePoint, SketchPlane};
        let dome = ComponentShape::Revolution {
            plane: SketchPlane::Front,
            points: vec![
                ProfilePoint::new("0.1", "0"),
                ProfilePoint::arc("0.5", "0", "0.4"),
                ProfilePoint::new("0.1", "0.4"),
            ],
            offset: "0".into(),
            axis: "0".into(),
            angle: "360".into(),
        };
        let slot =
            cc_core::ComponentItem { void: true, ..box_piece("slot", [0.2, -0.05, -0.1], [0.5, 0.1, 1.0], false) };
        let (doc, level, id) = placed(vec![piece("dome", dome, false), slot]);
        let geom = ModelGeometry::build(&doc.project);
        let parts = &geom.components[&id];
        assert_eq!(parts.len(), 1);
        assert!(parts[0].smooth.iter().filter(|s| **s).count() > 20, "a curved surface");
        let d = plan(&doc.project, &geom, level, 1.2, false, 50);
        let seen = lines_of(&d, id, LineStyle::Projection);
        assert!(seen.len() > 20);
        // On a chord of one rim (the next ring of facets is 7.7 mm in from
        // the outer one), along a side of the slot or across its end.
        let on_rim = |a: DVec2, b: DVec2| {
            let near = |p: DVec2, r: f64| (r - 0.002..=r + 1e-6).contains(&p.length());
            [0.5, 0.1].iter().any(|r| near(a, *r) && near(b, *r))
        };
        let side = |p: DVec2| (p.y.abs() - 0.05).abs() < 1e-6 && p.x.abs() > 0.2 - 1e-6;
        let end = |p: DVec2| (p.x.abs() - 0.2).abs() < 1e-6 && p.y.abs() < 0.05 + 1e-6;
        for &(a, b) in &seen {
            let slot = side(a) && side(b) && (a.y - b.y).abs() < 1e-9 || end(a) && end(b);
            assert!(on_rim(a, b) || slot, "a facet edge: {a} {b}");
        }
    }

    /// One curve drawn by two pieces with different chords reads as one
    /// line: a lid of 24 sides on a seat ring turned in 48 shows only its
    /// own outline at 1:10, while the seat alone keeps its rounded outline
    /// (its own surface rising from it hides nothing).
    #[test]
    fn outlines_of_one_curve_merge() {
        use cc_core::{ComponentShape, ProfilePoint, SketchPlane};
        let seat = ComponentShape::Revolution {
            plane: SketchPlane::Front,
            points: vec![
                ProfilePoint::new("0.12", "0.385"),
                ProfilePoint::new("0.20", "0.385"),
                ProfilePoint::arc("0.20", "0.395", "0.01"),
                ProfilePoint::new("0.19", "0.405"),
                ProfilePoint::new("0.12", "0.405"),
            ],
            offset: "0".into(),
            axis: "0".into(),
            angle: "360".into(),
        };
        let lid = ComponentShape::Cylinder {
            x: "0".into(),
            y: "0".into(),
            z: "0.405".into(),
            radius: "0.2".into(),
            height: "0.02".into(),
        };
        let polygon = |r: f64, n: f64| 2.0 * n * r * (std::f64::consts::PI / n).sin();
        let seen = |items| {
            let (doc, level, id) = placed(items);
            let geom = ModelGeometry::build(&doc.project);
            let d = plan_of(
                &doc.project,
                &geom,
                level,
                1.2,
                (false, false),
                10,
                (DetailLevel::Medium, VisualStyle::HiddenLine),
            );
            total(&lines_of(&d, id, LineStyle::Projection))
        };
        // The seat alone: its outer and inner circles.
        let alone = seen(vec![piece("seat", seat.clone(), false)]);
        assert!((alone - polygon(0.2, 48.0) - polygon(0.12, 48.0)).abs() < 1e-6, "{alone}");
        // Closed: the lid's outline only, no slivers of the seat between its chords.
        let closed = seen(vec![piece("seat", seat, false), piece("lid", lid, false)]);
        assert!((closed - polygon(0.2, 24.0)).abs() < 1e-6, "{closed}");
    }
}
