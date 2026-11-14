//! Schedule (table) view, its definition dialog (fields, filter, sorting and
//! grouping, formatting, appearance) and CSV export.

use crate::dialog::{Dialog, session};
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::theme::{self, PALETTE};
use cc_core::graphics::FilterOp;
use cc_core::{
    Category, ColumnAlign, ColumnFormat, ScheduleDef, ScheduleField, ScheduleFilter, SortLevel, ViewId, ViewKind,
};
use cc_views::Cell;
use egui::{Context, RichText, Ui};
use std::collections::BTreeSet;

/// The pages of the schedule definition dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Fields,
    Filter,
    Sorting,
    Formatting,
    Appearance,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Fields, Tab::Filter, Tab::Sorting, Tab::Formatting, Tab::Appearance];

    pub fn label(self) -> String {
        match self {
            Tab::Fields => t!("schedule-tab-fields"),
            Tab::Filter => t!("schedule-tab-filter"),
            Tab::Sorting => t!("schedule-tab-sorting"),
            Tab::Formatting => t!("schedule-tab-formatting"),
            Tab::Appearance => t!("schedule-tab-appearance"),
        }
    }
}

/// Open the definition dialog of schedule `view` on `tab`.
pub fn open(state: &mut AppState, view: ViewId, tab: Tab) {
    state.dialogs.schedule_def = Some(view);
    state.dialogs.schedule_tab = tab;
}

/// The rows of a schedule's properties: one per page of its definition, each with its *Edit* button.
pub fn props(ui: &mut Ui, state: &mut AppState, view: ViewId) {
    for tab in Tab::ALL {
        crate::table::row(ui, &tab.label(), |ui| {
            let button = egui::Button::new(t!("schedule-edit")).min_size(egui::vec2(ui.available_width(), 0.0));
            let r = ui.add(button);
            r.widget_info(|| {
                let label = format!("{}: {}", tab.label(), t!("schedule-edit"));
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
            });
            if r.clicked() {
                open(state, view, tab);
            }
        });
    }
}

fn cell_text(cell: &Cell) -> String {
    match cell {
        Cell::Text(s) => s.clone(),
        Cell::Key(k) => tr(k),
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState, view_id: ViewId) {
    let Some(view) = state.project().view(view_id).cloned() else { return };
    if !matches!(view.kind, ViewKind::Schedule { .. }) {
        return;
    }
    // The elements the schedule's phase and phase filter show.
    let table = state.session.schedule(view_id).unwrap_or_default();

    egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&view.name).family(theme::semibold()).size(17.0).color(PALETTE.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t!("schedule-export-csv")).clicked() {
                    export_csv(state, view_id, &view.name);
                }
                if ui.button(t!("schedule-fields")).clicked() {
                    open(state, view_id, Tab::Fields);
                }
            });
        });
        ui.label(RichText::new(t!("schedule-hint")).color(PALETTE.text_faint));
        ui.add_space(6.0);
        // A table that goes under it on paper (a bending schedule's summary per diameter): shown first here.
        if let Some(summary) = &table.summary {
            if let Some(caption) = summary.caption {
                ui.label(RichText::new(tr(caption)).family(theme::semibold()));
            }
            let id = ("schedule-summary", view_id.0);
            egui::Grid::new(id).striped(true).min_col_width(MIN_COLUMN).spacing([COLUMN_GAP, ROW_GAP]).show(ui, |ui| {
                for h in &summary.headers {
                    ui.label(RichText::new(tr(h)).family(theme::semibold()));
                }
                ui.end_row();
                for row in &summary.rows {
                    for cell in &row.cells {
                        ui.label(cell_text(cell));
                    }
                    ui.end_row();
                }
                for (i, cell) in summary.totals.iter().enumerate() {
                    let text = if i == 0 { t!("schedule-total", "count" => cell) } else { cell.clone() };
                    ui.label(RichText::new(text).family(theme::semibold()));
                }
                ui.end_row();
            });
            ui.add_space(12.0);
        }
        let widths = column_widths(ui, state.session.doc.revision(), view_id, &table);
        let headers = table.header_texts(&|key| tr(key));
        let row_height = ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Body));
        // The header stays on top; only the rows in sight are laid out, so long schedules stay cheap.
        egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(COLUMN_GAP, ROW_GAP);
            row_cells(ui, &widths, row_height, None, |ui, i| {
                let h = headers.get(i).cloned().unwrap_or_default();
                ui.label(RichText::new(h).family(theme::semibold()));
            });
            // The rows, then the totals as one more row.
            let total = table.rows.len() + 1;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_height, total, |ui, range| {
                for r in range {
                    let stripe = (r % 2 == 1).then(|| ui.visuals().faint_bg_color);
                    let Some(row) = table.rows.get(r) else {
                        row_cells(ui, &widths, row_height, stripe, |ui, i| {
                            let Some(cell) = table.totals.get(i) else { return };
                            let text = if i == 0 { t!("schedule-total", "count" => cell) } else { cell.clone() };
                            ui.label(RichText::new(text).family(theme::semibold()));
                        });
                        continue;
                    };
                    // A group's heading names the group in its first cell.
                    if row.heading {
                        row_cells(ui, &widths, row_height, stripe, |ui, i| {
                            if let (0, Some(cell)) = (i, row.cells.first()) {
                                ui.label(RichText::new(cell_text(cell)).family(theme::semibold()));
                            }
                        });
                        continue;
                    }
                    if row.footer {
                        row_cells(ui, &widths, row_height, stripe, |ui, i| {
                            let Some(cell) = row.cells.get(i) else { return };
                            let text = cell_text(cell);
                            let text = if i == 0 { t!("schedule-total", "count" => text) } else { text };
                            ui.label(RichText::new(text).family(theme::semibold()));
                        });
                        continue;
                    }
                    let selected = state.selection.contains(&row.element);
                    let mut clicked = false;
                    row_cells(ui, &widths, row_height, stripe, |ui, i| {
                        let Some(cell) = row.cells.get(i) else { return };
                        let text = cell_text(cell);
                        let text = if text.is_empty() && i == 0 { "—".to_string() } else { text };
                        clicked |= ui.selectable_label(selected, text).clicked();
                    });
                    if clicked {
                        state.selection = BTreeSet::from([row.element]);
                    }
                }
            });
        });
    });
}

/// Space between columns and between rows of the table.
const COLUMN_GAP: f32 = 16.0;
const ROW_GAP: f32 = 6.0;
const MIN_COLUMN: f32 = 70.0;

/// The column widths of the schedule shown: the widest of each column's header, cells and total.
/// Measuring every cell is the costly part, so the widths are kept until the document, the language
/// or the text size changes.
fn column_widths(ui: &Ui, revision: u64, view: ViewId, table: &cc_views::Table) -> Vec<f32> {
    #[derive(Clone, PartialEq)]
    struct Key(u64, crate::i18n::Language, u32, usize);
    let body = ui.text_style_height(&egui::TextStyle::Body);
    let key = Key(revision, crate::i18n::language(), body.to_bits(), table.headers.len());
    let id = egui::Id::new(("schedule-widths", view.0));
    if let Some((k, widths)) = ui.ctx().data(|d| d.get_temp::<(Key, Vec<f32>)>(id))
        && k == key
    {
        return widths;
    }
    let painter = ui.painter();
    let body_font = egui::TextStyle::Body.resolve(ui.style());
    let bold_font = egui::FontId::new(body_font.size, theme::semibold());
    let measure = |text: String, font: &egui::FontId| painter.layout_no_wrap(text, font.clone(), PALETTE.text).size().x;
    // A selectable label pads its text on both sides.
    let pad = 2.0 * ui.spacing().button_padding.x;
    let mut widths: Vec<f32> =
        table.header_texts(&|key| tr(key)).into_iter().map(|h| measure(h, &bold_font).max(MIN_COLUMN)).collect();
    for row in &table.rows {
        for (i, cell) in row.cells.iter().enumerate() {
            let w = measure(cell_text(cell), &body_font) + pad;
            if let Some(slot) = widths.get_mut(i) {
                *slot = slot.max(w);
            } else {
                widths.push(w.max(MIN_COLUMN));
            }
        }
    }
    for (i, cell) in table.totals.iter().enumerate() {
        let text = if i == 0 { t!("schedule-total", "count" => cell) } else { cell.clone() };
        let w = measure(text, &bold_font);
        if let Some(slot) = widths.get_mut(i) {
            *slot = slot.max(w);
        } else {
            widths.push(w.max(MIN_COLUMN));
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, (key, widths.clone())));
    widths
}

/// One table row: each column at its width, left aligned, over an optional stripe.
fn row_cells(
    ui: &mut Ui,
    widths: &[f32],
    height: f32,
    stripe: Option<egui::Color32>,
    mut cell: impl FnMut(&mut Ui, usize),
) {
    let gap = ui.spacing().item_spacing.x;
    let width = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if let Some(color) = stripe {
        ui.painter().rect_filled(rect.expand2(egui::vec2(gap / 2.0, ROW_GAP / 2.0)), 0.0, color);
    }
    let mut x = rect.left();
    for (i, w) in widths.iter().enumerate() {
        let cell_rect = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, height));
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(cell_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| cell(ui, i),
        );
        x += w + gap;
    }
}

fn export_csv(state: &mut AppState, view: ViewId, name: &str) {
    let dialog =
        rfd::FileDialog::new().add_filter(t!("file-filter-csv"), &["csv"]).set_file_name(format!("{name}.csv"));
    let Some(mut path) = dialog.save_file() else { return };
    if path.extension().is_none() {
        path.set_extension("csv");
    }
    let command = cc_engine::commands::io::ScheduleExportCsv { view: cc_engine::Ref::Id(view.0), path: path.clone() };
    match state.session.run(command) {
        Ok(_) => state.message = t!("msg-exported", "path" => path.display()),
        Err(e) => state.dialogs.error = Some(t!("msg-export-failed", "error" => e.message)),
    }
}

/// Edit the schedule definition of `view` through `f` (one undo step per change).
fn update(state: &mut AppState, view: ViewId, f: impl FnOnce(&mut ScheduleDef)) {
    crate::engine_edit::view(state, view, None, |x| {
        if let ViewKind::Schedule { def, .. } = &mut x.kind {
            f(def);
        }
    });
}

/// The definition of the schedule being edited: fields, filter, sorting and
/// grouping, formatting and appearance, a page each.
pub fn dialog(ctx: &Context, state: &mut AppState) {
    let Some(view_id) = state.dialogs.schedule_def else { return };
    let read = |state: &AppState| match state.project().view(view_id).map(|v| v.kind.clone()) {
        Some(ViewKind::Schedule { category, def }) => Some((category, def)),
        _ => None,
    };
    let Some((category, mut def)) = read(state) else {
        state.dialogs.schedule_def = None;
        return;
    };
    session::begin(state);
    // The pages edit the lists: an older single filter or sort field moves into them.
    if def.filter.is_some() || def.sort_by.is_some() {
        update(state, view_id, ScheduleDef::normalize);
        if let Some((_, d)) = read(state) {
            def = d;
        }
    }
    let mut tab = state.dialogs.schedule_tab;
    let mut ok = 0;
    let response = Dialog::new("schedule-def", t!("dlg-schedule")).width(720.0).show(
        ctx,
        |ui| {
            ui.horizontal(|ui| {
                for t in Tab::ALL {
                    ui.selectable_value(&mut tab, t, t.label());
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(400.0).auto_shrink([false, true]).show(ui, |ui| match tab {
                Tab::Fields => body(ui, state, view_id, category, &def),
                Tab::Filter => filter_page(ui, state, view_id, category, &def),
                Tab::Sorting => sorting_page(ui, state, view_id, category, &def),
                Tab::Formatting => formatting_page(ui, state, view_id, category, &def),
                Tab::Appearance => appearance_page(ui, state, view_id, &def),
            });
        },
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    state.dialogs.schedule_tab = tab;
    if crate::dialogs::ok_cancel(state, &response, ok) {
        state.dialogs.schedule_def = None;
    }
}

/// [`update`] for a value typed or dragged: the changes of one field are one undo step.
fn update_merged(state: &mut AppState, view: ViewId, key: u64, f: impl FnOnce(&mut ScheduleDef)) {
    crate::engine_edit::view(state, view, Some(view.0 ^ key), |x| {
        if let ViewKind::Schedule { def, .. } = &mut x.kind {
            f(def);
        }
    });
}

/// A combo of the fields of `category`; returns the field picked, when it changed.
fn field_combo(ui: &mut Ui, salt: (&str, usize), category: Category, current: ScheduleField) -> Option<ScheduleField> {
    let mut field = current;
    egui::ComboBox::from_id_salt(salt).width(170.0).selected_text(crate::properties::field_label(field)).show_ui(
        ui,
        |ui| {
            for f in ScheduleField::available(category) {
                ui.selectable_value(&mut field, f, crate::properties::field_label(f));
            }
        },
    );
    (field != current).then_some(field)
}

/// A small button that removes a row of a list.
fn remove_button(ui: &mut Ui) -> bool {
    ui.add(egui::Button::new("×").small()).on_hover_text(t!("schedule-remove")).clicked()
}

/// The filters: rows whose field compares with a value; a row of the table stays when it passes all.
fn filter_page(ui: &mut Ui, state: &mut AppState, view: ViewId, category: Category, def: &ScheduleDef) {
    if def.filters.is_empty() {
        ui.label(theme::muted(t!("schedule-filter-empty")));
    }
    for (i, filter) in def.filters.iter().enumerate() {
        ui.horizontal(|ui| {
            if let Some(field) = field_combo(ui, ("schedule-filter-field", i), category, filter.field) {
                update(state, view, |d| {
                    if let Some(f) = d.filters.get_mut(i) {
                        f.field = field;
                    }
                });
            }
            let mut op = filter.op;
            egui::ComboBox::from_id_salt(("schedule-filter-op", i)).width(170.0).selected_text(tr(op.key())).show_ui(
                ui,
                |ui| {
                    for o in FilterOp::ALL {
                        ui.selectable_value(&mut op, o, tr(o.key()));
                    }
                },
            );
            if op != filter.op {
                update(state, view, |d| {
                    if let Some(f) = d.filters.get_mut(i) {
                        f.op = op;
                    }
                });
            }
            let mut text = filter.value.clone();
            let enabled = filter.op.needs_value();
            let r = crate::table::live_text(ui, ("schedule-filter-value", view, i), &mut text, |ui, t| {
                let edit = egui::TextEdit::singleline(t).desired_width(170.0).hint_text(t!("schedule-filter-value"));
                ui.add_enabled(enabled, edit)
            });
            if text != filter.value {
                update_merged(state, view, 0x5c4e + i as u64, |d| {
                    if let Some(f) = d.filters.get_mut(i) {
                        f.value = text.chars().take(cc_core::schedule::MAX_SCHEDULE_TEXT).collect();
                    }
                });
            }
            if r.lost_focus() {
                state.session.doc.break_merge();
            }
            if remove_button(ui) {
                update(state, view, |d| {
                    if i < d.filters.len() {
                        d.filters.remove(i);
                    }
                });
            }
        });
    }
    ui.add_space(6.0);
    let first = ScheduleField::available(category).into_iter().next();
    let room = def.filters.len() < cc_core::schedule::MAX_FILTERS;
    if ui.add_enabled(room && first.is_some(), egui::Button::new(t!("schedule-filter-add"))).clicked()
        && let Some(field) = def.fields_for(category).first().copied().or(first)
    {
        update(state, view, |d| d.filters.push(ScheduleFilter { field, op: FilterOp::Equals, value: String::new() }));
    }
}

/// The sorting levels, each with its heading, footer and blank line, and whether every element has its row.
fn sorting_page(ui: &mut Ui, state: &mut AppState, view: ViewId, category: Category, def: &ScheduleDef) {
    if def.sorting.is_empty() {
        ui.label(theme::muted(t!("schedule-sort-empty")));
    }
    for (i, level) in def.sorting.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            if let Some(field) = field_combo(ui, ("schedule-sort-field", i), category, level.field) {
                update(state, view, |d| {
                    if let Some(l) = d.sorting.get_mut(i) {
                        l.field = field;
                    }
                });
            }
            let mut new = *level;
            ui.checkbox(&mut new.descending, t!("schedule-descending"));
            ui.checkbox(&mut new.header, t!("schedule-sort-header"));
            ui.checkbox(&mut new.footer, t!("schedule-sort-footer"));
            ui.checkbox(&mut new.blank_line, t!("schedule-sort-blank"));
            if new != *level {
                update(state, view, |d| {
                    if let Some(l) = d.sorting.get_mut(i) {
                        *l = new;
                    }
                });
            }
            if remove_button(ui) {
                update(state, view, |d| {
                    if i < d.sorting.len() {
                        d.sorting.remove(i);
                    }
                });
            }
        });
    }
    ui.add_space(6.0);
    let room = def.sorting.len() < cc_core::schedule::MAX_SORT_LEVELS;
    // A new level sorts by the first shown field no level sorts by yet.
    let next = def
        .fields_for(category)
        .into_iter()
        .chain(ScheduleField::available(category))
        .find(|f| !def.sorting.iter().any(|l| l.field == *f));
    if ui.add_enabled(room && next.is_some(), egui::Button::new(t!("schedule-sort-add"))).clicked()
        && let Some(field) = next
    {
        update(state, view, |d| d.sorting.push(SortLevel::new(field)));
    }
    ui.add_space(10.0);
    let mut itemize = !def.collapse;
    if ui.checkbox(&mut itemize, t!("schedule-itemize")).changed() {
        update(state, view, |d| d.collapse = !itemize);
    }
    ui.label(theme::muted(t!("schedule-itemize-hint")));
    ui.add_space(4.0);
    let mut totals = !def.hide_totals;
    if ui.checkbox(&mut totals, t!("schedule-show-totals")).changed() {
        update(state, view, |d| d.hide_totals = !totals);
    }
}

/// Change the format of `field`'s column through `f`, keeping only formats that say something.
fn set_format(d: &mut ScheduleDef, field: ScheduleField, f: impl FnOnce(&mut ColumnFormat)) {
    let mut format = d.format_of(field);
    f(&mut format);
    d.columns.retain(|c| c.field != field);
    if !format.is_plain() {
        d.columns.push(format);
    }
}

/// The width of a column's heading field in the formatting page.
const HEADING_WIDTH: f32 = 190.0;

/// Each column's heading, alignment, and whether it is hidden or totaled.
fn formatting_page(ui: &mut Ui, state: &mut AppState, view: ViewId, category: Category, def: &ScheduleDef) {
    let fields = def.fields_for(category);
    egui::Grid::new("schedule-formatting").num_columns(5).spacing([14.0, 8.0]).show(ui, |ui| {
        for key in ["schedule-column", "schedule-heading", "schedule-align", "schedule-hidden", "schedule-total-column"]
        {
            ui.label(theme::muted(tr(key)));
        }
        ui.end_row();
        for (i, field) in fields.iter().copied().enumerate() {
            let format = def.format_of(field);
            let name = crate::properties::field_label(field);
            ui.label(&name);
            let mut heading = format.heading.clone().unwrap_or_default();
            let before = heading.clone();
            let r = crate::table::live_text(ui, ("schedule-heading", view, i), &mut heading, |ui, t| {
                // A grid cell that is not the last offers only the width the column already has, so a
                // field that asks for a desired width stays as narrow as an empty cell: give it its size.
                let size = [HEADING_WIDTH, ui.spacing().interact_size.y];
                ui.add_sized(size, egui::TextEdit::singleline(t).hint_text(name.clone()))
            });
            if heading != before {
                let text: String = heading.chars().take(cc_core::schedule::MAX_SCHEDULE_TEXT).collect();
                update_merged(state, view, 0x7ead + i as u64, |d| {
                    set_format(d, field, |c| c.heading = (!text.trim().is_empty()).then_some(text));
                });
            }
            if r.lost_focus() {
                state.session.doc.break_merge();
            }
            let mut align = format.align;
            egui::ComboBox::from_id_salt(("schedule-align", i)).width(120.0).selected_text(tr(align.key())).show_ui(
                ui,
                |ui| {
                    for a in ColumnAlign::ALL {
                        ui.selectable_value(&mut align, a, tr(a.key()));
                    }
                },
            );
            if align != format.align {
                update(state, view, |d| set_format(d, field, |c| c.align = align));
            }
            let mut hidden = format.hidden;
            if crate::table::checkbox(ui, &mut hidden).changed() {
                update(state, view, |d| set_format(d, field, |c| c.hidden = hidden));
            }
            // Totaled as the field usually is, until the box says otherwise.
            let usual = cc_views::totaled_by_default(field);
            let mut total = format.total.unwrap_or(usual);
            if crate::table::checkbox(ui, &mut total).changed() {
                update(state, view, |d| set_format(d, field, |c| c.total = (total != usual).then_some(total)));
            }
            ui.end_row();
        }
    });
}

/// How the table looks on sheets and exports: its rules, title and header rows and text sizes.
fn appearance_page(ui: &mut Ui, state: &mut AppState, view: ViewId, def: &ScheduleDef) {
    use cc_core::schedule::{MAX_SCHEDULE_TEXT_SIZE, MIN_SCHEDULE_TEXT_SIZE};
    let look = def.appearance;
    let mut new = look;
    ui.checkbox(&mut new.grid_lines, t!("schedule-grid-lines"));
    ui.checkbox(&mut new.outline, t!("schedule-outline"));
    ui.checkbox(&mut new.show_title, t!("schedule-show-title"));
    ui.checkbox(&mut new.show_headers, t!("schedule-show-headers"));
    if new != look {
        update(state, view, |d| d.appearance = new);
    }
    ui.add_space(8.0);
    let mut sizes = look;
    let mut done = false;
    egui::Grid::new("schedule-appearance").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
        let range = MIN_SCHEDULE_TEXT_SIZE..=MAX_SCHEDULE_TEXT_SIZE;
        for (key, value) in [
            ("schedule-title-size", &mut sizes.title_size),
            ("schedule-header-size", &mut sizes.header_size),
            ("schedule-text-size", &mut sizes.text_size),
        ] {
            ui.label(theme::muted(tr(key)));
            let r = ui.add(crate::properties::paper_drag(value).speed(0.1).range(range.clone()));
            done |= r.drag_stopped() || r.lost_focus();
            ui.end_row();
        }
    });
    if sizes != look {
        update_merged(state, view, 0xa99e, |d| d.appearance = sizes);
    }
    if done {
        state.session.doc.break_merge();
    }
    ui.add_space(8.0);
    ui.label(theme::muted(t!("schedule-appearance-hint")));
}

fn body(ui: &mut Ui, state: &mut AppState, view: ViewId, category: Category, def: &ScheduleDef) {
    let chosen = def.fields_for(category);
    let available: Vec<ScheduleField> =
        ScheduleField::available(category).into_iter().filter(|f| !chosen.contains(f)).collect();
    let set_fields = |state: &mut AppState, fields: Vec<ScheduleField>| update(state, view, |d| d.fields = fields);

    ui.columns(2, |cols| {
        cols[0].label(theme::muted(t!("schedule-available")));
        egui::Frame::new().stroke(egui::Stroke::new(1.0, PALETTE.border)).inner_margin(6).show(&mut cols[0], |ui| {
            ui.set_min_height(220.0);
            ui.set_width(ui.available_width());
            for f in &available {
                if ui
                    .selectable_label(false, format!("+  {}", crate::properties::field_label(*f)))
                    .on_hover_text(t!("schedule-add"))
                    .clicked()
                {
                    let mut fields = chosen.clone();
                    fields.push(*f);
                    set_fields(state, fields);
                }
            }
        });
        cols[1].label(theme::muted(t!("schedule-chosen")));
        egui::Frame::new().stroke(egui::Stroke::new(1.0, PALETTE.border)).inner_margin(6).show(&mut cols[1], |ui| {
            ui.set_min_height(220.0);
            ui.set_width(ui.available_width());
            let n = chosen.len();
            for (i, f) in chosen.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(crate::properties::field_label(*f));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if ui
                            .add_enabled(n > 1, egui::Button::new("×").small())
                            .on_hover_text(t!("schedule-remove"))
                            .clicked()
                        {
                            let mut fields = chosen.clone();
                            fields.remove(i);
                            set_fields(state, fields);
                        }
                        if ui.add_enabled(i + 1 < n, egui::Button::new("↓").small()).clicked() {
                            let mut fields = chosen.clone();
                            fields.swap(i, i + 1);
                            set_fields(state, fields);
                        }
                        if ui.add_enabled(i > 0, egui::Button::new("↑").small()).clicked() {
                            let mut fields = chosen.clone();
                            fields.swap(i, i - 1);
                            set_fields(state, fields);
                        }
                    });
                });
            }
        });
    });
}
