//! Interior elevations in the app (see `cc_core::interior`): the *Interior
//! Elevation* tool (View tab), which places a marker with four views in the
//! space clicked in a floor plan; *Interior Elevations* for the selected
//! rooms (their Modify tab and right-click menu); the marker's properties
//! (a checkbox per side, its turn, its room); a view's *Automatic extents*;
//! and a double click on a marker's arrow, which opens its view.

use crate::paint::{PREVIEW, Xf};
use crate::state::AppState;
use crate::t;
use crate::table::{readonly_row, row, section};
use cc_core::interior::{InteriorMarker, Side};
use cc_core::{ElementId, ElementKind, LevelId, ViewId, ViewKind};
use cc_engine::Ref;
use cc_engine::commands::interior::{InteriorElevationSidesSet, InteriorElevationsCreate};
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

/// The tool in a floor plan: the space under the pointer is shaded with the
/// marker's four arrows; a click places the marker and its four views, and
/// selects it.
pub fn tool(resp: &Response, painter: &Painter, state: &mut AppState, level: LevelId, xf: &Xf) {
    let Some(p) = resp.hover_pos().map(|p| xf.to_world(p)) else { return };
    let boundary = state.session.geom().room_at(state.project(), level, p, state.session.doc.creation_phase);
    if let Some(b) = &boundary {
        let pts: Vec<Pos2> = b.iter().map(|q| xf.to_screen(*q)).collect();
        crate::paint::fill_polygon(painter, pts.clone(), PREVIEW.gamma_multiply(0.12));
        painter.add(egui::Shape::closed_line(pts, Stroke::new(1.5, PREVIEW)));
        let c = xf.to_screen(p);
        painter.circle_stroke(c, 7.0, Stroke::new(1.5, PREVIEW));
        for side in Side::ALL {
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
    let command =
        InteriorElevationsCreate { position: Some(p.to_array()), level: Some(Ref::Id(level.0)), ..Default::default() };
    if let Some(out) = crate::actions::run_command(state, command)
        && let Some(id) = out["marker"].as_u64()
    {
        state.selection = std::collections::BTreeSet::from([ElementId(id)]);
    }
}

/// The options bar of the tool: what to click.
pub fn options(ui: &mut Ui, _state: &mut AppState) {
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

/// The properties of a marker: a checkbox per side (at least one stays), an
/// *Open* button per view, its turn and its room.
pub fn props(ui: &mut Ui, state: &mut AppState, id: ElementId, m: &InteriorMarker) {
    let m = m.clone();
    let sides = state.project().marker_sides(id);
    section(ui, "interior-elevation", t!("section-interior-elevation"), |ui| {
        let mut toggled = None;
        let mut open = None;
        for side in Side::ALL {
            let view = sides.iter().find(|(s, ..)| *s == side).map(|(_, _, v)| *v);
            let mut on = view.is_some();
            let name = view.and_then(|v| state.project().view(v)).map(|v| v.name.clone()).unwrap_or_default();
            row(ui, &side_label(side), |ui| {
                ui.horizontal(|ui| {
                    // The last side cannot be taken away: delete the marker for that.
                    let last = on && sides.len() == 1;
                    let r = ui.add_enabled(!last, crate::table::boxed(egui::Checkbox::without_text(&mut on)));
                    let label = side_label(side);
                    let checked = on;
                    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, !last, checked, &label));
                    if r.on_disabled_hover_text(t!("why-interior-last-side")).changed() {
                        toggled = Some((side, on));
                    }
                    if let Some(v) = view {
                        ui.label(name);
                        if ui.small_button(t!("action-open-view")).clicked() {
                            open = Some(v);
                        }
                    }
                });
            });
        }
        if let Some((side, on)) = toggled {
            let mut keep: Vec<Side> = sides.iter().map(|(s, ..)| *s).filter(|s| *s != side).collect();
            if on {
                keep.push(side);
            }
            let command =
                InteriorElevationSidesSet { marker: id.0, sides: keep.iter().map(|s| s.code().to_string()).collect() };
            crate::actions::run_command(state, command);
        }
        if let Some(v) = open {
            state.open_view(v);
        }
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
