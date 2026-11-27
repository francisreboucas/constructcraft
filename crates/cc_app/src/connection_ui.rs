//! Connections in the app: the Structure tab's *Connections* buttons, which
//! run `connections_create` on the selected members (else the active
//! level's), and the rows of a selected connection, whose sizes change
//! through `element_set`.

use crate::member_host_ui::{finished, length};
use crate::state::AppState;
use crate::t;
use crate::table::{readonly_row, row, section};
use cc_core::connection::{COUNT, Connection, ConnectionKind, LENGTH};
use cc_core::{Category, ElementId, ElementKind};
use cc_engine::Ref;
use cc_engine::commands::connection::ConnectionsCreate;
use egui::{DragValue, Ui};

/// Place connections of `kind` with their default sizes: on the selected
/// columns (base plates) or beams, else on the active level's; the result
/// goes to the status bar.
pub fn place(state: &mut AppState, kind: ConnectionKind) {
    let project = state.project();
    let wanted = if kind.at_column() { Category::Column } else { Category::Beam };
    let members: Vec<u64> = state
        .selection
        .iter()
        .filter(|id| project.element(**id).is_some_and(|e| e.category() == wanted))
        .map(|id| id.0)
        .collect();
    if members.is_empty() && !state.selection.is_empty() {
        state.message = t!("msg-connections-none");
        return;
    }
    let level = if members.is_empty() { state.active_level().map(|l| Ref::Id(l.0)) } else { None };
    let command = ConnectionsCreate { kind: kind.code().into(), members, level, params: None };
    if let Some(out) = crate::actions::run_command(state, command) {
        let count = out["created"].as_array().map_or(0, Vec::len);
        let skipped = out["skipped"].as_u64().unwrap_or(0);
        state.message = t!("msg-connections-placed", "count" => count, "skipped" => skipped);
    }
}

/// Change connection `id` as `f` changes a copy of it (`element_set`);
/// edits under one `field` merge while dragging.
pub fn edit(state: &mut AppState, id: ElementId, field: &str, f: impl FnOnce(&mut Connection)) {
    crate::engine_edit::element(state, id, Some(crate::properties::key((id, field))), |e| {
        if let ElementKind::Connection(c) = &mut e.kind {
            f(c);
        }
    });
}

/// A plate size row: 0 reads *auto* (from the member); true when the edit ended.
fn size(ui: &mut Ui, label: &str, v: &mut f64) -> bool {
    let done = finished(&row(ui, label, |ui| {
        ui.horizontal(|ui| {
            let r = ui.add(crate::properties::length_drag(v).range(0.0..=*LENGTH.end()));
            if *v == 0.0 {
                ui.label(crate::theme::muted(t!("connection-size-auto")));
            }
            r
        })
        .inner
    }));
    // Between 0 and the least length: the least length.
    if *v > 0.0 && *v < *LENGTH.start() {
        *v = *LENGTH.start();
    }
    done
}

fn count(ui: &mut Ui, label: &str, v: &mut u32) -> bool {
    finished(&row(ui, label, |ui| ui.add(DragValue::new(v).range(COUNT))))
}

/// The rows of a selected connection.
pub fn props(ui: &mut Ui, state: &mut AppState, id: ElementId, connection: &Connection) {
    let mut c = connection.clone();
    let mut done = false;
    let (kind, end) = (c.kind, c.end);
    let member = state
        .project()
        .element(c.member)
        .map(|m| if m.mark.is_empty() { m.id.to_string() } else { m.mark.clone() })
        .unwrap_or_default();
    let built = state.session.geom().connections.get(&id).cloned();
    section(ui, "connection", t!("section-connection"), |ui| {
        readonly_row(ui, &t!("prop-connection-kind"), crate::i18n::tr(kind.key()));
        readonly_row(ui, &t!("prop-host"), member);
        readonly_row(ui, &t!("prop-member-end"), crate::i18n::tr(end.key()));
        let p = &mut c.params;
        if kind != ConnectionKind::TimberBolts {
            done |= length(ui, &t!("prop-plate-thickness"), &mut p.thickness, LENGTH);
            done |= size(ui, &t!("prop-plate-width"), &mut p.width);
            done |= size(ui, &t!("prop-plate-length"), &mut p.length);
        } else {
            done |= length(ui, &t!("prop-thickness"), &mut p.thickness, LENGTH);
        }
        done |= length(ui, &t!("col-bolt-diameter"), &mut p.bolt_diameter, LENGTH);
        done |= count(ui, &t!("prop-bolt-rows"), &mut p.rows);
        done |= count(ui, &t!("prop-bolt-columns"), &mut p.columns);
        done |= length(ui, &t!("prop-bolt-spacing"), &mut p.spacing, LENGTH);
        done |= length(ui, &t!("prop-edge-distance"), &mut p.edge, LENGTH);
        if kind == ConnectionKind::BasePlate {
            done |= length(ui, &t!("prop-anchor-length"), &mut p.anchor_length, LENGTH);
        }
        if let Some(g) = &built {
            readonly_row(
                ui,
                &t!("col-plate-mass"),
                format!("{} {}", cc_views::format::number(g.plate_mass(), 2), cc_views::format::unit_symbol("kg")),
            );
            readonly_row(ui, &t!("col-bolt-count"), g.bolt_count().to_string());
            if g.warning.is_some() {
                ui.label(crate::theme::muted(t!("warn-connection-end")));
            }
        }
    });
    if c != *connection {
        edit(state, id, "connection", move |x| *x = c);
    }
    if done {
        state.session.doc.break_merge();
    }
}
