//! Plumbing systems, electrical circuits, the structured cabling of racks
//! and lightning protection systems: names, codes, colors, the board (or
//! rack) feeding each circuit (or cabling) and its voltage, with the
//! circuits' loads.
//! Every change is an engine command (`system_*`); Cancel undoes them all.

use crate::dialog::{Dialog, session};
use crate::state::AppState;
use crate::t;
use crate::theme::PALETTE;
use cc_core::{DeviceKind, ElementId, SystemClass, SystemId, SystemKind};
use cc_engine::Ref;
use cc_engine::commands::mep;
use egui::{Context, RichText, Ui};

fn key(parts: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

fn done(state: &mut AppState, r: &egui::Response) {
    if r.drag_stopped() || r.lost_focus() {
        state.session.doc.break_merge();
    }
}

/// The distribution boards of the project: (element, label).
pub fn panels(state: &AppState) -> Vec<(ElementId, String)> {
    boards(state, DeviceKind::Panel)
}

/// The boards (or racks) of the project: (element, label).
pub fn boards(state: &AppState, board: DeviceKind) -> Vec<(ElementId, String)> {
    let p = state.project();
    p.elements
        .values()
        .filter(|e| p.mep_data(e).and_then(|m| m.device()) == Some(board))
        .map(|e| {
            (e.id, if e.mark.is_empty() { format!("{} {}", p.type_name(e.type_id), e.id.0) } else { e.mark.clone() })
        })
        .collect()
}

/// A choice of the systems of the families `classes` (any, when empty), with "none".
pub fn system_combo(
    ui: &mut Ui,
    state: &AppState,
    id: impl std::hash::Hash + std::fmt::Debug,
    classes: &[SystemClass],
    current: &mut Option<SystemId>,
) -> bool {
    let p = state.project();
    let label = |id: SystemId| p.system(id).map_or(String::new(), |s| format!("{} – {}", s.label(), s.name));
    let before = *current;
    let text = current.map_or(t!("value-none"), label);
    egui::ComboBox::from_id_salt(id).selected_text(text).width(170.0).show_ui(ui, |ui| {
        ui.selectable_value(current, None, t!("value-none"));
        for (sid, s) in &p.systems {
            if classes.is_empty() || classes.contains(&s.kind.class()) {
                ui.selectable_value(current, Some(*sid), label(*sid));
            }
        }
    });
    *current != before
}

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.dialogs.systems {
        return;
    }
    session::begin(state);
    let mut ok = 0;
    let response = Dialog::new("systems", t!("dlg-systems")).width(720.0).show(
        ctx,
        |ui| body(ui, state),
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    if crate::dialogs::ok_cancel(state, &response, ok) {
        state.dialogs.systems = false;
    }
}

fn body(ui: &mut Ui, state: &mut AppState) {
    let systems: Vec<(SystemId, cc_core::MepSystem)> =
        state.project().systems.iter().map(|(id, s)| (*id, s.clone())).collect();
    let panels = panels(state);
    let racks = boards(state, DeviceKind::Rack);
    let header = |ui: &mut Ui, text: String| {
        ui.label(RichText::new(text).color(PALETTE.text_muted));
    };
    for class in SystemClass::ALL {
        let of_class: Vec<&(SystemId, cc_core::MepSystem)> =
            systems.iter().filter(|(_, s)| s.kind.class() == class).collect();
        // Cabling and lightning protection only show once the project has some.
        if of_class.is_empty() && matches!(class, SystemClass::Data | SystemClass::Lightning) {
            continue;
        }
        let (title, add, kind) = match class {
            SystemClass::Plumbing => (t!("systems-plumbing"), t!("systems-add-system"), SystemKind::ColdWater),
            SystemClass::Circuit => (t!("systems-circuits"), t!("systems-add-circuit"), SystemKind::Power),
            SystemClass::Data => (t!("systems-cabling"), t!("systems-add-cabling"), SystemKind::Data),
            SystemClass::Lightning => (t!("systems-lightning"), t!("systems-add-lightning"), SystemKind::Lightning),
        };
        let feeders: &[(ElementId, String)] = match class {
            SystemClass::Circuit => &panels,
            SystemClass::Data => &racks,
            SystemClass::Plumbing | SystemClass::Lightning => &[],
        };
        ui.label(RichText::new(title).strong());
        egui::Grid::new(("systems", class as u8)).num_columns(7).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
            header(ui, String::new());
            header(ui, t!("info-number"));
            header(ui, t!("prop-name"));
            header(ui, t!("prop-system-kind"));
            match class {
                SystemClass::Circuit => {
                    header(ui, t!("device-panel"));
                    header(ui, t!("prop-voltage"));
                }
                SystemClass::Data => {
                    header(ui, t!("device-rack"));
                    ui.label("");
                }
                SystemClass::Plumbing | SystemClass::Lightning => {
                    ui.label("");
                    ui.label("");
                }
            }
            ui.label("");
            ui.end_row();
            for (id, s) in of_class {
                row(ui, state, *id, s, feeders);
                ui.end_row();
            }
        });
        if ui.button(add).clicked() {
            let panel = feeders.first().map(|(id, _)| id.0);
            let command = mep::SystemCreate { kind: kind.name().into(), panel, ..Default::default() };
            crate::actions::run_command(state, command);
        }
        ui.add_space(8.0);
    }
    loads(ui, state);
    fittings(ui, state);
}

/// The sizing of the pipes and circuits (`mep_sizing`), with a button that
/// gives undersized pipes the required diameters (`mep_size_apply`).
pub fn sizing(ctx: &Context, state: &mut AppState) {
    if !state.dialogs.sizing {
        return;
    }
    let Ok(out) = state.session.run(mep::MepSizing {}) else { return };
    let mut apply = false;
    let response = Dialog::new("mep-sizing", t!("dlg-mep-sizing")).width(760.0).show(
        ctx,
        |ui| {
            let n = |v: &serde_json::Value, d: usize| {
                v.as_f64().map_or(String::from("–"), |x| cc_views::format::number(x, d))
            };
            // Millimeters, or inches in imperial projects.
            let diameter = |v: &serde_json::Value| {
                use cc_views::format::{MetricSize, size};
                v.as_f64().map_or(String::from("–"), |mm| size(mm / 1000.0, MetricSize::MmNumber(0)))
            };
            let status = |ok: &serde_json::Value| {
                if ok.as_bool() == Some(true) {
                    RichText::new(t!("sizing-ok")).color(PALETTE.text_muted)
                } else {
                    RichText::new(t!("sizing-undersized")).color(PALETTE.warning)
                }
            };
            let header = |ui: &mut Ui, cols: Vec<String>| {
                for h in cols {
                    ui.label(RichText::new(h).color(PALETTE.text_muted));
                }
                ui.end_row();
            };
            let pipes = out["pipes"].as_array().cloned().unwrap_or_default();
            ui.label(RichText::new(t!("sizing-pipes")).strong());
            if pipes.is_empty() {
                ui.label(RichText::new(t!("sizing-no-pipes")).color(PALETTE.text_muted));
            }
            egui::ScrollArea::vertical().id_salt("sizing-pipes").max_height(240.0).show(ui, |ui| {
                egui::Grid::new("sizing-pipes").num_columns(7).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    if !pipes.is_empty() {
                        header(
                            ui,
                            vec![
                                t!("col-system"),
                                t!("sizing-element"),
                                t!("sizing-load"),
                                t!("sizing-flow"),
                                t!("col-diameter"),
                                t!("sizing-required"),
                                String::new(),
                            ],
                        );
                    }
                    for p in &pipes {
                        ui.label(p["system"].as_str().unwrap_or(""));
                        ui.label(format!("E{}", p["id"]));
                        let unit = if p["kind"] == "sanitary" { t!("col-drain-units") } else { t!("sizing-weights") };
                        ui.label(format!("{} {unit}", n(&p["load"], 1)));
                        ui.label(match p["flow_l_s"].as_f64() {
                            None => "–".into(),
                            Some(flow) => cc_views::format::quantity(cc_views::format::Quantity::Flow, flow, 2),
                        });
                        ui.label(format!("Ø{}", diameter(&p["diameter_mm"])));
                        ui.label(format!("Ø{}", diameter(&p["required_mm"])));
                        ui.label(status(&p["ok"]));
                        ui.end_row();
                    }
                });
            });
            let undersized = pipes.iter().any(|p| p["ok"] == false);
            if ui.add_enabled(undersized, egui::Button::new(t!("sizing-apply"))).clicked() {
                apply = true;
            }
            ui.add_space(8.0);
            let circuits = out["circuits"].as_array().cloned().unwrap_or_default();
            ui.label(RichText::new(t!("sizing-circuits")).strong());
            egui::Grid::new("sizing-circuits").num_columns(8).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                if !circuits.is_empty() {
                    header(
                        ui,
                        vec![
                            t!("info-number"),
                            t!("col-power"),
                            t!("col-current"),
                            t!("sizing-breaker"),
                            t!("sizing-conductor"),
                            t!("prop-length"),
                            t!("sizing-drop"),
                            String::new(),
                        ],
                    );
                }
                for c in &circuits {
                    ui.label(c["code"].as_str().unwrap_or(""));
                    ui.label(format!("{} {}", n(&c["power_va"], 0), cc_views::format::unit_symbol("VA")));
                    ui.label(format!("{} {}", n(&c["current_a"], 1), cc_views::format::unit_symbol("A")));
                    ui.label(format!("{} {}", n(&c["breaker_a"], 0), cc_views::format::unit_symbol("A")));
                    ui.label(format!("{} {}", n(&c["conductor_mm2"], 1), cc_views::format::unit_symbol("mm²")));
                    ui.label(if c["length_m"].is_null() {
                        "–".into()
                    } else {
                        crate::lps_cabling_ui::meters_text(&c["length_m"], 1)
                    });
                    ui.label(if c["voltage_drop_pct"].is_null() {
                        "–".into()
                    } else {
                        format!("{} %", n(&c["voltage_drop_pct"], 2))
                    });
                    ui.label(status(&c["ok"]));
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(t!("sizing-note")).size(11.0).color(PALETTE.text_faint));
        },
        |f| {
            f.primary(t!("action-close"));
        },
    );
    if apply && let Some(out) = crate::actions::run_command(state, mep::MepSizeApply { downsize: false }) {
        state.message = t!("msg-sized", "count" => out["changed"].as_array().map_or(0, Vec::len));
    }
    if response.clicked.is_some() || response.dismissed {
        state.dialogs.sizing = false;
    }
}

/// The fittings takeoff: how many of each kind, angle and diameter.
fn fittings(ui: &mut Ui, state: &mut AppState) {
    let Ok(out) = state.session.run(mep::MepFittings { level: None }) else { return };
    let rows = out["fittings"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.label(RichText::new(t!("systems-fittings")).strong());
    egui::Grid::new("fittings").num_columns(4).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
        for h in [t!("col-system"), t!("col-fitting"), t!("col-diameter"), t!("col-quantity")] {
            ui.label(RichText::new(h).color(PALETTE.text_muted));
        }
        ui.end_row();
        for r in &rows {
            let kind = crate::i18n::tr(&format!("fitting-{}", r["kind"].as_str().unwrap_or("")));
            let angle = r["angle"].as_f64().map_or(String::new(), |a| format!(" {a:.0}°"));
            let category = match r["category"].as_str() {
                Some("conduit") => t!("category-conduit"),
                Some("lightningconductor") => t!("category-lightning-conductor"),
                _ => String::new(),
            };
            let system = r["system"].as_str().map_or(category, str::to_string);
            let sizes: Vec<String> =
                r["diameters_mm"].as_array().into_iter().flatten().map(|d| format!("Ø{d}")).collect();
            ui.label(system);
            // A fitting of a pipe series' catalog by its name; one the series
            // does not sell marked, with the reason.
            match (r["name"].as_str(), r["missing"].as_bool()) {
                (_, Some(true)) => {
                    let reason = r["reason"].as_str().unwrap_or_default().to_string();
                    ui.label(RichText::new(format!("⚠ {kind}{angle}")).color(PALETTE.danger)).on_hover_text(reason);
                }
                (Some(name), _) => {
                    let pieces: Vec<&str> =
                        r["pieces"].as_array().into_iter().flatten().filter_map(|p| p["name"].as_str()).collect();
                    let label = ui.label(name);
                    if pieces.len() > 1 {
                        label.on_hover_text(pieces.join(
                            "
",
                        ));
                    }
                }
                _ => {
                    ui.label(format!("{kind}{angle}"));
                }
            }
            ui.label(sizes.join(" × "));
            ui.label(r["count"].to_string());
            ui.end_row();
        }
    });
}

fn row(ui: &mut Ui, state: &mut AppState, id: SystemId, s: &cc_core::MepSystem, panels: &[(ElementId, String)]) {
    let mut color = s.color();
    if ui.color_edit_button_srgb(&mut color).changed() {
        crate::engine_edit::system(state, id, Some(key((id, "color"))), |x| x.color = Some(color));
    }
    let mut code = s.code.clone();
    let r = crate::table::live_text(ui, ("system-code", id), &mut code, |ui, t| {
        ui.add_sized([60.0, 22.0], egui::TextEdit::singleline(t))
    });
    if r.changed() {
        crate::engine_edit::system(state, id, Some(key((id, "code"))), |x| x.code = code);
    }
    done(state, &r);
    let mut name = s.name.clone();
    let r = crate::table::live_text(ui, ("system-name", id), &mut name, |ui, t| {
        ui.add_sized([180.0, 22.0], egui::TextEdit::singleline(t))
    });
    if r.changed() && !name.trim().is_empty() {
        crate::engine_edit::system(state, id, Some(key((id, "name"))), |x| x.name = name);
    }
    done(state, &r);
    let mut kind = s.kind;
    egui::ComboBox::from_id_salt(("system-kind", id)).selected_text(crate::i18n::tr(kind.key())).width(150.0).show_ui(
        ui,
        |ui| {
            for k in SystemKind::ALL.into_iter().filter(|k| k.class() == s.kind.class()) {
                ui.selectable_value(&mut kind, k, crate::i18n::tr(k.key()));
            }
        },
    );
    if kind != s.kind {
        crate::engine_edit::system(state, id, None, |x| x.kind = kind);
    }
    if matches!(s.kind.class(), SystemClass::Circuit | SystemClass::Data) {
        let mut panel = s.panel;
        let text =
            panel.and_then(|p| panels.iter().find(|(e, _)| *e == p)).map_or(t!("value-none"), |(_, l)| l.clone());
        egui::ComboBox::from_id_salt(("system-panel", id)).selected_text(text).width(140.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut panel, None, t!("value-none"));
            for (e, label) in panels {
                ui.selectable_value(&mut panel, Some(*e), label);
            }
        });
        if panel != s.panel {
            crate::engine_edit::system(state, id, None, |x| x.panel = panel);
        }
        if s.kind.is_electrical() {
            let mut volts = s.voltage;
            let r = ui.add(egui::DragValue::new(&mut volts).range(1.0..=1000.0).suffix(" V"));
            if r.changed() {
                crate::engine_edit::system(state, id, Some(key((id, "volts"))), |x| x.voltage = volts);
            }
            done(state, &r);
        } else {
            ui.label("");
        }
    } else {
        ui.label("");
        ui.label("");
    }
    if ui.button(t!("action-delete")).clicked() {
        crate::actions::run_command(state, mep::SystemDelete { system: Ref::Id(id.0) });
    }
}

/// The circuits' installed loads and currents, per board.
fn loads(ui: &mut Ui, state: &mut AppState) {
    let Ok(out) = state.session.run(mep::CircuitLoads { panel: None }) else { return };
    let rows = out["circuits"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    ui.label(RichText::new(t!("systems-loads")).strong());
    let number = |v: &serde_json::Value, d: usize| cc_views::format::number(v.as_f64().unwrap_or(0.0), d);
    egui::Grid::new("loads").num_columns(5).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
        for h in [t!("info-number"), t!("prop-name"), t!("col-count"), t!("col-power"), t!("col-current")] {
            ui.label(RichText::new(h).color(PALETTE.text_muted));
        }
        ui.end_row();
        for r in &rows {
            ui.label(r["code"].as_str().unwrap_or(""));
            ui.label(r["name"].as_str().unwrap_or(""));
            ui.label(r["devices"].to_string());
            ui.label(format!("{} {}", number(&r["power_va"], 0), cc_views::format::unit_symbol("VA")));
            ui.label(format!("{} {}", number(&r["current_a"], 1), cc_views::format::unit_symbol("A")));
            ui.end_row();
        }
    });
    let total: f64 = rows.iter().filter_map(|r| r["power_va"].as_f64()).sum();
    ui.label(
        RichText::new(t!("systems-total-load", "power" => cc_views::format::number(total, 0)))
            .color(PALETTE.text_muted),
    );
}
