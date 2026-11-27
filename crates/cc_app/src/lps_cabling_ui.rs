//! Lightning protection and structured cabling in the app: the tools that
//! place air terminals, earth electrodes, racks and data outlets (their
//! built-in types made on first use by `device_type_create`), the earth
//! ring around the walls, the coverage check (`lps_check`) and the cabling
//! per rack (`cabling_list`). Every change is an engine command.

use crate::dialog::Dialog;
use crate::settings;
use crate::state::AppState;
use crate::t;
use crate::theme::PALETTE;
use crate::tools::Tool;
use cc_core::{DeviceKind, LpsClass, MepDiscipline};
use cc_engine::Ref;
use cc_engine::commands::{cabling, devices, lightning};
use egui::{Context, RichText, Ui};
use serde_json::Value;

/// Start placing devices of `device`: its type (made when the project has
/// none) in the component tool, in the services plan that shows it.
pub fn device_tool(state: &mut AppState, device: DeviceKind) {
    let command = devices::DeviceTypeCreate { device: device.name().into(), ports: None };
    let Some(out) = crate::actions::run_command(state, command) else { return };
    let Some(type_id) = out["id"].as_u64() else { return };
    state.set_tool(Tool::Component { type_id: cc_core::ElementTypeId(type_id) });
    let discipline = cc_views::discipline_of(device.class());
    let shown = state
        .active_view
        .and_then(|v| state.project().view(v))
        .is_some_and(|v| matches!(v.kind, cc_core::ViewKind::FloorPlan { mep: Some(d), .. } if d == discipline));
    if state.active_level().is_some() && !shown {
        crate::actions::open_mep_plan(state, discipline);
    }
    state.message = match device {
        DeviceKind::AirTerminal => t!("hint-air-terminal"),
        DeviceKind::EarthElectrode => t!("hint-earth-electrode"),
        DeviceKind::Rack => t!("hint-rack"),
        _ => t!("hint-data-outlet"),
    };
}

/// An earth ring around the walls of the active level (or the lowest).
pub fn earth_ring(state: &mut AppState) {
    let level = state.active_level().or_else(|| state.project().levels.first().map(|l| l.id));
    let command = lightning::EarthRingCreate { level: level.map(|l| Ref::Id(l.0)), ..Default::default() };
    if let Some(out) = crate::actions::run_command(state, command) {
        let length = out["length"].as_f64().unwrap_or(0.0);
        state.message = t!("msg-earth-ring", "length" => cc_views::format::length(length));
        if state.active_level().is_some() {
            crate::actions::open_mep_plan(state, MepDiscipline::Lightning);
        }
    }
}

pub fn show(ctx: &Context, state: &mut AppState) {
    lps_check(ctx, state);
    cabling_dialog(ctx, state);
}

fn number(v: &Value, decimals: usize) -> String {
    v.as_f64().map_or_else(|| String::from("–"), |x| cc_views::format::number(x, decimals))
}

/// A length in meters from a command's output: `12,5 m` in metric projects
/// (as always), feet and inches in imperial ones.
pub fn meters_text(v: &Value, decimals: usize) -> String {
    match v.as_f64() {
        Some(m) if cc_views::format::imperial() => cc_views::format::length(m),
        _ => format!("{} {}", number(v, decimals), cc_views::format::unit_symbol("m")),
    }
}

/// The coverage check, for the class picked in the dialog (computed again
/// when the class or the model changes).
fn lps_check(ctx: &Context, state: &mut AppState) {
    if !state.dialogs.lps_check {
        return;
    }
    let class = state.dialogs.lps_class;
    let revision = state.session.doc.revision();
    let fresh = state.dialogs.lps_result.as_ref().is_some_and(|(c, r, _)| *c == class && *r == revision);
    if !fresh {
        let command = lightning::LpsCheck { class: Some(class.name().into()), spacing: None };
        let Ok(out) = state.session.run(command) else {
            state.dialogs.lps_check = false;
            return;
        };
        state.dialogs.lps_result = Some((class, revision, out));
    }
    let out = state.dialogs.lps_result.as_ref().map(|r| r.2.clone()).unwrap_or_default();
    let mut chosen = class;
    let response = Dialog::new("lps-check", t!("dlg-lps-check")).width(480.0).show(
        ctx,
        |ui| lps_body(ui, &out, &mut chosen),
        |f| {
            f.primary(t!("action-close"));
        },
    );
    state.dialogs.lps_class = chosen;
    if response.clicked.is_some() || response.dismissed {
        state.dialogs.lps_check = false;
        state.dialogs.lps_result = None;
    }
}

fn lps_body(ui: &mut Ui, out: &Value, class: &mut LpsClass) {
    settings::setting(ui, &t!("prop-lps-class"), None, |ui| {
        let classes: Vec<(LpsClass, String)> = LpsClass::ALL.iter().map(|c| (*c, c.name().to_string())).collect();
        settings::combo_items(ui, "lps-class", 120.0, class, &classes);
    });
    settings::value_row(ui, &t!("lps-sphere-radius"), &meters_text(&out["sphere_radius"], 0));
    settings::value_row(ui, &t!("lps-mesh-width"), &meters_text(&out["mesh_width"], 0));
    settings::value_row(ui, &t!("lps-air-points"), &out["air_points"].to_string());
    settings::value_row(ui, &t!("lps-points-protected"), &format!("{} / {}", out["protected"], out["checked"]));
    ui.add_space(12.0);
    let (text, color) = match (out["checked"].as_u64(), out["ok"].as_bool()) {
        (Some(0) | None, _) => (t!("lps-no-roofs"), PALETTE.text_muted),
        (_, Some(true)) => (t!("lps-all-protected"), PALETTE.success),
        _ => (
            t!("lps-unprotected", "percent" => cc_views::format::number(100.0 - 100.0 * out["ratio"].as_f64().unwrap_or(0.0), 0)),
            PALETTE.warning,
        ),
    };
    settings::callout(ui, color, &text);
    ui.add_space(6.0);
    ui.add(egui::Label::new(RichText::new(t!("lps-check-note")).size(11.5).color(PALETTE.text_muted)).wrap());
}

/// The cabling per rack: outlets, cables, their lengths and those over the limit.
fn cabling_dialog(ctx: &Context, state: &mut AppState) {
    if !state.dialogs.cabling {
        return;
    }
    // Listed again only when the model changes.
    let revision = state.session.doc.revision();
    if !state.dialogs.cabling_result.as_ref().is_some_and(|(r, _)| *r == revision) {
        let Ok(out) = state.session.run(cabling::CablingList { rack: None }) else {
            state.dialogs.cabling = false;
            state.dialogs.cabling_result = None;
            return;
        };
        state.dialogs.cabling_result = Some((revision, out));
    }
    let Some((_, out)) = &state.dialogs.cabling_result else { return };
    let response = Dialog::new("cabling", t!("dlg-cabling")).width(560.0).show(
        ctx,
        |ui| cabling_body(ui, out),
        |f| {
            f.primary(t!("action-close"));
        },
    );
    if response.clicked.is_some() || response.dismissed {
        state.dialogs.cabling = false;
        state.dialogs.cabling_result = None;
    }
}

fn cabling_body(ui: &mut Ui, out: &Value) {
    let racks = out["racks"].as_array().cloned().unwrap_or_default();
    if racks.is_empty() {
        ui.label(RichText::new(t!("cabling-none")).color(PALETTE.text_muted));
        return;
    }
    egui::Grid::new("cabling").num_columns(6).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
        for h in [
            t!("device-rack"),
            t!("cabling-outlets"),
            t!("cabling-cables"),
            t!("col-cable-length"),
            t!("cabling-longest"),
            t!("cabling-over-limit"),
        ] {
            ui.label(RichText::new(h).color(PALETTE.text_muted));
        }
        ui.end_row();
        for r in &racks {
            ui.label(r["code"].as_str().unwrap_or(""));
            ui.label(r["outlets"].to_string());
            ui.label(r["cables"].to_string());
            ui.label(meters_text(&r["cable_length"], 1));
            ui.label(meters_text(&r["longest_cable"], 1));
            let over = r["over_limit"].as_u64().unwrap_or(0);
            let color = if over > 0 { PALETTE.warning } else { PALETTE.text_muted };
            ui.label(RichText::new(over.to_string()).color(color));
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    let note = if cc_views::format::imperial() {
        let limit = out["max_link_length"].as_f64().map_or_else(String::new, cc_views::format::length);
        t!("cabling-imp-note", "limit" => limit, "slack" => cc_views::format::length(cc_core::mep::CABLE_SLACK))
    } else {
        t!("cabling-note", "limit" => number(&out["max_link_length"], 0))
    };
    ui.label(RichText::new(note).size(11.0).color(PALETTE.text_faint));
}
