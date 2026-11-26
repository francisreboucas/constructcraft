//! The New Project dialog: the template a new project starts from, metric
//! or imperial, picked on one of two cards. Create goes on to
//! [`actions::guarded`], so the question about unsaved changes comes after
//! the choice, and cancelling either leaves the open project as it is.

use crate::actions;
use crate::dialog::Dialog;
use crate::state::{AppState, PendingAction};
use crate::t;
use crate::theme::{self, PALETTE};
use cc_core::units::{FOOT, INCH};
use cc_core::{AreaUnit, MeasurementSystem, SheetSize, Units, VolumeUnit};
use cc_views::format;
use egui::{Context, Sense, Stroke, StrokeKind, Ui, vec2};
use std::sync::Arc;

/// Width of each card.
const CARD_WIDTH: f32 = 220.0;
/// Gap between the cards.
const CARD_GAP: f32 = 8.0;
/// Padding inside a card.
const CARD_PAD: f32 = 12.0;

/// Open the dialog with the template picked last (kept when it is already open).
pub fn open(state: &mut AppState) {
    if state.dialogs.new_project.is_none() {
        state.dialogs.new_project = Some(state.prefs.new_project_system);
    }
}

/// A template's name.
fn title(system: MeasurementSystem) -> String {
    match system {
        MeasurementSystem::Metric => t!("new-project-metric"),
        MeasurementSystem::Imperial => t!("new-project-imperial"),
    }
}

/// What a template uses: units, drawing scales and sheets, with samples
/// formatted the way the template shows them.
fn details(system: MeasurementSystem) -> [String; 3] {
    match system {
        MeasurementSystem::Metric => [
            t!(
                "new-project-metric-units",
                "area" => format::unit_symbol(AreaUnit::SquareMeter.symbol()),
                "volume" => format::unit_symbol(VolumeUnit::CubicMeter.symbol())
            ),
            t!("new-project-scales", "scales" => "1:50, 1:100"),
            t!("new-project-metric-sheets", "size" => SheetSize::A3.label()),
        ],
        MeasurementSystem::Imperial => {
            let units = Units::imperial();
            let length = |meters| format::length_with(&units, false, meters);
            [
                t!("new-project-imperial-units", "sample" => length(12.0 * FOOT + 6.5 * INCH)),
                t!("new-project-scales", "scales" => cc_core::scale::label(MeasurementSystem::Imperial, 48)),
                t!("new-project-imperial-sheets", "size" => SheetSize::AnsiB.label()),
            ]
        }
    }
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut selected) = state.dialogs.new_project else { return };
    let mut accepted = false;
    let mut create = 0;
    let response = Dialog::new("new-project", t!("new-project-title")).width(2.0 * CARD_WIDTH + CARD_GAP).show(
        ctx,
        |ui| {
            ui.label(theme::muted(t!("new-project-intro")));
            ui.add_space(4.0);
            let cards: Vec<(MeasurementSystem, Laid)> =
                MeasurementSystem::ALL.into_iter().map(|s| (s, lay_out(ui, s, s == selected))).collect();
            // Both cards as tall as the taller one.
            let height = cards.iter().map(|(_, laid)| laid.height()).fold(0.0, f32::max);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                for (system, laid) in cards {
                    let r = card(ui, &laid, system == selected, height);
                    if r.clicked() || r.double_clicked() {
                        selected = system;
                        // Enter is the dialog's: a focused card would take it.
                        ui.memory_mut(|m| m.surrender_focus(r.id));
                    }
                    accepted |= r.double_clicked();
                }
            });
        },
        |f| {
            create = f.primary(t!("new-project-create"));
            f.secondary(t!("action-cancel"));
        },
    );
    state.dialogs.new_project = Some(selected);
    match response.clicked {
        Some(i) if i == create => accepted = true,
        Some(_) => state.dialogs.new_project = None,
        None if response.dismissed => state.dialogs.new_project = None,
        None => {}
    }
    if accepted {
        state.dialogs.new_project = None;
        actions::guarded(state, PendingAction::New(selected));
    }
}

/// A card's texts, laid out at its inner width.
struct Laid {
    title: Arc<egui::Galley>,
    lines: Vec<Arc<egui::Galley>>,
    label: String,
}

/// Space between the title and the first line, and between lines.
const TITLE_GAP: f32 = 6.0;
const LINE_GAP: f32 = 2.0;

impl Laid {
    fn height(&self) -> f32 {
        let lines: f32 = self.lines.iter().map(|g| g.size().y + LINE_GAP).sum();
        2.0 * CARD_PAD + self.title.size().y + TITLE_GAP + lines - LINE_GAP
    }
}

fn lay_out(ui: &Ui, system: MeasurementSystem, selected: bool) -> Laid {
    let wrap = CARD_WIDTH - 2.0 * CARD_PAD;
    let body = egui::TextStyle::Body.resolve(ui.style());
    let label = title(system);
    let title_font = egui::FontId::new(body.size + 1.0, theme::semibold());
    let title_color = if selected { PALETTE.accent_text } else { PALETTE.text };
    let painter = ui.painter();
    let title = painter.layout(label.clone(), title_font, title_color, wrap);
    let lines =
        details(system).into_iter().map(|line| painter.layout(line, body.clone(), PALETTE.text_muted, wrap)).collect();
    Laid { title, lines, label }
}

/// One selectable card: its title over the lines of what it uses, with the
/// accent stroke and selection fill when selected and the hover fill under
/// the pointer.
fn card(ui: &mut Ui, laid: &Laid, selected: bool, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(CARD_WIDTH, height), Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, selected, laid.label.clone()));
    let painter = ui.painter();
    let radius = theme::RADIUS as f32;
    let (fill, stroke) = if selected {
        (PALETTE.accent_soft, Stroke::new(1.5, PALETTE.accent))
    } else if response.hovered() {
        (PALETTE.hover, Stroke::new(1.0, PALETTE.border_strong))
    } else {
        (PALETTE.panel, Stroke::new(1.0, PALETTE.border))
    };
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(rect, radius, stroke, StrokeKind::Inside);
    let mut pos = rect.min + vec2(CARD_PAD, CARD_PAD);
    painter.galley(pos, laid.title.clone(), PALETTE.text);
    pos.y += laid.title.size().y + TITLE_GAP;
    for line in &laid.lines {
        painter.galley(pos, line.clone(), PALETTE.text_muted);
        pos.y += line.size().y + LINE_GAP;
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cards show samples formatted as each template shows them.
    #[test]
    fn cards_show_formatted_samples() {
        let [units, scales, sheets] = details(MeasurementSystem::Imperial);
        assert!(units.contains("12'-6 1/2\""), "{units}");
        assert!(scales.contains("1/4\" = 1'-0\""), "{scales}");
        assert!(sheets.contains("ANSI B"), "{sheets}");
        let [units, scales, sheets] = details(MeasurementSystem::Metric);
        assert!(units.contains("m²") && units.contains("m³"), "{units}");
        assert!(scales.contains("1:50, 1:100"), "{scales}");
        assert!(sheets.contains("A3"), "{sheets}");
    }
}
