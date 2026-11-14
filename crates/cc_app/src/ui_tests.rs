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
    draw: fn(&mut egui::Ui, &mut AppState),
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
    let mut harness = ribbon_harness_sized(state, egui::vec2(900.0, 300.0));
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
    state.ribbon_tab = RibbonTab::Structure;
    let harness = ribbon_harness(state, 1100.0);
    assert!(harness.query_by_label(&t!("ribbon-scroll-right")).is_none());
}

/// Double-clicking a tab collapses the ribbon to its tabs; a click shows a
/// tab's panel over the views, and running a command closes it.
#[test]
fn ribbon_collapses_and_peeks() {
    let mut harness = ribbon_harness_sized(sample_state(), egui::vec2(1100.0, 220.0));
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
        let harness = ribbon_harness(state, 1600.0);
        for node in harness.query_all_by_role(Role::Button) {
            let label = node.accesskit_node().label().unwrap_or_default();
            assert!(!label.trim().is_empty(), "{tab:?}: a button without a name");
            found.insert(label);
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
    let hidden = [UiCommand::CommandPalette, UiCommand::ToggleRibbon];
    for command in UiCommand::all().into_iter().filter(|c| !hidden.contains(c)) {
        assert!(found.contains(&command.label(&state)), "{command:?} is not in the ribbon");
    }
}

/// The wall tool switches between straight and arc walls in the options bar.
#[test]
fn wall_tool_has_an_arc_mode() {
    let mut state = sample_state();
    let wall_type = state.project().default_type(cc_core::Category::Wall).unwrap();
    state.tool = crate::tools::Tool::Wall { type_id: wall_type };
    let mut harness = themed_harness(state, egui::vec2(1600.0, 60.0), egui::Theme::Light, |ui, state| {
        crate::ribbon::options_bar(ui, state)
    });
    assert!(!harness.state().wall_arc);
    harness.get_by_label(&t!("wall-mode-arc")).click();
    harness.run();
    assert!(harness.state().wall_arc);
    harness.get_by_label(&t!("wall-mode-line")).click();
    harness.run();
    assert!(!harness.state().wall_arc);
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
    assert!(harness.state().beam_arc && !harness.state().wall_arc && !harness.state().strip_arc);
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
    assert!(harness.state().strip_arc && !harness.state().wall_arc);
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
    harness.get_by_role_and_label(Role::CheckBox, &tooltips).click();
    harness.get_by_role_and_label(Role::CheckBox, &ribbon).click();
    harness.run();
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
        let mut harness = themed_harness(state, egui::vec2(940.0, 600.0), egui::Theme::Light, |ui, state| {
            crate::preferences::show(ui.ctx(), state)
        });
        harness.run();
        results.add(harness.try_snapshot(format!("preferences_{section:?}").to_lowercase()));
    }
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
    let mut harness = ribbon_harness_sized(state, egui::vec2(1100.0, 400.0));
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
    let mut harness = ribbon_harness_sized(state, egui::vec2(1100.0, 400.0));
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
    state.sketch.as_mut().unwrap().add(vec![circle]);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    assert!(state.sketch.is_none(), "{}", state.message);
    let mass = *state.selection.iter().next().unwrap();
    let drawn = state.project().element(mass).and_then(|e| e.drawn_sketch()).cloned().unwrap();
    assert_eq!(drawn.curves, vec![circle]);

    // Edit Boundary opens the curves; a bigger circle draws the outline again.
    assert_eq!(UiCommand::EditBoundary.unavailable(&state), None);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::EditBoundary);
    let s = state.sketch.as_mut().unwrap();
    assert_eq!((s.target, s.draw, s.curves.clone()), (Target::Edit { id: mass }, Draw::Select, vec![circle]));
    let bigger = SketchCurve::Circle { center: glam::DVec2::new(30.0, 30.0), radius: 4.0 };
    s.set_curve(0, bigger);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::SketchFinish);
    let e = state.project().element(mass).unwrap();
    assert_eq!(e.drawn_sketch().unwrap().curves, vec![bigger]);

    // Inside a sketch, undo undoes the sketch's own changes; an open sketch does not finish.
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::EditBoundary);
    let s = state.sketch.as_mut().unwrap();
    s.selected.insert(0);
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::Delete);
    assert!(state.sketch.as_ref().unwrap().curves.is_empty());
    crate::commands::run(&mut state, &egui::Context::default(), UiCommand::Undo);
    assert_eq!(state.sketch.as_ref().unwrap().curves, vec![bigger]);
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
    let Some(SketchCurve::Line { a, b }) = s.curves.last().copied() else { panic!("{:?}", s.curves) };
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

/// Visual Programming opens the graph editor from the Manage tab.
#[test]
fn visual_programming_opens_from_the_manage_tab() {
    let mut state = sample_state();
    state.ribbon_tab = RibbonTab::Manage;
    let mut harness = app_harness(state);
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
        ("fill patterns".into(), Box::new(crate::fill_patterns_dialog::open)),
        ("phases".into(), Box::new(crate::phases_dialog::open)),
        ("filters".into(), Box::new(crate::graphics_dialog::open_filters)),
        ("view templates".into(), Box::new(crate::graphics_dialog::open_templates)),
        ("new project".into(), Box::new(crate::new_project_dialog::open)),
        ("new component".into(), Box::new(crate::new_component_dialog::open)),
    ];
    for section in [Section::General, Section::Interface, Section::FileLocations, Section::Saving, Section::Shortcuts] {
        opens.push((format!("preferences {section:?}"), Box::new(move |s| crate::preferences::open(s, section))));
    }
    // Sun Settings in each kind of sun, and in a 3D view (shadows enabled).
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
            let mut harness = themed_harness(state, egui::vec2(1500.0, 1000.0), egui::Theme::Light, draw);
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
    use crate::detail_ui::DetailMode;
    use cc_core::sketch::SketchCurve;
    let mut state = sample_state();
    let made = crate::actions::run_command(
        &mut state,
        cc_engine::commands::views::ViewCreate { kind: "drafting".into(), ..Default::default() },
    );
    let view = cc_core::ViewId(made.unwrap()["id"].as_u64().unwrap());
    state.active_view = Some(view);
    state.set_tool(crate::tools::Tool::DetailLine);
    state.detail.mode = DetailMode::Circle;
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
                cc_core::ElementKind::DetailLine(l) if l.view == view => Some(l.curve),
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
    harness.state_mut().detail.mode = DetailMode::Line;
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
    state.ribbon_tab = RibbonTab::Structure;
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
    harness.get_by_label("IPE 200").click();
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
    assert_eq!(t.members.len(), 33);
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
    assert_eq!(get(&state).members.len(), 12 + 7 + 6);
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
    assert_eq!((t.members.len(), t.detached.len()), (25, 0));
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
    assert!(harness.query_by_label(&t!("sun-shadows")).is_none(), "shadows only in 3D views");
    harness.get_by_label(&t!("prop-sun-path")).click();
    harness.run();
    assert!(!harness.state().project().view(plan).unwrap().sun.as_ref().unwrap().show_path);
    harness.get_by_label(&still).click();
    harness.run();
    assert!(harness.state().dialogs.sun.is_some());
}
