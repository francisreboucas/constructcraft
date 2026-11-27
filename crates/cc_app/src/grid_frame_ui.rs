//! *Structure → Grid Frame*: the levels, the types (of the tab's material)
//! and optional secondary beams for `grid_frame_create`.

use crate::dialog::Dialog;
use crate::settings;
use crate::state::AppState;
use crate::t;
use crate::theme::PALETTE;
use cc_core::{Category, ElementId, ElementKind, ElementTypeId, LevelId};
use cc_engine::Ref;
use cc_engine::commands::grid_frame::{GridFrameCreate, SecondaryArg};
use egui::Context;

#[derive(Debug, Clone)]
pub struct GridFrameDialog {
    pub base: LevelId,
    pub top: LevelId,
    pub column_type: Option<ElementTypeId>,
    pub beam_type: Option<ElementTypeId>,
    pub secondary: bool,
    pub parallel_to: Option<ElementId>,
    pub spacing: f64,
    pub error: Option<String>,
}

/// Levels by elevation.
fn levels(state: &AppState) -> Vec<(LevelId, String)> {
    let mut levels: Vec<_> = state.project().levels.iter().map(|l| (l.elevation, l.id, l.name.clone())).collect();
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    levels.into_iter().map(|(_, id, name)| (id, name)).collect()
}

/// The grids the command will frame (the selected ones, else all), the same list OK sends.
fn grids(state: &AppState) -> Vec<(ElementId, String)> {
    let ids = crate::actions::grids_for(state);
    state
        .project()
        .elements
        .values()
        .filter(|e| ids.contains(&e.id.0))
        .filter_map(|e| match &e.kind {
            ElementKind::Grid(g) => Some((e.id, g.name.clone())),
            _ => None,
        })
        .collect()
}

/// Open the dialog: from the active level (else the lowest) to the next one.
pub fn open(state: &mut AppState) {
    let levels = levels(state);
    let start = state.active_level().and_then(|a| levels.iter().position(|(id, _)| *id == a)).unwrap_or(0);
    let (Some(base), Some(top)) = (levels.get(start), levels.get(start + 1).or(levels.get(start))) else {
        return;
    };
    let (base, top) = (base.0, top.0);
    let m = state.prefs.structure_material;
    let first = |c| state.project().types_of_material(c, m).next().map(|t| t.id);
    let (column_type, beam_type) = (first(Category::Column), first(Category::Beam));
    let parallel_to = grids(state).first().map(|g| g.0);
    state.dialogs.grid_frame = Some(GridFrameDialog {
        base,
        top,
        column_type,
        beam_type,
        secondary: false,
        parallel_to,
        spacing: 2.0,
        error: None,
    });
}

pub(crate) fn combo<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, value: &mut T, items: &[(T, String)]) {
    let shown = items.iter().find(|(v, _)| v == value).map(|(_, n)| n.clone()).unwrap_or_default();
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(200.0).show_ui(ui, |ui| {
        for (v, name) in items {
            ui.selectable_value(value, *v, name);
        }
    });
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.dialogs.grid_frame.clone() else { return };
    let m = state.prefs.structure_material;
    let levels = levels(state);
    let grids = grids(state);
    let types = |c| -> Vec<(Option<ElementTypeId>, String)> {
        state.project().types_of_material(c, m).map(|t| (Some(t.id), t.name.clone())).collect()
    };
    let (column_types, beam_types) = (types(Category::Column), types(Category::Beam));
    let mut ok = None;
    let response = Dialog::new("grid-frame", t!("dlg-grid-frame")).width(480.0).show(
        ctx,
        |ui| {
            const W: f32 = 220.0;
            settings::setting(ui, &t!("label-base-level"), None, |ui| {
                settings::combo_items(ui, "gf-base", W, &mut d.base, &levels);
            });
            settings::setting(ui, &t!("label-top-level"), None, |ui| {
                settings::combo_items(ui, "gf-top", W, &mut d.top, &levels);
            });
            settings::setting(ui, &t!("label-column-type"), None, |ui| {
                settings::combo_items(ui, "gf-column", W, &mut d.column_type, &column_types);
            });
            settings::setting(ui, &t!("label-beam-type"), None, |ui| {
                settings::combo_items(ui, "gf-beam", W, &mut d.beam_type, &beam_types);
            });
            settings::switch_setting(ui, &t!("label-secondary-beams"), None, &mut d.secondary);
            ui.add_enabled_ui(d.secondary, |ui| {
                let items: Vec<(Option<ElementId>, String)> =
                    grids.iter().map(|(id, n)| (Some(*id), n.clone())).collect();
                settings::setting(ui, &t!("label-parallel-to"), None, |ui| {
                    settings::combo_items(ui, "gf-parallel", W, &mut d.parallel_to, &items);
                });
                settings::setting_row(ui, &t!("label-spacing"), None, |ui| {
                    settings::drag(
                        ui,
                        egui::DragValue::new(&mut d.spacing)
                            .range(0.3..=50.0)
                            .speed(0.05)
                            .suffix(format!(" {}", cc_views::format::unit_symbol("m"))),
                    );
                });
            });
            if let Some(e) = &d.error {
                ui.add_space(6.0);
                settings::callout(ui, PALETTE.danger, e);
            }
        },
        |f| {
            ok = Some(f.primary(t!("action-ok")));
            f.secondary(t!("action-cancel"));
        },
    );
    if response.clicked.is_some() && response.clicked == ok {
        let command = GridFrameCreate {
            grids: crate::actions::grids_for(state),
            base_level: Some(Ref::Id(d.base.0)),
            top_level: Some(Ref::Id(d.top.0)),
            column_type: d.column_type.map(|t| Ref::Id(t.0)),
            beam_type: d.beam_type.map(|t| Ref::Id(t.0)),
            material: Some(m.code().into()),
            secondary: d
                .secondary
                .then_some(d.parallel_to)
                .flatten()
                .map(|g| SecondaryArg { parallel_to: g.0, spacing: d.spacing }),
        };
        match state.session.run(command) {
            Ok(out) => {
                let count = |k: &str| out[k].as_array().map_or(0, Vec::len);
                state.message = t!(
                    "msg-grid-frame-created",
                    "columns" => count("columns"),
                    "beams" => count("beams"),
                    "secondary" => count("secondary"),
                    "skipped" => out["skipped"].as_u64().unwrap_or(0)
                );
                state.dialogs.grid_frame = None;
            }
            Err(e) => {
                d.error = Some(e.message);
                state.dialogs.grid_frame = Some(d);
            }
        }
        return;
    }
    if response.clicked.is_some() || response.dismissed {
        state.dialogs.grid_frame = None;
    } else {
        state.dialogs.grid_frame = Some(d);
    }
}
