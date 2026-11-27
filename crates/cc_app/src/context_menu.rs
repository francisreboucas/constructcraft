//! The right-click menu over a 2D or 3D view (Select tool): the commands
//! for the selection, its category's commands (the same list as the Modify
//! tab's highlighted groups, [`commands::contextual`]), or, with nothing
//! selected, the view's. With another tool active, the menu cancels it
//! ([`tool`]).

use std::collections::BTreeSet;

use cc_core::{ElementId, ViewId};
use egui::{Response, Ui};

use crate::commands::{self, UiCommand};
use crate::state::AppState;
use crate::t;
use crate::tools::Tool;

/// A right click in `view` on `hit` (the elements under the pointer, or
/// none). Elements not selected become the selection; then the menu opens.
pub fn picked(state: &mut AppState, view: ViewId, hit: &[ElementId]) {
    if !hit.is_empty() && !hit.iter().all(|id| state.selection.contains(id)) {
        state.selection = hit.iter().copied().collect::<BTreeSet<_>>();
    }
    state.interaction.element_menu = Some(view);
}

/// The menu [`picked`] opened in `view`, under `resp`, while it stays open.
pub fn show(resp: &Response, state: &mut AppState, view: ViewId) {
    if state.interaction.element_menu != Some(view) {
        return;
    }
    let mut chosen = None;
    let open = resp.context_menu(|ui| chosen = items(ui, state));
    if open.is_none() && !resp.secondary_clicked() {
        state.interaction.element_menu = None;
    }
    match chosen {
        Some(Choice::Command(command)) => commands::run(state, &resp.ctx, command),
        Some(Choice::EditType(id)) => state.dialogs.type_editor = Some(id),
        None => {}
    }
}

/// A right click in `view` while a tool other than Select is active opens a
/// menu to cancel the tool (as Esc does, pressed until the Select tool is
/// back: what is being drawn is dropped, or ended as Esc ends it) or to fit
/// the view. Sketches and the component environment keep their own clicks.
pub fn tool(resp: &Response, state: &mut AppState, view: ViewId) {
    if state.tool == Tool::Select || state.sketch.is_some() || state.component_env.is_some() {
        if state.interaction.tool_menu == Some(view) {
            state.interaction.tool_menu = None;
        }
        return;
    }
    if resp.secondary_clicked() {
        state.interaction.tool_menu = Some(view);
    }
    if state.interaction.tool_menu != Some(view) {
        return;
    }
    let (mut cancel, mut fit) = (false, false);
    let open = resp.context_menu(|ui| {
        ui.set_min_width(180.0);
        if ui.add(egui::Button::new(t!("action-cancel")).shortcut_text("Esc")).clicked() {
            cancel = true;
            ui.close();
        }
        ui.separator();
        fit = item(ui, state, UiCommand::ZoomFit);
    });
    if open.is_none() && !resp.secondary_clicked() {
        state.interaction.tool_menu = None;
    }
    if cancel {
        cancel_tool(state);
    }
    if fit {
        commands::run(state, &resp.ctx, UiCommand::ZoomFit);
    }
}

/// Press Esc until the Select tool is back (a few presses at most: a typed
/// value, what is being drawn, the tool).
pub fn cancel_tool(state: &mut AppState) {
    for _ in 0..8 {
        if state.tool == Tool::Select {
            break;
        }
        crate::app::escape_step(state);
    }
    state.interaction.tool_menu = None;
}

/// What a click in the menu asks for.
enum Choice {
    Command(UiCommand),
    EditType(cc_core::ElementTypeId),
}

/// The menu's entries: the selection's, or the view's with nothing selected.
fn items(ui: &mut Ui, state: &AppState) -> Option<Choice> {
    use UiCommand::*;
    ui.set_min_width(220.0);
    let mut chosen = None;
    let mut section = |ui: &mut Ui, list: &[UiCommand]| {
        for command in list {
            if item(ui, state, *command) {
                chosen = Some(Choice::Command(*command));
            }
        }
    };
    if state.selection.is_empty() {
        section(ui, &[Paste, ZoomFit, VisibilityGraphics, TempReset]);
        for (_, list) in commands::contextual(state) {
            ui.separator();
            section(ui, &list);
        }
        return chosen;
    }
    section(ui, &[SelectInstances, CreateSimilar]);
    let edit_type = single_type(state);
    let r = ui.add_enabled(edit_type.is_some(), egui::Button::new(t!("props-edit-type")));
    let edit_type_clicked = r.on_disabled_hover_text(t!("msg-select-first")).clicked();
    for (_, list) in commands::contextual(state) {
        ui.separator();
        section(ui, &list);
    }
    ui.separator();
    section(ui, &[HideInView, TempHide, TempIsolate, TempHideCategory, TempIsolateCategory, OverrideGraphics]);
    ui.separator();
    section(ui, &[Cut, ClipboardCopy, Delete]);
    if edit_type_clicked && let Some(id) = edit_type {
        ui.close();
        return Some(Choice::EditType(id));
    }
    chosen
}

/// The type of the selection when it is one element, or several of one type.
fn single_type(state: &AppState) -> Option<cc_core::ElementTypeId> {
    let project = state.project();
    let mut types = state.selection.iter().take(2000).filter_map(|id| project.element(*id)).map(|e| e.type_id);
    let first = types.next()?;
    (types.all(|t| t == first) && project.element_type(first).is_some()).then_some(first)
}

/// One command: its name and shortcut, greyed with its reason when it cannot
/// run. Returns whether it was clicked (the menu then closes).
fn item(ui: &mut Ui, state: &AppState, command: UiCommand) -> bool {
    let why = command.unavailable(state);
    let mut button = egui::Button::new(command.label(state));
    if let Some(keys) = command.shortcut() {
        button = button.shortcut_text(keys);
    }
    let r = ui.add_enabled(why.is_none(), button);
    let r = match why {
        Some(why) => r.on_disabled_hover_text(why),
        None => r,
    };
    if r.clicked() {
        ui.close();
        return true;
    }
    false
}
