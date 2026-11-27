//! Annotations drawn over views: room, door and window tags, grids and
//! dimensions. Sizes are given in paper millimeters and converted with the
//! view scale, so annotations read the same at any scale.

use crate::drawing::Anchor;
use crate::notes::{self, SymbolPens};
use crate::tag::{self, Align, TagCtx, TagSpot};
use crate::{Drawing2D, LineStyle, Pen, format};
use cc_core::dim_style::DimensionStyle;
use cc_core::grid::{BubbleEnds, GridStyle};
use cc_core::tag::{TagPlacement, TagSymbol, TagTarget};
use cc_core::{Dimension, DimensionKind, ElementKind, LevelId, Project, TypeData, ViewId};
use cc_geom::ModelGeometry;
use glam::DVec2;
use std::collections::BTreeMap;

/// Model meters per paper millimeter at `scale`.
fn mm(scale: u32) -> f64 {
    scale as f64 / 1000.0
}

/// Fill color that is invisible but highlights when the element is selected.
pub const HIGHLIGHT_ONLY: [u8; 4] = [255, 255, 255, 0];
/// Nearly invisible fill that makes text tags clickable.
const PICK_AREA: [u8; 4] = [255, 255, 255, 1];

/// Tags a view moved by hand: offsets in plan by the element they tag.
#[cfg(test)]
pub type TagOffsets = std::collections::BTreeMap<cc_core::ElementId, DVec2>;

/// Tags of rooms and openings, and grids, for a floor or ceiling plan of
/// `level`; `label` translates the keys some tag fields give.
#[allow(clippy::too_many_arguments)]
pub fn plan_annotations(
    d: &mut Drawing2D,
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    ceiling: bool,
    structural: bool,
    view: &cc_core::View,
    label: &dyn Fn(&str) -> String,
) {
    let ctx = TagCtx { project, geom, scale: view.scale, placements: &view.tags, types: &view.tag_types, label };
    for e in project.elements.values() {
        match &e.kind {
            ElementKind::Room(room) if e.level == level && !structural => {
                let boundary = geom.rooms.get(&e.id);
                if let Some(boundary) = boundary {
                    d.fill(boundary.clone(), HIGHLIGHT_ONLY, Some(e.id));
                }
                if let Some(tag) = tag::prepare(&ctx, e, TagTarget::Room) {
                    let spot = TagSpot {
                        point: room.position,
                        anchor: room.position,
                        host_dir: None,
                        align: Align::FirstBaseline,
                    };
                    // Not enclosed: the tag is crossed out.
                    tag::draw(d, &ctx, &tag, spot, boundary.is_none());
                }
            }
            ElementKind::Grid(grid) => {
                let placement = view.tags.get(&e.id).copied().unwrap_or_default();
                grid_line(d, project, e, grid, (grid.start, grid.end), (view.scale, placement));
            }
            _ => {}
        }
    }
    if !ceiling && !structural {
        opening_tags(d, &ctx, level);
    }
}

/// A section size: centimeters, without decimals when whole, or inches
/// with a fraction and the inch mark in imperial units.
fn cm(meters: f64) -> String {
    format::size(meters, format::MetricSize::CmTag)
}

/// A steel or cold-formed profile's name (`IPE 200`), else the section's size in
/// centimeters (inches in imperial units): `15×40`, `Ø30`.
pub fn section_label(project: &Project, data: &TypeData) -> Option<String> {
    use cc_core::ColumnSection as C;
    use cc_core::structural_profile::{ProfileMaterial as M, ProfileShape};
    if let TypeData::Column { profile: Some(id), .. } | TypeData::Beam { profile: Some(id), .. } = data
        && let Some(def) = project.profile(*id)
        && matches!(def.material, M::Steel | M::ColdFormed)
    {
        return Some(def.name.clone());
    }
    // A round profile (the legacy fields of a profile-made type are its box).
    if let Some(def) = project.type_profile_def(data)
        && let ProfileShape::Round { d } | ProfileShape::RoundTube { d, .. } = def.shape
    {
        return Some(format!("Ø{}", cm(d)));
    }
    Some(match data {
        TypeData::Beam { width, height, .. } => format!("{}×{}", cm(*width), cm(*height)),
        TypeData::Column { section: C::Round { diameter } | C::Pipe { diameter, .. }, .. } => {
            format!("Ø{}", cm(*diameter))
        }
        TypeData::Column { section, .. } => {
            let (w, d) = section.size();
            format!("{}×{}", cm(w), cm(d))
        }
        TypeData::Foundation { kind: cc_core::FoundationKind::Pad, width, length, .. } => {
            format!("{}×{}", cm(*width), cm(*length))
        }
        // A pile cap: its size and how many piles it stands on.
        TypeData::Foundation { kind: cc_core::FoundationKind::PileCap, width, length, pile_cap: Some(cap), .. } => {
            format!("{}×{} ({})", cm(*width), cm(*length), cap.layout.count())
        }
        _ => return None,
    })
}

/// Tags of the beams, columns, pad footings and pile caps of `level`: their mark and
/// section (`V1 15×40`). Beam tags run along the beam, beside it; column
/// and footing tags sit above and to the right.
#[cfg(test)]
pub fn structure_tags(
    d: &mut Drawing2D,
    project: &Project,
    geom: &ModelGeometry,
    level: LevelId,
    scale: u32,
    offsets: &TagOffsets,
) {
    let placements = offsets
        .iter()
        .map(|(id, offset)| (*id, cc_core::tag::TagPlacement { offset: *offset, ..Default::default() }))
        .collect();
    let ctx =
        TagCtx { project, geom, scale, placements: &placements, types: &Default::default(), label: &|k| k.to_string() };
    structure_tags_in(d, &ctx, level);
}

/// [`structure_tags`] with a view's tag adjustments and types.
pub fn structure_tags_in(d: &mut Drawing2D, ctx: &TagCtx, level: LevelId) {
    let (project, geom, mm) = (ctx.project, ctx.geom, ctx.mm());
    for e in project.elements.values().filter(|e| e.level == level) {
        let Some(target) = TagTarget::of(e, project) else { continue };
        let spot = match &e.kind {
            ElementKind::Beam(_) => {
                let Some(g) = geom.beams.get(&e.id) else { continue };
                let u = (g.start_ext.0.max(g.start_ext.1) + g.end_ext.0.min(g.end_ext.1)) / 2.0;
                // Readable: text runs left to right or bottom to top.
                let dir = tag::readable(g.dir_at(u));
                let up = dir.perp();
                let p = g.world(u, 0.0) + up * (g.half + 1.0 * mm);
                TagSpot { point: p, anchor: g.world(u, 0.0), host_dir: Some(dir), align: Align::CenterBottom }
            }
            ElementKind::Column(c) if c.structural => {
                let Some(data @ TypeData::Column { .. }) = project.element_type(e.type_id).map(|t| &t.data) else {
                    continue;
                };
                let outline = cc_geom::column_outline(project, data, c.position, c.rotation_deg);
                // A section that fails has no outline: tag the column's point.
                let hi = outline.iter().fold(c.position, |a, p| a.max(*p));
                let p = hi + DVec2::splat(1.0 * mm);
                TagSpot { point: p, anchor: c.position, host_dir: None, align: Align::LeftBottom }
            }
            ElementKind::Foundation(_) => {
                let Some(g) = geom.foundations.get(&e.id) else { continue };
                let lo = g.footprint.iter().fold(DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
                let hi = g.footprint.iter().fold(DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
                let p = DVec2::new(hi.x + mm, lo.y);
                TagSpot { point: p, anchor: (lo + hi) * 0.5, host_dir: None, align: Align::LeftBottom }
            }
            _ => continue,
        };
        if !(spot.point.is_finite() && spot.anchor.is_finite()) {
            continue;
        }
        if let Some(tag) = tag::prepare(ctx, e, target) {
            tag::draw(d, ctx, &tag, spot, false);
        }
    }
}

/// The bubble size (paper mm) and style of the grid `e`'s type, and the
/// ends that get a bubble (the grid's own, else its style's).
pub fn grid_style(project: &Project, e: &cc_core::Element, grid: &cc_core::Grid) -> (f64, GridStyle, BubbleEnds) {
    let (bubble, style) = match project.element_type(e.type_id).map(|t| &t.data) {
        Some(TypeData::Grid { bubble, style }) => (*bubble, *style),
        _ => (8.0, GridStyle::default()),
    };
    (bubble, style, grid.ends.unwrap_or(style.ends))
}

/// The line of the grid `e` from `start` to `end` (in the view's drawing
/// coordinates) at 1:`scale`, with a bubble at the ends its style says,
/// each moved as the view's `placement` says (with an elbow leader back to
/// the line's end) or, when the placement hides them, none.
fn grid_line(
    d: &mut Drawing2D,
    project: &Project,
    e: &cc_core::Element,
    grid: &cc_core::Grid,
    (start, end): (DVec2, DVec2),
    (scale, placement): (u32, TagPlacement),
) {
    let id = Some(e.id);
    let (bubble, style, ends) = grid_style(project, e, grid);
    let dir = (end - start).normalize_or_zero();
    let r = bubble * mm(scale) / 2.0;
    let styled = style.pen.is_some() || style.color.is_some();
    let line_pen = (styled || style.line_pattern.is_some()).then_some(Pen {
        color: style.color,
        weight: style.pen,
        pattern: style.line_pattern,
        halftone: false,
        layer: None,
    });
    let pen = styled.then_some(Pen { color: style.color, weight: style.pen, ..Pen::default() });
    d.line_pen(start, end, LineStyle::Center, line_pen, id);
    if placement.hidden {
        return;
    }
    let bubbles = [
        (true, ends.at_start(), start, -dir, placement.start_offset),
        (false, ends.at_end(), end, dir, placement.offset),
    ];
    for (at_start, shown, tip, out, offset) in bubbles {
        if !shown {
            continue;
        }
        let offset = Some(offset).filter(|o| o.is_finite()).unwrap_or(DVec2::ZERO);
        let center = tip + out * r + offset;
        if offset != DVec2::ZERO {
            // An elbow: along the line, across to the bubble, along again.
            let near = center - out * r;
            let run = (near - tip).dot(out).max(0.0) / 3.0;
            let pts = [tip, tip + out * run, near - out * run, near];
            d.polyline_pen(&pts, false, LineStyle::Annotation, pen, id);
        }
        let circle: Vec<DVec2> =
            (0..32).map(|i| center + DVec2::from_angle(i as f64 / 32.0 * std::f64::consts::TAU) * r).collect();
        match style.symbol {
            TagSymbol::Circle => d.polyline_pen(&circle, true, LineStyle::Annotation, pen, id),
            TagSymbol::None => {}
            TagSymbol::Component(t) => {
                let fill = |text: &str| tag::fill_fields(text, &|k| (k == "Name").then(|| grid.name.clone()));
                tag::draw_symbol_component(d, project, t, (center, 0.0, scale), id, &fill);
            }
            symbol => {
                let pts = tag::symbol_outline(symbol, center - DVec2::splat(r), center + DVec2::splat(r));
                d.polyline_pen(&pts, true, LineStyle::Annotation, pen, id);
            }
        }
        d.fill(circle, PICK_AREA, id);
        if !matches!(style.symbol, TagSymbol::Component(_)) {
            let size = style.text_height(bubble);
            d.colored_text(center, grid.name.as_str(), size, (Anchor::Center, 0.0), id, style.color);
        }
        d.bubbles.push(crate::BubbleHandle { element: e.id, start: at_start, center, radius: r, offset });
    }
}

/// How far a grid may lean from the direction an elevation or a section
/// looks in and still show there: the sine of the angle (about half a degree).
const GRID_LEAN: f64 = 0.01;

/// Grids in an elevation or a section looking through `frame` (issue #332):
/// each grid running square to the view's plane, as a vertical line where it
/// stands with its bubble on top, as tall as what the view draws. A section
/// shows the grids that cross what it sees: between the ends of its line,
/// and between its cut and its depth. They belong to their grid, so the
/// view's graphics hide them with the category or one by one. The grid's
/// style draws them, the line's bottom being its start; the view's
/// `placements` move or hide their bubbles as in plans.
pub fn projected_grids(
    d: &mut Drawing2D,
    project: &Project,
    frame: &crate::projected::Frame,
    scale: u32,
    placements: &BTreeMap<cc_core::ElementId, TagPlacement>,
) {
    let mm = mm(scale);
    let levels = project.levels.iter().map(|l| frame.project(glam::DVec3::new(0.0, 0.0, l.elevation)).y);
    let (lo, hi) = match d.bounds() {
        Some((lo, hi)) => (lo.y, hi.y),
        None => levels.fold((0.0_f64, 3.0_f64), |(lo, hi), y| (lo.min(y), hi.max(y))),
    };
    let (bottom, top) = (lo - 5.0 * mm, hi + 8.0 * mm);
    if !(bottom.is_finite() && top.is_finite()) {
        return;
    }
    for e in project.elements.values() {
        let ElementKind::Grid(grid) = &e.kind else { continue };
        let at = |p: DVec2| glam::DVec3::new(p.x, p.y, 0.0);
        let (a, b) = (frame.project(at(grid.start)).x, frame.project(at(grid.end)).x);
        let length = grid.start.distance(grid.end);
        if !(a.is_finite() && b.is_finite() && length > 1e-9) || (a - b).abs() > length * GRID_LEAN {
            continue;
        }
        let x = (a + b) * 0.5;
        if frame.crop_x.is_some_and(|(x0, x1)| x < x0 - 1e-6 || x > x1 + 1e-6) {
            continue;
        }
        let (near, far) = (frame.depth(at(grid.start)), frame.depth(at(grid.end)));
        let (near, far) = (near.min(far), near.max(far));
        if frame.near.is_some_and(|n| far < n - 1e-6) || frame.far.is_some_and(|f| near > f + 1e-6) {
            continue;
        }
        let placement = placements.get(&e.id).copied().unwrap_or_default();
        grid_line(d, project, e, grid, (DVec2::new(x, bottom), DVec2::new(x, top)), (scale, placement));
    }
}

/// Tags of the doors and windows in the walls of `level`: outside the
/// wall, on the side opposite the door swing.
fn opening_tags(d: &mut Drawing2D, ctx: &TagCtx, level: LevelId) {
    let (project, mm) = (ctx.project, ctx.mm());
    for wall in ctx.geom.walls.values() {
        if project.element(wall.element).is_none_or(|e| e.level != level) {
            continue;
        }
        for o in &wall.openings {
            let Some(e) = project.element(o.element) else { continue };
            let Some(host) = e.as_hosted() else { continue };
            let Some(target @ (TagTarget::Door | TagTarget::Window)) = TagTarget::of(e, project) else { continue };
            let Some(tag) = tag::prepare(ctx, e, target) else { continue };
            let side = if host.flip_facing { 1.0 } else { -1.0 };
            let u = (o.u0 + o.u1) / 2.0;
            // The tag's edge 2 mm off the wall's face.
            let half = tag.half(Align::Center, mm);
            let reach = half.x.max(half.y);
            let c = wall.world(u, side * (wall.half + reach + 2.0 * mm));
            let host_dir = Some(tag::readable(wall.tangent(u)));
            let spot = TagSpot { point: c, anchor: wall.world(u, side * wall.half), host_dir, align: Align::Center };
            tag::draw(d, ctx, &tag, spot, false);
        }
    }
}

/// The text size and style of a dimension type (the defaults for a missing one).
pub fn dimension_type(project: &Project, type_id: cc_core::ElementTypeId) -> (f64, DimensionStyle) {
    match project.element_type(type_id).map(|t| &t.data) {
        Some(TypeData::Dimension { text_size, style }) => (*text_size, style.clone()),
        _ => (2.5, DimensionStyle::default()),
    }
}

/// Dimensions placed in `view`, in its drawing coordinates.
pub fn dimensions(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32) {
    for e in project.elements.values() {
        let ElementKind::Dimension(dim) = &e.kind else { continue };
        if dim.view != view {
            continue;
        }
        let (text_size, style) = dimension_type(project, e.type_id);
        let Some(layout) = dimension_layout(dim, scale, &style) else { continue };
        draw_dimension(d, &layout, &style, text_size, scale, Some(e.id));
    }
}

/// The pen of a style's lines, if it sets any.
fn line_pen(style: &DimensionStyle) -> Option<Pen> {
    (style.line_weight.is_some() || style.color.is_some()).then_some(Pen {
        color: style.color,
        weight: style.line_weight,
        ..Pen::default()
    })
}

/// Draw a laid-out dimension: lines, marks, the box behind an opaque
/// text, and the text.
pub fn draw_dimension(
    d: &mut Drawing2D,
    layout: &DimensionLayout,
    style: &DimensionStyle,
    text_size: f64,
    scale: u32,
    id: Option<cc_core::ElementId>,
) {
    let pen = line_pen(style);
    for (a, b, line) in &layout.lines {
        d.line_pen(*a, *b, *line, pen, id);
    }
    draw_marks(d, layout, style, scale, id);
    if style.opaque_text {
        d.fill(text_box(layout, text_size, scale), OPAQUE, id);
    }
    d.tag_text(layout.text_pos, layout.text.clone(), text_size, Anchor::CenterBottom, layout.text_angle, id);
}

/// White behind an opaque dimension text.
const OPAQUE: [u8; 4] = [255, 255, 255, 255];

/// The marks at the ends of a laid-out dimension line.
pub fn draw_marks(
    d: &mut Drawing2D,
    layout: &DimensionLayout,
    style: &DimensionStyle,
    scale: u32,
    id: Option<cc_core::ElementId>,
) {
    let mark_pen = Pen { color: style.color, weight: style.mark_weight, ..Pen::default() };
    let pens = SymbolPens {
        stroke: (
            LineStyle::Annotation,
            line_pen(&DimensionStyle { line_weight: style.mark_weight.or(style.line_weight), ..style.clone() }),
        ),
        // Ticks are heavy unless the style gives them a width.
        tick: (
            if style.mark_weight.is_some() { LineStyle::Annotation } else { LineStyle::Cut },
            (mark_pen != Pen::default()).then_some(mark_pen),
        ),
        fill: notes::symbol_fill(style.color),
    };
    for (end, u) in &layout.marks {
        notes::symbol_at(d, style.mark, *u, *end, style.mark_size * mm(scale), pens, id);
    }
}

/// The box an opaque text hides, a little larger than the text.
fn text_box(layout: &DimensionLayout, size: f64, scale: u32) -> Vec<DVec2> {
    let mm = mm(scale);
    let (half, height, pad) = (layout.text.chars().count() as f64 * 0.6 * size * mm / 2.0, size * mm, 0.4 * mm);
    let (along, up) = (DVec2::from_angle(layout.text_angle), DVec2::from_angle(layout.text_angle).perp());
    let c = layout.text_pos;
    let (w, lo, hi) = (along * (half + pad), up * -pad, up * (height + pad));
    vec![c - w + lo, c + w + lo, c + w + hi, c - w + hi]
}

/// Linework and text of a dimension, in its view's drawing coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionLayout {
    pub lines: Vec<(DVec2, DVec2, LineStyle)>,
    /// Where the dimension line ends at a mark, and the direction (a unit
    /// vector) it arrives there in.
    pub marks: Vec<(DVec2, DVec2)>,
    /// The value with the style's prefix and suffix.
    pub text: String,
    /// Bottom center of the text, which is rotated by `text_angle` (radians).
    pub text_pos: DVec2,
    pub text_angle: f64,
}

/// Lay out a dimension at a view `scale` as `style` says; `None` when it measures nothing.
pub fn dimension_layout(dim: &Dimension, scale: u32, style: &DimensionStyle) -> Option<DimensionLayout> {
    let mm = mm(scale);
    let (gap, extension) = (style.witness_gap * mm, style.witness_extension * mm);
    let (overrun, text_offset) = (style.line_extension * mm, style.text_offset * mm);
    let line = LineStyle::Annotation;
    let mut lines = Vec::new();
    let mut marks = Vec::new();
    let (text, text_pos, text_angle) = match dim.kind {
        DimensionKind::Aligned | DimensionKind::Linear { .. } => {
            let dir = dim.direction()?;
            let n = dir.perp();
            let measured = (dim.b - dim.a).dot(dir);
            if measured.abs() < 1e-9 {
                return None;
            }
            let pa = dim.a + n * dim.offset;
            let pb = pa + dir * measured;
            // Witness lines leave a gap at the measured points and pass the dimension line.
            let side = if dim.offset >= 0.0 { 1.0 } else { -1.0 };
            for (p, q) in [(dim.a, pa), (dim.b, pb)] {
                let v = q - p;
                let out = v.try_normalize().unwrap_or(n * side);
                let from = p + out * gap.min(v.length());
                lines.push((from, q + out * extension, line));
            }
            let u = dir * measured.signum();
            lines.push((pa - u * overrun, pb + u * overrun, line));
            marks.extend([(pa, -u), (pb, u)]);
            let (angle, up) = upright(u);
            (format::meters(dim.value()), (pa + pb) / 2.0 + up * text_offset, angle)
        }
        DimensionKind::Ordinate { .. } => {
            let dir = dim.direction()?;
            let n = dir.perp();
            let measured = (dim.b - dim.a).dot(dir);
            if measured.abs() < 1e-9 {
                return None;
            }
            let pa = dim.a + n * dim.offset;
            let pb = pa + dir * measured;
            let side = if dim.offset >= 0.0 { 1.0 } else { -1.0 };
            let mut out_b = n * side;
            for (p, q) in [(dim.a, pa), (dim.b, pb)] {
                let v = q - p;
                let out = v.try_normalize().unwrap_or(n * side);
                out_b = out;
                let from = p + out * gap.min(v.length());
                lines.push((from, q + out * extension, line));
            }
            // The origin is a small circle; the line runs from it to an arrow at the point.
            let r = 1.0 * mm;
            let steps = 16;
            let at = |i: usize| pa + DVec2::from_angle(std::f64::consts::TAU * i as f64 / steps as f64) * r;
            for i in 0..steps {
                lines.push((at(i), at(i + 1), line));
            }
            let u = dir * measured.signum();
            lines.push((pa + u * r, pb, line));
            for s in [-1.0, 1.0] {
                lines.push((pb, pb + DVec2::from_angle(s * 0.3).rotate(-u) * 2.5 * mm, line));
            }
            // The running total reads along the witness line, past the dimension line.
            let text = format::meters(measured.abs());
            let (angle, up) = upright(out_b);
            let width = style.label(&text).chars().count() as f64 * 0.6 * 2.5 * mm;
            let pos = pb + out_b * (extension + 0.5 * mm + width / 2.0) - up * 1.25 * mm;
            (text, pos, angle)
        }
        DimensionKind::Angular { vertex, .. } => {
            let (r1, r2) = ((dim.a - vertex).try_normalize()?, (dim.b - vertex).try_normalize()?);
            let sweep = r1.angle_to(r2);
            if sweep.abs() < 1e-9 {
                return None;
            }
            let radius = dim.offset.abs().max(2.0 * mm);
            let at = |t: f64| vertex + DVec2::from_angle(sweep * t).rotate(r1) * radius;
            let steps = 32;
            for i in 0..steps {
                lines.push((at(i as f64 / steps as f64), at((i + 1) as f64 / steps as f64), line));
            }
            // Witness lines along the rays when the arc lies beyond the measured points.
            for (r, p) in [(r1, dim.a), (r2, dim.b)] {
                let reach = (p - vertex).length();
                if radius > reach + gap {
                    lines.push((vertex + r * (reach + gap), vertex + r * (radius + extension), line));
                }
            }
            // The arc arrives at each end along its tangent.
            let turn = sweep.signum();
            let end_ray = DVec2::from_angle(sweep).rotate(r1);
            marks.extend([(at(0.0), -r1.perp() * turn), (at(1.0), end_ray.perp() * turn)]);
            let mid = DVec2::from_angle(sweep / 2.0).rotate(r1);
            let (angle, up) = upright(mid.perp());
            // The text sits outside the arc.
            let out = if up.dot(mid) >= 0.0 { text_offset } else { text_offset + 2.5 * mm };
            (format::angle(dim.value().to_degrees()), vertex + mid * (radius + out), angle)
        }
        DimensionKind::Radial | DimensionKind::Diameter => {
            let center = dim.a;
            let r = (dim.b - center).length();
            let u = (dim.b - center).try_normalize()?;
            let diameter = matches!(dim.kind, DimensionKind::Diameter);
            let start = if diameter { center - u * r } else { center };
            lines.push((start, dim.b, line));
            let mut tips = vec![(dim.b, u)];
            if diameter {
                tips.push((start, -u));
            }
            for (tip, dir) in tips {
                for side in [-1.0, 1.0] {
                    let back = DVec2::from_angle(side * 0.33).rotate(-dir) * 2.5 * mm;
                    lines.push((tip, tip + back, line));
                }
            }
            // Center mark.
            let c = 1.0 * mm;
            lines.push((center - DVec2::X * c, center + DVec2::X * c, line));
            lines.push((center - DVec2::Y * c, center + DVec2::Y * c, line));
            let (angle, up) = upright(u);
            let prefix = if diameter { "\u{d8}" } else { "R" };
            let mid = if diameter { center + u * r * 0.5 } else { (center + dim.b) / 2.0 };
            (format!("{prefix} {}", format::meters(dim.value())), mid + up * text_offset, angle)
        }
    };
    Some(DimensionLayout { lines, marks, text: style.label(&text), text_pos, text_angle })
}

/// Text angle that keeps text along `u` upright, and the side "above" it.
fn upright(u: DVec2) -> (f64, DVec2) {
    let mut angle = u.y.atan2(u.x);
    let mut up = u.perp();
    if angle > std::f64::consts::FRAC_PI_2 + 1e-9 || angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
        angle += std::f64::consts::PI;
        up = -up;
    }
    (angle, up)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::{Dimension, ViewId};

    fn dim(a: (f64, f64), b: (f64, f64), offset: f64, kind: DimensionKind) -> Dimension {
        Dimension {
            view: ViewId(1),
            a: DVec2::new(a.0, a.1),
            b: DVec2::new(b.0, b.1),
            offset,
            kind,
            refs: Default::default(),
            locked: None,
        }
    }

    #[test]
    fn linear_dimensions_measure_along_their_direction() {
        let d = dim((0.0, 0.0), (3.0, 4.0), 1.0, DimensionKind::Linear { dir: DVec2::X });
        assert!((d.value() - 3.0).abs() < 1e-12);
        let d = dim((0.0, 0.0), (3.0, 4.0), 1.0, DimensionKind::Linear { dir: DVec2::Y });
        assert!((d.value() - 4.0).abs() < 1e-12);
        let layout = dimension_layout(&d, 100, &DimensionStyle::default()).unwrap();
        // The dimension line is vertical, one meter to the left of the points (x = -1).
        assert!(layout.lines.iter().any(|(a, b, _)| (a.x + 1.0).abs() < 1e-9 && (b.x + 1.0).abs() < 1e-9));
        assert!(
            dimension_layout(
                &dim((0.0, 0.0), (0.0, 4.0), 1.0, DimensionKind::Linear { dir: DVec2::X }),
                100,
                &DimensionStyle::default()
            )
            .is_none()
        );
    }

    #[test]
    fn angular_radial_and_diameter_values() {
        let d =
            dim((2.0, 0.0), (0.0, 2.0), 1.0, DimensionKind::Angular { vertex: DVec2::ZERO, rays: Default::default() });
        assert!((d.value().to_degrees() - 90.0).abs() < 1e-9);
        assert!(dimension_layout(&d, 50, &DimensionStyle::default()).is_some_and(|l| l.lines.len() > 30));
        let r = dim((1.0, 1.0), (1.0, 3.0), 0.0, DimensionKind::Radial);
        assert!((r.value() - 2.0).abs() < 1e-12);
        assert!(dimension_layout(&r, 50, &DimensionStyle::default()).unwrap().text.starts_with('R'));
        let d = Dimension { kind: DimensionKind::Diameter, ..r };
        assert!((d.value() - 4.0).abs() < 1e-12);
        assert!(dimension_layout(&d, 50, &DimensionStyle::default()).unwrap().text.starts_with('\u{d8}'));
    }

    #[test]
    fn ordinate_dimensions_show_the_running_total_along_their_witness_line() {
        // From the origin (0, 0) to (2.4, 0.3), the line 1 m below.
        let d = dim((0.0, 0.0), (2.4, 0.3), -1.0, DimensionKind::Ordinate { dir: DVec2::X });
        assert!((d.value() - 2.4).abs() < 1e-12);
        let mm = 0.05;
        let layout = dimension_layout(&d, 50, &DimensionStyle::default()).unwrap();
        assert_eq!(layout.text, format::meters(2.4));
        // The dimension line runs from the origin's circle to the point, at y = -1.
        assert!(layout.lines.iter().any(|(a, b, _)| (a.x - mm).abs() < 1e-9
            && (b.x - 2.4).abs() < 1e-9
            && (a.y + 1.0).abs() < 1e-9
            && (b.y + 1.0).abs() < 1e-9));
        // The text reads along the point's witness line (vertical), below the dimension line.
        assert!((layout.text_angle.abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(layout.text_pos.y < -1.0 && (layout.text_pos.x - 2.4).abs() < 0.1, "{:?}", layout.text_pos);
        // A point at the origin measures nothing.
        assert!(
            dimension_layout(
                &dim((0.0, 0.0), (0.0, 2.0), 1.0, DimensionKind::Ordinate { dir: DVec2::X }),
                50,
                &DimensionStyle::default()
            )
            .is_none()
        );
    }

    #[test]
    fn the_style_sets_lengths_marks_pens_and_text() {
        use crate::Prim;
        // At 1:100 a paper millimeter is 0.1 m.
        let d = dim((0.0, 0.0), (4.0, 0.0), 1.0, DimensionKind::Linear { dir: DVec2::X });
        let default = DimensionStyle::default();
        let mut plain = Drawing2D::default();
        let layout = dimension_layout(&d, 100, &default).unwrap();
        draw_dimension(&mut plain, &layout, &default, 2.5, 100, None);
        // Two heavy ticks, the dimension line 1.5 mm past the witness lines.
        let cut = plain.prims.iter().filter(|p| matches!(p, Prim::Line { style: LineStyle::Cut, .. })).count();
        assert_eq!(cut, 2);
        assert!(layout.lines.iter().any(|(a, b, _)| (a.x + 0.15).abs() < 1e-9 && (b.x - 4.15).abs() < 1e-9));

        let style = DimensionStyle {
            mark: cc_core::LeaderSymbol::Arrow,
            line_weight: Some(0.35),
            color: Some([200, 0, 0]),
            witness_gap: 0.0,
            witness_extension: 3.0,
            line_extension: 0.0,
            text_offset: 2.0,
            prefix: "~".into(),
            suffix: " m".into(),
            opaque_text: true,
            ..DimensionStyle::default()
        };
        let layout = dimension_layout(&d, 100, &style).unwrap();
        assert_eq!(layout.text, format!("~{} m", format::meters(4.0)));
        assert!((layout.text_pos.y - 1.2).abs() < 1e-9);
        // Witness lines from the points to 3 mm past the line; the line stops at them.
        assert!(layout.lines.iter().any(|(a, b, _)| a.y.abs() < 1e-9 && (b.y - 1.3).abs() < 1e-9));
        assert!(layout.lines.iter().any(|(a, b, _)| a.x.abs() < 1e-9 && (b.x - 4.0).abs() < 1e-9));
        let mut out = Drawing2D::default();
        draw_dimension(&mut out, &layout, &style, 2.5, 100, None);
        let pens: Vec<_> =
            out.prims.iter().filter_map(|p| if let Prim::Line { pen, .. } = p { Some(*pen) } else { None }).collect();
        assert!(
            !pens.is_empty()
                && pens.iter().all(|p| p.is_some_and(|p| p.weight == Some(0.35) && p.color == Some([200, 0, 0])))
        );
        let fills: Vec<_> = out
            .prims
            .iter()
            .filter_map(|p| if let Prim::Fill { color, .. } = p { Some(*color) } else { None })
            .collect();
        // Two red arrows, then the white box behind the text, drawn before it.
        assert_eq!(fills, vec![[200, 0, 0, 255], [200, 0, 0, 255], OPAQUE]);
        assert!(matches!(out.prims.last(), Some(Prim::Text { .. })));
    }

    #[test]
    fn angular_marks_follow_the_arc() {
        let d =
            dim((2.0, 0.0), (0.0, 2.0), 1.0, DimensionKind::Angular { vertex: DVec2::ZERO, rays: Default::default() });
        let layout = dimension_layout(&d, 50, &DimensionStyle::default()).unwrap();
        // The arc runs counter-clockwise from (1, 0) to (0, 1): it arrives at its ends going -Y and -X.
        let [(p0, u0), (p1, u1)] = layout.marks[..] else { panic!("{:?}", layout.marks) };
        assert!(p0.distance(DVec2::X) < 1e-9 && u0.distance(-DVec2::Y) < 1e-9);
        assert!(p1.distance(DVec2::Y) < 1e-9 && u1.distance(-DVec2::X) < 1e-9);
    }

    #[test]
    fn a_column_whose_profile_fails_is_tagged_at_its_point() {
        use cc_core::structural_profile::{ProfileDef, ProfileMaterial, ProfileShape};
        use cc_core::{Category, Column, Document, Element, ElementId, ProfileId, WallTop};
        let mut p = Project::from_template();
        // A root radius that does not fit: the profile has no outline.
        let shape = ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.5 };
        assert!(shape.outline().is_err());
        let pid = ProfileId(p.alloc_id());
        let def = ProfileDef {
            id: pid,
            uid: Default::default(),
            name: "Broken".into(),
            material: ProfileMaterial::Steel,
            shape,
            catalog: None,
            description: String::new(),
        };
        p.profiles.insert(pid, def);
        let type_id = p.default_type(Category::Column).unwrap();
        if let Some(TypeData::Column { profile, .. }) = p.types.get_mut(&type_id).map(|t| &mut t.data) {
            *profile = Some(pid);
        }
        let level = p.levels[0].id;
        let mut doc = Document::new(p);
        doc.edit("column", |e| {
            let id = ElementId(e.alloc_id());
            let column = Column {
                position: DVec2::new(1.0, 2.0),
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            };
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: "P1".into(),
                comments: String::new(),
                kind: ElementKind::Column(column),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        assert!(doc.project.type_section(&doc.project.types[&type_id].data).is_none());
        let geom = ModelGeometry::build(&doc.project);
        let mut d = Drawing2D::default();
        structure_tags(&mut d, &doc.project, &geom, level, 50, &TagOffsets::new());
        let texts: Vec<DVec2> = d
            .prims
            .iter()
            .filter_map(|p| if let crate::Prim::Text { pos, .. } = p { Some(*pos) } else { None })
            .collect();
        assert!(!texts.is_empty(), "the column is tagged");
        assert!(texts.iter().all(|p| p.is_finite()), "{texts:?}");
    }
}

#[cfg(test)]
mod section_label_tests {
    use super::*;
    use cc_core::structural_profile::{ProfileDef, ProfileMaterial, ProfileShape};
    use cc_core::{ColumnSection, ProfileId, SectionShape, Uid};

    fn with_profile(material: ProfileMaterial) -> (Project, ProfileId) {
        let mut p = Project::empty();
        let id = ProfileId(p.alloc_id());
        let shape = ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.012 };
        p.profiles.insert(
            id,
            ProfileDef {
                id,
                uid: Uid::generate(),
                name: "IPE 200".into(),
                material,
                shape,
                catalog: None,
                description: String::new(),
            },
        );
        (p, id)
    }

    fn beam(profile: Option<ProfileId>) -> TypeData {
        TypeData::Beam {
            width: 0.15,
            height: 0.4,
            material: "m".into(),
            shape: SectionShape::Solid,
            profile,
            profile_rotation_deg: 0.0,
            profile_mirror: false,
        }
    }

    #[test]
    fn steel_profiles_show_their_name_and_others_the_size() {
        let (p, id) = with_profile(ProfileMaterial::Steel);
        assert_eq!(section_label(&p, &beam(Some(id))).as_deref(), Some("IPE 200"));
        let column = TypeData::Column {
            section: ColumnSection::Rectangular { width: 0.2, depth: 0.2 },
            material: "m".into(),
            profile: Some(id),
            profile_rotation_deg: 0.0,
            profile_mirror: false,
        };
        assert_eq!(section_label(&p, &column).as_deref(), Some("IPE 200"));
        let (p, id) = with_profile(ProfileMaterial::Concrete);
        assert_eq!(section_label(&p, &beam(Some(id))).as_deref(), Some("15×40"));
        assert_eq!(section_label(&p, &beam(Some(ProfileId(9999)))).as_deref(), Some("15×40"));
        assert_eq!(section_label(&p, &beam(None)).as_deref(), Some("15×40"));
    }

    #[test]
    fn a_round_concrete_profile_column_is_tagged_with_its_diameter() {
        let (mut p, id) = with_profile(ProfileMaterial::Concrete);
        p.profiles.get_mut(&id).unwrap().shape = ProfileShape::Round { d: 0.3 };
        // The legacy fields of a profile-made type are its box.
        let column = TypeData::Column {
            section: ColumnSection::Rectangular { width: 0.3, depth: 0.3 },
            material: "m".into(),
            profile: Some(id),
            profile_rotation_deg: 0.0,
            profile_mirror: false,
        };
        assert_eq!(section_label(&p, &column).as_deref(), Some("Ø30"));
    }
}
