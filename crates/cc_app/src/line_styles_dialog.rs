//! The Line Styles dialog (*Annotate → Line Styles*): the built-in line
//! styles, read only, and the project's own, whose name, weight, color and
//! dash pattern are edited in place; *New* adds one and *Delete* removes
//! the selected one (what uses it moves to the thin style). Every change
//! runs a `line_style_*` command; OK keeps them, Cancel undoes them all
//! (`dialog::session`, through [`crate::graphics_dialog::Edits`]).

use crate::graphics_dialog::{Edits, key, run};
use crate::i18n::tr;
use crate::icons::Icon;
use crate::settings;
use crate::state::AppState;
use crate::t;
use crate::theme::PALETTE;
use cc_core::detail::{DetailStyle, MAX_WEIGHT_MM, MIN_WEIGHT_MM};
use cc_core::{LinePattern, LineStyleId};
use cc_engine::Ref;
use cc_engine::commands::line_style as cmd;
use egui::{Context, RichText, Ui};

/// The open Line Styles dialog.
pub struct LineStylesDialog {
    /// A name being typed: the style and its text.
    draft: Option<(LineStyleId, String)>,
    edits: Edits,
}

impl LineStylesDialog {
    /// Close the dialog's edits: keep them (OK) or undo them (Cancel).
    pub fn edits_finish(self, state: &mut AppState, keep: bool) {
        self.edits.finish(state, keep);
    }
}

pub fn open(state: &mut AppState) {
    let edits = Edits::begin(state);
    state.dialogs.line_styles = Some(LineStylesDialog { draft: None, edits });
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(d) = state.dialogs.line_styles.as_mut() else { return };
    let mut draft = d.draft.take();
    let mut ok = 0;
    let response = crate::dialog::Dialog::new("line-styles", t!("dlg-line-styles")).width(WIDTH).show(
        ctx,
        |ui| {
            ui.add(egui::Label::new(RichText::new(t!("ls-hint")).size(12.0).color(PALETTE.text_muted)).wrap());
            ui.add_space(14.0);
            egui::ScrollArea::vertical().max_height(520.0).auto_shrink([false, true]).show(ui, |ui| {
                project_styles(ui, state, &mut draft);
                ui.add_space(20.0);
                built_in(ui);
            });
        },
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    if let Some(d) = state.dialogs.line_styles.as_mut() {
        d.draft = draft;
    }
    let close = match response.clicked {
        Some(i) => Some(i == ok),
        None if response.dismissed => Some(false),
        None => None,
    };
    if let Some(keep) = close
        && let Some(d) = state.dialogs.line_styles.take()
    {
        d.edits_finish(state, keep);
    }
}

/// Width of the dialog body.
const WIDTH: f32 = 820.0;
/// Size of a line preview in a row.
const SAMPLE: egui::Vec2 = egui::vec2(170.0, 32.0);

fn pattern_label(p: LinePattern) -> String {
    tr(p.key())
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// A paper weight as the project reads it: the number alone in metric
/// projects (with "mm"), with the inch mark in imperial ones.
fn weight_text(mm: f64) -> String {
    if cc_views::format::imperial() {
        cc_views::format::paper_weight(mm)
    } else {
        format!("{mm:.2} {}", cc_views::format::unit_symbol("mm"))
    }
}

/// The project's own styles, each a row: its line as it prints, its name,
/// weight, color and pattern edited in place, how many things use it, and
/// a button that deletes it.
fn project_styles(ui: &mut Ui, state: &mut AppState, draft: &mut Option<(LineStyleId, String)>) {
    let styles: Vec<_> = state.project().line_styles.values().cloned().collect();
    let used: Vec<usize> = styles.iter().map(|s| state.project().line_style_users(s.id).len()).collect();
    settings::list_title(ui, &t!("ls-project"), Some(styles.len()), |ui| {
        let r = ui.scope(|ui| {
            settings::framed(ui);
            ui.add(egui::Button::new(format!("+ {}", t!("ls-new"))).min_size(egui::vec2(0.0, 28.0)))
        });
        let r = r.inner.on_hover_text(t!("ls-new-tip"));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, t!("ls-new")));
        if r.clicked() {
            let names: Vec<String> = state.project().line_styles.values().map(|s| s.name.to_lowercase()).collect();
            let name = (1..1000)
                .map(|n| t!("ls-default-name", "n" => n))
                .find(|n| !names.contains(&n.to_lowercase()))
                .unwrap_or_default();
            crate::actions::run_command(state, cmd::LineStyleCreate { name, ..Default::default() });
        }
    });
    settings::card(ui, |ui| {
        if styles.is_empty() {
            settings::empty_state(ui, Icon::LineStyles, &t!("ls-none"));
            return;
        }
        let w = [SAMPLE.x, 180.0, 96.0, 34.0, 140.0, 60.0];
        let mut x = 0.0;
        let columns: Vec<(f32, String)> =
            [String::new(), t!("prop-name"), t!("ls-weight"), t!("mat-color"), t!("mat-pattern"), t!("ls-used-by")]
                .into_iter()
                .zip(w)
                .map(|(name, width)| {
                    let at = x;
                    x += width + settings::CELL_GAP;
                    (at, name)
                })
                .collect();
        settings::card_header(ui, &columns);
        let count = styles.len();
        for (i, (s, n)) in styles.iter().zip(used).enumerate() {
            let id = s.id;
            let style = Ref::Id(id.0);
            settings::card_row(ui, id.0, 52.0, false, i + 1 == count, |ui| {
                settings::line_sample(ui, SAMPLE, s.weight_mm, color32(s.color), s.pattern.dashes_mm());
                settings::framed(ui);
                settings::cell(ui, w[1], |ui| {
                    let mut text = match draft {
                        Some((d, t)) if *d == id => t.clone(),
                        _ => s.name.clone(),
                    };
                    let r = ui.add(settings::text_edit(&mut text, w[1]));
                    if r.changed() {
                        *draft = Some((id, text.clone()));
                    }
                    if r.lost_focus() {
                        if text.trim() != s.name.trim() {
                            let command =
                                cmd::LineStyleSet { style: style.clone(), name: Some(text), ..Default::default() };
                            run(state, None, command);
                        }
                        *draft = None;
                    }
                });
                settings::cell(ui, w[2], |ui| {
                    let mut weight = s.weight_mm;
                    let drag =
                        crate::properties::weight_drag(&mut weight).speed(0.01).range(MIN_WEIGHT_MM..=MAX_WEIGHT_MM);
                    let r = settings::drag_wide(ui, drag);
                    if r.changed() {
                        let command =
                            cmd::LineStyleSet { style: style.clone(), weight: Some(weight), ..Default::default() };
                        run(state, Some(key((id, "weight"))), command);
                    }
                    if r.drag_stopped() || r.lost_focus() {
                        state.session.doc.break_merge();
                    }
                });
                settings::cell(ui, w[3], |ui| {
                    let mut c = s.color;
                    ui.spacing_mut().interact_size = egui::vec2(28.0, 28.0);
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        let command = cmd::LineStyleSet { style: style.clone(), color: Some(c), ..Default::default() };
                        run(state, Some(key((id, "color"))), command);
                    }
                });
                settings::cell(ui, w[4], |ui| {
                    let mut p = s.pattern;
                    settings::combo(
                        ui,
                        ("line-style-pattern", id.0),
                        w[4] - 8.0,
                        &mut p,
                        LinePattern::ALL,
                        pattern_label,
                    );
                    if p != s.pattern {
                        let pattern = Some(cmd::pattern_code(p).to_string());
                        run(state, None, cmd::LineStyleSet { style: style.clone(), pattern, ..Default::default() });
                    }
                });
                settings::cell(ui, w[5], |ui| {
                    let color = if n == 0 { PALETTE.text_faint } else { PALETTE.text_muted };
                    settings::badge(ui, &n.to_string(), color);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = format!("{} {}", t!("ls-delete"), s.name);
                    if settings::icon_button(ui, Icon::Delete, &label, true, true)
                        .on_hover_text(t!("ls-delete-tip"))
                        .clicked()
                    {
                        run(state, None, cmd::LineStyleDelete { style: style.clone(), replace_with: None });
                    }
                });
            });
        }
    });
}

/// The built-in styles, read only: a tile each with its line, name, weight and pattern.
fn built_in(ui: &mut Ui) {
    settings::list_title(ui, &t!("ls-built-ins"), Some(DetailStyle::ALL.len()), |_| {});
    let columns = 4;
    let gap = 12.0;
    let full = ui.available_width();
    let width = ((full - gap * (columns - 1) as f32) / columns as f32).floor();
    for chunk in DetailStyle::ALL.chunks(columns) {
        let (row, _) = ui.allocate_exact_size(egui::vec2(full, 92.0), egui::Sense::hover());
        for (i, s) in chunk.iter().enumerate() {
            let tile = egui::Rect::from_min_size(
                row.min + egui::vec2(i as f32 * (width + gap), 0.0),
                egui::vec2(width, row.height()),
            );
            let painter = ui.painter();
            painter.rect_stroke(tile, 8.0, egui::Stroke::new(1.0, PALETTE.border), egui::StrokeKind::Inside);
            let sample = egui::Rect::from_min_size(tile.min + egui::vec2(8.0, 8.0), egui::vec2(width - 16.0, 30.0));
            let black = egui::Color32::from_gray(20);
            settings::paint_line_sample(painter, sample, s.weight_mm(), black, s.pattern().dashes_mm());
            let name_font = egui::FontId::new(13.0, crate::theme::semibold());
            painter.text(
                sample.left_bottom() + egui::vec2(2.0, 8.0),
                egui::Align2::LEFT_TOP,
                tr(s.key()),
                name_font,
                PALETTE.text,
            );
            let meta = format!("{} · {}", weight_text(s.weight_mm()), pattern_label(s.pattern()));
            let meta_font = egui::FontId::proportional(11.5);
            painter.text(
                sample.left_bottom() + egui::vec2(2.0, 28.0),
                egui::Align2::LEFT_TOP,
                meta,
                meta_font,
                PALETTE.text_muted,
            );
        }
        ui.add_space(12.0);
    }
}
