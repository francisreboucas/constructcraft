//! Material browser: the project's materials in a searchable list and the
//! selected one's identity, graphics, appearance and physical properties
//! in tabs. Edits are undoable and the dialog's Cancel reverts them all.

use crate::dialog::{Dialog, session};
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::theme::{self, PALETTE};
use cc_core::Material;
use cc_core::material::{Behavior, Finish, MaterialClass, PatternFill};
use cc_engine::Ref;
use cc_engine::commands::materials;
use cc_views::format;
use egui::{Color32, Context, DragValue, Pos2, Rect, RichText, Sense, Stroke, Ui, vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Identity,
    Graphics,
    Appearance,
    Physical,
}

/// Browser state kept between frames.
#[derive(Debug, Clone, Default)]
pub struct Browser {
    pub selected: Option<String>,
    pub tab: Tab,
    pub search: String,
    /// Name being typed, committed on Enter or when the field loses focus.
    pub name_edit: Option<String>,
    pub error: Option<String>,
}

fn key(parts: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// Change material `name`, merging consecutive edits of one field.
fn update(state: &mut AppState, name: &str, field: &str, f: impl FnOnce(&mut Material)) {
    crate::engine_edit::material(state, name, Some(key((name, field))), f);
}

fn done(state: &mut AppState, r: &egui::Response) {
    if r.drag_stopped() || r.lost_focus() {
        state.session.doc.break_merge();
    }
}

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.dialogs.materials {
        return;
    }
    session::begin(state);
    let mut ok = 0;
    let title = match &state.dialogs.material_browser.selected {
        Some(name) => format!("{} — {name}", t!("dlg-materials")),
        None => t!("dlg-materials"),
    };
    let response = Dialog::new("materials", title).width(880.0).show(
        ctx,
        |ui| body(ui, state),
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    if crate::dialogs::ok_cancel(state, &response, ok) {
        state.dialogs.materials = false;
        state.dialogs.material_browser.name_edit = None;
        state.dialogs.material_browser.error = None;
    }
}

fn body(ui: &mut Ui, state: &mut AppState) {
    // Keep a valid selection.
    let exists =
        |state: &AppState, n: &Option<String>| n.as_ref().is_some_and(|n| state.project().material(n).is_some());
    if !exists(state, &state.dialogs.material_browser.selected) {
        state.dialogs.material_browser.selected = state.project().materials.first().map(|m| m.name.clone());
    }
    // Fit the window: header, footer, margins and the gap around the dialog take about 280 points.
    let height = (ui.ctx().content_rect().height() - 280.0).clamp(300.0, 560.0);
    // Two fixed-width, top-down columns. (A plain `allocate_ui` here would
    // inherit the row's horizontal layout and line every field up side by
    // side, making the dialog far wider than the screen.)
    let list_w = 270.0;
    // What the list, the divider and the gaps around it leave.
    const DIVIDER_W: f32 = 9.0;
    let gaps = 2.0 * ui.spacing().item_spacing.x;
    let details_w = (ui.available_width() - list_w - DIVIDER_W - gaps).max(300.0);
    let column = egui::Layout::top_down(egui::Align::Min);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(list_w, height), column, |ui| {
            ui.set_width(list_w);
            ui.set_min_height(height);
            list(ui, state, height);
        });
        // A fixed-height divider: `ui.separator()` in a row stretches to the
        // available height, which is unbounded in a modal.
        let (divider, _) = ui.allocate_exact_size(vec2(DIVIDER_W, height), Sense::hover());
        ui.painter().vline(divider.center().x, divider.y_range(), Stroke::new(1.0, PALETTE.border));
        ui.allocate_ui_with_layout(vec2(details_w, height), column, |ui| {
            ui.set_width(details_w);
            ui.set_min_height(height);
            let selected = state.dialogs.material_browser.selected.clone();
            match selected.and_then(|n| state.project().material(&n).cloned()) {
                Some(m) => details(ui, state, &m, height),
                None => {
                    ui.label(theme::muted(t!("mat-none")));
                }
            }
        });
    });
}

/// Small swatch: the shading color with the surface pattern over it.
fn swatch(painter: &egui::Painter, rect: Rect, m: &Material, project: &cc_core::Project) {
    let [r, g, b] = m.shading_color();
    painter.rect_filled(rect, 3.0, Color32::from_rgb(r, g, b));
    let fill = if m.graphics.surface.is_empty() { &m.graphics.cut } else { &m.graphics.surface };
    paint_pattern(&painter.with_clip_rect(rect), rect.shrink(1.0), fill, 6.0, false, project);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, PALETTE.border_strong), egui::StrokeKind::Inside);
}

/// Paint a pattern fill into `rect` with lines `spacing` points apart.
fn paint_pattern(
    painter: &egui::Painter,
    rect: Rect,
    fill: &PatternFill,
    spacing: f64,
    background: bool,
    project: &cc_core::Project,
) {
    if background {
        painter.rect_filled(rect, 0.0, Color32::WHITE);
    }
    for (pattern, color) in [(fill.background, fill.background_color), (fill.foreground, fill.foreground_color)] {
        crate::fill_patterns_dialog::paint(painter, rect, project, pattern, color, spacing);
    }
}

fn list(ui: &mut Ui, state: &mut AppState, height: f32) {
    let b = &mut state.dialogs.material_browser;
    ui.add(egui::TextEdit::singleline(&mut b.search).hint_text(t!("mat-search")).desired_width(f32::INFINITY));
    ui.add_space(4.0);
    let query = b.search.trim().to_lowercase();
    let selected = b.selected.clone();
    let mats: Vec<Material> = state
        .project()
        .materials
        .iter()
        .filter(|m| {
            query.is_empty()
                || m.name.to_lowercase().contains(&query)
                || m.identity.keywords.to_lowercase().contains(&query)
                || tr(m.identity.class.key()).to_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    let mut pick = None;
    egui::Frame::new().stroke(Stroke::new(1.0, PALETTE.border)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt("mat-list").max_height(height - 80.0).auto_shrink([false, false]).show(
            ui,
            |ui| {
                for m in &mats {
                    let is_sel = selected.as_deref() == Some(m.name.as_str());
                    let (rect, r) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, is_sel, &m.name)
                    });
                    if is_sel {
                        ui.painter().rect_filled(rect, 0.0, PALETTE.accent_soft);
                    } else if r.hovered() {
                        ui.painter().rect_filled(rect, 0.0, PALETTE.hover);
                    }
                    let sw = Rect::from_min_size(rect.left_top() + vec2(6.0, 5.0), vec2(24.0, 24.0));
                    swatch(ui.painter(), sw, m, state.project());
                    ui.painter().text(
                        rect.left_center() + vec2(38.0, -6.0),
                        egui::Align2::LEFT_CENTER,
                        &m.name,
                        egui::FontId::proportional(13.0),
                        PALETTE.text,
                    );
                    ui.painter().text(
                        rect.left_center() + vec2(38.0, 8.0),
                        egui::Align2::LEFT_CENTER,
                        tr(m.identity.class.key()),
                        egui::FontId::proportional(10.5),
                        PALETTE.text_faint,
                    );
                    if r.clicked() {
                        pick = Some(m.name.clone());
                    }
                }
            },
        );
    });
    if let Some(name) = pick {
        let b = &mut state.dialogs.material_browser;
        b.selected = Some(name);
        b.name_edit = None;
        b.error = None;
    }
    ui.add_space(6.0);
    let users = selected.as_deref().map_or(0, |n| state.project().material_users(n).len());
    ui.horizontal_wrapped(|ui| {
        if ui.button(t!("mat-new")).clicked() {
            let command = materials::MaterialCreate { name: Some(t!("mat-new-name")), color: None };
            if let Some(name) =
                crate::actions::run_command(state, command).and_then(|o| o["name"].as_str().map(String::from))
            {
                state.dialogs.material_browser.selected = Some(name);
            }
        }
        if ui.add_enabled(selected.is_some(), egui::Button::new(t!("mat-duplicate"))).clicked()
            && let Some(src) = selected.clone()
        {
            let command = materials::MaterialDuplicate { material: Ref::Name(src), name: None };
            if let Some(name) =
                crate::actions::run_command(state, command).and_then(|o| o["name"].as_str().map(String::from))
            {
                state.dialogs.material_browser.selected = Some(name);
            }
        }
        let delete = ui.add_enabled(selected.is_some() && users == 0, egui::Button::new(t!("mat-delete")));
        let delete = if users > 0 { delete.on_disabled_hover_text(t!("mat-in-use", "count" => users)) } else { delete };
        if delete.clicked()
            && let Some(name) = selected
        {
            crate::actions::run_command(state, materials::MaterialDelete { material: Ref::Name(name) });
            state.dialogs.material_browser.selected = None;
        }
        if ui.button(t!("mat-import-brazil")).on_hover_text(t!("mat-import-brazil-tip")).clicked() {
            let cmd = materials::MaterialLibraryImport { library: Some("brazil".into()), kind: None };
            if let Some(out) = crate::actions::run_command(state, cmd)
                && let Some(first) = out["imported"].as_array().and_then(|a| a.first()).and_then(|v| v.as_str())
            {
                state.dialogs.material_browser.selected = Some(first.to_string());
            }
        }
    });
}

fn details(ui: &mut Ui, state: &mut AppState, m: &Material, height: f32) {
    ui.horizontal(|ui| {
        for (tab, k) in [
            (Tab::Identity, "mat-tab-identity"),
            (Tab::Graphics, "mat-tab-graphics"),
            (Tab::Appearance, "mat-tab-appearance"),
            (Tab::Physical, "mat-tab-physical"),
        ] {
            let on = state.dialogs.material_browser.tab == tab;
            let text = RichText::new(tr(k)).family(if on { theme::semibold() } else { egui::FontFamily::Proportional });
            if ui.selectable_label(on, text).clicked() {
                state.dialogs.material_browser.tab = tab;
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt("mat-details").max_height(height - 40.0).auto_shrink([false, false]).show(
        ui,
        |ui| match state.dialogs.material_browser.tab {
            Tab::Identity => identity(ui, state, m),
            Tab::Graphics => graphics(ui, state, m),
            Tab::Appearance => appearance(ui, state, m),
            Tab::Physical => physical(ui, state, m),
        },
    );
}

fn heading(ui: &mut Ui, text: String) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).family(theme::semibold()).color(PALETTE.text));
    ui.add_space(2.0);
}

fn grid(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([12.0, 6.0]).min_col_width(150.0).show(ui, add);
}

fn text_row(
    ui: &mut Ui,
    state: &mut AppState,
    m: &Material,
    label: String,
    field: &str,
    get: fn(&Material) -> &String,
    set: fn(&mut Material, String),
) {
    ui.label(theme::muted(label));
    let mut text = get(m).clone();
    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(320.0));
    if r.changed() {
        update(state, &m.name, field, |x| set(x, text));
    }
    done(state, &r);
    ui.end_row();
}

fn identity(ui: &mut Ui, state: &mut AppState, m: &Material) {
    grid(ui, "mat-name", |ui| {
        ui.label(theme::muted(t!("mat-name")));
        let b = &mut state.dialogs.material_browser;
        let mut name = b.name_edit.clone().unwrap_or_else(|| m.name.clone());
        let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(320.0));
        if r.changed() {
            b.name_edit = Some(name.clone());
        }
        let enter = r.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && let Some(new) = state.dialogs.material_browser.name_edit.take() {
            let new = new.trim().to_string();
            if new != m.name {
                let old = m.name.clone();
                let command = materials::MaterialRename { material: Ref::Name(old), name: new.clone() };
                let renamed = state.session.run(command).is_ok();
                let b = &mut state.dialogs.material_browser;
                if renamed {
                    b.selected = Some(new);
                    b.error = None;
                } else {
                    b.error = Some(t!("mat-name-taken"));
                }
            }
        }
        ui.end_row();
    });
    if let Some(e) = &state.dialogs.material_browser.error {
        ui.label(RichText::new(e).color(PALETTE.danger));
    }
    heading(ui, t!("mat-descriptive"));
    grid(ui, "mat-desc", |ui| {
        text_row(
            ui,
            state,
            m,
            t!("info-description"),
            "desc",
            |m| &m.identity.description,
            |m, v| m.identity.description = v,
        );
        ui.label(theme::muted(t!("mat-class")));
        let mut class = m.identity.class;
        egui::ComboBox::from_id_salt("mat-class").selected_text(tr(class.key())).width(200.0).show_ui(ui, |ui| {
            for c in MaterialClass::ALL {
                ui.selectable_value(&mut class, c, tr(c.key()));
            }
        });
        if class != m.identity.class {
            update(state, &m.name, "class", |x| x.identity.class = class);
        }
        ui.end_row();
        text_row(
            ui,
            state,
            m,
            t!("info-comments"),
            "comments",
            |m| &m.identity.comments,
            |m, v| m.identity.comments = v,
        );
        text_row(
            ui,
            state,
            m,
            t!("mat-keywords"),
            "keywords",
            |m| &m.identity.keywords,
            |m, v| m.identity.keywords = v,
        );
    });
    heading(ui, t!("mat-product"));
    grid(ui, "mat-product", |ui| {
        text_row(
            ui,
            state,
            m,
            t!("info-manufacturer"),
            "mfr",
            |m| &m.identity.manufacturer,
            |m, v| m.identity.manufacturer = v,
        );
        text_row(ui, state, m, t!("info-model"), "model", |m| &m.identity.model, |m, v| m.identity.model = v);
        ui.label(theme::muted(t!("info-cost")));
        optional_value(ui, state, m, "cost", m.identity.cost, "", |m, v| m.identity.cost = v);
        ui.end_row();
        text_row(ui, state, m, t!("info-url"), "url", |m| &m.identity.url, |m, v| m.identity.url = v);
    });
    heading(ui, t!("mat-annotation"));
    grid(ui, "mat-annotation", |ui| {
        text_row(ui, state, m, t!("info-keynote"), "keynote", |m| &m.identity.keynote, |m, v| m.identity.keynote = v);
        text_row(ui, state, m, t!("col-mark"), "mark", |m| &m.identity.mark, |m, v| m.identity.mark = v);
    });
}

/// A number that may be unset: a checkbox plus a value.
fn optional_value(
    ui: &mut Ui,
    state: &mut AppState,
    m: &Material,
    field: &str,
    value: Option<f64>,
    unit: &str,
    set: fn(&mut Material, Option<f64>),
) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if crate::table::checkbox(ui, &mut on).changed() {
            update(state, &m.name, field, |x| set(x, if on { Some(value.unwrap_or(0.0)) } else { None }));
        }
        // Densities and strengths read in lb/ft³ and ksi in imperial projects.
        let quantity = match unit {
            "kg/m³" => Some(format::Quantity::Density),
            "MPa" => Some(format::Quantity::Stress),
            _ => None,
        };
        let system = format::system();
        let per = quantity.map_or(1.0, |q| q.per_si(system));
        let unit = quantity.map_or(unit, |q| q.symbol(system));
        if let Some(stored) = value {
            let mut v = stored * per;
            let speed = (v.abs() * 0.01).max(1e-6);
            let small = v != 0.0 && v.abs() < 1e-3;
            let mut drag = DragValue::new(&mut v).speed(speed);
            if small {
                drag = drag
                    .custom_formatter(|n, _| format!("{n:.3e}"))
                    .custom_parser(|s| s.trim().replace(',', ".").parse().ok());
            } else {
                drag = drag.max_decimals(4);
            }
            if !unit.is_empty() {
                drag = drag.suffix(format!(" {unit}"));
            }
            let r = ui.add(drag);
            if r.changed() {
                update(state, &m.name, field, |x| set(x, Some(v / per)));
            }
            done(state, &r);
        } else {
            ui.label(theme::muted(t!("mat-unset")));
        }
    });
}

fn color_row(
    ui: &mut Ui,
    state: &mut AppState,
    m: &Material,
    label: String,
    field: &str,
    value: [u8; 3],
    set: fn(&mut Material, [u8; 3]),
) {
    ui.label(theme::muted(label));
    let mut c = value;
    ui.horizontal(|ui| {
        let r = ui.color_edit_button_srgb(&mut c);
        ui.label(theme::muted(format!("RGB {} {} {}", c[0], c[1], c[2])));
        if r.changed() {
            update(state, &m.name, field, |x| set(x, c));
        }
    });
    ui.end_row();
}

fn pattern_rows(
    ui: &mut Ui,
    state: &mut AppState,
    m: &Material,
    which: &str,
    fill: PatternFill,
    set: fn(&mut Material, PatternFill),
) {
    for (layer, label) in [(0, t!("mat-foreground")), (1, t!("mat-background"))] {
        ui.label(RichText::new(label).color(PALETTE.text_muted).italics());
        ui.end_row();
        let (pattern, color) = if layer == 0 {
            (fill.foreground, fill.foreground_color)
        } else {
            (fill.background, fill.background_color)
        };
        ui.label(theme::muted(t!("mat-pattern")));
        let mut p = pattern;
        crate::fill_patterns_dialog::picker(ui, (which, layer), state.project(), &mut p, 170.0);
        if p != pattern {
            let mut f = fill;
            if layer == 0 {
                f.foreground = p
            } else {
                f.background = p
            }
            update(state, &m.name, &format!("{which}-{layer}-pattern"), |x| set(x, f));
        }
        ui.end_row();
        ui.label(theme::muted(t!("mat-color")));
        let mut c = color;
        if ui.color_edit_button_srgb(&mut c).changed() {
            let mut f = fill;
            if layer == 0 {
                f.foreground_color = c
            } else {
                f.background_color = c
            }
            update(state, &m.name, &format!("{which}-{layer}-color"), |x| set(x, f));
        }
        ui.end_row();
    }
    ui.label(theme::muted(t!("mat-preview")));
    let (rect, _) = ui.allocate_exact_size(vec2(200.0, 60.0), Sense::hover());
    paint_pattern(&ui.painter().with_clip_rect(rect), rect, &fill, 7.0, true, state.project());
    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, PALETTE.border_strong), egui::StrokeKind::Inside);
    ui.end_row();
}

fn graphics(ui: &mut Ui, state: &mut AppState, m: &Material) {
    heading(ui, t!("mat-shading"));
    grid(ui, "mat-shading", |ui| {
        ui.label("");
        let mut use_app = m.graphics.use_appearance;
        if ui.checkbox(&mut use_app, t!("mat-use-appearance")).changed() {
            update(state, &m.name, "use-app", |x| x.graphics.use_appearance = use_app);
        }
        ui.end_row();
        color_row(ui, state, m, t!("mat-color"), "color", m.color, |x, c| x.color = c);
        ui.label(theme::muted(t!("mat-transparency")));
        let mut pct = m.graphics.transparency * 100.0;
        let r = ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %").integer());
        if r.changed() {
            update(state, &m.name, "transparency", |x| x.graphics.transparency = (pct / 100.0).clamp(0.0, 1.0));
        }
        done(state, &r);
        ui.end_row();
    });
    heading(ui, t!("mat-surface-pattern"));
    grid(ui, "mat-surface", |ui| {
        pattern_rows(ui, state, m, "surface", m.graphics.surface, |x, f| x.graphics.surface = f)
    });
    heading(ui, t!("mat-cut-pattern"));
    grid(ui, "mat-cut", |ui| pattern_rows(ui, state, m, "cut", m.graphics.cut, |x, f| x.graphics.cut = f));
    ui.add_space(4.0);
    if ui.button(t!("fp-manage")).on_hover_text(t!("cmd-desc-fill-patterns")).clicked() {
        crate::fill_patterns_dialog::open(state);
    }
}

/// A shaded sphere in the appearance color, with a highlight by finish.
fn sphere(ui: &mut Ui, color: [u8; 3], finish: Finish) {
    let (rect, _) = ui.allocate_exact_size(vec2(120.0, 120.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_gray(70));
    let c = rect.center();
    let radius = 46.0f32;
    let light = vec2(-0.45, -0.5);
    let steps = 40;
    for i in (1..=steps).rev() {
        let t = i as f32 / steps as f32;
        let r = radius * t;
        // Larger circles are darker (rim), centers shifted towards the light.
        let shade = 0.25 + 0.75 * (1.0 - t * t).sqrt().max(0.0) * 0.9 + 0.1 * (1.0 - t);
        let col = Color32::from_rgb(
            (color[0] as f32 * shade).min(255.0) as u8,
            (color[1] as f32 * shade).min(255.0) as u8,
            (color[2] as f32 * shade).min(255.0) as u8,
        );
        painter.circle_filled(c + light * (radius - r), r, col);
    }
    let shine = finish.shininess() as f32;
    if shine > 0.0 {
        for i in 0..8 {
            let r = 4.0 + i as f32 * 2.5 * (1.2 - shine);
            let a = (shine * 180.0 * (1.0 - i as f32 / 8.0)) as u8;
            painter.circle_filled(c + light * radius * 0.55, r, Color32::from_white_alpha(a / 3));
        }
    }
}

fn appearance(ui: &mut Ui, state: &mut AppState, m: &Material) {
    ui.add_space(6.0);
    sphere(ui, m.appearance.color, m.appearance.finish);
    heading(ui, t!("mat-appearance"));
    grid(ui, "mat-appearance", |ui| {
        color_row(ui, state, m, t!("mat-color"), "app-color", m.appearance.color, |x, c| x.appearance.color = c);
        ui.label(theme::muted(t!("mat-finish")));
        let mut f = m.appearance.finish;
        egui::ComboBox::from_id_salt("mat-finish").selected_text(tr(f.key())).width(200.0).show_ui(ui, |ui| {
            for o in Finish::ALL {
                ui.selectable_value(&mut f, o, tr(o.key()));
            }
        });
        if f != m.appearance.finish {
            update(state, &m.name, "finish", |x| x.appearance.finish = f);
        }
        ui.end_row();
        text_row(
            ui,
            state,
            m,
            t!("info-description"),
            "app-desc",
            |m| &m.appearance.description,
            |m, v| m.appearance.description = v,
        );
    });
    texture(ui, state, m);
}

/// Picture files a texture is read from.
const TEXTURE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// The material's texture: a thumbnail, loading and clearing the image, and
/// the size, offset and rotation of one repeat on the faces.
fn texture(ui: &mut Ui, state: &mut AppState, m: &Material) {
    heading(ui, t!("mat-texture"));
    let tex = m.appearance.texture.as_ref();
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, Color32::from_gray(70));
        match tex.and_then(|t| crate::paint::image_texture(ui.ctx(), &t.image, false)) {
            Some(handle) => {
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(handle.id(), rect.shrink(2.0), uv, Color32::WHITE);
            }
            None => {
                let c = m.appearance.color;
                painter.rect_filled(rect.shrink(2.0), 2.0, Color32::from_rgb(c[0], c[1], c[2]));
            }
        }
        ui.vertical(|ui| {
            match tex {
                Some(t) => {
                    ui.label(theme::muted(t!("mat-texture-pixels", "w" => t.image.width, "h" => t.image.height)))
                }
                None => ui.label(theme::muted(t!("mat-texture-none"))),
            };
            ui.horizontal(|ui| {
                if ui.button(t!("mat-texture-load")).clicked() {
                    let mut dialog = rfd::FileDialog::new().add_filter(t!("file-filter-image"), &TEXTURE_EXTENSIONS);
                    if let Some(dir) = state.prefs.projects_dir() {
                        dialog = dialog.set_directory(dir);
                    }
                    if let Some(path) = dialog.pick_file() {
                        let command = materials::MaterialTextureSet {
                            material: Ref::Name(m.name.clone()),
                            path: Some(path),
                            data: None,
                            width: None,
                            height: None,
                            offset: None,
                            rotation: None,
                        };
                        crate::actions::run_command(state, command);
                    }
                }
                if tex.is_some() && ui.button(t!("mat-texture-clear")).clicked() {
                    let command = materials::MaterialTextureClear { material: Ref::Name(m.name.clone()) };
                    crate::actions::run_command(state, command);
                }
            });
        });
    });
    let Some(t) = tex else { return };
    let (lo, hi) = (cc_core::material::MIN_TEXTURE_SIZE, cc_core::material::MAX_TEXTURE_SIZE);
    grid(ui, "mat-texture", |ui| {
        type Set = fn(&mut cc_core::material::MaterialTexture, f64);
        let rows: [(String, &str, f64, Set); 4] = [
            (t!("mat-texture-width"), "tex-w", t.width, |x, v| x.width = v),
            (t!("mat-texture-height"), "tex-h", t.height, |x, v| x.height = v),
            (t!("mat-texture-offset-x"), "tex-x", t.offset[0], |x, v| x.offset[0] = v),
            (t!("mat-texture-offset-y"), "tex-y", t.offset[1], |x, v| x.offset[1] = v),
        ];
        for (i, (label, field, value, set)) in rows.into_iter().enumerate() {
            ui.label(theme::muted(label));
            let mut v = value;
            let drag = crate::properties::length_drag(&mut v);
            // Sizes stay positive; offsets may be anything.
            let r = ui.add(if i < 2 { drag.range(lo..=hi) } else { drag.range(-1e5..=1e5) });
            if r.changed() {
                update(state, &m.name, field, |x| {
                    if let Some(t) = x.appearance.texture.as_mut() {
                        set(t, v);
                    }
                });
            }
            done(state, &r);
            ui.end_row();
        }
        ui.label(theme::muted(t!("prop-rotation")));
        let mut r = t.rotation_deg;
        let resp = ui.add(DragValue::new(&mut r).speed(1.0).suffix("°"));
        if resp.changed() {
            update(state, &m.name, "tex-r", |x| {
                if let Some(t) = x.appearance.texture.as_mut() {
                    t.rotation_deg = r.rem_euclid(360.0);
                }
            });
        }
        done(state, &resp);
        ui.end_row();
    });
    ui.label(theme::muted(t!("mat-texture-hint")));
}

/// A physical property row: key, value, unit and setter.
type PhysicalRow = (&'static str, Option<f64>, &'static str, fn(&mut Material, Option<f64>));

/// Keys used at runtime (checked by the i18n tests).
#[cfg(test)]
pub const KEYS: [&str; 12] = [
    "mat-tab-identity",
    "mat-tab-graphics",
    "mat-tab-appearance",
    "mat-tab-physical",
    "phys-density",
    "phys-conductivity",
    "phys-specific-heat",
    "phys-expansion",
    "phys-youngs",
    "phys-poisson",
    "phys-compressive",
    "phys-tensile",
];

fn physical(ui: &mut Ui, state: &mut AppState, m: &Material) {
    heading(ui, t!("mat-behavior"));
    grid(ui, "mat-behavior", |ui| {
        ui.label(theme::muted(t!("mat-behavior")));
        let mut b = m.physical.behavior;
        egui::ComboBox::from_id_salt("mat-behavior-combo").selected_text(tr(b.key())).width(200.0).show_ui(ui, |ui| {
            for o in Behavior::ALL {
                ui.selectable_value(&mut b, o, tr(o.key()));
            }
        });
        if b != m.physical.behavior {
            update(state, &m.name, "behavior", |x| x.physical.behavior = b);
        }
        ui.end_row();
    });
    let p = &m.physical;
    let thermal: [PhysicalRow; 4] = [
        ("phys-density", p.density, "kg/m³", |x, v| x.physical.density = v),
        ("phys-conductivity", p.thermal_conductivity, "W/(m·K)", |x, v| x.physical.thermal_conductivity = v),
        ("phys-specific-heat", p.specific_heat, "J/(kg·K)", |x, v| x.physical.specific_heat = v),
        ("phys-expansion", p.thermal_expansion, "1/K", |x, v| x.physical.thermal_expansion = v),
    ];
    let mechanical: [PhysicalRow; 4] = [
        ("phys-youngs", p.youngs_modulus, "MPa", |x, v| x.physical.youngs_modulus = v),
        ("phys-poisson", p.poisson, "", |x, v| x.physical.poisson = v),
        ("phys-compressive", p.compressive_strength, "MPa", |x, v| x.physical.compressive_strength = v),
        ("phys-tensile", p.tensile_strength, "MPa", |x, v| x.physical.tensile_strength = v),
    ];
    for (title, rows, id) in
        [(t!("mat-thermal"), thermal, "mat-thermal"), (t!("mat-mechanical"), mechanical, "mat-mech")]
    {
        heading(ui, title);
        grid(ui, id, |ui| {
            for (k, v, unit, set) in rows {
                ui.label(theme::muted(tr(k)));
                optional_value(ui, state, m, k, v, unit, set);
                ui.end_row();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texts visible (inside the screen and their clip rect) in a headless
    /// frame of the browser.
    fn frame_texts(state: &mut AppState) -> Vec<String> {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
        let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
        let mut texts = Vec::new();
        // Many frames: a layout feedback loop once grew the dialog a little each frame.
        for _ in 0..30 {
            let mut out = ctx.run_ui(input.clone(), |ui| show(&ui.ctx().clone(), state));
            out.textures_delta.clear();
            texts = out
                .shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    egui::Shape::Text(t) => {
                        let visible = c.shape.visual_bounding_rect().intersect(c.clip_rect).intersect(screen);
                        (visible.is_positive() && visible.area() > 0.5 * c.shape.visual_bounding_rect().area())
                            .then(|| t.galley.text().to_string())
                    }
                    _ => None,
                })
                .collect();
        }
        texts
    }

    #[test]
    fn browser_shows_the_list_and_the_selected_material() {
        let mut state = AppState::new(cc_core::Document::new(crate::actions::new_project()));
        state.dialogs.materials = true;
        let texts = frame_texts(&mut state);
        let first = state.project().materials[0].name.clone();
        assert!(texts.iter().any(|t| t == &first), "list missing: {texts:?}");
        // Regression: the dialog once grew past the screen (first sideways,
        // then downwards) and showed only blank space.
        for key in ["action-ok", "action-cancel", "mat-name"] {
            assert!(texts.iter().any(|t| *t == tr(key)), "{key} not visible: {texts:?}");
        }
        assert!(texts.iter().any(|t| t.starts_with(&t!("dlg-materials"))), "title not visible: {texts:?}");
        for tab in ["mat-tab-identity", "mat-tab-graphics", "mat-tab-appearance", "mat-tab-physical"] {
            assert!(texts.iter().any(|t| *t == tr(tab)), "tab {tab} missing: {texts:?}");
        }
        for tab in [Tab::Graphics, Tab::Appearance, Tab::Physical] {
            state.dialogs.material_browser.tab = tab;
            assert!(frame_texts(&mut state).len() > 10, "{tab:?} empty");
        }
    }
}
