//! Interior elevations in the app (see `cc_core::interior`): the *Interior
//! Elevation* tool (View tab), which places a marker with its views in the
//! space clicked in a floor plan, four directions or a view per wall as the
//! options bar says; *Interior Elevations* for the selected rooms (their
//! Modify tab and right-click menu); the marker's properties (its mode, a
//! checkbox per side or wall, its turn or distances, its room); a view's
//! *Automatic extents*; and a double click on a marker's arrow, which opens
//! its view.

use crate::paint::{PREVIEW, Xf};
use crate::state::AppState;
use crate::t;
use crate::table::{readonly_row, row, section};
use cc_core::interior::{Face, FaceMode, InteriorMarker, Side};
use cc_core::{ElementId, ElementKind, LevelId, ViewId, ViewKind};
use cc_engine::Ref;
use cc_engine::commands::interior::{InteriorElevationFacesSet, InteriorElevationModeSet, InteriorElevationsCreate};
use cc_views::Drawing2D;
use egui::{DragValue, Painter, Pos2, Response, Stroke, Ui};

/// The translated name of a side.
fn side_label(side: Side) -> String {
    crate::i18n::tr(match side {
        Side::North => "tpl-elev-north",
        Side::East => "tpl-elev-east",
        Side::South => "tpl-elev-south",
        Side::West => "tpl-elev-west",
    })
}

/// The label of the `index`th wall around a marker: *Wall A*, *Wall B*…
/// (letters, so they are not taken for the numbers of the views).
fn wall_label(index: usize) -> String {
    let mut letters = String::new();
    let mut n = index;
    loop {
        letters.insert(0, char::from(b'A' + (n % 26) as u8));
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    format!("{} {letters}", t!("tool-wall"))
}

/// The walls marker `id` can show, found once per change of the document
/// (the properties are drawn every frame).
fn walls_of(ui: &Ui, state: &mut AppState, id: ElementId, level: LevelId, m: &InteriorMarker) -> Vec<Face> {
    let key = egui::Id::new(("interior-walls", id.0));
    let revision = state.session.doc.revision();
    if let Some((rev, walls)) = ui.ctx().data(|d| d.get_temp::<(u64, Vec<Face>)>(key))
        && rev == revision
    {
        return walls;
    }
    let phase = state.session.doc.creation_phase;
    let walls: Vec<Face> = cc_views::interior::wall_faces(state.project(), state.session.geom(), level, m, phase)
        .iter()
        .map(|f| Face::Wall(f.wall))
        .collect();
    ui.ctx().data_mut(|d| d.insert_temp(key, (revision, walls.clone())));
    walls
}

/// The name of a mode in lists.
fn mode_label(walls: bool) -> String {
    if walls { t!("interior-mode-walls") } else { t!("interior-mode-directions") }
}

/// The tool in a floor plan: the space under the pointer is shaded with the
/// marker (and its four arrows in the directions mode); a click places the
/// marker and its views, and selects it.
pub fn tool(resp: &Response, painter: &Painter, state: &mut AppState, level: LevelId, xf: &Xf) {
    let Some(p) = resp.hover_pos().map(|p| xf.to_world(p)) else { return };
    let boundary = state.session.geom().room_at(state.project(), level, p, state.session.doc.creation_phase);
    let walls = state.interior_walls;
    if let Some(b) = &boundary {
        let pts: Vec<Pos2> = b.iter().map(|q| xf.to_screen(*q)).collect();
        crate::paint::fill_polygon(painter, pts.clone(), PREVIEW.gamma_multiply(0.12));
        painter.add(egui::Shape::closed_line(pts, Stroke::new(1.5, PREVIEW)));
        let c = xf.to_screen(p);
        painter.circle_stroke(c, 7.0, Stroke::new(1.5, PREVIEW));
        for side in Side::ALL.into_iter().filter(|_| !walls) {
            let look = side.look(0.0);
            let dir = egui::vec2(look.x as f32, -look.y as f32);
            let across = egui::vec2(-dir.y, dir.x);
            let tri = vec![c + dir * 14.0, c + dir * 8.0 + across * 6.0, c + dir * 8.0 - across * 6.0];
            painter.add(egui::Shape::convex_polygon(tri, PREVIEW, Stroke::NONE));
        }
    }
    if !resp.clicked() {
        return;
    }
    if boundary.is_none() {
        state.message = t!("msg-interior-not-enclosed");
        return;
    }
    let command = InteriorElevationsCreate {
        position: Some(p.to_array()),
        level: Some(Ref::Id(level.0)),
        mode: walls.then(|| "walls".to_string()),
        ..Default::default()
    };
    if let Some(out) = crate::actions::run_command(state, command)
        && let Some(id) = out["marker"].as_u64()
    {
        state.selection = std::collections::BTreeSet::from([ElementId(id)]);
    }
}

/// The options bar of the tool: four directions or a view per wall, and what to click.
pub fn options(ui: &mut Ui, state: &mut AppState) {
    for walls in [false, true] {
        ui.selectable_value(&mut state.interior_walls, walls, mode_label(walls));
    }
    ui.separator();
    ui.label(crate::theme::muted(t!("hint-interior-elevation")));
}

/// The rooms among the selection.
pub fn selected_rooms(state: &AppState) -> Vec<ElementId> {
    let p = state.project();
    state
        .selection
        .iter()
        .take(2000)
        .copied()
        .filter(|id| matches!(p.element(*id).map(|e| &e.kind), Some(ElementKind::Room(_))))
        .collect()
}

/// Interior elevations of each selected room: a marker at its point with
/// its four views. The markers made are selected.
pub fn for_selected_rooms(state: &mut AppState) {
    let mut made = std::collections::BTreeSet::new();
    for room in selected_rooms(state) {
        let command = InteriorElevationsCreate { room: Some(room.0), ..Default::default() };
        if let Some(id) = crate::actions::run_command(state, command).and_then(|out| out["marker"].as_u64()) {
            made.insert(ElementId(id));
        }
    }
    if !made.is_empty() {
        state.selection = made;
    }
}

/// A double click on an arrow of a marker opens its view. Returns whether it did.
pub fn open_arrow(resp: &Response, state: &mut AppState, xf: &Xf, drawing: &Drawing2D) -> bool {
    if !(resp.double_clicked() || resp.triple_clicked()) {
        return false;
    }
    let Some(p) = resp.hover_pos().map(|p| xf.to_world(p)) else { return false };
    let Some(view) = drawing.interior_arrow_at(p, 3.0 / xf.scale.max(1e-9)) else { return false };
    state.interaction.drag = None;
    state.open_view(view);
    true
}

/// One face's row: a checkbox (disabled for the last face shown), the
/// view's name and *Open*. Returns the new state when toggled, and the view
/// to open.
fn face_row(
    ui: &mut Ui,
    state: &AppState,
    label: &str,
    view: Option<ViewId>,
    last: bool,
) -> (Option<bool>, Option<ViewId>) {
    let mut on = view.is_some();
    let name = view.and_then(|v| state.project().view(v)).map(|v| v.name.clone()).unwrap_or_default();
    let (mut toggled, mut open) = (None, None);
    row(ui, label, |ui| {
        ui.horizontal(|ui| {
            // The last face cannot be taken away: delete the marker for that.
            let last = on && last;
            let r = ui.add_enabled(!last, crate::table::boxed(egui::Checkbox::without_text(&mut on)));
            let checked = on;
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, !last, checked, label));
            if r.on_disabled_hover_text(t!("why-interior-last-side")).changed() {
                toggled = Some(on);
            }
            if let Some(v) = view {
                ui.label(name);
                if ui.small_button(t!("action-open-view")).clicked() {
                    open = Some(v);
                }
            }
        });
    });
    (toggled, open)
}

/// Run a change of the marker's mode values as one undo step per drag.
fn set_mode(state: &mut AppState, id: ElementId, field: &str, command: InteriorElevationModeSet) {
    let key = crate::properties::key((id, field));
    if let Err(e) = state.session.run_merged(key, command) {
        state.message = e.message;
    }
}

/// The properties of a marker: its mode, a checkbox per side or per wall of
/// its space (at least one stays) with an *Open* button per view, its turn
/// (directions) or its distance and minimum length (walls), and its room.
pub fn props(ui: &mut Ui, state: &mut AppState, id: ElementId, m: &InteriorMarker) {
    let m = m.clone();
    let Some(level) = state.project().element(id).map(|e| e.level) else { return };
    let faces = state.project().marker_faces(id);
    section(ui, "interior-elevation", t!("section-interior-elevation"), |ui| {
        // Four directions, or a view per wall: changing it remakes the views.
        let was_walls = !m.mode.is_directions();
        let mut walls = was_walls;
        row(ui, &t!("prop-interior-mode"), |ui| {
            egui::ComboBox::from_id_salt(("interior-mode", id.0)).selected_text(mode_label(walls)).show_ui(ui, |ui| {
                for option in [false, true] {
                    ui.selectable_value(&mut walls, option, mode_label(option));
                }
            });
        });
        if walls != was_walls {
            let mode = if walls { "walls" } else { "directions" };
            let command = InteriorElevationModeSet { marker: id.0, mode: mode.into(), ..Default::default() };
            crate::actions::run_command(state, command);
            return;
        }
        // The faces the marker can show, with the view of each shown one; then
        // the walls it still shows that are no longer walls of its space.
        let mut available: Vec<(Face, String)> = match m.mode {
            FaceMode::Directions => Side::ALL.into_iter().map(|s| (Face::Side(s), side_label(s))).collect(),
            FaceMode::Walls { .. } => {
                walls_of(ui, state, id, level, &m).into_iter().enumerate().map(|(i, f)| (f, wall_label(i))).collect()
            }
        };
        let out: Vec<Face> =
            faces.iter().map(|(f, ..)| *f).filter(|f| !available.iter().any(|(a, _)| a == f)).collect();
        available.extend(out.into_iter().map(|f| (f, t!("interior-wall-out"))));
        let (mut toggled, mut open) = (None, None);
        for (face, label) in &available {
            let view = faces.iter().find(|(f, ..)| f == face).map(|(_, _, v)| *v);
            let (t, o) = face_row(ui, state, label, view, faces.len() == 1);
            if let Some(on) = t {
                toggled = Some((*face, on));
            }
            open = open.or(o);
        }
        if let Some((face, on)) = toggled {
            let mut keep: Vec<Face> = faces.iter().map(|(f, ..)| *f).filter(|f| *f != face).collect();
            if on {
                keep.push(face);
            }
            let sides: Vec<String> = keep
                .iter()
                .filter_map(|f| match f {
                    Face::Side(s) => Some(s.code().to_string()),
                    Face::Wall(_) => None,
                })
                .collect();
            let wall_ids: Vec<u64> = keep
                .iter()
                .filter_map(|f| match f {
                    Face::Wall(w) => Some(w.0),
                    Face::Side(_) => None,
                })
                .collect();
            let command = match m.mode {
                FaceMode::Directions => InteriorElevationFacesSet { marker: id.0, sides: Some(sides), walls: None },
                FaceMode::Walls { .. } => {
                    InteriorElevationFacesSet { marker: id.0, sides: None, walls: Some(wall_ids) }
                }
            };
            crate::actions::run_command(state, command);
        }
        if let Some(v) = open {
            state.open_view(v);
        }
        match m.mode {
            FaceMode::Directions => {
                let mut rot = m.rotation_deg;
                let r = row(ui, &t!("prop-rotation"), |ui| ui.add(DragValue::new(&mut rot).speed(1.0).suffix("°")));
                if r.changed() {
                    let key = crate::properties::key((id, "interior-rotation"));
                    crate::engine_edit::element(state, id, Some(key), |e| {
                        if let ElementKind::InteriorMarker(m) = &mut e.kind {
                            m.rotation_deg = rot.rem_euclid(360.0);
                        }
                    });
                }
                if r.drag_stopped() || r.lost_focus() {
                    state.session.doc.break_merge();
                }
            }
            FaceMode::Walls { distance, min_length } => {
                let max = cc_core::interior::MAX_DISTANCE;
                let mut d = distance;
                let r = row(ui, &t!("prop-interior-distance"), |ui| {
                    ui.add(crate::properties::length_drag(&mut d).speed(0.01).range(0.0..=max))
                })
                .on_hover_text(t!("prop-interior-distance-tip"));
                if r.changed() {
                    let command = InteriorElevationModeSet {
                        marker: id.0,
                        mode: "walls".into(),
                        distance: Some(d),
                        min_length: None,
                    };
                    set_mode(state, id, "interior-distance", command);
                }
                if r.drag_stopped() || r.lost_focus() {
                    state.session.doc.break_merge();
                }
                let mut least = min_length;
                let r = row(ui, &t!("prop-interior-min-length"), |ui| {
                    ui.add(crate::properties::length_drag(&mut least).speed(0.01).range(0.0..=max))
                })
                .on_hover_text(t!("prop-interior-min-length-tip"));
                if r.changed() {
                    let command = InteriorElevationModeSet {
                        marker: id.0,
                        mode: "walls".into(),
                        distance: None,
                        min_length: Some(least),
                    };
                    set_mode(state, id, "interior-min-length", command);
                }
                if r.drag_stopped() || r.lost_focus() {
                    state.session.doc.break_merge();
                }
            }
        }
        let room = m.room.and_then(|r| match state.project().element(r).map(|e| &e.kind) {
            Some(ElementKind::Room(room)) => Some(format!("{} {}", room.number, room.name).trim().to_string()),
            _ => None,
        });
        readonly_row(ui, &t!("prop-room"), room.unwrap_or_else(|| t!("value-none")));
    });
}

/// An interior elevation's own rows: *Automatic extents* (off, the view
/// keeps the crop it has now, which can then be dragged).
pub fn view_rows(ui: &mut Ui, state: &mut AppState, id: ViewId) {
    let Some(view) = state.project().view(id).cloned() else { return };
    let ViewKind::InteriorElevation { auto_extents, .. } = view.kind else { return };
    section(ui, "interior-extents", t!("section-extents"), |ui| {
        let mut on = auto_extents;
        let label = t!("prop-auto-extents");
        let r = row(ui, &label, |ui| crate::table::checkbox(ui, &mut on));
        let checked = on;
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, checked, &label));
        let r = r.on_hover_text(t!("prop-auto-extents-tip"));
        if !r.changed() {
            return;
        }
        // Turned off, the view starts from the crop it was drawn with.
        let resolved = cc_views::interior::resolve(state.project(), state.session.geom(), &view).and_then(|r| r.crop);
        crate::engine_edit::view(state, id, None, |v| {
            if let ViewKind::InteriorElevation { auto_extents, .. } = &mut v.kind {
                *auto_extents = on;
            }
            if !on && v.crop.is_none() {
                v.crop = resolved;
            }
        });
    });
}
