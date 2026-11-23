//! View-local access to the visual styles supported by the engine.

use crate::state::AppState;
use cc_core::{ViewId, ViewKind, VisualStyle};
use egui::{Rect, Ui};

/// Show the current style beside the drawing and persist changes through the view command.
pub fn show(ui: &mut Ui, state: &mut AppState, view_id: ViewId, canvas: Rect) {
    let Some(view) = state.project().view(view_id) else { return };
    if !matches!(
        view.kind,
        ViewKind::FloorPlan { .. }
            | ViewKind::CeilingPlan { .. }
            | ViewKind::Elevation { .. }
            | ViewKind::Section { .. }
            | ViewKind::ThreeD { .. }
    ) {
        return;
    }
    let current = view.style();
    let mut selected = current;
    let controlled = crate::graphics_dialog::controlled(state, view_id, "visual_style");
    let position = canvas.left_bottom() + egui::vec2(12.0, -40.0);
    let area = Rect::from_min_size(position, egui::vec2(180.0, 28.0));
    ui.painter().rect_filled(area, 5.0, crate::theme::PALETTE.panel);
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        ui.add_enabled_ui(!controlled, |ui| {
            egui::ComboBox::from_id_salt(("view-visual-style", view_id.0))
                .selected_text(crate::i18n::tr(current.key()))
                .width(160.0)
                .show_ui(ui, |ui| {
                    for style in VisualStyle::ALL {
                        ui.selectable_value(&mut selected, style, crate::i18n::tr(style.key()));
                    }
                })
                .response
                .on_hover_text(crate::t!("prop-visual-style"));
        });
    });
    if selected != current {
        crate::engine_edit::view(state, view_id, None, |view| view.visual_style = Some(selected));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    /// The selector updates both plan and 3D views through the normal command path.
    #[test]
    fn style_selector_updates_model_views() {
        for category in [cc_core::ViewCategory::FloorPlan, cc_core::ViewCategory::ThreeD] {
            let project = cc_core::Project::from_template();
            let view_id = project.views_of(category).next().unwrap().id;
            let state = AppState::new(cc_core::Document::new(project));
            let mut harness = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
                move |ui, state: &mut AppState| {
                    let canvas = ui.max_rect();
                    show(ui, state, view_id, canvas);
                },
                state,
            );
            harness.get_by_role(Role::ComboBox).click();
            harness.run();
            harness.get_by_label(&crate::i18n::tr(VisualStyle::Wireframe.key())).click();
            harness.run();
            assert_eq!(harness.state().project().view(view_id).unwrap().style(), VisualStyle::Wireframe);
        }
    }
}
