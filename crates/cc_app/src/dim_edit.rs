//! Editing measured values in place. Clicking the text of a selected
//! dimension (or double-clicking a dimension) opens a small field over the
//! view; Enter applies the value, Esc or a click elsewhere cancels.
//!
//! Dimensions store points, not references, so a new value moves what the
//! dimension touches: the elements whose linework passes through its moving
//! end (`b`, or `a` when only that end touches something). Elements crossing
//! the move direction translate; walls, beams and grids along it stretch
//! their nearest end. This works in plans, where drawing coordinates are
//! model coordinates; elsewhere only free dimensions change.

use crate::paint::Xf;
use crate::state::AppState;
use crate::t;
use cc_core::{Category, Dimension, DimensionKind, ElementId, ElementKind, ViewId, ViewKind};
use cc_views::{Drawing2D, Prim, format};
use egui::{Painter, Pos2, Response, Ui};
use glam::DVec2;
use std::collections::BTreeSet;

/// What an edited value drives.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// The value of a dimension element.
    Dimension(ElementId),
    /// A temporary dimension of the selection: `ids` move along `dir` so
    /// that the measured `current` distance becomes the typed one.
    Offset { ids: Vec<ElementId>, dir: DVec2, current: f64 },
}

/// The open value field.
#[derive(Debug, Clone)]
pub struct ValueEdit {
    target: Target,
    view: ViewId,
    /// Screen point the field is centered on.
    at: Pos2,
    text: String,
    focus: bool,
}

/// Open the value field for `target` at the screen point `at`.
pub fn open(state: &mut AppState, view: ViewId, target: Target, at: Pos2) {
    let text = match &target {
        Target::Dimension(id) => match dimension(state, *id) {
            Some(d) if matches!(d.kind, DimensionKind::Angular { .. }) => format::number(d.value().to_degrees(), 2),
            Some(d) => format::meters(d.value()),
            None => return,
        },
        Target::Offset { current, .. } => format::meters(*current),
    };
    state.value_edit = Some(ValueEdit { target, view, at, text, focus: true });
}

fn dimension(state: &AppState, id: ElementId) -> Option<Dimension> {
    match state.project().element(id).map(|e| &e.kind) {
        Some(ElementKind::Dimension(d)) => Some((**d).clone()),
        _ => None,
    }
}

/// Clicks on dimension text with the Select tool open the value field.
/// Returns true when the pointer was used (the tools skip the frame).
#[allow(clippy::too_many_arguments)]
pub fn canvas(
    ui: &Ui,
    resp: &Response,
    painter: &Painter,
    state: &mut AppState,
    view: ViewId,
    xf: &Xf,
    px_per_mm: f64,
    drawing: &Drawing2D,
) -> bool {
    if state.tool != crate::tools::Tool::Select {
        return false;
    }
    if selection_temps(ui, resp, painter, state, view, xf, drawing) {
        return true;
    }
    let Some(pos) = resp.hover_pos() else { return false };
    // Double-click on any dimension.
    if resp.double_clicked()
        && let Some(hit) = drawing.hit_test(xf.to_world(pos), 5.0 / xf.scale)
        && dimension(state, hit).is_some()
    {
        state.selection = BTreeSet::from([hit]);
        open_at_text(state, view, hit, xf, px_per_mm, painter);
        return true;
    }
    // A click on the text of the selected dimension.
    let [id] = state.selection.iter().copied().collect::<Vec<_>>()[..] else { return false };
    let Some((center, size, angle)) = text_box(state, id, view, xf, px_per_mm, painter) else { return false };
    if lock_toggle(ui, resp, painter, state, id, (center, size, angle)) {
        return true;
    }
    let local = egui::emath::Rot2::from_angle(angle).inverse() * (pos - center);
    if local.x.abs() > size.x / 2.0 + 4.0 || local.y.abs() > size.y / 2.0 + 4.0 {
        return false;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| {
        center + egui::emath::Rot2::from_angle(angle) * egui::vec2(x * (size.x / 2.0 + 3.0), y * (size.y / 2.0 + 2.0))
    });
    let accent = crate::theme::PALETTE.accent;
    painter.add(egui::Shape::convex_polygon(
        corners.to_vec(),
        accent.gamma_multiply(0.12),
        egui::Stroke::new(1.0, accent),
    ));
    if resp.clicked() {
        open(state, view, Target::Dimension(id), center);
        return true;
    }
    false
}

/// Lock or unlock a dimension with `dimension_lock`; errors go to the status bar.
pub fn set_locked(state: &mut AppState, id: ElementId, locked: bool) {
    let command = cc_engine::commands::annotate::DimensionLock { ids: vec![id.0], locked };
    crate::actions::run_command(state, command);
}

/// A padlock in `rect` (about 14 points square): closed when `locked`, its
/// shackle lifted to one side when not.
pub(crate) fn paint_padlock(painter: &Painter, rect: egui::Rect, locked: bool, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    let body = egui::Rect::from_min_max(egui::pos2(rect.left() + 2.0, rect.center().y - 1.0), rect.right_bottom());
    painter.rect(body, 1.5, color.gamma_multiply(if locked { 0.9 } else { 0.15 }), stroke, egui::StrokeKind::Inside);
    let (left, right) = (body.left() + 2.5, body.right() - 2.5);
    let lift = if locked { 0.0 } else { 3.0 };
    let top = rect.top() + 1.0 - lift;
    let shackle = vec![
        egui::pos2(left, body.top()),
        egui::pos2(left, top + 3.0),
        egui::pos2((left + right) / 2.0, top),
        egui::pos2(right, top + 3.0),
        egui::pos2(right, body.top() - lift),
    ];
    painter.add(egui::Shape::line(shackle, stroke));
}

/// The padlock beside the text of a selected dimension that can be locked
/// (#447): closed when locked; a click locks or unlocks it. Returns true
/// when the click was used.
fn lock_toggle(
    ui: &Ui,
    resp: &Response,
    painter: &Painter,
    state: &mut AppState,
    id: ElementId,
    (center, size, angle): (Pos2, egui::Vec2, f32),
) -> bool {
    let Some(d) = dimension(state, id) else { return false };
    let plan = state.project().view(d.view).is_some_and(|v| v.is_plan());
    if !(plan && d.can_lock()) && d.locked.is_none() {
        return false;
    }
    let locked = d.locked.is_some();
    // Past the end of the text, along it.
    let rot = egui::emath::Rot2::from_angle(angle);
    let at = center + rot * egui::vec2(size.x / 2.0 + 12.0, 0.0);
    let rect = egui::Rect::from_center_size(at, egui::vec2(14.0, 14.0));
    let hovered = resp.hover_pos().is_some_and(|p| rect.expand(2.0).contains(p));
    let color = if hovered || locked { crate::theme::PALETTE.accent } else { crate::theme::PALETTE.text_muted };
    paint_padlock(painter, rect, locked, color);
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), egui::Id::new(("dim-lock", id.0)), rect)
            .show(|ui| ui.label(crate::t!("hint-dim-lock")));
        if resp.clicked() {
            set_locked(state, id, !locked);
            return true;
        }
    }
    false
}

fn open_at_text(state: &mut AppState, view: ViewId, id: ElementId, xf: &Xf, px_per_mm: f64, painter: &Painter) {
    let at = text_box(state, id, view, xf, px_per_mm, painter).map(|t| t.0);
    if let Some(at) = at {
        open(state, view, Target::Dimension(id), at);
    }
}

/// Screen center, size and screen rotation of a dimension's text in `view`.
fn text_box(
    state: &AppState,
    id: ElementId,
    view: ViewId,
    xf: &Xf,
    px_per_mm: f64,
    painter: &Painter,
) -> Option<(Pos2, egui::Vec2, f32)> {
    let el = state.project().element(id)?;
    let ElementKind::Dimension(d) = &el.kind else { return None };
    if d.view != view {
        return None;
    }
    let scale = state.project().view(view)?.scale.max(1);
    let (text_size, style) = cc_views::dimension_type(state.project(), el.type_id);
    let layout = cc_views::dimension_layout(d, scale, &style)?;
    let px = (text_size * px_per_mm).max(9.0) as f32;
    let size = painter.layout_no_wrap(layout.text, egui::FontId::proportional(px), egui::Color32::BLACK).size();
    // The text sits on its anchor (bottom center); screen Y points down.
    let angle = -layout.text_angle as f32;
    let center = xf.to_screen(layout.text_pos) + egui::emath::Rot2::from_angle(angle) * egui::vec2(0.0, -size.y / 2.0);
    Some((center, size, angle))
}

/// The value field over `view`, when open there.
pub fn show(ui: &Ui, state: &mut AppState, view: ViewId) {
    let Some(edit) = state.value_edit.as_mut().filter(|e| e.view == view) else { return };
    let (mut apply, mut cancel) = (false, false);
    egui::Area::new(egui::Id::new("value-edit"))
        .fixed_pos(edit.at - egui::vec2(48.0, 13.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(2)).show(ui, |ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut edit.text).desired_width(90.0));
                if edit.focus {
                    // Focused with the whole value selected, so typing replaces it.
                    r.request_focus();
                    if let Some(mut st) = egui::text_edit::TextEditState::load(ui.ctx(), r.id) {
                        let all = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(edit.text.chars().count()),
                        );
                        st.cursor.set_char_range(Some(all));
                        st.store(ui.ctx(), r.id);
                        edit.focus = false;
                    }
                }
                if r.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        apply = true;
                    } else {
                        cancel = true;
                    }
                }
            });
        });
    if !(apply || cancel) {
        return;
    }
    let Some(edit) = state.value_edit.take() else { return };
    if apply && let Err(message) = apply_text(state, &edit.target, &edit.text) {
        state.message = message;
    }
}

/// Parse `text` for `target` and apply it.
pub fn apply_text(state: &mut AppState, target: &Target, text: &str) -> Result<(), String> {
    match target {
        Target::Dimension(id) => {
            let angular = dimension(state, *id).is_some_and(|d| matches!(d.kind, DimensionKind::Angular { .. }));
            let value =
                if angular { format::parse_number(text).map(f64::to_radians) } else { format::parse_length(text) };
            set_dimension_value(state, *id, value.ok_or_else(|| t!("msg-invalid-value"))?)
        }
        Target::Offset { ids, dir, current } => {
            let value = format::parse_length(text).filter(|v| v.is_finite() && *v >= 0.0);
            let value = value.ok_or_else(|| t!("msg-invalid-value"))?;
            crate::tools::move_elements(state, ids, *dir * (value - current));
            Ok(())
        }
    }
}

/// Give dimension `id` the measured `value` (meters, or radians for angles)
/// through the engine's `dimension_set_value`.
pub fn set_dimension_value(state: &mut AppState, id: ElementId, value: f64) -> Result<(), String> {
    let angular = dimension(state, id).is_some_and(|d| matches!(d.kind, DimensionKind::Angular { .. }));
    let value = if angular { value.to_degrees() } else { value };
    let command = cc_engine::commands::annotate::DimensionSetValue { id: id.0, value };
    state.session.run(command).map(|_| ()).map_err(|e| e.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::Document;

    fn sample() -> AppState {
        AppState::new(Document::new(crate::home::sample_project("casa_simples").unwrap()))
    }

    /// A plan view of the sample and a dimension across the first wall that
    /// runs along X, from its start face line to a point 2 m beyond.
    fn plan(state: &AppState) -> ViewId {
        state.project().views.values().find(|v| matches!(v.kind, ViewKind::FloorPlan { .. })).map(|v| v.id).unwrap()
    }

    #[test]
    fn a_free_dimension_just_takes_its_new_value() {
        let mut state = sample();
        let view = plan(&state);
        let level = state.project().levels[0].id;
        let ty = state.project().default_type(Category::Dimension).unwrap();
        // Far from any linework: nothing is attached.
        let dim = Dimension {
            view,
            a: DVec2::new(100.0, 100.0),
            b: DVec2::new(103.0, 100.0),
            offset: 1.0,
            kind: Default::default(),
            refs: Default::default(),
            locked: None,
        };
        let id = state.create_element("dim".into(), level, ty, ElementKind::Dimension(Box::new(dim)));
        state.sync();
        set_dimension_value(&mut state, id, 5.0).unwrap();
        assert!((dimension(&state, id).unwrap().value() - 5.0).abs() < 1e-9);
        assert!(set_dimension_value(&mut state, id, -1.0).is_err());
        assert!(apply_text(&mut state, &Target::Dimension(id), "abc").is_err());
    }

    #[test]
    fn editing_a_dimension_moves_the_column_it_touches() {
        let mut state = sample();
        let view = plan(&state);
        let level = state.project().levels[0].id;
        let col_ty = state.project().default_type(Category::Column).unwrap();
        let col = cc_core::Column {
            position: DVec2::new(50.0, 50.0),
            rotation_deg: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: cc_core::WallTop::Unconnected,
            structural: true,
            base_support: Default::default(),
        };
        let cid = state.create_element("col".into(), level, col_ty, ElementKind::Column(col));
        state.sync();
        // The column's plan outline: find a vertical edge of it to measure to.
        let drawing = state.drawing(view).unwrap();
        let edge = drawing
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::Line { a, b, element: Some(e), .. } if *e == cid && (a.x - b.x).abs() < 1e-9 => {
                    Some((*a + *b) / 2.0)
                }
                _ => None,
            })
            .unwrap();
        let ty = state.project().default_type(Category::Dimension).unwrap();
        let a = DVec2::new(edge.x - 4.0, edge.y);
        let dim = Dimension {
            view,
            a,
            b: edge,
            offset: 1.0,
            kind: DimensionKind::Linear { dir: DVec2::X },
            refs: Default::default(),
            locked: None,
        };
        let id = state.create_element("dim".into(), level, ty, ElementKind::Dimension(Box::new(dim)));
        state.sync();
        set_dimension_value(&mut state, id, 6.0).unwrap();
        let moved = match &state.project().element(cid).unwrap().kind {
            ElementKind::Column(c) => c.position,
            other => panic!("{other:?}"),
        };
        assert!((moved - DVec2::new(52.0, 50.0)).length() < 1e-9, "{moved}");
        assert!((dimension(&state, id).unwrap().value() - 6.0).abs() < 1e-9);
        // One undo step.
        state.session.doc.undo();
        assert!(
            matches!(&state.project().element(cid).unwrap().kind, ElementKind::Column(c) if c.position == DVec2::new(50.0, 50.0))
        );
    }

    #[test]
    fn placed_dimensions_follow_the_column_they_measure() {
        let mut state = sample();
        let view = plan(&state);
        let level = state.project().levels[0].id;
        let col_ty = state.project().default_type(Category::Column).unwrap();
        let col = cc_core::Column {
            position: DVec2::new(50.0, 50.0),
            rotation_deg: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: cc_core::WallTop::Unconnected,
            structural: true,
            base_support: Default::default(),
        };
        let cid = state.create_element("col".into(), level, col_ty, ElementKind::Column(col));
        state.sync();
        let drawing = state.drawing(view).unwrap();
        let edge = drawing
            .prims
            .iter()
            .find_map(|p| match p {
                Prim::Line { a, b, element: Some(e), .. } if *e == cid && (a.x - b.x).abs() < 1e-9 => {
                    Some((*a + *b) / 2.0)
                }
                _ => None,
            })
            .unwrap();
        let dim = Dimension {
            view,
            a: edge - DVec2::new(4.0, 0.0),
            b: edge,
            offset: 1.0,
            kind: DimensionKind::Linear { dir: DVec2::X },
            refs: Default::default(),
            locked: None,
        };
        let refs = cc_engine::dimensions::associate(state.project(), &drawing, &dim).refs;
        assert!(refs[0].is_none() && refs[1].is_some_and(|r| r.element() == Some(cid)));
        let ty = state.project().default_type(Category::Dimension).unwrap();
        let id =
            state.create_element("dim".into(), level, ty, ElementKind::Dimension(Box::new(Dimension { refs, ..dim })));
        crate::tools::move_elements(&mut state, &[cid], DVec2::new(1.5, 0.0));
        assert!((dimension(&state, id).unwrap().value() - 5.5).abs() < 1e-9);
        // Editing the value moves the referenced column back.
        set_dimension_value(&mut state, id, 4.0).unwrap();
        assert!(
            matches!(&state.project().element(cid).unwrap().kind, ElementKind::Column(c) if (c.position.x - 50.0).abs() < 1e-9)
        );
    }

    #[test]
    fn angular_dimensions_follow_the_walls_they_measure() {
        let mut state = sample();
        let view = plan(&state);
        let walls: Vec<_> = state.project().walls().map(|(e, w)| (e.id, w.start, w.end)).collect();
        // The wall along y = 0 and the one along x = 0.
        let wx = walls.iter().find(|(_, s, e)| s.y.abs() < 1e-9 && e.y.abs() < 1e-9).unwrap().0;
        let wy = walls.iter().find(|(_, s, e)| s.x.abs() < 1e-9 && e.x.abs() < 1e-9).unwrap().0;
        let drawing = state.drawing(view).unwrap();
        // One cut face line of each.
        let face = |id: ElementId, horizontal: bool| {
            drawing
                .prims
                .iter()
                .find_map(|p| match p {
                    Prim::Line { a, b, element: Some(e), style: cc_views::LineStyle::Cut, .. }
                        if *e == id && ((a.y - b.y).abs() < 1e-9) == horizontal && a.distance(*b) > 1.0 =>
                    {
                        Some((*a, *b))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let (fx, fy) = (face(wx, true), face(wy, false));
        let vertex = DVec2::new(fy.0.x, fx.0.y);
        let a = DVec2::new(vertex.x + 2.0, vertex.y);
        let b = DVec2::new(vertex.x, vertex.y + 2.0);
        let dim = Dimension {
            view,
            a,
            b,
            offset: 1.0,
            kind: DimensionKind::Angular { vertex, rays: Default::default() },
            refs: Default::default(),
            locked: None,
        };
        let dim = cc_engine::dimensions::associate(state.project(), &drawing, &dim);
        assert!(dim.refs.iter().all(Option::is_some), "{:?}", dim.refs);
        assert!(matches!(dim.kind, DimensionKind::Angular { rays: [Some(_), Some(_)], .. }));
        let level = state.project().levels[0].id;
        let ty = state.project().default_type(Category::Dimension).unwrap();
        let id = state.create_element("dim".into(), level, ty, ElementKind::Dimension(Box::new(dim)));
        // Move the Y wall (and its face) half a meter along X: the vertex follows.
        crate::tools::move_elements(&mut state, &[wy], DVec2::new(0.5, 0.0));
        let d = dimension(&state, id).unwrap();
        let DimensionKind::Angular { vertex: moved, .. } = d.kind else { panic!("{:?}", d.kind) };
        assert!((moved - vertex - DVec2::new(0.5, 0.0)).length() < 1e-9, "{moved}");
        assert!((d.value().to_degrees() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn elevation_dimensions_between_levels_drive_the_upper_level() {
        let mut state = sample();
        let elevation = state.project().views.values().find(|v| v.projection().is_some()).map(|v| v.id).unwrap();
        let levels: Vec<_> = state.project().levels.iter().map(|l| (l.id, l.elevation)).collect();
        let [(low, z0), (high, z1), ..] = levels[..] else { panic!("the sample has two levels") };
        let drawing = state.drawing(elevation).unwrap();
        // Left of everything drawn, where only the level lines reach.
        let (lo, _) = drawing.bounds().unwrap();
        let x = lo.x + 0.2;
        let dim = Dimension {
            view: elevation,
            // Picked from the upper level down: the upper one still moves.
            a: DVec2::new(x, z1),
            b: DVec2::new(x, z0),
            offset: 0.5,
            kind: DimensionKind::Linear { dir: DVec2::Y },
            refs: Default::default(),
            locked: None,
        };
        let dim = cc_engine::dimensions::associate(state.project(), &drawing, &dim);
        let targets: Vec<_> = dim.refs.iter().map(|r| r.map(|r| r.target)).collect();
        assert_eq!(targets, vec![Some(cc_core::RefTarget::Level(high)), Some(cc_core::RefTarget::Level(low))]);
        let ty = state.project().default_type(Category::Dimension).unwrap();
        let id = state.create_element("dim".into(), low, ty, ElementKind::Dimension(Box::new(dim)));
        set_dimension_value(&mut state, id, 3.5).unwrap();
        let upper = state.project().level(high).unwrap().elevation;
        assert!((upper - (z0 + 3.5)).abs() < 1e-9, "{upper}");
        assert_eq!(state.project().level(low).unwrap().elevation, z0);
        assert!((dimension(&state, id).unwrap().value() - 3.5).abs() < 1e-9);
    }

    #[test]
    fn a_selected_column_shows_editable_distances() {
        let mut state = sample();
        let view = plan(&state);
        let level = state.project().levels[0].id;
        let col_ty = state.project().default_type(Category::Column).unwrap();
        // Inside the house, away from its walls.
        let walls: Vec<_> = state.project().walls().map(|(_, w)| (w.start, w.end)).collect();
        let lo = walls.iter().fold(DVec2::splat(f64::MAX), |m, (a, b)| m.min(*a).min(*b));
        let at = lo + DVec2::new(1.3, 1.7);
        let col = cc_core::Column {
            position: at,
            rotation_deg: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: cc_core::WallTop::Unconnected,
            structural: true,
            base_support: Default::default(),
        };
        let cid = state.create_element("col".into(), level, col_ty, ElementKind::Column(col));
        state.sync();
        state.selection = BTreeSet::from([cid]);
        let drawing = state.drawing(view).unwrap();
        let temps = temps(&state, &drawing);
        assert_eq!(temps.len(), 2, "{temps:?}");
        let along_x = temps.iter().find(|t| t.away.x.abs() > 0.5).cloned().unwrap();
        let current = along_x.from.distance(along_x.to);
        let target = Target::Offset { ids: vec![cid], dir: along_x.away, current };
        apply_text(&mut state, &target, "2").unwrap();
        state.sync();
        let moved = match &state.project().element(cid).unwrap().kind {
            ElementKind::Column(c) => c.position,
            other => panic!("{other:?}"),
        };
        assert!(((moved - along_x.to).dot(along_x.away) - 2.0).abs() < 1e-9, "{moved}");
    }
}

/// A temporary dimension of the selected element: from a point of it to
/// the nearest reference linework, editable to move the element.
#[derive(Debug, Clone, PartialEq)]
pub struct Temp {
    pub from: DVec2,
    pub to: DVec2,
    /// Unit direction the element moves for a larger value (away from `to`).
    pub away: DVec2,
}

/// Nearest reference linework (walls, grids and columns other than
/// `skip`) from `p` along `dir`, crossing the ray at a right angle.
fn nearest_along(
    state: &AppState,
    drawing: &Drawing2D,
    skip: &BTreeSet<ElementId>,
    p: DVec2,
    dir: DVec2,
) -> Option<DVec2> {
    let reference = |e: &ElementId| {
        let category = state.project().element(*e).map(|x| x.category());
        matches!(category, Some(Category::Wall | Category::Grid | Category::Column))
    };
    let mut best: Option<f64> = None;
    for prim in &drawing.prims {
        let Prim::Line { a, b, style, element: Some(e), .. } = prim else { continue };
        let style_ok =
            matches!(style, cc_views::LineStyle::Cut | cc_views::LineStyle::Projection | cc_views::LineStyle::Center);
        if skip.contains(e) || !style_ok || !reference(e) {
            continue;
        }
        let Some(u) = (*b - *a).try_normalize() else { continue };
        // Lines across the ray only (parallel to the element's side).
        if u.dot(dir).abs() > 0.02 {
            continue;
        }
        let t = (*a - p).dot(dir);
        if t <= 1e-6 || t > 50.0 {
            continue;
        }
        // The ray must hit the segment.
        let hit = p + dir * t;
        if !crate::tools::point_on_segment(*a, *b, hit, 1e-6) {
            continue;
        }
        if best.is_none_or(|d| t < d) {
            best = Some(t);
        }
    }
    best.map(|t| p + dir * t)
}

/// Temporary dimensions of the single selected element in a plan: walls,
/// beams and grids to the nearest parallel linework on each side; columns
/// and components along X and Y; doors and windows along their wall.
pub fn temps(state: &AppState, drawing: &Drawing2D) -> Vec<Temp> {
    let [id] = state.selection.iter().copied().collect::<Vec<_>>()[..] else { return Vec::new() };
    let project = state.project();
    let Some(el) = project.element(id) else { return Vec::new() };
    if project.is_pinned(id) {
        return Vec::new();
    }
    // The element's own linework (and its openings') is not a reference.
    let mut skip: BTreeSet<ElementId> = project.hosted_by(id).map(|h| h.id).collect();
    skip.insert(id);
    let mut out = Vec::new();
    let mut probe = |p: DVec2, dir: DVec2, skip: &BTreeSet<ElementId>| {
        if let Some(q) = nearest_along(state, drawing, skip, p, dir) {
            out.push(Temp { from: p, to: q, away: -dir });
        }
    };
    match &el.kind {
        ElementKind::Wall(_) | ElementKind::Beam(_) | ElementKind::Grid(_) => {
            let Some((a, b)) = cc_core::wall_ops::line_of(el) else { return out };
            let Some(u) = (b - a).try_normalize() else { return out };
            let mid = (a + b) / 2.0;
            for n in [u.perp(), -u.perp()] {
                probe(mid, n, &skip);
            }
        }
        ElementKind::Column(c) => {
            for d in [DVec2::X, DVec2::NEG_X, DVec2::Y, DVec2::NEG_Y] {
                probe(c.position, d, &skip);
            }
        }
        ElementKind::Component(c) => {
            for d in [DVec2::X, DVec2::NEG_X, DVec2::Y, DVec2::NEG_Y] {
                probe(c.position, d, &skip);
            }
        }
        ElementKind::Door(h) | ElementKind::Window(h) | ElementKind::Niche(h) => {
            let Some(w) = project.element(h.host).and_then(|e| e.as_wall()) else { return out };
            let Some(u) = (w.end - w.start).try_normalize() else { return out };
            let center = w.start + u * h.offset;
            // The host's own faces run along the move; other walls cross it.
            skip.insert(h.host);
            for d in [u, -u] {
                probe(center, d, &skip);
            }
        }
        _ => {}
    }
    // Points: keep the nearest on each axis (left or right, below or above).
    if matches!(el.kind, ElementKind::Column(_) | ElementKind::Component(_)) {
        let mut kept: Vec<Temp> = Vec::new();
        for axis in [DVec2::X, DVec2::Y] {
            let nearest = out
                .iter()
                .filter(|t| t.away.dot(axis).abs() > 0.5)
                .min_by(|x, y| x.from.distance(x.to).total_cmp(&y.from.distance(y.to)));
            kept.extend(nearest.cloned());
        }
        return kept;
    }
    out
}

/// Draw the selection's temporary dimensions and open one for editing
/// when its value is clicked. Returns true when the click was used.
fn selection_temps(
    ui: &Ui,
    resp: &Response,
    painter: &Painter,
    state: &mut AppState,
    view: ViewId,
    xf: &Xf,
    drawing: &Drawing2D,
) -> bool {
    let plan = state
        .project()
        .view(view)
        .is_some_and(|v| matches!(v.kind, ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. }));
    if !plan || state.interaction.drag.is_some() || resp.dragged() {
        return false;
    }
    let ids: Vec<ElementId> = state.selection.iter().copied().collect();
    let pointer = resp.hover_pos();
    let mut clicked = None;
    for t in temps(state, drawing) {
        let (center, size, angle) = crate::tools::temp_measure(painter, xf, t.from, t.to);
        let local = pointer.map(|p| egui::emath::Rot2::from_angle(angle).inverse() * (p - center));
        let over = local.is_some_and(|l| l.x.abs() <= size.x / 2.0 + 4.0 && l.y.abs() <= size.y / 2.0 + 4.0);
        if over {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            painter.rect_stroke(
                egui::Rect::from_center_size(center, size + egui::vec2(8.0, 6.0)),
                3.0,
                egui::Stroke::new(1.0, crate::theme::PALETTE.accent),
                egui::StrokeKind::Middle,
            );
            if resp.clicked() {
                clicked = Some((t, center));
            }
        }
    }
    let Some((t, center)) = clicked else { return false };
    let current = t.from.distance(t.to);
    open(state, view, Target::Offset { ids, dir: t.away, current }, center);
    true
}
