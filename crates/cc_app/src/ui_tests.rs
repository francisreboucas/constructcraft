//! Interface tests: the ribbon, the command palette and the icons run
//! headless with egui_kittest, queried through the accessibility tree that
//! the widgets describe (`widget_info`), and compared with screenshots.

use crate::state::{AppState, RibbonTab};
use crate::t;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

/// The sample house, open in the editor.
fn sample_state() -> AppState {
    let project = crate::home::sample_project("casa_simples").unwrap();
    let mut state = AppState::new(cc_core::Document::new(project));
    state.project_open = true;
    state.show_home = false;
    state
}

/// A harness drawing `draw` with the app theme. The theme's fonts load on
/// the frame after they are set, so the first frame (run by the builder)
/// only installs it.
fn themed_harness(
    state: AppState,
    size: egui::Vec2,
    theme: egui::Theme,
    mut draw: impl FnMut(&mut egui::Ui, &mut AppState) + 'static,
) -> Harness<'static, AppState> {
    let mut themed = false;
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        move |ui, state: &mut AppState| {
            if !themed {
                crate::theme::apply(ui.ctx());
                ui.ctx().set_theme(theme);
                themed = true;
                return;
            }
            crate::theme::sync(ui.ctx());
            draw(ui, state)
        },
        state,
    );
    harness.run();
    harness
}

/// [`themed_harness`] for a UI that keeps repainting (a spinner): its first
/// frames run without waiting for it to settle.
fn spinning_harness(
    state: AppState,
    size: egui::Vec2,
    draw: fn(&mut egui::Ui, &mut AppState),
) -> Harness<'static, AppState> {
    let mut themed = false;
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        move |ui, state: &mut AppState| {
            if !themed {
                crate::theme::apply(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Light);
                themed = true;
                return;
            }
            crate::theme::sync(ui.ctx());
            draw(ui, state)
        },
        state,
    );
    let _ = harness.try_run();
    harness
}

fn ribbon_harness(state: AppState, width: f32) -> Harness<'static, AppState> {
    ribbon_harness_sized(state, egui::vec2(width, 150.0))
}

fn ribbon_harness_sized(state: AppState, size: egui::Vec2) -> Harness<'static, AppState> {
    themed_harness(state, size, egui::Theme::Light, crate::ribbon::ribbon)
}

/// The command palette over an empty window.
fn palette_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(800.0, 480.0), egui::Theme::Light, |ui, state| {
        crate::palette::show(ui.ctx(), state)
    })
}

/// Screenshot tests compare renders with the images in `tests/snapshots`.
/// GPUs differ slightly, so they run on request: `CC_UI_SNAPSHOTS=1 cargo
/// test -p cc_app snapshots` compares, `UPDATE_SNAPSHOTS=1` rewrites them
/// after an intended change (review the new images before committing).
fn snapshots_requested() -> bool {
    std::env::var_os("CC_UI_SNAPSHOTS").is_some() || std::env::var_os("UPDATE_SNAPSHOTS").is_some()
}

#[test]
fn ribbon_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    let tabs = [
        RibbonTab::File,
        RibbonTab::Architecture,
        RibbonTab::Structure,
        RibbonTab::Mep,
        RibbonTab::MassingSite,
        RibbonTab::Insert,
        RibbonTab::Modify,
        RibbonTab::Annotate,
        RibbonTab::View,
        RibbonTab::Manage,
    ];
    for tab in tabs {
        let mut state = sample_state();
        state.ribbon_tab = tab;
        let mut harness = ribbon_harness(state, 1100.0);
        results.add(harness.try_snapshot(format!("ribbon_{tab:?}").to_lowercase()));
    }
    // The wall tool's Modify | Place Wall tab with its Draw group.
    let mut state = sample_state();
    let wall = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.set_tool(crate::tools::Tool::Wall { type_id: wall });
    results.add(ribbon_harness(state, 1600.0).try_snapshot("ribbon_place_wall"));
    // The Detail Line tool: Modify | Place Detail Line with Draw and Line Style.
    let mut state = sample_state();
    state.set_tool(crate::tools::Tool::DetailLine);
    results.add(ribbon_harness(state, 1600.0).try_snapshot("ribbon_place_detail_line"));
    // A floor sketch: Modify | Create Boundary with its Draw and Mode groups.
    let mut state = sample_state();
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    results.add(ribbon_harness(state, 1600.0).try_snapshot("ribbon_sketch_floor"));
    // Narrow window: the groups scroll.
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    results.add(ribbon_harness(state, 520.0).try_snapshot("ribbon_narrow"));
    // Collapsed, with a tab's panel shown over the views.
    let mut state = sample_state();
    state.ribbon_collapsed = true;
    state.ribbon_peek = true;
    state.ribbon_tab = RibbonTab::Annotate;
    results.add(ribbon_harness_sized(state, egui::vec2(900.0, 220.0)).try_snapshot("ribbon_peek"));
    // The rich tooltip of an unavailable command.
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Modify;
    let mut harness = ribbon_harness_sized(state, egui::vec2(1600.0, 300.0));
    harness.get_by_label(&t!("tool-move")).hover();
    harness.run();
    results.add(harness.try_snapshot("ribbon_tooltip"));
    // The palette searching.
    let mut state = sample_state();
    crate::palette::toggle(&mut state);
    let mut harness = palette_harness(state);
    harness.get_by_role(Role::TextInput).type_text("planta");
    harness.run();
    results.add(harness.try_snapshot("palette"));
    results.unwrap();
}

/// Every icon in both tones at the ribbon sizes, the badges, and the
/// enabled, selected and disabled states, in the light and dark themes.
fn icon_gallery(ui: &mut egui::Ui, _: &mut AppState) {
    use crate::icons::{self, Badge, Icon, Tones};
    let palette = crate::theme::palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.panel);
    let per_row = 18;
    for (i, icon) in Icon::ALL.iter().enumerate() {
        let origin = rect.min + egui::vec2(8.0 + (i % per_row) as f32 * 52.0, 8.0 + (i / per_row) as f32 * 30.0);
        for (k, side) in [16.0, 22.0].into_iter().enumerate() {
            let r = egui::Rect::from_min_size(origin + egui::vec2(k as f32 * 22.0, 0.0), egui::vec2(side, side));
            icons::paint_toned(ui.painter(), r, *icon, Tones::of(palette, None, true, false), None);
        }
    }
    let top = rect.min.y + 8.0 + Icon::ALL.len().div_ceil(per_row) as f32 * 30.0 + 8.0;
    let badges = [None, Some(Badge::New), Some(Badge::Import), Some(Badge::Export), Some(Badge::Remove)];
    for (row, (enabled, selected)) in [(true, false), (true, true), (false, false)].into_iter().enumerate() {
        for (col, badge) in badges.into_iter().enumerate() {
            let r = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + 8.0 + col as f32 * 36.0, top + row as f32 * 32.0),
                egui::vec2(24.0, 24.0),
            );
            if selected {
                ui.painter().rect_filled(r.expand(4.0), 4.0, palette.accent_soft);
            }
            let tones = Tones::of(palette, badge, enabled, selected);
            icons::paint_toned(ui.painter(), r, Icon::Component, tones, badge);
        }
    }
}

#[test]
fn icon_gallery_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for (theme, name) in [(egui::Theme::Light, "icons_light"), (egui::Theme::Dark, "icons_dark")] {
        let mut harness = themed_harness(sample_state(), egui::vec2(950.0, 360.0), theme, icon_gallery);
        results.add(harness.try_snapshot(name));
    }
    results.unwrap();
}

/// A ribbon wider than the window scrolls with arrows at its ends.
#[test]
fn narrow_ribbon_scrolls() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    let mut harness = ribbon_harness(state, 520.0);
    assert!(harness.get_by_label(&t!("ribbon-scroll-left")).accesskit_node().is_disabled(), "at the start");
    harness.get_by_label(&t!("ribbon-scroll-right")).click();
    harness.run();
    let left = t!("ribbon-scroll-left");
    assert!(!harness.get_by_label(&left).accesskit_node().is_disabled(), "scrolled away from the start");
    // Wide enough, no arrows.
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::MassingSite;
    let harness = ribbon_harness(state, 1100.0);
    assert!(harness.query_by_label(&t!("ribbon-scroll-right")).is_none());
}

/// Double-clicking a tab collapses the ribbon to its tabs; a click shows a
/// tab's panel over the views, and running a command closes it.
#[test]
fn ribbon_collapses_and_peeks() {
    let mut state = sample_state();
    // Text is in the default quick access toolbar; without it the button shows only in the panel.
    state.prefs.quick_access = Some(Vec::new());
    let mut harness = ribbon_harness_sized(state, egui::vec2(1100.0, 220.0));
    let tab = t!("ribbon-annotate");
    harness.get_by_label(&tab).click();
    harness.run();
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Annotate);
    // The harness clock runs faster than a hand, so allow a slow double click.
    harness.ctx.options_mut(|o| o.input_options.max_double_click_delay = 60.0);
    harness.get_by_label(&tab).click();
    harness.run();
    assert!(harness.state().ribbon_collapsed, "double click collapses");
    assert!(harness.query_by_role_and_label(Role::Button, &t!("tool-text")).is_none(), "no panel while collapsed");
    harness.get_by_label(&tab).click();
    harness.run();
    assert!(harness.state().ribbon_peek);
    harness.get_by_role_and_label(Role::Button, &t!("tool-text")).click();
    harness.run();
    assert!(matches!(harness.state().tool, crate::tools::Tool::Text { .. }));
    assert!(!harness.state().ribbon_peek, "the panel closes after a command");
    assert!(harness.state().ribbon_collapsed);
}

/// Typing in the palette finds commands by name, accents aside, and Enter runs the first.
#[test]
fn palette_runs_commands() {
    let mut state = sample_state();
    crate::palette::toggle(&mut state);
    let mut harness = palette_harness(state);
    harness.get_by_role(Role::TextInput).type_text("parede");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(matches!(harness.state().tool, crate::tools::Tool::Wall { .. }));
    assert!(harness.state().palette.is_none(), "the palette closes after running");
    // A two-letter shortcut finds its command too; unavailable ones explain why.
    crate::palette::toggle(harness.state_mut());
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("MV");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().message, t!("msg-select-first"));
}

/// The palette button opens the palette (its own click used to close it in
/// the same frame); a click outside closes it.
#[test]
fn palette_button_opens_the_palette() {
    let mut harness = themed_harness(sample_state(), egui::vec2(1400.0, 480.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::ribbon(ui, state);
        crate::palette::show(ui.ctx(), state);
    });
    harness.get_by_role_and_label(Role::Button, &t!("action-command-palette")).click();
    harness.run();
    assert!(harness.state().palette.is_some(), "the button opens the palette");
    assert!(harness.query_by_role(Role::TextInput).is_some());
    harness.get_by_role_and_label(Role::Button, &t!("action-command-palette")).click();
    harness.run();
    assert!(harness.state().palette.is_none(), "a click outside closes it");
}

/// Engine commands open a form with their arguments and run in the session.
#[test]
fn palette_runs_engine_commands() {
    let mut state = sample_state();
    let levels = state.project().levels.len();
    crate::palette::toggle(&mut state);
    let mut harness = palette_harness(state);
    harness.get_by_role(Role::TextInput).type_text("level_create");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let editor = harness.get_by_role(Role::MultilineTextInput);
    editor.focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_role(Role::MultilineTextInput).type_text(r#"{"elevation": 9.0, "name": "Roof deck"}"#);
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(harness.state().project().levels.len(), levels + 1);
    assert!(harness.state().project().levels.iter().any(|l| l.name == "Roof deck"));
}

/// Every ribbon button tells assistive technology its name, and every
/// interface command shown in a tab can be found by its label.
#[test]
fn ribbon_buttons_are_accessible() {
    use crate::commands::UiCommand;
    let tabs = [
        RibbonTab::File,
        RibbonTab::Architecture,
        RibbonTab::Structure,
        RibbonTab::Mep,
        RibbonTab::MassingSite,
        RibbonTab::Insert,
        RibbonTab::Modify,
        RibbonTab::Annotate,
        RibbonTab::View,
        RibbonTab::Manage,
    ];
    let mut found = std::collections::HashSet::new();
    // The Structure tab shows other groups per material.
    for (tab, material) in tabs.into_iter().flat_map(|t| cc_core::StructuralMaterial::ALL.map(|m| (t, m))) {
        let mut state = sample_state();
        state.ribbon_tab = tab;
        state.prefs.structure_material = material;
        // File is a page over the window: it needs the window's height.
        let height = if tab == RibbonTab::File { 800.0 } else { 150.0 };
        let mut harness = ribbon_harness_sized(state, egui::vec2(1600.0, height));
        // Families too long for the row open as menus, and File's exports are a page: open each one.
        let menus = [t!("group-temporary"), t!("group-documentation"), t!("group-export")];
        let mut open = vec![None];
        open.extend(menus.iter().map(Some));
        for menu in open {
            if let Some(menu) = menu {
                let Some(button) = harness.query_by_role_and_label(Role::Button, menu) else { continue };
                button.click();
                harness.run();
            }
            for node in harness.query_all_by_role(Role::Button) {
                let label = node.accesskit_node().label().unwrap_or_default();
                assert!(!label.trim().is_empty(), "{tab:?}: a button without a name");
                found.insert(label);
            }
        }
    }
    // Modify shows the commands of the selected category (and of the active sheet).
    let mut state = sample_state();
    let plan = state.project().views_of(cc_core::ViewCategory::FloorPlan).next().map(|v| v.id).unwrap();
    let sweep = serde_json::json!({"path": [[0.0, 0.0], [3.0, 0.0]]});
    let sweep = state.session.execute("sweep_create", sweep).unwrap()["id"].as_u64().unwrap();
    let text = serde_json::json!({"view": plan.0, "position": [1.0, 1.0], "text": "A"});
    let text = state.session.execute("text_create", text).unwrap()["id"].as_u64().unwrap();
    let project = state.project();
    let first = |c: cc_core::Category| project.elements_of(c).next().map(|e| e.id).unwrap();
    let mut picks: Vec<Vec<cc_core::ElementId>> =
        [cc_core::Category::Wall, cc_core::Category::Roof, cc_core::Category::Floor, cc_core::Category::Grid]
            .map(|c| vec![first(c)])
            .to_vec();
    picks.push(vec![first(cc_core::Category::Component)]);
    // A room: its interior elevations.
    picks.push(vec![first(cc_core::Category::Room)]);
    picks.push(vec![cc_core::ElementId(sweep), cc_core::ElementId(text)]);
    let sheet = project.sheets_in_order()[0];
    for pick in picks {
        let mut state = sample_state();
        state.session.execute("sweep_create", serde_json::json!({"path": [[0.0, 0.0], [3.0, 0.0]]})).unwrap();
        state
            .session
            .execute("text_create", serde_json::json!({"view": plan.0, "position": [1.0, 1.0], "text": "A"}))
            .unwrap();
        state.selection = pick.into_iter().collect();
        state.ribbon_tab = RibbonTab::Modify;
        state.active_view = Some(sheet);
        let harness = ribbon_harness(state, 2400.0);
        for node in harness.query_all_by_role(Role::Button) {
            found.insert(node.accesskit_node().label().unwrap_or_default());
        }
    }
    // The component environment shows its own ribbon.
    let mut state = sample_state();
    let table = state
        .project()
        .types_of(cc_core::Category::Component)
        .find(|t| matches!(&t.data, cc_core::TypeData::Component { component, .. } if component == "Mesa"))
        .map(|t| t.id)
        .unwrap();
    crate::component_env::open(&mut state, table);
    let harness = themed_harness(state, egui::vec2(1600.0, 140.0), egui::Theme::Light, |ui, state| {
        crate::component_env::ribbon(ui, state)
    });
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    // And the title block editor.
    let mut state = sample_state();
    let sheet = state.project().sheets_in_order()[0];
    state.active_view = Some(sheet);
    crate::title_block_env::edit_sheet(&mut state, sheet);
    let harness = themed_harness(state, egui::vec2(1600.0, 140.0), egui::Theme::Light, |ui, state| {
        crate::component_env::ribbon(ui, state)
    });
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    // The wall tool's Modify | Place Wall tab adds its own drawing modes.
    let mut state = sample_state();
    let wall = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.set_tool(crate::tools::Tool::Wall { type_id: wall });
    let harness = ribbon_harness(state, 2400.0);
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    // And the Detail Line tool's Modify | Place Detail Line tab.
    let mut state = sample_state();
    state.set_tool(crate::tools::Tool::DetailLine);
    let harness = ribbon_harness(state, 2400.0);
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    // So does an open sketch.
    let mut state = sample_state();
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    let harness = themed_harness(state, egui::vec2(1600.0, 140.0), egui::Theme::Light, |ui, state| {
        crate::sketch_mode::ribbon(ui, state)
    });
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    // A terrain's sketch adds the Point tool.
    let mut state = sample_state();
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Topography);
    let harness = themed_harness(state, egui::vec2(1600.0, 140.0), egui::Theme::Light, |ui, state| {
        crate::sketch_mode::ribbon(ui, state)
    });
    for node in harness.query_all_by_role(Role::Button) {
        found.insert(node.accesskit_node().label().unwrap_or_default());
    }
    let state = sample_state();
    // Zoom to Fit lives in each view's bar (and ZE), not in a tab.
    let hidden = [UiCommand::CommandPalette, UiCommand::ToggleRibbon, UiCommand::ZoomFit];
    for command in UiCommand::all().into_iter().filter(|c| !hidden.contains(c)) {
        assert!(found.contains(&command.label(&state)), "{command:?} is not in the ribbon");
    }
}

/// Picking the wall tool opens the *Modify | Place Wall* tab, whose Draw
/// group switches between its modes; leaving the tool goes back to the tab
/// shown before.
#[test]
fn wall_tool_draw_modes_are_in_its_modify_tab() {
    use crate::tools::{Tool, WallMode};
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    let wall_type = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.set_tool(Tool::Wall { type_id: wall_type });
    assert_eq!(state.ribbon_tab, RibbonTab::Modify);
    let mut harness = ribbon_harness(state, 2400.0);
    let title =
        format!("{} | {}", t!("ribbon-modify"), t!("ribbon-place-tool", "tool" => crate::i18n::tr("tool-wall")));
    assert!(harness.query_by_label(&title).is_some(), "the tab names the tool");
    assert_eq!(harness.state().wall_mode, WallMode::Line);
    for (label, mode) in [
        (t!("sketch-tool-arc3"), WallMode::Arc),
        (t!("sketch-tool-rect"), WallMode::Rect),
        (t!("sketch-tool-polygon-out"), WallMode::PolygonOut),
        (t!("sketch-tool-ellipse"), WallMode::Ellipse),
        (t!("sketch-tool-spline"), WallMode::Spline),
        (t!("wall-tool-fillet"), WallMode::Fillet),
        (t!("wall-tool-pick-lines"), WallMode::PickLines),
        (t!("sketch-tool-line"), WallMode::Line),
    ] {
        harness.get_by_role_and_label(Role::Button, &label).click();
        harness.run();
        assert_eq!(harness.state().wall_mode, mode, "{label}");
    }
    harness.state_mut().set_tool(Tool::Select);
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Architecture, "back to the tab shown before");
}

/// The Detail Line tool opens *Modify | Place Detail Line*: its Draw group
/// picks the shape, the Line Style group the style; the options bar keeps
/// Chain (lines) or Sides (polygons); leaving the tool goes back.
#[test]
fn detail_line_tool_shapes_are_in_its_modify_tab() {
    use crate::sketch_mode::Draw;
    use crate::tools::Tool;
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Annotate;
    state.set_tool(Tool::DetailLine);
    assert_eq!(state.ribbon_tab, RibbonTab::Modify);
    let mut harness = ribbon_harness(state, 2400.0);
    let tool = crate::i18n::tr("tool-detail-line");
    let title = format!("{} | {}", t!("ribbon-modify"), t!("ribbon-place-tool", "tool" => tool));
    assert!(harness.query_by_label(&title).is_some(), "the tab names the tool");
    assert!(harness.query_by_label(&t!("label-line-style")).is_some(), "the line style group");
    assert_eq!(harness.state().detail.draw, Draw::Line);
    for (label, draw) in [
        (t!("sketch-tool-rect"), Draw::Rect),
        (t!("sketch-tool-polygon"), Draw::Polygon),
        (t!("sketch-tool-circle"), Draw::Circle),
        (t!("sketch-tool-arc-tangent"), Draw::ArcTangent),
        (t!("sketch-tool-spline"), Draw::Spline),
        (t!("sketch-tool-line"), Draw::Line),
    ] {
        harness.get_by_role_and_label(Role::Button, &label).click();
        harness.run();
        assert_eq!(harness.state().detail.draw, draw, "{label}");
    }
    // Fillet Arc picks lines instead of drawing; a shape button goes back to drawing.
    harness.get_by_role_and_label(Role::Button, &t!("wall-tool-fillet")).click();
    harness.run();
    assert!(harness.state().detail.fillet);
    harness.get_by_role_and_label(Role::Button, &t!("sketch-tool-line")).click();
    harness.run();
    assert!(!harness.state().detail.fillet);
    // Wall-only modes stay out.
    assert!(harness.query_by_label(&t!("wall-tool-pick-lines")).is_none());
    harness.state_mut().set_tool(Tool::Select);
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Annotate, "back to the tab shown before");

    // The options bar: Chain for lines, Sides for polygons.
    let mut state = sample_state();
    state.set_tool(Tool::DetailLine);
    state.detail.draw = Draw::Polygon;
    let mut bar = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    assert!(bar.query_by_label(&t!("sketch-sides")).is_some());
    bar.state_mut().detail.draw = Draw::Line;
    bar.run();
    assert!(bar.query_by_label(&t!("sketch-chain")).is_some());
    assert!(bar.query_by_label(&t!("opt-offset")).is_none(), "a chain is made a segment at a time");
    // A rectangle: offset and corner radius; Fillet Arc: its own radius.
    bar.state_mut().detail.draw = Draw::Rect;
    bar.run();
    assert!(bar.query_by_label(&t!("opt-offset")).is_some());
    assert!(bar.query_by_label(&t!("opt-radius")).is_some());
    bar.state_mut().detail.fillet = true;
    bar.run();
    assert!(bar.query_by_label(&t!("opt-offset")).is_none());
    assert!(bar.query_by_label(&t!("hint-detail-fillet")).is_some());
}

/// The Detail Line tool draws polygons and splines too: a hexagon is six
/// detail lines, Enter ends a spline as one detail line (wherever the
/// pointer is), a tangent arc starts only at a free end, a radius rounds a
/// rectangle's corners and places a circle in one click, and Fillet Arc
/// rounds the corner of two lines.
#[test]
fn detail_line_tool_draws_polygons_and_splines() {
    use crate::sketch_mode::Draw;
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let made = crate::actions::run_command(
        &mut state,
        cc_engine::commands::views::ViewCreate { kind: "drafting".into(), ..Default::default() },
    );
    let view = cc_core::ViewId(made.unwrap()["id"].as_u64().unwrap());
    state.active_view = Some(view);
    state.set_tool(crate::tools::Tool::DetailLine);
    state.detail.draw = Draw::Polygon;
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: glam::DVec2::ZERO, scale: 20.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        crate::detail_ui::tool(ui, &resp, &painter, state, &view, &xf);
    });
    harness.run();
    let click = |h: &mut Harness<'static, AppState>, pos: egui::Pos2| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.run();
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
            h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
            h.run();
        }
    };
    let lines = |h: &Harness<'static, AppState>| -> Vec<SketchCurve> {
        h.state()
            .project()
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                cc_core::ElementKind::DetailLine(l) if l.view == view => Some(l.curve.clone()),
                _ => None,
            })
            .collect()
    };
    click(&mut harness, egui::pos2(150.0, 200.0));
    click(&mut harness, egui::pos2(190.0, 200.0));
    assert_eq!(lines(&harness).len(), 6, "{}", harness.state().message);

    harness.state_mut().detail.draw = Draw::Spline;
    harness.run();
    for (x, y) in [(300.0, 100.0), (360.0, 140.0), (420.0, 100.0)] {
        click(&mut harness, egui::pos2(x, y));
    }
    // Enter ends it even with the pointer off the view.
    harness.input_mut().events.push(egui::Event::PointerGone);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let made = lines(&harness);
    assert_eq!(made.len(), 7, "{}", harness.state().message);
    assert!(made.iter().any(|c| matches!(c, SketchCurve::Spline { .. })));

    // Two chained lines A-B-C; a tangent arc starts at the free end A, not at B.
    let (a, b, c) = (egui::pos2(100.0, 300.0), egui::pos2(200.0, 300.0), egui::pos2(200.0, 250.0));
    harness.state_mut().detail.draw = Draw::Line;
    harness.run();
    for p in [a, b, c] {
        click(&mut harness, p);
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(lines(&harness).len(), 9, "{}", harness.state().message);
    harness.state_mut().detail.draw = Draw::ArcTangent;
    harness.state_mut().detail.chain = false;
    harness.run();
    click(&mut harness, b);
    click(&mut harness, egui::pos2(260.0, 240.0));
    assert_eq!(lines(&harness).len(), 9, "a joined end is no start");
    click(&mut harness, a);
    click(&mut harness, egui::pos2(60.0, 260.0));
    let made = lines(&harness);
    assert_eq!(made.len(), 10, "{}", harness.state().message);
    assert!(made.iter().any(|c| matches!(c, SketchCurve::Arc { .. })));

    // A rectangle with a corner radius: four sides and four arcs.
    harness.state_mut().detail.draw = Draw::Rect;
    harness.state_mut().detail.radius = 0.5;
    harness.run();
    click(&mut harness, egui::pos2(350.0, 250.0));
    click(&mut harness, egui::pos2(450.0, 320.0));
    assert_eq!(lines(&harness).len(), 18, "{}", harness.state().message);
    // With a radius, a circle takes one click.
    harness.state_mut().detail.draw = Draw::Circle;
    harness.run();
    click(&mut harness, egui::pos2(520.0, 60.0));
    let made = lines(&harness);
    assert_eq!(made.len(), 19, "{}", harness.state().message);
    assert!(made.iter().any(|c| matches!(c, SketchCurve::Circle { radius, .. } if (radius - 0.5).abs() < 1e-9)));

    // Fillet Arc rounds the corner B of the two lines drawn before.
    harness.state_mut().detail.fillet = true;
    harness.state_mut().detail.fillet_radius = 1.0;
    harness.run();
    click(&mut harness, egui::pos2(150.0, 300.0));
    click(&mut harness, egui::pos2(200.0, 270.0));
    let made = lines(&harness);
    assert_eq!(made.len(), 20, "{}", harness.state().message);
    let arcs = made.iter().filter(|c| matches!(c, SketchCurve::Arc { .. })).count();
    assert_eq!(arcs, 6, "the tangent arc, four corners and the fillet");
}

/// The options bar of a shape mode shows its offset, radius and sides.
#[test]
fn wall_shape_modes_have_their_options() {
    use crate::tools::WallMode;
    let mut state = sample_state();
    let wall_type = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.tool = crate::tools::Tool::Wall { type_id: wall_type };
    state.wall_mode = WallMode::Polygon;
    let harness = themed_harness(state, egui::vec2(1800.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    for key in ["opt-offset", "opt-radius", "sketch-sides", "opt-wall-join"] {
        assert!(harness.query_by_label(&crate::i18n::tr(key)).is_some(), "{key}");
    }
}

/// The options bar of the hosted tool over `state`, 1600 px wide.
fn options_bar_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    })
}

/// The text the options bar's sill field shows.
fn sill_field_text(harness: &Harness<'static, AppState>) -> String {
    let label = t!("prop-sill");
    harness.get_by_role_and_label(Role::SpinButton, &label).value().unwrap_or_default()
}

/// A length as a field shows it, with its unit (feet and inches carry their own marks).
fn length_with_unit(state: &AppState, meters: f64) -> String {
    let unit = state.project().units.length.unit;
    if unit.is_fractional() { length_text(meters) } else { format!("{} {}", length_text(meters), unit.symbol()) }
}

/// Type `meters` into the options bar's sill field and press Enter.
fn type_sill(harness: &mut Harness<'static, AppState>, meters: f64) {
    harness.get_by_role_and_label(Role::SpinButton, &t!("prop-sill")).focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text(&length_text(meters));
    harness.key_press(egui::Key::Enter);
    harness.run();
}

/// Pick `name` in the options bar's type list.
fn pick_tool_type(harness: &mut Harness<'static, AppState>, name: &str) {
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

/// Place the tool's opening in the middle of the sample's longest wall
/// with no door, window or niche there (as a click in a 3D view does) and
/// return its sill.
fn place_and_read_sill(state: &mut AppState) -> f64 {
    state.session.geometry();
    let clear = |g: &cc_geom::WallGeom| {
        let mid = g.length * 0.5;
        let busy = g.openings.iter().map(|o| (o.u0, o.u1)).chain(g.recesses.iter().map(|r| (r.u0, r.u1)));
        g.curtain.is_none() && g.length > 2.5 && busy.into_iter().all(|(u0, u1)| u1 < mid - 1.0 || u0 > mid + 1.0)
    };
    let walls = state.session.geom().walls.iter().filter(|(_, g)| clear(g));
    let (wall, g) = walls.max_by(|a, b| a.1.length.total_cmp(&b.1.length)).unwrap();
    let (wall, point) = (*wall, g.world(g.length * 0.5, g.half));
    let tool = state.tool.clone();
    let before = state.project().elements.len();
    crate::tools::place_opening_3d(state, &tool, wall, point);
    assert_eq!(state.project().elements.len(), before + 1, "{}", state.message);
    let placed = state.selection.iter().next().copied().unwrap();
    state.project().element(placed).and_then(|e| e.as_hosted()).map(|h| h.sill_height).unwrap()
}

/// Issue #304: the window tool's options bar shows the sill of the type,
/// a typed sill is the one windows are placed at, and another type (or
/// another tool) brings the type's own sill back.
#[test]
fn the_window_tool_takes_a_sill() {
    use cc_core::Category;
    let mut state = sample_state();
    let windows: Vec<(cc_core::ElementTypeId, String, f64)> =
        state.project().types_of(Category::Window).map(|t| (t.id, t.name.clone(), t.default_sill())).collect();
    let first = windows[0].clone();
    let other = windows.iter().find(|w| (w.2 - first.2).abs() > 1e-6).unwrap().clone();
    state.set_tool(crate::tools::Tool::Hosted { category: Category::Window, type_id: first.0 });
    let mut harness = options_bar_harness(state);
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), first.2));

    type_sill(&mut harness, 1.5);
    assert_eq!(harness.state().hosted_sill, Some(1.5));
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), 1.5));
    let sill = place_and_read_sill(harness.state_mut());
    assert!((sill - 1.5).abs() < 1e-9, "placed at {sill}");

    // Another type: the field follows it again.
    pick_tool_type(&mut harness, &other.1);
    assert!(matches!(harness.state().tool, crate::tools::Tool::Hosted { type_id, .. } if type_id == other.0));
    assert_eq!(harness.state().hosted_sill, None);
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), other.2));
    let sill = place_and_read_sill(harness.state_mut());
    assert!((sill - other.2).abs() < 1e-9, "placed at {sill}");

    // Out of range is kept in range; another tool forgets the sill.
    type_sill(&mut harness, 50.0);
    assert_eq!(harness.state().hosted_sill, Some(*crate::tools::SILL_RANGE.end()));
    harness.state_mut().set_tool(crate::tools::Tool::Select);
    harness.state_mut().set_tool(crate::tools::Tool::Hosted { category: Category::Window, type_id: other.0 });
    harness.run();
    assert_eq!(harness.state().hosted_sill, None);
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), other.2));
}

/// Issue #304: the niche tool takes a sill the same way; the door tool has none.
#[test]
fn the_niche_tool_takes_a_sill_and_the_door_tool_none() {
    use cc_core::Category;
    let mut state = sample_state();
    crate::actions::activate_category_tool(&mut state, Category::Niche);
    let niche = state.project().default_type(Category::Niche).unwrap();
    let niche_sill = state.project().element_type(niche).unwrap().default_sill();
    let shelf = cc_engine::commands::niche::NicheTypeCreate {
        name: Some("Shelf".into()),
        sill: Some(0.2),
        ..Default::default()
    };
    crate::actions::run_command(&mut state, shelf);
    let mut harness = options_bar_harness(state);
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), niche_sill));
    type_sill(&mut harness, 0.4);
    assert_eq!(harness.state().hosted_sill, Some(0.4));
    let sill = place_and_read_sill(harness.state_mut());
    assert!((sill - 0.4).abs() < 1e-9, "placed at {sill}");
    pick_tool_type(&mut harness, "Shelf");
    assert_eq!(harness.state().hosted_sill, None);
    assert_eq!(sill_field_text(&harness), length_with_unit(harness.state(), 0.2));

    // Doors stand on the floor: no sill in their options.
    let door = harness.state().project().default_type(Category::Door).unwrap();
    harness.state_mut().set_tool(crate::tools::Tool::Hosted { category: Category::Door, type_id: door });
    harness.run();
    assert!(harness.query_by_label(&t!("prop-sill")).is_none());
    assert!(harness.query_by_label(&t!("prop-flip-hand")).is_some(), "the door's options are there");
}

/// The options bar of the window and niche tools, written as PNG files to
/// the folder `CC_RENDER_SILL_OPTION` names, for review.
#[test]
fn render_sill_option() {
    let Some(dir) = std::env::var_os("CC_RENDER_SILL_OPTION") else { return };
    let dir = std::path::PathBuf::from(dir);
    for category in [cc_core::Category::Window, cc_core::Category::Niche] {
        let mut state = sample_state();
        crate::actions::activate_category_tool(&mut state, category);
        let mut harness = options_bar_harness(state);
        harness.run();
        let name = format!("sill_option_{}.png", format!("{category:?}").to_lowercase());
        harness.render().unwrap().save(dir.join(name)).unwrap();
    }
}

/// A structural wall's palette offers its reinforcement: the typical button
/// makes horizontal and vertical bars near both faces, listed with their
/// spacing (#281).
#[test]
fn structural_walls_show_their_reinforcement() {
    let mut state = sample_state();
    let level = state.project().levels[0].id.0;
    let args = serde_json::json!({"level": level, "start": [100, 100], "end": [106, 100], "height": 3.0});
    let wall = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    // Not on a wall that carries nothing.
    state.selection = [wall].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("action-rebar-typical")).is_none());
    let params = serde_json::json!({"id": wall.0, "params": {"structural": true}});
    harness.state_mut().session.execute("element_set", params).unwrap();
    harness.run();
    harness.get_by_label(&t!("action-rebar-typical")).click();
    harness.run();
    let sets: Vec<cc_core::RebarLayout> =
        harness.state().project().rebar_of(wall).map(|(_, r)| r.layout.clone()).collect();
    assert_eq!(sets.len(), 4);
    assert_eq!(sets.iter().filter(|l| matches!(l, cc_core::RebarLayout::WallHorizontal { .. })).count(), 2);
    assert!(harness.query_by_label(&t!("action-rebar-typical")).is_none(), "reinforced now");
}

/// A structural floor's palette offers its reinforcement: the typical button
/// makes a mesh in its top and bottom layers (#282).
#[test]
fn floors_show_their_reinforcement() {
    let mut state = sample_state();
    let level = state.project().levels[0].id.0;
    let args = serde_json::json!({"level": level, "boundary": [[100, 100], [104, 100], [104, 103], [100, 103]]});
    let floor = cc_core::ElementId(state.session.execute("floor_create", args).unwrap()["id"].as_u64().unwrap());
    state.selection = [floor].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    // Not on a floor that carries nothing.
    assert!(harness.query_by_label(&t!("action-rebar-typical")).is_none());
    let params = serde_json::json!({"id": floor.0, "params": {"structural": true}});
    harness.state_mut().session.execute("element_set", params).unwrap();
    harness.run();
    harness.get_by_label(&t!("action-rebar-typical")).click();
    harness.run();
    let sets: Vec<cc_core::RebarLayout> =
        harness.state().project().rebar_of(floor).map(|(_, r)| r.layout.clone()).collect();
    assert_eq!(sets.len(), 4);
    assert!(sets.iter().all(|l| matches!(l, cc_core::RebarLayout::SlabStraight { .. })));
    assert!(harness.query_by_label(&t!("action-rebar-typical")).is_none(), "reinforced now");
}

/// The beam tool draws beams in line or arc mode (#279).
#[test]
fn beam_tool_has_an_arc_mode() {
    let mut state = sample_state();
    let beam = state.project().default_type(cc_core::Category::Beam).unwrap();
    state.tool = crate::tools::Tool::Beam { type_id: beam, material: None };
    let mut harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    assert!(!harness.state().beam_arc);
    harness.get_by_label(&t!("wall-mode-arc")).click();
    harness.run();
    assert!(
        harness.state().beam_arc
            && (harness.state().wall_mode != crate::tools::WallMode::Arc)
            && !harness.state().strip_arc
    );
    harness.get_by_label(&t!("wall-mode-line")).click();
    harness.run();
    assert!(!harness.state().beam_arc);
}

/// With a strip type, the footing tool draws strips in line or arc mode (#22).
#[test]
fn strip_footing_tool_has_an_arc_mode() {
    let mut state = sample_state();
    let strip = crate::tools::foundation_type(state.project(), cc_core::FoundationKind::Strip).unwrap();
    state.tool = crate::tools::Tool::Footing { type_id: strip };
    let mut harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    // The tool is named for strips, not pad footings.
    harness.get_by_label(&t!("tool-strip-footing"));
    assert!(harness.query_by_label(&t!("tool-footing")).is_none());
    assert!(!harness.state().strip_arc);
    harness.get_by_label(&t!("wall-mode-arc")).click();
    harness.run();
    assert!(harness.state().strip_arc && (harness.state().wall_mode != crate::tools::WallMode::Arc));
    harness.get_by_label(&t!("wall-mode-line")).click();
    harness.run();
    assert!(!harness.state().strip_arc);
}

/// Aligned and linear dimensions are placed singly or in a continuous chain.
#[test]
fn dimension_tool_has_a_continuous_mode() {
    use crate::tools::{DimChain, DimensionMode, Tool};
    let mut state = sample_state();
    let type_id = state.project().default_type(cc_core::Category::Dimension).unwrap();
    state.tool = Tool::Dimension { type_id, mode: DimensionMode::Linear };
    let mut harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    assert!(!harness.state().dim_continuous);
    harness.get_by_label(&t!("dim-continuous")).click();
    harness.run();
    assert!(harness.state().dim_continuous);
    // A chain in progress shows how to continue and end it.
    let view = cc_core::ViewId(1);
    let chain = DimChain {
        view,
        last: glam::DVec2::ZERO,
        dir: glam::DVec2::X,
        line: glam::DVec2::Y,
        linear: true,
        ordinate: false,
    };
    harness.state_mut().interaction.dim_chain = Some(chain);
    harness.run();
    assert!(harness.query_by_label(&t!("hint-dim-chain")).is_some());
    harness.get_by_label(&t!("dim-single")).click();
    harness.run();
    assert!(!harness.state().dim_continuous);
    // Ordinate dimensions always continue from their origin.
    harness.state_mut().tool = Tool::Dimension { type_id, mode: DimensionMode::Ordinate };
    harness.state_mut().interaction.dim_chain = Some(DimChain { ordinate: true, ..chain });
    harness.run();
    assert!(harness.query_by_label(&t!("dim-continuous")).is_none());
    assert!(harness.query_by_label(&t!("hint-dim-ordinate-next")).is_some());
    // Angular dimensions have no chain.
    harness.state_mut().interaction.dim_chain = None;
    harness.state_mut().tool = Tool::Dimension { type_id, mode: DimensionMode::Angular };
    harness.run();
    assert!(harness.query_by_label(&t!("dim-continuous")).is_none());
}

/// A roof gets an opening (a courtyard) sketched inside it; one outside is refused.
#[test]
fn roof_openings_are_added_inside_the_roof() {
    let mut state = sample_state();
    let (roof, footprint) = state
        .project()
        .elements
        .values()
        .find_map(|e| match &e.kind {
            cc_core::ElementKind::Roof(r) => Some((e.id, r.footprint.clone())),
            _ => None,
        })
        .unwrap();
    let lo = footprint.iter().fold(glam::DVec2::splat(f64::MAX), |a, p| a.min(*p));
    let hi = footprint.iter().fold(glam::DVec2::splat(f64::MIN), |a, p| a.max(*p));
    let c = (lo + hi) * 0.5;
    let square = |c: glam::DVec2| {
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().map(|&(x, y)| c + glam::DVec2::new(x, y)).collect()
    };
    let holes = |state: &AppState| match &state.project().element(roof).unwrap().kind {
        cc_core::ElementKind::Roof(r) => r.holes.len(),
        _ => 0,
    };
    state.set_tool(crate::tools::Tool::Opening { host: roof, floor: false });
    crate::tools::add_opening(&mut state, roof, square(hi + glam::DVec2::splat(5.0)));
    assert_eq!(holes(&state), 0, "outside the roof");
    crate::tools::add_opening(&mut state, roof, square(c));
    assert_eq!(holes(&state), 1);
    assert_eq!(state.tool, crate::tools::Tool::Select);
    state.sync();
    assert!(state.session.geom().roof_ridges.contains_key(&roof));
}

/// Visibility/Graphics of the ground plan, with a filter applied.
fn graphics_state() -> (AppState, cc_core::ViewId) {
    let mut state = sample_state();
    let plan = state.project().views_of(cc_core::ViewCategory::FloorPlan).next().unwrap().id;
    state.active_view = Some(plan);
    let rules = vec![cc_engine::commands::graphics::RuleArg {
        param: "length".into(),
        op: "greater".into(),
        value: Some(serde_json::json!(5.0)),
    }];
    let filter = cc_engine::commands::graphics::FilterCreate {
        name: "Long walls".into(),
        categories: vec!["wall".into()],
        rules,
        any: false,
    };
    state.session.run(filter).unwrap();
    let add = cc_engine::commands::graphics::ViewFilterAdd {
        view: Some(cc_engine::Ref::Id(plan.0)),
        filter: cc_engine::Ref::Name("Long walls".into()),
        overrides: Some(serde_json::json!({"cut_lines": {"color": [200, 0, 0], "weight": 0.7}})),
        ..Default::default()
    };
    state.session.run(add).unwrap();
    (state, plan)
}

#[test]
fn graphics_dialog_hides_categories_and_cancel_undoes() {
    let (mut state, plan) = graphics_state();
    let depth = state.session.doc.undo_depth();
    crate::graphics_dialog::open(&mut state, crate::graphics_dialog::Target::View(plan));
    let mut harness = themed_harness(state, egui::vec2(980.0, 680.0), egui::Theme::Light, |ui, state| {
        crate::graphics_dialog::show(ui.ctx(), state)
    });
    let walls = format!("{}: {}", t!("vg-visible"), crate::actions::t_category(cc_core::Category::Wall));
    harness.get_by_label(&walls).click_accesskit();
    harness.run();
    let hidden = |s: &AppState| !s.project().view(plan).unwrap().graphics.category(cc_core::Category::Wall).visible;
    assert!(hidden(harness.state()));
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(!hidden(harness.state()));
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
    assert!(harness.state().dialogs.graphics.is_none());
}

/// The filters and view templates dialogs fit a tall window: their header
/// and buttons stay on screen (a divider once stretched them past it).
#[test]
fn list_dialogs_fit_a_tall_window() {
    let size = egui::vec2(1600.0, 1400.0);
    let (mut state, _) = graphics_state();
    crate::graphics_dialog::open_filters(&mut state);
    let mut filters =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::graphics_dialog::show(ui.ctx(), state));
    let (mut state, plan) = graphics_state();
    let create = cc_engine::commands::graphics::TemplateCreate {
        name: "Working plan".into(),
        view: Some(cc_engine::Ref::Id(plan.0)),
    };
    state.session.run(create).unwrap();
    crate::graphics_dialog::open_templates(&mut state);
    let mut templates =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::graphics_dialog::show(ui.ctx(), state));
    // Empty lists: nothing selected.
    let mut state = sample_state();
    crate::graphics_dialog::open_filters(&mut state);
    let mut no_filters =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::graphics_dialog::show(ui.ctx(), state));
    let mut state = sample_state();
    crate::graphics_dialog::open_templates(&mut state);
    let mut no_templates =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::graphics_dialog::show(ui.ctx(), state));
    // The other dialogs with lists and tabs.
    let (mut state, plan) = graphics_state();
    crate::graphics_dialog::open(&mut state, crate::graphics_dialog::Target::View(plan));
    let mut graphics =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::graphics_dialog::show(ui.ctx(), state));
    let mut state = sample_state();
    crate::phases_dialog::open(&mut state);
    let mut phases =
        themed_harness(state, size, egui::Theme::Light, |ui, state| crate::phases_dialog::show(ui.ctx(), state));
    for harness in [&mut filters, &mut templates, &mut no_filters, &mut no_templates, &mut graphics, &mut phases] {
        // The dialog once grew a little every frame, until it left the screen.
        for _ in 0..40 {
            harness.step();
        }
        let ok = harness.get_by_label(&t!("action-ok")).rect();
        let close = harness.get_by_label(&t!("action-close")).rect();
        assert!(close.top() > 0.0 && ok.bottom() < size.y, "{close:?} {ok:?}");
        assert!(ok.bottom() - close.top() < 900.0, "the dialog grew to {}", ok.bottom() - close.top());
    }
}

#[test]
fn graphics_dialog_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    let (mut state, plan) = graphics_state();
    crate::graphics_dialog::open(&mut state, crate::graphics_dialog::Target::View(plan));
    let mut harness = themed_harness(state, egui::vec2(980.0, 680.0), egui::Theme::Light, |ui, state| {
        crate::graphics_dialog::show(ui.ctx(), state)
    });
    results.add(harness.try_snapshot("visibility_graphics"));
    harness.get_by_label(&t!("vg-filters")).click_accesskit();
    for _ in 0..4 {
        harness.run();
    }
    results.add(harness.try_snapshot("visibility_graphics_filters"));
    let (mut state, _) = graphics_state();
    crate::graphics_dialog::open_filters(&mut state);
    let mut harness = themed_harness(state, egui::vec2(980.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::graphics_dialog::show(ui.ctx(), state)
    });
    results.add(harness.try_snapshot("filters"));
    let (mut state, plan) = graphics_state();
    let create = cc_engine::commands::graphics::TemplateCreate {
        name: "Working plan".into(),
        view: Some(cc_engine::Ref::Id(plan.0)),
    };
    state.session.run(create).unwrap();
    crate::graphics_dialog::open_templates(&mut state);
    let mut harness = themed_harness(state, egui::vec2(980.0, 680.0), egui::Theme::Light, |ui, state| {
        crate::graphics_dialog::show(ui.ctx(), state)
    });
    results.add(harness.try_snapshot("view_templates"));
    results.unwrap();
}

fn phases_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1000.0, 640.0), egui::Theme::Light, |ui, state| {
        crate::phases_dialog::show(ui.ctx(), state)
    })
}

#[test]
fn phases_dialog_adds_a_phase_and_cancel_undoes() {
    let mut state = sample_state();
    let depth = state.session.doc.undo_depth();
    let phases = state.project().phases.len();
    crate::phases_dialog::open(&mut state);
    let mut harness = phases_harness(state);
    harness.get_by_label(&t!("ph-add")).click();
    harness.run();
    assert_eq!(harness.state().project().phases.len(), phases + 1);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert_eq!(harness.state().project().phases.len(), phases);
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
    assert!(harness.state().dialogs.phases.is_none());
}

#[test]
fn design_options_dialog_adds_a_set_and_cancel_undoes() {
    let mut state = sample_state();
    let depth = state.session.doc.undo_depth();
    assert_eq!(crate::commands::UiCommand::ActiveOption.unavailable(&state), Some(t!("why-no-option-sets")));
    crate::design_options_ui::open(&mut state);
    let mut harness = themed_harness(state, egui::vec2(800.0, 600.0), egui::Theme::Light, |ui, state| {
        crate::design_options_ui::show(ui.ctx(), state)
    });
    harness.get_by_label(&t!("opt-new-set")).click();
    harness.run();
    let sets = &harness.state().project().option_sets;
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].options.len(), 2);
    assert_eq!(crate::commands::UiCommand::ActiveOption.unavailable(harness.state()), None);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().project().option_sets.is_empty());
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
    assert!(harness.state().dialogs.design_options.is_none());
}

/// Demolish acts on a selection at once, also from a 3D view; with nothing
/// demolishable selected it starts the tool.
#[test]
fn demolish_takes_the_selection_in_3d() {
    let mut state = sample_state();
    let view = state
        .project()
        .views
        .values()
        .find(|v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. }))
        .map(|v| v.id)
        .unwrap();
    state.active_view = Some(view);
    assert!(crate::commands::phase_view(&state).is_some());
    let walls: Vec<_> =
        state.project().elements.values().filter(|e| e.category() == cc_core::Category::Wall).map(|e| e.id).collect();
    assert!(!walls.is_empty());
    let depth = state.session.doc.undo_depth();
    state.selection = walls.iter().copied().collect();
    crate::phases_dialog::start_demolish(&mut state);
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    assert!(walls.iter().all(|w| state.project().element(*w).unwrap().phase_demolished.is_some()));
    assert!(state.selection.is_empty());
    assert_eq!(state.tool, crate::tools::Tool::Select);
    // Demolished walls are not demolishable again: the tool starts instead.
    state.selection = walls.iter().copied().collect();
    assert!(crate::phases_dialog::demolishable(&state, view, walls.iter().copied()).is_empty());
    crate::phases_dialog::start_demolish(&mut state);
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    assert_eq!(state.tool, crate::tools::Tool::Demolish);
}

#[test]
fn phases_dialog_ok_keeps_one_undo_step() {
    let mut state = sample_state();
    let depth = state.session.doc.undo_depth();
    let filters = state.project().phase_filters.len();
    crate::phases_dialog::open(&mut state);
    let mut harness = phases_harness(state);
    harness.get_by_label(&t!("ph-add")).click();
    harness.run();
    harness.get_by_label(&t!("ph-tab-filters")).click();
    harness.run();
    harness.get_by_label(&t!("ph-new-filter")).click();
    harness.run();
    assert_eq!(harness.state().project().phase_filters.len(), filters + 1);
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.phases.is_none());
    assert_eq!(harness.state().project().phase_filters.len(), filters + 1);
    // OK keeps both edits.
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 2);
}

#[test]
fn phases_dialog_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    // The dialog opens on the phases tab.
    for (tab, name) in
        [(None, "phases"), (Some("ph-tab-filters"), "phase_filters"), (Some("ph-tab-graphics"), "phase_graphics")]
    {
        let mut state = sample_state();
        crate::phases_dialog::open(&mut state);
        let mut harness = phases_harness(state);
        if let Some(tab) = tab {
            harness.get_by_label(&crate::i18n::tr(tab)).click();
        }
        for _ in 0..3 {
            harness.run();
        }
        results.add(harness.try_snapshot(name));
    }
    results.unwrap();
}

fn type_dialog_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(900.0, 900.0), egui::Theme::Light, |ui, state| {
        crate::type_dialog::show(ui.ctx(), state)
    })
}

fn pipe_catalog_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1100.0, 900.0), egui::Theme::Light, |ui, state| {
        crate::pipe_catalog_dialog::show(ui.ctx(), state)
    })
}

/// The Pipe Catalog loads a built-in series with the sizes checked as pipe
/// types, in one session: OK keeps them, Cancel takes them back.
#[test]
fn the_pipe_catalog_loads_a_series_and_makes_types() {
    use crate::pipe_catalog_dialog::Selection;
    let code = "BR-PVC-ESG-R";
    let count = |s: &AppState| {
        s.project()
            .types
            .values()
            .filter(|t| s.project().pipe_row(&t.data).is_some_and(|(x, _)| x.code == code))
            .count()
    };
    for keep in [true, false] {
        let mut state = sample_state();
        assert!(state.project().pipe_series_by_code(code).is_none());
        crate::pipe_catalog_dialog::open_on(&mut state, Some(Selection::BuiltIn(code.into())));
        let mut harness = pipe_catalog_harness(state);
        harness.run();
        // The first two sizes.
        for _ in 0..2 {
            let unchecked = harness
                .query_all_by_role(Role::CheckBox)
                .find(|n| n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::False));
            unchecked.unwrap().click();
            harness.run();
        }
        harness.get_by_role_and_label(Role::Button, &t!("pipe-catalog-create-types")).click();
        harness.run();
        assert_eq!(count(harness.state()), 2, "{}", harness.state().message);
        let selected = harness.state().dialogs.pipe_catalog.as_ref().and_then(|d| d.selected.clone());
        assert!(matches!(selected, Some(Selection::Project(_))), "the project's copy is selected: {selected:?}");
        let label = if keep { t!("action-ok") } else { t!("action-cancel") };
        harness.get_by_role_and_label(Role::Button, &label).click();
        harness.run();
        assert!(harness.state().dialogs.pipe_catalog.is_none());
        assert_eq!(count(harness.state()), if keep { 2 } else { 0 });
        assert_eq!(harness.state().project().pipe_series_by_code(code).is_some(), keep);
    }
}

/// A pipe type of a series picks its size from the series' rows and shows
/// the row's diameters read-only.
#[test]
fn a_pipe_types_size_comes_from_its_series() {
    let mut state = sample_state();
    let pipe = state
        .project()
        .types
        .values()
        .find(|t| state.project().pipe_row(&t.data).is_some_and(|(s, r)| s.code == "BR-PVC-SOLD" && r.size == "25 mm"))
        .unwrap()
        .id;
    state.dialogs.type_editor = Some(pipe);
    let mut harness = type_dialog_harness(state);
    harness.run();
    assert!(harness.query_by_label("21,6 mm").is_some() || harness.query_by_label("21.6 mm").is_some(), "di 21.6 mm");
    // Focused first, so the properties scroll to it.
    let size = |h: &Harness<'static, AppState>| {
        h.query_all_by_role(Role::ComboBox).any(|n| n.value().as_deref() == Some("25 mm"))
    };
    harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some("25 mm")).unwrap().focus();
    harness.run();
    assert!(size(&harness));
    harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some("25 mm")).unwrap().click();
    harness.run();
    harness.get_by_label("32 mm").click();
    harness.run();
    let diameter = harness.state().project().element_type(pipe).and_then(|t| t.diameter());
    assert_eq!(diameter, Some(0.032));
}

/// Duplicate a window type, type a name and press OK: the copy keeps the
/// typed name (before, OK dropped a rename that Enter had not confirmed).
#[test]
fn duplicated_type_keeps_the_typed_name() {
    let mut state = sample_state();
    let window = state.project().default_type(cc_core::Category::Window).unwrap();
    state.dialogs.type_editor = Some(window);
    let mut harness = type_dialog_harness(state);
    harness.get_by_label(&t!("type-duplicate")).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text("Janela 2.5 x 1.8");
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    let names: Vec<_> = harness.state().project().types_of(cc_core::Category::Window).map(|t| t.name.clone()).collect();
    assert!(names.iter().any(|n| n == "Janela 2.5 x 1.8"), "{names:?}");
    assert!(harness.state().dialogs.type_editor.is_none());
}

/// A text type's size is set in Type Properties (#450): duplicate the
/// 2.5 mm type, name the copy and give it 7 mm, for titles.
#[test]
fn a_duplicated_text_type_takes_its_own_size() {
    let mut state = sample_state();
    let text = state.project().default_type(cc_core::Category::Text).unwrap();
    let size = |s: &AppState, id: cc_core::ElementTypeId| match s.project().element_type(id).map(|t| &t.data) {
        Some(cc_core::TypeData::Text { size, .. }) => *size,
        other => panic!("not a text type: {other:?}"),
    };
    assert!((size(&state, text) - 2.5).abs() < 1e-9);
    state.dialogs.type_editor = Some(text);
    let mut harness = type_dialog_harness(state);
    harness.get_by_label(&t!("type-duplicate")).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text("Title 7 mm");
    harness.key_press(egui::Key::Enter);
    harness.run();
    // The copy's text size, in paper millimeters.
    harness.query_all_by_role(Role::SpinButton).find(|n| n.value().as_deref() == Some("2.5 mm")).unwrap().focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text("7");
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    let s = harness.state();
    let copy = s.project().types_of(cc_core::Category::Text).find(|t| t.name == "Title 7 mm").map(|t| t.id);
    let copy = copy.expect("the duplicated type");
    assert!((size(s, copy) - 7.0).abs() < 1e-9, "{}", size(s, copy));
    assert!((size(s, text) - 2.5).abs() < 1e-9, "the original keeps its size");
}

/// A wall type's layers table gives each layer a base offset and a height
/// (issue #306): typing a base offset sets that layer's own extent, and the
/// layers given no height read "To top".
#[test]
fn a_wall_layers_base_offset_is_set_in_type_properties() {
    let mut state = sample_state();
    let wall = state.project().default_type(cc_core::Category::Wall).unwrap();
    let layers = |s: &AppState| s.project().element_type(wall).map(|t| t.layers().to_vec()).unwrap_or_default();
    let unit = state.project().units.length.unit.symbol().to_string();
    let thickness = format!("{} {unit}", length_text(layers(&state)[0].thickness));
    state.dialogs.type_editor = Some(wall);
    state.dialogs.type_layers_open = true;
    let mut harness = type_dialog_harness(state);
    harness.run();
    assert!(harness.query_by_label(&t!("prop-base-offset")).is_some(), "the column's heading");
    let count = layers(harness.state()).len();
    assert_eq!(harness.query_all_by_label(&t!("layer-to-top")).count(), count, "every layer reaches the top");
    // The first layer's base offset: the number field after its thickness.
    let values: Vec<String> =
        harness.query_all_by_role(Role::SpinButton).map(|n| n.value().unwrap_or_default()).collect();
    let at = values.iter().position(|v| *v == thickness).unwrap_or_else(|| panic!("{thickness} in {values:?}")) + 1;
    harness.query_all_by_role(Role::SpinButton).nth(at).unwrap().focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text(&length_text(1.0));
    harness.key_press(egui::Key::Enter);
    harness.run();
    let after = layers(harness.state());
    assert!((after[0].base_offset - 1.0).abs() < 1e-9, "{:?}", after[0]);
    assert!(after[0].height.is_none() && after[1..].iter().all(|l| l.base_offset == 0.0 && l.height.is_none()));
}

/// *Edit Component* in Type Properties closes the dialog as it opens the
/// environment (before, the dialog stayed in front of it).
#[test]
fn edit_component_closes_type_properties() {
    let mut state = sample_state();
    let table = state
        .project()
        .types_of(cc_core::Category::Component)
        .find(|t| matches!(&t.data, cc_core::TypeData::Component { component, .. } if component == "Mesa"))
        .map(|t| t.id)
        .unwrap();
    state.dialogs.type_editor = Some(table);
    let mut harness = type_dialog_harness(state);
    harness.get_by_label(&t!("action-edit-component")).click();
    harness.run();
    assert!(harness.state().component_env.is_some());
    assert!(harness.state().dialogs.type_editor.is_none());
}

/// Typing a name and moving on to another field confirms the name.
#[test]
fn type_name_is_kept_when_another_field_is_clicked() {
    let mut state = sample_state();
    let window = state.project().default_type(cc_core::Category::Window).unwrap();
    state.dialogs.type_editor = Some(window);
    let mut harness = type_dialog_harness(state);
    harness.get_by_label(&t!("type-rename")).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text("Janela alta");
    harness.run();
    harness.query_all_by_role(Role::TextInput).find(|n| !n.is_focused()).unwrap().click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().project().element_type(window).unwrap().name, "Janela alta");
    assert!(harness.state().dialogs.type_rename.is_none());
}

/// The component library lists a folder's component files and loads the
/// one picked into the project.
#[test]
fn component_library_loads_a_component() {
    let mut state = sample_state();
    let dir = std::env::temp_dir().join(format!("ccad-library-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let door = state
        .project()
        .component_types()
        .find(|t| t.category() == cc_core::Category::Door)
        .and_then(|t| match &t.data {
            cc_core::TypeData::Component { component, .. } => Some(component.clone()),
            _ => None,
        })
        .unwrap();
    let path = dir.join("porta.ccomp");
    state.session.run(cc_engine::commands::io::ComponentSave { component: door.clone(), path: path.clone() }).unwrap();
    state.prefs.library_folders = vec![dir];
    crate::library::open(&mut state);
    assert_eq!(state.dialogs.library.as_ref().unwrap().entries.len(), 1);
    let before = state.project().types.len();
    let mut harness = themed_harness(state, egui::vec2(1000.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::library::show(ui.ctx(), state)
    });
    harness.get_by_label_contains(&door).click();
    harness.run();
    harness.get_by_label(&t!("lib-load")).click();
    harness.run();
    crate::jobs::wait(harness.state_mut());
    assert!(harness.state().dialogs.library.is_none());
    assert!(harness.state().project().types.len() > before, "the door's types joined the project");
    assert!(matches!(harness.state().tool, crate::tools::Tool::Hosted { .. }), "doors are placed in walls");
}

/// A catalog entry of the online library, as `crafthouse::parse_catalog` gives it.
fn online_entry(file: &str, component: &str) -> crate::library::Entry {
    crate::library::Entry {
        path: file.into(),
        component: component.into(),
        kind: "Door".into(),
        author: "Ana".into(),
        ..Default::default()
    }
}

/// The component library's 3D CraftHouse source lists the online catalog
/// and loads the component picked from its downloaded file.
#[test]
fn component_library_loads_a_component_from_the_online_library() {
    let mut state = sample_state();
    let dir = std::env::temp_dir().join(format!("ccad-crafthouse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let door = state
        .project()
        .component_types()
        .find(|t| t.category() == cc_core::Category::Door)
        .and_then(|t| match &t.data {
            cc_core::TypeData::Component { component, .. } => Some(component.clone()),
            _ => None,
        })
        .unwrap();
    let path = dir.join("porta.ccomp");
    state.session.run(cc_engine::commands::io::ComponentSave { component: door, path: path.clone() }).unwrap();
    let file = std::path::Path::new("components/doors/porta-online.ccomp");
    state.crafthouse.set_catalog(vec![online_entry("components/doors/porta-online.ccomp", "Porta online")]);
    state.crafthouse.set_downloaded(file, path);
    crate::library::open(&mut state);
    let before = state.project().types.len();
    let mut harness = themed_harness(state, egui::vec2(1000.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::library::show(ui.ctx(), state)
    });
    assert!(harness.query_by_label_contains("Porta online").is_none(), "the folders are shown first");
    harness.get_by_label(&t!("lib-source-online")).click();
    harness.run();
    let lib = harness.state().dialogs.library.as_ref().unwrap();
    assert_eq!((lib.source, lib.entries.len()), (crate::library::Source::Online, 1));
    harness.get_by_label(&t!("lib-count", "count" => 1));
    harness.get_by_label_contains("Porta online").click();
    harness.run();
    harness.get_by_label(&t!("lib-load")).click();
    harness.run();
    crate::jobs::wait(harness.state_mut());
    assert!(harness.state().dialogs.library.is_none());
    assert!(harness.state().project().types.len() > before, "the door's types joined the project");
}

/// A component of the online library that cannot be downloaded (tests are
/// offline) says so and is not loaded.
#[test]
fn an_online_component_that_does_not_download_says_why() {
    let mut state = sample_state();
    let file = std::path::Path::new("components/doors/porta-online.ccomp");
    state.crafthouse.set_catalog(vec![online_entry("components/doors/porta-online.ccomp", "Porta online")]);
    crate::library::open(&mut state);
    let before = state.project().types.len();
    let mut harness = themed_harness(state, egui::vec2(1000.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::library::show(ui.ctx(), state)
    });
    harness.get_by_label(&t!("lib-source-online")).click();
    harness.run();
    harness.get_by_label_contains("Porta online").click();
    harness.run();
    let failed = crate::crafthouse::FileStatus::Failed("offline".into());
    for _ in 0..200 {
        if harness.state().crafthouse.file_status(file) == failed {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        harness.run();
    }
    assert_eq!(harness.state().crafthouse.file_status(file), failed);
    harness.run();
    harness.get_by_label(&t!("lib-online-download-failed", "message" => "offline"));
    harness.get_by_label(&t!("lib-load")).click();
    harness.run();
    assert!(harness.state().dialogs.library.is_some(), "nothing to load: the window stays");
    assert_eq!(harness.state().project().types.len(), before);
}

/// A card of the online library shows the picture its catalog entry gives,
/// and its component file is not downloaded until the card is picked.
#[test]
fn an_online_card_shows_its_catalog_picture_without_downloading_the_component() {
    let mut state = sample_state();
    let dir = std::env::temp_dir().join(format!("ccad-crafthouse-picture-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("porta.png");
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 36, 24);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&[200; 36 * 24 * 4]).unwrap();
    std::fs::write(&png, bytes).unwrap();
    let file = std::path::Path::new("components/doors/porta-online.ccomp");
    let picture = std::path::Path::new("thumbnails/doors/porta-online.png");
    let mut entry = online_entry("components/doors/porta-online.ccomp", "Porta online");
    entry.thumbnail = Some(picture.into());
    state.crafthouse.set_catalog(vec![entry]);
    state.crafthouse.set_picture(picture, png.clone());
    crate::library::open(&mut state);
    let mut harness = library_harness(state, egui::vec2(1200.0, 800.0), egui::Theme::Light);
    harness.get_by_label(&t!("lib-source-online")).click();
    let failed = crate::crafthouse::FileStatus::Failed("offline".into());
    // Offline, a download started would fail at once.
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        harness.run_steps(2);
        assert_ne!(harness.state().crafthouse.file_status(file), failed, "the card in sight downloads nothing");
    }
    assert!(harness.state().thumbnail_jobs.is_empty(), "nothing to render: the picture is there");
    assert_eq!(harness.state().crafthouse.picture(picture), Some(&png));
    // Picked, the component is downloaded (and fails offline).
    harness.get_by_label_contains("Porta online").click();
    harness.run_steps(2);
    for _ in 0..200 {
        if harness.state().crafthouse.file_status(file) == failed {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        harness.run_steps(2);
    }
    assert_eq!(harness.state().crafthouse.file_status(file), failed);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The component library's window over an empty screen.
fn library_harness(state: AppState, size: egui::Vec2, theme: egui::Theme) -> Harness<'static, AppState> {
    themed_harness(state, size, theme, |ui, state| crate::library::show(ui.ctx(), state))
}

/// The component of the sample house's first type of `category`, saved to `path`.
fn save_sample_component(state: &mut AppState, category: cc_core::Category, path: &std::path::Path) -> String {
    let component = state
        .project()
        .component_types()
        .find(|t| t.category() == category)
        .and_then(|t| match &t.data {
            cc_core::TypeData::Component { component, .. } => Some(component.clone()),
            _ => None,
        })
        .unwrap();
    let save = cc_engine::commands::io::ComponentSave { component: component.clone(), path: path.to_path_buf() };
    state.session.run(save).unwrap();
    component
}

/// A kind picked in the filters leaves only its components, and the count
/// says how many of all they are; Clear Filters, offered when nothing is
/// left, brings them all back.
#[test]
fn library_filters_by_kind_and_clears_the_filters() {
    let mut state = sample_state();
    let mut window = online_entry("components/windows/basculante.ccomp", "Basculante de alumínio");
    window.kind = "Window".into();
    state.crafthouse.set_catalog(vec![
        online_entry("components/doors/porta-a.ccomp", "Porta de abrir"),
        online_entry("components/doors/porta-b.ccomp", "Porta de correr"),
        window,
    ]);
    crate::library::open(&mut state);
    // The cards' downloads keep asking for frames, so the harness steps a few at a time.
    let mut harness = library_harness(state, egui::vec2(1200.0, 800.0), egui::Theme::Light);
    harness.get_by_label(&t!("lib-source-online")).click();
    harness.run_steps(4);
    harness.get_by_label(&t!("lib-count", "count" => 3));
    harness.get_by_label(&crate::i18n::tr(cc_core::ComponentKind::Window.key())).click();
    harness.run_steps(4);
    assert_eq!(harness.state().dialogs.library.as_ref().unwrap().kind.as_deref(), Some("Window"));
    harness.get_by_label(&t!("lib-count-of", "shown" => 1, "total" => 3));
    assert!(harness.query_by_label_contains("Porta de abrir").is_none());
    assert!(harness.query_by_label_contains("Basculante de alumínio").is_some());
    // A search that finds none of what is left.
    harness.state_mut().dialogs.library.as_mut().unwrap().search = "nada assim".into();
    harness.run_steps(4);
    harness.get_by_label(&t!("lib-none"));
    harness.get_by_label(&t!("lib-clear-filters")).click();
    harness.run_steps(4);
    let lib = harness.state().dialogs.library.as_ref().unwrap();
    assert!(lib.kind.is_none() && lib.search.is_empty());
    harness.get_by_label(&t!("lib-count", "count" => 3));
    assert!(harness.query_by_label_contains("Porta de abrir").is_some());
}

/// Double-clicking a card loads its component, as Load into Project does.
#[test]
fn double_clicking_a_library_card_loads_the_component() {
    let mut state = sample_state();
    let dir = std::env::temp_dir().join(format!("ccad-library-double-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let door = save_sample_component(&mut state, cc_core::Category::Door, &dir.join("porta.ccomp"));
    state.prefs.library_folders = vec![dir.clone()];
    crate::library::open(&mut state);
    let before = state.project().types.len();
    let mut harness = library_harness(state, egui::vec2(1200.0, 800.0), egui::Theme::Light);
    // Both clicks in one frame: the harness gives each queued event a frame
    // of its own, a quarter second apart, too slow for a double click.
    let at = harness.get_by_label_contains(&door).rect().center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let events = [egui::Event::PointerMoved(at), button(true), button(false), button(true), button(false)];
    harness.input_mut().events.extend(events);
    harness.step();
    harness.run();
    crate::jobs::wait(harness.state_mut());
    assert!(harness.state().dialogs.library.is_none(), "the window closes");
    assert!(harness.state().project().types.len() > before, "the door's types joined the project");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The online library's body fits in the window while the catalog is read,
/// when it cannot be read, and with a component of long texts and many
/// types picked among many others.
#[test]
fn the_online_library_keeps_its_body_inside() {
    let size = egui::vec2(1200.0, 760.0);
    // The dialog panics in tests when its body is too wide; its footer must stay in the window too.
    // Background work keeps asking for frames, so the harness steps a few at a time.
    let footer_inside = |harness: &Harness<'static, AppState>| {
        let rect = harness.get_by_label(&t!("lib-load")).rect();
        assert!(rect.max.x <= size.x && rect.max.y <= size.y, "{rect:?}");
    };
    // Reading the catalog, then failing to (tests are offline).
    let mut state = sample_state();
    crate::library::open(&mut state);
    let mut harness = library_harness(state, size, egui::Theme::Light);
    harness.get_by_label(&t!("lib-source-online")).click();
    harness.run_steps(4);
    footer_inside(&harness);
    for _ in 0..200 {
        if matches!(harness.state().crafthouse.status(), crate::crafthouse::Status::Failed(_)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        harness.run_steps(4);
    }
    assert!(matches!(harness.state().crafthouse.status(), crate::crafthouse::Status::Failed(_)));
    harness.run_steps(4);
    harness.get_by_label(&t!("lib-try-again"));
    footer_inside(&harness);

    // Read, with a component of long texts picked.
    // First by name, so its card is in sight.
    let long = "Armário de nome muito comprido ".repeat(6);
    let mut entry = online_entry("components/doors/longa.ccomp", &long);
    entry.author = "Autora de nome igualmente comprido ".repeat(5);
    entry.types =
        (0..60).map(|i| (format!("Tipo {i} ").repeat(8), "Uma descrição bastante longa do tipo ".repeat(4))).collect();
    entry.parameters = (0..40).map(|i| format!("Parâmetro{i}")).collect();
    let mut entries = vec![entry];
    entries.extend((0..40).map(|i| {
        let mut e = online_entry(&format!("components/misc/c-{i}.ccomp"), &format!("Componente {i}"));
        e.author = format!("Autor {}", i % 3);
        e.kind = ["Furniture", "Casework", "Lighting"][i % 3].into();
        e
    }));
    let mut state = sample_state();
    state.crafthouse.set_catalog(entries);
    crate::library::open(&mut state);
    let mut harness = library_harness(state, size, egui::Theme::Light);
    harness.get_by_label(&t!("lib-source-online")).click();
    harness.run_steps(4);
    harness.get_by_label_contains(long.trim()).click();
    harness.run_steps(4);
    let lib = harness.state().dialogs.library.as_ref().unwrap();
    assert_eq!(lib.selected.as_deref(), Some(std::path::Path::new("components/doors/longa.ccomp")));
    footer_inside(&harness);
}

/// The library window in My Folders with a few components and in 3D
/// CraftHouse with a catalog, a component picked, in both themes. The
/// components are not rendered here, so the cards show placeholders.
#[test]
fn library_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    let dir = std::env::temp_dir().join("ccad-library-snapshots");
    let _ = std::fs::remove_dir_all(&dir);
    let folders = [dir.join("Mobiliario"), dir.join("Esquadrias")];
    let mut state = sample_state();
    let components =
        [(0, "Cama"), (0, "Mesa"), (0, "Cadeira"), (0, "Armário"), (0, "Bacia sanitária"), (1, "Porta de giro")];
    let components = components.into_iter().chain([(1, "Janela de correr"), (1, "Janela maxim-ar")]);
    for (i, (folder, component)) in components.enumerate() {
        std::fs::create_dir_all(&folders[folder]).unwrap();
        let path = folders[folder].join(format!("component-{i}.ccomp"));
        state.session.run(cc_engine::commands::io::ComponentSave { component: component.into(), path }).unwrap();
    }
    let door = folders[1].join("component-5.ccomp");
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut state = sample_state();
        state.prefs.library_folders = folders.to_vec();
        crate::library::open(&mut state);
        let mut harness = library_harness(state, egui::vec2(1240.0, 800.0), theme);
        harness.get_by_label_contains("Porta de giro").click();
        harness.run();
        harness.remove_cursor();
        harness.run();
        // The file's path differs from one machine to another.
        let path = harness.get_by_label_contains(&dir.display().to_string()).rect();
        harness.mask(path);
        results.add(harness.try_snapshot(format!("library_folders_{theme:?}").to_lowercase()));
    }
    let catalog = [
        ("Porta de giro", "Door", "Ana", 3),
        ("Porta de correr", "Door", "Ana", 2),
        ("Janela de correr", "Window", "Bruno", 4),
        ("Janela basculante", "Window", "Bruno", 1),
        ("Cama de casal", "Furniture", "Carla", 2),
        ("Mesa de jantar", "Furniture", "Carla", 3),
        ("Cadeira", "Furniture", "Ana", 0),
        ("Armário aéreo", "Casework", "Bruno", 5),
        ("Bancada de cozinha", "Casework", "Carla", 2),
        ("Bacia sanitária", "Plumbing", "Ana", 1),
        ("Pendente", "Lighting", "Bruno", 2),
        ("Tomada dupla", "Electrical", "Carla", 1),
    ];
    let catalog: Vec<crate::library::Entry> = catalog
        .into_iter()
        .enumerate()
        .map(|(i, (name, kind, author, types))| crate::library::Entry {
            path: format!("components/items/c-{i}.ccomp").into(),
            component: name.into(),
            kind: kind.into(),
            author: author.into(),
            types: (1..=types).map(|t| (format!("{name} {t}"), format!("Variante {t}"))).collect(),
            parameters: vec!["Largura".into(), "Altura".into()],
            ..Default::default()
        })
        .collect();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut state = sample_state();
        state.crafthouse.set_catalog(catalog.clone());
        for e in &catalog {
            state.crafthouse.set_downloaded(&e.path, door.clone());
        }
        crate::library::open(&mut state);
        let mut harness = library_harness(state, egui::vec2(1240.0, 800.0), theme);
        harness.get_by_label(&t!("lib-source-online")).click();
        harness.run();
        harness.get_by_label_contains("Porta de giro").click();
        harness.run();
        harness.remove_cursor();
        harness.run();
        results.add(harness.try_snapshot(format!("library_online_{theme:?}").to_lowercase()));
    }
    let _ = std::fs::remove_dir_all(&dir);
    results.unwrap();
}

/// The ribbon, the app's shortcuts and the dialogs (Preferences among them).
fn app_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1200.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::ribbon::ribbon(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    })
}

/// Preferences opens from the File tab and with Ctrl+,.
#[test]
fn preferences_open_from_the_file_tab_and_the_shortcut() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::File;
    let mut harness = app_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("action-preferences")).click();
    harness.run();
    assert!(harness.state().dialogs.preferences.is_some(), "the File tab button opens it");
    assert!(harness.query_by_label(&t!("pref-file-locations")).is_some(), "its sections are listed");
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.preferences.is_none());
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Comma);
    harness.run();
    assert!(harness.state().dialogs.preferences.is_some(), "Ctrl+, opens it");
}

/// Changes show at once; Cancel brings back the values the window opened
/// with, and OK keeps them.
#[test]
fn preferences_cancel_restores_and_ok_keeps() {
    let mut state = sample_state();
    crate::preferences::open(&mut state, crate::preferences::Section::Interface);
    let mut harness = app_harness(state);
    let tooltips = t!("pref-detailed-tooltips");
    let ribbon = t!("pref-ribbon-collapsed");
    // The switches are down the page, which scrolls.
    for label in [&tooltips, &ribbon] {
        harness.get_by_role_and_label(Role::CheckBox, label).scroll_to_me();
        harness.run();
        harness.get_by_role_and_label(Role::CheckBox, label).click();
        harness.run();
    }
    assert!(!harness.state().prefs.detailed_tooltips);
    assert!(harness.state().prefs.ribbon_collapsed);
    assert!(harness.state().ribbon_collapsed, "the ribbon collapses at once");
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.preferences.is_none());
    assert!(harness.state().prefs.detailed_tooltips, "Cancel restores");
    assert!(!harness.state().prefs.ribbon_collapsed && !harness.state().ribbon_collapsed);
    // Again, with OK.
    crate::preferences::open(harness.state_mut(), crate::preferences::Section::Interface);
    harness.run();
    harness.get_by_role_and_label(Role::CheckBox, &tooltips).scroll_to_me();
    harness.run();
    harness.get_by_role_and_label(Role::CheckBox, &tooltips).click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.preferences.is_none());
    assert!(!harness.state().prefs.detailed_tooltips, "OK keeps");
}

/// Project Information adds a project field; Cancel undoes it, OK keeps it.
#[test]
fn project_fields_are_kept_by_ok_and_undone_by_cancel() {
    for (button, kept) in [(t!("action-cancel"), false), (t!("action-ok"), true)] {
        let mut state = sample_state();
        state.dialogs.project_info = true;
        let mut harness = app_harness(state);
        harness.get_by_label(&format!("+ {}", t!("info-add-field"))).click();
        harness.run();
        assert_eq!(harness.state().project().info.fields.len(), 1);
        harness.get_by_label(&button).click();
        harness.run();
        assert!(!harness.state().dialogs.project_info);
        assert_eq!(harness.state().project().info.fields.len(), usize::from(kept), "{button}");
    }
}

/// The library window's Manage Folders opens Preferences on the file
/// locations, where the folder list now lives.
#[test]
fn library_manages_folders_in_preferences() {
    let mut state = sample_state();
    state.prefs.library_folders = vec![std::path::PathBuf::from("/no/such/library/ccad")];
    crate::library::open(&mut state);
    let mut harness = app_harness(state);
    harness.get_by_label(&t!("lib-manage-folders")).click();
    harness.run();
    let dialog = harness.state().dialogs.preferences.as_ref().map(|d| d.section);
    assert_eq!(dialog, Some(crate::preferences::Section::FileLocations));
    let remove = format!("{} {}", t!("action-remove"), std::path::Path::new("/no/such/library/ccad").display());
    harness.get_by_label(&remove).click();
    harness.run();
    assert!(harness.state().prefs.library_folders.is_empty());
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.library.is_some(), "the library stays open");
    assert!(harness.state().prefs.library_folders.is_empty());
}

/// Shortcuts: typing two letters gives a command new keys, and Default
/// brings the old ones back.
#[test]
fn preferences_change_shortcuts() {
    let mut state = sample_state();
    crate::preferences::open(&mut state, crate::preferences::Section::Shortcuts);
    let mut harness = app_harness(state);
    let label = crate::commands::UiCommand::Demolish.label(harness.state());
    harness.run();
    // The search field has the focus; one command is left.
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text(&label);
    harness.run();
    harness.query_all_by_role(Role::TextInput).find(|n| n.value().as_deref() == Some("DE")).unwrap().click();
    harness.run();
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text("QZ");
    harness.run();
    assert_eq!(harness.state().prefs.shortcut_keys.get("Demolish").map(String::as_str), Some("QZ"));
    harness.get_by_label(&format!("{} {label}", t!("pref-shortcut-reset"))).click();
    harness.run();
    assert!(harness.state().prefs.shortcut_keys.is_empty(), "Default restores the keys");
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    crate::commands::set_key_overrides(&Default::default());
}

#[test]
fn preferences_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for section in crate::preferences::Section::ALL {
        let mut state = sample_state();
        state.prefs.library_folders = vec![std::path::PathBuf::from("/no/such/library/ccad")];
        crate::preferences::open(&mut state, section);
        let mut harness = themed_harness(state, egui::vec2(1000.0, 640.0), egui::Theme::Light, |ui, state| {
            crate::preferences::show(ui.ctx(), state)
        });
        harness.run();
        results.add(harness.try_snapshot(format!("preferences_{section:?}").to_lowercase()));
    }
    // The dark theme, on the page with the theme cards.
    let mut state = sample_state();
    crate::preferences::open(&mut state, crate::preferences::Section::Interface);
    let mut harness = themed_harness(state, egui::vec2(1000.0, 640.0), egui::Theme::Dark, |ui, state| {
        crate::preferences::show(ui.ctx(), state)
    });
    harness.run();
    results.add(harness.try_snapshot("preferences_interface_dark"));
    results.unwrap();
}

/// Run Macro is a menu of the macros in the macros folder, plus Browse….
#[test]
fn macro_menu_lists_the_macros_folder() {
    let dir = std::env::temp_dir().join(format!("ccad-macro-menu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("walls.ccmacro"), "").unwrap();
    std::fs::write(dir.join("doors.ccmacro"), "").unwrap();
    std::fs::write(dir.join("notes.txt"), "").unwrap();
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    state.prefs.macro_folder = Some(dir.clone());
    let mut harness = ribbon_harness_sized(state, egui::vec2(1300.0, 400.0));
    harness.get_by_role_and_label(Role::Button, &t!("action-macro-run")).click();
    harness.run();
    assert!(harness.query_by_label("walls").is_some());
    assert!(harness.query_by_label("doors").is_some());
    assert!(harness.query_by_label("notes").is_none(), "only macro files");
    assert!(harness.query_by_label(&t!("pref-browse")).is_some());
    let _ = std::fs::remove_dir_all(&dir);
    // Without a folder: a hint and Browse….
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    let mut harness = ribbon_harness_sized(state, egui::vec2(1300.0, 400.0));
    harness.get_by_role_and_label(Role::Button, &t!("action-macro-run")).click();
    harness.run();
    assert!(harness.query_by_label(&t!("macro-no-folder")).is_some());
    assert!(harness.query_by_label(&t!("pref-browse")).is_some());
}

/// The structure tab's automation, the way its buttons run it, with the
/// panels and every 2D view drawn after each step and the 3D mesh rebuilt
/// (issue #51: a crash while placing columns, beams, footings and slabs).
#[test]
fn structure_automation_redraws_panels_and_views() {
    let mut state = sample_state();
    for (a, b) in [([-1.0, 0.0], [12.0, 0.0]), ([-1.0, 6.0], [12.0, 6.0]), ([0.0, -1.0], [0.0, 8.0])] {
        state.session.execute("grid_create", serde_json::json!({"start": a, "end": b})).unwrap();
    }
    state.session.execute("grid_create", serde_json::json!({"start": [10.0, -1.0], "end": [10.0, 8.0]})).unwrap();
    state.session.execute("view_create", serde_json::json!({"kind": "structural_plan"})).unwrap();
    let mut harness = themed_harness(state, egui::vec2(1400.0, 900.0), egui::Theme::Light, |ui, state| {
        state.sync();
        state.follow_selection();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.0);
                crate::browser::show(ui, state);
                crate::properties::show(ui, state);
            });
            if let Some(view) = state.active_view
                && state.project().view(view).is_some_and(|v| !matches!(v.kind, cc_core::ViewKind::ThreeD { .. }))
            {
                crate::canvas::show(ui, state, view);
            }
        });
    });
    let steps: [fn(&mut AppState); 9] = [
        crate::actions::columns_at_grids,
        crate::actions::beams_on_grids,
        crate::actions::footings_under_columns,
        crate::actions::strip_footings_under_walls,
        |s| drop(s.session.execute("floor_create", serde_json::json!({"from_walls": true}))),
        |s| drop(s.session.execute("foundation_slab_create", serde_json::json!({"from_walls": true}))),
        crate::actions::slab_join_supports,
        crate::actions::number_structure,
        crate::actions::beams_between_columns,
    ];
    let views: Vec<cc_core::ViewId> = harness.state().project().views.keys().copied().collect();
    for round in 0..2 {
        for step in steps {
            step(harness.state_mut());
            for &view in &views {
                harness.state_mut().active_view = Some(view);
                harness.run();
            }
            let state = harness.state_mut();
            let selection = state.selection.clone();
            let tess = cc_render::Tessellation::new(state.session.geometry());
            cc_render::SceneMesh::build_plain(&tess, &selection);
        }
        if round == 0 {
            // Undo all of it, then redo, as Ctrl+Z / Ctrl+Y would.
            while harness.state_mut().session.execute("undo", serde_json::json!({})).is_ok() {
                harness.run();
            }
            while harness.state_mut().session.execute("redo", serde_json::json!({})).is_ok() {
                harness.run();
            }
        }
    }
}

/// Renaming a view in the project browser ends with Enter or a click
/// elsewhere (issue #82: the field kept the focus for good, so the name
/// never changed and no other field took typing).
#[test]
fn renaming_a_view_in_the_browser_ends() {
    let mut state = sample_state();
    let original = state.project().views_of(cc_core::ViewCategory::ThreeD).next().map(|v| v.id).unwrap();
    let out = state.session.execute("view_duplicate", serde_json::json!({"view": original.0})).unwrap();
    let copy = cc_core::ViewId(out["id"].as_u64().unwrap());
    state.rename = Some((copy, String::new()));
    let mut harness = themed_harness(state, egui::vec2(400.0, 900.0), egui::Theme::Light, |ui, state| {
        egui::ScrollArea::vertical().show(ui, |ui| crate::browser::show(ui, state));
    });
    harness.run();
    harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text("Perspectiva");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(harness.state().rename.is_none(), "Enter ends the rename");
    assert_eq!(harness.state().project().view(copy).map(|v| v.name.as_str()), Some("Perspectiva"));

    // A click on another row ends it too and keeps the name.
    harness.state_mut().rename = Some((copy, "Vista externa".into()));
    harness.run();
    let other = harness.state().project().view(original).map(|v| v.name.clone()).unwrap();
    harness.get_by_label(&other).click();
    harness.run();
    harness.run();
    assert!(harness.state().rename.is_none(), "a click elsewhere ends the rename");
    assert_eq!(harness.state().project().view(copy).map(|v| v.name.as_str()), Some("Vista externa"));
}

/// Typed one key per frame, as people type, a name keeps its spaces and
/// can be emptied and typed again, though the command trims the name and
/// refuses an empty one at every key (issue #82: fields went back to the
/// model's value under the cursor).
#[test]
fn names_typed_key_by_key_keep_spaces_and_survive_emptying() {
    let mut state = sample_state();
    state.dialogs.levels = true;
    let (level, old) = state.project().levels.last().map(|l| (l.id, l.name.clone())).unwrap();
    let mut harness = app_harness(state);
    harness.run();
    harness.query_all_by_role(Role::TextInput).find(|n| n.value().as_deref() == Some(old.as_str())).unwrap().click();
    harness.run();
    for _ in 0..old.chars().count() {
        harness.key_press(egui::Key::End);
        harness.key_press(egui::Key::Backspace);
        harness.run();
    }
    for ch in "Piso 2 B".chars() {
        harness.query_all_by_role(Role::TextInput).find(|n| n.is_focused()).unwrap().type_text(&ch.to_string());
        harness.run();
    }
    assert_eq!(harness.state().project().level_name(level), "Piso 2 B");
}

/// A view placed on a sheet shows its own properties when picked on the
/// sheet, so its scale or graphics change there (#90).
#[test]
fn a_view_picked_on_a_sheet_shows_its_properties() {
    let mut state = sample_state();
    let sheet = state
        .project()
        .views
        .values()
        .find_map(|v| match &v.kind {
            cc_core::ViewKind::Sheet { viewports, .. } => viewports.first().map(|_| v.id),
            _ => None,
        })
        .expect("the sample has a sheet with a view");
    state.active_view = Some(sheet);
    state.sheet_selection = Some((sheet, 0));
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("section-position")).is_some(), "the viewport's position");
    assert!(harness.query_by_label(&t!("section-graphics")).is_some(), "and the view's own graphics");
}

/// A dimension between two columns shows *Locked* in its properties (#447):
/// checking it locks the dimension, so moving one column moves the other.
#[test]
fn the_locked_row_locks_a_dimension() {
    let mut state = sample_state();
    let plan = state
        .project()
        .views
        .values()
        .find(|v| matches!(v.kind, cc_core::ViewKind::FloorPlan { .. }))
        .map(|v| v.id)
        .unwrap();
    let mut column = |x: f64| {
        let out = state.session.execute("column_create", serde_json::json!({"position": [x, 60.0]})).unwrap();
        out["id"].as_u64().unwrap()
    };
    let (a, b) = (column(60.0), column(64.0));
    let dim = state
        .session
        .execute(
            "dimension_create",
            serde_json::json!({"view": plan.0, "a": [60.1, 60.0], "b": [63.9, 60.0], "kind": "linear"}),
        )
        .unwrap();
    let dim = dim["id"].as_u64().unwrap();
    state.active_view = Some(plan);
    state.selection = std::collections::BTreeSet::from([cc_core::ElementId(dim)]);
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    harness.get_by_label(&t!("prop-dim-locked")).click();
    harness.run();
    let state = harness.state_mut();
    let locked = match state.project().element(cc_core::ElementId(dim)).map(|e| &e.kind) {
        Some(cc_core::ElementKind::Dimension(d)) => d.locked,
        _ => None,
    };
    assert!(locked.is_some_and(|v| (v - 3.8).abs() < 1e-9), "{locked:?}");
    state.session.execute("elements_move", serde_json::json!({"ids": [a], "delta": [1.0, 0.0]})).unwrap();
    let x = match state.project().element(cc_core::ElementId(b)).map(|e| &e.kind) {
        Some(cc_core::ElementKind::Column(c)) => c.position.x,
        _ => f64::NAN,
    };
    assert!((x - 65.0).abs() < 1e-9, "{x}");
}

/// The sheet's rows run `sheet_set`: Portrait turns the paper, Today sets
/// the issue date, each one undo step.
#[test]
fn sheet_rows_change_the_sheet() {
    let mut state = sample_state();
    let sheet = state
        .project()
        .views
        .values()
        .find(|v| matches!(v.kind, cc_core::ViewKind::Sheet { .. }))
        .map(|v| v.id)
        .expect("the sample has a sheet");
    state.active_view = Some(sheet);
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("action-today")).is_some());
    let depth = harness.state().session.doc.undo_depth();
    harness.get_by_label(&t!("prop-portrait")).click();
    harness.run();
    harness.get_by_label(&t!("action-today")).click();
    harness.run();
    let state = harness.state();
    assert_eq!(state.session.doc.undo_depth(), depth + 2);
    let Some(cc_core::ViewKind::Sheet { portrait, issue_date, .. }) = state.project().view(sheet).map(|v| &v.kind)
    else {
        panic!("not a sheet")
    };
    assert!(*portrait);
    assert_eq!(*issue_date, crate::sheet_props::today_iso());
}

/// A view dragged from the project browser and dropped on a sheet is
/// placed there, where it was dropped, and picked.
#[test]
fn a_view_dragged_from_the_browser_lands_on_the_sheet() {
    let mut state = sample_state();
    let project = state.project();
    let sheet = project
        .views
        .values()
        .find(|v| matches!(v.kind, cc_core::ViewKind::Sheet { .. }))
        .map(|v| v.id)
        .expect("the sample has a sheet");
    // A view on no sheet yet, whose name no other view has (its browser row is found by name).
    let unique = |name: &str| project.views.values().filter(|v| v.name == name).count() == 1;
    let (view, name) = project
        .views
        .values()
        .filter(|v| v.placeable() && crate::actions::sheet_of(project, v.id).is_none())
        .filter(|v| !matches!(v.category(), cc_core::ViewCategory::Legend) && unique(&v.name))
        .map(|v| (v.id, v.name.clone()))
        .next()
        .expect("a view to place");
    state.active_view = Some(sheet);
    let mut harness = themed_harness(state, egui::vec2(1100.0, 1000.0), egui::Theme::Light, |ui, state| {
        let Some(sheet) = state.active_view else { return };
        ui.horizontal_top(|ui| {
            ui.allocate_ui(egui::vec2(320.0, 980.0), |ui| crate::browser::show(ui, state));
            ui.allocate_ui(ui.available_size(), |ui| crate::sheet::show(ui, state, sheet));
        });
    });
    harness.run();
    let start = harness.get_by_label(&name).rect().center();
    let target = egui::pos2(720.0, 500.0);
    let pointer = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, pressed: Option<bool>| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        if let Some(pressed) = pressed {
            h.input_mut().events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    };
    pointer(&mut harness, start, None);
    pointer(&mut harness, start, Some(true));
    for i in 1..=10 {
        pointer(&mut harness, start + (target - start) * (i as f32 / 10.0), None);
    }
    pointer(&mut harness, target, Some(false));
    harness.run();
    let state = harness.state_mut();
    let placed = match state.project().view(sheet).map(|v| &v.kind) {
        Some(cc_core::ViewKind::Sheet { viewports, .. }) => viewports.iter().position(|vp| vp.view == view),
        _ => None,
    };
    let index = placed.expect("the dropped view is on the sheet");
    assert_eq!(state.sheet_selection, Some((sheet, index)), "and picked");
    let paper = state.sheet(sheet).map(|s| s.paper).unwrap();
    let Some(cc_core::ViewKind::Sheet { viewports, .. }) = state.project().view(sheet).map(|v| &v.kind) else {
        panic!("not a sheet")
    };
    let center = viewports[index].center;
    assert!(center.cmpge(glam::DVec2::ZERO).all() && center.cmple(paper).all(), "{center} on {paper}");
}

/// The home screen in both themes, the Open button under the pointer: the
/// sidebar buttons take the theme's colors (they stayed white in the dark
/// theme, their light text unreadable).
#[test]
fn home_snapshots() {
    if !snapshots_requested() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for (theme, name) in [(egui::Theme::Light, "home_light"), (egui::Theme::Dark, "home_dark")] {
        let mut state = sample_state();
        state.show_home = true;
        let mut harness = themed_harness(state, egui::vec2(900.0, 500.0), theme, |ui, state| {
            crate::home::show(ui, state);
        });
        harness.run();
        let label = t!("action-open");
        harness.get_by_label(&label).hover();
        harness.run();
        results.add(harness.try_snapshot(name));
    }
    results.unwrap();
}

/// Update Now's dialog: a failed update offers the download; an installer
/// ready to run is kept for when the app closes, "On Exit" or not.
#[test]
fn update_install_dialog_offers_what_is_left_to_do() {
    use crate::update::{Install, Version};
    let mut state = sample_state();
    let version = Version { major: 9, minor: 0, patch: 0 };
    state.updates.install = Some(Install::Failed("checksum".into(), "https://example.com/r".into()));
    let mut harness = app_harness(state);
    harness.run();
    assert!(harness.query_by_label(&t!("update-download")).is_some());
    harness.query_all_by_label(&t!("action-close")).last().unwrap().click();
    harness.run();
    assert!(harness.state().updates.install.is_none());

    let msi = std::path::PathBuf::from("C:/Temp/constructcraft-v9.0.0-x86_64.msi");
    let next = crate::update_install::Next::Installer(msi, "0".repeat(64));
    harness.state_mut().updates.install = Some(Install::Ready(version, next.clone()));
    harness.run();
    harness.get_by_label(&t!("update-install-on-exit")).click();
    harness.run();
    assert!(harness.state().updates.install.is_none());
    assert_eq!(harness.state().updates.on_exit, Some(next), "the installer runs when the app closes");
    assert!(!harness.state().quit_confirmed, "On Exit does not close the app");
}

/// A mass's properties list a box per level it spans with that floor's area;
/// clearing one takes the mass floor away (#31).
#[test]
fn mass_floors_are_chosen_in_the_properties() {
    let mut state = sample_state();
    let out = state
        .session
        .execute("mass_create", serde_json::json!({"outline": [[0, 0], [10, 0], [10, 6], [0, 6]], "height": 5.5}))
        .unwrap();
    let mass = cc_core::ElementId(out["id"].as_u64().unwrap());
    state.selection = [mass].into_iter().collect();
    let ground = state.project().levels[0].name.clone();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("section-mass-floors")).is_some());
    let floors = |h: &Harness<'_, AppState>| match &h.state().project().element(mass).unwrap().kind {
        cc_core::ElementKind::Mass(m) => m.floors.len(),
        _ => 0,
    };
    let before = floors(&harness);
    assert!(before >= 1, "the mass spans the ground level at least");
    harness.get_by_role_and_label(Role::CheckBox, &ground).click();
    harness.run();
    assert_eq!(floors(&harness), before - 1);
}

/// A scripted component shows its instance and shared parameters, and the
/// errors of its script (#170).
#[test]
fn scripted_components_show_their_parameters_and_errors() {
    let mut state = sample_state();
    let source = "param.length { 'width', label = 'Width', default = 1, instance = true }\n\
                  param.choice { 'finish', { {'oak', 'Oak'}, {'pine', 'Pine'} }, label = 'Finish' }\n\
                  function geometry(p) assert(p.width > 0, 'needs a width') return { box { 0, 0, 0, p.width, 1, 1 } } end";
    let def = serde_json::json!({ "items": [{ "name": "Script", "shape": { "Script": { "source": source } } }] });
    let ty =
        state.session.execute("component_create", serde_json::json!({ "name": "Shelf", "def": def })).unwrap()["id"]
            .clone();
    let out = state.session.execute("component_place", serde_json::json!({ "type": ty, "position": [0, 0] })).unwrap();
    let id = out["id"].as_u64().unwrap();
    state.selection = [cc_core::ElementId(id)].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("section-instance-params")).is_some());
    assert!(harness.query_by_label(&t!("section-type-params")).is_some());
    assert!(harness.query_by_label(&t!("section-script-errors")).is_none());
    harness
        .state_mut()
        .session
        .execute("component_params_set", serde_json::json!({ "elements": [id], "name": "width", "value": -1 }))
        .unwrap();
    harness.run();
    assert!(harness.query_by_label(&t!("section-script-errors")).is_some());
}

/// Walls, floors and roofs from masses need a mass selected, and select
/// what they make (#31).
#[test]
fn walls_floors_and_roofs_come_from_selected_masses() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    assert!(UiCommand::MassWalls.unavailable(&state).is_some(), "no mass selected");
    let out = state
        .session
        .execute("mass_create", serde_json::json!({"outline": [[40, 0], [50, 0], [50, 6], [40, 6]], "height": 3.0}))
        .unwrap();
    let mass = cc_core::ElementId(out["id"].as_u64().unwrap());
    state.selection = [mass].into_iter().collect();
    assert_eq!(UiCommand::MassWalls.unavailable(&state), None);
    let ctx = egui::Context::default();
    crate::commands::run(&mut state, &ctx, UiCommand::MassWalls);
    assert_eq!(state.selection.len(), 4, "the four walls are selected");
    assert!(
        state
            .selection
            .iter()
            .all(|id| state.project().element(*id).is_some_and(|e| e.category() == cc_core::Category::Wall))
    );
    state.selection = [mass].into_iter().collect();
    crate::commands::run(&mut state, &ctx, UiCommand::MassRoof);
    assert!(
        state
            .selection
            .iter()
            .all(|id| state.project().element(*id).is_some_and(|e| e.category() == cc_core::Category::Roof))
    );
}

/// A roof type's dialog has the covering and framing; with tiles and timber
/// framing, a roof's properties show them.
#[test]
fn roof_types_take_tiles_and_framing() {
    let mut state = sample_state();
    let roof_type = state.project().default_type(cc_core::Category::Roof).unwrap();
    state.dialogs.type_editor = Some(roof_type);
    let mut harness = app_harness(state);
    harness.run();
    assert!(harness.query_by_label(&t!("section-roofing")).is_some());

    let mut state = sample_state();
    let params = serde_json::json!({"covering": "CeramicColonial", "framing": {"system": "Timber"}});
    state.session.execute("type_set", serde_json::json!({"type": roof_type.0, "params": params})).unwrap();
    let roof = state.project().elements.values().find(|e| e.type_id == roof_type).map(|e| e.id).unwrap();
    state.selection = [roof].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1800.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("section-roofing")).is_some(), "the roof shows its covering and framing");
}

/// Sketch mode end to end: a mass sketched as a circle, its boundary edited
/// (the circle's radius in the properties' place), Escape and Delete inside
/// the sketch, and Cancel leaving the model as it was.
#[test]
fn sketch_mode_creates_and_edits_outlines() {
    use crate::commands::UiCommand;
    use crate::sketch_mode::{Draw, Target};
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Mass);
    assert!(matches!(state.sketch.as_ref().map(|s| s.target), Some(Target::New { .. })));
    let circle = SketchCurve::Circle { center: glam::DVec2::new(30.0, 30.0), radius: 3.0 };
    state.sketch.as_mut().unwrap().add(vec![circle.clone()]);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_none(), "{}", state.message);
    let mass = *state.selection.iter().next().unwrap();
    let drawn = state.project().element(mass).and_then(|e| e.drawn_sketch()).cloned().unwrap();
    assert_eq!(drawn.curves, vec![circle.clone()]);

    // Edit Boundary opens the curves; a bigger circle draws the outline again.
    assert_eq!(UiCommand::EditBoundary.unavailable(&state), None);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::EditBoundary);
    let s = state.sketch.as_mut().unwrap();
    assert_eq!((s.target, s.draw, s.curves.clone()), (Target::Edit { id: mass }, Draw::Select, vec![circle.clone()]));
    let bigger = SketchCurve::Circle { center: glam::DVec2::new(30.0, 30.0), radius: 4.0 };
    s.set_curve(0, bigger.clone());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    let e = state.project().element(mass).unwrap();
    assert_eq!(e.drawn_sketch().unwrap().curves, vec![bigger.clone()]);

    // Inside a sketch, undo undoes the sketch's own changes; an open sketch does not finish.
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::EditBoundary);
    let s = state.sketch.as_mut().unwrap();
    s.selected.insert(0);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::Delete);
    assert!(state.sketch.as_ref().unwrap().curves.is_empty());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::Undo);
    assert_eq!(state.sketch.as_ref().unwrap().curves, vec![bigger.clone()]);
    state
        .sketch
        .as_mut()
        .unwrap()
        .add(vec![SketchCurve::Line { a: glam::DVec2::new(40.0, 40.0), b: glam::DVec2::new(41.0, 40.0) }]);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_some(), "an open line keeps the sketch open");
    assert_eq!(state.message, t!("sketch-open-end"));
    // In a sketch the modify tools act on its curves: those on the picked
    // curves need some, the others pick curves themselves.
    let s = state.sketch.as_mut().unwrap();
    s.selected.clear();
    assert_eq!(UiCommand::Move.unavailable(&state), Some(t!("msg-select-first")));
    assert_eq!(UiCommand::Offset.unavailable(&state), None);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::TrimCorner);
    assert_eq!(state.sketch.as_ref().unwrap().draw, Draw::TrimCorner);
    assert!(UiCommand::TrimCorner.selected(&state) && !UiCommand::Split.selected(&state));
    state.sketch.as_mut().unwrap().selected.insert(0);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::Rotate);
    assert_eq!(state.sketch.as_ref().unwrap().draw, Draw::Rotate);
    assert!(matches!(state.tool, crate::tools::Tool::Sketch { .. }), "the element tools stay out of the sketch");
    let revision = state.session.doc.revision();
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchCancel);
    assert!(state.sketch.is_none() && state.tool == crate::tools::Tool::Select);
    assert_eq!(state.session.doc.revision(), revision, "cancel changes nothing");
}

/// Issue #179: a slab's sketch takes typed sizes. A rectangle takes its
/// width, then (after Tab) its depth, and Enter places it; a line takes its
/// length; Escape drops what was typed before the picks.
#[test]
fn sketches_take_typed_lengths() {
    use crate::sketch_mode::Draw;
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Floor);
    state.sketch.as_mut().unwrap().draw = Draw::Rect;
    // The app's keys and a canvas of the sketch, 20 points per meter around the origin.
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: glam::DVec2::ZERO, scale: 20.0 };
        let raw = resp.hover_pos().map(|p| xf.to_world(p));
        let snapped = raw.map(|r| crate::tools::Snap::plain(r, false));
        let Some(mut s) = state.sketch.take() else { return };
        crate::sketch_mode::interact(ui, &resp, &painter, &mut s, &state.modify, raw, snapped, &xf);
        state.sketch = Some(s);
    });
    harness.run();
    let center = egui::pos2(300.0, 200.0);
    let pointer = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, click: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.run();
        if click {
            for pressed in [true, false] {
                let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
                h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
                h.run();
            }
        }
    };
    let typed = |h: &mut Harness<'static, AppState>, text: &str| {
        h.input_mut().events.push(egui::Event::Text(text.into()));
        h.run();
    };
    let sketch = |h: &Harness<'static, AppState>| h.state().sketch.clone().unwrap();

    // The first corner at the origin; the cursor up and to the left; 5, Tab, 4, Enter.
    pointer(&mut harness, center, true);
    assert!(sketch(&harness).typing());
    pointer(&mut harness, center + egui::vec2(-30.0, -50.0), false);
    typed(&mut harness, "5");
    // Tab switches the field in the app's raw input hook, which the harness skips.
    harness.state_mut().sketch.as_mut().unwrap().typed.angle_active = true;
    typed(&mut harness, "4");
    assert_eq!((sketch(&harness).typed.length.as_str(), sketch(&harness).typed.angle.as_str()), ("5", "4"));
    harness.key_press(egui::Key::Enter);
    harness.run();
    let s = sketch(&harness);
    assert_eq!(s.curves.len(), 4, "Enter placed the rectangle");
    let pts: Vec<glam::DVec2> = s.curves.iter().flat_map(|c| c.points()).collect();
    let (min, max) = pts.iter().fold((pts[0], pts[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    assert!((max - min).distance(glam::DVec2::new(5.0, 4.0)) < 1e-9, "{min} {max}");
    assert!(min.x.abs() > 1.0 && max.x.abs() < 1e-9, "on the cursor's side: {min} {max}");
    assert!(s.typed.is_empty() && s.picks.is_empty());

    // A line: 3 m towards the cursor.
    harness.state_mut().sketch.as_mut().unwrap().draw = Draw::Line;
    let start = center + egui::vec2(0.0, 100.0);
    pointer(&mut harness, start, true);
    pointer(&mut harness, start + egui::vec2(60.0, 0.0), false);
    typed(&mut harness, "3");
    harness.key_press(egui::Key::Enter);
    harness.run();
    let s = sketch(&harness);
    let Some(SketchCurve::Line { a, b }) = s.curves.last().cloned() else { panic!("{:?}", s.curves) };
    assert!((a.distance(b) - 3.0).abs() < 1e-9 && (b - a).x > 0.0, "{a} {b}");
    assert_eq!(s.picks, vec![b], "the line goes on from its end");
    // Escape drops the typed value first, then the chain.
    typed(&mut harness, "7");
    harness.key_press(egui::Key::Escape);
    harness.run();
    let s = sketch(&harness);
    assert!(s.typed.is_empty() && s.picks == vec![b]);
    // Letters are not lengths.
    typed(&mut harness, "x");
    assert!(sketch(&harness).typed.is_empty());
}

/// Issue #311: *From Walls* follows the face of the walls chosen in the
/// sketch's options bar, the core faces among them.
#[test]
fn from_walls_follows_the_chosen_wall_face() {
    let mut state = sample_state();
    // The plan of a level with walls.
    let level = state.project().walls().map(|(e, _)| e.level).next().unwrap();
    let plan = state.project().views_of(cc_core::ViewCategory::FloorPlan).find(|v| v.level() == Some(level));
    state.active_view = plan.map(|v| v.id);
    assert_eq!(state.active_level(), Some(level));
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    let picked = |state: &mut AppState, face: Option<cc_core::LocationLine>| {
        state.modify.wall_face = face;
        state.sketch.as_mut().unwrap().curves.clear();
        crate::sketch_mode::pick_walls(state);
        let sketch = cc_core::sketch::Sketch { curves: state.sketch.as_ref().unwrap().curves.clone() };
        sketch.profile().expect("a closed outline").outline
    };
    let area = |pts: &[glam::DVec2]| cc_geom::polygon::area(pts);
    // By default the outside finish faces; the core faces are inside the plaster.
    let finish = picked(&mut state, None);
    let expected = cc_engine::walls_loop_at(state.project(), level, cc_core::LocationLine::FinishExterior).unwrap();
    assert!((area(&finish) - area(&expected)).abs() < 1e-9);
    let core = picked(&mut state, Some(cc_core::LocationLine::CoreExterior));
    let expected = cc_engine::walls_loop_at(state.project(), level, cc_core::LocationLine::CoreExterior).unwrap();
    assert!((area(&core) - area(&expected)).abs() < 1e-9);
    assert!(area(&core) < area(&finish) - 0.1, "{} inside {}", area(&core), area(&finish));
    let inside = picked(&mut state, Some(cc_core::LocationLine::CoreInterior));
    assert!(area(&inside) < area(&core) - 0.1);

    // The options bar offers the faces.
    let harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        ui.horizontal(|ui| crate::sketch_mode::options(ui, state));
    });
    assert!(harness.query_all_by_label(&t!("sketch-wall-face")).next().is_some(), "the wall face option");
    assert!(harness.query_all_by_role(Role::ComboBox).next().is_some(), "the list of faces");
}

/// Issue #496: lines *From Walls* added move when the wall face changes,
/// except those changed since; undo brings them back.
#[test]
fn from_walls_lines_follow_a_new_wall_face() {
    use cc_core::LocationLine::{CoreExterior, FinishExterior};
    let mut state = sample_state();
    let level = state.project().walls().map(|(e, _)| e.level).next().unwrap();
    let plan = state.project().views_of(cc_core::ViewCategory::FloorPlan).find(|v| v.level() == Some(level));
    state.active_view = plan.map(|v| v.id);
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    let area = |state: &AppState| {
        let sketch = cc_core::sketch::Sketch { curves: state.sketch.as_ref().unwrap().curves.clone() };
        cc_geom::polygon::area(&sketch.profile().expect("a closed outline").outline)
    };
    let expected = |state: &AppState, face| {
        cc_geom::polygon::area(&cc_engine::walls_loop_at(state.project(), level, face).unwrap())
    };
    crate::sketch_mode::set_wall_face(&mut state, FinishExterior);
    crate::sketch_mode::pick_walls(&mut state);
    assert!((area(&state) - expected(&state, FinishExterior)).abs() < 1e-9);
    // A new face moves the lines: the outline is the core faces' loop now.
    crate::sketch_mode::set_wall_face(&mut state, CoreExterior);
    assert!((area(&state) - expected(&state, CoreExterior)).abs() < 1e-9, "{}", area(&state));
    // And back again.
    crate::sketch_mode::set_wall_face(&mut state, FinishExterior);
    assert!((area(&state) - expected(&state, FinishExterior)).abs() < 1e-9);
    // Undo brings the core faces back, and a new face still moves them.
    state.sketch.as_mut().unwrap().undo();
    assert!((area(&state) - expected(&state, CoreExterior)).abs() < 1e-9);
    crate::sketch_mode::set_wall_face(&mut state, FinishExterior);
    assert!((area(&state) - expected(&state, FinishExterior)).abs() < 1e-9);
    // A line the user moved stays where it was put.
    let s = state.sketch.as_mut().unwrap();
    let cc_core::sketch::SketchCurve::Line { a, .. } = &mut s.curves[0] else { panic!("a line") };
    *a += glam::DVec2::new(0.0, 0.0001);
    let edited = s.curves[0].clone();
    crate::sketch_mode::set_wall_face(&mut state, CoreExterior);
    let s = state.sketch.as_ref().unwrap();
    assert_eq!(s.curves[0], edited);
    assert!(s.curves[1..].iter().all(|c| s.from_walls[0].contains(c)), "the others moved");
}

/// A piece of the component environment is sketched in the same sketch as
/// outlines in plans: its ribbon, options, picked curve and Finish.
#[test]
fn component_pieces_are_sketched_with_the_sketch_tools() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let table = state
        .project()
        .types_of(cc_core::Category::Component)
        .find(|t| matches!(&t.data, cc_core::TypeData::Component { component, .. } if component == "Mesa"))
        .map(|t| t.id)
        .unwrap();
    crate::component_env::open(&mut state, table);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::CompExtrusion);
    assert!(UiCommand::CompExtrusion.selected(&state) && UiCommand::SketchEllipse.unavailable(&state).is_none());
    assert!(UiCommand::SketchPickWalls.unavailable(&state).is_some(), "walls are picked in plans only");
    let s = state.sketch.as_mut().unwrap();
    s.curves = vec![
        cc_core::sketch::SketchCurve::ellipse(glam::DVec2::new(0.0, 0.0), glam::DVec2::new(0.5, 0.3), 15.0),
        cc_core::sketch::SketchCurve::Circle { center: glam::DVec2::new(0.6, 0.0), radius: 0.1 },
    ];
    s.selected.insert(0);
    s.piece.as_mut().unwrap().view = Some(cc_views::ComponentViewKind::Plan);
    let mut harness = themed_harness(state, egui::vec2(1280.0, 820.0), egui::Theme::Light, |ui, state| {
        egui::Panel::top("ribbon").show(ui, |ui| crate::sketch_mode::ribbon(ui, state));
        egui::Panel::top("options").show(ui, |ui| ui.horizontal(|ui| crate::sketch_mode::options(ui, state)));
        egui::Panel::left("left").default_size(290.0).show(ui, |ui| crate::component_env::panel(ui, state));
        egui::CentralPanel::default().show(ui, |ui| crate::component_env::central(ui, state));
    });
    for _ in 0..3 {
        harness.run();
    }
    if let Some(path) = std::env::var_os("CC_RENDER_PIECE_SKETCH") {
        harness.render().unwrap().save(path).unwrap();
    }
    let title = t!("sketch-title-piece", "kind" => t!("comp-env-extrusion"));
    assert!(harness.query_by_label(&title).is_some(), "{title}");
    // The ribbon's Ellipse tool, and the picked ellipse's section in the panel.
    assert_eq!(harness.query_all_by_label(&t!("sketch-ellipse")).count(), 2);
    assert!(!UiCommand::Select.selected(harness.state()), "the sketch's tool, not the environment's");
    assert!(harness.query_by_label(&t!("comp-sketch-void")).is_some(), "the piece's options");
    // Two loops apart make no profile: the sketch stays open with the reason.
    crate::commands::run(harness.state_mut(), &egui::Context::default(), UiCommand::SketchFinish);
    assert!(harness.state().sketch.is_some());
    assert_eq!(harness.state().message, t!("sketch-apart"));
}

/// The component environment's side panel keeps its fields and buttons
/// clear of the panel's left border (they were drawn against it, cut).
#[test]
fn component_panel_is_inset() {
    let mut state = sample_state();
    let table = state
        .project()
        .types_of(cc_core::Category::Component)
        .find(|t| matches!(&t.data, cc_core::TypeData::Component { component, .. } if component == "Mesa"))
        .map(|t| t.id)
        .unwrap();
    crate::component_env::open(&mut state, table);
    if let Some(e) = state.component_env.as_mut() {
        e.pick = Some(crate::component_env::Pick::Piece(0));
    }
    let harness = themed_harness(state, egui::vec2(700.0, 700.0), egui::Theme::Light, |ui, state| {
        // The app's side panel has no margin of its own.
        egui::Panel::left("left").frame(egui::Frame::new()).default_size(290.0).show(ui, |ui| {
            let left = ui.max_rect().left();
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("panel-left"), left));
            crate::component_env::panel(ui, state)
        });
    });
    let panel_left: f32 = harness.ctx.data(|d| d.get_temp(egui::Id::new("panel-left"))).unwrap();
    for label in [format!("+ {}", t!("comp-env-add-param")), t!("action-delete")] {
        let inset = harness.get_by_label(&label).rect().min.x - panel_left;
        assert!(inset >= 6.0, "{label} starts {inset} from the panel's border");
    }
}

/// Issues #47 and #69: several elements of one category show their type,
/// level and shared parameters, a differing value marked; a change goes to
/// all of them in one step.
#[test]
fn several_elements_are_edited_together() {
    let mut state = sample_state();
    let columns: Vec<cc_core::ElementId> =
        state.project().elements.values().filter(|e| e.category() == cc_core::Category::Column).map(|e| e.id).collect();
    assert!(columns.len() >= 2, "the sample has columns");
    // One column lower than the others: the base offset differs.
    crate::engine_edit::element(&mut state, columns[0], None, |e| {
        if let cc_core::ElementKind::Column(c) = &mut e.kind {
            c.base_offset = -0.5;
        }
    });
    state.selection = columns.iter().copied().collect();
    let mut harness = themed_harness(state, egui::vec2(320.0, 700.0), egui::Theme::Light, |ui, state| {
        egui::Panel::left("left").default_size(300.0).show(ui, |ui| crate::properties::show(ui, state));
    });
    harness.run();
    if let Some(path) = std::env::var_os("CC_RENDER_MULTI") {
        harness.render().unwrap().save(path).unwrap();
    }
    assert!(harness.query_by_label(&t!("section-type")).is_some(), "the shared type and level");
    assert!(harness.query_by_label(&t!("section-constraints")).is_some(), "the shared parameters");
    // Down to the footings: every column, one undo step.
    let state = harness.state_mut();
    let depth = state.session.doc.undo_depth();
    let patch = serde_json::json!({"base_offset": -1.5});
    assert!(crate::engine_edit::elements(state, &columns, None, None, Some(patch)));
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    for id in &columns {
        let Some(cc_core::ElementKind::Column(c)) = state.project().element(*id).map(|e| &e.kind) else { panic!() };
        assert_eq!(c.base_offset, -1.5);
    }
}

/// The sweep tool: a project without a sweep type gets a baseboard when the
/// tool starts; the clicked points (closed, flipped, raised by the options)
/// make the element, and its properties show it.
#[test]
fn the_sweep_tool_draws_a_baseboard_around_a_room() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    assert!(state.project().default_type(cc_core::Category::Sweep).is_none());
    assert_eq!(UiCommand::Place(cc_core::Category::Sweep).unavailable(&state), None);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Sweep);
    let crate::tools::Tool::Sweep { type_id } = state.tool.clone() else {
        panic!("not the sweep tool: {:?}", state.tool)
    };
    state.sweep = crate::tools::SweepOptions { base_offset: 0.05, closed: true, flip: true };
    state.interaction.points =
        vec![glam::DVec2::new(60.0, 0.0), glam::DVec2::new(64.0, 0.0), glam::DVec2::new(64.0, 3.0)];
    let level = state.active_level().unwrap();
    crate::tools::finish_sweep(&mut state, level, type_id);
    assert!(state.interaction.points.is_empty());
    let sweep = *state.selection.iter().next().unwrap_or_else(|| panic!("{}", state.message));
    let e = state.project().element(sweep).unwrap();
    let cc_core::ElementKind::Sweep(s) = &e.kind else { panic!("not a sweep") };
    assert!(s.closed && s.flip && (s.base_offset - 0.05).abs() < 1e-12 && s.path.len() == 3);
    assert!(state.session.geom().solids.iter().any(|solid| solid.element == sweep));
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    // The sweep's own rows: its constraints and dimensions, with the two checkboxes (closed, flip).
    for key in ["section-constraints", "section-dimensions"] {
        assert!(harness.query_by_label(&crate::i18n::tr(key)).is_some(), "{key}");
    }
    assert!(harness.query_all_by_role(Role::CheckBox).count() >= 2);
}

/// A view's visual style in its properties: the active view with nothing
/// selected shows a combo box with its style, and a change made through the
/// app is an engine edit (one undo step).
#[test]
fn a_views_visual_style_is_chosen_in_its_properties() {
    let mut state = sample_state();
    let view = state.project().views.values().find(|v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. })).unwrap().id;
    state.active_view = Some(view);
    state.selection.clear();
    assert_eq!(state.project().view(view).unwrap().style(), cc_core::VisualStyle::Shaded);
    crate::engine_edit::view(&mut state, view, None, |v| v.visual_style = Some(cc_core::VisualStyle::Textures));
    assert_eq!(state.project().view(view).unwrap().style(), cc_core::VisualStyle::Textures);
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    let shown = t!("style-textures");
    let combo = harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some(shown.as_str()));
    assert!(combo.is_some(), "the combo shows the style");
    harness.state_mut().session.execute("undo", serde_json::json!({})).unwrap();
    assert_eq!(harness.state().project().view(view).unwrap().style(), cc_core::VisualStyle::Shaded);
}

/// A railing on a stair: the railing tool recognizes stairs and ramps as
/// hosts, and the railing's properties take it off its host.
#[test]
fn a_hosted_railing_is_detached_from_its_properties() {
    let mut state = sample_state();
    let stair = state.session.execute("stair_create", serde_json::json!({"start": [70, 0]})).unwrap();
    let stair = cc_core::ElementId(stair["id"].as_u64().unwrap());
    assert!(crate::tools::railing_host(&state, stair));
    let added =
        state.session.execute("stair_railings_add", serde_json::json!({"id": stair.0, "near": [71.0, 5.0]})).unwrap();
    let railing = cc_core::ElementId(added["ids"][0].as_u64().unwrap());
    assert!(!crate::tools::railing_host(&state, railing));
    state.selection = [railing].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1200.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("action-railing-detach")).click();
    harness.run();
    let hosted = |h: &Harness<'_, AppState>| match &h.state().project().element(railing).unwrap().kind {
        cc_core::ElementKind::Railing(r) => r.stair.is_some(),
        _ => true,
    };
    assert!(!hosted(&harness));
}

/// The shaft tool: a project without a shaft type gets one when the tool
/// starts, the sketch makes a shaft, and the shaft cuts the ceiling it
/// passes through.
#[test]
fn the_shaft_tool_cuts_a_ceiling() {
    use crate::commands::UiCommand;
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    let level = state.active_level().unwrap();
    let outline = serde_json::json!([[80, 0], [86, 0], [86, 6], [80, 6]]);
    let ceiling =
        state.session.execute("ceiling_create", serde_json::json!({"boundary": outline, "level": level.0})).unwrap();
    let ceiling = cc_core::ElementId(ceiling["id"].as_u64().unwrap());
    assert_eq!(UiCommand::Place(cc_core::Category::Shaft).unavailable(&state), None);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Shaft);
    assert!(state.project().default_type(cc_core::Category::Shaft).is_some());
    let circle = SketchCurve::Circle { center: glam::DVec2::new(83.0, 3.0), radius: 0.6 };
    state.sketch.as_mut().unwrap().add(vec![circle]);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_none(), "{}", state.message);
    let shaft = *state.selection.iter().next().unwrap();
    assert!(matches!(state.project().element(shaft).unwrap().kind, cc_core::ElementKind::Shaft(_)));
    assert_eq!(state.project().element(ceiling).unwrap().shaft_cuts().len(), 1);
}

/// The terrain tool: a project without a terrain type gets one; the sketch
/// takes the boundary and the measured points (undone with the curves), and
/// *Finish* makes the terrain through them. The Point tool is for terrain only.
#[test]
fn the_terrain_tool_sketches_a_boundary_and_points() {
    use crate::commands::UiCommand;
    use cc_core::sketch::SketchCurve;
    use glam::{DVec2, DVec3};
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    assert!(UiCommand::SketchPoint.unavailable(&state).is_some(), "floors take no points");
    state.set_tool(crate::tools::Tool::Select);

    assert_eq!(UiCommand::Place(cc_core::Category::Topography).unavailable(&state), None);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Topography);
    assert!(state.project().default_type(cc_core::Category::Topography).is_some());
    assert_eq!(UiCommand::SketchPoint.unavailable(&state), None);
    let p = |x: f64, y: f64| DVec2::new(x, y);
    let s = state.sketch.as_mut().unwrap();
    s.add(vec![
        SketchCurve::Line { a: p(100.0, 0.0), b: p(110.0, 0.0) },
        SketchCurve::Line { a: p(110.0, 0.0), b: p(110.0, 10.0) },
        SketchCurve::Line { a: p(110.0, 10.0), b: p(100.0, 10.0) },
        SketchCurve::Line { a: p(100.0, 10.0), b: p(100.0, 0.0) },
    ]);
    for (x, y, z) in [(100.0, 0.0, 0.0), (110.0, 0.0, 1.0), (110.0, 10.0, 2.0), (100.0, 10.0, 1.0), (105.0, 5.0, 9.0)] {
        s.add_point(DVec3::new(x, y, z));
    }
    // The last point was a mistake.
    s.undo();
    assert_eq!(s.points.len(), 4);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_none(), "{}", state.message);
    let id = *state.selection.iter().next().unwrap();
    let cc_core::ElementKind::Topography(t) = &state.project().element(id).unwrap().kind else {
        panic!("a terrain");
    };
    assert_eq!(t.points.len(), 4);
    let g = state.session.geometry().topographies.get(&id).cloned().unwrap();
    assert_eq!((g.min_z, g.max_z), (0.0, 2.0));
}

/// The building pad tool: unavailable without a terrain on the plan's level;
/// the sketch takes the outline at the options bar's elevation, and
/// *Finish* grades the level's terrain with a new pad type.
#[test]
fn the_pad_tool_grades_the_terrain_of_the_level() {
    use crate::commands::UiCommand;
    use cc_core::sketch::SketchCurve;
    use glam::DVec2;
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    let level = state.active_level().unwrap();
    assert!(UiCommand::Place(cc_core::Category::Pad).unavailable(&state).is_some(), "no terrain yet");
    let points = serde_json::json!([[100, 0, 0], [130, 0, 3], [130, 30, 3], [100, 30, 0], [115, 15, 1.5]]);
    let boundary = serde_json::json!([[100, 0], [130, 0], [130, 30], [100, 30]]);
    let t = state
        .session
        .execute("topography_create", serde_json::json!({"points": points, "boundary": boundary, "level": level.0}))
        .unwrap();
    let terrain = cc_core::ElementId(t["id"].as_u64().unwrap());
    assert_eq!(UiCommand::Place(cc_core::Category::Pad).unavailable(&state), None);
    crate::actions::activate_category_tool(&mut state, cc_core::Category::Pad);
    assert!(state.project().default_type(cc_core::Category::Pad).is_some());
    state.modify.pad_elevation = 1.5;
    let p = |x: f64, y: f64| DVec2::new(x, y);
    state.sketch.as_mut().unwrap().add(vec![
        SketchCurve::Line { a: p(110.0, 10.0), b: p(120.0, 10.0) },
        SketchCurve::Line { a: p(120.0, 10.0), b: p(120.0, 20.0) },
        SketchCurve::Line { a: p(120.0, 20.0), b: p(110.0, 20.0) },
        SketchCurve::Line { a: p(110.0, 20.0), b: p(110.0, 10.0) },
    ]);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_none(), "{}", state.message);
    let id = *state.selection.iter().next().unwrap();
    let cc_core::ElementKind::Pad(pad) = &state.project().element(id).unwrap().kind else { panic!("a pad") };
    assert_eq!((pad.host, pad.elevation), (terrain, 1.5));
    let g = state.session.geometry().pads.get(&id).cloned().unwrap();
    assert!(g.cut > 0.0 && g.fill > 0.0, "{g:?}");
    // A click on the graded earth inside it picks the pad.
    assert_eq!(cc_geom::pad::pad_at(state.project(), terrain, p(115.0, 15.0)), Some(id));
    assert_eq!(cc_geom::pad::pad_at(state.project(), terrain, p(105.0, 5.0)), None);
}

/// Attach Top and Detach Top: available with walls selected; the tool
/// attaches the selected walls' tops to the roof clicked, and Detach Top
/// gives them back their own tops.
#[test]
fn walls_attach_their_tops_to_a_roof_and_detach_them() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let wall = state
        .session
        .execute("wall_create", serde_json::json!({"start": [90, 0], "end": [96, 0], "height": 3}))
        .unwrap();
    let wall = cc_core::ElementId(wall["id"].as_u64().unwrap());
    let outline = serde_json::json!([[89, -1], [97, -1], [97, 5], [89, 5]]);
    let roof = state.session.execute("roof_create", serde_json::json!({"boundary": outline, "offset": 3.5})).unwrap();
    let roof = cc_core::ElementId(roof["id"].as_u64().unwrap());
    state.selection.clear();
    assert!(UiCommand::AttachTop.unavailable(&state).is_some(), "no wall selected");
    state.selection = [wall].into_iter().collect();
    assert_eq!(UiCommand::AttachTop.unavailable(&state), None);
    assert!(UiCommand::DetachTop.unavailable(&state).is_some(), "not attached yet");
    let ctx = egui::Context::default();
    crate::commands::run(&mut state, &ctx, UiCommand::AttachTop);
    assert_eq!(state.tool, crate::tools::Tool::AttachTop);
    assert!(crate::modify::attach_target(&state, roof) && !crate::modify::attach_target(&state, wall));
    crate::modify::attach_to(&mut state, roof);
    let attached = |state: &AppState| match &state.project().element(wall).unwrap().kind {
        cc_core::ElementKind::Wall(w) => w.attach_top,
        _ => None,
    };
    assert_eq!(attached(&state), Some(roof));
    assert_eq!(state.tool, crate::tools::Tool::Select);
    assert_eq!(UiCommand::DetachTop.unavailable(&state), None);
    crate::commands::run(&mut state, &ctx, UiCommand::DetachTop);
    assert_eq!(attached(&state), None);

    // A click in a 3D view: on the wall itself the tool waits, on the roof it attaches.
    crate::commands::run(&mut state, &ctx, UiCommand::AttachTop);
    crate::modify::attach_pick(&mut state, Some(wall));
    crate::modify::attach_pick(&mut state, None);
    assert_eq!((attached(&state), &state.tool), (None, &crate::tools::Tool::AttachTop));
    crate::modify::attach_pick(&mut state, Some(roof));
    assert_eq!(attached(&state), Some(roof));
    assert_eq!(state.tool, crate::tools::Tool::Select);
}

/// Several walls edited together: location line, base and top constraints,
/// joins and identity data, each set on all of them in one undo step.
#[test]
fn several_walls_share_constraints_and_identity_data() {
    use cc_core::{ElementKind, WallTop};
    let mut state = sample_state();
    let walls: Vec<_> =
        state.project().elements.values().filter(|e| matches!(e.kind, ElementKind::Wall(_))).map(|e| e.id).collect();
    // One wall unconnected, the others up to the level above: their tops differ.
    let above = state.project().levels[1].id.0;
    let ids: Vec<u64> = walls.iter().map(|id| id.0).collect();
    let top = serde_json::json!({ "top": { "Level": { "level": above, "offset": 0.0 } } });
    state
        .session
        .run(cc_engine::commands::edit::ElementsSet {
            ids: ids[1..].to_vec(),
            level: None,
            params: Some(top),
            mark: None,
            comments: None,
        })
        .unwrap();
    let free = serde_json::json!({ "top": "Unconnected", "height": 2.5 });
    state
        .session
        .run(cc_engine::commands::edit::ElementsSet {
            ids: ids[..1].to_vec(),
            level: None,
            params: Some(free),
            mark: None,
            comments: None,
        })
        .unwrap();
    let heights: Vec<f64> = walls
        .iter()
        .map(|id| {
            let e = state.project().element(*id).unwrap();
            let ElementKind::Wall(w) = &e.kind else { panic!("wall") };
            state.project().wall_height(e, w)
        })
        .collect();
    state.selection = walls.iter().copied().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 900.0), egui::Theme::Light, |ui, state| {
        egui::ScrollArea::vertical().show(ui, |ui| crate::properties::show(ui, state));
    });
    // The constraints and identity sections; the location line all walls share.
    for section in [t!("section-constraints"), t!("section-identity")] {
        assert!(harness.query_by_role_and_label(Role::Button, &section).is_some(), "{section}");
    }
    let centerline = crate::i18n::tr(cc_core::LocationLine::Centerline.key());
    assert!(harness.query_by_value(&centerline).is_some());
    let wall = |h: &Harness<'_, AppState>, id| match h.state().project().element(id).map(|e| e.kind.clone()) {
        Some(ElementKind::Wall(w)) => w,
        _ => panic!("wall"),
    };

    // Top constraint: unconnected, each wall keeping its height.
    let depth = harness.state().session.doc.undo_depth();
    // The walls differ (up to different levels): the box says so.
    harness.get_by_value(&t!("props-varies")).click();
    harness.run();
    harness.get_by_label(&t!("top-unconnected")).click();
    harness.run();
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1, "one undo step");
    for (id, h) in walls.iter().zip(&heights) {
        let w = wall(&harness, *id);
        assert_eq!(w.top, WallTop::Unconnected);
        assert!((w.height - h).abs() < 1e-9, "{} vs {h}", w.height);
    }

    // Comments typed once go to every wall.
    let inputs = harness.get_all_by_role(Role::TextInput).count();
    harness.get_all_by_role(Role::TextInput).nth(inputs - 1).unwrap().click();
    harness.run();
    harness.get_all_by_role(Role::TextInput).nth(inputs - 1).unwrap().type_text("Existente");
    harness.run();
    for id in &walls {
        assert_eq!(harness.state().project().element(*id).unwrap().comments, "Existente");
    }
}

/// A plugins folder with one plugin, `room_tools`, whose command creates a wall.
fn plugins_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-plugins-ui-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("room_tools");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plugin.toml"), "id = \"room_tools\"\nname = \"Room tools\"\nversion = \"0.3.0\"\n")
        .unwrap();
    std::fs::write(
        dir.join("main.lua"),
        "ccad.command { name = 'one_wall', description = 'A wall', \
         run = function() print('made a wall') return ccad.run('wall_create', { start = { 0, 0 }, ['end'] = { 2, 0 } }).id end }",
    )
    .unwrap();
    root
}

/// Plugins hear `project_opened` for a new or opened project, not for the
/// blank document behind the home screen after Close.
#[test]
fn closing_a_project_opens_none_for_plugins() {
    let root = std::env::temp_dir().join(format!("ccad-plugins-ui-{}-events", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("watch");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("plugin.toml"),
        "id = \"watch\"
name = \"Watch\"
version = \"1.0\"
api = 2
",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.lua"),
        "ccad.on('project_opened', function(e) print('opened ' .. tostring(e.new)) end)",
    )
    .unwrap();
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(root.clone());
    crate::plugins_ui::load(&mut state);
    assert_eq!(state.session.take_extension_messages(), vec!["[watch] opened true"], "heard when loaded");
    crate::actions::run_pending(&mut state, crate::state::PendingAction::Close);
    assert!(state.session.take_extension_messages().is_empty(), "nothing opened on Close");
    assert!(state.session.extension().is_some(), "the plugins stay");
    crate::actions::run_pending(&mut state, crate::state::PendingAction::New(cc_core::MeasurementSystem::Metric));
    assert_eq!(state.session.take_extension_messages(), vec!["[watch] opened true"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// The Plugins window lists the folder's plugins; turning one off takes its
/// commands out of the session, and the loaded plugins stay with a new document.
#[test]
fn plugins_window_turns_plugins_off_and_on() {
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(plugins_folder("window"));
    crate::plugins_ui::load(&mut state);
    assert!(state.session.kind_of("room_tools__one_wall").is_some(), "its command joined the session");
    state.replace_document(cc_core::Document::new(crate::actions::new_project()));
    assert!(state.session.kind_of("room_tools__one_wall").is_some(), "kept with a new document");
    crate::plugins_ui::open(&mut state);
    let mut harness = app_harness(state);
    harness.get_by_label_contains("Room tools").click();
    harness.run();
    assert!(harness.query_by_label_contains("room_tools__one_wall").is_some(), "its commands are listed");
    harness.get_by_label(&t!("plug-enabled")).click();
    harness.run();
    assert!(harness.state().prefs.plugins_off.contains("room_tools"));
    assert!(harness.state().session.kind_of("room_tools__one_wall").is_none(), "turned off");
    harness.get_by_label(&t!("plug-enabled")).click();
    harness.run();
    assert!(harness.state().session.kind_of("room_tools__one_wall").is_some(), "turned on again");
    // The footer's Close (the header has a close button too).
    harness.get_all_by_label(&t!("action-close")).last().unwrap().click();
    harness.run();
    assert!(!harness.state().plugins.open);
}

/// Plugin Commands opens a plugin command's form in the palette, which runs
/// it as one undo step and shows what it printed.
#[test]
fn plugin_commands_run_from_the_palette() {
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(plugins_folder("palette"));
    crate::plugins_ui::load(&mut state);
    let walls =
        |s: &AppState| s.project().elements.values().filter(|e| e.category() == cc_core::Category::Wall).count();
    let before = walls(&state);
    crate::palette::open_command(&mut state, "room_tools__one_wall");
    let mut harness = themed_harness(state, egui::vec2(900.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::palette::show(ui.ctx(), state)
    });
    harness.get_by_label(&t!("palette-run")).click();
    harness.run();
    assert_eq!(walls(harness.state()), before + 1);
    assert!(harness.query_by_label_contains("made a wall").is_some(), "what it printed is shown");
}

/// The plugin `form_tools`, whose commands take every kind of field.
const FORM_TOOLS: &str = r#"
ccad.command {
  name = "mark_wall",
  title = "Mark wall",
  params = {
    properties = {
      wall = { type = "integer", ui = "element", category = "wall", description = "The wall to mark." },
      note = { type = "string", default = "marked" },
      walls = { type = "array", ui = "elements", category = "wall" },
      at = { type = "array", ui = "point" },
      flag = { type = "boolean" },
      count = { type = "integer", minimum = 1, maximum = 9 },
    },
    required = { "wall" },
  },
  run = function(args)
    ccad.run("element_set", { id = args.wall, comments = args.note or "marked" })
  end,
}

ccad.command {
  name = "wall_at",
  title = "Wall at a point",
  params = {
    properties = {
      at = { type = "array", ui = "point", description = "Where the wall starts." },
      length = { type = "number", ui = "length", default = 2, minimum = 0.1 },
    },
    required = { "at", "length" },
  },
  run = function(args)
    local id = ccad.run("wall_create", { start = args.at, ["end"] = { args.at[1] + args.length, args.at[2] } }).id
    print("wall " .. id)
    return { id = id }
  end,
}

ccad.command {
  name = "layout",
  title = "Every field",
  description = "A form with one field of each kind, all of them required, to see how it is laid out.",
  params = {
    properties = {
      a_length = { type = "number", ui = "length", default = 2.5, minimum = 0.1 },
      b_count = { type = "integer", default = 3 },
      c_flag = { type = "boolean", default = true },
      d_side = { type = "string", enum = { "left", "right" }, enum_labels = { "Left side", "Right side" } },
      e_name = { type = "string", default = "A rather long default name to see how it is cut in its cell" },
      f_wall = { type = "integer", ui = "element", category = "wall" },
      g_walls = { type = "array", ui = "elements" },
      h_at = { type = "array", ui = "point", default = { 1.5, 2.25 } },
      i_level = { type = "integer", ui = "level" },
      j_view = { type = "integer", ui = "view" },
      k_type = { type = "integer", ui = "type", category = "wall" },
      l_material = { type = "string", ui = "material" },
      m_extra = { type = "object", default = { a = 1 } },
    },
    required = { "a_length", "b_count", "c_flag", "d_side", "e_name", "f_wall", "g_walls", "h_at", "i_level",
      "j_view", "k_type", "l_material", "m_extra" },
  },
  run = function() end,
}

ccad.command { name = "noop", title = "Do nothing", run = function() end }

ccad.command {
  name = "count_walls",
  title = "Count walls",
  kind = "query",
  run = function() return { walls = #ccad.run("elements_list", { category = "wall" }) } end,
}

ccad.command {
  name = "report",
  title = "Report",
  kind = "query",
  run = function()
    print(string.rep("a long printed line ", 30))
    return { text = string.rep("wide ", 60), list = { 1, 2, 3 } }
  end,
}

ccad.command {
  name = "fail",
  title = "Always fails",
  params = {
    properties = { why = { type = "string", default = "no luck: " .. string.rep("the reason goes on ", 8) } },
    required = { "why" },
  },
  run = function(args) print("trying") error(args.why, 0) end,
}
"#;

/// A plugins folder with the plugin `form_tools` ([`FORM_TOOLS`]).
fn form_plugins_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-plugin-form-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("form_tools");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plugin.toml"), "id = \"form_tools\"\nname = \"Form tools\"\nversion = \"1.0.0\"\n")
        .unwrap();
    std::fs::write(dir.join("main.lua"), FORM_TOOLS).unwrap();
    root
}

/// The sample house with the plugins of `folder` loaded.
fn state_with_plugins(folder: std::path::PathBuf) -> AppState {
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(folder);
    crate::plugins_ui::load(&mut state);
    state
}

fn form_field(state: &AppState, name: &str) -> crate::plugin_form::Field {
    let form = state.dialogs.plugin_form.as_ref().expect("the form is open");
    form.fields().iter().find(|f| f.name == name).cloned().expect("the field")
}

/// *Plugin Commands* opens the example plugin's command in its form: the
/// lengths typed, Run creates the room as one undo step and shows what it
/// returned; Run Again goes back to the form with the values kept.
#[test]
fn plugin_commands_open_their_form_from_the_ribbon() {
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins");
    let mut state = state_with_plugins(examples);
    state.ribbon_tab = RibbonTab::Manage;
    let walls =
        |s: &AppState| s.project().elements.values().filter(|e| e.category() == cc_core::Category::Wall).count();
    let before = (walls(&state), state.session.doc.undo_depth());
    // Wide enough for the whole Manage tab.
    let mut harness = themed_harness(state, egui::vec2(1800.0, 800.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::ribbon::ribbon(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    });
    harness.get_by_role_and_label(Role::Button, &t!("action-plugin-commands")).click();
    harness.run();
    // The menu and the form show the command's title in the language (the
    // tests run in pt-BR, the reference language); its lengths are typed in
    // the project's units, or with a unit.
    harness.get_by_role_and_label(Role::Button, "Sala retangular").click();
    harness.run();
    assert!(harness.state().dialogs.plugin_form.is_some(), "the menu opens the form");
    assert!(harness.query_by_label("Sala retangular").is_some(), "titled by the command's title");
    assert!(harness.query_by_label("Salas rápidas").is_some(), "the plugin's name by the title");
    let run_disabled = |h: &Harness<'static, AppState>| {
        h.get_by_role_and_label(Role::Button, &t!("palette-run")).accesskit_node().is_disabled()
    };
    assert!(run_disabled(&harness), "the width and depth are required");
    let hint = harness.get_by_label("Largura").accesskit_node().placeholder().map(str::to_string);
    assert_eq!(hint, Some(t!("pform-required-hint")), "an empty required field says so");
    for (label, value) in [("Largura", "4 m"), ("Profundidade", "3 m")] {
        harness.get_by_label(label).focus();
        harness.run();
        harness.get_by_label(label).type_text(value);
        harness.run();
    }
    assert!(!run_disabled(&harness));
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(walls(harness.state()), before.0 + 4);
    assert_eq!(harness.state().session.doc.undo_depth(), before.1 + 1, "one undo step");
    assert!(harness.state().dialogs.plugin_form.as_ref().unwrap().shows_result());
    assert!(harness.query_by_label_contains("\"floor\"").is_some(), "what it returned is shown");
    harness.get_by_role_and_label(Role::Button, &t!("pform-run-again")).click();
    harness.run();
    assert_eq!(harness.get_by_label("Largura").value().as_deref(), Some("4 m"), "the values are kept");
    harness.get_by_role_and_label(Role::Button, &t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.plugin_form.is_none());
}

/// A copy of `from` (a folder) at `to`.
fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// A plugins folder with a copy of the example plugin, so its settings are
/// written outside the sources.
fn quick_rooms_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-quick-rooms-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/quick_rooms");
    copy_dir(&example, &root.join("quick_rooms"));
    root
}

/// The ribbon, the shortcuts and the dialogs, wide enough for every tab.
fn wide_ribbon_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1800.0, 800.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::ribbon::ribbon(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    })
}

/// The Plugins tab is there only while a loaded plugin has buttons (a tab
/// left picked falls back to Manage); its buttons carry the plugin's text
/// in the language, follow a change of language, and open their form.
#[test]
fn the_plugins_tab_shows_the_plugins_buttons() {
    let plugins_label = t!("ribbon-plugins");
    let count = |h: &Harness<'static, AppState>| h.query_all_by_label(&plugins_label).count();
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Plugins;
    let mut harness = wide_ribbon_harness(state);
    harness.run();
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Manage, "no plugin buttons: back to Manage");
    let without = count(&harness);
    let mut state = state_with_plugins(quick_rooms_folder("tab"));
    state.ribbon_tab = RibbonTab::Manage;
    let mut harness = wide_ribbon_harness(state);
    harness.run();
    assert_eq!(count(&harness), without + 1, "the Plugins tab is shown");
    harness.state_mut().ribbon_tab = RibbonTab::Plugins;
    harness.run();
    assert!(harness.query_by_label("Salas rápidas").is_some(), "the panel, named in the language");
    assert!(harness.query_by_role_and_label(Role::Button, "Totais de paredes").is_some());
    harness.get_by_role_and_label(Role::Button, "Sala retangular").click();
    harness.run();
    let title = harness.state().plugins.command_title("quick_rooms__rectangle_room").map(str::to_string);
    assert_eq!(title.as_deref(), Some("Sala retangular"));
    assert!(harness.state().dialogs.plugin_form.is_some(), "the button opens the form");
    assert!(harness.query_all_by_label("Sala retangular").count() >= 2, "the form is titled by the command's title");
    // Another language: the plugins load again, their text in it.
    crate::actions::set_language(harness.state_mut(), crate::i18n::Language::En);
    let labels: Vec<String> = harness.state().plugins.buttons.iter().map(|b| b.button.label.clone()).collect();
    assert_eq!(labels, ["Rectangular room", "Wall totals"]);
    assert_eq!(harness.state().plugins.command_title("quick_rooms__wall_totals"), Some("Wall totals"));
    crate::actions::set_language(harness.state_mut(), crate::i18n::Language::PtBr);
}

/// A plugins folder with `sel_tools`, whose small button needs walls selected.
fn selection_plugins_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-plugin-when-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("sel_tools");
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = "id = \"sel_tools\"\nname = \"Selection tools\"\nversion = \"1.0.0\"\napi = 2\n\n\
                    [[buttons]]\ncommand = \"mark\"\nlabel = \"Mark walls\"\nsize = \"small\"\nwhen = \"selection:wall\"\n";
    std::fs::write(dir.join("plugin.toml"), manifest).unwrap();
    std::fs::write(dir.join("main.lua"), "ccad.command { name = 'mark', title = 'Mark walls', run = function() end }")
        .unwrap();
    root
}

/// A button with `when = "selection:wall"` is available only while walls,
/// and nothing else, are selected; its tooltip tells why it is not.
#[test]
fn plugin_buttons_follow_their_when() {
    use std::collections::BTreeSet;
    let mut state = state_with_plugins(selection_plugins_folder("ribbon"));
    state.ribbon_tab = RibbonTab::Plugins;
    let wall = state.project().walls().map(|(e, _)| e.id).next().unwrap();
    let door = state.project().elements.values().find(|e| e.category() == cc_core::Category::Door).unwrap().id;
    let mut harness = wide_ribbon_harness(state);
    harness.run();
    let disabled = |h: &Harness<'static, AppState>| {
        h.get_by_role_and_label(Role::Button, "Mark walls").accesskit_node().is_disabled()
    };
    assert!(disabled(&harness), "nothing selected");
    harness.get_by_role_and_label(Role::Button, "Mark walls").hover();
    harness.run();
    let why = t!("plug-when-category", "category" => crate::actions::t_category(cc_core::Category::Wall));
    assert!(harness.query_by_label_contains(&why).is_some(), "the tooltip says why");
    harness.state_mut().selection = BTreeSet::from([wall]);
    harness.run();
    assert!(!disabled(&harness), "a wall selected");
    harness.state_mut().selection = BTreeSet::from([wall, door]);
    harness.run();
    assert!(disabled(&harness), "a door too");
    harness.state_mut().selection = BTreeSet::from([wall]);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Mark walls").click();
    harness.run();
    assert_eq!(harness.state().message, t!("pform-done", "title" => "Mark walls"), "no parameters: it ran");
}

/// The Settings section of the Plugins window stores the values that
/// differ from the defaults, keeps the stored one when a value typed does
/// not fit, and the plugin loaded again runs with them; Reset to defaults
/// empties them.
#[test]
fn plugin_settings_are_saved_and_used() {
    let folder = quick_rooms_folder("settings");
    let mut state = state_with_plugins(folder.clone());
    crate::plugins_ui::open(&mut state);
    state.plugins.selected = Some("quick_rooms".into());
    let mut harness = app_harness(state);
    harness.run();
    assert!(harness.query_by_label(&t!("plug-settings")).is_some(), "the section");
    let height = "Altura padrão das paredes";
    let retype = |h: &mut Harness<'static, AppState>, text: &str| {
        h.get_by_label(height).focus();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.get_by_label(height).type_text(text);
        h.run();
    };
    retype(&mut harness, "3,5 m");
    // The dialog's Save, after the quick access toolbar's.
    harness.get_all_by_label(&t!("plug-settings-save")).last().unwrap().click();
    harness.run();
    let stored = |folder: &std::path::Path| cc_plugins::store::read_settings(folder, "quick_rooms");
    assert_eq!(serde_json::Value::Object(stored(&folder)), serde_json::json!({"default_height": 3.5}));
    // Below the minimum: shown as invalid, and the stored value stays.
    retype(&mut harness, "1 m");
    // The dialog's Save, after the quick access toolbar's.
    harness.get_all_by_label(&t!("plug-settings-save")).last().unwrap().click();
    harness.run();
    assert_eq!(serde_json::Value::Object(stored(&folder)), serde_json::json!({"default_height": 3.5}));
    // The plugin was loaded again with the setting: a room without a height uses it.
    let out = harness
        .state_mut()
        .session
        .execute("quick_rooms__rectangle_room", serde_json::json!({"width": 4, "depth": 3}))
        .unwrap();
    let south = cc_core::ElementId(out["walls"]["south"].as_u64().unwrap());
    let wall_height = harness.state().project().walls().find(|(e, _)| e.id == south).map(|(_, w)| w.height);
    assert_eq!(wall_height, Some(3.5));
    let room = out["room"].as_u64().unwrap();
    let data = harness
        .state_mut()
        .session
        .execute("plugin_data_get", serde_json::json!({"plugin": "quick_rooms", "element": room}))
        .unwrap();
    assert_eq!(data, serde_json::json!({"width": 4, "depth": 3}), "the size kept on the room");
    harness.get_by_role_and_label(Role::Button, &t!("plug-settings-reset")).click();
    harness.run();
    assert!(stored(&folder).is_empty(), "back to the defaults");
    assert_eq!(harness.get_by_label(height).value().as_deref().map(|v| v.starts_with("2")), Some(true));
    // Turned off, it can still be set, and the window says when it applies.
    harness.get_by_label(&t!("plug-enabled")).click();
    harness.run();
    assert!(harness.query_by_label(height).is_some());
    assert!(harness.query_by_label(&t!("plug-settings-off")).is_some());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Elements come from the selection: a list starts with the selected walls
/// (not the door), and Selection fills a single wall when exactly one is selected.
#[test]
fn plugin_form_takes_the_selection() {
    use std::collections::BTreeSet;
    let mut state = state_with_plugins(form_plugins_folder("selection"));
    let walls: Vec<cc_core::ElementId> = state.project().walls().map(|(e, _)| e.id).take(2).collect();
    let door = state.project().elements.values().find(|e| e.category() == cc_core::Category::Door).unwrap().id;
    state.selection = BTreeSet::from([walls[0], walls[1], door]);
    crate::plugin_form::open(&mut state, "form_tools__mark_wall");
    let ids = serde_json::json!(walls.iter().map(|w| w.0).collect::<Vec<_>>());
    assert_eq!(form_field(&state, "walls").effective(), Some(&ids), "the selected walls, without the door");
    let mut harness = app_harness(state);
    let selection = |h: &Harness<'static, AppState>| {
        h.get_by_role_and_label(Role::Button, &t!("pform-selection")).accesskit_node().is_disabled()
    };
    assert!(selection(&harness), "two walls are selected");
    harness.state_mut().selection = BTreeSet::from([walls[0], door]);
    harness.run();
    assert!(!selection(&harness));
    harness.get_by_role_and_label(Role::Button, &t!("pform-selection")).click();
    harness.run();
    assert_eq!(form_field(harness.state(), "wall").effective(), Some(&serde_json::json!(walls[0].0)));
    assert!(harness.query_by_label_contains(&format!("#{} ", walls[0].0)).is_some(), "the wall is named");
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(harness.state().project().element(walls[0]).unwrap().comments, "marked");
    assert!(harness.state().dialogs.plugin_form.is_none(), "nothing to show: the form closes");
    assert_eq!(harness.state().message, t!("pform-done", "title" => "Mark wall"));
}

/// Where the pick harness centers the plan, and its scale (points a meter).
const PICK_ORIGIN: glam::DVec2 = glam::DVec2::new(36.0, 2.0);
const PICK_SCALE: f64 = 20.0;
const PICK_SIZE: egui::Vec2 = egui::vec2(900.0, 640.0);

/// The active plan over the whole window with its tools, the dialogs over
/// it and the keyboard shortcuts, as the app runs them.
fn pick_harness(state: AppState) -> Harness<'static, AppState> {
    pick_harness_at(state, PICK_ORIGIN)
}

/// [`pick_harness`] centered on the drawing point `origin`.
fn pick_harness_at(state: AppState, origin: glam::DVec2) -> Harness<'static, AppState> {
    themed_harness(state, PICK_SIZE, egui::Theme::Light, move |ui, state| {
        state.sync();
        state.hint.begin_frame();
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin, scale: PICK_SCALE };
        if let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned())
            && let Some(drawing) = state.drawing(view.id)
        {
            crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
        }
        crate::dialogs::show(ui.ctx(), state);
        state.hint.end_frame();
    })
}

/// A click at the plan point `p` of the pick harness.
fn click_plan(harness: &mut Harness<'static, AppState>, p: [f64; 2]) {
    click_at(harness, PICK_ORIGIN, p);
}

/// A click at the drawing point `p` of a pick harness centered on `origin`.
fn click_at(harness: &mut Harness<'static, AppState>, origin: glam::DVec2, p: [f64; 2]) {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, PICK_SIZE);
    let xf = crate::paint::Xf { screen: rect.center(), origin, scale: PICK_SCALE };
    let pos = xf.to_screen(glam::DVec2::from(p));
    harness.input_mut().events.push(egui::Event::PointerMoved(pos));
    harness.run();
    for pressed in [true, false] {
        let button = egui::PointerButton::Primary;
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Default::default(),
        });
        harness.run();
    }
}

/// The pipes of the project as (start, end, start elevation, end elevation) in plan, rounded to cm.
fn pipe_runs(state: &AppState) -> Vec<([i64; 2], [i64; 2], i64, i64)> {
    let cm = |v: f64| (v * 100.0).round() as i64;
    state
        .project()
        .elements
        .values()
        .filter_map(|e| match &e.kind {
            cc_core::ElementKind::Pipe(s) => {
                Some(([cm(s.start.x), cm(s.start.y)], [cm(s.end.x), cm(s.end.y)], cm(s.start_offset), cm(s.end_offset)))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_pipe_tool_joins_runs_at_other_elevations_with_vertical_runs() {
    let mut state = sample_state();
    let level = state.project().levels.first().map(|l| l.id).unwrap();
    let plan = state.project().views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    let system = state.session.execute("system_create", serde_json::json!({"kind": "cold_water"})).unwrap()["id"]
        .as_u64()
        .unwrap();
    // A pipe 0.60 m above the level, away from the house.
    state
        .session
        .execute(
            "pipe_create",
            serde_json::json!({"start": [32.0, 2.0], "end": [36.0, 2.0], "offset": 0.6, "system": system}),
        )
        .unwrap();
    let type_id = state.project().default_type(cc_core::Category::Pipe).unwrap();
    state.tool = crate::tools::Tool::Pipe { type_id };
    state.mep.system = Some(cc_core::SystemId(system));
    state.mep.offset = Some(2.5);
    let mut harness = pick_harness(state);
    harness.run();
    // Starting on the end of the pipe at 2.50 m: a riser from it, then the run.
    click_plan(&mut harness, [36.0, 2.0]);
    assert_eq!(harness.state().mep.chain, Some(0.6));
    click_plan(&mut harness, [40.0, 2.0]);
    let runs = pipe_runs(harness.state());
    assert!(runs.contains(&([3600, 200], [3600, 200], 60, 250)), "{runs:?}");
    assert!(runs.contains(&([3600, 200], [4000, 200], 250, 250)), "{runs:?}");
    // A new elevation, clicking the last point again: only the vertical run.
    harness.state_mut().mep.rise = Some(1.0);
    click_plan(&mut harness, [40.0, 2.0]);
    assert!(pipe_runs(harness.state()).contains(&([4000, 200], [4000, 200], 250, 100)));
    assert_eq!(harness.state().mep.chain, Some(1.0));
    // A run at 2.50 m ending over the first pipe goes down to it there.
    harness.state_mut().interaction.anchor = None;
    harness.state_mut().mep.offset = Some(2.5);
    click_plan(&mut harness, [34.0, 6.0]);
    assert_eq!(harness.state().mep.chain, None);
    click_plan(&mut harness, [34.0, 2.0]);
    let runs = pipe_runs(harness.state());
    assert!(runs.contains(&([3400, 600], [3400, 200], 250, 250)), "{runs:?}");
    assert!(runs.contains(&([3400, 200], [3400, 200], 250, 60)), "{runs:?}");
    assert_eq!(runs.len(), 6);
    // The network joins them: a tee on the first pipe where the riser lands.
    let geom = harness.state().session.geom();
    let tee = geom.mep.fittings.iter().any(|f| {
        matches!(f.kind, cc_geom::mep_net::FittingKind::Tee)
            && f.point.truncate().distance(glam::dvec2(34.0, 2.0)) < 0.02
    });
    assert!(tee, "{:?}", geom.mep.fittings);
}

#[test]
fn pipe_tags_and_fitting_labels_are_dragged_in_a_plumbing_plan() {
    let mut state = sample_state();
    let run = |state: &mut AppState, name: &str, args: serde_json::Value| state.session.execute(name, args).unwrap();
    let plan = run(&mut state, "view_create", serde_json::json!({"kind": "plumbing_plan"}))["id"].as_u64().unwrap();
    let plan = cc_core::ViewId(plan);
    run(&mut state, "view_set", serde_json::json!({"view": plan.0, "fitting_tags": true}));
    let pipe = |state: &mut AppState, a: [f64; 2], b: [f64; 2]| {
        let args = serde_json::json!({"start": a, "end": b, "offset": 0.6, "system": "cold_water"});
        cc_core::ElementId(run(state, "pipe_create", args)["id"].as_u64().unwrap())
    };
    let first = pipe(&mut state, [32.0, 2.0], [38.0, 2.0]);
    let second = pipe(&mut state, [38.0, 2.0], [38.0, 6.0]);
    state.active_view = Some(plan);
    state.tool = crate::tools::Tool::Select;
    let mut harness = pick_harness(state);
    harness.run();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, PICK_SIZE);
    let xf = crate::paint::Xf { screen: rect.center(), origin: PICK_ORIGIN, scale: PICK_SCALE };
    let drag = |h: &mut Harness<'static, AppState>, from: egui::Pos2, by: egui::Vec2| {
        let button = egui::PointerButton::Primary;
        h.input_mut().events.push(egui::Event::PointerMoved(from));
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: from,
            button,
            pressed: true,
            modifiers: Default::default(),
        });
        h.run();
        for k in 1..=4 {
            h.input_mut().events.push(egui::Event::PointerMoved(from + by * (k as f32 / 4.0)));
            h.run();
        }
        let pos = from + by;
        h.input_mut().events.push(egui::Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Default::default(),
        });
        h.run();
    };
    // The pipe's tag moves 1 m to the right; the pipe stays.
    let tag = harness.state_mut().drawing(plan).unwrap().tags.iter().find(|t| t.element == first).copied().unwrap();
    drag(&mut harness, xf.to_screen(tag.center), egui::vec2(20.0, 0.0));
    let placement = harness.state().project().view(plan).unwrap().tags.get(&first).copied().unwrap_or_default();
    assert!((placement.offset - glam::DVec2::new(1.0, 0.0)).length() < 1e-6, "{placement:?}");
    assert!(pipe_runs(harness.state()).contains(&([3200, 200], [3800, 200], 60, 60)));
    // The elbow's label moves by the runs meeting there.
    let key = cc_core::tag::fitting_key(&[first, second]);
    let label = harness.state_mut().drawing(plan).unwrap().fitting_tags.iter().find(|t| t.key == key).cloned();
    let label = label.unwrap_or_else(|| panic!("no label of fitting {key}"));
    drag(&mut harness, xf.to_screen(label.tag.center), egui::vec2(20.0, 0.0));
    let moved = harness.state().project().view(plan).unwrap().fitting_placements.get(&key).copied().unwrap_or_default();
    assert!((moved.offset - glam::DVec2::new(1.0, 0.0)).length() < 1e-6, "{moved:?}");
}

#[test]
fn the_pipe_tool_draws_vertical_runs_in_a_section() {
    let mut state = sample_state();
    let system = state.session.execute("system_create", serde_json::json!({"kind": "cold_water"})).unwrap()["id"]
        .as_u64()
        .unwrap();
    // A pipe 0.60 m above the ground level, 3 m behind a section looking south.
    state
        .session
        .execute(
            "pipe_create",
            serde_json::json!({"start": [32.0, 2.0], "end": [36.0, 2.0], "offset": 0.6, "system": system}),
        )
        .unwrap();
    let section = state
        .session
        .execute("view_create", serde_json::json!({"kind": "section", "start": [30.0, 5.0], "end": [42.0, 5.0]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let view = state.project().view(cc_core::ViewId(section)).cloned().unwrap();
    let frame = cc_views::view_frame(&view).unwrap();
    let ground = state.project().levels.iter().map(|l| state.project().level_elevation(l.id)).fold(f64::MAX, f64::min);
    let at = |x: f64, y: f64, z: f64| frame.project(glam::dvec3(x, y, ground + z)).to_array();
    state.active_view = Some(view.id);
    let type_id = state.project().default_type(cc_core::Category::Pipe).unwrap();
    state.tool = crate::tools::Tool::Pipe { type_id };
    state.mep.system = Some(cc_core::SystemId(system));
    // Clicks land where they are (no snap to the level lines of the section).
    state.prefs.snaps.enabled = false;
    let origin = glam::DVec2::from(at(36.0, 2.0, 1.5));
    let mut harness = pick_harness_at(state, origin);
    harness.run();
    // From the end of the pipe straight up, then across at 2.50 m.
    click_at(&mut harness, origin, at(36.0, 2.0, 0.6));
    click_at(&mut harness, origin, at(36.0, 2.0, 2.5));
    click_at(&mut harness, origin, at(40.0, 2.0, 2.5));
    // (The harness's clicks may land a few pixels off in height: compare within 15 cm.)
    let runs = pipe_runs(harness.state());
    let riser = runs.iter().find(|r| r.0 == [3600, 200] && r.1 == [3600, 200]);
    let Some(&(_, _, 60, top)) = riser else { panic!("no riser from the pipe: {runs:?}") };
    assert!((top - 250).abs() <= 15, "{runs:?}");
    assert!(runs.contains(&([3600, 200], [4000, 200], top, top)), "{runs:?}");
    // The riser joins the pipe: an elbow at its foot and one at its top.
    let geom = harness.state().session.geom();
    let elbows = geom.mep.fittings.iter().filter(|f| matches!(f.kind, cc_geom::mep_net::FittingKind::Elbow { .. }));
    assert_eq!(elbows.count(), 2, "{:?}", geom.mep.fittings);
}

/// The sample house with the form plugin, a wall and a column away from
/// the house, and their plan active: (state, wall, column).
fn pick_state(name: &str) -> (AppState, u64, u64) {
    let mut state = state_with_plugins(form_plugins_folder(name));
    let run = |s: &mut AppState, command: &str, args: serde_json::Value| {
        s.session.execute(command, args).unwrap()["id"].as_u64().unwrap()
    };
    let wall = run(&mut state, "wall_create", serde_json::json!({"start": [30.0, 0.0], "end": [34.0, 0.0]}));
    let column = run(&mut state, "column_create", serde_json::json!({"position": [40.0, 0.0]}));
    let level = state.project().element(cc_core::ElementId(wall)).unwrap().level;
    let plan = state.project().views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    (state, wall, column)
}

/// Pick hides the form and picks in the view: Esc brings the form back
/// unchanged, an element of another category is refused with a notice, and
/// a wall fills the field; Run then edits it.
#[test]
fn plugin_form_picks_an_element_in_the_view() {
    use crate::tools::Tool;
    let (mut state, wall, column) = pick_state("pick-element");
    crate::plugin_form::open(&mut state, "form_tools__mark_wall");
    let mut harness = pick_harness(state);
    let shown = |h: &Harness<'static, AppState>| h.query_by_label(&t!("palette-run")).is_some();
    assert!(shown(&harness));
    harness.get_by_role_and_label(Role::Button, &t!("pform-pick")).click();
    harness.run();
    assert_eq!(harness.state().tool, Tool::Pick { point: false });
    assert!(!shown(&harness), "the form is hidden while picking");
    let prompt = t!("pform-pick-element-of", "category" => crate::actions::t_category(cc_core::Category::Wall));
    assert_eq!(harness.state().hint.shown(), Some(prompt.as_str()), "the status bar prompts");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().tool, Tool::Select);
    assert!(shown(&harness), "Esc brings the form back");
    assert_eq!(form_field(harness.state(), "wall").effective(), None, "unchanged");

    harness.get_by_role_and_label(Role::Button, &t!("pform-pick")).click();
    harness.run();
    click_plan(&mut harness, [40.0, 0.0]);
    assert_eq!(harness.state().tool, Tool::Pick { point: false }, "a column is not a wall: still picking");
    assert!(harness.state().hint.shown().is_some_and(|h| h.contains(&prompt) && h.len() > prompt.len()));
    assert_eq!(harness.state().dialogs.plugin_form.as_ref().unwrap().picking(), Some(0));
    assert!(harness.state().project().element(cc_core::ElementId(column)).is_some());
    click_plan(&mut harness, [32.0, 0.0]);
    assert_eq!(harness.state().tool, Tool::Select);
    assert!(shown(&harness), "the form comes back");
    assert_eq!(form_field(harness.state(), "wall").effective(), Some(&serde_json::json!(wall)));
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(harness.state().project().element(cc_core::ElementId(wall)).unwrap().comments, "marked");
}

/// A point field picks a point in the plan; the command's result and what
/// it printed show after Run.
#[test]
fn plugin_form_picks_a_point_in_the_plan() {
    let (mut state, _, _) = pick_state("pick-point");
    let walls =
        |s: &AppState| s.project().elements.values().filter(|e| e.category() == cc_core::Category::Wall).count();
    let before = walls(&state);
    crate::plugin_form::open(&mut state, "form_tools__wall_at");
    let mut harness = pick_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("pform-pick")).click();
    harness.run();
    assert_eq!(harness.state().tool, crate::tools::Tool::Pick { point: true });
    click_plan(&mut harness, [38.0, 6.0]);
    let at = form_field(harness.state(), "at").effective().cloned().unwrap();
    let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
    assert!((x - 38.0).abs() < 0.05 && (y - 6.0).abs() < 0.05, "{at}");
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(walls(harness.state()), before + 1);
    assert!(harness.state().dialogs.plugin_form.as_ref().unwrap().shows_result());
    assert!(harness.query_by_label_contains("[form_tools] wall").is_some(), "what it printed");
    harness.get_all_by_label(&t!("action-close")).last().unwrap().click();
    harness.run();
    assert!(harness.state().dialogs.plugin_form.is_none());
}

/// The JSON button shows the arguments as JSON and reads them back; JSON
/// the form cannot hold stays JSON, with the reason.
#[test]
fn plugin_form_goes_to_json_and_back() {
    let mut state = state_with_plugins(form_plugins_folder("json"));
    let wall = state.project().walls().map(|(e, _)| e.id).next().unwrap();
    crate::plugin_form::open(&mut state, "form_tools__mark_wall");
    let mut harness = app_harness(state);
    let type_json = |h: &mut Harness<'static, AppState>, text: &str| {
        h.get_by_role_and_label(Role::MultilineTextInput, &t!("palette-arguments")).focus();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.get_by_role_and_label(Role::MultilineTextInput, &t!("palette-arguments")).type_text(text);
        h.run();
    };
    let form = |h: &Harness<'static, AppState>| h.state().dialogs.plugin_form.as_ref().unwrap().shows_json();
    harness.get_by_role_and_label(Role::Button, &t!("pform-json")).click();
    harness.run();
    assert!(form(&harness));
    type_json(&mut harness, r#"{"nope": 1}"#);
    harness.get_by_role_and_label(Role::Button, &t!("pform-json")).click();
    harness.run();
    assert!(form(&harness), "an unknown property stays in JSON");
    assert!(harness.query_by_label(&t!("pform-json-unknown", "name" => "nope")).is_some(), "with the reason");
    type_json(&mut harness, &format!(r#"{{"wall": {}, "note": "via json"}}"#, wall.0));
    harness.get_by_role_and_label(Role::Button, &t!("pform-json")).click();
    harness.run();
    assert!(!form(&harness), "back in the form");
    assert_eq!(form_field(harness.state(), "note").effective(), Some(&serde_json::json!("via json")));
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert_eq!(harness.state().project().element(wall).unwrap().comments, "via json");
}

/// A command that fails leaves the form open with its message and what it
/// printed; one without parameters runs at once, closing or showing its result.
#[test]
fn plugin_form_errors_and_commands_without_parameters() {
    let mut state = state_with_plugins(form_plugins_folder("run"));
    crate::plugin_form::open(&mut state, "form_tools__noop");
    assert!(state.dialogs.plugin_form.is_none(), "nothing to ask nor to show");
    assert_eq!(state.message, t!("pform-done", "title" => "Do nothing"));
    crate::plugin_form::open(&mut state, "form_tools__fail");
    let mut harness = app_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    let error = harness.state().dialogs.plugin_form.as_ref().unwrap().error().unwrap_or_default().to_string();
    assert!(error.contains("[form_tools] trying") && error.contains("no luck"), "{error}");
    assert!(harness.query_by_label_contains("no luck").is_some(), "shown in the form");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().dialogs.plugin_form.is_none());
    let count = harness.state().project().walls().count();
    crate::plugin_form::open(harness.state_mut(), "form_tools__count_walls");
    harness.run();
    assert!(harness.state().dialogs.plugin_form.as_ref().unwrap().shows_result());
    assert!(harness.query_by_label_contains(&format!("\"walls\": {count}")).is_some());
}

/// The palette opens a plugin's command in its form.
#[test]
fn the_palette_opens_plugin_commands_in_their_form() {
    let mut state = state_with_plugins(form_plugins_folder("palette"));
    crate::palette::toggle(&mut state);
    let mut harness = palette_harness(state);
    harness.get_by_role(Role::TextInput).type_text("form_tools__mark_wall");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(harness.state().palette.is_none());
    assert!(harness.state().dialogs.plugin_form.is_some());
}

/// Installing from a local archive and removing a plugin, as the window does.
#[test]
fn removing_a_plugin_from_the_window() {
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(plugins_folder("remove"));
    crate::plugins_ui::load(&mut state);
    crate::plugins_ui::open(&mut state);
    let mut harness = app_harness(state);
    harness.get_by_label_contains("Room tools").click();
    harness.run();
    harness.get_by_label(&t!("plug-remove")).click();
    harness.run();
    assert!(harness.state().plugins.listed.is_empty());
    assert!(harness.state().session.kind_of("room_tools__one_wall").is_none());
}

/// The Available tab lists the index's plugins; installing one that cannot
/// be downloaded (tests are offline) reports the failure.
#[test]
fn available_plugins_install_or_report() {
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(plugins_folder("available"));
    crate::plugins_ui::load(&mut state);
    let index = cc_plugins::store::Index::parse(
        br#"{"format": 1, "plugins": [{"id": "stairs_plus", "name": "Stairs plus", "description": "More stairs.",
             "versions": [{"version": "1.0.0", "api": 1, "url": "https://example.org/s.zip", "sha256": "00",
                           "permissions": ["files"]}]}]}"#,
    )
    .unwrap();
    crate::plugins_ui::set_index(&mut state, index);
    crate::plugins_ui::open(&mut state);
    let mut harness = app_harness(state);
    harness.get_by_label_contains("Stairs plus").click();
    harness.run();
    assert!(harness.query_by_label(&t!("plug-files")).is_some(), "it asks for files");
    harness.get_by_label(&t!("plug-install")).click();
    // One frame: the install runs in a thread (a spinner keeps repainting).
    harness.step();
    crate::plugins_ui::finish_job(harness.state_mut());
    harness.run();
    let notice = harness.state().plugins.notice.clone();
    assert!(matches!(notice, Some(Err(_))), "{notice:?}");
    assert!(!harness.state().prefs.plugins_files.contains("stairs_plus"), "no permission for a failed install");
}

/// A plugin that needs a newer ConstructCraft shows its version and why it
/// cannot be installed, with no Install button.
#[test]
fn available_plugins_say_what_they_need() {
    let mut state = sample_state();
    let index = cc_plugins::store::Index::parse(
        br#"{"format": 1, "plugins": [{"id": "future", "name": "Future tool",
             "versions": [{"version": "2.0.0", "api": 1, "url": "https://example.org/f.zip", "sha256": "00",
                           "min_app": "99.0.0"}]}]}"#,
    )
    .unwrap();
    crate::plugins_ui::set_index(&mut state, index);
    crate::plugins_ui::open(&mut state);
    let mut harness = app_harness(state);
    harness.get_by_label_contains("Future tool").click();
    harness.run();
    let needs = t!("plug-needs-app", "version" => "99.0.0", "current" => env!("CARGO_PKG_VERSION"));
    assert!(harness.query_by_label(&needs).is_some(), "says what it needs");
    assert!(
        harness.query_by_label(&t!("plug-version", "version" => "2.0.0")).is_some()
            || harness.query_by_label_contains("2.0.0").is_some()
    );
    assert!(harness.query_by_label(&t!("plug-install")).is_none());
}

/// A template project with a curtain wall selected (issue #155).
fn curtain_state() -> (AppState, cc_core::ElementId, cc_core::ElementTypeId) {
    let mut state = AppState::new(cc_core::Document::new(cc_core::Project::from_template()));
    state.project_open = true;
    state.show_home = false;
    let ty = crate::curtain_ui::new_curtain_type(&mut state).unwrap();
    let args = serde_json::json!({"start": [0.0, 0.0], "end": [6.0, 0.0], "type": ty.0});
    let id = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    state.session.geometry();
    state.selection.insert(id);
    (state, id, ty)
}

/// The Curtain Grid tool's options bar picks what a click does, and the
/// panel to put in.
#[test]
fn curtain_grid_tool_options() {
    use crate::curtain_ui::GridMode;
    let (mut state, _, _) = curtain_state();
    state.tool = crate::tools::Tool::CurtainGrid;
    let mut harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    assert_eq!(harness.state().curtain.mode, GridMode::AddLine);
    harness.get_by_label(&t!("curtain-axis-horizontal")).click();
    harness.run();
    assert_eq!(harness.state().curtain.axis, cc_core::curtain::GridAxis::Horizontal);
    harness.get_by_label(&t!("curtain-mode-panel")).click();
    harness.run();
    assert_eq!(harness.state().curtain.mode, GridMode::Panel);
    assert!(harness.query_by_label(&t!("prop-panel")).is_some(), "the panel to put in");
    assert!(harness.query_by_label(&t!("curtain-axis-horizontal")).is_none());
}

/// A selected curtain wall shows its grid in the properties palette; a
/// wall with its own lines resets them to the type's.
#[test]
fn curtain_wall_properties_show_the_grid() {
    let (mut state, id, _) = curtain_state();
    let add = serde_json::json!({"element": id.0, "axis": "Vertical", "add": [2.2]});
    state.session.execute("curtain_grid_set", add).unwrap();
    state.session.geometry();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1400.0), egui::Theme::Light, |ui, state| {
        egui::ScrollArea::vertical().show(ui, |ui| crate::properties::show(ui, state));
    });
    assert!(harness.query_by_label(&t!("section-curtain-grid")).is_some());
    assert!(harness.query_by_label(&t!("curtain-lines-own")).is_some());
    let own = |s: &AppState| match &s.project().element(id).unwrap().kind {
        cc_core::ElementKind::Wall(w) => w.curtain.vertical.is_some(),
        _ => false,
    };
    assert!(own(harness.state()));
    // Vertical lines first: theirs are the wall's own.
    harness.query_all_by_label(&t!("curtain-reset")).next().unwrap().click();
    harness.run();
    assert!(!own(harness.state()));
}

/// The Type Properties of a curtain type edit its grids, panel and
/// mullions; Cancel undoes the changes.
#[test]
fn curtain_type_properties_edit_the_grid() {
    let (mut state, _, ty) = curtain_state();
    state.dialogs.type_editor = Some(ty);
    let depth = state.session.doc.undo_depth();
    let mut harness = themed_harness(state, egui::vec2(900.0, 1400.0), egui::Theme::Light, |ui, state| {
        crate::type_dialog::show(ui.ctx(), state)
    });
    for key in
        ["section-curtain-vertical", "section-curtain-horizontal", "section-curtain-panel", "section-mullions-border"]
    {
        assert!(harness.query_by_label(&crate::i18n::tr(key)).is_some(), "{key}");
    }
    assert!(harness.query_by_label(&t!("prop-structure")).is_none(), "curtain types have no layers");
    // Border mullions off, then Cancel.
    let border = |s: &AppState| s.project().element_type(ty).unwrap().curtain().unwrap().border_mullion.is_some();
    assert!(border(harness.state()));
    let switch = format!("{}: {}", t!("section-mullions-border"), t!("prop-mullions-on"));
    harness.get_by_label(&switch).click_accesskit();
    harness.run();
    harness.run();
    assert!(!border(harness.state()));
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(border(harness.state()));
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
}

/// The wall tool's type list offers a curtain type when the project has
/// none; picking it makes one and draws with it.
#[test]
fn wall_tool_offers_a_curtain_type() {
    let mut state = sample_state();
    let wall_type = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.tool = crate::tools::Tool::Wall { type_id: wall_type };
    let mut harness = themed_harness(state, egui::vec2(1600.0, 400.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    let name = harness.state().project().type_name(wall_type).to_string();
    harness.get_by_value(&name).click();
    harness.run();
    harness.get_by_label(&crate::curtain_ui::new_type_label()).click();
    harness.run();
    let state = harness.state();
    let ty = state.tool.type_id().and_then(|id| state.project().element_type(id)).unwrap();
    assert!(ty.curtain().is_some(), "the tool draws curtain walls");
    assert_eq!(state.project().types_of(cc_core::Category::Wall).filter(|t| t.curtain().is_some()).count(), 1);
}

/// The graph editor alone, in a panel of its own.
fn graph_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1200.0, 520.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::graph_ui::show(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    })
}

fn wall_count(state: &AppState) -> usize {
    state.project().elements.values().filter(|e| e.category() == cc_core::Category::Wall).count()
}

/// The app harness wide enough for the whole Manage tab (narrower, the
/// ribbon scrolls and its last groups are out of reach).
fn manage_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1700.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::ribbon::ribbon(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    })
}

/// Visual Programming opens the graph editor from the Manage tab.
#[test]
fn visual_programming_opens_from_the_manage_tab() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    let mut harness = manage_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("action-visual-programming")).click();
    harness.run();
    assert!(harness.state().graph.open);
}

/// Run builds the graph's walls once; running again changes them in place.
#[test]
fn the_graph_editor_runs_the_graph() {
    let mut state = sample_state();
    let graph = serde_json::json!({
        "uid": "00000000000000000000000000000abc",
        "nodes": [
            {"id": 1, "kind": {"builtin": "number"}, "values": {"value": 3}, "pos": [0, 0]},
            {"id": 2, "kind": {"builtin": "sequence"}, "values": {"start": 30, "step": 4}, "pos": [220, 0]},
            {"id": 3, "kind": {"builtin": "point"}, "values": {"y": 30}, "pos": [440, 0]},
            {"id": 4, "kind": {"builtin": "point"}, "values": {"y": 34}, "pos": [440, 120]},
            {"id": 5, "kind": {"command": "wall_create"}, "pos": [660, 0]}
        ],
        "wires": [
            {"from": [1, "value"], "to": [2, "count"]},
            {"from": [2, "list"], "to": [3, "x"]},
            {"from": [2, "list"], "to": [4, "x"]},
            {"from": [3, "point"], "to": [5, "start"]},
            {"from": [4, "point"], "to": [5, "end"]}
        ]
    });
    state.graph.graph = cc_engine::graph::Graph::from_value(graph).unwrap().0;
    state.graph.open = true;
    let before = wall_count(&state);
    let mut harness = graph_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("graph-run")).click();
    harness.run();
    let report = harness.state().graph.report.clone().unwrap().unwrap();
    assert_eq!(report["errors"], 0, "{report}");
    assert_eq!(wall_count(harness.state()), before + 3);
    harness.get_by_role_and_label(Role::Button, &t!("graph-run")).click();
    harness.run();
    assert_eq!(wall_count(harness.state()), before + 3, "a second run changes the same walls");
}

/// The node search adds the node typed, where it was opened.
#[test]
fn the_node_search_adds_a_node() {
    let mut state = sample_state();
    state.graph.open = true;
    crate::graph_ui::open_search(&mut state, egui::pos2(100.0, 50.0), egui::pos2(300.0, 200.0));
    let mut harness = graph_harness(state);
    harness.get_by_label(&t!("graph-search-hint")).type_text("polygon_points");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let g = &harness.state().graph.graph;
    assert_eq!(g.nodes.len(), 1, "one node added");
    assert_eq!(g.nodes[0].kind, cc_engine::graph::NodeKind::Builtin("polygon_points".into()));
    assert_eq!(g.nodes[0].pos, [100.0, 50.0]);
}

/// The macro editor alone, with the app's shortcuts and dialogs.
fn macro_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1300.0, 520.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::macro_ui::show(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    })
}

/// Two walls away from the sample house, one step each.
const TWO_WALLS: &str = "// Two walls\n\
    {\"command\": \"wall_create\", \"args\": {\"start\": [30, 0], \"end\": [36, 0]}}\n\
    {\"command\": \"wall_create\", \"args\": {\"start\": [36, 0], \"end\": [36, 5]}}\n";

fn macro_state(text: &str) -> AppState {
    let mut state = sample_state();
    state.macro_editor.open = true;
    state.macro_editor.load_text(text);
    state
}

fn click(harness: &mut Harness<'static, AppState>, label: &str) {
    harness.get_by_role_and_label(Role::Button, label).click();
    harness.run();
}

fn disabled(harness: &Harness<'static, AppState>, label: &str) -> bool {
    harness.get_by_role_and_label(Role::Button, label).accesskit_node().is_disabled()
}

/// Macro Editor opens the panel from the Manage tab, in the graph editor's
/// place: opening one hides the other, which keeps its state.
#[test]
fn macro_editor_opens_from_the_manage_tab() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    state.graph.open = true;
    let mut harness = manage_harness(state);
    click(&mut harness, &t!("action-macro-editor"));
    assert!(harness.state().macro_editor.open);
    assert!(!harness.state().graph.open, "one panel under the views at a time");
    assert!(crate::commands::UiCommand::MacroEditor.selected(harness.state()));
    harness.state_mut().macro_editor.text = "kept".into();
    click(&mut harness, &t!("action-visual-programming"));
    assert!(harness.state().graph.open && !harness.state().macro_editor.open);
    click(&mut harness, &t!("action-macro-editor"));
    assert_eq!(harness.state().macro_editor.text, "kept", "the text is kept");
}

/// A macro typed in the editor runs a step at a time, each one undo step;
/// Step Back undoes the last one.
#[test]
fn the_macro_editor_steps_through_a_macro() {
    let mut state = sample_state();
    state.macro_editor.open = true;
    let (walls, depth) = (wall_count(&state), state.session.doc.undo_depth());
    let mut harness = macro_harness(state);
    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.run();
    for line in TWO_WALLS.lines() {
        harness.get_by_role(Role::MultilineTextInput).type_text(line);
        harness.key_press(egui::Key::Enter);
        harness.run();
    }
    assert_eq!(harness.state().macro_editor.total(), 2, "{:?}", harness.state().macro_editor.text);
    assert!(harness.state().macro_editor.dirty);
    click(&mut harness, &t!("macro-ed-step"));
    click(&mut harness, &t!("macro-ed-step"));
    let s = harness.state();
    assert_eq!((wall_count(s), s.session.doc.undo_depth()), (walls + 2, depth + 2));
    assert_eq!(s.macro_editor.position(), 2);
    assert!(harness.query_by_label(&t!("macro-ed-position", "n" => 2, "total" => 2)).is_some());
    assert!(disabled(&harness, &t!("macro-ed-step")), "no step is left");
    click(&mut harness, &t!("macro-ed-step-back"));
    let s = harness.state();
    assert_eq!((wall_count(s), s.session.doc.undo_depth(), s.macro_editor.position()), (walls + 1, depth + 1, 1));
    // F10 steps too, while the pointer is over the panel.
    harness.state_mut().macro_editor.hovered = true;
    harness.key_press(egui::Key::F10);
    harness.run();
    assert_eq!(wall_count(harness.state()), walls + 2);
}

/// A line that is not a step shows in Problems and stops stepping there.
#[test]
fn an_invalid_line_shows_a_problem_and_stops_stepping() {
    let text = "{\"command\": \"wall_create\", \"args\": {\"start\": [30, 0], \"end\": [36, 0]}}\nnot json\n";
    let mut harness = macro_harness(macro_state(text));
    click(&mut harness, &t!("macro-ed-check"));
    let problems = harness.state().macro_editor.problems().unwrap().to_vec();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].line, Some(2));
    let row = t!("macro-ed-problem", "line" => 2, "message" => problems[0].message.clone());
    assert!(harness.query_by_label(&row).is_some(), "the problem is listed");
    assert!(!disabled(&harness, &t!("macro-ed-step")), "the first step comes before the bad line");
    assert!(disabled(&harness, &t!("macro-ed-run-end")));
    click(&mut harness, &t!("macro-ed-step"));
    assert!(disabled(&harness, &t!("macro-ed-step")), "no step after the bad line");
    // Fixing it lets the run go on; a clean check says so.
    harness.state_mut().macro_editor.load_text(&text.replace("not json", "// a comment"));
    click(&mut harness, &t!("macro-ed-check"));
    assert!(harness.query_by_label(&t!("macro-ed-no-problems")).is_some());
}

/// Editing a step that ran makes the run stale; Restart steps back over it.
#[test]
fn editing_an_executed_step_needs_a_restart() {
    let state = macro_state(TWO_WALLS);
    let walls = wall_count(&state);
    let mut harness = macro_harness(state);
    click(&mut harness, &t!("macro-ed-step"));
    assert_eq!(wall_count(harness.state()), walls + 1);
    // Edits after the position are fine.
    harness.state_mut().macro_editor.load_text(&TWO_WALLS.replace("[36, 5]", "[36, 6]"));
    harness.run();
    assert!(!harness.state().macro_editor.is_stale());
    // Not the step that ran.
    harness.state_mut().macro_editor.load_text(&TWO_WALLS.replace("[30, 0]", "[31, 0]"));
    harness.run();
    assert!(harness.state().macro_editor.is_stale());
    assert!(harness.query_by_label(&t!("macro-ed-stale")).is_some(), "the notice shows");
    assert!(disabled(&harness, &t!("macro-ed-step")));
    assert!(disabled(&harness, &t!("macro-ed-step-back")));
    click(&mut harness, &t!("macro-ed-restart"));
    let s = harness.state();
    assert_eq!((wall_count(s), s.macro_editor.position(), s.macro_editor.is_stale()), (walls, 0, false));
    click(&mut harness, &t!("macro-ed-step"));
    assert_eq!(wall_count(harness.state()), walls + 1, "the edited step runs");
}

/// Run to End with *Single undo step* leaves one undo step, named for the run.
#[test]
fn run_to_end_can_make_one_undo_step() {
    let state = macro_state(TWO_WALLS);
    let (walls, depth) = (wall_count(&state), state.session.doc.undo_depth());
    let mut harness = macro_harness(state);
    harness.get_by_label(&t!("macro-ed-single-undo")).click();
    harness.run();
    assert!(harness.state().macro_editor.single_undo);
    click(&mut harness, &t!("macro-ed-run-end"));
    let s = harness.state();
    assert_eq!((wall_count(s), s.session.doc.undo_depth()), (walls + 2, depth + 1));
    assert_eq!(s.session.doc.undo_name(), Some(t!("macro-ed-run-name").as_str()));
    // Restarting steps back over the run whole.
    click(&mut harness, &t!("macro-ed-restart"));
    assert_eq!(wall_count(harness.state()), walls);
}

/// A restart that cannot undo (the model changed since) asks before starting over.
#[test]
fn a_restart_that_cannot_undo_asks_first() {
    let mut harness = macro_harness(macro_state(TWO_WALLS));
    click(&mut harness, &t!("macro-ed-step"));
    harness.state_mut().session.execute("project_info_set", serde_json::json!({"name": "Changed"})).unwrap();
    click(&mut harness, &t!("macro-ed-restart"));
    assert_eq!(harness.state().macro_editor.position(), 1, "nothing was undone");
    click(&mut harness, &t!("macro-ed-start-over"));
    assert_eq!(harness.state().macro_editor.position(), 0);
    assert_eq!(harness.state().project().info.name, "Changed");
}

/// *Insert Command…* opens the palette on engine commands; the form's
/// Insert adds the step after the cursor's line instead of running it.
#[test]
fn insert_command_adds_a_line() {
    let mut state = macro_state("// first\n// second\n");
    let walls = wall_count(&state);
    crate::palette::open_insert(&mut state);
    let mut harness = palette_harness(state);
    harness.get_by_role(Role::TextInput).type_text("wall_create");
    harness.run();
    assert!(harness.query_by_label(&t!("action-macro-editor")).is_none(), "only engine commands");
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_role(Role::MultilineTextInput).type_text(r#"{"start": [0, 0], "end": [4, 0]}"#);
    harness.run();
    click(&mut harness, &t!("palette-insert"));
    let s = harness.state();
    assert!(s.palette.is_none(), "the palette closes");
    assert_eq!(wall_count(s), walls, "nothing ran");
    let lines: Vec<&str> = s.macro_editor.text.lines().collect();
    assert_eq!(lines.len(), 3, "{}", s.macro_editor.text);
    assert_eq!(lines[0], "// first", "after the cursor's line, the first");
    let step: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(step, serde_json::json!({"command": "wall_create", "args": {"start": [0, 0], "end": [4, 0]}}));
    assert_eq!(s.macro_editor.total(), 1);
}

#[test]
fn sheet_properties_snapshot() {
    if !snapshots_requested() {
        return;
    }
    let mut state = sample_state();
    let sheet = state.project().sheets_in_order()[0];
    state.active_view = Some(sheet);
    let mut harness = themed_harness(state, egui::vec2(360.0, 720.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    harness.snapshot("sheet_properties");
}

/// *Edit Title Block* in a sheet's browser menu opens its title block on that
/// sheet, though another view is active.
#[test]
fn browser_menu_edits_a_sheets_title_block() {
    let mut state = sample_state();
    let sheet = state.project().sheets_in_order()[0];
    let other = state.project().views.keys().copied().find(|&v| v != sheet).unwrap();
    state.active_view = Some(other);
    let label = match state.project().view(sheet).map(|v| (&v.kind, &v.name)) {
        Some((cc_core::ViewKind::Sheet { number, .. }, name)) => format!("{number} - {name}"),
        _ => panic!("a sheet"),
    };
    let mut harness = themed_harness(state, egui::vec2(320.0, 900.0), egui::Theme::Light, |ui, state| {
        crate::browser::show(ui, state)
    });
    harness.get_by_label(&label).click_secondary();
    harness.run();
    harness.get_by_label(&t!("action-edit-title-block")).click();
    harness.run();
    let state = harness.state();
    let ed = state.component_env.as_ref().and_then(|e| e.sheet.as_ref()).expect("the title block editor");
    assert_eq!(ed.sheet, Some(sheet));
    assert_eq!(state.active_view, Some(sheet));
}

/// In the title block editor the ribbon's Text tool and a click on the
/// paper add a text piece and pick it.
#[test]
fn the_text_tool_places_a_text_on_the_sheet() {
    let mut state = sample_state();
    let sheet = state.project().sheets_in_order()[0];
    state.active_view = Some(sheet);
    crate::title_block_env::edit_sheet(&mut state, sheet);
    let before = crate::component_env::component(&state).unwrap().1.sheet.len();
    let mut harness = themed_harness(state, egui::vec2(1100.0, 800.0), egui::Theme::Light, |ui, state| {
        crate::component_env::ribbon(ui, state);
        crate::component_env::central(ui, state);
    });
    harness.run();
    harness.get_by_label(&t!("tb-tool-text")).click();
    harness.run();
    let at = egui::pos2(550.0, 500.0);
    harness.input_mut().events.push(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
    let state = harness.state();
    assert_eq!(crate::component_env::component(state).unwrap().1.sheet.len(), before + 1);
    assert_eq!(state.component_env.as_ref().unwrap().pick, Some(crate::component_env::Pick::Sheet(vec![before])));
}

#[test]
fn title_block_editor_snapshot() {
    if !snapshots_requested() {
        return;
    }
    let mut state = sample_state();
    let sheet = state.project().sheets_in_order()[0];
    state.active_view = Some(sheet);
    crate::title_block_env::edit_sheet(&mut state, sheet);
    let mut harness = themed_harness(state, egui::vec2(1100.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::component_env::ribbon(ui, state);
        crate::component_env::options(ui, state);
        crate::component_env::central(ui, state);
    });
    harness.run();
    harness.snapshot("title_block_editor");
}

/// Space with walls selected flips them (one undo step), and the Flip
/// Walls command does the same.
#[test]
fn space_flips_the_selected_walls() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let wall = state.project().walls().map(|(e, _)| e.id).next().unwrap();
    let ends = |state: &AppState| state.project().element(wall).and_then(|e| e.as_wall()).map(|w| (w.start, w.end));
    let (start, end) = ends(&state).unwrap();
    assert_eq!(UiCommand::FlipWalls.unavailable(&state), Some(t!("why-select-walls")));
    state.selection = [wall].into_iter().collect();
    assert_eq!(UiCommand::FlipWalls.unavailable(&state), None);
    let mut harness = themed_harness(state, egui::vec2(300.0, 200.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
    });
    harness.run();
    let depth = harness.state().session.doc.undo_depth();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert_eq!(ends(harness.state()), Some((end, start)), "flipped");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
    assert_eq!(harness.state().message, t!("msg-walls-flipped"));
    // The command flips it back.
    crate::commands::run(harness.state_mut(), &egui::Context::default(), UiCommand::FlipWalls);
    assert_eq!(ends(harness.state()), Some((start, end)), "flipped back");
}

/// A section's mark in a plan is picked by a click on its head, flipped
/// with Space and moved by dragging its line.
#[test]
fn section_marks_are_picked_flipped_and_moved() {
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    let section_of = |state: &AppState| {
        state.project().views.values().find_map(|v| match v.kind {
            cc_core::ViewKind::Section { start, end, .. } => Some((v.id, start, end)),
            _ => None,
        })
    };
    let (section, start, end) = section_of(&state).expect("the sample's section");
    state.active_view = Some(plan);
    // The app's keys and a canvas of the plan, 20 points per meter around the origin.
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: glam::DVec2::ZERO, scale: 20.0 };
        if let Some(drawing) = state.active_view.and_then(|plan| state.drawing(plan)) {
            crate::section_mark::select(ui, &resp, &painter, state, &xf, &drawing);
        }
    });
    harness.run();
    let screen = |p: glam::DVec2| egui::pos2(300.0 + p.x as f32 * 20.0, 200.0 - p.y as f32 * 20.0);
    let press = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, pressed: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
        h.run();
    };

    press(&mut harness, screen(start), true);
    press(&mut harness, screen(start), false);
    assert_eq!(harness.state().section_selection, Some(section));

    harness.key_press(egui::Key::Space);
    harness.run();
    let (_, s, e) = section_of(harness.state()).unwrap();
    assert_eq!((s, e), (end, start), "flipped");

    // Dragged by the middle of its line, 2 m to the right.
    let mid = (start + end) * 0.5;
    press(&mut harness, screen(mid), true);
    for step in 1..=4 {
        harness.input_mut().events.push(egui::Event::PointerMoved(screen(mid) + egui::vec2(10.0 * step as f32, 0.0)));
        harness.run();
    }
    press(&mut harness, screen(mid) + egui::vec2(40.0, 0.0), false);
    let (_, s, e) = section_of(harness.state()).unwrap();
    let shift = glam::DVec2::new(2.0, 0.0);
    assert!(s.distance(end + shift) < 1e-6 && e.distance(start + shift) < 1e-6, "{s} {e}");
}

/// Issue #149: the Detail Line tool works in views that hold drawings; its
/// curves become selected elements of the view, the style is changed in
/// the properties and the Modify tools take them outside plans.
#[test]
fn detail_lines_are_drawn_in_a_drafting_view() {
    use crate::commands::UiCommand;
    use cc_core::sketch::SketchCurve;
    use glam::DVec2;
    let mut state = sample_state();
    let three_d = state.project().views.values().find(|v| v.category() == cc_core::ViewCategory::ThreeD);
    state.active_view = three_d.map(|v| v.id);
    assert_eq!(UiCommand::DetailLine.unavailable(&state), Some(t!("why-cad-view")));
    let made = crate::actions::run_command(
        &mut state,
        cc_engine::commands::views::ViewCreate { kind: "drafting".into(), ..Default::default() },
    );
    let view = cc_core::ViewId(made.unwrap()["id"].as_u64().unwrap());
    state.active_view = Some(view);
    assert_eq!(UiCommand::DetailLine.unavailable(&state), None);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::DetailLine);
    assert_eq!(state.tool, crate::tools::Tool::DetailLine);

    let curves = [
        SketchCurve::Line { a: DVec2::ZERO, b: DVec2::new(1.0, 0.0) },
        SketchCurve::Circle { center: DVec2::new(3.0, 0.0), radius: 0.5 },
    ];
    crate::detail_ui::create(&mut state, view, cc_core::LineStyleRef::BuiltIn(cc_core::DetailStyle::Dashed), &curves);
    assert_eq!(state.selection.len(), 2, "{}", state.message);
    let first = *state.selection.iter().next().unwrap();
    let Some(cc_core::ElementKind::DetailLine(l)) = state.project().element(first).map(|e| &e.kind) else {
        panic!("a detail line")
    };
    assert_eq!((l.view, l.style), (view, cc_core::LineStyleRef::BuiltIn(cc_core::DetailStyle::Dashed)));

    // Moving them is allowed in the drafting view: they belong to it.
    crate::modify::start(&mut state, crate::tools::Tool::Move);
    assert!(crate::detail_ui::view_owned(&state, &crate::tools::Tool::Move));
}

/// Issue #150: Filled Region and Masking Region sketch in a drafting view
/// and *Finish* makes the region there; *Edit Boundary* reopens it; the
/// Line Styles dialog adds a style (Cancel undoes it, OK keeps it) that the
/// Detail Line tool and the region's boundary can use.
#[test]
fn regions_are_sketched_and_line_styles_edited() {
    use crate::commands::UiCommand;
    use crate::sketch_mode::{Target, finish};
    use cc_core::sketch::SketchCurve;
    use glam::DVec2;
    let mut state = sample_state();
    let three_d = state.project().views.values().find(|v| v.category() == cc_core::ViewCategory::ThreeD);
    state.active_view = three_d.map(|v| v.id);
    assert_eq!(UiCommand::FilledRegion.unavailable(&state), Some(t!("why-cad-view")));
    let made = crate::actions::run_command(
        &mut state,
        cc_engine::commands::views::ViewCreate { kind: "drafting".into(), ..Default::default() },
    );
    let view = cc_core::ViewId(made.unwrap()["id"].as_u64().unwrap());
    state.active_view = Some(view);
    for command in [UiCommand::FilledRegion, UiCommand::MaskingRegion] {
        assert_eq!(command.unavailable(&state), None);
        crate::commands::run(&mut state, &egui::Context::default(), command);
        assert!(command.selected(&state));
        let square = |o: DVec2, s: f64| {
            let p = [o, o + DVec2::new(s, 0.0), o + DVec2::new(s, s), o + DVec2::new(0.0, s)];
            (0..4).map(|i| SketchCurve::Line { a: p[i], b: p[(i + 1) % 4] }).collect::<Vec<_>>()
        };
        let mut curves = square(DVec2::ZERO, 4.0);
        curves.extend(square(DVec2::ONE, 1.0));
        state.sketch.as_mut().unwrap().curves = curves;
        finish(&mut state);
        assert!(state.sketch.is_none(), "{}", state.message);
        let id = *state.selection.iter().next().unwrap();
        let e = state.project().element(id).unwrap();
        assert_eq!(e.owner_view(), Some(view));
        assert!(matches!(&e.kind,
            cc_core::ElementKind::FilledRegion(r) | cc_core::ElementKind::MaskingRegion(r) if r.openings.len() == 1));
        // Edit Boundary opens its sketch with its curves.
        assert_eq!(UiCommand::EditBoundary.unavailable(&state), None);
        crate::commands::run(&mut state, &egui::Context::default(), UiCommand::EditBoundary);
        let s = state.sketch.as_ref().unwrap();
        assert_eq!((s.target, s.curves.len()), (Target::Edit { id }, 8));
        crate::sketch_mode::cancel(&mut state);
    }

    // The Line Styles dialog: Cancel undoes a new style, OK keeps it.
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::LineStyles);
    assert!(state.dialogs.line_styles.is_some());
    let create = cc_engine::commands::line_style::LineStyleCreate { name: "Mine".into(), ..Default::default() };
    crate::actions::run_command(&mut state, create);
    assert_eq!(state.project().line_styles.len(), 1);
    if let Some(d) = state.dialogs.line_styles.take() {
        d.edits_finish(&mut state, false);
    }
    assert!(state.project().line_styles.is_empty());
    crate::line_styles_dialog::open(&mut state);
    let create = cc_engine::commands::line_style::LineStyleCreate { name: "Mine".into(), ..Default::default() };
    crate::actions::run_command(&mut state, create);
    if let Some(d) = state.dialogs.line_styles.take() {
        d.edits_finish(&mut state, true);
    }
    let style = *state.project().line_styles.keys().next().unwrap();
    // Detail lines take it.
    let curves = [SketchCurve::Line { a: DVec2::ZERO, b: DVec2::new(1.0, 0.0) }];
    crate::detail_ui::create(&mut state, view, cc_core::LineStyleRef::Project(style), &curves);
    let line = *state.selection.iter().next().unwrap();
    assert!(matches!(&state.project().element(line).unwrap().kind,
        cc_core::ElementKind::DetailLine(l) if l.style == cc_core::LineStyleRef::Project(style)));
}

/// The Callout tool makes a detail view from two clicks in a plan; the
/// callout's mark is picked, resized by a corner (the detail's crop) and
/// opens its view on a double-click.
#[test]
fn callouts_are_drawn_resized_and_opened() {
    let mut state = sample_state();
    let plan = state.project().views.values().find(|v| v.level().is_some()).map(|v| v.id).unwrap();
    state.active_view = Some(plan);
    state.set_tool(crate::tools::Tool::Callout);
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: glam::DVec2::ZERO, scale: 20.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        if state.tool == crate::tools::Tool::Callout {
            crate::callout_ui::tool(ui, &resp, &painter, state, &view, &xf);
        } else if let Some(drawing) = state.drawing(view.id) {
            crate::callout_ui::select(ui, &resp, &painter, state, &xf, &drawing);
        }
    });
    harness.run();
    let screen = |p: glam::DVec2| egui::pos2(300.0 + p.x as f32 * 20.0, 200.0 - p.y as f32 * 20.0);
    let press = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, pressed: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
        h.run();
    };
    let click = |h: &mut Harness<'static, AppState>, p: glam::DVec2| {
        press(h, screen(p), true);
        press(h, screen(p), false);
    };
    // Far from the model so nothing snaps: corners (-12, -8) and (-8, -5).
    click(&mut harness, glam::DVec2::new(-12.0, -8.0));
    click(&mut harness, glam::DVec2::new(-8.0, -5.0));
    let detail = harness.state().section_selection.expect("the new detail's callout, selected");
    let crop = |state: &AppState| state.project().view(detail).and_then(|v| v.crop).unwrap();
    assert_eq!(
        crop(harness.state()),
        cc_core::CropBox::new(glam::DVec2::new(-12.0, -8.0), glam::DVec2::new(-8.0, -5.0))
    );
    assert_eq!(harness.state().project().view(detail).unwrap().category(), cc_core::ViewCategory::FloorPlan);

    // Drag the upper right corner 1 m right and 1 m up.
    harness.state_mut().set_tool(crate::tools::Tool::Select);
    harness.run();
    let corner = screen(glam::DVec2::new(-8.0, -5.0));
    press(&mut harness, corner, true);
    for step in 1..=4 {
        harness.input_mut().events.push(egui::Event::PointerMoved(corner + egui::vec2(5.0, -5.0) * step as f32));
        harness.run();
    }
    press(&mut harness, corner + egui::vec2(20.0, -20.0), false);
    assert_eq!(crop(harness.state()).max, glam::DVec2::new(-7.0, -4.0));

    // A double-click on a side opens the detail view (the harness clock
    // runs faster than a hand, so allow a slow double click).
    harness.ctx.options_mut(|o| o.input_options.max_double_click_delay = 60.0);
    let side = screen(glam::DVec2::new(-12.0, -6.0));
    for _ in 0..2 {
        press(&mut harness, side, true);
        press(&mut harness, side, false);
    }
    assert_eq!(harness.state().open_requests.last(), Some(&detail));
}

/// While a file is read on the worker thread, a dialog shows what is read
/// and blocks the interface; Cancel leaves the project as it was.
#[test]
fn reading_a_file_shows_the_progress_dialog() {
    let mut state = sample_state();
    let path = std::env::temp_dir().join("cc_progress_dialog.dwg");
    std::fs::write(&path, vec![0u8; 3000]).unwrap();
    let (release, wait) = std::sync::mpsc::channel::<()>();
    let run = move |_: &mut cc_engine::Session| {
        let _ = wait.recv();
        Ok(serde_json::Value::Null)
    };
    let finish: crate::jobs::Finish = Box::new(|state, _, _| state.message = "finished".into());
    let title = t!("job-importing-cad");
    crate::jobs::start(&mut state, title.clone(), path.clone(), crate::jobs::Base::Current, run, finish);
    // The spinner keeps asking for frames, so the harness steps by hand.
    let mut themed = false;
    let mut harness = Harness::builder().with_size(egui::vec2(800.0, 500.0)).build_ui_state(
        move |ui, state: &mut AppState| {
            if !themed {
                crate::theme::apply(ui.ctx());
                themed = true;
                return;
            }
            crate::theme::sync(ui.ctx());
            crate::jobs::poll(state);
            crate::jobs::show(ui.ctx(), state);
        },
        state,
    );
    harness.run_steps(3);
    harness.get_by_label(&title);
    harness.get_by_label(&t!("job-reading", "name" => "cc_progress_dialog.dwg"));
    assert!(crate::jobs::busy(harness.state()));
    // Cancel: the job's answer is dropped when it comes.
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run_steps(2);
    assert!(!crate::jobs::busy(harness.state()));
    let _ = release.send(());
    harness.run_steps(2);
    assert_ne!(harness.state().message, "finished");
    let _ = std::fs::remove_file(path);
}

/// Opening a project reads it on the worker thread and then replaces the document.
#[test]
fn opening_a_project_runs_as_a_job() {
    let path = std::env::temp_dir().join("cc_job_open.ccad");
    let mut session =
        cc_engine::Session::new(cc_core::Document::new(crate::home::sample_project("casa_simples").unwrap()));
    session.run(cc_engine::commands::project::ProjectSave { path: Some(path.clone()) }).unwrap();
    let walls = session.doc.project.elements.len();
    let mut state = AppState::new(cc_core::Document::new(crate::actions::new_project()));
    crate::actions::open_path(&mut state, path.clone());
    assert!(crate::jobs::busy(&state));
    crate::jobs::wait(&mut state);
    assert_eq!(state.project().elements.len(), walls);
    assert_eq!(state.session.doc.path.as_deref(), Some(path.as_path()));
    let _ = std::fs::remove_file(path);
}

/// The Project Browser and Properties dock on either side, stacked or in
/// columns of their own: dragged by their headers, or from the header's menu.
#[test]
fn side_panels_dock_by_dragging_their_headers() {
    use crate::side_panels::{Pane, PanelLayout};
    let mut harness = themed_harness(sample_state(), egui::vec2(1200.0, 800.0), egui::Theme::Light, |ui, state| {
        crate::side_panels::show(ui, state);
        egui::CentralPanel::default().show(ui, |ui| ui.label("views"));
    });
    let title = |h: &Harness<'static, AppState>, pane: Pane| h.get_by_label(&pane.title()).rect();
    assert!(title(&harness, Pane::Browser).center().x < 300.0, "both on the left");
    assert!(title(&harness, Pane::Properties).center().x < 300.0);

    let press = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, pressed: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
        h.run();
    };
    let drag = |h: &mut Harness<'static, AppState>, from: egui::Pos2, to: egui::Pos2| {
        press(h, from, true);
        for step in 1..=6 {
            h.input_mut().events.push(egui::Event::PointerMoved(from + (to - from) * step as f32 / 6.0));
            h.run();
        }
        press(h, to, false);
        h.run();
    };
    // Properties to the window's right edge: one panel on each side.
    let from = title(&harness, Pane::Properties).center();
    drag(&mut harness, from, egui::pos2(1195.0, 400.0));
    assert_eq!(harness.state().prefs.panels.right, vec![vec![Pane::Properties]]);
    assert!(title(&harness, Pane::Properties).center().x > 900.0);
    // The browser onto the top of Properties: stacked on the right.
    let from = title(&harness, Pane::Browser).center();
    let props = harness.get_by_label(&Pane::Properties.title()).rect();
    drag(&mut harness, from, egui::pos2(props.center().x, props.top() + 30.0));
    let stacked = PanelLayout { left: vec![], right: vec![vec![Pane::Browser, Pane::Properties]] };
    assert_eq!(harness.state().prefs.panels, stacked);
    let (b, p) = (title(&harness, Pane::Browser), title(&harness, Pane::Properties));
    assert!(b.center().x > 900.0 && b.center().y < p.center().y, "browser above properties, on the right");

    // The header's menu separates them, and resets the layout.
    let mut state = sample_state();
    state.prefs.panels = stacked;
    let mut menu =
        Harness::new_ui_state(|ui, state: &mut AppState| _ = crate::side_panels::menu(ui, state, Pane::Browser), state);
    menu.get_by_label(&t!("panel-separate-from", "panel" => Pane::Properties.title())).click();
    menu.run();
    assert_eq!(menu.state().prefs.panels.right, vec![vec![Pane::Properties], vec![Pane::Browser]]);
    menu.get_by_label(&t!("panel-reset-layout")).click();
    menu.run();
    assert_eq!(menu.state().prefs.panels, PanelLayout::default());
}

/// Enter in a dialog's text field confirms the field, not the dialog: a
/// single-line field gives up the focus in the frame Enter is pressed in,
/// and the Site Location search closed the window. With no field focused,
/// Enter still presses the main action.
#[test]
fn enter_in_a_dialog_text_field_does_not_press_ok() {
    #[derive(Default)]
    struct Probe {
        text: String,
        ok: usize,
        themed: bool,
    }
    let mut harness = Harness::builder().with_size(egui::vec2(600.0, 400.0)).build_ui_state(
        |ui, p: &mut Probe| {
            let ctx = &ui.ctx().clone();
            if !p.themed {
                crate::theme::apply(ctx);
                p.themed = true;
                return;
            }
            crate::theme::sync(ctx);
            let mut ok = 0;
            let text = &mut p.text;
            let r = crate::dialog::Dialog::new("enter-probe", "Probe").show(
                ctx,
                |ui| {
                    ui.add(egui::TextEdit::singleline(text).hint_text("search"));
                },
                |f| ok = f.primary("OK"),
            );
            if r.clicked == Some(ok) {
                p.ok += 1;
            }
        },
        Probe::default(),
    );
    harness.run();
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("Fortaleza");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().text, "Fortaleza");
    assert_eq!(harness.state().ok, 0, "Enter in the field did not press OK");
    // The field has let go of the focus: Enter now presses OK.
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().ok, 1);
}

/// What the dialog layout tests show: one dialog, while `open`.
struct DialogProbe {
    themed: bool,
    open: bool,
}

/// A harness that shows the dialog `show` builds, every frame.
fn dialog_probe(mut show: impl FnMut(&egui::Context) + 'static) -> Harness<'static, DialogProbe> {
    let mut harness = Harness::builder().with_size(egui::vec2(900.0, 600.0)).build_ui_state(
        move |ui, p: &mut DialogProbe| {
            let ctx = &ui.ctx().clone();
            if !p.themed {
                crate::theme::apply(ctx);
                p.themed = true;
                return;
            }
            crate::theme::sync(ctx);
            if p.open {
                show(ctx);
            }
        },
        DialogProbe { themed: false, open: true },
    );
    harness.run();
    harness
}

/// A dialog is moved by dragging its header, stays where it was put, and
/// opens centered again the next time.
#[test]
fn a_dialog_moves_by_its_header() {
    let mut harness = dialog_probe(|ctx| {
        crate::dialog::Dialog::new("move-probe", "Probe").show(
            ctx,
            |ui| {
                ui.label("Body");
            },
            |f| {
                f.primary("OK");
            },
        );
    });
    let start = harness.get_by_label("Probe").rect();
    let by = egui::vec2(60.0, 40.0);
    // Pressed on the title itself.
    harness.drag_at(start.center());
    harness.step();
    for k in 1..=4 {
        harness.hover_at(start.center() + by * (k as f32 / 4.0));
        harness.step();
    }
    harness.drop_at(start.center() + by);
    harness.run();
    let moved = harness.get_by_label("Probe").rect();
    assert!((moved.min - start.min - by).length() < 1.5, "moved by {:?}", moved.min - start.min);
    // The body and the footer went along.
    harness.run();
    assert!((harness.get_by_label("Probe").rect().min - moved.min).length() < 0.5);

    harness.state_mut().open = false;
    harness.run();
    harness.state_mut().open = true;
    harness.run();
    let again = harness.get_by_label("Probe").rect();
    assert!((again.min - start.min).length() < 1.5, "reopened at {:?}, not {:?}", again.min, start.min);
}

/// Footer buttons that do not fit the dialog's width widen it: they never
/// lie over each other, and the header follows.
#[test]
fn dialog_footer_buttons_never_overlap() {
    let labels = ["Ignore this version", "Update now", "Download", "Later"];
    let mut harness = dialog_probe(move |ctx| {
        crate::dialog::Dialog::new("footer-probe", "Probe").width(200.0).show(
            ctx,
            |ui| {
                ui.label("Body");
            },
            |f| {
                f.extra(labels[0]);
                f.primary(labels[1]);
                f.secondary(labels[2]);
                f.secondary(labels[3]);
            },
        );
    });
    harness.run();
    let rects: Vec<egui::Rect> = labels.iter().map(|l| harness.get_by_label(l).rect()).collect();
    for pair in rects.windows(2) {
        assert!(pair[0].right() + 7.0 < pair[1].left(), "{:?} lies over {:?}", pair[0], pair[1]);
    }
    let close = harness.get_by_label(&crate::t!("action-close")).rect();
    assert!(
        close.right() > rects[3].right() - 12.0,
        "the header ends at {}, the footer at {}",
        close.right(),
        rects[3].right()
    );
}

/// A body wider than the dialog takes the header and the footer with it.
#[test]
fn dialog_header_and_footer_follow_a_wide_body() {
    crate::dialog::overflow::allow();
    let mut harness = dialog_probe(|ctx| {
        crate::dialog::Dialog::new("wide-probe", "Probe").width(300.0).show(
            ctx,
            |ui| {
                ui.add(egui::Button::new("Wide").min_size(egui::vec2(500.0, 20.0)));
            },
            |f| {
                f.primary("OK");
            },
        );
    });
    harness.run();
    let wide = harness.get_by_label("Wide").rect();
    let ok = harness.get_by_label("OK").rect();
    let close = harness.get_by_label(&crate::t!("action-close")).rect();
    assert!((ok.right() - wide.right()).abs() < 1.5, "OK ends at {}, the body at {}", ok.right(), wide.right());
    assert!(close.right() > wide.right(), "the close button stayed at {}", close.right());
}

/// A value too wide for its table cell is cut off there and does not widen
/// the dialog (the dialog itself fails the test when its body overflows).
#[test]
fn a_table_row_never_widens_its_dialog() {
    let mut harness = dialog_probe(|ctx| {
        crate::dialog::Dialog::new("row-probe", "Probe").width(300.0).show(
            ctx,
            |ui| {
                crate::table::row(ui, "Definition", |ui| {
                    let _ = ui.button("Edit the component in its own environment");
                    let _ = ui.button("Save the component to a file of the library");
                });
                crate::table::row(ui, "Next", |ui| {
                    ui.label("value");
                });
            },
            |f| {
                f.primary("OK");
            },
        );
    });
    harness.run();
    let ok = harness.get_by_label("OK").rect();
    let title = harness.get_by_label("Probe").rect();
    assert!((ok.right() - title.left() - 300.0).abs() < 1.5, "the dialog is {} wide", ok.right() - title.left());
}

/// Every dialog keeps its body inside its width (a dialog fails the test
/// when its body overflows): the plain ones, Type Properties of every type
/// of the sample project, and each section of the Preferences.
#[test]
fn every_dialog_keeps_its_body_inside() {
    use crate::preferences::Section;
    type Open = Box<dyn Fn(&mut AppState)>;
    type Make = fn() -> AppState;
    let mut opens: Vec<(String, Open)> = vec![
        ("project info".into(), Box::new(|s| s.dialogs.project_info = true)),
        ("units".into(), Box::new(|s| s.dialogs.units = true)),
        ("levels".into(), Box::new(|s| s.dialogs.levels = true)),
        ("materials".into(), Box::new(|s| s.dialogs.materials = true)),
        ("about".into(), Box::new(|s| s.dialogs.about = true)),
        ("systems".into(), Box::new(|s| s.dialogs.systems = true)),
        ("sizing".into(), Box::new(|s| s.dialogs.sizing = true)),
        ("lightning check".into(), Box::new(|s| s.dialogs.lps_check = true)),
        ("cabling".into(), Box::new(|s| s.dialogs.cabling = true)),
        ("error".into(), Box::new(|s| s.dialogs.error = Some("The file could not be read. ".repeat(12)))),
        ("design options".into(), Box::new(crate::design_options_ui::open)),
        ("browser organization".into(), Box::new(crate::browser_org_dialog::open)),
        ("line styles".into(), Box::new(crate::line_styles_dialog::open)),
        ("tag types".into(), Box::new(crate::tag_type_ui::open_list)),
        ("fill patterns".into(), Box::new(crate::fill_patterns_dialog::open)),
        ("pipe catalog".into(), Box::new(crate::pipe_catalog_dialog::open)),
        ("material library".into(), Box::new(crate::material_library_dialog::open)),
        ("phases".into(), Box::new(crate::phases_dialog::open)),
        ("filters".into(), Box::new(crate::graphics_dialog::open_filters)),
        ("view templates".into(), Box::new(crate::graphics_dialog::open_templates)),
        ("new project".into(), Box::new(crate::new_project_dialog::open)),
        ("new component".into(), Box::new(crate::new_component_dialog::open)),
        ("component library".into(), Box::new(crate::library::open)),
        (
            "gltf export".into(),
            Box::new(|s| s.dialogs.gltf_export = Some(std::path::PathBuf::from("a rather long project name.glb"))),
        ),
        ("obj import".into(), Box::new(|s| s.dialogs.obj_import = Some(std::path::PathBuf::from("a model.obj")))),
        ("gltf import".into(), Box::new(|s| s.dialogs.obj_import = Some(std::path::PathBuf::from("a model.glb")))),
        ("grid frame".into(), Box::new(crate::grid_frame_ui::open)),
        ("steel building".into(), Box::new(crate::steel_building_ui::open)),
        (
            "wall framing".into(),
            Box::new(|s| crate::wall_framing_ui::open(s, cc_core::wall_framing::FramingSystem::Wood)),
        ),
        ("macro restart".into(), Box::new(|s| crate::macro_ui::ask(s, true))),
        ("macro discard".into(), Box::new(|s| crate::macro_ui::ask(s, false))),
        (
            "render".into(),
            Box::new(|s| {
                open_render_window(s, None);
            }),
        ),
    ];
    for section in Section::ALL {
        opens.push((format!("preferences {section:?}"), Box::new(move |s| crate::preferences::open(s, section))));
    }
    // A plugin command's form with every kind of field (a wall selected), as
    // JSON, a result with long lines, and an error.
    let plugins = form_plugins_folder("layout");
    type Then = fn(&mut AppState);
    let forms: [(&str, &'static str, Then); 4] = [
        ("plugin form", "form_tools__layout", |_| {}),
        ("plugin form json", "form_tools__layout", |s| {
            if let Some(form) = s.dialogs.plugin_form.as_mut() {
                form.toggle_json(&crate::plugin_form::Fmt::current());
            }
        }),
        ("plugin form result", "form_tools__report", |_| {}),
        ("plugin form error", "form_tools__fail", crate::plugin_form::run_open),
    ];
    for (name, command, then) in forms {
        let plugins = plugins.clone();
        opens.push((
            name.into(),
            Box::new(move |s| {
                s.prefs.plugin_folder = Some(plugins.clone());
                crate::plugins_ui::load(s);
                let wall = s.project().walls().map(|(e, _)| e.id).next();
                s.selection.extend(wall);
                crate::plugin_form::open(s, command);
                then(s);
            }),
        ));
    }
    // A plugin command waiting for its program, and the Plugins window
    // showing a plugin's external programs.
    let sim = sim_plugins_folder("layout");
    let sim_job = sim.clone();
    opens.push((
        "plugin job".into(),
        Box::new(move |s| {
            s.prefs.plugins_tools.insert("sim".into());
            s.prefs.plugin_folder = Some(sim_job.clone());
            crate::plugins_ui::load(s);
            crate::plugin_form::open(s, "sim__simulate");
            assert!(s.session.has_job(), "{}", s.message);
        }),
    ));
    opens.push((
        "plugins tools".into(),
        Box::new(move |s| {
            s.prefs.plugins_tools.insert("sim".into());
            s.prefs.plugin_folder = Some(sim.clone());
            crate::plugins_ui::load(s);
            crate::plugins_ui::open(s);
            s.plugins.selected = Some("sim".into());
        }),
    ));
    // The Plugins window with a plugin's settings shown.
    let quick_rooms = quick_rooms_folder("layout");
    opens.push((
        "plugins settings".into(),
        Box::new(move |s| {
            s.prefs.plugin_folder = Some(quick_rooms.clone());
            crate::plugins_ui::load(s);
            crate::plugins_ui::open(s);
            s.plugins.selected = Some("quick_rooms".into());
        }),
    ));
    // Sun Settings in each kind of sun, and in a 3D view.
    for (mode, three_d) in
        [("still", false), ("single_day", false), ("multi_day", false), ("lighting", false), ("multi_day", true)]
    {
        opens.push((format!("sun settings {mode} 3d={three_d}"), Box::new(move |s| open_sun_dialog(s, mode, three_d))));
    }
    // The schedule definition, a page at a time, with a filter, sorting levels and a column format to lay out.
    for tab in crate::schedule_ui::Tab::ALL {
        opens.push((
            format!("schedule {tab:?}"),
            Box::new(move |s| {
                let schedule = |v: &&cc_core::View| matches!(v.kind, cc_core::ViewKind::Schedule { .. });
                let Some(view) = s.project().views.values().find(schedule).map(|v| v.id) else { return };
                let Some(cc_core::ViewKind::Schedule { category, def }) = s.project().view(view).map(|v| v.kind.clone())
                else {
                    return;
                };
                let fields: Vec<serde_json::Value> =
                    def.fields_for(category).iter().map(|f| serde_json::to_value(f).unwrap()).collect();
                let args = serde_json::json!({
                    "view": view.0,
                    "filters": [{"field": fields[0], "op": "not_begins_with", "value": "A rather long value to filter by"}],
                    "sorting": [
                        {"field": fields[0], "descending": true, "header": true, "footer": true, "blank_line": true},
                        {"field": fields[1], "header": true},
                    ],
                    "columns": [{"field": fields[0], "heading": "A heading of some length", "align": "Center"}],
                });
                s.session.execute("schedule_set", args).unwrap();
                crate::schedule_ui::open(s, view, tab);
            }),
        ));
    }
    let types: Vec<(cc_core::ElementTypeId, String)> =
        sample_state().project().types.values().map(|t| (t.id, t.name.clone())).collect();
    for (id, name) in types {
        opens.push((
            format!("type {name}"),
            Box::new(move |s| {
                s.dialogs.type_editor = Some(id);
                s.dialogs.type_layers_open = true;
            }),
        ));
    }
    // Type Properties of a tag type of each target and of a view mark type of each kind.
    let tag_kinds = cc_core::tag::TagTarget::ALL.map(|t| ("tag_type_create", "target", t.code()));
    let mark_kinds = cc_core::view_mark::MarkKind::ALL.map(|k| ("view_mark_type_create", "kind", k.code()));
    for (command, field, code) in tag_kinds.into_iter().chain(mark_kinds) {
        opens.push((
            format!("type {command} {code}"),
            Box::new(move |s| {
                let made = s.session.execute(command, serde_json::json!({ field: code })).unwrap();
                s.dialogs.type_editor = made["id"].as_u64().map(cc_core::ElementTypeId);
            }),
        ));
    }
    let mut runs: Vec<(String, Open, Make)> =
        opens.into_iter().map(|(name, open)| (name, open, sample_state as Make)).collect();
    // Paper sizes read in inches in imperial projects: the dialogs showing them, again.
    let imperial: Vec<(String, Open)> = vec![
        ("line styles".into(), Box::new(crate::line_styles_dialog::open)),
        ("fill patterns".into(), Box::new(crate::fill_patterns_dialog::open)),
        ("filters".into(), Box::new(crate::graphics_dialog::open_filters)),
        ("view templates".into(), Box::new(crate::graphics_dialog::open_templates)),
    ];
    runs.extend(imperial.into_iter().map(|(name, open)| (format!("{name} (imperial)"), open, imperial_state as _)));
    let annotations = [cc_core::Category::Dimension, cc_core::Category::Text, cc_core::Category::Grid];
    for t in imperial_state().project().types.values().filter(|t| annotations.contains(&t.category())) {
        let id = t.id;
        let open: Open = Box::new(move |s| s.dialogs.type_editor = Some(id));
        runs.push((format!("type {} (imperial)", t.name), open, imperial_state));
    }
    for (name, open, make) in runs {
        let mut state = make();
        open(&mut state);
        let draw: fn(&mut egui::Ui, &mut AppState) = |ui, state| crate::dialogs::show(ui.ctx(), state);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Some show a spinner (a plugin command waiting for its program).
            let mut harness = spinning_harness(state, egui::vec2(1500.0, 1000.0), draw);
            for _ in 0..6 {
                harness.step();
            }
        }));
        assert!(result.is_ok(), "the dialog \"{name}\" overflows");
    }
}

/// A new imperial project, its units in force (as `AppState::sync` does each frame).
fn imperial_state() -> AppState {
    let project = cc_engine::new_project_with(cc_core::MeasurementSystem::Imperial);
    let mut state = AppState::new(cc_core::Document::new(project));
    state.project_open = true;
    state.show_home = false;
    state.sync();
    state
}

/// In an imperial project a dimension type's text size reads in inches,
/// and a bare number typed there is inches.
#[test]
fn paper_sizes_read_in_inches_in_imperial_projects() {
    let mut state = imperial_state();
    let dimension = state.project().default_type(cc_core::Category::Dimension).unwrap();
    state.dialogs.type_editor = Some(dimension);
    let mut harness = type_dialog_harness(state);
    harness.run();
    let size = |h: &Harness<'static, AppState>| match h.state().project().element_type(dimension).map(|t| &t.data) {
        Some(cc_core::TypeData::Dimension { text_size, .. }) => *text_size,
        other => panic!("not a dimension type: {other:?}"),
    };
    assert!((size(&harness) - 3.0 / 32.0 * 25.4).abs() < 1e-9);
    let values: Vec<String> = harness.query_all_by_role(Role::SpinButton).filter_map(|n| n.value()).collect();
    assert!(values.iter().all(|v| !v.contains("mm")), "{values:?}");
    harness.query_all_by_role(Role::SpinButton).find(|n| n.value().as_deref() == Some("3/32\"")).unwrap().focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text("1/8");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!((size(&harness) - 3.175).abs() < 1e-9, "{}", size(&harness));
    assert!(harness.query_all_by_role(Role::SpinButton).any(|n| n.value().as_deref() == Some("1/8\"")));
}

/// In an imperial project a pipe type's diameter reads in inches without
/// feet and a bare number typed is inches; a stair type's riser too; the
/// schedule field labels name no metric unit; and the tools' sizes follow
/// the project's system.
#[test]
fn small_sizes_read_in_inches_in_imperial_projects() {
    let mut state = imperial_state();
    let inch = cc_core::units::INCH;
    let pipe = state.project().types.values().find(|t| matches!(t.data, cc_core::TypeData::Pipe { .. })).unwrap().id;
    // A free pipe type (a size of a series keeps its row's diameter).
    state.session.doc.edit("free", |e| {
        if let Some(cc_core::TypeData::Pipe { diameter, series, size, .. }) = e.type_mut(pipe).map(|t| &mut t.data) {
            *diameter = 0.5 * inch;
            *series = None;
            size.clear();
        }
    });
    state.dialogs.type_editor = Some(pipe);
    let mut harness = type_dialog_harness(state);
    harness.run();
    let diameter = |h: &Harness<'static, AppState>| h.state().project().element_type(pipe).and_then(|t| t.diameter());
    assert_eq!(diameter(&harness), Some(0.5 * inch));
    let values: Vec<String> = harness.query_all_by_role(Role::SpinButton).filter_map(|n| n.value()).collect();
    assert!(values.iter().any(|v| v == "1/2\""), "{values:?}");
    harness.query_all_by_role(Role::SpinButton).find(|n| n.value().as_deref() == Some("1/2\"")).unwrap().focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text("3/4");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(diameter(&harness).is_some_and(|d| (d - 0.75 * inch).abs() < 1e-12), "{:?}", diameter(&harness));

    let mut state = imperial_state();
    let stair = state.project().default_type(cc_core::Category::Stair).unwrap();
    state.dialogs.type_editor = Some(stair);
    let mut harness = type_dialog_harness(state);
    harness.run();
    let values: Vec<String> = harness.query_all_by_role(Role::SpinButton).filter_map(|n| n.value()).collect();
    // The riser and the tread, in inches; the width, in feet and inches.
    for want in ["7 3/4\"", "10\"", "3'-0\""] {
        assert!(values.iter().any(|v| v == want), "{want}: {values:?}");
    }
    assert!(values.iter().all(|v| !v.contains("mm") && !v.contains(" m")), "{values:?}");

    let state = imperial_state();
    use cc_core::ScheduleField as F;
    for field in [F::Length, F::Thickness, F::Area, F::Perimeter, F::Volume, F::Height] {
        let label = crate::properties::field_label(field);
        assert!(!label.contains("(m") && !label.contains("m²") && !label.contains("m³"), "{label}");
    }
    // New walls are 10'-0" tall, the offset 3'-0"; a metric project brings 3 m back.
    let foot = cc_core::units::FOOT;
    assert!((state.wall_height - 10.0 * foot).abs() < 1e-9 && (state.modify.offset - 3.0 * foot).abs() < 1e-9);
    let mut state = state;
    state.replace_document(cc_core::Document::new(cc_engine::new_project_with(cc_core::MeasurementSystem::Metric)));
    state.sync();
    assert_eq!((state.wall_height, state.modify.offset), (3.0, 1.0));
}

/// New asks for the template first: Cancel keeps the open project, the
/// Imperial card and Create make an imperial project and remember the
/// choice, and with unsaved changes the question about them comes after
/// Create, the project still in place.
#[test]
fn new_asks_for_the_template() {
    use cc_core::MeasurementSystem;
    let state = sample_state();
    let uid = state.project().info.uid;
    let mut harness = themed_harness(state, egui::vec2(1000.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state);
    });
    let new = |h: &mut Harness<'static, AppState>| {
        crate::commands::run(h.state_mut(), &egui::Context::default(), crate::commands::UiCommand::New);
        h.run();
    };

    new(&mut harness);
    assert_eq!(harness.state().dialogs.new_project, Some(MeasurementSystem::Metric));
    assert_eq!(harness.state().project().info.uid, uid, "nothing replaced yet");
    let label = t!("new-project-metric");
    assert_eq!(harness.get_by_label(&label).accesskit_node().role(), Role::RadioButton);
    harness.get_by_role_and_label(Role::Button, &t!("action-cancel")).click();
    harness.run();
    assert_eq!(harness.state().dialogs.new_project, None);
    assert_eq!(harness.state().project().info.uid, uid, "Cancel keeps the project");

    new(&mut harness);
    harness.get_by_label(&t!("new-project-imperial")).click();
    harness.run();
    assert_eq!(harness.state().dialogs.new_project, Some(MeasurementSystem::Imperial));
    harness.get_by_role_and_label(Role::Button, &t!("new-project-create")).click();
    harness.run();
    let state = harness.state();
    assert_eq!(state.dialogs.new_project, None);
    assert_ne!(state.project().info.uid, uid, "a new project");
    assert_eq!(state.project().measurement_system(), MeasurementSystem::Imperial);
    assert_eq!(state.prefs.new_project_system, MeasurementSystem::Imperial, "remembered");
    let imperial = state.project().info.uid;

    // Unsaved changes: the template first, then the question about them.
    let set = cc_engine::commands::project::ProjectInfoSet {
        name: Some("Changed".into()),
        number: None,
        client: None,
        address: None,
        author: None,
        fields: None,
    };
    crate::actions::run_command(harness.state_mut(), set);
    assert!(harness.state().session.doc.is_dirty());
    new(&mut harness);
    assert_eq!(harness.state().dialogs.new_project, Some(MeasurementSystem::Imperial), "the last choice first");
    harness.get_by_label(&t!("new-project-metric")).click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let state = harness.state();
    assert_eq!(state.dialogs.new_project, None);
    assert_eq!(state.dialogs.confirm, Some(crate::state::PendingAction::New(MeasurementSystem::Metric)));
    assert_eq!(state.project().info.uid, imperial, "kept until the changes are answered");
}

/// Issue #159: the Detail Line tool takes typed values after the first
/// point, like the sketches: a circle's radius, a line's length; Enter
/// places them and Esc drops the value first.
#[test]
fn detail_lines_take_typed_lengths() {
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let made = crate::actions::run_command(
        &mut state,
        cc_engine::commands::views::ViewCreate { kind: "drafting".into(), ..Default::default() },
    );
    let view = cc_core::ViewId(made.unwrap()["id"].as_u64().unwrap());
    state.active_view = Some(view);
    state.set_tool(crate::tools::Tool::DetailLine);
    state.detail.draw = crate::sketch_mode::Draw::Circle;
    // The app's keys and the tool on a canvas, 20 points per meter around the origin.
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: glam::DVec2::ZERO, scale: 20.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        crate::detail_ui::tool(ui, &resp, &painter, state, &view, &xf);
    });
    harness.run();
    let pointer = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, click: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.run();
        if click {
            for pressed in [true, false] {
                let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
                h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
                h.run();
            }
        }
    };
    let typed = |h: &mut Harness<'static, AppState>, text: &str| {
        h.input_mut().events.push(egui::Event::Text(text.into()));
        h.run();
    };
    let lines = |h: &Harness<'static, AppState>| -> Vec<SketchCurve> {
        h.state()
            .project()
            .elements
            .values()
            .filter_map(|e| match &e.kind {
                cc_core::ElementKind::DetailLine(l) if l.view == view => Some(l.curve.clone()),
                _ => None,
            })
            .collect()
    };
    let center = egui::pos2(300.0, 200.0);

    // A circle: its center, the cursor away from it, 2, Enter.
    pointer(&mut harness, center, true);
    assert!(crate::detail_ui::typing(harness.state()));
    pointer(&mut harness, center + egui::vec2(30.0, 10.0), false);
    typed(&mut harness, "2");
    harness.key_press(egui::Key::Enter);
    harness.run();
    let made = lines(&harness);
    assert_eq!(made.len(), 1, "{}", harness.state().message);
    let SketchCurve::Circle { radius, .. } = made[0] else { panic!("a circle: {:?}", made[0]) };
    assert!((radius - 2.0).abs() < 1e-9, "{radius}");

    // Esc drops the typed value before the first point.
    harness.state_mut().detail.draw = crate::sketch_mode::Draw::Line;
    harness.state_mut().detail.chain = false;
    let start = center + egui::vec2(0.0, 100.0);
    pointer(&mut harness, start, true);
    pointer(&mut harness, start + egui::vec2(40.0, 0.0), false);
    typed(&mut harness, "9");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(crate::detail_ui::typing(harness.state()), "the first point stays");
    // A line 3 m towards the cursor.
    typed(&mut harness, "3");
    harness.key_press(egui::Key::Enter);
    harness.run();
    let line = lines(&harness).into_iter().find_map(|c| match c {
        SketchCurve::Line { a, b } => Some(a.distance(b)),
        _ => None,
    });
    assert!(line.is_some_and(|l| (l - 3.0).abs() < 1e-9), "{line:?}");
}

/// The Fill Patterns dialog (#226): opened from the Manage tab, it lists a
/// project pattern next to the built-in ones, adds one with New, and
/// Cancel undoes what it did; the material browser's picker then offers
/// the kept pattern.
#[test]
fn fill_patterns_are_managed_and_picked() {
    let mut state = sample_state();
    crate::commands::run(&mut state, &egui::Context::default(), crate::commands::UiCommand::FillPatterns);
    assert!(state.dialogs.fill_patterns.is_some());
    let text = "*MINE, my test\n45,0,0,0,2\n135,0,0,0,2,1,-1\n";
    let import =
        cc_engine::commands::fill_patterns::FillPatternImport { text: Some(text.into()), ..Default::default() };
    crate::actions::run_command(&mut state, import);
    assert_eq!(state.project().fill_patterns.len(), 1);
    let mut harness = themed_harness(state, egui::vec2(1100.0, 800.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state);
    });
    harness.run();
    harness.get_by_label("MINE");
    harness.get_by_label(&t!("fp-new")).click();
    harness.run();
    assert_eq!(harness.state().project().fill_patterns.len(), 2);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.fill_patterns.is_none());
    assert!(state.project().fill_patterns.is_empty(), "Cancel undoes the import and New");
    let choices = crate::fill_patterns_dialog::choices(state.project());
    assert_eq!(choices.len(), cc_core::Pattern::ALL.len());
}

/// Hiding the Imported CAD category in Visibility/Graphics hides the
/// drawings imported into the view.
#[test]
fn graphics_dialog_hides_imported_cad() {
    let mut state = sample_state();
    let plan = state.project().views_of(cc_core::ViewCategory::FloorPlan).next().unwrap().id;
    state.active_view = Some(plan);
    let dir = std::env::temp_dir().join(format!("ccad-ui-cad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("lines.dxf");
    let dxf = "  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n  8\nA\n 10\n0\n 20\n0\n 30\n0\n 11\n4\n 21\n0\n 31\n0\n  0\nENDSEC\n  0\nEOF\n";
    std::fs::write(&path, dxf).unwrap();
    crate::cad_ui::import_cad_path(&mut state, path);
    crate::jobs::wait(&mut state);
    let id = *state.selection.iter().next().expect("the import is selected");
    let drawn = |s: &mut AppState| {
        s.drawing(plan)
            .unwrap()
            .prims
            .iter()
            .any(|p| matches!(p, cc_views::Prim::Line { element: Some(e), .. } if *e == id))
    };
    assert!(drawn(&mut state));
    crate::graphics_dialog::open(&mut state, crate::graphics_dialog::Target::View(plan));
    let mut harness = themed_harness(state, egui::vec2(980.0, 680.0), egui::Theme::Light, |ui, state| {
        crate::graphics_dialog::show(ui.ctx(), state)
    });
    harness.get_by_label(&t!("vg-annotation-categories")).click_accesskit();
    harness.run();
    let label = format!("{}: {}", t!("vg-visible"), crate::actions::t_category(cc_core::Category::CadImport));
    harness.get_by_label(&label).click_accesskit();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(!harness.state().project().view(plan).unwrap().graphics.category(cc_core::Category::CadImport).visible);
    assert!(!drawn(harness.state_mut()));
}

/// The Profile Manager (*Structure → Profiles*): *Import from catalog*
/// brings IPE profiles in, which OK keeps as one undo step; deleting one a
/// beam type uses, with another taking its place, is undone by Cancel.
#[test]
fn profile_manager_imports_and_cancels() {
    use cc_engine::Ref;
    use cc_engine::commands::profiles::ProfileTypeCreate;
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    let depth = state.session.doc.undo_depth();
    let mut harness = app_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("cmd-profiles")).click();
    harness.run();
    assert!(harness.state().dialogs.profiles.is_some());
    harness.get_by_label(&t!("profiles-import")).click();
    harness.run();
    // The series combo box, as if IPE were picked in it.
    let catalog = harness.state_mut().dialogs.profiles.as_mut().and_then(|d| d.catalog.as_mut());
    catalog.expect("the catalog opens").series = "IPE".into();
    harness.run();
    let add = t!("profiles-catalog-add");
    let disabled = harness.get_by_role_and_label(Role::Button, &add).accesskit_node().is_disabled();
    assert!(disabled, "Add waits for a ticked designation");
    harness.get_by_role_and_label(Role::CheckBox, "IPE 200").click();
    harness.get_by_role_and_label(Role::CheckBox, "IPE 220").click();
    harness.run();
    harness.get_by_label(&t!("profiles-catalog-add")).click();
    harness.run();
    assert!(harness.state().project().profile_named("IPE 200").is_some());
    assert!(harness.state().project().profile_named("IPE 220").is_some());
    assert!(harness.state().dialogs.profiles.as_ref().is_some_and(|d| d.catalog.is_none()), "Add closes the catalog");
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.profiles.is_none());
    let ipe = harness.state().project().profile_named("IPE 200").map(|p| p.id).expect("OK keeps the import");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);

    // A beam type of IPE 200; deleting the profile moves it to another one.
    let command =
        ProfileTypeCreate { profile: Ref::Name("IPE 200".into()), category: "beam".into(), ..Default::default() };
    let beam = crate::actions::run_command(harness.state_mut(), command).and_then(|o| o["id"].as_u64());
    let beam = cc_core::ElementTypeId(beam.expect("a beam type of IPE 200"));
    let users = |h: &Harness<'static, AppState>| h.state().project().profile_users(ipe);
    assert_eq!(users(&harness), vec![beam]);
    harness.get_by_role_and_label(Role::Button, &t!("cmd-profiles")).click();
    harness.run();
    // The list names each profile with its material: "IPE 200 (Steel)".
    let row = harness
        .query_all_by_role(Role::Button)
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .find(|l| l.starts_with("IPE 200 ("))
        .expect("IPE 200 in the list");
    harness.get_by_role_and_label(Role::Button, &row).click();
    harness.run();
    assert_eq!(harness.state().dialogs.profiles.as_ref().and_then(|d| d.selected), Some(ipe));
    harness.get_by_role_and_label(Role::Button, &t!("profiles-delete")).click();
    harness.run();
    assert!(harness.state().project().profile(ipe).is_none(), "{}", harness.state().message);
    assert!(harness.state().project().profile_users(ipe).is_empty());
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.profiles.is_none());
    assert!(harness.state().project().profile(ipe).is_some(), "Cancel undoes the deletion");
    assert_eq!(users(&harness), vec![beam]);
}

/// Type Properties of the template beam: the profile picked in the combo
/// box (IPE 200, imported first) is the type's after OK, as `type_get` shows.
#[test]
fn type_properties_pick_a_profile() {
    use cc_engine::Ref;
    use cc_engine::commands::profiles::ProfileImport;
    let mut state = sample_state();
    let import = ProfileImport { series: "IPE".into(), designations: vec!["IPE 200".into()], on_clash: None };
    state.session.run(import).unwrap();
    let beam = state.project().default_type(cc_core::Category::Beam).unwrap();
    let shown = match &state.project().element_type(beam).unwrap().data {
        cc_core::TypeData::Beam { profile, .. } => {
            profile.and_then(|p| state.project().profile(p)).map_or("—".to_string(), |p| p.name.clone())
        }
        _ => panic!("not a beam type"),
    };
    state.dialogs.type_editor = Some(beam);
    let mut harness = app_harness(state);
    harness.run();
    let combo = harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some(shown.as_str()));
    combo.expect("the profile combo shows the type's profile").click();
    harness.run();
    harness.get_by_label("IPE 200").click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.type_editor.is_none());
    let out = harness.state_mut().session.run(cc_engine::commands::query::TypeGet { type_ref: Ref::Id(beam.0) });
    assert_eq!(out.unwrap()["params"]["profile"], "IPE 200");
}

fn sketch_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1400.0, 900.0), egui::Theme::Light, |ui, state| {
        egui::Panel::top("ribbon").show(ui, |ui| {
            if state.sketch.is_some() {
                crate::sketch_mode::ribbon(ui, state)
            } else {
                crate::ribbon::ribbon(ui, state)
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            if crate::profile_sketch::is_open(state) {
                crate::profile_sketch::central(ui, state);
            }
        });
        crate::dialogs::show(ui.ctx(), state);
    })
}

/// *Edit sketch* in the Profile Manager opens a sketched profile; Finish
/// without changes leaves it as it was (its expressions kept) and goes back
/// to the manager, as Cancel does.
#[test]
fn a_sketched_profile_is_edited_and_kept() {
    use cc_engine::commands::profiles::ProfileCreate;
    let mut state = sample_state();
    let command = ProfileCreate { material: "steel".into(), shape: Some("sketch".into()), ..Default::default() };
    let id = cc_core::ProfileId(state.session.run(command).unwrap()["id"].as_u64().unwrap());
    let before = state.project().profile(id).unwrap().shape.clone();
    assert!(matches!(before, cc_core::ProfileShape::Sketch(_)));
    crate::profiles_dialog::open(&mut state);
    state.dialogs.profiles.as_mut().unwrap().selected = Some(id);
    let depth = state.session.doc.undo_depth();
    let mut harness = sketch_harness(state);
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("profiles-edit-sketch")).click();
    harness.run();
    assert!(harness.state().dialogs.profiles.is_none(), "the manager makes way for the sketch");
    assert!(crate::profile_sketch::is_open(harness.state()));
    assert_eq!(harness.state().sketch.as_ref().unwrap().curves.len(), 4, "the square's sides");
    harness.get_by_role_and_label(Role::Button, &t!("sketch-finish-mode")).click();
    harness.run();
    assert!(harness.state().sketch.is_none(), "{}", harness.state().message);
    assert_eq!(harness.state().project().profile(id).unwrap().shape, before, "unchanged");
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
    let back = harness.state().dialogs.profiles.as_ref().and_then(|d| d.selected);
    assert_eq!(back, Some(id), "back in the manager, on the profile");

    // Cancel goes back to the manager too.
    harness.get_by_role_and_label(Role::Button, &t!("profiles-edit-sketch")).click();
    harness.run();
    assert!(crate::profile_sketch::is_open(harness.state()));
    harness.get_by_role_and_label(Role::Button, &t!("sketch-cancel-mode")).click();
    harness.run();
    assert!(harness.state().sketch.is_none());
    assert_eq!(harness.state().dialogs.profiles.as_ref().and_then(|d| d.selected), Some(id));
}

/// The Profile Manager's session spans *Edit sketch*: a parameter changed
/// before it and a point moved in it are both undone by the manager's
/// Cancel and both kept by its OK.
#[test]
fn the_profile_managers_cancel_undoes_its_sketch() {
    use cc_engine::Ref;
    use cc_engine::commands::profiles::{ProfileCreate, ProfileSet};
    let mut state = sample_state();
    let command = ProfileCreate { material: "steel".into(), shape: Some("sketch".into()), ..Default::default() };
    let id = cc_core::ProfileId(state.session.run(command).unwrap()["id"].as_u64().unwrap());
    let original = state.project().profile(id).unwrap().shape.clone();
    let mut harness = sketch_harness(state);
    let edit = |h: &mut Harness<'static, AppState>| {
        crate::profiles_dialog::open(h.state_mut());
        h.state_mut().dialogs.profiles.as_mut().unwrap().selected = Some(id);
        h.run();
        // A dimension changed in the manager (as its field would).
        let set = ProfileSet {
            profile: Ref::Id(id.0),
            params: Some([("b".to_string(), 0.12)].into_iter().collect()),
            ..Default::default()
        };
        h.state_mut().session.run(set).unwrap();
        h.run();
        h.get_by_role_and_label(Role::Button, &t!("profiles-edit-sketch")).click();
        h.run();
        assert!(h.state().dialogs.edit_start.is_some(), "the manager's session stays open");
        // The top right corner (b/2, h/2) moves 2 cm right.
        let corner = glam::DVec2::new(0.06, 0.05);
        for c in &mut h.state_mut().sketch.as_mut().unwrap().curves {
            if let cc_core::sketch::SketchCurve::Line { a, b } = c {
                for p in [a, b] {
                    if p.distance(corner) < 1e-9 {
                        *p = glam::DVec2::new(0.08, 0.05);
                    }
                }
            }
        }
        h.get_by_role_and_label(Role::Button, &t!("sketch-finish-mode")).click();
        h.run();
        assert!(h.state().sketch.is_none(), "{}", h.state().message);
        let label = t!("profiles-edit-sketch");
        let button = h.get_by_role_and_label(Role::Button, &label);
        assert!(!button.accesskit_node().is_disabled(), "the manager is not nested");
    };
    let outline = |h: &Harness<'static, AppState>| match &h.state().project().profile(id).unwrap().shape {
        cc_core::ProfileShape::Sketch(s) => s.outline.clone(),
        _ => panic!("not a sketch"),
    };
    edit(&mut harness);
    assert!(outline(&harness).contains(&cc_core::profile::ProfilePoint::new("0.08", "0.05")));
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.profiles.is_none());
    assert!(harness.state().dialogs.edit_start.is_none());
    assert_eq!(harness.state().project().profile(id).unwrap().shape, original, "Cancel undoes both");

    edit(&mut harness);
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.edit_start.is_none());
    assert!(outline(&harness).contains(&cc_core::profile::ProfilePoint::new("0.08", "0.05")), "OK keeps the sketch");
    let b = harness.state().project().profile(id).unwrap().shape.params();
    assert!(b.iter().any(|(n, v)| n == "b" && (*v - 0.12).abs() < 1e-12), "OK keeps the dimension: {b:?}");
}

/// A profile's sketch left by another tool (or dropped any other way) ends
/// the Profile Manager's session, keeping its edits, so a later dialog's
/// Cancel undoes only its own; the manager cannot open over the sketch.
#[test]
fn leaving_a_profile_sketch_by_another_tool_ends_the_managers_session() {
    use crate::commands::UiCommand;
    use cc_engine::Ref;
    use cc_engine::commands::profiles::{ProfileCreate, ProfileSet};
    use cc_engine::commands::project::ProjectInfoSet;
    let mut state = sample_state();
    let command = ProfileCreate { material: "steel".into(), shape: Some("sketch".into()), ..Default::default() };
    let id = cc_core::ProfileId(state.session.run(command).unwrap()["id"].as_u64().unwrap());
    let mut harness = sketch_harness(state);
    let to_sketch = |h: &mut Harness<'static, AppState>, v: f64| {
        crate::profiles_dialog::open(h.state_mut());
        h.state_mut().dialogs.profiles.as_mut().unwrap().selected = Some(id);
        h.run();
        let set = ProfileSet {
            profile: Ref::Id(id.0),
            params: Some([("b".to_string(), v)].into_iter().collect()),
            ..Default::default()
        };
        h.state_mut().session.run(set).unwrap();
        h.run();
        h.get_by_role_and_label(Role::Button, &t!("profiles-edit-sketch")).click();
        h.run();
        assert!(crate::profile_sketch::is_open(h.state()));
        assert!(h.state().dialogs.edit_start.is_some());
        assert!(UiCommand::Profiles.unavailable(h.state()).is_some(), "no manager over its own sketch");
        crate::profiles_dialog::open(h.state_mut());
        assert!(h.state().dialogs.profiles.is_none());
    };
    let b = |h: &Harness<'static, AppState>| {
        let params = h.state().project().profile(id).unwrap().shape.params();
        params.iter().find(|(n, _)| n == "b").map(|(_, v)| *v)
    };
    let later_dialog_cancel_keeps = |h: &mut Harness<'static, AppState>, name: &str| {
        let info = ProjectInfoSet {
            name: Some(name.into()),
            number: None,
            client: None,
            address: None,
            author: None,
            fields: None,
        };
        h.state_mut().session.run(info).unwrap();
        crate::dialog::session::begin(h.state_mut());
        crate::dialog::session::cancel(h.state_mut());
        assert_eq!(h.state().project().info.name, name, "a later Cancel keeps unrelated edits");
    };

    // Another tool.
    to_sketch(&mut harness, 0.12);
    harness.state_mut().set_tool(crate::tools::Tool::Measure);
    assert!(harness.state().sketch.is_none());
    assert!(harness.state().dialogs.edit_start.is_none(), "the manager's session ended");
    assert!(harness.state().dialogs.profiles_resume.is_none());
    assert_eq!(b(&harness), Some(0.12), "the manager's edits are kept");
    later_dialog_cancel_keeps(&mut harness, "Unrelated");

    // Dropped directly: settled on the next frame.
    harness.state_mut().set_tool(crate::tools::Tool::Select);
    to_sketch(&mut harness, 0.14);
    harness.state_mut().sketch = None;
    harness.run();
    assert!(harness.state().dialogs.edit_start.is_none());
    assert!(harness.state().dialogs.profiles_resume.is_none());
    later_dialog_cancel_keeps(&mut harness, "Unrelated 2");
}

/// Zooming in until an input node's field passes the canvas edge leaves the editor's panel its size.
#[test]
fn zooming_the_graph_keeps_its_panel_size() {
    // Where the toolbar stands in a panel of the default size: with an empty graph, then zoomed in
    // until input nodes' fields pass the canvas's bottom edge.
    fn toolbar_top(graph: Option<serde_json::Value>) -> f32 {
        let mut state = sample_state();
        if let Some(graph) = graph {
            state.graph.graph = cc_engine::graph::Graph::from_value(graph).unwrap().0;
            state.graph.set_view_for_test(2.0, egui::vec2(0.0, -35.0));
        }
        state.graph.open = true;
        let mut harness = themed_harness(state, egui::vec2(1200.0, 800.0), egui::Theme::Light, |ui, state| {
            egui::Panel::bottom("graph")
                .resizable(true)
                .default_size(300.0)
                .size_range(160.0..=900.0)
                .show(ui, |ui| crate::graph_ui::show(ui, state));
        });
        for _ in 0..5 {
            harness.run();
        }
        harness.get_by_role_and_label(Role::Button, &t!("graph-run")).rect().top()
    }
    let zoomed = toolbar_top(Some(serde_json::json!({
        "uid": "00000000000000000000000000000abd",
        "nodes": [
            {"id": 1, "kind": {"builtin": "number"}, "values": {"value": 8}, "pos": [0, 0]},
            {"id": 2, "kind": {"builtin": "slider"}, "values": {"value": 0.3}, "pos": [250, 112]},
            {"id": 3, "kind": {"builtin": "text"}, "values": {"value": "abc"}, "pos": [500, 122]},
            {"id": 4, "kind": {"builtin": "number"}, "values": {"value": 1}, "pos": [750, 130]}
        ],
        "wires": []
    })));
    let empty = toolbar_top(None);
    assert!((zoomed - empty).abs() < 0.5, "the toolbar moved from {empty} to {zoomed}");
}

/// Issue #339: *Zoom to fit* frames the nodes in the canvas, not in what is
/// left of the toolbar's row: a few nodes stay readable and in the middle.
#[test]
fn zoom_to_fit_frames_the_graph_in_its_canvas() {
    let mut state = sample_state();
    state.graph.graph = cc_engine::graph::Graph::from_value(serde_json::json!({
        "uid": "00000000000000000000000000000abe",
        "nodes": [
            {"id": 1, "kind": {"builtin": "number"}, "values": {"value": 8}, "pos": [900, 700]},
            {"id": 2, "kind": {"builtin": "number"}, "values": {"value": 1}, "pos": [1200, 760]}
        ],
        "wires": []
    }))
    .unwrap()
    .0;
    state.graph.set_view_for_test(0.3, egui::vec2(-2000.0, 50.0));
    state.graph.open = true;
    let mut harness = themed_harness(state, egui::vec2(1200.0, 800.0), egui::Theme::Light, |ui, state| {
        egui::Panel::bottom("graph")
            .resizable(true)
            .default_size(400.0)
            .size_range(160.0..=900.0)
            .show(ui, |ui| crate::graph_ui::show(ui, state));
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("action-zoom-fit")).click();
    for _ in 0..3 {
        harness.run();
    }
    let graph = &harness.state().graph;
    let (zoom, nodes) = graph.view_for_test();
    let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, graph.canvas_for_test());
    assert!(canvas.height() > 200.0, "the canvas, not the toolbar: {canvas:?}");
    // Two nodes are small beside the canvas: they are not shrunk.
    assert!(zoom >= 1.0, "zoom {zoom}");
    assert!(canvas.contains_rect(nodes), "{nodes:?} in {canvas:?}");
    assert!((nodes.center() - canvas.center()).length() < 1.0, "{nodes:?} in the middle of {canvas:?}");
}

/// A schedule lays out only the rows in sight, under its header, and a click on a row selects its element.
#[test]
fn schedule_shows_visible_rows_and_selects() {
    fn first_schedule(state: &AppState) -> Option<cc_core::ViewId> {
        let project = state.project();
        project.views.values().find(|v| matches!(v.kind, cc_core::ViewKind::Schedule { .. })).map(|v| v.id)
    }
    let mut state = sample_state();
    let view = first_schedule(&state).unwrap();
    let table = state.session.schedule(view).unwrap();
    assert!(table.rows.len() > 1, "the sample's first schedule has rows");
    let mut harness = themed_harness(state, egui::vec2(900.0, 400.0), egui::Theme::Light, |ui, state| {
        if let Some(view) = first_schedule(state) {
            crate::schedule_ui::show(ui, state, view);
        }
    });
    harness.run();
    let header = crate::i18n::tr(table.headers[0]);
    assert!(harness.query_by_label(&header).is_some(), "header {header}");
    let row = &table.rows[0];
    let first = match &row.cells[0] {
        cc_views::Cell::Text(s) => s.clone(),
        cc_views::Cell::Key(k) => crate::i18n::tr(k),
    };
    harness.query_all_by_label(&first).next().unwrap().click();
    harness.run();
    assert!(harness.state().selection.contains(&row.element));
}

/// A schedule's properties list the pages of its definition, each with an *Edit* button that opens
/// the dialog on that page; the pages change the schedule through its command.
#[test]
fn schedule_properties_open_the_definition_on_each_page() {
    use crate::schedule_ui::Tab;
    fn first_schedule(state: &AppState) -> Option<cc_core::ViewId> {
        let project = state.project();
        project.views.values().find(|v| matches!(v.kind, cc_core::ViewKind::Schedule { .. })).map(|v| v.id)
    }
    let state = sample_state();
    let view = first_schedule(&state).unwrap();
    let mut harness = themed_harness(state, egui::vec2(360.0, 300.0), egui::Theme::Light, |ui, state| {
        if let Some(view) = first_schedule(state) {
            crate::schedule_ui::props(ui, state, view);
        }
    });
    harness.run();
    for tab in Tab::ALL {
        let label = format!("{}: {}", tab.label(), t!("schedule-edit"));
        harness.get_by_label(&label).click();
        harness.run();
        assert_eq!(harness.state().dialogs.schedule_def, Some(view), "{tab:?}");
        assert_eq!(harness.state().dialogs.schedule_tab, tab);
    }
    // The sorting page: a level is added, and the rows gathered, through the schedule's definition.
    let mut state = sample_state();
    crate::schedule_ui::open(&mut state, view, Tab::Sorting);
    let mut harness = themed_harness(state, egui::vec2(900.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::schedule_ui::dialog(ui.ctx(), state);
    });
    harness.run();
    harness.get_by_label(&t!("schedule-sort-add")).click();
    harness.run();
    harness.get_by_label(&t!("schedule-itemize")).click();
    harness.run();
    let def = |state: &AppState| match state.project().view(view).map(|v| v.kind.clone()) {
        Some(cc_core::ViewKind::Schedule { def, .. }) => def,
        _ => panic!("a schedule"),
    };
    let changed = def(harness.state());
    assert_eq!((changed.sorting.len(), changed.collapse), (1, true));
    // Cancel undoes what the pages changed.
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    let back = def(harness.state());
    assert!(back.sorting.is_empty() && !back.collapse && harness.state().dialogs.schedule_def.is_none());
}

/// The heading fields of a schedule's formatting page are wide enough to type in: a grid offers a
/// cell only its column's width, which left them as narrow as an empty cell.
#[test]
fn schedule_heading_fields_keep_their_width() {
    let mut state = sample_state();
    let view = state.project().views.values().find(|v| matches!(v.kind, cc_core::ViewKind::Schedule { .. }));
    let view = view.map(|v| v.id).unwrap();
    crate::schedule_ui::open(&mut state, view, crate::schedule_ui::Tab::Formatting);
    let mut harness = themed_harness(state, egui::vec2(900.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::schedule_ui::dialog(ui.ctx(), state);
    });
    harness.run();
    let widths: Vec<f32> = harness.query_all_by_role(Role::TextInput).map(|n| n.rect().width()).collect();
    assert!(!widths.is_empty() && widths.iter().all(|w| *w > 150.0), "{widths:?}");
}

/// The project browser follows the organization scheme in use (issue #278):
/// a scheme by view type and level nests the plans under their levels, and
/// the Browser Organization dialog undoes its changes on Cancel.
#[test]
fn the_browser_groups_views_by_the_scheme_in_use() {
    use crate::commands::UiCommand;
    use crate::i18n::tr;
    let mut state = sample_state();
    let level = state.project().levels.first().map(|l| l.name.clone()).unwrap();
    let plans = tr(cc_core::ViewCategory::FloorPlan.key());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::BrowserOrganization);
    assert!(state.dialogs.browser_org.is_some());
    let create = cc_engine::commands::browser::BrowserSchemeCreate { name: "By level".into(), ..Default::default() };
    crate::actions::run_command(&mut state, create);
    if let Some(d) = state.dialogs.browser_org.take() {
        d.edits_finish(&mut state, false);
    }
    assert!(state.project().browser.is_default(), "Cancel undoes the new scheme");

    let mut harness = themed_harness(state, egui::vec2(400.0, 1200.0), egui::Theme::Light, |ui, state| {
        egui::ScrollArea::vertical().show(ui, |ui| crate::browser::show(ui, state));
    });
    harness.run();
    // By type alone: the level's name is only its plans'.
    let flat = harness.query_all_by_label(&level).count();
    assert!(flat >= 1);
    assert!(harness.query_by_label(&format!("{level}  ")).is_none(), "no level group");

    let create = cc_engine::commands::browser::BrowserSchemeCreate { name: "By level".into(), ..Default::default() };
    crate::actions::run_command(harness.state_mut(), create);
    harness.run();
    // The level is now a group under each plan type that has plans on it.
    assert!(harness.query_all_by_label(&format!("{level}  ")).count() >= 1, "a level group");
    assert!(harness.query_by_label(&format!("{plans}  ")).is_some());
    // Its plans are still there, one row each.
    assert_eq!(harness.query_all_by_label(&level).count(), flat);

    // A click on the level group folds its plans away.
    harness.query_all_by_label(&format!("{level}  ")).next().unwrap().click();
    harness.run();
    assert!(harness.query_all_by_label(&level).count() < flat);
}

/// The levels dialog adds a level above or below the picked one, pushing
/// the levels beyond it away, so the others keep their order (issue #297).
#[test]
fn the_levels_dialog_adds_a_level_above_or_below_the_picked_one() {
    let mut state = sample_state();
    state.dialogs.levels = true;
    let before: Vec<_> = state.project().levels.iter().map(|l| (l.id, l.elevation)).collect();
    let (lowest, base) = before[0];
    state.dialogs.level_selected = Some(lowest);
    let mut harness = app_harness(state);
    harness.run();
    harness.get_by_label(&t!("level-add-above")).click();
    harness.run();
    let levels = harness.state().project().levels.clone();
    assert_eq!(levels.len(), before.len() + 1);
    let added = levels[1].id;
    assert!(before.iter().all(|(id, _)| *id != added), "the new level stands right above the picked one");
    assert_eq!(harness.state().dialogs.level_selected, Some(added), "and is picked");
    let storey = levels[1].elevation - base;
    assert!(storey > 0.0);
    // The levels that were above moved up by the new storey, in their order.
    let after: Vec<_> = levels.iter().filter(|l| l.id != added).map(|l| (l.id, l.elevation)).collect();
    for ((id, old), (new_id, new)) in before.iter().zip(&after) {
        assert_eq!(id, new_id);
        let moved = if *id == lowest { 0.0 } else { storey };
        assert!((new - old - moved).abs() < 1e-9);
    }
    harness.get_by_label(&t!("level-add-below")).click();
    harness.run();
    let levels = harness.state().project().levels.clone();
    assert_eq!(levels.len(), before.len() + 2);
    assert_eq!(levels[2].id, added, "the level under it went in between");
    assert_eq!(levels[0].id, lowest);
    assert!(levels[0].elevation < base, "and pushed the lowest level down");
}

/// Column in Steel starts on a steel type, and the options bar lists only
/// steel column types; Timber makes its default type, undone in one step.
#[test]
fn structure_column_tool_keeps_to_its_material() {
    use crate::commands::UiCommand;
    use cc_core::{Category, StructuralMaterial as M};
    let mut state = sample_state();
    let ctx = egui::Context::default();
    crate::commands::run(&mut state, &ctx, UiCommand::StructurePlace(Category::Column, M::Steel));
    let crate::tools::Tool::Column { type_id, material } = state.tool.clone() else { panic!("{:?}", state.tool) };
    assert_eq!(material, Some(M::Steel));
    let project = state.project();
    assert_eq!(project.structural_material(&project.element_type(type_id).unwrap().data), Some(M::Steel));
    let listed = crate::tools::tool_types(project, Category::Column, type_id, Some(M::Steel));
    assert!(!listed.is_empty());
    assert!(
        listed
            .iter()
            .all(|(id, _)| project.structural_material(&project.element_type(*id).unwrap().data) == Some(M::Steel))
    );
    assert!(listed.len() < project.types_of(Category::Column).count(), "concrete types are left out");
    // Timber: the project has none, so one is made.
    let depth = state.session.doc.undo_depth();
    crate::commands::run(&mut state, &ctx, UiCommand::StructurePlace(Category::Beam, M::Timber));
    let crate::tools::Tool::Beam { type_id, .. } = state.tool.clone() else { panic!() };
    assert_eq!(state.project().type_name(type_id), "MLC 9x32");
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
}

/// The Structure tab's material selector swaps the material's groups:
/// reinforcement in Concrete, bracing and the grid tools in Steel.
#[test]
fn structure_material_selector_swaps_groups() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Concrete;
    let mut harness = ribbon_harness(state, 1600.0);
    let rebar = crate::commands::UiCommand::TypicalRebar.label(harness.state());
    let braces = crate::commands::UiCommand::Bracing.label(harness.state());
    assert!(harness.query_by_label(&rebar).is_some(), "concrete shows reinforcement");
    assert!(harness.query_by_label(&braces).is_none());
    harness.get_by_label(&crate::i18n::tr("profile-material-steel")).click();
    harness.run();
    assert_eq!(harness.state().prefs.structure_material, cc_core::StructuralMaterial::Steel);
    assert!(harness.query_by_label(&rebar).is_none(), "steel hides reinforcement");
    assert!(harness.query_by_label(&braces).is_some());
    assert!(harness.query_by_label(&crate::commands::UiCommand::ColumnsAtGrids.label(harness.state())).is_some());
}

/// *Grid Frame* creates columns and beams on the grids for the chosen
/// levels in one undo step; Cancel leaves the model as it was.
#[test]
fn grid_frame_dialog_creates_and_cancels() {
    let mut state = sample_state();
    let grids =
        [([0.0, -1.0], [0.0, 7.0]), ([6.0, -1.0], [6.0, 7.0]), ([-1.0, 0.0], [7.0, 0.0]), ([-1.0, 6.0], [7.0, 6.0])];
    // Selecting the new grids keeps the example's own out of the frame.
    let mut created = Vec::new();
    for (start, end) in grids {
        let command = cc_engine::commands::create::GridCreate { start, end, ..Default::default() };
        let out = crate::actions::run_command(&mut state, command).expect("grid");
        created.push(cc_core::ElementId(out["id"].as_u64().expect("grid id")));
    }
    state.selection = created.into_iter().collect();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    let depth = state.session.doc.undo_depth();
    let columns = |h: &Harness<'static, AppState>| h.state().project().elements_of(cc_core::Category::Column).count();
    let mut harness = app_harness(state);
    let before = columns(&harness);
    harness.get_by_role_and_label(Role::Button, &t!("action-grid-frame")).click();
    harness.run();
    assert!(harness.state().dialogs.grid_frame.is_some());
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.grid_frame.is_none());
    assert_eq!(harness.state().session.doc.undo_depth(), depth, "Cancel changes nothing");
    harness.get_by_role_and_label(Role::Button, &t!("action-grid-frame")).click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.grid_frame.is_none());
    assert_eq!(columns(&harness), before + 4, "four intersections on the first storey");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
}

/// With grids selected, the secondary beams' *Parallel to* offers (and
/// starts on) the selected grids, so OK is never refused for another one.
#[test]
fn grid_frame_secondary_beams_use_the_selected_grids() {
    let mut state = sample_state();
    let grids =
        [([0.0, -1.0], [0.0, 7.0]), ([6.0, -1.0], [6.0, 7.0]), ([-1.0, 0.0], [7.0, 0.0]), ([-1.0, 6.0], [7.0, 6.0])];
    let mut created = Vec::new();
    for (start, end) in grids {
        let command = cc_engine::commands::create::GridCreate { start, end, ..Default::default() };
        let out = crate::actions::run_command(&mut state, command).expect("grid");
        created.push(cc_core::ElementId(out["id"].as_u64().expect("grid id")));
    }
    state.selection = created.iter().copied().collect();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    let beams = |h: &Harness<'static, AppState>| h.state().project().elements_of(cc_core::Category::Beam).count();
    let mut harness = app_harness(state);
    let before = beams(&harness);
    harness.get_by_role_and_label(Role::Button, &t!("action-grid-frame")).click();
    harness.run();
    let parallel = harness.state().dialogs.grid_frame.as_ref().and_then(|d| d.parallel_to).expect("a default grid");
    assert!(created.contains(&parallel), "the default is one of the selected grids");
    harness.get_by_label(&t!("label-secondary-beams")).click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.grid_frame.is_none(), "OK was accepted: {:?}", state.message);
    assert!(beams(&harness) > before + 4, "main beams and the secondary ones");
}

/// *Structure → Steel Building* (steel) opens its dialog; Cancel changes
/// nothing, OK creates the building and its members in one undo step.
#[test]
fn steel_building_dialog_creates_a_building() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    let depth = state.session.doc.undo_depth();
    let count = |h: &Harness<'static, AppState>, c| h.state().project().elements_of(c).count();
    let mut harness = app_harness(state);
    let columns = count(&harness, cc_core::Category::Column);
    harness.get_by_role_and_label(Role::Button, &t!("action-steel-building")).click();
    harness.run();
    assert!(harness.state().dialogs.steel_building.is_some());
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.steel_building.is_none());
    assert_eq!(harness.state().session.doc.undo_depth(), depth, "Cancel changes nothing");
    harness.get_by_role_and_label(Role::Button, &t!("action-steel-building")).click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.steel_building.is_none());
    assert_eq!(count(&harness, cc_core::Category::SteelBuilding), 1);
    assert_eq!(count(&harness, cc_core::Category::Column), columns + 12, "two columns on each of six frames");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
}

/// A steel building's rows edit it through `element_set`: a new span moves
/// its columns (same ids), and *Reset Members* brings back a deleted one.
#[test]
fn steel_building_properties_regenerate_it() {
    let mut state = sample_state();
    let args = serde_json::json!({"origin": [100, 100], "purlins": null, "girts": null});
    let out = state.session.execute("steel_building_create", args).unwrap();
    let host = cc_core::ElementId(out["id"].as_u64().unwrap());
    let members = |s: &AppState| match &s.project().element(host).unwrap().kind {
        cc_core::ElementKind::SteelBuilding(b) => (b.members.clone(), b.detached.len()),
        _ => panic!("not a steel building"),
    };
    let (first, _) = members(&state);
    // The path the palette's rows take.
    crate::steel_building_ui::edit(&mut state, host, "span", |b| b.span = 24.0);
    let (after, _) = members(&state);
    let ids = |m: &[cc_core::steel_building::FrameMember]| m.iter().map(|m| m.id).collect::<Vec<_>>();
    assert_eq!(ids(&after), ids(&first), "same members");
    let right = after[1].id;
    let cc_core::ElementKind::Column(c) = &state.project().element(right).unwrap().kind else { panic!("a column") };
    assert!((c.position.x - 124.0).abs() < 1e-9);
    state.session.execute("elements_delete", serde_json::json!({"ids": [right.0]})).unwrap();
    assert_eq!(members(&state).1, 1, "the deleted column is detached");
    state.selection = [host].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("action-reset-members")).is_some());
    harness.get_by_label(&t!("action-reset-members")).click();
    harness.run();
    let (members_now, detached) = members(harness.state());
    assert_eq!((members_now.len(), detached), (first.len(), 0));
}

/// The Truss tool shows for steel and timber (not concrete) and starts with
/// the tab's material.
#[test]
fn truss_tool_follows_the_structure_material() {
    use cc_core::StructuralMaterial as M;
    for (material, shown) in [(M::Concrete, false), (M::Steel, true), (M::Timber, true)] {
        let mut state = sample_state();
        state.ribbon_tab = RibbonTab::Structure;
        state.prefs.structure_material = material;
        let mut harness = ribbon_harness(state, 1600.0);
        let label = crate::commands::UiCommand::Truss.label(harness.state());
        assert_eq!(harness.query_by_label(&label).is_some(), shown, "{material:?}");
        if shown {
            harness.get_by_role_and_label(Role::Button, &label).click();
            harness.run();
            assert_eq!(harness.state().tool, crate::tools::Tool::Truss { material });
        }
    }
}

/// The options the tool runs with make a parametric truss of the tab's material.
#[test]
fn truss_tool_options_make_a_parametric_truss() {
    use cc_core::truss::{TrussPattern, TrussShape};
    let mut state = sample_state();
    state.truss = crate::tools::TrussOptions {
        shape: TrussShape::Gable,
        pattern: TrussPattern::KingPost,
        height: 1.5,
        panels: 4,
        copies: 3,
        spacing: 1.2,
    };
    let level = state.project().levels[0].id;
    let (a, b) = (glam::DVec2::new(100.0, 100.0), glam::DVec2::new(106.0, 100.0));
    let command = crate::tools::truss_command(&state.truss, cc_core::StructuralMaterial::Timber, level, a, b);
    let out = state.session.run(command).expect("truss");
    let host = cc_core::ElementId(out["id"].as_u64().unwrap());
    let cc_core::ElementKind::Truss(t) = &state.project().element(host).unwrap().kind else { panic!("a truss") };
    assert_eq!((t.shape, t.pattern, t.panels, t.copies), (TrussShape::Gable, TrussPattern::KingPost, 4, 3));
    assert_eq!(t.members.len(), 18, "two top chords, one bottom chord, two diagonals and the king post per copy");
    assert!(state.project().types.values().any(|x| x.name == "Peça 6x16"), "timber chords");
}

/// The Bracing command shows for steel and timber (not concrete) and, with
/// no two columns selected, starts the tool with the tab's material.
#[test]
fn bracing_tool_follows_the_structure_material() {
    use cc_core::StructuralMaterial as M;
    for (material, shown) in [(M::Concrete, false), (M::Steel, true), (M::Timber, true)] {
        let mut state = sample_state();
        state.selection.clear();
        state.ribbon_tab = RibbonTab::Structure;
        state.prefs.structure_material = material;
        let mut harness = ribbon_harness(state, 1600.0);
        let label = crate::commands::UiCommand::Bracing.label(harness.state());
        assert_eq!(harness.query_by_label(&label).is_some(), shown, "{material:?}");
        if shown {
            harness.get_by_role_and_label(Role::Button, &label).click();
            harness.run();
            assert_eq!(harness.state().tool, crate::tools::Tool::Bracing { material });
        }
    }
}

/// The tool's options make a parametric bracing, two selected columns give
/// the command their bay, and the palette's rows regenerate the braces.
#[test]
fn bracing_options_columns_and_properties() {
    use cc_core::bracing::BracingPattern;
    let mut state = sample_state();
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    state.bracing =
        crate::tools::BracingOptions { pattern: BracingPattern::K, height: 3.0, stories: 2, ..Default::default() };
    let level = state.project().levels[0].id;
    let (a, b) = (glam::DVec2::new(100.0, 100.0), glam::DVec2::new(105.0, 100.0));
    let command = crate::tools::bracing_command(&state.bracing, cc_core::StructuralMaterial::Steel, level, a, b);
    let out = state.session.run(command).expect("bracing");
    let host = cc_core::ElementId(out["id"].as_u64().unwrap());
    let get = |s: &AppState, id: cc_core::ElementId| match &s.project().element(id).unwrap().kind {
        cc_core::ElementKind::Bracing(b) => (**b).clone(),
        _ => panic!("not a bracing"),
    };
    assert_eq!(get(&state, host).members.len(), 8, "four braces per storey");
    // The path the palette's rows take.
    crate::bracing_ui::edit(&mut state, host, "pattern", |b| b.pattern = BracingPattern::Chevron);
    assert_eq!(get(&state, host).members.len(), 4);
    // Two selected columns: the command braces their bay and selects the bracing.
    let column = |s: &mut AppState, x: f64| {
        let args = serde_json::json!({"position": [x, 120.0], "height": 6.0});
        let id = s.session.execute("column_create", args).unwrap()["id"].as_u64().unwrap();
        let free = serde_json::json!({"id": id, "params": {"top": "Unconnected"}});
        s.session.execute("element_set", free).unwrap();
        cc_core::ElementId(id)
    };
    state.selection = [column(&mut state, 100.0), column(&mut state, 104.0)].into_iter().collect();
    crate::bracing_ui::start(&mut state);
    let made = *state.selection.iter().next().expect("the new bracing is selected");
    let bay = get(&state, made);
    assert_eq!((bay.pattern, bay.stories), (BracingPattern::K, 2));
    assert!(
        (bay.height - 3.0).abs() < 1e-9 && (bay.length() - 4.0).abs() < 1e-9,
        "the columns' height over two storeys"
    );
    // A deleted brace is detached; the palette's Reset Members brings it back.
    let brace = bay.members[0].id;
    state.session.execute("elements_delete", serde_json::json!({"ids": [brace.0]})).unwrap();
    assert_eq!(get(&state, made).detached.len(), 1);
    state.selection = [made].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    harness.get_by_label(&t!("action-reset-members")).click();
    harness.run();
    let bay = get(harness.state(), made);
    assert_eq!((bay.members.len(), bay.detached.len()), (8, 0));
}

/// A truss's rows edit it through `element_set`: new panels regenerate it,
/// and *Reset Members* brings back a deleted post.
#[test]
fn truss_properties_regenerate_it() {
    let mut state = sample_state();
    let args = serde_json::json!({"start": [100, 100], "end": [106, 100], "height": 1.0, "panels": 4});
    let out = state.session.execute("truss_create", args).unwrap();
    let host = cc_core::ElementId(out["id"].as_u64().unwrap());
    let get = |s: &AppState| match &s.project().element(host).unwrap().kind {
        cc_core::ElementKind::Truss(t) => (**t).clone(),
        _ => panic!("not a truss"),
    };
    // The path the palette's rows take.
    crate::truss_ui::edit(&mut state, host, "panels", |t| t.panels = 6);
    assert_eq!(get(&state).members.len(), 2 + 7 + 6);
    let post = get(&state).members.iter().find(|m| m.role.kind == cc_core::truss::TrussRoleKind::Vertical).unwrap().id;
    state.session.execute("elements_delete", serde_json::json!({"ids": [post.0]})).unwrap();
    assert_eq!(get(&state).detached.len(), 1);
    state.selection = [host].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(
        harness.query_by_label(&t!("action-reset-members")).is_some(),
        "row labels are painted, the button is a widget"
    );
    harness.get_by_label(&t!("action-reset-members")).click();
    harness.run();
    let t = get(harness.state());
    assert_eq!((t.members.len(), t.detached.len()), (15, 0));
}

/// Light Steel Frame shows in the Steel framing group, Wood Frame in the
/// Timber one, neither for concrete.
#[test]
fn wall_framing_buttons_follow_the_structure_material() {
    use cc_core::StructuralMaterial as M;
    for (material, steel, wood) in [(M::Concrete, false, false), (M::Steel, true, false), (M::Timber, false, true)] {
        let mut state = sample_state();
        state.ribbon_tab = RibbonTab::Structure;
        state.prefs.structure_material = material;
        let harness = ribbon_harness(state, 1800.0);
        assert_eq!(harness.query_by_label(&t!("action-light-steel-frame")).is_some(), steel, "{material:?}");
        assert_eq!(harness.query_by_label(&t!("action-wood-frame")).is_some(), wood, "{material:?}");
    }
}

/// *Structure → Light Steel Frame* opens the dialog; OK frames the selected
/// wall only, in one undo step, and selects the framing.
#[test]
fn wall_framing_dialog_frames_the_selected_wall() {
    use cc_core::wall_framing::FramingSystem;
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    let args = serde_json::json!({"start": [100, 100], "end": [103.6, 100], "height": 3.0});
    let wall = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    state.selection = [wall].into_iter().collect();
    let depth = state.session.doc.undo_depth();
    let mut harness = app_harness(state);
    harness.get_by_role_and_label(Role::Button, &t!("action-light-steel-frame")).click();
    harness.run();
    let system = harness.state().dialogs.wall_framing.as_ref().map(|d| d.system);
    assert_eq!(system, Some(FramingSystem::LightSteel));
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.wall_framing.is_none(), "OK was accepted: {:?}", state.message);
    let framings: Vec<(cc_core::ElementId, cc_core::ElementId, usize)> = state
        .project()
        .elements
        .values()
        .filter_map(|e| match &e.kind {
            cc_core::ElementKind::WallFraming(f) => Some((e.id, f.wall, f.members.len())),
            _ => None,
        })
        .collect();
    assert_eq!(framings.len(), 1, "only the selected wall");
    assert_eq!((framings[0].1, framings[0].2), (wall, 12), "10 studs and two tracks");
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    assert!(state.selection.contains(&framings[0].0));
}

/// A framing's rows edit it through `element_set`, *Reset Members* brings
/// back a deleted stud, and a member's *Framing* row selects the framing.
#[test]
fn wall_framing_properties_and_the_member_row() {
    use cc_core::wall_framing::{FramingRoleKind, WallFraming};
    let mut state = sample_state();
    let args = serde_json::json!({"start": [100, 100], "end": [103.6, 100], "height": 3.0});
    let wall = state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap();
    let out = state.session.execute("wall_framing_create", serde_json::json!({"walls": [wall]})).unwrap();
    let host = cc_core::ElementId(out["framings"][0]["id"].as_u64().unwrap());
    let get = |s: &AppState| match &s.project().element(host).unwrap().kind {
        cc_core::ElementKind::WallFraming(f) => (**f).clone(),
        _ => panic!("not a wall framing"),
    };
    let studs = |f: &WallFraming| f.members.iter().filter(|m| m.role.kind == FramingRoleKind::Stud).count();
    // The path the palette's rows take.
    crate::wall_framing_ui::edit(&mut state, host, "spacing", |f| f.spacing = 0.6);
    assert_eq!(studs(&get(&state)), 7);
    let stud = get(&state).members.iter().find(|m| m.role.kind == FramingRoleKind::Stud).unwrap().id;
    state.session.execute("elements_delete", serde_json::json!({"ids": [stud.0]})).unwrap();
    assert_eq!(get(&state).detached.len(), 1);
    state.selection = [host].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(
        harness.query_by_label(&t!("action-reset-members")).is_some(),
        "row labels are painted, the button is a widget"
    );
    harness.get_by_label(&t!("action-reset-members")).click();
    harness.run();
    let f = get(harness.state());
    assert_eq!((studs(&f), f.detached.len()), (7, 0));
    let member = f.members[0].id;
    harness.state_mut().selection = [member].into_iter().collect();
    harness.run();
    harness.get_by_label(&t!("action-select-framing")).click();
    harness.run();
    let selection = &harness.state().selection;
    assert!(selection.len() == 1 && selection.contains(&host));
}

/// *Structure → Connections → Base Plate* (steel) places a base plate
/// under each selected column, in one undo step, and says so.
#[test]
fn base_plate_button_places_connections_on_the_selected_columns() {
    use cc_core::connection::ConnectionKind;
    let mut state = sample_state();
    let columns: Vec<cc_core::ElementId> =
        state.project().elements_of(cc_core::Category::Column).map(|e| e.id).collect();
    assert!(!columns.is_empty(), "the sample house has columns");
    state.selection = columns.iter().copied().collect();
    state.ribbon_tab = RibbonTab::Structure;
    state.prefs.structure_material = cc_core::StructuralMaterial::Steel;
    let depth = state.session.doc.undo_depth();
    let mut harness = ribbon_harness_sized(state, egui::vec2(2400.0, 150.0));
    let label = crate::commands::UiCommand::Connection(ConnectionKind::BasePlate).label(harness.state());
    harness.get_by_role_and_label(Role::Button, &label).click();
    harness.run();
    let state = harness.state();
    let placed: Vec<cc_core::ElementId> = state
        .project()
        .elements
        .values()
        .filter_map(|e| match &e.kind {
            cc_core::ElementKind::Connection(c) => Some(c.member),
            _ => None,
        })
        .collect();
    assert_eq!(placed.len(), columns.len());
    assert!(columns.iter().all(|c| placed.contains(c)));
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    assert!(state.message.contains(&columns.len().to_string()), "{}", state.message);
}

/// Steel shows base, end and fin plates; timber bolted joints and hangers.
#[test]
fn connection_buttons_follow_the_structure_material() {
    use cc_core::StructuralMaterial as M;
    use cc_core::connection::ConnectionKind;
    for material in [M::Steel, M::Timber] {
        let mut state = sample_state();
        state.ribbon_tab = RibbonTab::Structure;
        state.prefs.structure_material = material;
        let harness = ribbon_harness_sized(state, egui::vec2(2400.0, 150.0));
        for kind in ConnectionKind::ALL {
            let label = crate::commands::UiCommand::Connection(kind).label(harness.state());
            assert_eq!(
                harness.query_by_label(&label).is_some(),
                kind.material() == material,
                "{kind:?} in {material:?}"
            );
        }
    }
}

/// A connection's rows edit it through `element_set`.
#[test]
fn connection_properties_change_its_bolts() {
    let mut state = sample_state();
    let column = state.project().elements_of(cc_core::Category::Column).map(|e| e.id).next().unwrap();
    let args = serde_json::json!({"kind": "base_plate", "members": [column.0]});
    let out = state.session.execute("connections_create", args).unwrap();
    let id = cc_core::ElementId(out["created"][0].as_u64().unwrap());
    crate::connection_ui::edit(&mut state, id, "rows", |c| c.params.rows = 3);
    let cc_core::ElementKind::Connection(c) = &state.project().element(id).unwrap().kind else {
        panic!("a connection")
    };
    assert_eq!(c.params.rows, 3);
    state.selection = [id].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    // Row labels are painted; the plate, bolt and anchor values are widgets.
    let values = harness.query_all_by_role(Role::SpinButton).count();
    assert!(values >= 9, "{values} value widgets");
}

/// Beams selected and *Base Plate* clicked: nothing is placed (not even on the
/// level's columns) and the status bar says why; with no selection the active
/// level's columns get theirs.
#[test]
fn a_selection_without_the_kinds_members_places_nothing() {
    use cc_core::connection::ConnectionKind;
    let mut state = sample_state();
    let _ = state.session.execute("beam_create", serde_json::json!({"start": [0, 0], "end": [3, 0]}));
    let beams: Vec<cc_core::ElementId> = state.project().elements_of(cc_core::Category::Beam).map(|e| e.id).collect();
    assert!(!beams.is_empty());
    state.selection = beams.into_iter().collect();
    let depth = state.session.doc.undo_depth();
    crate::connection_ui::place(&mut state, ConnectionKind::BasePlate);
    assert_eq!(state.session.doc.undo_depth(), depth);
    assert_eq!(state.message, crate::t!("msg-connections-none"));
    state.selection.clear();
    crate::connection_ui::place(&mut state, ConnectionKind::BasePlate);
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
}

/// New Component lists the kinds with the parameters each starts with;
/// picking Door and Create makes a door component, open in the component
/// environment.
#[test]
fn new_component_picks_a_kind_and_opens_the_environment() {
    let mut state = sample_state();
    crate::new_component_dialog::open(&mut state);
    let mut harness = themed_harness(state, egui::vec2(900.0, 700.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state)
    });
    harness.run();
    // A generic component starts with no parameters.
    assert!(harness.query_by_label(&t!("new-component-generic")).is_some());
    harness.get_by_label(&crate::i18n::tr(cc_core::ComponentKind::Door.key())).click();
    harness.run();
    assert!(harness.query_by_label(&t!("new-component-generic")).is_none());
    assert!(harness.query_by_label(&t!("tpl-param-width")).is_some(), "a door starts with its width");
    if snapshots_requested() {
        harness.snapshot("new_component");
    }
    harness.get_by_label(&t!("new-project-create")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.new_component.is_none());
    let made = state.component_env.as_ref().expect("the environment opens").type_id;
    let ty = state.project().element_type(made).unwrap();
    assert_eq!(ty.category(), cc_core::Category::Door);
    assert_eq!(ty.name, t!("comp-new-name"));
}

/// A length as a number field shows it, without the unit.
fn length_text(meters: f64) -> String {
    let (units, comma) = cc_views::format::current();
    cc_views::format::length_number_with(&units, comma, meters)
}

/// The text of the number field being typed in.
fn typed_text(harness: &Harness<'static, AppState>) -> String {
    harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).and_then(|n| n.value()).unwrap_or_default()
}

/// Type `text` over the number field showing `shown`, a key at a time as a
/// person does, and return what the field read after each key.
fn type_over(harness: &mut Harness<'static, AppState>, shown: &str, text: &str) -> Vec<String> {
    harness.query_all_by_role(Role::SpinButton).find(|n| n.value().as_deref() == Some(shown)).unwrap().focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    let mut read = Vec::new();
    for ch in text.chars() {
        harness.query_all_by_role(Role::SpinButton).find(|n| n.is_focused()).unwrap().type_text(&ch.to_string());
        harness.run();
        read.push(typed_text(harness));
    }
    read
}

/// Typing a number in Type Properties keeps what was typed until the field
/// is left: a half-typed value the type would clamp (a riser of `1`) or
/// refuse (a door `0` wide) must not rewrite the text under the caret
/// (issue 319: typing `1` showed `5`, and `0.9` over `0.80` gave `0.9.80`).
#[test]
fn typed_numbers_are_kept_until_the_field_is_left() {
    let mut state = sample_state();
    let stair = state.project().default_type(cc_core::Category::Stair).unwrap();
    let riser = |s: &AppState| match s.project().element_type(stair).map(|t| &t.data) {
        Some(cc_core::TypeData::Stair { max_riser, .. }) => *max_riser,
        other => panic!("not a stair type: {other:?}"),
    };
    let before = riser(&state);
    let shown = format!("{} {}", length_text(before), state.project().units.length.unit.symbol());
    state.dialogs.type_editor = Some(stair);
    let mut harness = type_dialog_harness(state);
    harness.run();
    let typed = length_text(0.16);
    let read = type_over(&mut harness, &shown, &typed);
    let want: Vec<String> = (1..=typed.chars().count()).map(|n| typed.chars().take(n).collect()).collect();
    assert_eq!(read, want);
    assert_eq!(riser(harness.state()), before, "nothing changes while typing");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!((riser(harness.state()) - 0.16).abs() < 1e-9, "{}", riser(harness.state()));

    // A door's width, which starts with a zero.
    let mut state = sample_state();
    let door = state.project().default_type(cc_core::Category::Door).unwrap();
    let width = |s: &AppState| s.project().element_type(door).and_then(|t| t.opening_size()).map(|(w, _)| w);
    let before = width(&state).unwrap();
    let shown = format!("{} {}", length_text(before), state.project().units.length.unit.symbol());
    state.dialogs.type_editor = Some(door);
    let mut harness = type_dialog_harness(state);
    harness.run();
    let typed = length_text(0.95);
    let read = type_over(&mut harness, &shown, &typed);
    let want: Vec<String> = (1..=typed.chars().count()).map(|n| typed.chars().take(n).collect()).collect();
    assert_eq!(read, want);
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(width(harness.state()).is_some_and(|w| (w - 0.95).abs() < 1e-9), "{:?}", width(harness.state()));
}

/// The first view of the sample that `test` accepts.
fn sample_view(state: &AppState, test: impl Fn(&cc_core::View) -> bool) -> cc_core::ViewId {
    state.project().views.values().find(|v| test(v)).map(|v| v.id).expect("the sample has such a view")
}

/// Make a plan (or a 3D view) of the sample active, give it a sun of the
/// kind `mode` and open Sun Settings on it.
fn open_sun_dialog(state: &mut AppState, mode: &str, three_d: bool) {
    let view = if three_d {
        sample_view(state, |v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. }))
    } else {
        sample_view(state, cc_core::View::is_plan)
    };
    state.active_view = Some(view);
    let sun = cc_engine::commands::sun::SunArg { mode: Some(mode.to_string()), ..Default::default() };
    let set = cc_engine::commands::views::ViewSet {
        view: cc_engine::Ref::Id(view.0),
        sun: Some(Some(sun)),
        ..Default::default()
    };
    state.session.run(set).unwrap();
    crate::sun_ui::open(state);
    assert!(state.dialogs.sun.is_some());
}

fn sun_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1400.0, 1000.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state)
    })
}

/// Sun Settings on a plan without a sun gives it one; Cancel takes it away
/// with no undo step left, OK keeps it as one step, and *Remove Sun* clears it.
#[test]
fn sun_settings_give_a_view_a_sun_that_cancel_takes_away() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let plan = sample_view(&state, cc_core::View::is_plan);
    state.active_view = Some(plan);
    assert!(state.project().view(plan).unwrap().sun.is_none());
    let depth = state.session.doc.undo_depth();
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SunSettings);
    assert!(state.dialogs.sun.is_some());
    assert_eq!(state.project().view(plan).unwrap().sun, Some(cc_core::sun::SunSettings::default()));
    let mut harness = sun_harness(state);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.sun.is_none());
    assert!(state.project().view(plan).unwrap().sun.is_none());
    assert_eq!(state.session.doc.undo_depth(), depth);

    // OK keeps the sun and every change made, as one undo step.
    crate::commands::run(harness.state_mut(), &egui::Context::default(), UiCommand::SunSettings);
    harness.run();
    harness.get_by_label(&t!("sun-preset-december-solstice")).click();
    harness.run();
    harness.get_by_label(&t!("sun-show-path")).click();
    harness.run();
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.sun.is_none());
    let sun = state.project().view(plan).unwrap().sun.clone().unwrap();
    assert!(!sun.show_path);
    assert!(matches!(sun.mode, cc_core::sun::SunMode::Still { date, time: 720 } if date.iso() == "2026-12-21"));
    assert_eq!(state.session.doc.undo_depth(), depth + 1);

    // Remove Sun clears it and closes, keeping that.
    crate::commands::run(harness.state_mut(), &egui::Context::default(), UiCommand::SunSettings);
    harness.run();
    harness.get_by_label(&t!("sun-remove")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.sun.is_none());
    assert!(state.project().view(plan).unwrap().sun.is_none());
}

/// *Remove Sun* on a view that had none changes nothing: no undo step and
/// the document as it was. On a view with a sun it is one undo step.
#[test]
fn removing_a_sun_the_view_did_not_have_changes_nothing() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let plan = sample_view(&state, cc_core::View::is_plan);
    state.active_view = Some(plan);
    let (depth, before, dirty) =
        (state.session.doc.undo_depth(), state.project().clone(), state.session.doc.is_dirty());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SunSettings);
    let mut harness = sun_harness(state);
    harness.get_by_label(&t!("sun-remove")).click();
    harness.run();
    let state = harness.state();
    assert!(state.dialogs.sun.is_none());
    assert_eq!(state.session.doc.undo_depth(), depth);
    assert!(*state.project() == before, "the project is as it was");
    assert_eq!(state.session.doc.is_dirty(), dirty);

    // With a sun: removing it is one step, which undo takes back.
    let set = cc_engine::commands::views::ViewSet {
        view: cc_engine::Ref::Id(plan.0),
        sun: Some(Some(Default::default())),
        ..Default::default()
    };
    harness.state_mut().session.run(set).unwrap();
    let depth = harness.state().session.doc.undo_depth();
    crate::commands::run(harness.state_mut(), &egui::Context::default(), UiCommand::SunSettings);
    harness.run();
    harness.get_by_label(&t!("sun-remove")).click();
    harness.run();
    let state = harness.state_mut();
    assert!(state.project().view(plan).unwrap().sun.is_none());
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    state.session.doc.undo();
    assert!(state.project().view(plan).unwrap().sun.is_some());
}

/// Switching to lighting and back keeps the date; a built-in preset applies
/// its mode; a preset saved in the dialog goes away on Cancel.
#[test]
fn sun_settings_switch_modes_apply_and_save_presets() {
    use crate::i18n::tr;
    use cc_core::sun::SunMode;
    let mut state = sample_state();
    open_sun_dialog(&mut state, "still", false);
    let plan = state.active_view.unwrap();
    let mode = |h: &Harness<'static, AppState>| h.state().project().view(plan).unwrap().sun.clone().unwrap().mode;
    let mut harness = sun_harness(state);
    harness.get_by_label(&t!("sun-preset-march-equinox")).click();
    harness.run();
    assert!(matches!(mode(&harness), SunMode::Still { date, time: 720 } if date.iso() == "2026-03-20"));
    harness.get_by_label(&tr(SunMode::from_name("lighting").unwrap().key())).click();
    harness.run();
    assert!(matches!(mode(&harness), SunMode::Lighting { .. }));
    harness.get_by_label(&t!("sun-preset-light-left")).click();
    harness.run();
    assert!(matches!(mode(&harness), SunMode::Lighting { azimuth, .. } if azimuth == 225.0));
    harness.get_by_label(&tr(SunMode::from_name("still").unwrap().key())).click();
    harness.run();
    harness.get_by_label(&tr(SunMode::from_name("single_day").unwrap().key())).click();
    harness.run();
    assert!(matches!(mode(&harness), SunMode::SingleDay { date, .. } if date.iso() == "2026-03-20"));

    // Save the single-day sun as a preset, by name.
    harness.get_by_label(&t!("sun-preset-save")).click();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("Afternoon study");
    harness.run();
    harness.get_by_label(&t!("sun-preset-store")).click();
    harness.run();
    let presets = &harness.state().project().sun_presets;
    assert_eq!(presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Afternoon study"]);
    assert!(harness.query_by_label("Afternoon study").is_some(), "listed with the single-day presets");
    // The same name again is refused inline, with the engine's reason.
    harness.get_by_label(&t!("sun-preset-save")).click();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("Afternoon study");
    harness.run();
    harness.get_by_label(&t!("sun-preset-store")).click();
    harness.run();
    assert_eq!(harness.state().project().sun_presets.len(), 1);
    assert!(harness.state().dialogs.sun.is_some());
    assert!(harness.query_by_label(&t!("msg-sun-preset-exists", "name" => "Afternoon study")).is_some());
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    let state = harness.state();
    assert!(state.project().sun_presets.is_empty(), "Cancel undoes the saved preset");
    assert!(state.project().view(plan).unwrap().sun.as_ref().is_some_and(|s| matches!(s.mode, SunMode::Still { .. })));
}

/// The dialog's rows fit with the largest text size, in every kind of sun.
#[test]
fn sun_settings_fit_large_text() {
    for mode in cc_core::sun::SunMode::NAMES {
        let mut state = sample_state();
        open_sun_dialog(&mut state, mode, mode == "multi_day");
        let draw: fn(&mut egui::Ui, &mut AppState) = |ui, state| {
            crate::theme::set_font_scale(ui.ctx(), *crate::theme::FONT_SCALE_RANGE.end());
            crate::dialogs::show(ui.ctx(), state)
        };
        let mut harness = themed_harness(state, egui::vec2(1500.0, 1000.0), egui::Theme::Light, draw);
        for _ in 0..6 {
            harness.step();
        }
        assert!(harness.state().dialogs.sun.is_some(), "{mode}");
    }
}

/// Sun Settings is unavailable, with its reason, outside plans, 3D views,
/// elevations and sections.
#[test]
fn sun_settings_need_a_view_that_takes_a_sun() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let why = Some(t!("why-sun-view"));
    let schedule = sample_view(&state, |v| matches!(v.kind, cc_core::ViewKind::Schedule { .. }));
    let sheet = sample_view(&state, |v| matches!(v.kind, cc_core::ViewKind::Sheet { .. }));
    let plan = sample_view(&state, cc_core::View::is_plan);
    state.active_view = None;
    assert_eq!(UiCommand::SunSettings.unavailable(&state), why);
    for (view, expected) in [(schedule, why.clone()), (sheet, why.clone()), (plan, None)] {
        state.active_view = Some(view);
        assert_eq!(UiCommand::SunSettings.unavailable(&state), expected);
    }
    assert_eq!(UiCommand::SunSettings.engine(), Some("view_set"));
}

/// A plan's properties show its sun: the *Sun* button opens the dialog,
/// and the sun path row turns the diagram off.
#[test]
fn view_properties_show_the_sun() {
    use crate::i18n::tr;
    let mut state = sample_state();
    let plan = sample_view(&state, cc_core::View::is_plan);
    state.active_view = Some(plan);
    let draw: fn(&mut egui::Ui, &mut AppState) = |ui, state| {
        crate::properties::show(ui, state);
        crate::dialogs::show(ui.ctx(), state);
    };
    // Without a sun: the button says so, and no sun rows.
    let harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, draw);
    assert!(harness.query_by_label(&t!("sun-none")).is_some());
    assert!(harness.query_by_label(&t!("prop-sun-path")).is_none());
    let mut state = harness.into_state();
    let set = cc_engine::commands::views::ViewSet {
        view: cc_engine::Ref::Id(plan.0),
        sun: Some(Some(Default::default())),
        ..Default::default()
    };
    state.session.run(set).unwrap();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, draw);
    let still = tr(cc_core::sun::SunMode::default().key());
    assert!(harness.query_by_label(&still).is_some());
    harness.get_by_label(&t!("prop-sun-path")).click();
    harness.run();
    assert!(!harness.state().project().view(plan).unwrap().sun.as_ref().unwrap().show_path);
    // Shadows are cast in plans, elevations and sections too: the row is there and sets them.
    assert!(!harness.state().project().view(plan).unwrap().sun.as_ref().unwrap().shadows);
    harness.get_by_label(&t!("sun-shadows")).click();
    harness.run();
    assert!(harness.state().project().view(plan).unwrap().sun.as_ref().unwrap().shadows);
    harness.get_by_label(&still).click();
    harness.run();
    assert!(harness.state().dialogs.sun.is_some());
}

/// An elevation given a sun in Sun Settings starts with the conventional
/// sun casting shadows; *Conventional (45°)* brings it back from another
/// mode; the dialog and the view's properties show the shading rows.
#[test]
fn elevations_start_with_the_conventional_sun() {
    use crate::commands::UiCommand;
    use crate::i18n::tr;
    use cc_core::sun::SunMode;
    let mut state = sample_state();
    let elevation = sample_view(&state, |v| matches!(v.kind, cc_core::ViewKind::Elevation { .. }));
    state.active_view = Some(elevation);
    assert!(state.project().view(elevation).unwrap().sun.is_none());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SunSettings);
    let sun = |s: &AppState| s.project().view(elevation).unwrap().sun.clone().unwrap();
    assert_eq!(sun(&state).mode, SunMode::conventional());
    assert!(sun(&state).shadows);
    let mut harness = sun_harness(state);
    assert!(harness.query_by_label(&t!("prop-ambient-light")).is_some());
    assert!(harness.query_by_label(&t!("prop-shadow-intensity")).is_some());
    harness.get_by_label(&tr(SunMode::from_name("still").unwrap().key())).click();
    harness.run();
    assert!(matches!(sun(harness.state()).mode, SunMode::Still { .. }));
    harness.get_by_label(&t!("sun-conventional")).click();
    harness.run();
    assert_eq!(sun(harness.state()).mode, SunMode::conventional());
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    // The properties name the conventional sun and show the shading rows.
    let state = harness.into_state();
    let draw: fn(&mut egui::Ui, &mut AppState) = |ui, state| crate::properties::show(ui, state);
    let harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, draw);
    assert!(harness.query_by_label(&t!("sun-conventional")).is_some());
    assert!(harness.query_by_label(&t!("prop-ambient-light")).is_some());
    assert!(harness.query_by_label(&t!("prop-shadow-intensity")).is_some());
}

// ---------------------------------------------------------------- render window

/// Make the sample's 3D view active, give it `render` settings (a JSON patch,
/// when given) and open the render window on it.
fn open_render_window(state: &mut AppState, render: Option<serde_json::Value>) -> cc_core::ViewId {
    let view = sample_view(state, |v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. }));
    state.active_view = Some(view);
    if let Some(render) = render {
        state.session.execute("view_set", serde_json::json!({ "view": view.0, "render": render })).unwrap();
    }
    crate::render_ui::open(state);
    assert!(state.dialogs.render.is_some());
    view
}

fn render_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(1400.0, 1000.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state)
    })
}

/// *View → Render* is on the View tab, unavailable (with its reason) outside
/// 3D views, and names the engine command that renders without the window.
#[test]
fn render_needs_a_3d_view() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let plan = sample_view(&state, cc_core::View::is_plan);
    let three_d = sample_view(&state, |v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. }));
    state.active_view = Some(plan);
    state.ribbon_tab = RibbonTab::View;
    assert_eq!(UiCommand::Render.unavailable(&state), Some(t!("why-render-view")));
    assert_eq!(UiCommand::Render.engine(), Some("view_render"));
    let label = t!("action-render");
    let harness = ribbon_harness(state, 1600.0);
    let disabled = harness.get_by_role_and_label(Role::Button, &label).accesskit_node().is_disabled();
    assert!(disabled, "disabled on a plan");
    let mut state = harness.into_state();
    state.active_view = Some(three_d);
    assert_eq!(UiCommand::Render.unavailable(&state), None);
    let harness = ribbon_harness(state, 1600.0);
    let disabled = harness.get_by_role_and_label(Role::Button, &label).accesskit_node().is_disabled();
    assert!(!disabled, "enabled on a 3D view");
}

/// The window shows its three sections; a change writes the view's render
/// settings, and everything changed in the window is one undo step.
#[test]
fn the_render_window_edits_the_view_in_one_step() {
    let mut state = sample_state();
    let view = open_render_window(&mut state, None);
    assert!(state.project().view(view).unwrap().render.is_none());
    let depth = state.session.doc.undo_depth();
    let mut harness = render_harness(state);
    for section in [t!("render-quality"), t!("render-output"), t!("render-environment")] {
        assert!(harness.query_by_label(&section).is_some(), "{section}");
    }
    // The preset list (the defaults are the medium preset): Draft.
    let medium = t!("render-preset-medium");
    let combo = harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some(medium.as_str()));
    combo.expect("the preset list shows the medium preset").click();
    harness.run();
    harness.get_by_label(&t!("render-preset-draft")).click();
    harness.run();
    let render = |h: &Harness<'static, AppState>| h.state().project().view(view).unwrap().render.clone();
    let r = render(&harness).expect("the view has render settings");
    assert_eq!((r.samples, r.max_depth), (32, 4));
    harness.get_by_label(&t!("render-clay")).click();
    harness.run();
    assert!(render(&harness).unwrap().clay);
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1, "one undo step");
    // Close keeps the changes; one undo takes them all back.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().dialogs.render.is_none());
    assert!(render(&harness).is_some());
    harness.state_mut().session.doc.undo();
    assert!(render(&harness).is_none());
}

/// A tiny render runs to the end and gives an image of the output's size; a
/// region renders over it.
#[test]
fn a_tiny_render_completes() {
    let mut state = sample_state();
    let tiny = serde_json::json!({ "width": 16, "height": 16, "samples": 2, "max_depth": 2 });
    open_render_window(&mut state, Some(tiny));
    let mut harness = render_harness(state);
    harness.get_by_label(&t!("action-render")).click();
    harness.step();
    crate::render_ui::wait(harness.state_mut());
    harness.run();
    let w = harness.state().dialogs.render.as_ref().unwrap();
    assert!(!w.is_rendering());
    assert_eq!(w.phase_done(), Some((2, false)));
    let image = w.image().unwrap();
    assert_eq!((image.width, image.height), (16, 16));
    assert!(image.is_valid());
    assert!(image.pixels.iter().any(|p| p.iter().any(|c| *c > 0.0)), "something is lit");

    // A region keeps the rest of the image.
    let before = image.clone();
    let region = cc_raytracing::Region { x: 0, y: 0, width: 8, height: 8 };
    harness.state_mut().dialogs.render.as_mut().unwrap().set_region(region, [16, 16]);
    harness.get_by_label(&t!("action-render")).click();
    harness.step();
    crate::render_ui::wait(harness.state_mut());
    harness.run();
    let w = harness.state().dialogs.render.as_ref().unwrap();
    assert_eq!(w.phase_done(), Some((2, false)));
    let image = w.image().unwrap();
    assert_eq!((image.width, image.height), (16, 16));
    assert_eq!(image.pixels[16 * 15 + 15], before.pixels[16 * 15 + 15], "outside the region: the image before");
}

/// Closing the window while it renders closes it at once (the render stops by itself).
#[test]
fn closing_a_running_render_does_not_wait() {
    let mut state = sample_state();
    open_render_window(&mut state, Some(serde_json::json!({ "width": 400, "height": 300, "samples": 16384 })));
    let mut harness = render_harness(state);
    harness.get_by_label(&t!("action-render")).click();
    harness.step();
    assert!(harness.state().dialogs.render.as_ref().unwrap().is_rendering());
    let start = std::time::Instant::now();
    harness.key_press(egui::Key::Escape);
    harness.step();
    harness.step();
    assert!(harness.state().dialogs.render.is_none());
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

/// *Sun Settings…* hides the window behind Sun Settings, which opens it
/// again when it closes, Cancel too.
#[test]
fn sun_settings_from_the_render_window_come_back_to_it() {
    let mut state = sample_state();
    let view = open_render_window(&mut state, None);
    let mut harness = render_harness(state);
    harness.get_by_label(&t!("render-sun-settings")).click();
    harness.run();
    assert!(harness.state().dialogs.render.is_none());
    assert!(harness.state().dialogs.sun.is_some());
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.sun.is_none());
    assert!(harness.state().dialogs.render.is_some(), "the render window is back");
    assert!(harness.state().project().view(view).unwrap().sun.is_none(), "Cancel took the sun away");
}

/// `RENDER_WINDOW_PNG=path cargo nextest run -p cc_app render_window_picture --run-ignored all`
/// writes the window after a small render, for review.
#[test]
#[ignore]
fn render_window_picture() {
    let Ok(path) = std::env::var("RENDER_WINDOW_PNG") else { return };
    let mut state = sample_state();
    open_render_window(&mut state, Some(serde_json::json!({ "width": 320, "height": 180, "samples": 16 })));
    let mut harness = render_harness(state);
    harness.get_by_label(&t!("action-render")).click();
    harness.step();
    crate::render_ui::wait(harness.state_mut());
    for _ in 0..4 {
        harness.step();
    }
    harness.render().unwrap().save(path).unwrap();
}

/// The Tag Types dialog (*Annotate → Tag Types*): New makes a door tag
/// type and Cancel undoes it; Edit on a target's built-in default makes its
/// type and opens Type Properties, where a line is added and OK keeps it.
#[test]
fn tag_types_dialog_creates_and_edits_types() {
    use crate::tag_type_ui::{Kind, Pick};
    use cc_core::tag::TagTarget;
    let mut state = sample_state();
    // Doors tagged with no type of their own: the built-in default.
    let doors: Vec<u64> = state.project().tag_types_of(TagTarget::Door).map(|t| t.id.0).collect();
    for id in doors {
        state.session.execute("type_delete", serde_json::json!({"type": id})).unwrap();
    }
    crate::commands::run(&mut state, &egui::Context::default(), crate::commands::UiCommand::TagTypes);
    let mut harness = themed_harness(state, egui::vec2(1100.0, 1000.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state)
    });
    let doors = |h: &Harness<'static, AppState>| h.state().project().tag_types_of(TagTarget::Door).count();
    let pick_door = |h: &mut Harness<'static, AppState>| {
        h.state_mut().dialogs.tag_types.as_mut().unwrap().selected = Some(Pick::Default(Kind::Tag(TagTarget::Door)));
        h.run();
    };
    assert_eq!(doors(&harness), 0);
    pick_door(&mut harness);
    harness.get_by_label(&t!("tag-types-new")).click();
    harness.run();
    assert_eq!(doors(&harness), 1);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert!(harness.state().dialogs.tag_types.is_none());
    assert_eq!(doors(&harness), 0, "Cancel undoes it");

    crate::tag_type_ui::open_list(harness.state_mut());
    pick_door(&mut harness);
    harness.get_by_label(&t!("tag-types-edit")).click();
    harness.run();
    assert!(harness.state().dialogs.tag_types.is_none());
    let id = harness.state().dialogs.type_editor.expect("Type Properties of the new type");
    assert_eq!(doors(&harness), 1);
    let lines = |h: &Harness<'static, AppState>| match h.state().project().element_type(id).map(|t| &t.data) {
        Some(cc_core::TypeData::Tag { style, .. }) => style.lines.len(),
        other => panic!("not a tag type: {other:?}"),
    };
    assert_eq!(lines(&harness), 1);
    // The dialog settles where it stands over a few frames.
    for _ in 0..8 {
        harness.step();
    }
    harness.get_by_label(&t!("tag-line-add")).click();
    harness.run();
    assert_eq!(lines(&harness), 2, "{}", harness.state().message);
    for _ in 0..8 {
        harness.step();
    }
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.type_editor.is_none());
    assert_eq!(lines(&harness), 2, "OK keeps it");
}

/// A door selected in a plan shows its tag's section, whose Reset Tag puts
/// back a tag moved there; with nothing selected the plan's properties have
/// a tag type per target, and a section's its mark type.
#[test]
fn the_tag_section_adjusts_the_selected_elements_tag() {
    let mut state = sample_state();
    let p = state.project();
    let door = p.elements.values().find(|e| matches!(e.kind, cc_core::ElementKind::Door(_))).unwrap();
    let (door, level) = (door.id, door.level);
    let plan = p.views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    state.selection = [door].into_iter().collect();
    let args = serde_json::json!({"view": plan.0, "ids": [door.0], "offset": [1.0, 0.5], "rotation": 30.0});
    state.session.execute("tag_set", args).unwrap();
    let mut harness = themed_harness(state, egui::vec2(360.0, 2400.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.get_by_label(&t!("section-tag"));
    harness.get_by_label(&t!("action-reset-tag")).click();
    harness.run();
    let tags = |h: &Harness<'static, AppState>| h.state().project().view(plan).unwrap().tags.clone();
    assert!(tags(&harness).is_empty(), "{:?}", tags(&harness));
    harness.state_mut().selection.clear();
    harness.run();
    harness.get_by_label(&t!("section-tag-types"));
    let section = harness.state().project().views.values().find(|v| v.category() == cc_core::ViewCategory::Section);
    if let Some(section) = section.map(|v| v.id) {
        harness.state_mut().active_view = Some(section);
        harness.run();
        harness.get_by_label(&t!("section-tag-types"));
    }
}

/// Issue #406: Tab steps through what lies under the pointer in a plan.
/// Over a wall of the sample house the wall comes first and the chain of
/// walls joined to it next; the status bar names the candidate, a click
/// selects it (the whole chain), and moving the pointer starts over. The
/// app's raw input hook turns Tab into steps (tested in `preselect`); the
/// harness skips it, so the test sets the steps as the hook would.
#[test]
fn tab_cycles_the_preselection_on_the_canvas() {
    use crate::preselect::{Candidate, label};
    /// Where the view is centered (the drawing closure cannot capture).
    static ORIGIN: std::sync::OnceLock<glam::DVec2> = std::sync::OnceLock::new();
    let mut state = sample_state();
    // A point on a wall where a click takes the wall (no grid, opening or tag there).
    let walls: Vec<_> = state.project().walls().map(|(e, w)| (e.id, e.level, w.start, w.end)).collect();
    let (wall, plan, mid) = walls
        .into_iter()
        .flat_map(|(wall, level, a, b)| {
            let across = (b - a).perp().normalize_or_zero();
            (1..10).flat_map(move |k| [-0.1, 0.1].map(|off| (wall, level, a.lerp(b, k as f64 / 10.0) + across * off)))
        })
        .find_map(|(wall, level, at)| {
            let p = state.project();
            let plan = p.views.values().find(|v| v.is_plan() && v.level() == Some(level))?.id;
            let drawing = state.drawing(plan)?;
            let first = drawing.hits_at(at, 5.0 / 20.0).first().copied();
            let tagged = drawing.tags.iter().any(|t| t.center.distance(at) < 2.0);
            (first == Some(wall) && !tagged).then_some((wall, plan, at))
        })
        .unwrap();
    ORIGIN.set(mid).unwrap();
    let p = state.project();
    let chain = cc_core::wall_ops::wall_chain(p, wall);
    assert!(chain.len() > 1, "the sample's walls are joined: {chain:?}");
    let wall_text = label(p, &Candidate::One(wall), None).unwrap();
    state.active_view = Some(plan);
    state.set_tool(crate::tools::Tool::Select);
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        state.hint.begin_frame();
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let Some(origin) = ORIGIN.get().copied() else { return };
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin, scale: 20.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
        state.hint.end_frame();
    });
    harness.run();
    let center = egui::pos2(300.0, 200.0);
    let point = |h: &mut Harness<'static, AppState>, pos: egui::Pos2| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.run();
    };
    let tab = |h: &mut Harness<'static, AppState>, steps: i64| {
        h.state_mut().interaction.tab.steps = steps;
        h.run();
    };
    let click = |h: &mut Harness<'static, AppState>, pos: egui::Pos2| {
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
            h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
            h.run();
        }
    };
    let hint = |h: &Harness<'static, AppState>| h.state().hint.shown().map(str::to_string);
    let index = |h: &Harness<'static, AppState>| h.state().interaction.tab.cycle.map(|c| c.index);

    // Over the wall: it is pre-selected, the status bar says Tab steps through
    // what else is there, and the canvas takes Tab next frame.
    point(&mut harness, center);
    // Five candidates are under the pointer: the wall, its chain and what lies there.
    let tab_hint = t!("status-preselect-tab", "label" => wall_text.clone(), "count" => 5);
    assert_eq!(hint(&harness), Some(tab_hint.clone()));
    assert!(harness.state().interaction.tab.armed.is_some());
    assert_eq!(index(&harness), None);
    // Tab: the chain, counted among the candidates.
    tab(&mut harness, 1);
    assert_eq!(index(&harness), Some(1));
    let shown = hint(&harness).unwrap();
    assert!(shown.starts_with(&t!("status-wall-chain")) && shown.contains(" (2/"), "{shown}");
    // Shift+Tab twice wraps round to the last candidate, Tab once more to the first.
    tab(&mut harness, -2);
    let last = index(&harness).unwrap();
    let shown = hint(&harness).unwrap();
    assert!(last >= 1 && shown.ends_with(&format!("({}/{})", last + 1, last + 1)), "{last} {shown}");
    tab(&mut harness, 1);
    assert_eq!(index(&harness), Some(0));
    let shown = hint(&harness).unwrap();
    assert!(shown.starts_with(&wall_text) && shown.contains(" (1/"), "cycled: the count follows: {shown}");
    // Within 3 points the cycle holds; farther it starts over.
    tab(&mut harness, 1);
    point(&mut harness, center + egui::vec2(2.0, 0.0));
    assert_eq!(index(&harness), Some(1));
    point(&mut harness, center + egui::vec2(6.0, 0.0));
    assert_eq!(index(&harness), None);
    // Back over the wall: the wall again, with no count.
    point(&mut harness, center);
    assert_eq!((index(&harness), hint(&harness)), (None, Some(tab_hint.clone())));
    // A click on the chain selects all its walls and ends the cycle.
    tab(&mut harness, 1);
    click(&mut harness, center);
    let selected: Vec<_> = harness.state().selection.iter().copied().collect();
    let mut expected = chain.clone();
    expected.sort();
    assert_eq!(selected, expected);
    assert_eq!(index(&harness), None);
    // Without Tab a click takes the wall alone, as before.
    click(&mut harness, center);
    assert_eq!(harness.state().selection.iter().copied().collect::<Vec<_>>(), vec![wall]);
    // Off the canvas nothing is pre-selected and no step is taken.
    harness.input_mut().events.push(egui::Event::PointerGone);
    harness.run();
    assert_eq!(hint(&harness), None);
    tab(&mut harness, 1);
    assert_eq!((harness.state().interaction.tab.steps, index(&harness)), (1, None));
}

/// In a plan a room's tag is dragged to a new spot (a click on it still
/// selects the room), turned by the grip of the room selected alone, hidden
/// from its context menu; a grid's bubble is dragged with an elbow.
#[test]
fn tags_and_bubbles_are_dragged_turned_and_hidden_on_the_canvas() {
    use glam::DVec2;
    let mut state = sample_state();
    let p = state.project();
    let room = p.elements.values().find(|e| matches!(e.kind, cc_core::ElementKind::Room(_))).unwrap();
    let (room, level) = (room.id, room.level);
    let plan = p.views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    state.set_tool(crate::tools::Tool::Select);
    /// The view drawn around the first room's point, 20 points a meter.
    fn xf_of(state: &AppState, rect: egui::Rect) -> crate::paint::Xf {
        let origin = state
            .project()
            .elements
            .values()
            .find_map(|e| match &e.kind {
                cc_core::ElementKind::Room(r) => Some(r.position),
                _ => None,
            })
            .unwrap_or_default();
        crate::paint::Xf { screen: rect.center(), origin, scale: 20.0 }
    }
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = xf_of(state, resp.rect);
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
    });
    harness.run();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
    let screen = |h: &Harness<'static, AppState>, p: DVec2| xf_of(h.state(), rect).to_screen(p);
    let handle = |h: &mut Harness<'static, AppState>| {
        h.state_mut().drawing(plan).unwrap().tags.iter().find(|t| t.element == room).copied().unwrap()
    };
    let press = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, button: egui::PointerButton, pressed: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers: Default::default() });
        h.run();
    };
    let drag = |h: &mut Harness<'static, AppState>, from: egui::Pos2, by: egui::Vec2| {
        press(h, from, egui::PointerButton::Primary, true);
        for k in 1..=4 {
            h.input_mut().events.push(egui::Event::PointerMoved(from + by * (k as f32 / 4.0)));
            h.run();
        }
        press(h, from + by, egui::PointerButton::Primary, false);
    };
    let placement = |h: &Harness<'static, AppState>, id: cc_core::ElementId| {
        h.state().project().view(plan).unwrap().tags.get(&id).copied().unwrap_or_default()
    };

    // A click on the room's tag selects the room.
    let tag = handle(&mut harness);
    let at = screen(&harness, tag.center);
    press(&mut harness, at, egui::PointerButton::Primary, true);
    press(&mut harness, at, egui::PointerButton::Primary, false);
    assert_eq!(harness.state().selection.iter().copied().collect::<Vec<_>>(), vec![room]);
    // A drag moves the tag 1 m to the right; the room stays.
    let position = |h: &Harness<'static, AppState>| match &h.state().project().element(room).unwrap().kind {
        cc_core::ElementKind::Room(r) => r.position,
        other => panic!("not a room: {other:?}"),
    };
    let before = position(&harness);
    drag(&mut harness, at, egui::vec2(20.0, 0.0));
    assert!(
        (placement(&harness, room).offset - DVec2::new(1.0, 0.0)).length() < 1e-6,
        "{:?}",
        placement(&harness, room)
    );
    assert_eq!(position(&harness), before);
    // The grip past its right side turns it a quarter turn.
    let tag = handle(&mut harness);
    let grip = screen(&harness, tag.rotate_grip(14.0 / 20.0));
    let center = screen(&harness, tag.center);
    drag(&mut harness, grip, (center - grip) + egui::vec2(0.0, -40.0));
    assert_eq!(placement(&harness, room).rotation_deg, Some(90.0));
    assert!(handle(&mut harness).rotation > 1.5);
    // Its context menu hides it.
    let center = handle(&mut harness).center;
    let at = screen(&harness, center);
    press(&mut harness, at, egui::PointerButton::Secondary, true);
    press(&mut harness, at, egui::PointerButton::Secondary, false);
    harness.get_by_label(&t!("menu-hide-tag")).click();
    harness.run();
    assert!(placement(&harness, room).hidden);

    // A grid's end bubble is dragged off its line.
    let origin = xf_of(harness.state(), rect).origin;
    let args = serde_json::json!({
        "start": (origin + DVec2::new(-6.0, -6.0)).to_array(),
        "end": (origin + DVec2::new(-6.0, -3.0)).to_array(),
        "name": "Z",
        "level": level.0,
    });
    let grid =
        cc_core::ElementId(harness.state_mut().session.execute("grid_create", args).unwrap()["id"].as_u64().unwrap());
    harness.state_mut().selection.clear();
    harness.run();
    let bubble =
        harness.state_mut().drawing(plan).unwrap().bubbles.iter().find(|b| b.element == grid).copied().unwrap();
    let at = screen(&harness, bubble.center);
    drag(&mut harness, at, egui::vec2(-20.0, 0.0));
    assert!(
        (placement(&harness, grid).offset - DVec2::new(-1.0, 0.0)).length() < 1e-6,
        "{:?}",
        placement(&harness, grid)
    );
}

/// A straight wall 6 m long and 3 m high far from the sample house, with a
/// door at `door` meters along it (none for `None`).
fn profile_wall(state: &mut AppState, door: Option<f64>) -> cc_core::ElementId {
    let level = state.project().levels[0].id.0;
    let args = serde_json::json!({"level": level, "start": [100, 100], "end": [106, 100], "height": 3.0});
    let wall = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    if let Some(offset) = door {
        state.session.execute("door_create", serde_json::json!({"wall": wall.0, "offset": offset})).unwrap();
    }
    wall
}

/// The wall's rectangle with its top sloping from 3 m at the start to `end` at the end.
fn sloped_top(end: f64) -> Vec<cc_core::sketch::SketchCurve> {
    use glam::DVec2;
    let outline = [DVec2::ZERO, DVec2::new(6.0, 0.0), DVec2::new(6.0, end), DVec2::new(0.0, 3.0)];
    cc_core::sketch::Sketch::from_polygons(&outline, &[]).curves
}

fn wall_profile(state: &AppState, wall: cc_core::ElementId) -> Option<cc_core::wall_profile::WallProfile> {
    state.project().element(wall).and_then(|e| e.as_wall()).and_then(|w| w.profile.clone())
}

/// The properties palette, while no sketch is open.
fn wall_palette_harness(state: AppState) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        if state.sketch.is_none() {
            crate::properties::show(ui, state);
        }
    })
}

/// Issue #306: *Edit Profile* takes one straight wall; an arc wall says why
/// not (and its palette has no Profile section), and *Reset Profile* needs a
/// profile.
#[test]
fn edit_wall_profile_follows_the_selected_wall() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let wall = profile_wall(&mut state, None);
    let level = state.project().levels[0].id.0;
    let args = serde_json::json!({"level": level, "start": [100, 110], "end": [106, 110], "bulge": 0.3});
    let arc = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    assert_eq!(UiCommand::EditWallProfile.unavailable(&state), Some(t!("why-select-wall")));
    state.selection = [arc].into_iter().collect();
    assert_eq!(UiCommand::EditWallProfile.unavailable(&state), Some(t!("why-wall-profile-arc")));
    state.selection = [wall].into_iter().collect();
    assert_eq!(UiCommand::EditWallProfile.unavailable(&state), None);
    assert_eq!(UiCommand::ResetWallProfile.unavailable(&state), Some(t!("why-wall-no-profile")));

    let mut harness = wall_palette_harness(state);
    harness.run();
    assert!(harness.query_by_label(&t!("wall-profile-rectangular")).is_some());
    let (edit_label, reset_label) = (t!("edit-wall-profile"), t!("reset-wall-profile"));
    let edit = harness.get_by_role_and_label(Role::Button, &edit_label);
    assert!(!edit.accesskit_node().is_disabled());
    let reset = harness.get_by_role_and_label(Role::Button, &reset_label);
    assert!(reset.accesskit_node().is_disabled(), "no profile to reset");
    harness.state_mut().selection = [arc].into_iter().collect();
    harness.run();
    assert!(harness.query_by_label(&t!("wall-profile-rectangular")).is_none(), "arc walls take no profile");
}

/// Issue #306: the wall's profile opens on its nominal rectangle in the
/// profile canvas; a sloped top finishes in one undo step and leaves the
/// sketch; Cancel changes nothing; an outline past the wall's end stays open
/// with the reason; *Reset Profile* takes the profile off.
#[test]
fn a_wall_profile_is_sketched_in_its_own_canvas() {
    use glam::DVec2;
    let mut state = sample_state();
    let wall = profile_wall(&mut state, Some(1.5));
    state.selection = [wall].into_iter().collect();
    let depth = state.session.doc.undo_depth();
    let mut harness = wall_palette_harness(state);
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("edit-wall-profile")).click();
    harness.run();
    assert!(crate::profile_sketch::is_open(harness.state()));
    assert!(crate::wall_profile_ui::is_open(harness.state()));
    let rect = [DVec2::ZERO, DVec2::new(6.0, 0.0), DVec2::new(6.0, 3.0), DVec2::new(0.0, 3.0)];
    let nominal = cc_core::sketch::Sketch::from_polygons(&rect, &[]).curves;
    assert_eq!(harness.state().sketch.as_ref().unwrap().curves, nominal, "the nominal rectangle");

    // The canvas and its ribbon: lines and arcs only.
    let mut harness = sketch_harness(harness.into_state());
    harness.run();
    let around = crate::wall_profile_ui::context(harness.state().project(), wall, &nominal).unwrap();
    assert_eq!((around.length, around.height, around.openings.len()), (6.0, 3.0, 1), "the lines and the door");
    assert!(!around.levels.is_empty(), "the levels near the wall");
    let circle = crate::commands::UiCommand::SketchCircle.label(harness.state());
    assert!(harness.query_by_label(&circle).is_none(), "no circles");

    // Cancel changes nothing.
    let side = cc_core::sketch::SketchCurve::Line { a: DVec2::new(6.0, 0.0), b: DVec2::new(6.0, 2.0) };
    harness.state_mut().sketch.as_mut().unwrap().set_curve(1, side);
    harness.get_by_role_and_label(Role::Button, &t!("sketch-cancel-mode")).click();
    harness.run();
    assert!(harness.state().sketch.is_none());
    assert!(wall_profile(harness.state(), wall).is_none());
    assert_eq!(harness.state().session.doc.undo_depth(), depth);

    // Past the wall's end: refused, the sketch stays with the reason.
    assert!(crate::wall_profile_ui::open(harness.state_mut(), wall));
    let past = [DVec2::ZERO, DVec2::new(7.0, 0.0), DVec2::new(7.0, 3.0), DVec2::new(0.0, 3.0)];
    harness.state_mut().sketch.as_mut().unwrap().curves = cc_core::sketch::Sketch::from_polygons(&past, &[]).curves;
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("sketch-finish-mode")).click();
    harness.run();
    assert!(crate::wall_profile_ui::is_open(harness.state()), "the sketch stays open");
    assert_eq!(harness.state().message, t!("wall-profile-out-of-bounds"));
    assert_eq!(harness.state().session.doc.undo_depth(), depth);

    // A sloped top: one undo step, the wall stays selected, the door fits.
    harness.state_mut().sketch.as_mut().unwrap().curves = sloped_top(2.0);
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("sketch-finish-mode")).click();
    harness.run();
    assert!(harness.state().sketch.is_none(), "{}", harness.state().message);
    let profile = wall_profile(harness.state(), wall).unwrap();
    assert!((profile.z_range().1 - 3.0).abs() < 1e-9 && (profile.area() - 15.0).abs() < 1e-9, "{profile:?}");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1, "one undo step");
    assert_eq!(harness.state().selection, [wall].into_iter().collect());
    assert_eq!(crate::wall_profile_ui::misfits(harness.state().project(), wall), 0);
    // Opened again it shows the profile's curves; Finish unchanged adds no step.
    assert!(crate::wall_profile_ui::open(harness.state_mut(), wall));
    assert_eq!(harness.state().sketch.as_ref().unwrap().curves, sloped_top(2.0));
    crate::sketch_mode::finish(harness.state_mut());
    assert!(harness.state().sketch.is_none());
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
    // Undone in one step.
    harness.state_mut().session.doc.undo();
    assert!(wall_profile(harness.state(), wall).is_none());
    harness.state_mut().session.doc.redo();
    assert!(wall_profile(harness.state(), wall).is_some());

    // Reset Profile from the palette.
    let mut harness = wall_palette_harness(harness.into_state());
    harness.run();
    assert!(harness.query_by_label(&t!("wall-profile-custom")).is_some());
    harness.get_by_role_and_label(Role::Button, &t!("reset-wall-profile")).click();
    harness.run();
    assert!(wall_profile(harness.state(), wall).is_none());
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 2);
    let reset_label = t!("reset-wall-profile");
    let reset = harness.get_by_role_and_label(Role::Button, &reset_label);
    assert!(reset.accesskit_node().is_disabled());
}

/// Issue #306: a profile that leaves a hosted door outside the wall's body
/// says so in the status bar.
#[test]
fn a_wall_profile_that_leaves_a_door_out_warns() {
    let mut state = sample_state();
    let wall = profile_wall(&mut state, Some(4.5));
    state.selection = [wall].into_iter().collect();
    crate::commands::run(&mut state, &egui::Context::default(), crate::commands::UiCommand::EditWallProfile);
    assert!(crate::wall_profile_ui::is_open(&state));
    state.sketch.as_mut().unwrap().curves = sloped_top(1.0);
    crate::sketch_mode::finish(&mut state);
    assert!(state.sketch.is_none(), "{}", state.message);
    assert!(wall_profile(&state, wall).is_some());
    assert_eq!(state.message, t!("wall-profile-openings-out", "count" => 1));
    // What the geometry leaves out is what the warning counts.
    let door = state.project().hosted_by(wall).map(|e| e.id).next().unwrap();
    assert!(state.session.geometry().walls.get(&wall).unwrap().openings.iter().all(|o| o.element != door));
}

/// Where the spline wall tests draw: far from the sample house, so its
/// snaps stay out of the way, 100 points a meter.
const SPLINE_ORIGIN: glam::DVec2 = glam::DVec2::new(200.0, 200.0);

/// The sample house's first plan, active, with the wall tool in Spline mode.
fn spline_state() -> AppState {
    let mut state = sample_state();
    let level = state.project().levels[0].id;
    let plan = state.project().views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    let wall_type = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.set_tool(crate::tools::Tool::Wall { type_id: wall_type });
    state.wall_mode = crate::tools::WallMode::Spline;
    state
}

/// The active plan's canvas around [`SPLINE_ORIGIN`] (with the app's keys),
/// in `theme`.
fn spline_canvas(state: AppState, theme: egui::Theme) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(600.0, 400.0), theme, |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin: SPLINE_ORIGIN, scale: 100.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
    })
}

/// The screen point of `p` (meters from [`SPLINE_ORIGIN`]) in [`spline_canvas`].
fn spline_screen(p: [f64; 2]) -> egui::Pos2 {
    let xf = crate::paint::Xf { screen: egui::pos2(300.0, 200.0), origin: SPLINE_ORIGIN, scale: 100.0 };
    xf.to_screen(SPLINE_ORIGIN + glam::DVec2::from(p))
}

fn spline_press(h: &mut Harness<'static, AppState>, pos: egui::Pos2, pressed: bool) {
    h.input_mut().events.push(egui::Event::PointerMoved(pos));
    let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
    h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
    h.run();
}

fn spline_click(h: &mut Harness<'static, AppState>, p: [f64; 2]) {
    spline_press(h, spline_screen(p), true);
    spline_press(h, spline_screen(p), false);
}

fn spline_hover(h: &mut Harness<'static, AppState>, p: [f64; 2]) {
    h.input_mut().events.push(egui::Event::PointerMoved(spline_screen(p)));
    h.run();
}

fn walls_of(state: &AppState) -> Vec<cc_core::Element> {
    state.project().elements.values().filter(|e| e.as_wall().is_some()).cloned().collect()
}

/// Issue #307: the wall tool's Spline mode makes one wall through the
/// points clicked when Enter is pressed, in one undo step; Backspace takes
/// back the last point, and the tool stays for the next wall.
#[test]
fn the_wall_tool_draws_spline_walls() {
    let mut harness = spline_canvas(spline_state(), egui::Theme::Light);
    let (walls, depth) = (walls_of(harness.state()).len(), harness.state().session.doc.undo_depth());
    for p in [[-1.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.5, 0.5]] {
        spline_click(&mut harness, p);
    }
    assert_eq!(harness.state().interaction.points.len(), 4);
    harness.key_press(egui::Key::Backspace);
    harness.run();
    assert_eq!(harness.state().interaction.points.len(), 3, "Backspace takes back the last point");
    // The points as snapped (to lengths and angles), near those clicked.
    let picked = harness.state().interaction.points.clone();
    for (q, p) in picked.iter().zip([[-1.0, 0.0], [0.0, 1.0], [1.0, 0.0]]) {
        assert!(q.distance(SPLINE_ORIGIN + glam::DVec2::from(p)) < 0.05, "{q}");
    }
    spline_hover(&mut harness, [2.0, -1.0]);
    harness.key_press(egui::Key::Enter);
    harness.run();
    let state = harness.state();
    assert!(state.message.is_empty(), "{}", state.message);
    let new = walls_of(state);
    assert_eq!(new.len(), walls + 1);
    let wall = new.iter().find(|e| e.as_wall().unwrap().spline.is_some()).expect("a spline wall");
    let spline = wall.as_wall().unwrap().spline.clone().unwrap();
    let knots: Vec<glam::DVec2> = spline.knots().collect();
    assert_eq!(knots, picked, "through the points picked");
    assert_eq!(state.session.doc.undo_depth(), depth + 1, "one undo step");
    assert!(matches!(state.tool, crate::tools::Tool::Wall { .. }));
    assert!(state.interaction.points.is_empty(), "ready for the next wall");
    // Escape drops a single point: it makes no wall.
    spline_click(&mut harness, [2.5, 0.0]);
    assert_eq!(harness.state().interaction.points.len(), 1);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().interaction.points.is_empty());
    assert_eq!(walls_of(harness.state()).len(), walls + 1);
}

/// Issue #401: Esc ends a spline wall where it is, as it ends a chain of
/// straight walls, instead of dropping the points clicked; the tool stays,
/// and the next Esc leaves it.
#[test]
fn escape_ends_a_spline_wall_where_it_is() {
    let mut harness = spline_canvas(spline_state(), egui::Theme::Light);
    let (walls, depth) = (walls_of(harness.state()).len(), harness.state().session.doc.undo_depth());
    for p in [[-1.0, 0.0], [0.0, 1.0], [1.0, 0.0]] {
        spline_click(&mut harness, p);
    }
    let picked = harness.state().interaction.points.clone();
    assert_eq!(picked.len(), 3);
    harness.key_press(egui::Key::Escape);
    harness.run();
    let state = harness.state();
    assert!(state.message.is_empty(), "{}", state.message);
    let new = walls_of(state);
    assert_eq!(new.len(), walls + 1, "the wall is made");
    let wall = new.iter().find(|e| e.as_wall().unwrap().spline.is_some()).expect("a spline wall");
    let knots: Vec<glam::DVec2> = wall.as_wall().unwrap().spline.clone().unwrap().knots().collect();
    assert_eq!(knots, picked, "through the points picked");
    assert_eq!(state.session.doc.undo_depth(), depth + 1, "one undo step");
    assert!(matches!(state.tool, crate::tools::Tool::Wall { .. }), "the tool stays");
    assert!(state.interaction.points.is_empty());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().tool, crate::tools::Tool::Select, "the next Esc leaves the tool");
    assert_eq!(walls_of(harness.state()).len(), walls + 1);
}

/// Esc on a curve that makes no wall drops it and says why (issue #401).
#[test]
fn escape_drops_a_spline_wall_that_cannot_be_made() {
    let mut harness = spline_canvas(spline_state(), egui::Theme::Light);
    let walls = walls_of(harness.state()).len();
    for p in [[-0.2, 0.0], [0.0, 0.2], [0.2, 0.0]] {
        spline_click(&mut harness, p);
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    let state = harness.state();
    assert_eq!(state.message, crate::i18n::tr("wall-spline-tight"));
    assert!(state.interaction.points.is_empty(), "the points are dropped");
    assert_eq!(walls_of(state).len(), walls);
}

/// A spline too tight for the wall's thickness is not made: the status bar
/// says why, and the points stay to be changed.
#[test]
fn a_spline_wall_too_tight_is_refused_with_the_reason() {
    let state = spline_state();
    let points = [[-0.2, 0.0], [0.0, 0.2], [0.2, 0.0]];
    let fit: Vec<glam::DVec2> = points.iter().map(|p| SPLINE_ORIGIN + glam::DVec2::from(*p)).collect();
    let spline = cc_core::wall_spline::WallSpline::through(&fit).unwrap();
    let ty = state.tool.type_id().unwrap();
    let check = state.project().check_wall_spline(ty, state.wall_location_line, &spline);
    assert!(matches!(check, Err(cc_core::wall_spline::SplineError::Tight(_))), "{check:?}");
    let mut harness = spline_canvas(state, egui::Theme::Light);
    let (walls, depth) = (walls_of(harness.state()).len(), harness.state().session.doc.undo_depth());
    for p in points {
        spline_click(&mut harness, p);
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    let state = harness.state();
    assert_eq!(state.message, crate::i18n::tr("wall-spline-tight"));
    assert_eq!(walls_of(state).len(), walls);
    assert_eq!(state.session.doc.undo_depth(), depth);
    assert_eq!(state.interaction.points.len(), 3);
}

/// A spline wall made with the engine, from [`SPLINE_ORIGIN`].
fn spline_wall(state: &mut AppState) -> cc_core::ElementId {
    let at = |x: f64, y: f64| (SPLINE_ORIGIN + glam::DVec2::new(x, y)).to_array();
    let args = serde_json::json!({"start": at(-1.0, 0.0), "end": at(1.0, 0.0), "through": [at(0.0, 0.6)]});
    cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap())
}

/// The Select tool shows a grip on each point a selected spline wall passes
/// through; dragging one moves it with its handles, in one undo step.
#[test]
fn a_spline_walls_knot_grip_reshapes_it() {
    let mut state = spline_state();
    let wall = spline_wall(&mut state);
    state.set_tool(crate::tools::Tool::Select);
    state.selection = [wall].into_iter().collect();
    let mut harness = spline_canvas(state, egui::Theme::Light);
    let control = |h: &Harness<'static, AppState>| {
        h.state().project().element(wall).unwrap().as_wall().unwrap().spline.clone().unwrap().control
    };
    let (before, depth) = (control(&harness), harness.state().session.doc.undo_depth());
    let from = spline_screen([0.0, 0.6]);
    spline_press(&mut harness, from, true);
    for k in 1..=4 {
        harness.input_mut().events.push(egui::Event::PointerMoved(from + egui::vec2(0.0, 5.0 * k as f32)));
        harness.run();
    }
    spline_press(&mut harness, from + egui::vec2(0.0, 20.0), false);
    let after = control(&harness);
    assert!(harness.state().message.is_empty(), "{}", harness.state().message);
    let shift = after[3] - before[3];
    assert!((shift - glam::DVec2::new(0.0, -0.2)).length() < 1e-6, "{shift}");
    for i in [2, 4] {
        assert!((after[i] - before[i] - shift).length() < 1e-9, "handle {i} goes along");
    }
    assert_eq!((after[0], after[6]), (before[0], before[6]), "the ends stay");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
}

/// The door tool says spline walls take no doors yet, and places none.
#[test]
fn the_door_tool_places_no_door_in_a_spline_wall() {
    let mut state = spline_state();
    spline_wall(&mut state);
    let door = state.project().default_type(cc_core::Category::Door).unwrap();
    state.set_tool(crate::tools::Tool::Hosted { category: cc_core::Category::Door, type_id: door });
    let doors =
        |s: &AppState| s.project().elements.values().filter(|e| e.category() == cc_core::Category::Door).count();
    let (count, depth) = (doors(&state), state.session.doc.undo_depth());
    let mut harness = spline_canvas(state, egui::Theme::Light);
    spline_click(&mut harness, [0.0, 0.6]);
    assert_eq!(harness.state().message, t!("msg-spline-wall-hosts"));
    assert_eq!(doors(harness.state()), count);
    assert_eq!(harness.state().session.doc.undo_depth(), depth);
}

/// A spline wall's properties show its length, but no arc angle and no
/// elevation profile (it takes neither).
#[test]
fn a_spline_walls_palette_shows_its_length_only() {
    let mut state = spline_state();
    let wall = spline_wall(&mut state);
    // A straight wall of the same type, beside it.
    let at = |x: f64, y: f64| (SPLINE_ORIGIN + glam::DVec2::new(x, y)).to_array();
    let args = serde_json::json!({"start": at(-1.0, -2.0), "end": at(1.0, -2.0)});
    let straight = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    state.set_tool(crate::tools::Tool::Select);
    state.selection = [straight].into_iter().collect();
    let length = {
        let e = state.project().element(wall).unwrap();
        cc_views::format::length(state.project().wall_length(e, e.as_wall().unwrap()))
    };
    let mut harness = wall_palette_harness(state);
    harness.run();
    let spins = |h: &Harness<'static, AppState>| h.query_all_by_role(Role::SpinButton).count();
    let straight_spins = spins(&harness);
    assert!(harness.query_by_label(&t!("section-wall-profile")).is_some());
    harness.state_mut().selection = [wall].into_iter().collect();
    harness.run();
    assert!(harness.query_by_label(&length).is_some(), "{length}");
    assert_eq!(spins(&harness), straight_spins - 1, "no arc angle");
    assert!(harness.query_by_label(&t!("section-wall-profile")).is_none());
}

/// Trim/extend refuses a spline wall with the localized reason in the status bar.
#[test]
fn trimming_a_spline_wall_says_why_not() {
    let mut state = spline_state();
    let wall = spline_wall(&mut state);
    let at = |x: f64, y: f64| (SPLINE_ORIGIN + glam::DVec2::new(x, y)).to_array();
    let args = serde_json::json!({"start": at(3.0, -2.0), "end": at(3.0, 2.0)});
    let line = state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap();
    let command = cc_engine::commands::modify::TrimExtend { id: wall.0, pick: at(0.0, 1.0), reference: line };
    assert!(crate::actions::run_command(&mut state, command).is_none());
    assert_eq!(state.message, t!("msg-spline-wall"));
}

/// The view tabs of the app, with the open views list of each tab bar (#371).
struct Tabs {
    dock: egui_dock::DockState<cc_core::ViewId>,
    state: AppState,
}

fn tabs_harness(views: &[cc_core::ViewId]) -> Harness<'static, Tabs> {
    let mut state = sample_state();
    let mut dock = egui_dock::DockState::new(views.iter().take(1).copied().collect());
    if let Some(path) = views.first().and_then(|v| dock.find_tab(v)) {
        dock.set_focused_node_and_surface(path.node_path());
    }
    views.iter().for_each(|v| state.open_view(*v));
    let mut themed = false;
    let mut harness = Harness::builder().with_size(egui::vec2(520.0, 420.0)).build_ui_state(
        move |ui, tabs: &mut Tabs| {
            if !themed {
                crate::theme::apply(ui.ctx());
                themed = true;
                return;
            }
            crate::theme::sync(ui.ctx());
            crate::app::sync_tabs(&mut tabs.dock, &mut tabs.state);
            crate::app::show_views(ui, &mut tabs.dock, &mut tabs.state, None);
        },
        Tabs { dock, state },
    );
    harness.run();
    harness
}

/// The button at the end of the tab bar lists every open view; picking one
/// brings its tab to the front, scrolled into view, and Close Other Views
/// leaves only the active tab.
#[test]
fn the_tab_bar_lists_the_open_views() {
    let project = sample_state().session.doc.project;
    let mut views: Vec<(cc_core::ViewId, String)> = Vec::new();
    for category in [cc_core::ViewCategory::FloorPlan, cc_core::ViewCategory::Elevation] {
        for v in project.views_of(category) {
            let label = cc_core::browser::label(v);
            if !views.iter().any(|(_, l)| *l == label) {
                views.push((v.id, label));
            }
        }
    }
    assert!(views.len() >= 5, "enough views to crowd the bar");
    let ids: Vec<_> = views.iter().map(|(id, _)| *id).collect();
    let mut harness = tabs_harness(&ids);
    let last = *ids.last().unwrap();
    assert_eq!(harness.state().state.active_view, Some(last));
    // Whether a tab is inside its bar, short of the button at its end.
    let shown = |h: &Harness<'static, Tabs>, id| {
        let bar = h.state().dock.iter_leaves().next().unwrap().1.rect;
        let tab = h.state().state.tab_bar.tab_rects[&id];
        tab.left() >= bar.left() - 0.5 && tab.right() <= bar.right() - f32::from(crate::tab_list::BUTTON_WIDTH) + 0.5
    };
    assert!(shown(&harness, last), "the last view opened was scrolled into view");
    assert!(!shown(&harness, ids[0]), "the bar is crowded");

    harness.get_by_role_and_label(Role::Button, &t!("tabs-open-views")).click();
    harness.run();
    for (_, label) in &views {
        assert!(harness.query_by_label(label).is_some(), "{label} is listed");
    }
    // The active tab is checked.
    let (_, last_label) = views.last().unwrap();
    let toggled = |label: &str| harness.get_by_label(label).accesskit_node().toggled();
    assert_eq!(toggled(last_label), Some(egui::accesskit::Toggled::True));
    assert_eq!(toggled(&views[0].1), Some(egui::accesskit::Toggled::False));
    // Picking the first view, scrolled out of the crowded bar, brings it back.
    harness.get_by_label(&views[0].1).click();
    harness.run();
    assert_eq!(harness.state().state.active_view, Some(ids[0]));
    assert!(shown(&harness, ids[0]), "its tab is in view");

    harness.get_by_role_and_label(Role::Button, &t!("tabs-open-views")).click();
    harness.run();
    harness.get_by_label(&t!("action-close-other-views")).click();
    harness.run();
    let open: Vec<_> = harness.state().dock.iter_all_tabs().map(|(_, t)| *t).collect();
    assert_eq!(open, [ids[0]]);
    assert_eq!(harness.state().state.open_tabs, 1);
}

/// Issue #431: Close Other Views from the list of a bar whose active tab is
/// not the first left the bar pointing past its only tab, and the view was
/// not drawn (black) until its tab was clicked.
#[test]
fn closing_the_other_views_keeps_a_later_tab_shown() {
    let project = sample_state().session.doc.project;
    let plans = project.views_of(cc_core::ViewCategory::FloorPlan);
    let ids: Vec<_> = plans.chain(project.views_of(cc_core::ViewCategory::Elevation)).map(|v| v.id).take(3).collect();
    assert_eq!(ids.len(), 3);
    let mut harness = tabs_harness(&ids);
    harness.state_mut().state.open_view(ids[1]);
    harness.run();
    assert_eq!(harness.state().state.active_view, Some(ids[1]));

    harness.get_by_role_and_label(Role::Button, &t!("tabs-open-views")).click();
    harness.run();
    harness.get_by_label(&t!("action-close-other-views")).click();
    harness.run();
    let open: Vec<_> = harness.state().dock.iter_all_tabs().map(|(_, t)| *t).collect();
    assert_eq!(open, [ids[1]]);
    let (_, leaf) = harness.state().dock.iter_leaves().next().unwrap();
    assert_eq!(leaf.active.0, 0, "the bar's active tab is its only one");
    assert_eq!(harness.state().state.active_view, Some(ids[1]));
    // The view is drawn: its zoom button is there.
    assert!(harness.query_by_label(&t!("canvas-zoom-menu")).is_some());
}

/// The plugin `idf_tools`: an exporter with an option, one with only its
/// path, and an importer.
const IDF_TOOLS: &str = r#"
ccad.command { name = "export_idf", kind = "query", title = "Export IDF",
  params = { required = { "path" }, properties = {
    path = { type = "string", ui = "save_path", extensions = { "idf" }, title = "File" },
    comment = { type = "string" } } },
  run = function(a) ccad.file.write(a.path, "Version,24.1; ! " .. (a.comment or "")) end }
ccad.command { name = "export_plain", kind = "query",
  params = { properties = { path = { type = "string", ui = "save_path", extensions = { "txt" } } } },
  run = function(a) ccad.file.write(a.path, "plain") end }
ccad.command { name = "import_epw",
  params = { properties = { path = { type = "string", ui = "open_path", extensions = { "epw" } } } },
  run = function(a) ccad.data.set(nil, ccad.file.read(a.path)) end }
"#;

/// A plugins folder with `idf_tools` ([`IDF_TOOLS`]).
fn idf_plugins_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-plugin-idf-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("idf_tools");
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = "id = \"idf_tools\"\nname = \"IDF tools\"\nversion = \"1.0.0\"\napi = 2\n\n\
                    [[exporters]]\ncommand = \"export_idf\"\nlabel = \"EnergyPlus (.idf)\"\n\n\
                    [[exporters]]\ncommand = \"export_plain\"\nlabel = \"Plain text (.txt)\"\n\n\
                    [[importers]]\ncommand = \"import_epw\"\nlabel = \"Weather (.epw)\"\n";
    std::fs::write(dir.join("plugin.toml"), manifest).unwrap();
    std::fs::write(dir.join("main.lua"), IDF_TOOLS).unwrap();
    root
}

/// A plugin's exporters join the File tab's Export group and its importers
/// the Insert tab's Import group; a chosen path opens the
/// command's form with it set (its field has Browse…), or runs a command
/// whose only parameter is the path at once.
#[test]
fn plugin_exporters_and_importers_join_the_ribbon() {
    let folder = idf_plugins_folder("ribbon");
    let mut state = state_with_plugins(folder.clone());
    assert_eq!(state.plugins.exporters.len(), 2);
    assert_eq!(state.plugins.importers.len(), 1);
    state.ribbon_tab = RibbonTab::File;
    let mut harness = wide_ribbon_harness(state);
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("group-export")).click();
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "EnergyPlus (.idf)").is_some(), "File, Export page");
    assert!(harness.query_by_role_and_label(Role::Button, "Weather (.epw)").is_none(), "no Import group there");
    harness.state_mut().ribbon_tab = RibbonTab::Insert;
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "Weather (.epw)").is_some(), "Insert tab, Import");
    assert!(harness.query_by_role_and_label(Role::Button, "EnergyPlus (.idf)").is_none(), "exports only in File");
    // The dialog cannot be driven: the path it would return opens the form.
    let out = folder.join("model.idf");
    let entry = harness.state().plugins.exporters[0].clone();
    crate::plugins_ui::open_file_entry(harness.state_mut(), &entry, &out);
    harness.run();
    assert_eq!(form_field(harness.state(), "path").shown(), Some(&serde_json::json!(out.to_string_lossy())));
    assert!(harness.query_by_role_and_label(Role::Button, &t!("pform-browse")).is_some(), "the path field has Browse");
    harness.get_by_role_and_label(Role::Button, &t!("palette-run")).click();
    harness.run();
    assert!(std::fs::read_to_string(&out).unwrap().starts_with("Version,24.1;"));
    // A command whose only parameter is the path runs at once.
    let plain = folder.join("plain.txt");
    let entry = harness.state().plugins.exporters[1].clone();
    harness.state_mut().dialogs.plugin_form = None;
    crate::plugins_ui::open_file_entry(harness.state_mut(), &entry, &plain);
    assert!(harness.state().dialogs.plugin_form.is_none(), "it ran and had nothing to show");
    assert_eq!(std::fs::read_to_string(&plain).unwrap(), "plain");
}

/// A program on every CI system that waits about `secs` seconds, with its
/// arguments: the stand-in for a simulator.
fn sleeper(secs: u32) -> (std::path::PathBuf, Vec<String>) {
    if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").map_or_else(|| "C:\\Windows".into(), std::path::PathBuf::from);
        (root.join("System32").join("PING.EXE"), vec!["-n".into(), (secs + 1).to_string(), "127.0.0.1".into()])
    } else {
        ("/bin/sleep".into(), vec![secs.to_string()])
    }
}

/// A folder with the plugin `sim`: its command `simulate` draws a wall, then
/// runs its tool `sleeper` ([`sleeper`], chosen in `.tools/sim.json`) for
/// 30 seconds, then draws another wall.
fn sim_plugins_folder(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ccad-sim-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("sim");
    std::fs::create_dir_all(&dir).unwrap();
    let (program, args) = sleeper(30);
    let (_, test_args) = sleeper(0);
    let list = |a: &[String]| a.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ");
    let manifest = format!(
        "id = \"sim\"\nname = \"Simulator plugin\"\nversion = \"1.0\"\napi = 2\npermissions = [\"tools\"]\n\n\
         [[tools]]\nid = \"sleeper\"\nname = \"Sleeper\"\nexecutables = [\"ping.exe\", \"sleep\"]\n\
         homepage = \"https://example.org/sleeper\"\nversion_args = [{}]\n",
        list(&test_args)
    );
    std::fs::write(dir.join("plugin.toml"), manifest).unwrap();
    let main = format!(
        "ccad.command {{ name = \"simulate\", title = \"Simulate\", run = function()\n\
           ccad.run(\"wall_create\", {{ start = {{ 0, 0 }}, [\"end\"] = {{ 5, 0 }} }})\n\
           local r = ccad.process.run(\"sleeper\", {{ args = {{ {} }}, timeout = 120 }})\n\
           ccad.run(\"wall_create\", {{ start = {{ 0, 3 }}, [\"end\"] = {{ 5, 3 }} }})\n\
           return {{ code = r.code }}\n\
         end }}\n",
        list(&args)
    );
    std::fs::write(dir.join("main.lua"), main).unwrap();
    let paths = std::collections::BTreeMap::from([("sleeper".to_string(), program)]);
    cc_plugins::tools::write_tool_paths(&root, "sim", &paths).unwrap();
    root
}

/// A plugin command running an external program leaves the app working: a
/// modal dialog names the plugin and the program, shortcuts wait, and
/// Cancel stops the program and undoes what the command did.
#[test]
fn a_plugin_command_waiting_for_a_program_shows_progress_and_cancels() {
    let mut state = sample_state();
    state.prefs.plugins_tools.insert("sim".into());
    state.prefs.plugin_folder = Some(sim_plugins_folder("cancel"));
    crate::plugins_ui::load(&mut state);
    let (walls, depth) = (wall_count(&state), state.session.doc.undo_depth());
    crate::plugin_form::open(&mut state, "sim__simulate");
    assert!(state.session.has_job(), "{}", state.message);
    assert!(state.dialogs.plugin_form.as_ref().unwrap().is_waiting());
    assert!(crate::jobs::busy(&state), "shortcuts and dropped files wait");
    assert_eq!(wall_count(&state), walls + 1, "the first wall is drawn meanwhile");
    let mut harness = spinning_harness(state, egui::vec2(1200.0, 800.0), |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        crate::dialogs::show(ui.ctx(), state);
    });
    assert!(harness.query_by_label("Simulator plugin").is_some(), "the dialog is titled after the plugin");
    assert!(harness.query_by_label(&t!("job-plugin-running", "tool" => "Sleeper")).is_some());
    // The command's form waits behind it.
    assert!(harness.query_by_label(&t!("palette-run")).is_none());
    // Undo waits too: the command's step is still open.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(2);
    assert_eq!(wall_count(harness.state()), walls + 1);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run_steps(2);
    let state = harness.state();
    assert!(!state.session.has_job());
    assert!(state.dialogs.plugin_form.is_some(), "the form shows how it ended");
    assert_eq!(state.dialogs.plugin_form.as_ref().unwrap().error(), Some(t!("job-plugin-cancelled").as_str()));
    assert_eq!((wall_count(state), state.session.doc.undo_depth()), (walls, depth), "nothing stays");
    assert!(harness.query_by_label(&t!("job-plugin-running", "tool" => "Sleeper")).is_none());
}

/// The Plugins window grants the `tools` permission and chooses each tool's
/// program: a program of another name is refused, a chosen one is stored
/// and *Test* runs its version check.
#[test]
fn the_plugins_window_chooses_and_tests_external_programs() {
    let folder = sim_plugins_folder("window");
    let program = cc_plugins::tools::read_tool_paths(&folder, "sim")["sleeper"].clone();
    cc_plugins::tools::write_tool_paths(&folder, "sim", &Default::default()).unwrap();
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(folder.clone());
    crate::plugins_ui::load(&mut state);
    crate::plugins_ui::open(&mut state);
    state.plugins.selected = Some("sim".into());
    let mut harness = themed_harness(state, egui::vec2(1200.0, 900.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("plug-programs")).is_some());
    assert!(harness.query_by_label("Sleeper").is_some());
    assert!(harness.query_by_label(&t!("plug-program-none")).is_some());
    harness.get_by_label(&t!("plug-tools")).click();
    harness.run();
    assert!(harness.state().prefs.plugins_tools.contains("sim"), "the permission is granted");
    // Browse… picks a file: one of another name is refused.
    let other = folder.join("other.exe");
    std::fs::write(&other, "x").unwrap();
    crate::plugins_ui::choose_program_for_test(harness.state_mut(), "sim", "sleeper", &other);
    assert!(matches!(harness.state().plugins.notice, Some(Err(_))));
    assert!(cc_plugins::tools::read_tool_paths(&folder, "sim").is_empty());
    crate::plugins_ui::choose_program_for_test(harness.state_mut(), "sim", "sleeper", &program);
    assert!(matches!(harness.state().plugins.notice, Some(Ok(_))), "{:?}", harness.state().plugins.notice);
    assert_eq!(cc_plugins::tools::read_tool_paths(&folder, "sim")["sleeper"], program);
    harness.run();
    assert!(harness.query_by_label(&program.display().to_string()).is_some());
    harness.get_by_label(&t!("plug-program-test")).click();
    // A spinner while it runs.
    harness.run_steps(2);
    crate::plugins_ui::finish_tests(harness.state_mut());
    assert!(crate::plugins_ui::test_answer(harness.state(), "sim", "sleeper").is_some_and(|r| r.is_ok()));
    harness.run();
}

/// Reload loads the plugins again from their folder (a command added to
/// `main.lua` joins the session, the permissions stay); the Plugin console
/// lists what the plugins printed, filters by plugin and clears.
#[test]
fn plugins_reload_and_their_console() {
    let root = plugins_folder("console");
    let noisy = root.join("noisy");
    std::fs::create_dir_all(&noisy).unwrap();
    std::fs::write(noisy.join("plugin.toml"), "id = \"noisy\"\nname = \"Noisy\"\nversion = \"1.0\"\n").unwrap();
    std::fs::write(noisy.join("main.lua"), "print('noisy is here')").unwrap();
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(root.clone());
    state.prefs.plugins_files.insert("room_tools".into());
    crate::plugins_ui::load(&mut state);
    assert!(state.session.kind_of("room_tools__two").is_none());
    crate::plugins_ui::open(&mut state);
    let mut harness = app_harness(state);
    std::fs::write(
        root.join("room_tools/main.lua"),
        "ccad.command { name = 'two', run = function() print('two ran') end }",
    )
    .unwrap();
    harness.get_by_role_and_label(Role::Button, &t!("plug-reload")).click();
    harness.run();
    assert!(harness.state().session.kind_of("room_tools__two").is_some(), "the new command is there");
    assert!(harness.state().prefs.plugins_files.contains("room_tools"), "the permissions stay");
    // The console, from the Plugins window.
    harness.get_by_role_and_label(Role::Button, &t!("action-plugin-console")).click();
    harness.run();
    assert!(harness.state().plugins.console.open && !harness.state().plugins.open);
    assert!(
        harness.query_all_by_label_contains("noisy is here").count() == 2,
        "printed while loading, and again on Reload"
    );
    harness.state_mut().session.execute("room_tools__two", serde_json::json!({})).unwrap();
    harness.run();
    assert!(harness.query_by_label_contains("two ran").is_some(), "printed by a command");
    assert_eq!(harness.state().message, "[room_tools] two ran", "and in the status bar, as before");
    // Only one plugin's entries.
    let all = t!("pcon-all");
    harness.query_all_by_role(Role::ComboBox).find(|n| n.value().as_deref() == Some(all.as_str())).unwrap().click();
    harness.run();
    harness.get_by_label("room_tools").click();
    harness.run();
    assert_eq!(harness.state().plugins.console.filter.as_deref(), Some("room_tools"));
    assert!(harness.query_by_label_contains("noisy is here").is_none());
    assert!(harness.query_by_label_contains("two ran").is_some());
    harness.get_by_role_and_label(Role::Button, &t!("pcon-clear")).click();
    harness.run();
    assert!(harness.query_by_label_contains("two ran").is_none());
    assert_eq!(harness.state().plugins.console.entries().count(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// In developer mode the development folder's plugins load too, its copy
/// of a plugin wins over the installed one (noted in the console), and a
/// change to one of its files loads the plugins again.
#[test]
fn developer_mode_loads_the_development_folder_and_reloads_on_change() {
    let installed = plugins_folder("dev-installed");
    let dev_root = std::env::temp_dir().join(format!("ccad-plugins-ui-{}-dev", std::process::id()));
    let _ = std::fs::remove_dir_all(&dev_root);
    let dev = dev_root.join("room_tools");
    std::fs::create_dir_all(&dev).unwrap();
    std::fs::write(dev.join("plugin.toml"), "id = \"room_tools\"\nname = \"Room tools (dev)\"\nversion = \"0.4.0\"\n")
        .unwrap();
    std::fs::write(dev.join("main.lua"), "ccad.command { name = 'dev_only', run = function() end }").unwrap();
    let mut state = sample_state();
    state.prefs.plugin_folder = Some(installed.clone());
    state.prefs.plugin_dev_folder = Some(dev.clone());
    crate::plugins_ui::load(&mut state);
    assert!(state.session.kind_of("room_tools__one_wall").is_some(), "not in developer mode: the installed one");
    state.prefs.plugin_dev_mode = true;
    let mut harness = app_harness(state);
    harness.run();
    let s = harness.state();
    assert!(s.session.kind_of("room_tools__dev_only").is_some(), "the development copy");
    assert!(s.session.kind_of("room_tools__one_wall").is_none());
    assert!(s.plugins.listed.iter().any(|l| l.dev && l.id == "room_tools"));
    let wins = t!("pcon-dev-wins", "id" => "room_tools");
    assert!(s.plugins.console.entries().any(|e| e.text == wins), "noted in the console");
    // A change is seen at the next listing (a second later).
    std::fs::write(dev.join("main.lua"), "ccad.command { name = 'dev_two', run = function() end }").unwrap();
    std::thread::sleep(crate::plugin_dev::INTERVAL + std::time::Duration::from_millis(100));
    harness.run();
    assert!(harness.state().session.kind_of("room_tools__dev_two").is_some(), "loaded again");
    assert_eq!(harness.state().message, t!("plug-dev-reloaded"));
    let _ = std::fs::remove_dir_all(&installed);
    let _ = std::fs::remove_dir_all(&dev_root);
}

/// In an elevation a level's head is dragged up off its line with an elbow
/// (dropped near the line it goes back onto it), and the level's context
/// menu hides it in the view (issue #453).
#[test]
fn level_heads_are_dragged_and_hidden_in_elevations() {
    use glam::DVec2;
    let mut state = sample_state();
    let p = state.project();
    let view = p.views.values().find(|v| matches!(v.kind, cc_core::ViewKind::Elevation { .. })).unwrap().id;
    let level = p.levels[0].id;
    state.active_view = Some(view);
    state.set_tool(crate::tools::Tool::Select);
    let head = move |state: &mut AppState| {
        let d = state.drawing(view).unwrap();
        d.levels.iter().find(|l| l.level == level).and_then(|l| l.heads.first().copied()).unwrap()
    };
    /// The view drawn around the end of the first level's line, 20 points a meter.
    fn xf_of(state: &mut AppState, rect: egui::Rect) -> crate::paint::Xf {
        let view = state.active_view.unwrap();
        let d = state.drawing(view).unwrap();
        let level = state.project().levels[0].id;
        let origin = d.levels.iter().find(|l| l.level == level).map(|l| l.b).unwrap_or_default();
        crate::paint::Xf { screen: rect.center(), origin, scale: 20.0 }
    }
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = xf_of(state, resp.rect);
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
    });
    harness.run();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
    let xf = xf_of(harness.state_mut(), rect);
    let screen = |p: DVec2| xf.to_screen(p);
    let press = |h: &mut Harness<'static, AppState>, pos: egui::Pos2, button: egui::PointerButton, pressed: bool| {
        h.input_mut().events.push(egui::Event::PointerMoved(pos));
        h.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers: Default::default() });
        h.run();
    };
    let drag = |h: &mut Harness<'static, AppState>, from: egui::Pos2, by: egui::Vec2| {
        press(h, from, egui::PointerButton::Primary, true);
        for k in 1..=4 {
            h.input_mut().events.push(egui::Event::PointerMoved(from + by * (k as f32 / 4.0)));
            h.run();
        }
        press(h, from + by, egui::PointerButton::Primary, false);
    };
    let setting = |h: &Harness<'static, AppState>| {
        h.state().project().view(view).unwrap().level_heads.get(&level).copied().unwrap_or_default()
    };

    // Dragged 20 points (1 m) up: the end head's elbow.
    let h = head(harness.state_mut());
    drag(&mut harness, screen(h.center), egui::vec2(0.0, -20.0));
    assert!((setting(&harness).elbow - 1.0).abs() < 1e-6, "{:?}", setting(&harness));
    // Dropped a few points off the line, it goes back onto it.
    let h = head(harness.state_mut());
    drag(&mut harness, screen(h.center), egui::vec2(0.0, 18.0));
    assert_eq!(setting(&harness).elbow, 0.0);
    // Its context menu hides it in this view.
    let at = screen(head(harness.state_mut()).center);
    press(&mut harness, at, egui::PointerButton::Secondary, true);
    press(&mut harness, at, egui::PointerButton::Secondary, false);
    harness.get_by_label(&t!("menu-hide-level")).click();
    harness.run();
    assert!(setting(&harness).hidden);
    assert!(harness.state_mut().drawing(view).unwrap().levels.iter().all(|l| l.level != level));
}

/// A main gable roof 10 × 8 and a wing 4 wide against its south eave (as
/// in the engine's roof join tests), away from the sample house.
fn cross_wing(state: &mut AppState) -> (cc_core::ElementId, cc_core::ElementId) {
    let mut gable = |boundary: serde_json::Value, ridge_deg: f64| {
        let out = state
            .session
            .execute("roof_create", serde_json::json!({"boundary": boundary, "offset": 3.0, "overhang": 0.0}))
            .unwrap();
        let id = out["id"].as_u64().unwrap();
        let shape = serde_json::json!({"shape": {"Gable": {"slope": 0.5, "ridge_deg": ridge_deg}}});
        state.session.execute("element_set", serde_json::json!({"id": id, "params": shape})).unwrap();
        cc_core::ElementId(id)
    };
    let main = gable(serde_json::json!([[100, 0], [110, 0], [110, 8], [100, 8]]), 0.0);
    let wing = gable(serde_json::json!([[103, -6], [107, -6], [107, 0], [103, 0]]), 90.0);
    (main, wing)
}

fn joined_to(state: &AppState, roof: cc_core::ElementId) -> Option<cc_core::ElementId> {
    match &state.project().element(roof).unwrap().kind {
        cc_core::ElementKind::Roof(r) => r.join_to,
        _ => None,
    }
}

/// Join Roof (#381): the wing picked first, then the roof it meets, joins
/// them in one undo step; clicks off roofs are ignored, the tool stays for
/// the next wing, and Esc steps back one pick, then leaves the tool.
#[test]
fn join_roof_tool_joins_a_wing_to_the_main_roof() {
    use crate::commands::UiCommand;
    use crate::tools::Tool;
    let mut state = sample_state();
    let (main, wing) = cross_wing(&mut state);
    let wall = state.project().walls().next().map(|(e, _)| e.id).unwrap();
    state.selection.clear();
    let ctx = egui::Context::default();
    assert_eq!(UiCommand::JoinRoof.unavailable(&state), None);
    crate::commands::run(&mut state, &ctx, UiCommand::JoinRoof);
    assert_eq!(state.tool, Tool::JoinRoof);
    assert_eq!(state.message, t!("hint-roof-join-first"));
    // A wall is not picked.
    assert!(!crate::modify::is_roof(&state, wall));
    crate::modify::roof_join_pick(&mut state, Some(wall));
    crate::modify::roof_join_pick(&mut state, None);
    assert!(state.interaction.targets.is_empty());
    crate::modify::roof_join_pick(&mut state, Some(wing));
    assert_eq!(state.interaction.targets, vec![wing]);
    assert_eq!(state.message, t!("hint-roof-join-second"));
    crate::modify::roof_join_pick(&mut state, Some(wall));
    assert_eq!(state.interaction.targets, vec![wing], "the wing stays picked");
    let depth = state.session.doc.undo_depth();
    crate::modify::roof_join_pick(&mut state, Some(main));
    assert_eq!(joined_to(&state, wing), Some(main), "{}", state.message);
    assert_eq!(state.session.doc.undo_depth(), depth + 1);
    assert_eq!((&state.tool, state.interaction.targets.is_empty()), (&Tool::JoinRoof, true), "ready for the next wing");
    state.session.doc.undo();
    assert_eq!(joined_to(&state, wing), None, "one undo step undoes the join");

    // One roof selected is the wing; Esc steps back to the first pick, then leaves.
    state.selection = [wing].into_iter().collect();
    crate::commands::run(&mut state, &ctx, UiCommand::JoinRoof);
    assert_eq!(state.interaction.targets, vec![wing]);
    let mut harness = app_harness(state);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().tool, Tool::JoinRoof);
    assert!(harness.state().interaction.targets.is_empty());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().tool, Tool::Select);
}

/// A joined roof's properties name the roof it is joined to, and Unjoin
/// there (or Unjoin Roof on the selection) clears the join.
#[test]
fn joined_roofs_show_and_unjoin_in_the_properties() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    let (main, wing) = cross_wing(&mut state);
    state.selection = [wing].into_iter().collect();
    assert!(UiCommand::UnjoinRoof.unavailable(&state).is_some(), "not joined yet");
    state.session.run(cc_engine::commands::roof_join::RoofJoin { roof: wing.0, to: main.0 }).unwrap();
    assert_eq!(UiCommand::UnjoinRoof.unavailable(&state), None);
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    // Row labels are painted; the value names the roof joined to.
    let name = format!("{} {}", crate::i18n::tr(cc_core::Category::Roof.key()), main.0);
    assert!(harness.query_by_label(&name).is_some(), "Joined to: {name}");
    assert!(harness.query_by_label(&t!("section-roof-edges")).is_some());
    harness.get_by_role_and_label(Role::Button, &t!("action-roof-unjoin")).click();
    harness.run();
    assert_eq!(joined_to(harness.state(), wing), None);
    assert!(harness.query_by_label(&t!("action-roof-unjoin")).is_none(), "nothing left to unjoin");

    // Unjoin Roof on the selection, one undo step for all.
    let state = harness.state_mut();
    state.session.run(cc_engine::commands::roof_join::RoofJoin { roof: wing.0, to: main.0 }).unwrap();
    let depth = state.session.doc.undo_depth();
    crate::commands::run(state, &egui::Context::default(), UiCommand::UnjoinRoof);
    assert_eq!((joined_to(state, wing), state.session.doc.undo_depth()), (None, depth + 1));
}

/// Flat and shed roofs list an overhang per edge too (#382).
#[test]
fn flat_roofs_list_edge_overhangs() {
    let mut state = sample_state();
    let boundary = serde_json::json!([[100, 0], [104, 0], [104, 3], [100, 3]]);
    let out = state.session.execute("roof_create", serde_json::json!({"boundary": boundary, "offset": 3})).unwrap();
    let roof = cc_core::ElementId(out["id"].as_u64().unwrap());
    state.session.execute("element_set", serde_json::json!({"id": roof.0, "params": {"shape": "Flat"}})).unwrap();
    state.selection = [roof].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.query_by_label(&t!("section-roof-edges")).is_some());
    // Row labels are painted; each of the four edges has its overhang choice.
    let roof_overhang = t!("roof-edge-overhang-default");
    let rows = harness
        .query_all_by_role(Role::ComboBox)
        .filter(|n| n.value().as_deref() == Some(roof_overhang.as_str()))
        .count();
    assert_eq!(rows, 4, "an overhang row per edge");
}

/// Plugin API 3 catalog tab renders categories, color/dimension filters, item cards, and Open Website button.
#[test]
fn plugin_catalog_tab_shows_categories_colors_and_details() {
    let mut state = sample_state();
    state.plugins.catalogs.push(cc_plugins::Catalog {
        plugin_id: "ikea_browser".into(),
        plugin_name: "IKEA Browser".into(),
        id: "ikea".into(),
        label: "IKEA".into(),
        description: "Test catalog".into(),
    });
    state.prefs.plugins_network.insert("ikea_browser".into());
    state.plugin_catalog.set_loaded(
        "ikea_browser",
        "ikea",
        vec![crate::plugin_catalog::CatalogCategory { id: "st001".into(), name: "Storage".into(), count: 1 }],
        vec!["galvanized".into()],
        vec![crate::plugin_catalog::CatalogItem {
            id: "00069768".into(),
            name: "OMAR".into(),
            subtitle: "Shelf unit".into(),
            kind: "casework".into(),
            article: "000.697.68".into(),
            bytes: 211_088,
            width: Some(0.46),
            depth: Some(0.355),
            height: Some(0.94),
            colors: vec!["galvanized".into()],
            url: "https://www.ikea.com/us/en/p/omar-shelf-unit-galvanized-00069768/".into(),
            has_thumbnail: false,
        }],
    );
    crate::library::open(&mut state);
    if let Some(lib) = state.dialogs.library.as_mut() {
        lib.source = crate::library::Source::PluginCatalog(0);
        lib.catalog_selected = Some("00069768".into());
    }
    let mut harness = themed_harness(state, egui::vec2(1240.0, 760.0), egui::Theme::Light, |ui, state| {
        crate::library::show(ui.ctx(), state);
    });
    harness.run();
    assert!(harness.query_by_label("IKEA").is_some(), "plugin catalog tab shown");
    assert_eq!(harness.query_all_by_label("OMAR").count(), 2, "item card and details title shown");
    assert!(harness.query_by_label("000.697.68").is_some(), "article number shown");
    assert!(harness.query_by_role_and_label(Role::Button, &t!("lib-open-website")).is_some(), "Open Website shown");
}

/// With a wall selected the Modify tab is titled "Modify | Walls" and shows
/// the wall commands; a grid shows its own and none of the wall's; nothing
/// selected shows neither.
#[test]
fn modify_shows_the_selected_category_commands() {
    let mut state = sample_state();
    let wall = state.project().walls().next().map(|(e, _)| e.id).unwrap();
    let grid = state.project().elements_of(cc_core::Category::Grid).next().map(|e| e.id).unwrap();
    state.ribbon_tab = RibbonTab::Modify;
    state.selection.insert(wall);
    let harness = ribbon_harness(state, 2400.0);
    let walls = crate::actions::t_category(cc_core::Category::Wall);
    let title = format!("{} | {walls}", t!("ribbon-modify"));
    assert!(harness.query_by_label(&title).is_some(), "the tab names the category");
    for key in ["tool-flip-walls", "tool-attach-top", "edit-wall-profile"] {
        assert!(harness.query_by_role_and_label(Role::Button, &crate::i18n::tr(key)).is_some(), "{key} with a wall");
    }
    assert!(harness.query_by_role_and_label(Role::Button, &t!("action-align-grid-ends")).is_none());

    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Modify;
    state.selection.insert(grid);
    let harness = ribbon_harness(state, 2400.0);
    assert!(harness.query_by_role_and_label(Role::Button, &t!("action-align-grid-ends")).is_some());
    assert!(harness.query_by_role_and_label(Role::Button, &t!("tool-attach-top")).is_none(), "no wall commands");

    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Modify;
    state.selection.extend([wall, grid]);
    let harness = ribbon_harness(state, 2400.0);
    let title = format!("{} | {}", t!("ribbon-modify"), t!("ribbon-modify-several"));
    assert!(harness.query_by_label(&title).is_some(), "several categories");
    assert!(harness.query_by_role_and_label(Role::Button, &t!("tool-flip-walls")).is_some());
    assert!(harness.query_by_role_and_label(Role::Button, &t!("action-align-grid-ends")).is_some());

    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Modify;
    let harness = ribbon_harness(state, 2400.0);
    assert!(harness.query_by_role_and_label(Role::Button, &t!("ribbon-modify")).is_some());
    assert!(harness.query_by_role_and_label(Role::Button, &t!("tool-flip-walls")).is_none());
}

/// A right click on a wall in a plan selects it and opens its menu, which
/// holds the wall's commands; picking Flip flips it as one undo step.
#[test]
fn right_click_opens_the_selection_menu() {
    static ORIGIN: std::sync::OnceLock<glam::DVec2> = std::sync::OnceLock::new();
    let mut state = sample_state();
    // A point on a wall where a click takes the wall (no grid, opening or tag there).
    let walls: Vec<_> = state.project().walls().map(|(e, w)| (e.id, e.level, w.start, w.end)).collect();
    let (wall, plan, mid) = walls
        .into_iter()
        .flat_map(|(wall, level, a, b)| {
            let across = (b - a).perp().normalize_or_zero();
            (1..10).flat_map(move |k| [-0.1, 0.1].map(|off| (wall, level, a.lerp(b, k as f64 / 10.0) + across * off)))
        })
        .find_map(|(wall, level, at)| {
            let p = state.project();
            let plan = p.views.values().find(|v| v.is_plan() && v.level() == Some(level))?.id;
            let drawing = state.drawing(plan)?;
            let first = drawing.hits_at(at, 5.0 / 20.0).first().copied();
            let tagged = drawing.tags.iter().any(|t| t.center.distance(at) < 2.0);
            (first == Some(wall) && !tagged).then_some((wall, plan, at))
        })
        .unwrap();
    ORIGIN.set(mid).unwrap();
    state.active_view = Some(plan);
    state.set_tool(crate::tools::Tool::Select);
    let depth = state.session.doc.undo_depth();
    let mut harness = themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, |ui, state| {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let Some(origin) = ORIGIN.get().copied() else { return };
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin, scale: 20.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
    });
    harness.run();
    let pos = egui::pos2(300.0, 200.0);
    harness.input_mut().events.push(egui::Event::PointerMoved(pos));
    harness.run();
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Secondary, egui::Modifiers::NONE);
        harness.input_mut().events.push(egui::Event::PointerButton { pos, button, pressed, modifiers });
        harness.run();
    }
    harness.run();
    assert_eq!(
        harness.state().selection,
        std::collections::BTreeSet::from([wall]),
        "the wall under the pointer is selected"
    );
    assert_eq!(harness.state().interaction.element_menu, Some(plan));
    // A menu entry's name is its label, then its shortcut.
    let entry = |h: &Harness<'static, AppState>, key: &str| {
        let label = crate::i18n::tr(key);
        h.query_all_by_role(Role::Button)
            .map(|n| n.accesskit_node().label().unwrap_or_default())
            .find(|l| l.starts_with(&label))
    };
    for key in ["action-select-instances", "action-delete", "props-edit-type", "tool-attach-top"] {
        assert!(entry(&harness, key).is_some(), "{key} in the menu");
    }
    let before = harness.state().project().element(wall).and_then(|e| e.as_wall()).map(|w| (w.start, w.end));
    let flip = entry(&harness, "tool-flip-walls").unwrap();
    harness.get_by_role_and_label(Role::Button, &flip).click();
    harness.run();
    harness.run();
    let after = harness.state().project().element(wall).and_then(|e| e.as_wall()).map(|w| (w.start, w.end));
    assert_ne!(before, after, "the wall was flipped");
    assert_eq!(harness.state().session.doc.undo_depth(), depth + 1);
    assert!(entry(&harness, "action-delete").is_none(), "the menu closed");
}

/// A narrow window first turns rows of large buttons into medium ones, then
/// the last groups into menus, and scrolls only when nothing else fits; a
/// collapsed group's menu holds its buttons.
#[test]
fn ribbon_compacts_before_scrolling() {
    use crate::commands::UiCommand;
    let state = sample_state();
    let grid = UiCommand::Place(cc_core::Category::Grid).label(&state);
    // Wide: as designed, no group menus.
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    let harness = ribbon_harness(state, 1600.0);
    assert!(harness.query_by_role_and_label(Role::Button, &t!("group-datum")).is_none());
    // Narrow: the last group is a menu and the row fits without arrows.
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    let mut harness = ribbon_harness_sized(state, egui::vec2(700.0, 400.0));
    harness.run();
    assert!(harness.query_by_label(&t!("ribbon-scroll-right")).is_none(), "compacted, not scrolled");
    assert!(harness.query_by_role_and_label(Role::Button, &grid).is_none(), "Grid is inside the menu");
    harness.get_by_role_and_label(Role::Button, &t!("group-datum")).click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, &grid).click();
    harness.run();
    assert!(matches!(harness.state().tool, crate::tools::Tool::Grid { .. }), "the menu's button runs");
    // Wide again: back as designed.
    harness.set_size(egui::vec2(1600.0, 400.0));
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, &t!("group-datum")).is_none(), "groups back");
    assert!(harness.query_by_role_and_label(Role::Button, &grid).is_some());
}

/// A ribbon button's right-click menu adds it to the quick access toolbar;
/// the toolbar's menu moves and removes it, and restores the default set.
#[test]
fn quick_access_toolbar_is_customized() {
    use crate::commands::UiCommand;
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    let mut harness = ribbon_harness_sized(state, egui::vec2(1600.0, 500.0));
    let levels = UiCommand::Levels.label(harness.state());
    assert_eq!(harness.query_all_by_role_and_label(Role::Button, &levels).count(), 1, "only in the tab");
    harness.get_by_role_and_label(Role::Button, &levels).click_secondary();
    harness.run();
    harness.get_by_role_and_label(Role::Button, &t!("qat-add")).click();
    harness.run();
    let ids = harness.state().prefs.quick_access.clone().unwrap();
    assert_eq!(ids.last().map(String::as_str), Some("Levels"));
    assert_eq!(harness.query_all_by_role_and_label(Role::Button, &levels).count(), 2, "and in the toolbar");
    assert_eq!(crate::ribbon::quick_access(harness.state()).last(), Some(&UiCommand::Levels));
    // Its menu moves it left, then removes it.
    harness.get_by_role_and_label(Role::Button, &t!("qat-customize")).click();
    harness.run();
    harness.get_all_by_role_and_label(Role::Button, "←").last().unwrap().click();
    harness.run();
    let list = crate::ribbon::quick_access(harness.state());
    assert_eq!(list[list.len() - 2], UiCommand::Levels, "moved one place left");
    harness.get_all_by_role_and_label(Role::Button, "×").nth(list.len() - 2).unwrap().click();
    harness.run();
    assert!(!crate::ribbon::quick_access(harness.state()).contains(&UiCommand::Levels));
    harness.get_by_role_and_label(Role::Button, &t!("qat-reset")).click();
    harness.run();
    assert_eq!(harness.state().prefs.quick_access, None, "the default set again");
}

/// File opens a page over the window with the recent projects and, on its
/// Export page, the export formats; Esc goes back to the tab shown before.
#[test]
fn file_is_a_page_over_the_window() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::View;
    state.recent = vec![std::path::PathBuf::from("Z:/nowhere/old house.ccad")];
    let mut harness = ribbon_harness_sized(state, egui::vec2(1400.0, 700.0));
    harness.get_by_label(&t!("ribbon-file")).click();
    harness.run();
    assert_eq!(harness.state().ribbon_tab, RibbonTab::File);
    assert!(harness.query_by_role_and_label(Role::Button, "old house").is_some(), "the recent projects");
    harness.get_by_role_and_label(Role::Button, &t!("group-export")).click();
    harness.run();
    let ifc = crate::commands::UiCommand::ExportIfc.label(harness.state());
    assert!(harness.query_by_role_and_label(Role::Button, &ifc).is_some(), "the export formats");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().ribbon_tab, RibbonTab::View, "back to the tab shown before");
}

/// The status bar's count opens the selection's filter: unchecking a
/// category deselects it, checking it again brings it back, and *Only*
/// keeps that category alone.
#[test]
fn the_selection_is_filtered_by_category() {
    let mut state = sample_state();
    let walls: Vec<_> = state.project().walls().map(|(e, _)| e.id).take(2).collect();
    let grid = state.project().elements_of(cc_core::Category::Grid).next().map(|e| e.id).unwrap();
    state.selection = walls.iter().copied().chain([grid]).collect();
    let mut harness = themed_harness(state, egui::vec2(900.0, 400.0), egui::Theme::Light, |ui, state| {
        ui.add_space(300.0);
        crate::ribbon::status_bar(ui, state);
    });
    harness.get_by_label(&t!("status-selected", "count" => 3)).click();
    harness.run();
    let wall_row = format!("{} (2)", crate::actions::t_category(cc_core::Category::Wall));
    harness.get_by_label(&wall_row).click();
    harness.run();
    assert_eq!(harness.state().selection, std::collections::BTreeSet::from([grid]), "walls unchecked");
    harness.get_by_label(&wall_row).click();
    harness.run();
    assert_eq!(harness.state().selection.len(), 3, "walls checked again");
    harness.get_all_by_label(&t!("selection-filter-only")).last().unwrap().click();
    harness.run();
    assert_eq!(harness.state().selection, std::collections::BTreeSet::from([grid]), "only the grids");
}

/// The properties panel's search keeps the rows whose name holds the text
/// and leaves the others out: hidden, they no longer take clicks.
#[test]
fn properties_are_searched_by_name() {
    let mut state = sample_state();
    let level = state.project().levels[0].id.0;
    let args = serde_json::json!({"level": level, "start": [100, 100], "end": [106, 100], "height": 3.0});
    let wall = cc_core::ElementId(state.session.execute("wall_create", args).unwrap()["id"].as_u64().unwrap());
    let params = serde_json::json!({"id": wall.0, "params": {"structural": true}});
    state.session.execute("element_set", params).unwrap();
    state.selection = [wall].into_iter().collect();
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    // The search field is the first text field, above the rows.
    harness.get_all_by_role(Role::TextInput).next().unwrap().focus();
    harness.run();
    harness.get_all_by_role(Role::TextInput).next().unwrap().type_text("zzzz");
    harness.run();
    let typed: Option<String> = harness.ctx.data(|d| d.get_temp(egui::Id::new("props-search")));
    assert_eq!(typed.as_deref(), Some("zzzz"));
    // The typical reinforcement button's row matches nothing: a click on it does nothing.
    harness.get_by_label(&t!("action-rebar-typical")).click();
    harness.run();
    assert_eq!(harness.state().project().rebar_of(wall).count(), 0, "the hidden row took no click");
}

/// The Material Libraries dialog loads the materials checked of a built-in
/// library (or all of them); OK keeps them, Cancel takes them out again.
#[test]
fn material_libraries_load_and_cancel() {
    let mut state = sample_state();
    let before = state.project().materials.len();
    crate::material_library_dialog::open(&mut state);
    let mut harness = themed_harness(state, egui::vec2(1200.0, 900.0), egui::Theme::Light, |ui, state| {
        crate::dialogs::show(ui.ctx(), state);
    });
    harness.run();
    let concretes = format!("{} (12)", t!("material-library-br-concreto"));
    harness.get_by_label(&concretes).click();
    harness.run();
    assert_eq!(
        harness.state().dialogs.material_library.as_ref().and_then(|d| d.selected.clone()).as_deref(),
        Some("BR-CONCRETO")
    );
    harness.get_by_label(&t!("mat-lib-load-all")).click();
    harness.run();
    assert_eq!(harness.state().project().materials.len(), before + 12);
    assert!(harness.state().project().material("Concreto C30 (NBR 6118)").is_some());
    harness.get_by_label(&t!("action-ok")).click();
    harness.run();
    assert!(harness.state().dialogs.material_library.is_none());
    assert_eq!(harness.state().project().materials.len(), before + 12, "OK keeps them");

    // Two steels checked, then Cancel: nothing stays.
    crate::material_library_dialog::open(harness.state_mut());
    harness.run();
    let steels = format!("{} (12)", t!("material-library-br-aco-estrutural"));
    harness.get_by_label(&steels).click();
    harness.run();
    if let Some(d) = harness.state_mut().dialogs.material_library.as_mut() {
        d.picked = ["Aço MR250 (NBR 7007)".to_string(), "Aço ASTM A36".to_string()].into();
    }
    harness.run();
    harness.get_by_label(&t!("mat-lib-load-picked", "count" => 2)).click();
    harness.run();
    assert_eq!(harness.state().project().materials.len(), before + 14);
    harness.get_by_label(&t!("action-cancel")).click();
    harness.run();
    assert_eq!(harness.state().project().materials.len(), before + 12, "Cancel undoes the load");
}

/// Issue #497: the wall tool's Rectangle mode makes four joined walls from
/// two corners, in one undo step, and stays for the next shape; Esc drops
/// a first corner.
#[test]
fn the_wall_tool_draws_rectangles() {
    let mut state = spline_state();
    state.wall_mode = crate::tools::WallMode::Rect;
    let mut harness = spline_canvas(state, egui::Theme::Light);
    let (walls, depth) = (walls_of(harness.state()).len(), harness.state().session.doc.undo_depth());
    spline_click(&mut harness, [-1.0, -0.5]);
    spline_click(&mut harness, [1.0, 1.0]);
    let state = harness.state();
    assert!(state.message.is_empty(), "{}", state.message);
    assert_eq!(walls_of(state).len(), walls + 4);
    assert_eq!(state.session.doc.undo_depth(), depth + 1, "one undo step");
    assert_eq!(state.selection.len(), 4, "the new walls are selected");
    assert!(matches!(state.tool, crate::tools::Tool::Wall { .. }));
    spline_click(&mut harness, [2.0, 0.0]);
    assert!(crate::wall_draw_ui::typing(harness.state()), "a first corner");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!crate::wall_draw_ui::typing(harness.state()), "Esc drops it");
    assert!(matches!(harness.state().tool, crate::tools::Tool::Wall { .. }), "the tool stays");
    assert_eq!(walls_of(harness.state()).len(), walls + 4);
}

/// With a radius in the options bar, one click places a circle: two
/// half-circle walls.
#[test]
fn a_circle_with_a_radius_takes_one_click() {
    let mut state = spline_state();
    state.wall_mode = crate::tools::WallMode::Circle;
    state.wall_draw.radius = 1.0;
    let mut harness = spline_canvas(state, egui::Theme::Light);
    let walls = walls_of(harness.state()).len();
    spline_click(&mut harness, [0.0, 0.0]);
    let new = walls_of(harness.state());
    assert_eq!(new.len(), walls + 2, "{}", harness.state().message);
    let arcs: Vec<_> = new.iter().filter_map(|e| e.as_wall().and_then(|w| w.arc())).collect();
    assert!(arcs.iter().filter(|a| (a.radius - 1.0).abs() < 1e-9).count() >= 2);
}

/// A sketch (a new floor) opens the main ribbon's Modify tab as *Modify |
/// Create Boundary*, with the drawing tools, *From Walls* and Finish and
/// Cancel; the other tabs wait, and Cancel brings back the tab shown before.
#[test]
fn a_sketch_opens_its_contextual_modify_tab() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Architecture;
    let floor = state.project().default_type(cc_core::Category::Floor).unwrap();
    state.set_tool(crate::tools::Tool::Sketch { category: cc_core::Category::Floor, type_id: floor });
    assert!(state.sketch.is_some(), "the floor tool opens a sketch");
    let mut harness = ribbon_harness(state, 1600.0);
    harness.run();
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Modify);
    let title = format!("{} | {}", t!("ribbon-modify"), crate::sketch_mode::title(harness.state()));
    assert!(harness.query_by_label(&title).is_some(), "the tab is named after the sketch");
    use crate::commands::UiCommand;
    for command in [UiCommand::SketchRect, UiCommand::SketchPickWalls, UiCommand::SketchFinish] {
        let label = command.label(harness.state());
        assert!(harness.query_by_label(&label).is_some(), "{label} is in the tab");
    }
    let architecture = t!("ribbon-architecture");
    assert!(harness.get_by_label(&architecture).accesskit_node().is_disabled(), "the other tabs wait for the sketch");
    let cancel = UiCommand::SketchCancel.label(harness.state());
    harness.get_by_label(&cancel).click();
    harness.run();
    harness.run();
    assert!(harness.state().sketch.is_none());
    assert_eq!(harness.state().ribbon_tab, RibbonTab::Architecture, "the tab before comes back");
}

/// A room of the sample project on its first level, and the plan of that level.
fn interior_state() -> (AppState, cc_core::ElementId, glam::DVec2) {
    let mut state = sample_state();
    let level = state.project().levels[0].id;
    let plan = state.project().views.values().find(|v| v.is_plan() && v.level() == Some(level)).unwrap().id;
    state.active_view = Some(plan);
    let rooms: Vec<(cc_core::ElementId, glam::DVec2)> = state
        .project()
        .elements
        .values()
        .filter(|e| e.level == level)
        .filter_map(|e| match &e.kind {
            cc_core::ElementKind::Room(r) => Some((e.id, r.position)),
            _ => None,
        })
        .collect();
    let (room, at) = rooms.into_iter().find(|(id, _)| state.session.geom().rooms.contains_key(id)).unwrap();
    (state, room, at)
}

/// The active plan's canvas centered on `origin`, 100 points per meter, with the app's keys.
fn interior_canvas(state: AppState, origin: glam::DVec2) -> Harness<'static, AppState> {
    themed_harness(state, egui::vec2(600.0, 400.0), egui::Theme::Light, move |ui, state| {
        crate::app::shortcuts(ui.ctx(), state);
        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let xf = crate::paint::Xf { screen: resp.rect.center(), origin, scale: 100.0 };
        let Some(view) = state.active_view.and_then(|v| state.project().view(v).cloned()) else { return };
        let Some(drawing) = state.drawing(view.id) else { return };
        crate::tools::handle(ui, &resp, &painter, state, &view, &xf, &drawing);
    })
}

fn interior_click(h: &mut Harness<'static, AppState>, at: egui::Pos2, times: usize) {
    h.input_mut().events.push(egui::Event::PointerMoved(at));
    h.run();
    for _ in 0..times {
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
            h.input_mut().events.push(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
            h.run();
        }
    }
}

/// Issue #477: the Interior Elevation tool (View tab) places a marker with
/// its four views in the space clicked, and a double click on an arrow of
/// the marker opens that arrow's view.
#[test]
fn the_interior_elevation_tool_places_a_marker_and_its_arrows_open_views() {
    let (mut state, _, at) = interior_state();
    state.ribbon_tab = RibbonTab::View;
    let mut ribbon = ribbon_harness(state, 2400.0);
    ribbon.get_by_role_and_label(Role::Button, &t!("tool-interior-elevation")).click();
    ribbon.run();
    assert_eq!(ribbon.state().tool, crate::tools::Tool::InteriorElevation);
    let state = std::mem::replace(ribbon.state_mut(), sample_state());

    let mut canvas = interior_canvas(state, at);
    interior_click(&mut canvas, egui::pos2(300.0, 200.0), 1);
    let state = canvas.state();
    let markers: Vec<cc_core::ElementId> = state
        .project()
        .elements
        .values()
        .filter(|e| matches!(e.kind, cc_core::ElementKind::InteriorMarker(_)))
        .map(|e| e.id)
        .collect();
    assert_eq!(markers.len(), 1, "{}", state.message);
    assert_eq!(state.project().marker_sides(markers[0]).len(), 4);
    assert_eq!(state.selection, [markers[0]].into_iter().collect(), "the marker is selected");

    // With the Select tool, a double click on the east arrow opens the east view.
    let plan = state.active_view.unwrap();
    let east = state
        .project()
        .marker_sides(markers[0])
        .into_iter()
        .find(|(side, ..)| *side == cc_core::interior::Side::East)
        .unwrap()
        .2;
    canvas.state_mut().set_tool(crate::tools::Tool::Select);
    let arrow =
        canvas.state_mut().drawing(plan).unwrap().interior_arrows.iter().find(|a| a.view == east).copied().unwrap();
    let xf = crate::paint::Xf { screen: egui::pos2(300.0, 200.0), origin: at, scale: 100.0 };
    // The harness clock runs faster than a hand, so allow a slow double click.
    canvas.ctx.options_mut(|o| o.input_options.max_double_click_delay = 60.0);
    interior_click(&mut canvas, xf.to_screen(arrow.center), 2);
    assert_eq!(canvas.state().open_requests.last(), Some(&east), "the east view opens");
}

/// The marker's properties take a side away (its view goes) and back (a
/// view with the freed number); the last side cannot be taken away; a
/// selected room's command makes its four views.
#[test]
fn interior_markers_set_their_sides_in_properties() {
    let (mut state, room, _) = interior_state();
    state.selection = [room].into_iter().collect();
    assert!(crate::commands::UiCommand::RoomInteriorElevations.unavailable(&state).is_none());
    crate::interior_ui::for_selected_rooms(&mut state);
    let marker = *state.selection.iter().next().unwrap();
    assert_eq!(state.project().marker_sides(marker).len(), 4, "{}", state.message);

    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    let north = t!("tpl-elev-north");
    harness.get_by_role_and_label(Role::CheckBox, &north).click();
    harness.run();
    let sides = harness.state().project().marker_sides(marker);
    assert_eq!(sides.len(), 3, "{}", harness.state().message);
    assert!(sides.iter().all(|(s, ..)| *s != cc_core::interior::Side::North));
    harness.get_by_role_and_label(Role::CheckBox, &north).click();
    harness.run();
    let sides = harness.state().project().marker_sides(marker);
    let (_, number, _) = sides.iter().find(|(s, ..)| *s == cc_core::interior::Side::North).unwrap();
    assert_eq!(*number, 1, "the freed number comes back");
}

/// Turning *Automatic extents* off keeps the crop the view had, which then
/// stays as drawn.
#[test]
fn interior_elevations_can_keep_their_crop() {
    let (mut state, room, _) = interior_state();
    state.selection = [room].into_iter().collect();
    crate::interior_ui::for_selected_rooms(&mut state);
    let marker = *state.selection.iter().next().unwrap();
    let view = state.project().marker_sides(marker)[0].2;
    state.selection.clear();
    state.active_view = Some(view);
    let mut harness = themed_harness(state, egui::vec2(360.0, 1600.0), egui::Theme::Light, |ui, state| {
        crate::properties::show(ui, state);
    });
    harness.run();
    assert!(harness.state().project().view(view).unwrap().crop.is_none());
    harness.get_by_role_and_label(Role::CheckBox, &t!("prop-auto-extents")).click();
    harness.run();
    let v = harness.state().project().view(view).unwrap().clone();
    assert!(matches!(v.kind, cc_core::ViewKind::InteriorElevation { auto_extents: false, .. }), "{:?}", v.kind);
    assert!(v.crop.is_some_and(|c| c.is_valid()), "the crop it had is kept: {:?}", v.crop);
}
