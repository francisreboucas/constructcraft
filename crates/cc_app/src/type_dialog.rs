//! Type Properties dialog: switch, duplicate and rename types, edit the
//! compound structure (layers) and the type parameters in sections.

use crate::actions;
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::table::{readonly_row, row, section};
use crate::theme::{self, PALETTE};
use cc_core::{
    ElementType, ElementTypeId, Layer, LayerFunction, ProfileId, ProfileMaterial, TypeData, TypeInfo, WallFunction,
};
use cc_views::format;
use egui::{Context, DragValue, RichText, Ui};
use std::hash::{Hash, Hasher};

fn key(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// The dialog's width while a wall type's layers table, with each layer's
/// base offset and height, is open.
const WALL_LAYERS_WIDTH: f32 = 780.0;

fn finish(state: &mut AppState, r: &egui::Response) {
    if r.drag_stopped() || r.lost_focus() {
        state.session.doc.break_merge();
    }
}

/// Apply `f` to type `tid`, merging consecutive edits of the same field.
fn update(state: &mut AppState, tid: ElementTypeId, field: &str, f: impl FnOnce(&mut ElementType)) {
    crate::engine_edit::element_type(state, tid, Some(key((tid, field))), f);
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(tid) = state.dialogs.type_editor else { return };
    let Some(ty) = state.project().element_type(tid).cloned() else {
        state.dialogs.type_editor = None;
        return;
    };
    crate::dialog::session::begin(state);
    let mut ok = 0;
    let max_height = (ctx.content_rect().height() - 320.0).clamp(240.0, 560.0);
    // A wall type's layers table has two more columns (each layer's extent): wider while it is open.
    let wall_layers = state.dialogs.type_layers_open
        && matches!(ty.data, TypeData::Wall { curtain: None, .. })
        && !ty.layers().is_empty();
    let width = if wall_layers { WALL_LAYERS_WIDTH } else { 580.0 };
    let response = crate::dialog::Dialog::new("type-editor", t!("dlg-type")).width(width).show(
        ctx,
        |ui| {
            header(ui, state, &ty);
            ui.add_space(6.0);
            ui.label(theme::muted(t!("type-parameters")).size(12.0));
            egui::Frame::new().stroke(egui::Stroke::new(1.0, PALETTE.border)).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(max_height).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    parameters(ui, state, &ty);
                });
            });
        },
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    // While renaming, Esc only cancels the rename.
    let renaming = state.dialogs.type_rename.is_some();
    let response = crate::dialog::DialogResponse { dismissed: response.dismissed && !renaming, ..response };
    // OK keeps a name still being typed.
    if response.clicked == Some(ok)
        && let Some(name) = state.dialogs.type_rename.take()
    {
        rename(state, &ty, &name);
    }
    if crate::dialogs::ok_cancel(state, &response, ok) {
        state.dialogs.type_editor = None;
        state.dialogs.type_rename = None;
    }
}

/// Family (category), type selector and the Duplicate / Rename buttons.
fn header(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    let category = ty.category();
    egui::Grid::new("type-header").num_columns(3).striped(false).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label(theme::muted(t!("type-category")));
        ui.label(RichText::new(actions::t_category(category)).color(PALETTE.text));
        ui.label("");
        ui.end_row();

        ui.label(theme::muted(t!("type-type")));
        let button_w = 110.0;
        let field_w = (ui.available_width() - button_w - 12.0).max(200.0);
        match state.dialogs.type_rename.take() {
            Some(mut name) => {
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(field_w));
                // Focus once: the user may then click another field, which
                // confirms the name.
                let starting = std::mem::take(&mut state.dialogs.type_rename_focus);
                if starting {
                    r.request_focus();
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if !escape {
                    if enter || (!starting && !r.has_focus()) {
                        rename(state, ty, &name);
                    } else {
                        state.dialogs.type_rename = Some(name);
                    }
                }
            }
            None => {
                let mut selected = tid;
                let types: Vec<_> = state.project().types_of(category).map(|t| (t.id, t.name.clone())).collect();
                egui::ComboBox::from_id_salt("type-switch")
                    .width(field_w)
                    .truncate()
                    .selected_text(ty.name.clone())
                    .show_ui(ui, |ui| {
                        for (id, name) in types {
                            ui.selectable_value(&mut selected, id, name);
                        }
                    });
                if selected != tid {
                    state.dialogs.type_editor = Some(selected);
                }
            }
        }
        ui.vertical(|ui| {
            if ui.add_sized([button_w, 24.0], egui::Button::new(t!("type-duplicate"))).clicked() {
                duplicate(state, ty);
            }
            if ui.add_sized([button_w, 24.0], egui::Button::new(t!("type-rename"))).clicked() {
                state.dialogs.type_rename = Some(ty.name.clone());
                state.dialogs.type_rename_focus = true;
            }
        });
        ui.end_row();
    });
}

/// Copy the type under a new name; selected instances of it switch to the copy.
fn duplicate(state: &mut AppState, ty: &ElementType) {
    let selection: Vec<u64> = state.selection.iter().map(|e| e.0).collect();
    let name = t!("copy-of", "name" => ty.name);
    let command = cc_engine::commands::types::TypeDuplicate {
        type_ref: cc_engine::Ref::Id(ty.id.0),
        name: Some(name.clone()),
        elements: selection,
    };
    let Some(new_id) = crate::actions::run_command(state, command).and_then(|o| o["id"].as_u64()) else { return };
    state.dialogs.type_editor = Some(ElementTypeId(new_id));
    state.dialogs.type_rename = Some(name);
    state.dialogs.type_rename_focus = true;
}

/// Give type `ty` the name typed in the rename field.
fn rename(state: &mut AppState, ty: &ElementType, name: &str) {
    let name = name.trim().to_string();
    if !name.is_empty() && name != ty.name {
        crate::engine_edit::element_type(state, ty.id, None, |t| t.name = name);
    }
}

fn length_row(
    ui: &mut Ui,
    state: &mut AppState,
    label: &str,
    tid: ElementTypeId,
    field: &str,
    value: f64,
    set: impl FnOnce(&mut TypeData, f64),
) {
    let mut v = value;
    let r = row(ui, label, |ui| ui.add(crate::properties::length_drag(&mut v).speed(0.005).range(0.001..=20.0)));
    if r.changed() {
        update(state, tid, field, |t| set(&mut t.data, v));
    }
    finish(state, &r);
}

/// A row editing a small size of a type (see [`crate::properties::size_drag`]).
fn size_row(
    ui: &mut Ui,
    state: &mut AppState,
    label: &str,
    tid: ElementTypeId,
    field: &str,
    value: f64,
    set: impl FnOnce(&mut TypeData, f64),
) {
    let mut v = value;
    let r = row(ui, label, |ui| ui.add(crate::properties::size_drag(&mut v).range(0.001..=20.0)));
    if r.changed() {
        update(state, tid, field, |t| set(&mut t.data, v));
    }
    finish(state, &r);
}

/// A small length as shown in a read-only row (millimeters, inches in imperial projects).
fn small(v: f64) -> String {
    cc_views::format::size(v, cc_views::format::MetricSize::Mm(1))
}

/// A pipe type: its series and size (or *Free*), the size's outside and
/// inside diameters, wall and pressure class read-only; a free type keeps
/// its diameter and an inside diameter of its own. *Load from catalog…*
/// opens the Pipe Catalog over these properties.
fn pipe_rows(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let TypeData::Pipe { diameter, series, size, inside, .. } = &ty.data else { return };
    let tid = ty.id;
    let project = state.project();
    let all: Vec<(cc_core::PipeSeriesId, String)> =
        project.pipe_series.values().map(|s| (s.id, s.name.clone())).collect();
    let row_of = project.pipe_row(&ty.data).map(|(s, r)| (s.clone(), r.clone()));
    section(ui, "type-pipe-series", t!("section-pipe-series"), |ui| {
        let current = series.and_then(|id| all.iter().find(|(s, _)| *s == id)).map(|(_, n)| n.clone());
        let mut choice = *series;
        let mut open_catalog = false;
        row(ui, &t!("col-pipe-series"), |ui| {
            egui::ComboBox::from_id_salt(("pipe-series", tid.0))
                .selected_text(current.unwrap_or_else(|| t!("pipe-series-free")))
                .width(260.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, None, t!("pipe-series-free"));
                    for (id, name) in &all {
                        ui.selectable_value(&mut choice, Some(*id), name);
                    }
                    ui.separator();
                    open_catalog = ui.selectable_label(false, t!("pipe-catalog-load-more")).clicked();
                });
        });
        if open_catalog {
            crate::pipe_catalog_dialog::open(state);
        }
        if choice != *series {
            // Another series: the size nearest the type's diameter.
            let near = choice.and_then(|id| state.project().pipe_series.get(&id)).and_then(|s| {
                s.pipes.iter().min_by(|a, b| (a.de - diameter).abs().total_cmp(&(b.de - diameter).abs())).cloned()
            });
            update(state, tid, "pipe-series", |t| {
                if let TypeData::Pipe { series, size, diameter, inside, .. } = &mut t.data {
                    *series = choice;
                    match &near {
                        Some(r) => {
                            size.clone_from(&r.size);
                            *diameter = r.de;
                            *inside = None;
                        }
                        None => size.clear(),
                    }
                }
            });
            state.session.doc.break_merge();
            return;
        }
        if let Some((s, r)) = &row_of {
            let mut picked = size.clone();
            row(ui, &t!("col-pipe-size"), |ui| {
                egui::ComboBox::from_id_salt(("pipe-size", tid.0)).selected_text(picked.clone()).width(260.0).show_ui(
                    ui,
                    |ui| {
                        for p in &s.pipes {
                            ui.selectable_value(&mut picked, p.size.clone(), &p.size);
                        }
                    },
                );
            });
            if picked != *size {
                update(state, tid, "pipe-size", |t| {
                    if let TypeData::Pipe { size, .. } = &mut t.data {
                        *size = picked;
                    }
                });
                state.session.doc.break_merge();
            }
            readonly_row(ui, &t!("col-diameter"), small(r.de));
            readonly_row(ui, &t!("prop-pipe-wall"), small(r.e));
            readonly_row(ui, &t!("col-inside-diameter"), small(r.di));
            if !s.pressure.is_empty() {
                readonly_row(ui, &t!("prop-pipe-pressure"), s.pressure.clone());
            }
            if !s.standard.is_empty() {
                readonly_row(ui, &t!("prop-pipe-standard"), s.standard.clone());
            }
        }
    });
    if row_of.is_some() {
        return;
    }
    section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
        size_row(ui, state, &t!("col-diameter"), tid, "pd", *diameter, |d, v| {
            if let TypeData::Pipe { diameter, .. } = d {
                *diameter = v.clamp(0.005, 2.0);
            }
        });
        // The inside diameter, when known (else sizing estimates it from the outside one).
        let mut v = inside.unwrap_or(*diameter * 0.88);
        let r = row(ui, &t!("col-inside-diameter"), |ui| {
            ui.add(crate::properties::size_drag(&mut v).range(0.001..=(*diameter).max(0.001)))
        });
        if r.changed() {
            update(state, tid, "pi", |t| {
                if let TypeData::Pipe { inside, diameter, .. } = &mut t.data {
                    *inside = Some(v.clamp(0.001, *diameter));
                }
            });
        }
        finish(state, &r);
    });
}

/// Component name and kind, the editor button and this type's parameter values.
fn component_parameters(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let TypeData::Component { component, def } = &ty.data else { return };
    let tid = ty.id;
    section(ui, "type-component", t!("section-component"), |ui| {
        readonly_row(ui, &t!("col-component"), component.clone());
        readonly_row(ui, &t!("col-component-kind"), tr(def.kind.key()));
        let (edit, save) = crate::table::button_row(ui, |ui| {
            (ui.button(t!("action-edit-component")).clicked(), ui.button(t!("action-save-component")).clicked())
        });
        if edit {
            // The dialog's edits are kept and it closes, so it does not stay
            // in front of the environment.
            crate::dialog::session::commit(state);
            state.dialogs.type_editor = None;
            state.dialogs.type_rename = None;
            crate::component_env::open(state, tid);
        }
        if save {
            actions::save_component(state, component);
        }
    });
    let materials: Vec<String> = state.project().materials.iter().map(|m| m.name.clone()).collect();
    section(ui, "type-comp-params", t!("comp-params"), |ui| {
        for (i, p) in def.params.iter().enumerate() {
            let label = if p.instance {
                format!("{} ({})", p.display(), t!("comp-param-instance"))
            } else {
                p.display().to_string()
            };
            let (r, changed) =
                row(ui, &label, |ui| crate::properties::param_editor(ui, p, p.value, &p.text, &materials));
            if let Some(value) = changed {
                update(state, tid, &format!("comp-param-{i}"), |t| {
                    if let TypeData::Component { def, .. } = &mut t.data
                        && let Some(p) = def.params.get_mut(i)
                    {
                        match value {
                            crate::properties::ParamEdit::Number(v) => p.value = v,
                            crate::properties::ParamEdit::Text(t) => p.text = t,
                        }
                    }
                });
            }
            finish(state, &r);
        }
    });
}

fn parameters(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    if matches!(ty.data, TypeData::Component { .. }) {
        component_parameters(ui, state, ty);
        identity(ui, state, tid, &ty.info);
        return;
    }
    let layered = !ty.layers().is_empty();

    section(ui, "type-construction", t!("section-construction"), |ui| {
        if layered {
            let editing = state.dialogs.type_layers_open;
            let label = if editing { t!("action-hide") } else { t!("action-edit") };
            if row(ui, &t!("prop-structure"), |ui| ui.button(label).clicked()) {
                state.dialogs.type_layers_open = !editing;
            }
            if state.dialogs.type_layers_open {
                layers_editor(ui, state, ty);
            }
        }
        if let Some(function) = ty.wall_function() {
            let mut f = function;
            row(ui, &t!("prop-function"), |ui| {
                egui::ComboBox::from_id_salt("wall-function").selected_text(tr(f.key())).show_ui(ui, |ui| {
                    for option in WallFunction::ALL {
                        ui.selectable_value(&mut f, option, tr(option.key()));
                    }
                });
            });
            if f != function {
                update(state, tid, "function", |t| {
                    if let TypeData::Wall { function, .. } = &mut t.data {
                        *function = f;
                    }
                });
            }
        }
        // New walls of the type start structural (each wall keeps its own value).
        if let TypeData::Wall { structural, .. } = ty.data {
            let mut on = structural;
            let r = row(ui, &t!("prop-structural"), |ui| crate::table::checkbox(ui, &mut on))
                .on_hover_text(t!("type-structural-hint"));
            if r.changed() {
                update(state, tid, "structural", |t| {
                    if let TypeData::Wall { structural, .. } = &mut t.data {
                        *structural = on;
                    }
                });
            }
        }
        match ty.data {
            TypeData::Ceiling { thickness, .. } => {
                size_row(ui, state, &t!("prop-thickness"), tid, "thk", thickness, |d, v| {
                    if let TypeData::Ceiling { thickness, .. } = d {
                        *thickness = v;
                    }
                });
            }
            _ if layered => {
                readonly_row(ui, &t!("prop-thickness"), format::length(ty.thickness()));
            }
            _ => {}
        }
    });

    crate::curtain_ui::type_sections(ui, state, ty);
    structure_parameters(ui, state, ty);

    if let TypeData::Door { width, height, .. } | TypeData::Window { width, height, .. } = ty.data {
        section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
            length_row(ui, state, &t!("col-width"), tid, "w", width, |d, v| {
                if let TypeData::Door { width, .. } | TypeData::Window { width, .. } = d {
                    *width = v.max(0.1);
                }
            });
            length_row(ui, state, &t!("col-height"), tid, "h", height, |d, v| {
                if let TypeData::Door { height, .. } | TypeData::Window { height, .. } = d {
                    *height = v.max(0.1);
                }
            });
            if let TypeData::Window { default_sill, .. } = ty.data {
                length_row(ui, state, &t!("prop-sill"), tid, "sill", default_sill, |d, v| {
                    if let TypeData::Window { default_sill, .. } = d {
                        *default_sill = v.max(0.0);
                    }
                });
            }
        });
    }

    if let TypeData::Door { material, .. }
    | TypeData::Window { material, .. }
    | TypeData::Ceiling { material, .. }
    | TypeData::Column { material, .. }
    | TypeData::Beam { material, .. }
    | TypeData::Foundation { material, .. }
    | TypeData::Stair { material, .. }
    | TypeData::Ramp { material, .. }
    | TypeData::Pipe { material, .. }
    | TypeData::Conduit { material, .. }
    | TypeData::CableTray { material, .. }
    | TypeData::Valve { material, .. }
    | TypeData::LightningConductor { material, .. }
    | TypeData::Railing { material, .. }
    | TypeData::Topography { material, .. }
    | TypeData::Sweep { material, .. } = &ty.data
    {
        section(ui, "type-materials", t!("section-materials"), |ui| {
            let mut m = material.clone();
            let changed =
                row(ui, &t!("prop-material"), |ui| crate::dialogs::material_combo(ui, state, "type-material", &mut m));
            if changed {
                update(state, tid, "mat", |t| match &mut t.data {
                    TypeData::Door { material, .. }
                    | TypeData::Window { material, .. }
                    | TypeData::Ceiling { material, .. }
                    | TypeData::Column { material, .. }
                    | TypeData::Beam { material, .. }
                    | TypeData::Foundation { material, .. }
                    | TypeData::Stair { material, .. }
                    | TypeData::Ramp { material, .. }
                    | TypeData::Pipe { material, .. }
                    | TypeData::Conduit { material, .. }
                    | TypeData::CableTray { material, .. }
                    | TypeData::Valve { material, .. }
                    | TypeData::LightningConductor { material, .. }
                    | TypeData::Railing { material, .. }
                    | TypeData::Topography { material, .. }
                    | TypeData::Sweep { material, .. } => *material = m,
                    _ => {}
                });
            }
        });
    }

    if let TypeData::Topography { contour_interval, major_every, .. } = ty.data {
        section(ui, "type-contours", t!("section-contours"), |ui| {
            length_row(ui, state, &t!("prop-contour-interval"), tid, "contour", contour_interval, |d, v| {
                if let TypeData::Topography { contour_interval, .. } = d {
                    *contour_interval = v.max(cc_core::topography::MIN_CONTOUR_INTERVAL);
                }
            });
            let mut k = major_every;
            let r = row(ui, &t!("prop-major-every"), |ui| ui.add(egui::DragValue::new(&mut k).range(1..=100)));
            if r.changed() {
                update(state, tid, "major", |t| {
                    if let TypeData::Topography { major_every, .. } = &mut t.data {
                        *major_every = k.max(1);
                    }
                });
            }
            finish(state, &r);
        });
    }

    crate::level_mark_ui::type_rows(ui, state, tid, &ty.data);
    crate::dim_type_ui::type_sections(ui, state, ty);
    crate::dim_type_ui::text_type_sections(ui, state, ty);
    crate::tag_type_ui::type_sections(ui, state, ty);

    if let TypeData::Pad { cut_slope, fill_slope } = ty.data {
        section(ui, "type-slopes", t!("section-slopes"), |ui| {
            for (label, field, value) in
                [(t!("prop-cut-slope"), "cut", cut_slope), (t!("prop-fill-slope"), "fill", fill_slope)]
            {
                let mut v = value;
                let range = cc_core::pad::MIN_SLOPE..=cc_core::pad::MAX_SLOPE;
                let r =
                    row(ui, &label, |ui| ui.add(egui::DragValue::new(&mut v).speed(0.05).range(range).prefix("1:")));
                if r.changed() {
                    update(state, tid, field, |t| {
                        if let TypeData::Pad { cut_slope, fill_slope } = &mut t.data {
                            *(if field == "cut" { cut_slope } else { fill_slope }) = v;
                        }
                    });
                }
                finish(state, &r);
            }
        });
    }

    crate::region_ui::type_section(ui, state, ty);

    if let TypeData::Roof { covering, framing, .. } = &ty.data {
        section(ui, "type-roofing", t!("section-roofing"), |ui| roofing(ui, state, tid, *covering, framing));
    }

    section(ui, "type-identity", t!("section-identity"), |ui| identity(ui, state, tid, &ty.info));

    section(ui, "type-ifc", t!("section-ifc"), |ui| {
        readonly_row(ui, &t!("prop-ifc-guid"), cc_ifc::guid::encode(ty.uid.0));
    });
}

/// A roof type's covering (from the catalog, with its least slope and
/// weight) and framing: the system, material, and the members' sizes and
/// spacings (rafters and battens only for timber).
fn roofing(
    ui: &mut Ui,
    state: &mut AppState,
    tid: ElementTypeId,
    covering: Option<cc_core::roofing::TileKind>,
    framing: &cc_core::roofing::RoofFraming,
) {
    use cc_core::roofing::{FramingSystem, MemberSize, RoofFraming, TileKind};
    let set = |state: &mut AppState, field: &str, f: &dyn Fn(&mut Option<TileKind>, &mut RoofFraming)| {
        update(state, tid, field, |t| {
            if let TypeData::Roof { covering, framing, .. } = &mut t.data {
                f(covering, framing);
            }
        });
    };
    let mut c = covering;
    let name = |c: Option<TileKind>| c.map_or_else(|| t!("tile-none"), |k| tr(k.key()));
    row(ui, &t!("prop-covering"), |ui| {
        egui::ComboBox::from_id_salt(("covering", tid.0)).selected_text(name(c)).show_ui(ui, |ui| {
            ui.selectable_value(&mut c, None, name(None));
            for k in TileKind::ALL {
                ui.selectable_value(&mut c, Some(k), name(Some(k)));
            }
        });
    });
    if c != covering {
        // `type_set` brings the framing the covering usually lies on, if none was chosen.
        set(state, "covering", &|cov, _| *cov = c);
    }
    if let Some(spec) = covering.map(TileKind::spec) {
        readonly_row(ui, &t!("prop-min-slope"), format::slope(spec.min_slope));
        readonly_row(ui, &t!("prop-covering-weight"), format::quantity(format::Quantity::AreaMass, spec.weight, 0));
        if let Some(n) = spec.per_m2 {
            if format::imperial() {
                let foot = cc_core::units::FOOT;
                readonly_row(ui, &t!("prop-imp-tiles-per-ft2"), format::number(n * foot * foot, 1));
            } else {
                readonly_row(ui, &t!("prop-tiles-per-m2"), format!("{n:.1}"));
            }
        }
    }
    let mut system = framing.system;
    row(ui, &t!("prop-framing"), |ui| {
        egui::ComboBox::from_id_salt(("framing", tid.0)).selected_text(tr(system.key())).show_ui(ui, |ui| {
            for s in FramingSystem::ALL {
                ui.selectable_value(&mut system, s, tr(s.key()));
            }
        });
    });
    if system != framing.system {
        set(state, "framing", &|_, fr| fr.system = system);
    }
    if framing.system == FramingSystem::None {
        return;
    }
    let mut m = framing.material.clone();
    if row(ui, &t!("prop-material"), |ui| crate::dialogs::material_combo(ui, state, ("framing-mat", tid.0), &mut m)) {
        set(state, "framing-mat", &|_, fr| fr.material = m.clone());
    }
    type Pick = fn(&mut RoofFraming) -> &mut MemberSize;
    let timber = matches!(framing.system, FramingSystem::Timber | FramingSystem::TimberTrusses);
    let trusses = framing.system == FramingSystem::TimberTrusses;
    let mut members: Vec<(&str, Pick)> = Vec::new();
    if trusses {
        members.push(("prop-truss", |f| &mut f.truss));
    }
    members.push(("prop-purlin", |f| &mut f.purlin));
    if timber {
        members.push(("prop-rafter", |f| &mut f.rafter));
        members.push(("prop-batten", |f| &mut f.batten));
    }
    for (label, pick) in members {
        let size = *pick(&mut framing.clone());
        for (axis, value) in [("w", size.width), ("h", size.height)] {
            let caption = format!("{} ({})", tr(label), if axis == "w" { t!("col-width") } else { t!("col-height") });
            size_row(ui, state, &caption, tid, &format!("{label}-{axis}"), value, |d, v| {
                if let TypeData::Roof { framing, .. } = d {
                    let s = pick(framing);
                    if axis == "w" {
                        s.width = v.max(0.005);
                    } else {
                        s.height = v.max(0.005);
                    }
                }
            });
        }
    }
    if trusses {
        length_row(ui, state, &t!("prop-truss-spacing"), tid, "truss-sp", framing.truss_spacing, |d, v| {
            if let TypeData::Roof { framing, .. } = d {
                framing.truss_spacing = v.max(0.3);
            }
        });
    }
    length_row(ui, state, &t!("prop-purlin-spacing"), tid, "purlin-sp", framing.purlin_spacing, |d, v| {
        if let TypeData::Roof { framing, .. } = d {
            framing.purlin_spacing = v.max(0.1);
        }
    });
    if timber {
        length_row(ui, state, &t!("prop-rafter-spacing"), tid, "rafter-sp", framing.rafter_spacing, |d, v| {
            if let TypeData::Roof { framing, .. } = d {
                framing.rafter_spacing = v.max(0.1);
            }
        });
    }
}

/// Text fields of the type information.
fn identity(ui: &mut Ui, state: &mut AppState, tid: ElementTypeId, info: &TypeInfo) {
    type Field = (&'static str, fn(&mut TypeInfo) -> &mut String);
    let fields: [Field; 9] = [
        ("info-type-mark", |i| &mut i.type_mark),
        ("info-description", |i| &mut i.description),
        ("info-manufacturer", |i| &mut i.manufacturer),
        ("info-model", |i| &mut i.model),
        ("info-url", |i| &mut i.url),
        ("info-keynote", |i| &mut i.keynote),
        ("info-classification", |i| &mut i.classification),
        ("info-fire-rating", |i| &mut i.fire_rating),
        ("info-type-comments", |i| &mut i.comments),
    ];
    for (label, field) in fields {
        let mut text = field(&mut info.clone()).clone();
        let r = row(ui, &tr(label), |ui| {
            crate::table::live_text(ui, ("type-info", tid, label), &mut text, |ui, t| ui.text_edit_singleline(t))
        });
        if r.changed() {
            update(state, tid, label, |t| *field(&mut t.info) = text);
        }
        finish(state, &r);
    }
    let mut cost = info.cost.unwrap_or(0.0);
    let r = row(ui, &t!("info-cost"), |ui| {
        ui.add(DragValue::new(&mut cost).speed(1.0).range(0.0..=f64::MAX).fixed_decimals(2))
    });
    if r.changed() {
        update(state, tid, "cost", |t| t.info.cost = Some(cost));
    }
    finish(state, &r);
}

/// Layers table: function, material and thickness, with reorder and removal;
/// a wall type's layers also their own vertical extent (see
/// [`cc_core::layer_extent`]): a base offset, and a height or up to the top.
fn layers_editor(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    let layers = ty.layers().to_vec();
    let extents = matches!(ty.data, TypeData::Wall { .. });
    let set_layers = |state: &mut AppState, field: &str, layers: Vec<Layer>| {
        update(state, tid, field, |t| match &mut t.data {
            TypeData::Wall { layers: l, .. } | TypeData::Floor { layers: l } | TypeData::Roof { layers: l, .. } => {
                *l = layers
            }
            _ => {}
        });
    };
    egui::Frame::new().fill(PALETTE.panel).inner_margin(egui::Margin::symmetric(10, 8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        ui.label(theme::muted(t!("type-layers")).size(12.0));
        let columns = if extents { 7 } else { 5 };
        egui::Grid::new(("layers", tid.0)).num_columns(columns).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
            let headers: &[&str] = if extents {
                &["", "col-function", "prop-material", "prop-thickness", "prop-base-offset", "prop-height", ""]
            } else {
                &["", "col-function", "prop-material", "prop-thickness", ""]
            };
            for h in headers {
                ui.label(theme::muted(if h.is_empty() { String::new() } else { tr(h) }).size(11.5));
            }
            ui.end_row();
            for (i, layer) in layers.iter().enumerate() {
                ui.label(theme::muted((i + 1).to_string()));
                let mut f = layer.function;
                egui::ComboBox::from_id_salt(("lfn", tid.0, i)).width(120.0).selected_text(tr(f.key())).show_ui(
                    ui,
                    |ui| {
                        for option in LayerFunction::ALL {
                            ui.selectable_value(&mut f, option, tr(option.key()));
                        }
                    },
                );
                if f != layer.function {
                    let mut l = layers.clone();
                    l[i].function = f;
                    set_layers(state, "lfn", l);
                    state.session.doc.break_merge();
                }
                let mut m = layer.material.clone();
                if crate::dialogs::material_combo(ui, state, ("lmat", tid.0, i), &mut m) {
                    let mut l = layers.clone();
                    l[i].material = m;
                    set_layers(state, "lmat", l);
                    state.session.doc.break_merge();
                }
                let mut thk = layer.thickness;
                let r = ui.add(crate::properties::size_drag(&mut thk).range(0.001..=2.0));
                if r.changed() {
                    let mut l = layers.clone();
                    l[i].thickness = thk;
                    set_layers(state, &format!("lthk{i}"), l);
                }
                finish(state, &r);
                if extents {
                    extent_cells(ui, state, i, &layers, &set_layers);
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let n = layers.len();
                    let moves = [("↑", i > 0, i.wrapping_sub(1), "layer-up"), ("↓", i + 1 < n, i + 1, "layer-down")];
                    for (glyph, enabled, target, tip) in moves {
                        if ui.add_enabled(enabled, egui::Button::new(glyph).small()).on_hover_text(tr(tip)).clicked() {
                            let mut l = layers.clone();
                            l.swap(i, target);
                            set_layers(state, "lmove", l);
                            state.session.doc.break_merge();
                        }
                    }
                    let remove =
                        ui.add_enabled(n > 1, egui::Button::new("×").small()).on_hover_text(t!("layer-remove"));
                    if remove.clicked() {
                        let mut l = layers.clone();
                        l.remove(i);
                        set_layers(state, "lremove", l);
                        state.session.doc.break_merge();
                    }
                });
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            if ui.button(t!("type-add-layer")).clicked() {
                let mut l = layers.clone();
                let material = state.project().materials.first().map(|m| m.name.clone()).unwrap_or_default();
                l.push(Layer { material, thickness: 0.02, function: LayerFunction::Finish, ..Default::default() });
                set_layers(state, "ladd", l);
                state.session.doc.break_merge();
            }
            let total = format!("{}: {}", t!("type-total-thickness"), format::length(ty.thickness()));
            ui.label(theme::muted(total));
        });
    });
}

/// A wall layer's own extent in the layers table: its base offset from the
/// wall's bottom, and its height (a checkbox to give it one; without, up to
/// the wall's top).
fn extent_cells(
    ui: &mut Ui,
    state: &mut AppState,
    i: usize,
    layers: &[Layer],
    set_layers: &dyn Fn(&mut AppState, &str, Vec<Layer>),
) {
    use cc_core::layer_extent::{MAX_HEIGHT, MAX_OFFSET, MIN_HEIGHT};
    let Some(layer) = layers.get(i) else { return };
    let with = |f: &dyn Fn(&mut Layer)| {
        let mut l = layers.to_vec();
        if let Some(x) = l.get_mut(i) {
            f(x);
        }
        l
    };
    let mut offset = layer.base_offset;
    let r = ui.add(crate::properties::length_drag(&mut offset).range(-MAX_OFFSET..=MAX_OFFSET));
    if r.changed() {
        set_layers(state, &format!("loff{i}"), with(&|x| x.base_offset = offset));
    }
    finish(state, &r);
    let mut on = layer.height.is_some();
    // Given a height, a layer starts at the project's storey height, as tall as a wall usually is.
    let mut height = layer.height.unwrap_or_else(|| state.project().defaults().story);
    let (toggled, dragged) = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let toggled = crate::table::checkbox(ui, &mut on).changed();
            let dragged = if on {
                Some(ui.add(crate::properties::length_drag(&mut height).range(MIN_HEIGHT..=MAX_HEIGHT)))
            } else {
                ui.label(theme::muted(t!("layer-to-top")));
                None
            };
            (toggled, dragged)
        })
        .inner;
    if toggled {
        set_layers(state, "lheight", with(&|x| x.height = on.then_some(height)));
        state.session.doc.break_merge();
    }
    if let Some(r) = dragged {
        if r.changed() {
            set_layers(state, &format!("lh{i}"), with(&|x| x.height = Some(height)));
        }
        finish(state, &r);
    }
}

/// A pile cap type's piles: the layout (read only), spacing, edge distance,
/// and the piles' length and embedment.
fn pile_cap_rows(ui: &mut Ui, state: &mut AppState, tid: ElementTypeId, cap: &cc_core::pile::PileCap) {
    fn set(d: &mut TypeData) -> Option<&mut cc_core::pile::PileCap> {
        match d {
            TypeData::Foundation { pile_cap: Some(cap), .. } => Some(cap),
            _ => None,
        }
    }
    readonly_row(ui, &t!("prop-pile-layout"), format!("{} ({})", tr(cap.layout.key()), cap.layout.count()));
    if !matches!(cap.layout, cc_core::pile::PileLayout::Single | cc_core::pile::PileLayout::Circle { .. }) {
        length_row(ui, state, &t!("prop-pile-spacing"), tid, "pcs", cap.spacing, |d, v| {
            if let Some(c) = set(d) {
                c.spacing = v.clamp(cc_core::pile::MIN_SPACING, cc_core::pile::MAX_SPACING);
            }
        });
    }
    if !matches!(cap.layout, cc_core::pile::PileLayout::Circle { ring_width: Some(_), .. }) {
        length_row(ui, state, &t!("prop-edge-distance"), tid, "pce", cap.edge_distance, |d, v| {
            if let Some(c) = set(d) {
                c.edge_distance = v.clamp(cc_core::pile::MIN_EDGE_DISTANCE, cc_core::pile::MAX_EDGE_DISTANCE);
            }
        });
    }
    length_row(ui, state, &t!("prop-pile-length"), tid, "pcl", cap.pile_length, |d, v| {
        if let Some(c) = set(d) {
            c.pile_length = v.clamp(cc_core::pile::MIN_PILE_LENGTH, cc_core::pile::MAX_PILE_LENGTH);
        }
    });
    length_row(ui, state, &t!("prop-embedment"), tid, "pcm", cap.embedment, |d, v| {
        if let Some(c) = set(d) {
            c.embedment = v.clamp(0.0, cc_core::pile::MAX_EMBEDMENT);
        }
    });
}

/// Section and layout parameters of columns, beams, stairs, ramps and railings.
/// A strip footing type's beam on top (an inverted T): a checkbox, and with
/// it the edge thickness of the footing and the beam's width and height.
fn strip_haunch_rows(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    let TypeData::Foundation { width, thickness, haunch, .. } = ty.data else { return };
    crate::table::section(ui, "type-haunch", t!("section-strip-beam"), |ui| {
        let mut on = haunch.is_some();
        if row(ui, &t!("prop-strip-beam"), |ui| crate::table::checkbox(ui, &mut on)).changed() {
            update(state, tid, "fh", |t| {
                if let TypeData::Foundation { haunch, .. } = &mut t.data {
                    // A beam a third of the strip wide and as high as the footing is thick.
                    let beam = cc_core::StripHaunch {
                        edge_thickness: thickness,
                        beam_width: width / 3.0,
                        beam_height: thickness,
                    };
                    *haunch = on.then_some(beam);
                }
            });
            state.session.doc.break_merge();
        }
        let Some(h) = haunch else { return };
        type Set = fn(&mut cc_core::StripHaunch, f64);
        let rows: [(String, &str, f64, Set); 3] = [
            (t!("prop-strip-edge"), "fhe", h.edge_thickness, |h, v| h.edge_thickness = v),
            (t!("prop-strip-beam-width"), "fhw", h.beam_width, |h, v| h.beam_width = v),
            (t!("prop-strip-beam-height"), "fhh", h.beam_height, |h, v| h.beam_height = v),
        ];
        for (label, key, value, set) in rows {
            size_row(ui, state, &label, tid, key, value, move |d, v| {
                if let TypeData::Foundation { width, thickness, haunch: Some(h), .. } = d {
                    // Kept within what makes a section.
                    let mut next = *h;
                    set(&mut next, v.max(0.0));
                    if next.fits(*width, *thickness) {
                        *h = next;
                    }
                }
            });
        }
    });
}

fn structure_parameters(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    match ty.data {
        TypeData::Column { .. } | TypeData::Beam { .. } => profile_rows(ui, state, ty),
        TypeData::Foundation { kind, width, length, thickness, ref pile_cap, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                let mut k = kind;
                row(ui, &t!("prop-foundation-kind"), |ui| {
                    egui::ComboBox::from_id_salt(("foundation-kind", tid.0)).selected_text(tr(k.key())).show_ui(
                        ui,
                        |ui| {
                            for option in cc_core::FoundationKind::ALL {
                                ui.selectable_value(&mut k, option, tr(option.key()));
                            }
                        },
                    );
                });
                if k != kind {
                    update(state, tid, "fk", |t| {
                        if let TypeData::Foundation { kind, pile_cap, .. } = &mut t.data {
                            *kind = k;
                            // A pile cap holds its piles' layout; other kinds none.
                            *pile_cap =
                                (k == cc_core::FoundationKind::PileCap).then(|| pile_cap.clone().unwrap_or_default());
                        }
                    });
                    state.session.doc.break_merge();
                }
                // A pile cap's size follows from its layout.
                if !matches!(kind, cc_core::FoundationKind::Slab | cc_core::FoundationKind::PileCap) {
                    length_row(ui, state, &t!("col-width"), tid, "fw", width, |d, v| {
                        if let TypeData::Foundation { width, .. } = d {
                            *width = v.max(0.05);
                        }
                    });
                }
                if kind == cc_core::FoundationKind::Pad {
                    length_row(ui, state, &t!("prop-length"), tid, "fl", length, |d, v| {
                        if let TypeData::Foundation { length, .. } = d {
                            *length = v.max(0.05);
                        }
                    });
                }
                size_row(
                    ui,
                    state,
                    &crate::properties::field_label(cc_core::ScheduleField::Thickness),
                    tid,
                    "ft",
                    thickness,
                    |d, v| {
                        if let TypeData::Foundation { thickness, .. } = d {
                            *thickness = v.max(0.05);
                        }
                    },
                );
                if let Some(cap) = pile_cap {
                    pile_cap_rows(ui, state, tid, cap);
                }
            });
            if kind == cc_core::FoundationKind::Strip {
                strip_haunch_rows(ui, state, ty);
            }
        }
        TypeData::Stair { width, max_riser, tread, build, rules, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                length_row(ui, state, &t!("col-width"), tid, "sw", width, |d, v| {
                    if let TypeData::Stair { width, .. } = d {
                        *width = v.max(0.3);
                    }
                });
                size_row(ui, state, &t!("prop-max-riser"), tid, "sr", max_riser, |d, v| {
                    if let TypeData::Stair { max_riser, .. } = d {
                        *max_riser = v.clamp(0.05, 0.5);
                    }
                });
                size_row(ui, state, &t!("col-tread"), tid, "st", tread, |d, v| {
                    if let TypeData::Stair { tread, .. } = d {
                        *tread = v.clamp(0.1, 1.0);
                    }
                });
            });
            crate::table::section(ui, "type-stair-build", t!("section-stair-build"), |ui| {
                let mut b = build;
                let edited = crate::stair_ui::build_rows(ui, tid.0, &mut b);
                if edited.changed {
                    update(state, tid, "sb", |t| {
                        if let TypeData::Stair { build, .. } = &mut t.data {
                            *build = b;
                        }
                    });
                }
                if edited.done {
                    state.session.doc.break_merge();
                }
            });
            crate::table::section(ui, "type-stair-rules", t!("section-stair-rules"), |ui| {
                let mut r = rules;
                let edited = crate::stair_ui::rules_rows(ui, tid.0, &mut r, max_riser, tread);
                if edited.changed {
                    update(state, tid, "sq", |t| {
                        if let TypeData::Stair { rules, .. } = &mut t.data {
                            *rules = r;
                        }
                    });
                }
                if edited.done {
                    state.session.doc.break_merge();
                }
            });
        }
        TypeData::Pipe { .. } => pipe_rows(ui, state, ty),
        TypeData::Conduit { diameter, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                size_row(ui, state, &t!("col-diameter"), tid, "pd", diameter, |d, v| {
                    if let TypeData::Conduit { diameter, .. } = d {
                        *diameter = v.clamp(0.005, 2.0);
                    }
                });
            });
        }
        TypeData::LightningConductor { diameter, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                use cc_core::lightning::{MAX_CONDUCTOR_DIAMETER, MIN_CONDUCTOR_DIAMETER};
                size_row(ui, state, &t!("col-diameter"), tid, "cd", diameter, |d, v| {
                    if let TypeData::LightningConductor { diameter, .. } = d {
                        *diameter = v.clamp(MIN_CONDUCTOR_DIAMETER, MAX_CONDUCTOR_DIAMETER);
                    }
                });
            });
        }
        TypeData::CableTray { width, height, style, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                size_row(ui, state, &t!("col-width"), tid, "tw", width, |d, v| {
                    if let TypeData::CableTray { width, .. } = d {
                        *width = v.clamp(0.02, cc_core::mep::MAX_TRAY_WIDTH);
                    }
                });
                size_row(ui, state, &t!("col-height"), tid, "th", height, |d, v| {
                    if let TypeData::CableTray { height, .. } = d {
                        *height = v.clamp(0.01, cc_core::mep::MAX_TRAY_HEIGHT);
                    }
                });
                let mut s = style;
                row(ui, &t!("prop-tray-style"), |ui| {
                    egui::ComboBox::from_id_salt(("tray-style", tid.0)).selected_text(tr(s.key())).show_ui(ui, |ui| {
                        for option in cc_core::TrayStyle::ALL {
                            ui.selectable_value(&mut s, option, tr(option.key()));
                        }
                    });
                });
                if s != style {
                    update(state, tid, "tray-style", |t| {
                        if let TypeData::CableTray { style, .. } = &mut t.data {
                            *style = s;
                        }
                    });
                }
            });
        }
        TypeData::Valve { kind, .. } => {
            crate::table::section(ui, "type-valve", t!("section-mep"), |ui| {
                let mut k = kind;
                row(ui, &t!("prop-valve-kind"), |ui| {
                    egui::ComboBox::from_id_salt(("valve-kind", tid.0)).selected_text(tr(k.key())).show_ui(ui, |ui| {
                        for option in cc_core::ValveKind::ALL {
                            ui.selectable_value(&mut k, option, tr(option.key()));
                        }
                    });
                });
                if k != kind {
                    update(state, tid, "valve-kind", |t| {
                        if let TypeData::Valve { kind, .. } = &mut t.data {
                            *kind = k;
                        }
                    });
                }
            });
        }
        TypeData::Ramp { width, thickness, rules, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                length_row(ui, state, &t!("col-width"), tid, "rw", width, |d, v| {
                    if let TypeData::Ramp { width, .. } = d {
                        *width = v.clamp(0.3, 50.0);
                    }
                });
                size_row(
                    ui,
                    state,
                    &crate::properties::field_label(cc_core::ScheduleField::Thickness),
                    tid,
                    "rt",
                    thickness,
                    |d, v| {
                        if let TypeData::Ramp { thickness, .. } = d {
                            *thickness = v.clamp(0.01, 5.0);
                        }
                    },
                );
            });
            crate::table::section(ui, "type-ramp-rules", t!("section-ramp-rules"), |ui| {
                let mut r = rules;
                let edited = crate::stair_ui::ramp_rules_rows(ui, &mut r);
                if edited.changed {
                    update(state, tid, "rq", |t| {
                        if let TypeData::Ramp { rules, .. } = &mut t.data {
                            *rules = r;
                        }
                    });
                }
                if edited.done {
                    state.session.doc.break_merge();
                }
            });
        }
        TypeData::Sweep { ref profile, .. } => {
            // The profile's extent; *Edit Profile* draws it in a sketch.
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                let (lo, hi) = profile
                    .iter()
                    .fold((glam::DVec2::splat(f64::INFINITY), glam::DVec2::splat(f64::NEG_INFINITY)), |(lo, hi), p| {
                        (lo.min(*p), hi.max(*p))
                    });
                if lo.is_finite() && hi.is_finite() {
                    crate::table::readonly_row(ui, &t!("col-width"), cc_views::format::length(hi.x - lo.x));
                    crate::table::readonly_row(ui, &t!("col-height"), cc_views::format::length(hi.y - lo.y));
                }
                crate::table::readonly_row(ui, &t!("prop-sweep-points"), profile.len().to_string());
                ui.add_space(4.0);
                if ui.button(t!("edit-profile")).on_hover_text(t!("cmd-desc-edit-profile")).clicked() {
                    // The dialog's edits are kept; the profile opens in the sketch.
                    crate::dialog::session::commit(state);
                    state.dialogs.type_editor = None;
                    state.dialogs.type_rename = None;
                    crate::profile_sketch::open(state, tid);
                }
            });
        }
        TypeData::Railing { height, post_spacing, .. } => {
            crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
                length_row(ui, state, &t!("col-height"), tid, "rh", height, |d, v| {
                    if let TypeData::Railing { height, .. } = d {
                        *height = v.clamp(0.3, 3.0);
                    }
                });
                length_row(ui, state, &t!("prop-post-spacing"), tid, "rp", post_spacing, |d, v| {
                    if let TypeData::Railing { post_spacing, .. } = d {
                        *post_spacing = v.clamp(0.1, 5.0);
                    }
                });
            });
        }
        _ => {}
    }
}
/// The profile a column or beam type uses, how it is turned and mirrored.
fn profile_of(data: &TypeData) -> Option<(Option<ProfileId>, f64, bool)> {
    match data {
        TypeData::Column { profile, profile_rotation_deg, profile_mirror, .. }
        | TypeData::Beam { profile, profile_rotation_deg, profile_mirror, .. } => {
            Some((*profile, *profile_rotation_deg, *profile_mirror))
        }
        _ => None,
    }
}

/// Change the profile fields of a column or beam type as `f` does.
fn set_profile(t: &mut ElementType, f: impl FnOnce(&mut Option<ProfileId>, &mut f64, &mut bool)) {
    if let TypeData::Column { profile, profile_rotation_deg, profile_mirror, .. }
    | TypeData::Beam { profile, profile_rotation_deg, profile_mirror, .. } = &mut t.data
    {
        f(profile, profile_rotation_deg, profile_mirror);
    }
}

/// A profile's depth and width (`d × bf`) in millimeters (in inches in
/// imperial projects), unturned.
fn profile_size(project: &cc_core::Project, id: ProfileId) -> Option<String> {
    let outline = project.profile(id)?.shape.outline().ok()?;
    let (lo, hi) = outline.bbox();
    if format::imperial() {
        let i = format::inch_size;
        return Some(format!("{} × {}", i(hi.y - lo.y), i(hi.x - lo.x)));
    }
    let mm = |v: f64| {
        let v = v * 1000.0;
        format::number(v, if (v - v.round()).abs() < 0.05 { 0 } else { 1 })
    };
    Some(format!("{} × {} {}", mm(hi.y - lo.y), mm(hi.x - lo.x), cc_views::format::unit_symbol("mm")))
}

/// The project's profiles grouped by material with a search box; returns
/// the one clicked.
fn profile_combo(
    ui: &mut Ui,
    project: &cc_core::Project,
    tid: ElementTypeId,
    current: Option<ProfileId>,
) -> Option<ProfileId> {
    let name = current.and_then(|id| project.profile(id)).map_or_else(|| "—".to_string(), |p| p.name.clone());
    let search_id = egui::Id::new(("type-profile-search", tid.0));
    let mut picked = None;
    let combo = egui::ComboBox::from_id_salt(("type-profile", tid.0)).selected_text(name).width(200.0).height(360.0);
    combo.show_ui(ui, |ui| {
        let mut search = ui.ctx().data(|d| d.get_temp::<String>(search_id)).unwrap_or_default();
        ui.add(egui::TextEdit::singleline(&mut search).hint_text(t!("profiles-search")).desired_width(180.0));
        let needle = search.trim().to_lowercase();
        for m in ProfileMaterial::ALL {
            let mut rows: Vec<(ProfileId, &str)> = project
                .profiles
                .values()
                .filter(|p| p.material == m && p.name.to_lowercase().contains(&needle))
                .map(|p| (p.id, p.name.as_str()))
                .collect();
            if rows.is_empty() {
                continue;
            }
            rows.sort_by_key(|(_, n)| n.to_lowercase());
            ui.label(theme::muted(tr(m.key())).size(11.0));
            for (id, name) in rows {
                if ui.selectable_label(current == Some(id), name).clicked() {
                    picked = Some(id);
                }
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(search_id, search));
    });
    picked
}

/// A column's or beam's section: its profile (picked from the project's;
/// *Manage…* opens the Profile Manager), turned and mirrored, and the
/// profile's size.
fn profile_rows(ui: &mut Ui, state: &mut AppState, ty: &ElementType) {
    let tid = ty.id;
    let Some((profile, rotation, mirror)) = profile_of(&ty.data) else { return };
    crate::table::section(ui, "type-dimensions", t!("section-dimensions"), |ui| {
        let mut picked = None;
        let manage = row(ui, &t!("col-profile"), |ui| {
            ui.horizontal(|ui| {
                picked = profile_combo(ui, state.project(), tid, profile);
                ui.button(t!("type-profile-manage")).clicked()
            })
            .inner
        });
        if let Some(id) = picked.filter(|id| Some(*id) != profile) {
            update(state, tid, "profile", |t| set_profile(t, |p, _, _| *p = Some(id)));
            state.session.doc.break_merge();
        }
        if manage {
            crate::profiles_dialog::open(state);
            if let Some(d) = state.dialogs.profiles.as_mut() {
                d.selected = profile;
            }
        }
        let mut r = rotation;
        let resp = row(ui, &t!("prop-rotation"), |ui| {
            ui.add(DragValue::new(&mut r).speed(1.0).range(-180.0..=180.0).max_decimals(2).suffix("°"))
        });
        if resp.changed() {
            update(state, tid, "profile-rotation", |t| set_profile(t, |_, angle, _| *angle = r));
        }
        finish(state, &resp);
        let mut m = mirror;
        if row(ui, &t!("prop-mirrored"), |ui| ui.checkbox(&mut m, "")).changed() {
            update(state, tid, "profile-mirror", |t| set_profile(t, |_, _, flip| *flip = m));
            state.session.doc.break_merge();
        }
        let size = profile.and_then(|id| profile_size(state.project(), id)).unwrap_or_else(|| "—".to_string());
        readonly_row(ui, &t!("type-profile-size"), size);
    });
}
