//! Ribbon, options bar and status bar.

use crate::actions;
use crate::commands::{self, UiCommand};
use crate::icons::{self, Badge, Icon, Tones};
use crate::plugins_ui::RibbonButton;
use crate::state::{AppState, RibbonTab};
use crate::t;
use crate::theme::{self, PALETTE};
use crate::tools::{DimensionMode, Run, Tool};
use cc_core::{Category, ViewCategory};
use cc_plugins::ButtonSize;
use cc_views::format;
use egui::{Align, Button, DragValue, Layout, Response, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

/// Height of the large ribbon buttons.
const BUTTON_HEIGHT: f32 = 66.0;
const ICON_SIZE: f32 = 22.0;
/// Icon size of the small stacked ribbon buttons.
const STACK_ICON_SIZE: f32 = 20.0;

/// Icon of a model category, shared by the ribbon, options bar and browser.
pub fn category_icon(category: Category) -> Icon {
    match category {
        Category::Wall => Icon::Wall,
        Category::Door => Icon::Door,
        Category::Window => Icon::Window,
        Category::Niche => Icon::Niche,
        Category::Floor => Icon::Floor,
        Category::Ceiling => Icon::Ceiling,
        Category::Roof => Icon::Roof,
        Category::Room => Icon::Room,
        Category::Grid => Icon::Grid,
        Category::Level => Icon::Levels,
        Category::Foundation | Category::Pile => Icon::Footing,
        Category::Rebar => Icon::Rebar,
        Category::Pipe => Icon::Pipe,
        Category::Conduit => Icon::Conduit,
        Category::CableTray => Icon::CableTray,
        Category::Valve => Icon::Valve,
        Category::LightningConductor => Icon::LightningConductor,
        Category::Dimension => Icon::Dimension,
        Category::Column => Icon::Column,
        Category::Beam => Icon::Beam,
        Category::Stair => Icon::Stair,
        Category::Ramp => Icon::Ramp,
        Category::Railing => Icon::Railing,
        Category::Component => Icon::Component,
        Category::Text | Category::TitleBlock | Category::BarTag | Category::Tag | Category::ViewMark => Icon::Text,
        Category::LevelMark => Icon::LevelMarkPlan,
        Category::CadImport => Icon::CadImport,
        Category::DetailLine => Icon::DetailLine,
        Category::DetailComponent => Icon::DetailComponent,
        Category::FilledRegion => Icon::FilledRegion,
        Category::MaskingRegion => Icon::MaskingRegion,
        Category::Mass => Icon::Mass,
        Category::Shaft => Icon::Shaft,
        Category::SteelBuilding => Icon::SteelBuilding,
        Category::Truss => Icon::Truss,
        Category::Bracing => Icon::Brace,
        Category::Connection => Icon::Connection,
        Category::WallFraming => Icon::WallFraming,
        Category::Topography => Icon::Topography,
        Category::Pad => Icon::Pad,
        Category::Sweep => Icon::Sweep,
    }
}

fn tool_icon(tool: &Tool) -> Icon {
    match tool {
        Tool::Select => Icon::Select,
        Tool::Section => Icon::Section,
        Tool::Callout => Icon::Callout,
        Tool::Move => Icon::Move,
        Tool::Copy => Icon::Copy,
        Tool::Rotate => Icon::Rotate,
        Tool::Mirror { draw: false } => Icon::MirrorPick,
        Tool::Mirror { draw: true } => Icon::MirrorDraw,
        Tool::Array => Icon::Array,
        Tool::TrimCorner => Icon::TrimCorner,
        Tool::TrimExtend => Icon::TrimExtend,
        Tool::Split => Icon::Split,
        Tool::Offset => Icon::Offset,
        Tool::Align => Icon::Align,
        Tool::PushPull => Icon::PushPull,
        Tool::ToggleJoin => Icon::Join,
        Tool::Measure => Icon::Measure,
        Tool::JoinGeometry { unjoin: false } => Icon::JoinGeometry,
        Tool::JoinGeometry { unjoin: true } => Icon::UnjoinGeometry,
        Tool::AttachTop => Icon::AttachTop,
        Tool::JoinRoof => Icon::RoofJoin,
        Tool::Paste => Icon::Paste,
        Tool::Dimension { mode, .. } => dimension_icon(*mode),
        Tool::MatchType => Icon::MatchType,
        Tool::Demolish => Icon::Demolish,
        Tool::CurtainGrid => Icon::CurtainGrid,
        Tool::Paint { .. } => Icon::Paint,
        Tool::AddLeader { .. } => Icon::Leader,
        Tool::Opening { floor: false, .. } => category_icon(Category::Roof),
        Tool::Opening { floor: true, .. } => category_icon(Category::Floor),
        _ => tool.category().map_or(Icon::Select, category_icon),
    }
}

/// Labels wider than this (points) wrap onto two lines at a space.
const WRAP_WIDTH: f32 = 64.0;

/// The least width of a large ribbon button, so short labels keep the
/// buttons of a group at an even rhythm.
const BUTTON_MIN_WIDTH: f32 = 60.0;
/// Room at each side of a large button's label.
const BUTTON_PAD: f32 = 8.0;

/// Width of a large button whose label lines are at most `text_w` wide,
/// with `chevron` more for a menu's arrow.
fn button_width(text_w: f32, chevron: f32) -> f32 {
    (text_w + chevron + 2.0 * BUTTON_PAD).max(BUTTON_MIN_WIDTH)
}

/// Where `label` may break into two lines, as (end of the first line, start
/// of the second): at a space, which is dropped, or after a slash or a
/// hyphen between two words, which stays on the first line.
fn break_points(label: &str) -> Vec<(usize, usize)> {
    label
        .char_indices()
        .filter_map(|(i, c)| match c {
            ' ' => Some((i, i, i + 1)),
            '/' | '-' => Some((i, i + 1, i + 1)),
            _ => None,
        })
        // Words on both sides, so neither line is empty or a lone separator.
        .filter(|(at, _, start)| {
            let word = |t: Option<&str>| t.is_some_and(|t| !t.trim().is_empty());
            word(label.get(..*at)) && word(label.get(*start..))
        })
        .map(|(_, end, start)| (end, start))
        .collect()
}

/// Split `label` into one or two lines: long labels break where the two
/// lines are best balanced (see [`break_points`]).
fn label_lines(ui: &Ui, label: &str, font: &egui::FontId) -> Vec<String> {
    let width = |t: &str| ui.painter().layout_no_wrap(t.to_string(), font.clone(), PALETTE.text).size().x;
    if width(label) <= WRAP_WIDTH {
        return vec![label.to_string()];
    }
    let best = break_points(label)
        .into_iter()
        .filter_map(|(end, start)| Some((label.get(..end)?.trim_end(), label.get(start..)?.trim_start())))
        .map(|(first, second)| (first, second, width(first).max(width(second))))
        .min_by(|a, b| a.2.total_cmp(&b.2));
    match best {
        Some((first, second, _)) => vec![first.to_string(), second.to_string()],
        None => vec![label.to_string()],
    }
}

/// What a ribbon button shows as its icon: a built-in one with its badge, or
/// a plugin's SVG file (the plugin icon when it cannot be drawn).
#[derive(Debug, Clone, Copy)]
enum Glyph<'a> {
    Icon(Icon, Option<Badge>),
    File(Option<&'a std::path::Path>),
}

impl Glyph<'_> {
    fn badge(self) -> Option<Badge> {
        match self {
            Glyph::Icon(_, badge) => badge,
            Glyph::File(_) => None,
        }
    }

    fn paint(self, painter: &egui::Painter, rect: egui::Rect, tones: Tones) {
        match self {
            Glyph::Icon(icon, badge) => icons::paint_toned(painter, rect, icon, tones, badge),
            Glyph::File(path) => {
                if !path.is_some_and(|p| icons::paint_file_toned(painter, rect, p, tones)) {
                    icons::paint_toned(painter, rect, Icon::Plugin, tones, None);
                }
            }
        }
    }
}

/// A large ribbon button: icon above a label of up to two lines. `dropdown` adds a chevron.
fn big_button(ui: &mut Ui, glyph: Glyph<'_>, label: &str, selected: bool, enabled: bool, dropdown: bool) -> Response {
    big_button_ink(ui, glyph, label, selected, enabled, dropdown, None)
}

/// [`big_button`] whose icon strokes are `ink` when given (Finish in green,
/// Cancel in red).
fn big_button_ink(
    ui: &mut Ui,
    glyph: Glyph<'_>,
    label: &str,
    selected: bool,
    enabled: bool,
    dropdown: bool,
    ink: Option<egui::Color32>,
) -> Response {
    let text_color = if enabled { PALETTE.text } else { PALETTE.text_faint };
    let font = egui::FontId::proportional(12.0);
    let galleys: Vec<_> = label_lines(ui, label, &font)
        .into_iter()
        .map(|line| ui.painter().layout_no_wrap(line, font.clone(), text_color))
        .collect();
    let chevron = if dropdown { 11.0 } else { 0.0 };
    let text_w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
    let width = button_width(text_w, chevron);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON_HEIGHT), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = theme::RADIUS as f32;
        let pressed = response.is_pointer_button_down_on();
        if selected {
            painter.rect_filled(rect, radius, PALETTE.accent_soft);
            let border = Stroke::new(1.0, PALETTE.accent.gamma_multiply(0.55));
            painter.rect_stroke(rect, radius, border, StrokeKind::Inside);
        } else if enabled && (response.hovered() || pressed) {
            painter.rect_filled(rect, radius, if pressed { PALETTE.pressed } else { PALETTE.hover });
        }
        let icon_color = match (enabled, selected) {
            (false, _) => PALETTE.text_faint,
            (true, true) => PALETTE.accent_text,
            (true, false) => PALETTE.text,
        };
        let icon_center = egui::pos2(rect.center().x, rect.top() + 6.0 + ICON_SIZE / 2.0);
        let mut tones = Tones::of(theme::palette(ui.ctx()), glyph.badge(), enabled, selected);
        if let Some(ink) = ink.filter(|_| enabled) {
            tones.ink = ink;
        }
        let icon_rect = egui::Rect::from_center_size(icon_center, vec2(ICON_SIZE, ICON_SIZE));
        glyph.paint(painter, icon_rect, tones);
        // Lines start right under the icon, each centered; the chevron follows the last one.
        let text_color = if selected { PALETTE.accent_text } else { text_color };
        let mut y = rect.top() + 6.0 + ICON_SIZE + 3.0;
        let n = galleys.len();
        for (k, galley) in galleys.into_iter().enumerate() {
            let size = galley.size();
            let extra = if k + 1 == n { chevron } else { 0.0 };
            let pos = egui::pos2(rect.center().x - (size.x + extra) / 2.0, y);
            if dropdown && k + 1 == n {
                let side = size.y - 3.0;
                let c = egui::Rect::from_min_size(egui::pos2(pos.x + size.x + 2.0, pos.y + 2.0), vec2(side, side));
                icons::paint(painter, c, Icon::ChevronDown, icon_color);
            }
            painter.galley_with_override_text_color(pos, galley, text_color);
            y += size.y - 1.0;
        }
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label));
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// Rows of a stacked family: families of 2 or 4 use two rows (one column,
/// or 2 x 2), everything else three.
fn stack_rows(n: usize) -> usize {
    if n == 2 || n == 4 { 2 } else { 3 }
}

/// One family of related commands as small icon buttons in columns of
/// two or three (see [`stack_rows`]); each family starts its own column,
/// so unrelated commands never share one. Two-row columns are centered on
/// the large buttons' height. Returns the clicked command.
fn stacked(ui: &mut Ui, state: &AppState, family: &[UiCommand]) -> Option<UiCommand> {
    if let Some(first) = family.first() {
        remember_icon(ui.ctx(), first.icon());
    }
    let mut clicked = None;
    stack_columns(ui, family, |ui, command| {
        let why = command.unavailable(state);
        let enabled = why.is_none();
        let glyph = Glyph::Icon(command.icon(), command.badge());
        let r = small_button(ui, glyph, &command.label(state), enabled, command.selected(state));
        if command_tooltip(r, state, *command, why).clicked() && enabled {
            clicked = Some(*command);
        }
    });
    clicked
}

/// `items` in columns of two or three (see [`stack_rows`]), two-row
/// columns centered on the large buttons' height.
fn stack_columns<T>(ui: &mut Ui, items: &[T], mut each: impl FnMut(&mut Ui, &T)) {
    let rows = stack_rows(items.len());
    let column_h = rows as f32 * STACK_BUTTON.y + (rows as f32 - 1.0) * 2.0;
    for chunk in items.chunks(rows) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add_space(((BUTTON_HEIGHT - column_h) / 2.0).max(0.0));
            for item in chunk {
                each(ui, item);
            }
        });
    }
}

/// One family of related commands as medium buttons (icon and label on one
/// line) in columns of two or three, like [`stacked`]; a column's buttons
/// share its widest one's width. Returns the clicked command.
fn stacked_labelled(ui: &mut Ui, state: &AppState, family: &[UiCommand]) -> Option<UiCommand> {
    let mut clicked = None;
    let rows = stack_rows(family.len());
    let column_h = rows as f32 * STACK_BUTTON.y + (rows as f32 - 1.0) * 2.0;
    for chunk in family.chunks(rows) {
        ui.vertical(|ui| {
            ui.add_space(((BUTTON_HEIGHT - column_h) / 2.0).max(0.0));
            clicked = labelled_column(ui, state, chunk).or(clicked);
        });
    }
    clicked
}

/// `commands` as medium buttons one under the other, all as wide as the
/// widest. Returns the clicked command.
fn labelled_column(ui: &mut Ui, state: &AppState, commands: &[UiCommand]) -> Option<UiCommand> {
    if let Some(first) = commands.first() {
        remember_icon(ui.ctx(), first.icon());
    }
    let font = egui::FontId::proportional(12.0);
    let labels: Vec<String> = commands.iter().map(|c| c.label(state)).collect();
    let text_w = labels
        .iter()
        .map(|l| ui.painter().layout_no_wrap(l.clone(), font.clone(), PALETTE.text).size().x)
        .fold(0.0, f32::max);
    let width = MEDIUM_PAD + MEDIUM_ICON_SIZE + MEDIUM_GAP + text_w + MEDIUM_PAD;
    ui.spacing_mut().item_spacing.y = 2.0;
    let mut clicked = None;
    for (command, label) in commands.iter().zip(labels) {
        let why = command.unavailable(state);
        let enabled = why.is_none();
        let glyph = Glyph::Icon(command.icon(), command.badge());
        let r = medium_button(ui, glyph, &label, width, enabled, command.selected(state));
        if command_tooltip(r, state, *command, why).clicked() && enabled {
            clicked = Some(*command);
        }
    }
    clicked
}

/// A large button named `label` (with the first command's icon) that opens a
/// menu of `commands` as medium buttons: for a family whose names are too
/// long to show in the row. Unavailable ones are greyed, their reason on hover.
fn menu_of(ui: &mut Ui, state: &mut AppState, label: &str, commands: &[UiCommand]) {
    let Some(first) = commands.first() else { return };
    let r = big_button(ui, Glyph::Icon(first.icon(), None), label, false, true, true);
    let mut clicked = None;
    egui::Popup::menu(&r).show(|ui| clicked = labelled_column(ui, state, commands));
    if let Some(command) = clicked {
        commands::run(state, &ui.ctx().clone(), command);
    }
}

/// Icon size of a medium ribbon button.
const MEDIUM_ICON_SIZE: f32 = 16.0;
/// Room before the icon and after the label of a medium button.
const MEDIUM_PAD: f32 = 4.0;
/// Room between a medium button's icon and its label.
const MEDIUM_GAP: f32 = 5.0;

/// A medium ribbon button: a small icon with its label on one line, for the
/// secondary commands of a group, whose icons do not explain themselves.
fn medium_button(ui: &mut Ui, glyph: Glyph<'_>, label: &str, width: f32, enabled: bool, selected: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, r) = ui.allocate_exact_size(vec2(width, STACK_BUTTON.y), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = theme::RADIUS as f32;
        if selected {
            painter.rect_filled(rect, radius, PALETTE.accent_soft);
            let border = Stroke::new(1.0, PALETTE.accent.gamma_multiply(0.55));
            painter.rect_stroke(rect, radius, border, StrokeKind::Inside);
        } else if enabled && r.hovered() {
            let fill = if r.is_pointer_button_down_on() { PALETTE.pressed } else { PALETTE.hover };
            painter.rect_filled(rect, radius, fill);
        }
        let color = match (enabled, selected) {
            (false, _) => PALETTE.text_faint,
            (true, true) => PALETTE.accent_text,
            (true, false) => PALETTE.text,
        };
        let icon_center = egui::pos2(rect.left() + MEDIUM_PAD + MEDIUM_ICON_SIZE / 2.0, rect.center().y);
        let icon_rect = egui::Rect::from_center_size(icon_center, vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE));
        let tones = Tones { ink: color, ..Tones::of(theme::palette(ui.ctx()), glyph.badge(), enabled, selected) };
        glyph.paint(painter, icon_rect, tones);
        let text_pos = egui::pos2(icon_rect.right() + MEDIUM_GAP, rect.center().y);
        painter.text(text_pos, egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.0), color);
    }
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label));
    if enabled { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r }
}

/// Size of a small stacked ribbon button.
const STACK_BUTTON: egui::Vec2 = vec2(28.0, (BUTTON_HEIGHT - 4.0) / 3.0);

/// A small stacked ribbon button: an icon, its label told to screen readers
/// (and tests) and in the tooltip.
fn small_button(ui: &mut Ui, glyph: Glyph<'_>, label: &str, enabled: bool, selected: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, r) = ui.allocate_exact_size(STACK_BUTTON, sense);
    let painter = ui.painter();
    let radius = theme::RADIUS as f32;
    if selected {
        painter.rect_filled(rect, radius, PALETTE.accent_soft);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, PALETTE.accent.gamma_multiply(0.55)), StrokeKind::Inside);
    } else if enabled && r.hovered() {
        let fill = if r.is_pointer_button_down_on() { PALETTE.pressed } else { PALETTE.hover };
        painter.rect_filled(rect, radius, fill);
    }
    let color = match (enabled, selected) {
        (false, _) => PALETTE.text_faint,
        (true, true) => PALETTE.accent_text,
        (true, false) => PALETTE.text,
    };
    let icon_rect = egui::Rect::from_center_size(rect.center(), vec2(STACK_ICON_SIZE, STACK_ICON_SIZE));
    let tones = Tones { ink: color, ..Tones::of(theme::palette(ui.ctx()), glyph.badge(), enabled, selected) };
    glyph.paint(painter, icon_rect, tones);
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label));
    if enabled { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r }
}

/// Small square icon button (quick access toolbar).
fn icon_button(ui: &mut Ui, (icon, badge): (Icon, Option<Badge>), enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 26.0), sense);
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, theme::RADIUS as f32, PALETTE.hover);
    }
    let color = if enabled { PALETTE.text_muted } else { PALETTE.text_faint.gamma_multiply(0.6) };
    let tones = Tones { ink: color, ..Tones::of(theme::palette(ui.ctx()), badge, enabled, false) };
    icons::paint_toned(ui.painter(), rect.shrink(5.0), icon, tones, badge);
    response
}

/// A group of the component environment's ribbon.
pub fn env_group(ui: &mut Ui, state: &mut AppState, caption: String, commands: &[UiCommand]) {
    group(ui, caption, |ui| command_buttons(ui, state, commands));
}

/// A group of large buttons, on the accent ground when `highlight`.
pub fn sketch_group(ui: &mut Ui, state: &mut AppState, caption: String, highlight: bool, commands: &[UiCommand]) {
    group_with(ui, caption, highlight, None, |ui| command_buttons(ui, state, commands));
}

/// A group of small buttons in columns, on the accent ground when `highlight`.
pub fn sketch_stacks(ui: &mut Ui, state: &mut AppState, caption: String, highlight: bool, families: &[&[UiCommand]]) {
    group_with(ui, caption, highlight, None, |ui| stacks(ui, state, families));
}

/// Whether the main ribbon shows an open sketch: its Modify tab becomes
/// *Modify | Create Boundary: …* with the sketch's tools, and the other tabs
/// wait until it is finished or cancelled (the component environment keeps
/// its own ribbon).
pub fn sketching(state: &AppState) -> bool {
    state.sketch.is_some() && state.component_env.is_none()
}

/// Open the Modify tab when a sketch starts, and bring back the tab shown
/// before when it ends (if Modify is still shown).
fn follow_sketch(state: &mut AppState) {
    let now = sketching(state);
    if now == state.ribbon_sketching {
        return;
    }
    state.ribbon_sketching = now;
    if now {
        if state.ribbon_tab != RibbonTab::Modify {
            state.ribbon_tab_before_tool = Some(state.ribbon_tab);
        }
        state.ribbon_tab = RibbonTab::Modify;
    } else if !places(&state.tool)
        && let Some(before) = state.ribbon_tab_before_tool.take()
        && state.ribbon_tab == RibbonTab::Modify
    {
        state.ribbon_tab = before;
    }
}

/// A captioned group of ribbon buttons followed by a vertical divider.
fn group(ui: &mut Ui, caption: String, add: impl FnOnce(&mut Ui)) {
    group_with(ui, caption, false, None, add);
}

/// A group's launcher: the command opening the settings of what the group
/// makes, drawn as a small arrow at the right of its caption, and its name.
type Launcher = Option<(UiCommand, String)>;

/// The launcher of `command`, named for `state`.
fn launcher(state: &AppState, command: UiCommand) -> Launcher {
    Some((command, command.label(state)))
}

/// [`group`], on a soft accent ground when `highlight` (the Modify tab's
/// groups for the selected categories).
/// `launcher` adds a small arrow at the right of the caption. While the row
/// is compacted (see [`tab_row`]) a group from [`RowFrame::collapse_from`] on
/// is one large button, named after the group, that opens its buttons as a menu.
fn group_with(ui: &mut Ui, caption: String, highlight: bool, launcher: Launcher, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let mut frame = RowFrame::get(&ctx);
    let index = frame.next;
    frame.next += 1;
    frame.set(&ctx);
    let collapsed = !frame.popup && index >= frame.collapse_from;
    let icon_id = egui::Id::new(("ribbon-group-icon", caption.clone()));
    let ground = ui.painter().add(egui::Shape::Noop);
    let caption_font = egui::FontId::proportional(11.0);
    let caption_w = ui.painter().layout_no_wrap(caption.clone(), caption_font, PALETTE.text).size().x;
    let launcher_w = if launcher.is_some() { LAUNCHER_SIZE + 4.0 } else { 0.0 };
    let inner = ui.vertical(|ui| {
        // At least as wide as the caption, so it never wraps.
        ui.set_min_width(caption_w + launcher_w + 8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if collapsed {
                let icon = ctx.data(|d| d.get_temp::<Icon>(icon_id)).unwrap_or(Icon::ChevronDown);
                let r = big_button(ui, Glyph::Icon(icon, None), &caption, false, true, true);
                egui::Popup::menu(&r).show(|ui| {
                    RowFrame::popup(&ctx, true);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        add(ui);
                    });
                    RowFrame::popup(&ctx, false);
                });
            } else {
                // The first icon drawn in the group stands for it once it is collapsed.
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CURRENT_GROUP), Some(icon_id)));
                add(ui);
                ctx.data_mut(|d| d.insert_temp(egui::Id::new(CURRENT_GROUP), None::<egui::Id>));
            }
        });
        let width = ui.min_rect().width();
        let caption_row = ui.allocate_ui_with_layout(vec2(width, 14.0), Layout::top_down(Align::Center), |ui| {
            ui.label(RichText::new(&caption).size(11.0).color(PALETTE.text_muted));
        });
        if let Some((command, label)) = launcher {
            let row = caption_row.response.rect;
            let at = egui::pos2(row.left() + width - LAUNCHER_SIZE / 2.0 - 1.0, row.center().y);
            let rect = egui::Rect::from_center_size(at, vec2(LAUNCHER_SIZE, LAUNCHER_SIZE));
            let r = ui.interact(rect, egui::Id::new(("ribbon-launcher", command.id())), Sense::click());
            paint_launcher(ui.painter(), rect, r.hovered());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            if r.on_hover_text(&label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                defer(&ctx, command);
            }
        }
    });
    let height = inner.response.rect.height();
    if !frame.popup {
        let width = inner.response.rect.width() + 13.0;
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<Vec<(bool, f32)>>(egui::Id::new(DRAWN_GROUPS)).push((collapsed, width))
        });
    }
    if highlight {
        let rect = inner.response.rect.expand2(vec2(4.0, 2.0));
        ui.painter().set(ground, egui::Shape::rect_filled(rect, theme::RADIUS as f32, PALETTE.accent_soft));
    }
    let (r, _) = ui.allocate_exact_size(vec2(13.0, height), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range().shrink(4.0), Stroke::new(1.0, PALETTE.border));
}

/// Side of a group's launcher.
const LAUNCHER_SIZE: f32 = 11.0;

/// The launcher's arrow: a corner and a diagonal pointing down and right.
fn paint_launcher(painter: &egui::Painter, rect: egui::Rect, hovered: bool) {
    if hovered {
        painter.rect_filled(rect.expand(2.0), 2.0, PALETTE.hover);
    }
    let stroke = Stroke::new(1.2, if hovered { PALETTE.accent_text } else { PALETTE.text_muted });
    let r = rect.shrink(1.5);
    painter.line_segment([r.left_top(), r.right_bottom()], stroke);
    painter.line_segment([r.right_bottom(), r.right_bottom() - vec2(4.0, 0.0)], stroke);
    painter.line_segment([r.right_bottom(), r.right_bottom() - vec2(0.0, 4.0)], stroke);
}

/// Temp-data key of the group being drawn (the key its icon is kept under).
const CURRENT_GROUP: &str = "ribbon-current-group";

/// Keep `icon` as the icon of the group being drawn, unless it has one.
fn remember_icon(ctx: &egui::Context, icon: Icon) {
    let Some(Some(group)) = ctx.data(|d| d.get_temp::<Option<egui::Id>>(egui::Id::new(CURRENT_GROUP))) else {
        return;
    };
    if ctx.data(|d| d.get_temp::<Icon>(group)).is_none() {
        ctx.data_mut(|d| d.insert_temp(group, icon));
    }
}

/// The quick access toolbar's first commands, which the user cannot remove.
const QUICK_ACCESS_FIXED: [UiCommand; 6] =
    [UiCommand::Home, UiCommand::New, UiCommand::Open, UiCommand::Save, UiCommand::Undo, UiCommand::Redo];

/// The quick access toolbar's commands after Undo and Redo when the user has not changed them.
fn default_quick_access() -> Vec<UiCommand> {
    vec![
        UiCommand::ExportPdf,
        UiCommand::Measure,
        UiCommand::Dimension(DimensionMode::Aligned),
        UiCommand::Text,
        UiCommand::Default3D,
        UiCommand::Section,
        UiCommand::CloseInactiveViews,
    ]
}

/// The quick access toolbar's commands after Undo and Redo: the user's
/// (`Preferences::quick_access`, unknown ids skipped) or the default set.
pub fn quick_access(state: &AppState) -> Vec<UiCommand> {
    let Some(ids) = &state.prefs.quick_access else { return default_quick_access() };
    let all = UiCommand::all();
    ids.iter().filter_map(|id| all.iter().find(|c| c.id() == *id).copied()).collect()
}

/// A change to the quick access toolbar a menu asked for, applied once the ribbon is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
enum QatEdit {
    Add(UiCommand),
    Remove(UiCommand),
    /// Move the command one place left (-1) or right (+1).
    Move(UiCommand, isize),
    Reset,
}

/// Temp-data key of a pending [`QatEdit`].
const QAT_EDIT: &str = "ribbon-qat-edit";

/// Apply `edit` to the user's quick access commands.
fn apply_quick_access(state: &mut AppState, edit: QatEdit) {
    let mut list = quick_access(state);
    match edit {
        QatEdit::Add(c) if !list.contains(&c) => list.push(c),
        QatEdit::Remove(c) => list.retain(|x| *x != c),
        QatEdit::Move(c, by) => {
            if let Some(i) = list.iter().position(|x| *x == c) {
                let j = i.saturating_add_signed(by).min(list.len().saturating_sub(1));
                list.swap(i, j);
            }
        }
        QatEdit::Reset => {
            state.prefs.quick_access = None;
            return;
        }
        QatEdit::Add(_) => {}
    }
    state.prefs.quick_access = Some(list.iter().map(|c| c.id()).collect());
}

/// The ▾ at the end of the quick access toolbar: its commands, each moved
/// left or right or removed, and the default set back.
fn quick_access_menu(ui: &mut Ui, state: &AppState) {
    let label = t!("qat-customize");
    // A fixed id, so the menu stays open while its list changes the toolbar's width.
    let (rect, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
    let r = ui.interact(rect, egui::Id::new("quick-access-menu"), Sense::click());
    if r.hovered() {
        ui.painter().rect_filled(rect, theme::RADIUS as f32, PALETTE.hover);
    }
    icons::paint(ui.painter(), rect.shrink(7.0), Icon::ChevronDown, PALETTE.text_muted);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    let r = r.on_hover_text(&label);
    let mut edit = None;
    egui::Popup::menu(&r).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.set_min_width(260.0);
        ui.label(RichText::new(&label).family(theme::semibold()).color(PALETTE.text));
        let list = quick_access(state);
        if list.is_empty() {
            ui.label(theme::muted(t!("qat-empty")));
        }
        let last = list.len().saturating_sub(1);
        for (i, command) in list.iter().enumerate() {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                icons::paint(ui.painter(), rect, command.icon(), PALETTE.text_muted);
                ui.label(command.label(state));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("×").on_hover_text(t!("qat-remove")).clicked() {
                        edit = Some(QatEdit::Remove(*command));
                    }
                    if ui.add_enabled(i < last, Button::new("→").small()).on_hover_text(t!("qat-move-right")).clicked()
                    {
                        edit = Some(QatEdit::Move(*command, 1));
                    }
                    if ui.add_enabled(i > 0, Button::new("←").small()).on_hover_text(t!("qat-move-left")).clicked() {
                        edit = Some(QatEdit::Move(*command, -1));
                    }
                });
            });
        }
        ui.separator();
        if ui.button(t!("qat-reset")).clicked() {
            edit = Some(QatEdit::Reset);
        }
    });
    if let Some(edit) = edit {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(QAT_EDIT), Some(edit)));
    }
}

/// Temp-data key of a command a ribbon click asked for, run once the row is drawn.
const DEFERRED: &str = "ribbon-deferred";

/// Run `command` once the ribbon is drawn (for clicks where the state is borrowed).
fn defer(ctx: &egui::Context, command: UiCommand) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(DEFERRED), Some(command)));
}

/// What the groups drawn this frame read about the row's compaction.
#[derive(Clone, Copy)]
struct RowFrame {
    /// Rows of large buttons are drawn as medium ones.
    medium: bool,
    /// Groups from this index on are drawn as menus.
    collapse_from: usize,
    /// The index of the next group drawn.
    next: usize,
    /// A collapsed group's menu is being drawn: its buttons as designed.
    popup: bool,
}

impl Default for RowFrame {
    fn default() -> Self {
        Self { medium: false, collapse_from: usize::MAX, next: 0, popup: false }
    }
}

impl RowFrame {
    fn id() -> egui::Id {
        egui::Id::new("ribbon-row-frame")
    }

    fn get(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp(Self::id())).unwrap_or_default()
    }

    fn set(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(Self::id(), self));
    }

    fn popup(ctx: &egui::Context, on: bool) {
        let mut frame = Self::get(ctx);
        frame.popup = on;
        frame.set(ctx);
    }

    /// Whether rows of large buttons are drawn as medium ones here.
    fn medium_here(ctx: &egui::Context) -> bool {
        let frame = Self::get(ctx);
        frame.medium && !frame.popup
    }
}

/// How far a tab's row is compacted to fit the window: 0 as designed, 1 its
/// rows of large buttons as medium ones, 1 + k also its last k groups as menus.
/// Each group's width is measured as it is drawn, so the level that fits is
/// predicted rather than reached one step per frame.
#[derive(Clone, Default)]
struct Compaction {
    level: usize,
    /// The groups the tab drew last frame.
    groups: usize,
    /// The row's width as designed, when last drawn so.
    full: Option<f32>,
    /// Each group's width with medium buttons, and as a menu.
    medium: Vec<Option<f32>>,
    collapsed: Vec<Option<f32>>,
}

/// Width assumed for a group drawn as a menu before it has been measured.
const COLLAPSED_GUESS: f32 = 84.0;

impl Compaction {
    /// The frame the groups read at this level.
    fn frame(&self) -> RowFrame {
        let collapsed = self.level.saturating_sub(1);
        RowFrame {
            medium: self.level >= 1,
            collapse_from: if collapsed == 0 { usize::MAX } else { self.groups.saturating_sub(collapsed) },
            next: 0,
            popup: false,
        }
    }

    /// After a frame: the row was `content` wide for `room`, its groups as
    /// `drawn` (whether a menu, width). Keeps the widths and picks the least
    /// compaction predicted to fit. Returns whether the level changed.
    fn measured(&mut self, content: f32, room: f32, drawn: &[(bool, f32)]) -> bool {
        let n = drawn.len();
        self.groups = n;
        self.medium.resize(n, None);
        self.collapsed.resize(n, None);
        for (i, (collapsed, width)) in drawn.iter().enumerate() {
            if *collapsed {
                self.collapsed[i] = Some(*width);
            } else if self.level >= 1 {
                self.medium[i] = Some(*width);
            }
        }
        if self.level == 0 {
            self.full = Some(content);
        }
        let fits = |width: f32| width <= room - 2.0;
        let target = if self.full.is_some_and(fits) {
            0
        } else if self.level == 0 || self.medium.iter().any(Option::is_none) {
            // The medium widths are measured at level 1 first.
            1
        } else {
            let predict = |k: usize| -> f32 {
                (0..n)
                    .map(|i| match i >= n - k {
                        true => self.collapsed[i].unwrap_or(COLLAPSED_GUESS),
                        false => self.medium[i].unwrap_or(0.0),
                    })
                    .sum()
            };
            1 + (0..=n).find(|k| fits(predict(*k))).unwrap_or(n)
        };
        let changed = target != self.level;
        self.level = target;
        changed
    }
}

/// Temp-data key of the groups drawn this frame: whether each is a menu, and its width.
const DRAWN_GROUPS: &str = "ribbon-drawn-groups";

/// A ribbon tab title in the top strip.
/// Font of a ribbon tab title (semibold when active).
fn tab_font(active: bool) -> egui::FontId {
    egui::FontId::new(13.0, if active { theme::semibold() } else { egui::FontFamily::Proportional })
}

/// Room around each tab title's text: [`TAB_PAD`], shrunk evenly down to
/// [`TAB_PAD_MIN`] when the titles (of `text_widths`) do not fit in `available`.
fn tab_padding(text_widths: &[f32], available: f32) -> f32 {
    let n = text_widths.len().max(1) as f32;
    let text: f32 = text_widths.iter().sum();
    let fit = (available - text) / n;
    if fit.is_finite() { fit.clamp(TAB_PAD_MIN, TAB_PAD) } else { TAB_PAD }
}

/// Room around a ribbon tab title's text, pixels.
const TAB_PAD: f32 = 28.0;
/// The least room around a title in a narrow window.
const TAB_PAD_MIN: f32 = 4.0;

/// A tab title in a cell of `width`, its text centered.
/// `contextual` (Modify while something is selected) sits on a soft accent ground.
fn tab_title(ui: &mut Ui, label: &str, active: bool, contextual: bool, width: f32) -> Response {
    let color = if active { PALETTE.accent_text } else { PALETTE.text_muted };
    let galley = ui.painter().layout_no_wrap(label.to_string(), tab_font(active), color);
    let (rect, response) = ui.allocate_exact_size(vec2(width, galley.size().y + 12.0), Sense::click());
    if contextual {
        ui.painter().rect_filled(rect.shrink2(vec2(2.0, 3.0)), theme::RADIUS as f32, PALETTE.accent_soft);
    } else if response.hovered() && !active {
        ui.painter().rect_filled(rect.shrink2(vec2(2.0, 3.0)), theme::RADIUS as f32, PALETTE.hover);
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, label));
    let text_w = galley.size().x;
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    if active {
        let y = rect.bottom() - 1.0;
        let half = text_w / 2.0 + 4.0;
        let line = [egui::pos2(rect.center().x - half, y), egui::pos2(rect.center().x + half, y)];
        ui.painter().line_segment(line, Stroke::new(2.0, PALETTE.accent));
    }
    response
}

/// A large ribbon button for `command`: its state, label and tooltip come from the command.
fn command_button(ui: &mut Ui, state: &mut AppState, command: UiCommand) {
    if command_big(ui, state, command, false).clicked() {
        commands::run(state, &ui.ctx().clone(), command);
    }
}

/// A large ribbon button for `command` that opens a menu (the caller attaches it).
fn command_menu(ui: &mut Ui, state: &AppState, command: UiCommand) -> Response {
    command_big(ui, state, command, true)
}

fn command_big(ui: &mut Ui, state: &AppState, command: UiCommand, dropdown: bool) -> Response {
    remember_icon(ui.ctx(), command.icon());
    let why = command.unavailable(state);
    let label = command.label(state);
    let glyph = Glyph::Icon(command.icon(), command.badge());
    // A sketch's Finish and Cancel in the colors of yes and no.
    let ink = match command {
        UiCommand::SketchFinish => Some(PALETTE.success),
        UiCommand::SketchCancel => Some(PALETTE.danger),
        _ => None,
    };
    let r = big_button_ink(ui, glyph, &label, command.selected(state), why.is_none(), dropdown, ink);
    command_tooltip(r, state, command, why)
}

/// The rich tooltip of a command: title and shortcut, what it does, why it
/// is unavailable, and the engine command that scripts call for the same change.
/// With detailed tooltips off (a preference), only the title, shortcut and
/// why it is unavailable.
/// The tooltip also comes with a right-click menu that adds the command to
/// the quick access toolbar, or removes it from there.
pub fn command_tooltip(response: Response, state: &AppState, command: UiCommand, why: Option<String>) -> Response {
    let title = command.title(state);
    let detailed = state.prefs.detailed_tooltips;
    if !QUICK_ACCESS_FIXED.contains(&command) {
        let shown = quick_access(state).contains(&command);
        response.context_menu(|ui| {
            let (label, edit) = if shown {
                (t!("qat-remove"), QatEdit::Remove(command))
            } else {
                (t!("qat-add"), QatEdit::Add(command))
            };
            if ui.button(label).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(QAT_EDIT), Some(edit)));
                ui.close();
            }
        });
    }
    response.on_hover_ui(|ui| {
        ui.set_max_width(300.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new(&title).family(theme::semibold()).color(PALETTE.text));
            if let Some(keys) = command.shortcut() {
                key_cap(ui, &keys);
            }
        });
        if detailed {
            ui.label(theme::muted(command.description()));
        }
        if let Some(why) = &why {
            ui.label(RichText::new(t!("tip-unavailable", "reason" => why)).color(PALETTE.warning));
        }
        if let Some(name) = command.engine().filter(|_| detailed) {
            ui.label(RichText::new(t!("tip-engine", "command" => name)).size(11.0).color(PALETTE.text_faint));
        }
    })
}

/// A keyboard shortcut drawn as a key cap.
pub(crate) fn key_cap(ui: &mut Ui, keys: &str) -> egui::Response {
    egui::Frame::new()
        .fill(PALETTE.surface)
        .stroke(Stroke::new(1.0, PALETTE.border))
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(keys).size(11.0).color(PALETTE.text_muted));
        })
        .response
}

/// A small quick access button for `command`.
fn command_icon(ui: &mut Ui, state: &mut AppState, command: UiCommand) {
    let why = command.unavailable(state);
    let r = icon_button(ui, (command.icon(), command.badge()), why.is_none());
    let label = command.label(state);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, why.is_none(), &label));
    if command_tooltip(r, state, command, why).clicked() {
        commands::run(state, &ui.ctx().clone(), command);
    }
}

pub fn ribbon(ui: &mut Ui, state: &mut AppState) {
    follow_sketch(state);
    let sketch = sketching(state);
    // Measured first: rows wider than the window grow the Ui's rectangle.
    let full_width = ui.available_width();
    // Quick access toolbar at the top left, where desktop applications keep it.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        ui.add_space(2.0);
        let (mark, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
        if let Some(logo) = theme::logo(ui.ctx()) {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(logo.id(), mark, uv, egui::Color32::WHITE);
        }
        ui.add_space(8.0);
        let divider = |ui: &mut Ui| {
            let (div, _) = ui.allocate_exact_size(vec2(9.0, 18.0), Sense::hover());
            ui.painter().vline(div.center().x, div.y_range(), Stroke::new(1.0, PALETTE.border));
        };
        for command in [UiCommand::Home, UiCommand::New, UiCommand::Open, UiCommand::Save] {
            command_icon(ui, state, command);
        }
        divider(ui);
        command_icon(ui, state, UiCommand::Undo);
        command_icon(ui, state, UiCommand::Redo);
        // The user's frequent commands (right-click a ribbon button to add it), then the menu to arrange them.
        divider(ui);
        for command in quick_access(state) {
            command_icon(ui, state, command);
        }
        quick_access_menu(ui, state);
    });
    // Ribbon tab titles, each as wide as its own text (measured bold, as when
    // active, so a tab keeps its width when picked) plus some room around it.
    let mut tabs = vec![
        (RibbonTab::File, t!("ribbon-file")),
        (RibbonTab::Architecture, t!("ribbon-architecture")),
        (RibbonTab::Structure, t!("ribbon-structure")),
        (RibbonTab::Mep, t!("ribbon-mep")),
        (RibbonTab::MassingSite, t!("ribbon-massing-site")),
        (RibbonTab::Annotate, t!("ribbon-annotate")),
        (RibbonTab::View, t!("ribbon-view")),
        (RibbonTab::Insert, t!("ribbon-insert")),
        (RibbonTab::Manage, t!("ribbon-manage")),
    ];
    // The Plugins tab, while a loaded plugin has buttons.
    if state.plugins.buttons.is_empty() {
        if state.ribbon_tab == RibbonTab::Plugins {
            state.ribbon_tab = RibbonTab::Manage;
        }
    } else {
        tabs.push((RibbonTab::Plugins, t!("ribbon-plugins")));
    }
    // Modify last: the selection's edits, where the ribbon goes when something is selected.
    tabs.push((RibbonTab::Modify, modify_title(state)));
    let text_widths: Vec<f32> = tabs
        .iter()
        .map(|(_, label)| ui.painter().layout_no_wrap(label.clone(), tab_font(true), PALETTE.text).size().x)
        .collect();
    let pad = tab_padding(&text_widths, full_width - 40.0);
    let collapsed = state.ribbon_collapsed;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for ((tab, label), text_w) in tabs.into_iter().zip(text_widths) {
            let width = text_w + pad;
            let active = state.ribbon_tab == tab && (!collapsed || state.ribbon_peek);
            let contextual = tab == RibbonTab::Modify && (!state.selection.is_empty() || places(&state.tool) || sketch);
            // While a sketch is open only its tab answers: finish or cancel it first.
            let waits = sketch && tab != RibbonTab::Modify;
            let r = ui.add_enabled_ui(!waits, |ui| tab_title(ui, &label, active, contextual, width)).inner;
            let r = if waits { r.on_disabled_hover_text(t!("sketch-finish-first")) } else { r };
            // A double click collapses or expands; a click on a collapsed
            // ribbon shows that tab's panel over the views, or hides it again.
            if r.double_clicked() {
                state.ribbon_collapsed = !state.ribbon_collapsed;
                state.ribbon_peek = false;
            } else if r.clicked() {
                if tab == RibbonTab::File {
                    // File is a page over the window, not a row of buttons.
                    if state.ribbon_tab != RibbonTab::File {
                        crate::backstage::opened(ui.ctx(), state.ribbon_tab);
                    }
                    state.ribbon_peek = false;
                } else if collapsed {
                    state.ribbon_peek = !(state.ribbon_peek && state.ribbon_tab == tab);
                }
                state.ribbon_tab = tab;
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            command_icon(ui, state, UiCommand::CommandPalette);
            let command = UiCommand::ToggleRibbon;
            let icon = if collapsed { Icon::ChevronDown } else { Icon::ChevronUp };
            let r = icon_button(ui, (icon, None), true);
            let label = command.label(state);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            if command_tooltip(r, state, command, None).clicked() {
                commands::run(state, &ui.ctx().clone(), command);
            }
        });
    });
    let strip_bottom = ui.min_rect().bottom();
    ui.painter().hline(ui.max_rect().x_range(), strip_bottom + 3.0, Stroke::new(1.0, PALETTE.border));
    if state.ribbon_tab == RibbonTab::File {
        // The File page covers the window under the tab titles; the row keeps its room.
        if !collapsed {
            ui.add_space(8.0 + BUTTON_HEIGHT + 30.0);
        }
        crate::backstage::show(&ui.ctx().clone(), state, strip_bottom + 4.0);
    } else if !collapsed {
        ui.add_space(8.0);
        tab_row(ui, state, full_width);
    } else if state.ribbon_peek {
        peek(ui, state, strip_bottom + 4.0);
    }
    let ctx = ui.ctx().clone();
    if let Some(Some(command)) = ctx.data_mut(|d| d.remove_temp::<Option<UiCommand>>(egui::Id::new(DEFERRED))) {
        commands::run(state, &ctx, command);
    }
    if let Some(Some(edit)) = ctx.data_mut(|d| d.remove_temp::<Option<QatEdit>>(egui::Id::new(QAT_EDIT))) {
        apply_quick_access(state, edit);
    }
    ui.add_space(2.0);
}

/// The collapsed ribbon's panel, shown over the views until a command
/// runs, Escape is pressed or the pointer clicks elsewhere.
fn peek(ui: &mut Ui, state: &mut AppState, top: f32) {
    let rect = ui.max_rect();
    let palette = theme::palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .shadow(ui.style().visuals.popup_shadow)
        .inner_margin(egui::Margin { left: 10, right: 10, top: 8, bottom: 4 });
    let area = egui::Area::new(egui::Id::new("ribbon-peek"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(rect.left() - 10.0, top))
        .show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                ui.set_width(rect.width());
                tab_row(ui, state, rect.width());
            })
        });
    let panel = area.response.rect;
    let (escape, outside) = ui.input(|i| {
        let pos = i.pointer.interact_pos();
        let outside = i.pointer.any_click() && pos.is_some_and(|p| p.y > top && !panel.contains(p));
        (i.key_pressed(egui::Key::Escape), outside)
    });
    if escape || outside {
        state.ribbon_peek = false;
    }
}

/// Width of the arrows that scroll a ribbon wider than the window.
const SCROLL_ARROW_WIDTH: f32 = 18.0;
/// How far one arrow click scrolls.
const SCROLL_STEP: f32 = 240.0;

/// What the ribbon row measured last frame: content width, offset and visible width.
#[derive(Clone, Copy, Default)]
struct RowExtent {
    content: f32,
    offset: f32,
    visible: f32,
    /// Offset an arrow asked for, applied on the next frame.
    pending: Option<f32>,
}

/// The active tab's groups. When they are wider than the window they
/// scroll sideways, with arrows at the ends (and the mouse wheel).
/// A row too wide is first compacted (see [`Compaction`]), and scrolls only
/// once every group is a menu.
fn tab_row(ui: &mut Ui, state: &mut AppState, available: f32) {
    let id = egui::Id::new(("ribbon-row", state.ribbon_tab as u8));
    let compaction_id = id.with("compaction");
    let mut compaction: Compaction = ui.ctx().data(|d| d.get_temp(compaction_id)).unwrap_or_default();
    compaction.frame().set(ui.ctx());
    let mut extent: RowExtent = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    let overflow = extent.content > available + 0.5;
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let max_offset = (extent.content - extent.visible).max(0.0);
        let mut target = extent.pending.take();
        if overflow && scroll_arrow(ui, false, extent.offset > 0.5).clicked() {
            target = Some((extent.offset - SCROLL_STEP).max(0.0));
        }
        let arrows = if overflow { 2.0 * SCROLL_ARROW_WIDTH } else { 0.0 };
        let mut area = egui::ScrollArea::horizontal()
            .id_salt(id)
            .max_width((available - arrows).max(0.0))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if let Some(offset) = target {
            area = area.horizontal_scroll_offset(offset);
        }
        let out = area.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                match state.ribbon_tab {
                    // File is a page (`backstage`), never a row.
                    RibbonTab::File => {}
                    RibbonTab::Architecture => architecture_tab(ui, state),
                    RibbonTab::Structure => structure_tab(ui, state),
                    RibbonTab::Mep => mep_tab(ui, state),
                    RibbonTab::MassingSite => massing_site_tab(ui, state),
                    RibbonTab::Insert => insert_tab(ui, state),
                    RibbonTab::Modify => modify_tab(ui, state),
                    RibbonTab::Annotate => annotate_tab(ui, state),
                    RibbonTab::View => view_tab(ui, state),
                    RibbonTab::Manage => manage_tab(ui, state),
                    RibbonTab::Plugins => plugins_tab(ui, state),
                }
            });
        });
        if overflow && scroll_arrow(ui, true, extent.offset < max_offset - 0.5).clicked() {
            extent.pending = Some((extent.offset + SCROLL_STEP).min(max_offset));
            ui.ctx().request_repaint();
        }
        extent.content = out.content_size.x;
        extent.offset = out.state.offset.x;
        extent.visible = out.inner_rect.width();
    });
    RowFrame::default().set(ui.ctx());
    let drawn = ui.ctx().data_mut(|d| d.remove_temp::<Vec<(bool, f32)>>(egui::Id::new(DRAWN_GROUPS)));
    if compaction.measured(extent.content, available, &drawn.unwrap_or_default()) {
        ui.ctx().request_repaint();
    }
    ui.ctx().data_mut(|d| d.insert_temp(compaction_id, compaction));
    if (extent.content > available + 0.5) != overflow {
        // The arrows appear or go: lay out again with them.
        ui.ctx().request_repaint();
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, extent));
}

/// An arrow at one end of an overflowing ribbon row.
fn scroll_arrow(ui: &mut Ui, right: bool, enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(SCROLL_ARROW_WIDTH, BUTTON_HEIGHT + 16.0), sense);
    let palette = theme::palette(ui.ctx());
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, theme::RADIUS as f32, palette.hover);
    }
    let color = if enabled { palette.text_muted } else { palette.text_faint.gamma_multiply(0.5) };
    let (c, d) = (rect.center(), if right { 1.0 } else { -1.0 });
    let tip = c + vec2(3.0 * d, 0.0);
    let stroke = Stroke::new(1.6, color);
    ui.painter().line_segment([tip, tip + vec2(-6.0 * d, -6.0)], stroke);
    ui.painter().line_segment([tip, tip + vec2(-6.0 * d, 6.0)], stroke);
    let label = if right { t!("ribbon-scroll-right") } else { t!("ribbon-scroll-left") };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &label));
    response.on_hover_text(label)
}

/// Large buttons for `list`, in order.
/// While the row is compacted, rows of two or more are medium buttons.
fn command_buttons(ui: &mut Ui, state: &mut AppState, list: &[UiCommand]) {
    if list.len() > 1 && RowFrame::medium_here(ui.ctx()) {
        labelled(ui, state, &[list]);
        return;
    }
    for command in list {
        command_button(ui, state, *command);
    }
}

/// Files brought into the project, and the reusable types (components,
/// detail components, title blocks) loaded or made. Exports live in File.
fn insert_tab(ui: &mut Ui, state: &mut AppState) {
    use UiCommand::{
        ComponentLibrary, CreateDetailComponent, ImportCad, ImportGltf, ImportIfc, ImportImage, ImportObj, ImportSkp,
        LoadComponent, NewComponent, NewTitleBlock,
    };
    let imports = [
        ImportIfc,
        #[cfg(feature = "rvt")]
        UiCommand::ImportRvt,
        ImportCad,
        ImportSkp,
        ImportObj,
        ImportGltf,
        ImportImage,
    ];
    group(ui, t!("group-import"), |ui| {
        command_buttons(ui, state, &imports);
        plugin_file_entries(ui, state, false);
    });
    group(ui, t!("group-library"), |ui| {
        command_buttons(ui, state, &[LoadComponent, ComponentLibrary]);
        // Editing a component is in Modify, with one selected.
        labelled(ui, state, &[&[NewComponent, CreateDetailComponent, NewTitleBlock]]);
    });
}

fn place(category: Category) -> UiCommand {
    UiCommand::Place(category)
}

fn architecture_tab(ui: &mut Ui, state: &mut AppState) {
    use Category::*;
    let buttons = |list: &[Category]| list.iter().map(|c| place(*c)).collect::<Vec<_>>();
    group(ui, t!("group-build"), |ui| {
        command_buttons(ui, state, &buttons(&[Wall, Door, Window, Niche]));
        command_button(ui, state, UiCommand::CurtainGrid);
    });
    group(ui, t!("group-horizontal"), |ui| command_buttons(ui, state, &buttons(&[Floor, Ceiling, Roof, Shaft])));
    group(ui, t!("group-circulation"), |ui| command_buttons(ui, state, &buttons(&[Stair, Ramp, Railing])));
    group(ui, t!("group-room"), |ui| command_button(ui, state, place(Room)));
    // Making, editing and loading components live in Insert.
    group(ui, t!("group-model"), |ui| command_buttons(ui, state, &buttons(&[Component, Sweep])));
    // Aligning grid ends is in Modify, with a grid selected.
    group(ui, t!("group-datum"), |ui| command_button(ui, state, place(Grid)));
}

/// Conceptual masses (and the buildings made from them) and the terrain.
fn massing_site_tab(ui: &mut Ui, state: &mut AppState) {
    group(ui, t!("group-massing"), |ui| {
        command_buttons(ui, state, &[place(Category::Mass), UiCommand::PushPull]);
        labelled(ui, state, &[&[UiCommand::MassWalls, UiCommand::MassSlabs, UiCommand::MassRoof]]);
    });
    group(ui, t!("group-site"), |ui| {
        command_buttons(ui, state, &[place(Category::Topography), place(Category::Pad), UiCommand::SiteLocation]);
    });
}

/// The Structure tab's material: a segmented control, three options in one
/// framed box with the one picked filled, which sets the tab's groups.
fn material_selector(ui: &mut Ui, state: &mut AppState) {
    let font = egui::FontId::proportional(12.0);
    let labels: Vec<_> = cc_core::StructuralMaterial::ALL.iter().map(|m| (*m, crate::i18n::tr(m.key()))).collect();
    let text_w = labels
        .iter()
        .map(|(_, l)| ui.painter().layout_no_wrap(l.clone(), font.clone(), PALETTE.text).size().x)
        .fold(0.0, f32::max);
    let segment = vec2(text_w + 24.0, STACK_BUTTON.y);
    let (frame, _) = ui.allocate_exact_size(vec2(segment.x, BUTTON_HEIGHT), Sense::hover());
    let box_rect = egui::Rect::from_center_size(frame.center(), vec2(segment.x, segment.y * labels.len() as f32));
    let radius = theme::RADIUS as f32;
    ui.painter().rect_filled(box_rect, radius, PALETTE.surface);
    for (k, (m, label)) in labels.into_iter().enumerate() {
        let rect = egui::Rect::from_min_size(box_rect.min + vec2(0.0, k as f32 * segment.y), segment);
        let response = ui.interact(rect, ui.id().with(("structure-material", k)), Sense::click());
        let on = state.prefs.structure_material == m;
        let painter = ui.painter();
        if on {
            painter.rect_filled(rect.shrink(1.0), radius, PALETTE.accent_soft);
        } else if response.hovered() {
            painter.rect_filled(rect.shrink(1.0), radius, PALETTE.hover);
        }
        let color = if on { PALETTE.accent_text } else { PALETTE.text };
        let galley = painter.layout_no_wrap(label.clone(), font.clone(), color);
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, label.clone()));
        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            state.prefs.structure_material = m;
        }
    }
    ui.painter().rect_stroke(box_rect, radius, Stroke::new(1.0, PALETTE.border), StrokeKind::Inside);
}

fn structure_tab(ui: &mut Ui, state: &mut AppState) {
    use UiCommand::*;
    use cc_core::StructuralMaterial as M;
    use cc_core::connection::ConnectionKind as K;
    let m = state.prefs.structure_material;
    group(ui, t!("group-structure-material"), |ui| material_selector(ui, state));
    let (column, beam) = (StructurePlace(Category::Column, m), StructurePlace(Category::Beam, m));
    match m {
        M::Concrete => {
            group(ui, t!("group-structure"), |ui| {
                command_buttons(ui, state, &[column, beam, place(Category::Floor)]);
            });
            group(ui, t!("group-reinforcement"), |ui| {
                labelled(ui, state, &[&[TypicalRebar, PileCapRebar, PileRebar], &[RebarView, SteelSchedule]]);
            });
        }
        M::Steel => {
            group(ui, t!("group-structure"), |ui| command_buttons(ui, state, &[column, beam, Bracing]));
            group(ui, t!("group-framing"), |ui| {
                labelled(
                    ui,
                    state,
                    &[&[ColumnsAtGrids, BeamsOnGrids, BeamsBetweenColumns], &[GridFrame, Truss, SteelBuilding]],
                );
                command_button(ui, state, LightSteelFrame);
            });
            group(ui, t!("group-connections"), |ui| {
                labelled(ui, state, &[&[Connection(K::BasePlate), Connection(K::EndPlate), Connection(K::FinPlate)]])
            });
        }
        M::Timber => {
            group(ui, t!("group-structure"), |ui| command_buttons(ui, state, &[column, beam, Bracing]));
            group(ui, t!("group-framing"), |ui| {
                labelled(ui, state, &[&[ColumnsAtGrids, BeamsOnGrids, BeamsBetweenColumns], &[GridFrame, Truss]]);
                command_button(ui, state, WoodFrame);
            });
            group(ui, t!("group-connections"), |ui| {
                labelled(ui, state, &[&[Connection(K::TimberBolts), Connection(K::Hanger)]])
            });
        }
    }
    // Profiles live in Manage, grids in Architecture.
    group(ui, t!("group-foundation"), |ui| {
        command_button(ui, state, place(Category::Foundation));
        labelled(ui, state, &[&[FootingsUnderColumns, StripFootingsUnderWalls, FoundationSlab]]);
    });
    group(ui, t!("group-analytical"), |ui| {
        labelled(ui, state, &[&[SlabJoinSupports, AnalyticalView, AnalyticalExport]]);
    });
    group(ui, t!("group-documentation"), |ui| {
        labelled(ui, state, &[&[StructuralPlan, NumberStructure, StructuralSchedules]]);
    });
}

fn mep_tab(ui: &mut Ui, state: &mut AppState) {
    use UiCommand::*;
    // The pipe catalog (in Manage) opens from the group's launcher too.
    let catalog = launcher(state, PipeCatalog);
    group_with(ui, t!("group-plumbing"), false, catalog, |ui| {
        command_buttons(ui, state, &[place(Category::Pipe), place(Category::Valve), PlaceFixture]);
    });
    group(ui, t!("group-electrical"), |ui| {
        command_buttons(ui, state, &[place(Category::Conduit), place(Category::CableTray), PlaceDevice]);
    });
    group(ui, t!("group-communications"), |ui| {
        use cc_core::DeviceKind as D;
        command_buttons(ui, state, &[PlaceDeviceOf(D::Rack), PlaceDeviceOf(D::DataOutlet), Cabling]);
    });
    group(ui, t!("group-lightning"), |ui| {
        use cc_core::DeviceKind as D;
        command_buttons(ui, state, &[place(Category::LightningConductor), PlaceDeviceOf(D::AirTerminal)]);
        labelled(ui, state, &[&[PlaceDeviceOf(D::EarthElectrode), EarthRing, LpsCheck]]);
    });
    group(ui, t!("group-systems"), |ui| command_buttons(ui, state, &[MepConnect, Systems, MepSizing]));
    // The discipline plans, schedules and panel diagram: names too long for the row, so a menu.
    let documentation = [PlumbingPlan, ElectricalPlan, CommunicationsPlan, LightningPlan, MepSchedules, PanelDiagram];
    let label = t!("group-documentation");
    group(ui, label.clone(), |ui| menu_of(ui, state, &label, &documentation));
}

pub fn dimension_icon(mode: DimensionMode) -> Icon {
    match mode {
        DimensionMode::Aligned => Icon::Dimension,
        DimensionMode::Linear => Icon::DimLinear,
        DimensionMode::Ordinate => Icon::DimOrdinate,
        DimensionMode::Angular => Icon::DimAngular,
        DimensionMode::Radial => Icon::DimRadial,
        DimensionMode::Diameter => Icon::DimDiameter,
    }
}

fn annotate_tab(ui: &mut Ui, state: &mut AppState) {
    group(ui, t!("group-dimension"), |ui| {
        command_buttons(ui, state, &DimensionMode::ALL.map(UiCommand::Dimension));
    });
    // Line styles and tag types (in Manage) open from the groups' launchers too.
    let styles = launcher(state, UiCommand::LineStyles);
    group_with(ui, t!("group-detail"), false, styles, |ui| {
        command_buttons(ui, state, &[UiCommand::DetailLine, UiCommand::FilledRegion, UiCommand::MaskingRegion])
    });
    // Making a detail component lives in Insert, with the other reusable types.
    group(ui, t!("group-detail-components"), |ui| command_button(ui, state, UiCommand::DetailComponent));
    // Adding a leader is in Modify, with a text note selected.
    group(ui, t!("group-text"), |ui| command_button(ui, state, UiCommand::Text));
    group(ui, t!("group-level-marks"), |ui| {
        command_buttons(ui, state, &[UiCommand::LevelMarkPlan, UiCommand::LevelMarkSection]);
    });
    let tag_types = launcher(state, UiCommand::TagTypes);
    group_with(ui, t!("group-tags"), false, tag_types, |ui| command_button(ui, state, UiCommand::StructureTags));
}

/// The text note selected alone, if any.
pub fn selected_text(state: &AppState) -> Option<cc_core::ElementId> {
    let [id] = state.selection.iter().copied().collect::<Vec<_>>()[..] else { return None };
    matches!(state.project().element(id).map(|e| &e.kind), Some(cc_core::ElementKind::Text(_))).then_some(id)
}

/// Families of small buttons, each starting its own column.
/// A captioned group of small buttons in columns (a contextual ribbon's).
pub fn env_stacks(ui: &mut Ui, state: &mut AppState, caption: String, families: &[&[UiCommand]]) {
    group(ui, caption, |ui| stacks(ui, state, families));
}

fn stacks(ui: &mut Ui, state: &mut AppState, families: &[&[UiCommand]]) {
    for family in families {
        if let Some(command) = stacked(ui, state, family) {
            commands::run(state, &ui.ctx().clone(), command);
        }
    }
}

/// Families of medium buttons (icon and label), each starting its own
/// column: for secondary commands whose icons need their names.
fn labelled(ui: &mut Ui, state: &mut AppState, families: &[&[UiCommand]]) {
    for family in families {
        if let Some(command) = stacked_labelled(ui, state, family) {
            commands::run(state, &ui.ctx().clone(), command);
        }
    }
}

fn modify_tab(ui: &mut Ui, state: &mut AppState) {
    use UiCommand::*;
    if sketching(state) {
        crate::sketch_mode::groups(ui, state, true);
        return;
    }
    group(ui, t!("group-selection"), |ui| command_buttons(ui, state, &[Select, SelectInstances]));
    group(ui, t!("group-clipboard"), |ui| clipboard_group(ui, state));
    group(ui, t!("group-modify"), |ui| {
        let transforms: &[UiCommand] = &[Move, CopyElements, Rotate, MirrorPick, MirrorDraw, Array];
        // Geometry edits pick their elements, no selection needed.
        let picks: &[UiCommand] = &[Align, Offset, TrimCorner, TrimExtend, Split, ToggleJoin];
        stacks(ui, state, &[transforms, picks, &[Pin, Unpin, Delete]]);
    });
    group(ui, t!("group-measure"), |ui| command_button(ui, state, Measure));
    group(ui, t!("group-geometry"), |ui| {
        command_buttons(ui, state, &[JoinGeometry, UnjoinGeometry, Paint, RemovePaint])
    });
    group(ui, t!("group-group"), |ui| command_buttons(ui, state, &[Group, Ungroup, Explode]));
    group(ui, t!("group-phasing"), |ui| command_button(ui, state, Demolish));
    // The active tool's drawing modes, highlighted.
    if matches!(state.tool, Tool::Wall { .. }) {
        let draw: &[&[UiCommand]] = &[
            &[SketchLine, SketchRect, SketchPolygon],
            &[SketchPolygonOut, SketchCircle, SketchEllipse],
            &[SketchArc3, SketchArcCenter, SketchArcTangent],
            &[WallFillet, SketchSpline, WallPickLines],
        ];
        group_with(ui, t!("group-draw"), true, None, |ui| stacks(ui, state, draw));
    }
    // The Detail Line tool's shapes, and the style of the lines it draws.
    if state.tool == Tool::DetailLine {
        let draw: &[&[UiCommand]] = &[
            &[SketchLine, SketchRect, SketchPolygon],
            &[SketchPolygonOut, SketchCircle, SketchEllipse],
            &[SketchArc3, SketchArcCenter, SketchArcTangent],
            &[DetailFillet, SketchSpline],
        ];
        group_with(ui, t!("group-draw"), true, None, |ui| stacks(ui, state, draw));
        let styles = launcher(state, LineStyles);
        group_with(ui, t!("label-line-style"), true, styles, |ui| {
            ui.vertical(|ui| {
                ui.add_space((BUTTON_HEIGHT - ui.spacing().interact_size.y).max(0.0) / 2.0);
                crate::detail_ui::style_combo(ui, state);
            });
        });
    }
    // What edits only the selected categories (and the active sheet), highlighted.
    for (caption, list) in commands::contextual(state) {
        group_with(ui, caption, true, None, |ui| command_buttons(ui, state, &list));
    }
}

/// Whether `tool` has its own *Modify | Place …* tab, with its drawing
/// modes, that the ribbon opens while it is active.
pub fn places(tool: &Tool) -> bool {
    matches!(tool, Tool::Wall { .. } | Tool::DetailLine)
}

/// The Modify tab's title: "Modify | Place Wall" while such a tool is
/// active (see [`places`]), "Modify | Walls" while one category is
/// selected, "Modify | Multiple" for several, plain "Modify" with nothing
/// selected.
fn modify_title(state: &AppState) -> String {
    let modify = t!("ribbon-modify");
    if sketching(state) {
        return format!("{modify} | {}", crate::sketch_mode::title(state));
    }
    if places(&state.tool) {
        let tool = crate::i18n::tr(state.tool.key());
        return format!("{modify} | {}", t!("ribbon-place-tool", "tool" => tool));
    }
    let project = state.project();
    let mut categories = state.selection.iter().take(2000).filter_map(|id| project.element(*id)).map(|e| e.category());
    let Some(first) = categories.next() else { return modify };
    let what = if categories.all(|c| c == first) { actions::t_category(first) } else { t!("ribbon-modify-several") };
    format!("{modify} | {what}")
}

/// Paste (a menu of where to paste), then cut, copy, match type and create similar.
fn clipboard_group(ui: &mut Ui, state: &mut AppState) {
    let r = command_menu(ui, state, UiCommand::Paste);
    egui::Popup::menu(&r).show(|ui| {
        if ui.button(format!("{} (Ctrl+V)", t!("paste-click"))).clicked() {
            crate::clipboard::paste(state);
        }
        if ui.button(t!("paste-same-place")).clicked() {
            crate::clipboard::paste_aligned(state, false);
        }
        let plan = state.active_level().is_some();
        if ui.add_enabled(plan, Button::new(t!("paste-current-level"))).clicked() {
            crate::clipboard::paste_aligned(state, true);
        }
    });
    stacks(ui, state, &[&[UiCommand::Cut, UiCommand::ClipboardCopy]]);
    labelled(ui, state, &[&[UiCommand::MatchType, UiCommand::CreateSimilar]]);
}

fn view_tab(ui: &mut Ui, state: &mut AppState) {
    let levels: Vec<_> = state.project().levels.iter().map(|l| (l.id, l.name.clone())).collect();
    group(ui, t!("group-create-views"), |ui| {
        for command in [UiCommand::FloorPlan, UiCommand::CeilingPlan] {
            let r = command_menu(ui, state, command);
            egui::Popup::menu(&r).show(|ui| {
                for (id, name) in &levels {
                    if ui.button(name).clicked() {
                        actions::new_plan(state, *id, command == UiCommand::CeilingPlan);
                    }
                }
            });
        }
        command_buttons(ui, state, &[UiCommand::New3D, UiCommand::Section, UiCommand::Callout]);
        let r = command_menu(ui, state, UiCommand::Elevations);
        egui::Popup::menu(&r).show(|ui| {
            let elevations: Vec<_> =
                state.project().views_of(ViewCategory::Elevation).map(|v| (v.id, v.name.clone())).collect();
            for (id, name) in elevations {
                if ui.button(name).clicked() {
                    state.open_view(id);
                }
            }
        });
    });
    group(ui, t!("group-documentation"), |ui| {
        command_buttons(ui, state, &[UiCommand::Legend, UiCommand::DraftingView]);
        let r = command_menu(ui, state, UiCommand::Schedule);
        egui::Popup::menu(&r).show(|ui| {
            for c in Category::SCHEDULABLE {
                if ui.button(actions::t_category(c)).clicked() {
                    actions::new_schedule(state, c);
                }
            }
        });
        command_button(ui, state, UiCommand::Sheet);
    });
    group(ui, t!("group-graphics"), |ui| {
        command_buttons(
            ui,
            state,
            &[
                UiCommand::VisibilityGraphics,
                UiCommand::ViewFilters,
                UiCommand::ViewTemplates,
                UiCommand::SunSettings,
                UiCommand::Render,
            ],
        );
    });
    // Hiding and isolating: eight commands with long names, so a menu.
    let label = t!("group-temporary");
    group(ui, label.clone(), |ui| {
        use UiCommand::*;
        let commands = [
            TempHide,
            TempIsolate,
            TempHideCategory,
            TempIsolateCategory,
            TempReset,
            TempPermanent,
            HideInView,
            OverrideGraphics,
        ];
        menu_of(ui, state, &label, &commands);
    });
    // Zoom to Fit is in each view's bar (and ZE).
    group(ui, t!("group-windows"), |ui| command_button(ui, state, UiCommand::CloseInactiveViews));
}

fn manage_tab(ui: &mut Ui, state: &mut AppState) {
    use UiCommand::*;
    group(ui, t!("group-project"), |ui| command_buttons(ui, state, &[ProjectInfo, Units, Levels]));
    // The project's definitions; line styles, tag types and the pipe catalog also open from launchers.
    group(ui, t!("group-standards"), |ui| {
        command_buttons(ui, state, &[Materials, FillPatterns, LineStyles, TagTypes, Profiles, PipeCatalog])
    });
    // View templates and filters live in View, the site's location in Massing & Site.
    group(ui, t!("group-browser"), |ui| command_button(ui, state, BrowserOrganization));
    group(ui, t!("group-phasing"), |ui| command_button(ui, state, Phases));
    group(ui, t!("group-design-options"), |ui| {
        command_button(ui, state, DesignOptions);
        active_option_menu(ui, state);
    });
    group(ui, t!("group-macros"), |ui| {
        command_button(ui, state, MacroRecord);
        macro_menu(ui, state);
        command_button(ui, state, MacroEditor);
        command_button(ui, state, VisualProgramming);
    });
    group(ui, t!("group-plugins"), |ui| {
        command_button(ui, state, Plugins);
        plugin_menu(ui, state);
        command_button(ui, state, PluginConsole);
    });
}

/// Plugin Commands: a menu of the loaded plugins' commands, by plugin;
/// each opens its form (see [`crate::plugin_form`]).
fn plugin_menu(ui: &mut Ui, state: &mut AppState) {
    let r = command_menu(ui, state, UiCommand::PluginCommands);
    egui::Popup::menu(&r).show(|ui| {
        let groups: Vec<(String, Vec<(String, String)>)> =
            state.plugins.commands().into_iter().map(|(name, list)| (name.to_string(), list.to_vec())).collect();
        if groups.is_empty() {
            ui.label(theme::muted(t!("plug-menu-none")).size(12.0));
        }
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (plugin, commands) in groups {
                ui.label(theme::muted(plugin).size(11.5));
                for (name, description) in commands {
                    let title = match state.plugins.command_title(&name) {
                        Some(title) => title.to_string(),
                        None => name.split_once("__").map_or(name.as_str(), |(_, c)| c).to_string(),
                    };
                    let mut r = ui.button(title);
                    if !description.is_empty() {
                        r = r.on_hover_text(description);
                    }
                    if r.clicked() {
                        state.ribbon_peek = false;
                        crate::plugin_form::open(state, &name);
                    }
                }
            }
        });
        ui.separator();
        if ui.button(t!("action-plugins")).clicked() {
            state.ribbon_peek = false;
            crate::plugins_ui::open(state);
        }
    });
}

/// The Plugins tab: the loaded plugins' buttons, one group per plugin and
/// panel (in load order, then the manifest's), drawn as the built-in ones;
/// each opens its command's form.
fn plugins_tab(ui: &mut Ui, state: &mut AppState) {
    let buttons = state.plugins.buttons.clone();
    let mut groups: Vec<(&str, &str, Vec<&RibbonButton>)> = Vec::new();
    for b in &buttons {
        match groups.iter_mut().find(|(id, panel, _)| *id == b.plugin_id && *panel == b.button.panel) {
            Some((_, _, list)) => list.push(b),
            None => groups.push((&b.plugin_id, &b.button.panel, vec![b])),
        }
    }
    for (_, panel, list) in groups {
        group(ui, panel.to_string(), |ui| {
            // Consecutive small buttons stack in columns, as built-in families do.
            let mut i = 0;
            while let Some(b) = list.get(i) {
                if b.button.size == ButtonSize::Large {
                    plugin_big(ui, state, b);
                    i += 1;
                    continue;
                }
                let rest = list.get(i..).unwrap_or_default();
                let run = rest.iter().take_while(|b| b.button.size == ButtonSize::Small).count();
                stack_columns(ui, rest.get(..run).unwrap_or_default(), |ui, b| plugin_small(ui, state, b));
                i += run;
            }
        });
    }
}

fn plugin_big(ui: &mut Ui, state: &mut AppState, b: &RibbonButton) {
    let why = crate::plugins_ui::unavailable(state, &b.button.when);
    let glyph = Glyph::File(b.button.icon.as_deref());
    let r = big_button(ui, glyph, &b.button.label, false, why.is_none(), false);
    if plugin_tooltip(r, state, b, why).clicked() {
        run_plugin_button(state, b);
    }
}

fn plugin_small(ui: &mut Ui, state: &mut AppState, b: &RibbonButton) {
    let why = crate::plugins_ui::unavailable(state, &b.button.when);
    let enabled = why.is_none();
    let r = small_button(ui, Glyph::File(b.button.icon.as_deref()), &b.button.label, enabled, false);
    if plugin_tooltip(r, state, b, why).clicked() && enabled {
        run_plugin_button(state, b);
    }
}

fn run_plugin_button(state: &mut AppState, b: &RibbonButton) {
    state.ribbon_peek = false;
    crate::plugin_form::open(state, &b.button.command);
}

/// A plugin button's tooltip, as a built-in command's: its label, what it
/// does, the plugin it comes from, and why it is unavailable.
fn plugin_tooltip(response: Response, state: &AppState, b: &RibbonButton, why: Option<String>) -> Response {
    let detailed = state.prefs.detailed_tooltips;
    response.on_hover_ui(|ui| {
        ui.set_max_width(300.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(RichText::new(&b.button.label).family(theme::semibold()).color(PALETTE.text));
        // A command without a description is described by its title, often the label.
        if detailed && !b.description.is_empty() && b.description != b.button.label {
            ui.label(theme::muted(&b.description));
        }
        ui.label(theme::muted(t!("tip-plugin", "name" => b.plugin.clone())).size(11.0));
        if let Some(why) = &why {
            ui.label(RichText::new(t!("tip-unavailable", "reason" => why)).color(PALETTE.warning));
        }
    })
}

/// The loaded plugins' exporters (`export`) or importers after the built-in
/// entries of an Export or Import group: a separator, then one button each,
/// its tooltip naming the plugin. A click asks for the file first (see
/// [`crate::plugins_ui::run_file_entry`]).
pub(crate) fn plugin_file_entries(ui: &mut Ui, state: &mut AppState, export: bool) {
    let entries = if export { state.plugins.exporters.clone() } else { state.plugins.importers.clone() };
    if entries.is_empty() {
        return;
    }
    ui.separator();
    let badge = if export { Badge::Export } else { Badge::Import };
    for entry in &entries {
        let r = big_button(ui, Glyph::Icon(Icon::Plugin, Some(badge)), &entry.action.label, false, true, false);
        let r = r.on_hover_ui(|ui| {
            ui.set_max_width(300.0);
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new(&entry.action.label).family(theme::semibold()).color(PALETTE.text));
            ui.label(theme::muted(t!("tip-plugin", "name" => entry.plugin.clone())).size(11.0));
        });
        if r.clicked() {
            state.ribbon_peek = false;
            crate::plugins_ui::run_file_entry(state, entry, export);
        }
    }
}

/// Run Macro: a menu of the macros in the macros folder (see Preferences)
/// and Browse… for any other file.
/// The option being edited: the main model or one option, from a menu.
fn active_option_menu(ui: &mut Ui, state: &mut AppState) {
    let r = command_menu(ui, state, UiCommand::ActiveOption);
    egui::Popup::menu(&r).show(|ui| crate::design_options_ui::active_menu(ui, state));
}

fn macro_menu(ui: &mut Ui, state: &mut AppState) {
    let r = command_menu(ui, state, UiCommand::MacroRun);
    egui::Popup::menu(&r).show(|ui| {
        match state.prefs.macro_folder.clone() {
            None => {
                ui.label(theme::muted(t!("macro-no-folder")).size(12.0));
            }
            Some(folder) => {
                let files = crate::preferences::macro_files(&folder);
                if files.is_empty() {
                    ui.label(theme::muted(t!("macro-none")).size(12.0));
                }
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for path in files {
                        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                            state.ribbon_peek = false;
                            actions::run_macro_file(state, &path);
                        }
                    }
                });
            }
        }
        ui.separator();
        if ui.button(t!("pref-browse")).clicked() {
            state.ribbon_peek = false;
            actions::run_macro(state);
        }
    });
}

/// Context options for the active tool (options bar).
/// Options of the pipe, conduit, cable tray and lightning conductor tools:
/// the system (or circuit), the elevation of the run's start and, for
/// pipes, the fall along it. Changing the system brings back its defaults.
fn segment_options(ui: &mut Ui, state: &mut AppState, type_id: cc_core::ElementTypeId, run: Run) {
    use cc_core::SystemClass as C;
    let pipe = run == Run::Pipe;
    let (mut system, classes, label): (_, &[C], _) = match run {
        Run::Pipe => (state.mep.system, &[C::Plumbing], t!("col-system")),
        Run::Conduit | Run::Tray => (state.mep.circuit, &[C::Circuit, C::Data], t!("prop-circuit")),
        Run::Conductor => (state.mep.lightning, &[C::Lightning], t!("col-system")),
    };
    ui.label(label);
    if crate::systems_dialog::system_combo(ui, state, ("segment-system", run as u8), classes, &mut system) {
        match run {
            Run::Pipe => state.mep.system = system,
            Run::Conduit | Run::Tray => state.mep.circuit = system,
            Run::Conductor => state.mep.lightning = system,
        }
        state.mep.offset = None;
        state.mep.slope = None;
    }
    let project = state.project();
    let kind = system.and_then(|id| project.system(id)).map(|s| s.kind);
    let default_offset = crate::tools::default_run_offset(project, run, system, state.active_level());
    let diameter = project.element_type(type_id).and_then(|t| t.diameter()).unwrap_or(0.1);
    let default_slope = if kind.is_some_and(cc_core::SystemKind::is_drainage) {
        cc_engine::drain_slope(project, diameter) * 100.0
    } else {
        0.0
    };
    ui.label(t!("prop-elevation-offset"));
    let mut offset = state.mep.offset.unwrap_or(default_offset);
    let before = offset;
    if ui.add(crate::properties::length_drag(&mut offset).speed(0.01)).changed() {
        state.mep.offset = Some(offset);
        // While drawing, the chain goes up or down to it at its last point,
        // also at its first point (a riser from the elevation shown before);
        // conductors without a chain lie on the roof, so only a chain rises.
        if state.interaction.anchor.is_some() && state.mep.chain.is_none() && run != Run::Conductor {
            state.mep.chain = Some(before);
        }
        if state.interaction.anchor.is_some() && state.mep.chain.is_some() {
            state.mep.rise = Some(offset);
        } else {
            state.mep.chain = None;
        }
    }
    if pipe {
        ui.label(t!("col-slope"));
        // Kept in percent; shown and typed in the project's slope unit.
        let mut slope = state.mep.slope.unwrap_or(default_slope) / 100.0;
        if ui.add(crate::properties::slope_drag(&mut slope).range(-0.2..=0.2)).changed() {
            state.mep.slope = Some(slope * 100.0);
        }
    }
    ui.separator();
    let hint = match run {
        Run::Pipe => t!("hint-pipe"),
        Run::Conduit => t!("hint-conduit"),
        Run::Tray => t!("hint-cable-tray"),
        Run::Conductor => t!("hint-lightning-conductor"),
    };
    ui.label(theme::muted(hint));
}

/// Options of the device tool after the rotation: the circuit of an
/// electrical device, the cabling (rack) of a data outlet or the lightning
/// protection system of an air terminal or earth electrode, with a hint.
fn device_options(ui: &mut Ui, state: &mut AppState, type_id: cc_core::ElementTypeId) {
    use cc_core::{DeviceKind as D, SystemClass as C};
    let Some(device) = crate::tools::device_type(state.project(), type_id) else { return };
    let class = device.class();
    if device != D::Rack {
        let (label, mut system) = match class {
            C::Circuit => (t!("prop-circuit"), state.mep.circuit),
            C::Data => (t!("device-rack"), state.mep.data),
            C::Lightning | C::Plumbing => (t!("col-system"), state.mep.lightning),
        };
        ui.label(label);
        if crate::systems_dialog::system_combo(ui, state, ("device-system", type_id.0), &[class], &mut system) {
            match class {
                C::Circuit => state.mep.circuit = system,
                C::Data => state.mep.data = system,
                C::Lightning | C::Plumbing => state.mep.lightning = system,
            }
        }
    }
    let hint = match device {
        D::DataOutlet => t!("hint-data-outlet"),
        D::Rack => t!("hint-rack"),
        D::AirTerminal => t!("hint-air-terminal"),
        D::EarthElectrode => t!("hint-earth-electrode"),
        _ => t!("hint-device"),
    };
    ui.label(theme::muted(hint));
}

/// The sill of the windows or niches the hosted tool places (issue #304):
/// the type's until a value is typed or dragged here, which then holds for
/// this tool and type (see [`crate::tools::hosted_sill`]).
fn hosted_sill_field(ui: &mut Ui, state: &mut AppState, category: Category, type_id: cc_core::ElementTypeId) {
    let Some(ty) = state.project().element_type(type_id) else { return };
    let mut sill = crate::tools::hosted_sill(state.hosted_sill, category, ty);
    let label = t!("prop-sill");
    let caption = ui.label(&label);
    let response = ui
        .add(crate::properties::length_drag(&mut sill).speed(0.01).range(crate::tools::SILL_RANGE))
        .labelled_by(caption.id);
    // Named for the accessibility tree (and the interface tests), keeping the number.
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::drag_value(true, sill);
        info.label = Some(label.clone());
        info
    });
    if response.changed() && sill.is_finite() {
        state.hosted_sill = Some(sill.clamp(*crate::tools::SILL_RANGE.start(), *crate::tools::SILL_RANGE.end()));
    }
}

pub fn options_bar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(PALETTE.accent_soft)
            .corner_radius(theme::RADIUS)
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (r, _) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                icons::paint(ui.painter(), r, tool_icon(&state.tool), PALETTE.accent_text);
                let name = crate::i18n::tr(state.tool.name_key(state.project()));
                ui.label(RichText::new(name).family(theme::semibold()).color(PALETTE.accent_text));
            });
        ui.separator();
        if let (Some(category), Some(type_id)) = (state.tool.category(), state.tool.type_id()) {
            let types = crate::tools::tool_types(state.project(), category, type_id, state.tool.material());
            let mut selected = type_id;
            // Walls: a project without a curtain type gets one from the list.
            let offer_curtain = category == Category::Wall && !crate::curtain_ui::has_curtain_type(state);
            let mut new_curtain = false;
            let mut open_catalog = false;
            egui::ComboBox::from_id_salt("tool_type")
                .selected_text(state.project().type_name(type_id).to_string())
                .width(200.0)
                .show_ui(ui, |ui| {
                    if category == Category::Pipe {
                        // Pipes: the sizes of each series under its name, then the free types.
                        for (series, list) in crate::pipe_catalog_dialog::grouped_types(state.project(), types) {
                            ui.label(theme::muted(series.unwrap_or_else(|| t!("pipe-catalog-free-types"))));
                            for (id, name) in list {
                                ui.selectable_value(&mut selected, id, name);
                            }
                        }
                        ui.separator();
                        open_catalog = ui.selectable_label(false, t!("pipe-catalog-load-more")).clicked();
                    } else {
                        for (id, name) in types {
                            ui.selectable_value(&mut selected, id, name);
                        }
                    }
                    if offer_curtain {
                        ui.separator();
                        new_curtain = ui.selectable_label(false, crate::curtain_ui::new_type_label()).clicked();
                    }
                });
            if new_curtain && let Some(id) = crate::curtain_ui::new_curtain_type(state) {
                selected = id;
            }
            if open_catalog {
                crate::pipe_catalog_dialog::open(state);
            }
            if selected != type_id {
                state.tool = state.tool.with_type(selected);
                // Another type: its own sill again.
                state.hosted_sill = None;
            }
            if ui.button(t!("props-edit-type")).clicked() {
                state.dialogs.type_editor = Some(type_id);
            }
            ui.separator();
        }
        match state.tool.clone() {
            Tool::Select => {
                ui.label(theme::muted(t!("hint-select")));
            }
            Tool::Wall { .. } => {
                // The drawing mode is picked in the ribbon's Draw group; its options come first.
                use crate::tools::WallMode;
                crate::wall_draw_ui::options(ui, state);
                ui.label(t!("prop-top-constraint"));
                let levels: Vec<_> = state.project().levels.iter().map(|l| (l.id, l.name.clone())).collect();
                let current = match state.wall_top_level.and_then(|id| levels.iter().find(|l| l.0 == id)) {
                    Some((_, name)) => t!("top-up-to", "level" => name),
                    None => t!("top-unconnected"),
                };
                egui::ComboBox::from_id_salt("wall_top").selected_text(current).width(170.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.wall_top_level, None, t!("top-unconnected"));
                    for (id, name) in &levels {
                        ui.selectable_value(&mut state.wall_top_level, Some(*id), t!("top-up-to", "level" => name));
                    }
                });
                if state.wall_top_level.is_none() {
                    ui.add(crate::properties::length_drag(&mut state.wall_height).speed(0.01).range(0.1..=50.0));
                }
                ui.separator();
                ui.label(t!("prop-location-line"));
                let location = &mut state.wall_location_line;
                egui::ComboBox::from_id_salt("wall_location")
                    .selected_text(crate::i18n::tr(location.key()))
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for l in cc_core::LocationLine::ALL {
                            ui.selectable_value(location, l, crate::i18n::tr(l.key()));
                        }
                    });
                ui.separator();
                let hint = match state.wall_mode {
                    WallMode::Line => t!("hint-wall"),
                    WallMode::Arc => t!("hint-wall-arc"),
                    WallMode::Spline if state.interaction.points.is_empty() => t!("hint-wall-spline"),
                    WallMode::Spline => t!("hint-wall-spline-next"),
                    _ => crate::wall_draw_ui::hint(state),
                };
                ui.label(theme::muted(hint));
            }
            Tool::Hosted { category, type_id } => {
                if crate::tools::takes_sill(category) {
                    hosted_sill_field(ui, state, category, type_id);
                    ui.separator();
                }
                ui.checkbox(&mut state.interaction.flip_hand, t!("prop-flip-hand"));
                ui.label(theme::muted(t!("hint-hosted")));
                ui.label(theme::muted(t!("hint-hosted-distance")));
            }
            Tool::Sketch { .. } => crate::sketch_mode::options(ui, state),
            Tool::Section => {
                ui.label(theme::muted(t!("msg-section-hint")));
            }
            Tool::Callout => crate::callout_ui::options(ui, state),
            Tool::Opening { host, floor } => {
                let n = state.interaction.points.len();
                if ui.add_enabled(n >= 3, Button::new(t!("sketch-finish"))).clicked() {
                    let pts = state.interaction.points.clone();
                    crate::tools::add_opening(state, host, pts);
                }
                if ui.button(t!("action-cancel")).clicked() {
                    state.set_tool(Tool::Select);
                }
                ui.label(theme::muted(if floor { t!("hint-floor-opening") } else { t!("hint-roof-opening") }));
            }
            Tool::Room { .. } => {
                ui.label(theme::muted(t!("hint-room")));
            }
            Tool::Grid { .. } => {
                ui.label(theme::muted(t!("hint-grid")));
            }
            Tool::Dimension { mode, .. } => {
                if matches!(mode, DimensionMode::Aligned | DimensionMode::Linear) {
                    let c = &mut state.dim_continuous;
                    ui.selectable_value(c, false, t!("dim-single")).on_hover_text(t!("dim-single-tip"));
                    ui.selectable_value(c, true, t!("dim-continuous")).on_hover_text(t!("dim-continuous-tip"));
                    ui.separator();
                }
                let hint = if state.interaction.dim_chain.is_some_and(|c| c.ordinate) {
                    t!("hint-dim-ordinate-next")
                } else if state.interaction.dim_chain.is_some() {
                    t!("hint-dim-chain")
                } else {
                    crate::i18n::tr(mode.hint_key())
                };
                let r = ui.label(theme::muted(hint));
                if matches!(mode, DimensionMode::Aligned | DimensionMode::Linear | DimensionMode::Ordinate) {
                    r.on_hover_text(t!("hint-dim-align"));
                }
            }
            Tool::Text { .. } => crate::text_tool::options(ui, state, false),
            Tool::LevelMark { .. } => {
                ui.label(theme::muted(t!("hint-level-mark")));
            }
            Tool::AddLeader { .. } => crate::text_tool::options(ui, state, true),
            Tool::DetailLine => crate::detail_ui::options(ui, state),
            Tool::DetailComponent { .. } => crate::detail_component_ui::options(ui, state),
            Tool::Column { .. } => {
                ui.label(theme::muted(t!("hint-column")));
            }
            Tool::Beam { .. } => {
                // Straight, or arcs through a third point, as walls.
                let mut arc = state.beam_arc;
                ui.selectable_value(&mut arc, false, t!("wall-mode-line")).on_hover_text(t!("hint-beam"));
                ui.selectable_value(&mut arc, true, t!("wall-mode-arc")).on_hover_text(t!("hint-wall-arc"));
                if arc != state.beam_arc {
                    state.beam_arc = arc;
                    state.interaction.points.clear();
                }
                ui.separator();
                ui.label(theme::muted(if state.beam_arc { t!("hint-wall-arc") } else { t!("hint-beam") }));
            }
            Tool::Footing { type_id } if crate::tools::is_strip(state.project(), type_id) => {
                // Strip footings: straight, or arcs through a third point, as walls.
                let mut arc = state.strip_arc;
                ui.selectable_value(&mut arc, false, t!("wall-mode-line")).on_hover_text(t!("hint-wall"));
                ui.selectable_value(&mut arc, true, t!("wall-mode-arc")).on_hover_text(t!("hint-wall-arc"));
                if arc != state.strip_arc {
                    state.strip_arc = arc;
                    state.interaction.points.clear();
                }
                ui.separator();
                ui.label(theme::muted(if state.strip_arc { t!("hint-wall-arc") } else { t!("hint-wall") }));
            }
            Tool::Footing { .. } => {
                ui.label(theme::muted(t!("hint-footing")));
            }
            Tool::Truss { .. } => crate::truss_ui::tool_options(ui, state),
            Tool::Bracing { .. } => crate::bracing_ui::tool_options(ui, state),
            Tool::Stair { .. } => crate::stair_ui::tool_options(ui, state),
            Tool::Ramp { .. } => crate::stair_ui::ramp_tool_options(ui, state),
            Tool::Pipe { type_id } => segment_options(ui, state, type_id, Run::Pipe),
            Tool::Conduit { type_id } => segment_options(ui, state, type_id, Run::Conduit),
            Tool::CableTray { type_id } => segment_options(ui, state, type_id, Run::Tray),
            Tool::LightningConductor { type_id } => segment_options(ui, state, type_id, Run::Conductor),
            Tool::Valve { .. } => {
                ui.label(theme::muted(t!("hint-valve")));
            }
            Tool::Component { type_id } if crate::tools::device_type(state.project(), type_id).is_some() => {
                ui.label(t!("prop-rotation"));
                let rotation = &mut state.interaction.rotation;
                if ui.add(DragValue::new(rotation).speed(1.0).suffix("°")).changed() {
                    *rotation = rotation.rem_euclid(360.0);
                }
                device_options(ui, state, type_id);
            }
            Tool::Component { .. } => {
                ui.label(t!("prop-rotation"));
                let rotation = &mut state.interaction.rotation;
                if ui.add(DragValue::new(rotation).speed(1.0).suffix("°")).changed() {
                    *rotation = rotation.rem_euclid(360.0);
                }
                ui.label(theme::muted(t!("hint-component")));
            }
            Tool::Move => {
                ui.label(theme::muted(t!("hint-base-point")));
            }
            Tool::Copy => {
                ui.checkbox(&mut state.modify.multiple, t!("opt-multiple"));
                ui.label(theme::muted(t!("hint-base-point")));
            }
            Tool::Array => {
                ui.label(t!("opt-count"));
                ui.add(DragValue::new(&mut state.modify.count).range(2..=200));
                ui.label(theme::muted(t!("hint-array")));
            }
            Tool::Rotate => {
                ui.checkbox(&mut state.modify.rotate_copy, t!("opt-copy"));
                ui.separator();
                ui.label(t!("prop-rotation"));
                ui.add(DragValue::new(&mut state.modify.angle).speed(1.0).suffix("°"));
                if ui.button(t!("action-apply")).clicked()
                    && let Some(center) = state.interaction.center
                {
                    let targets = state.interaction.targets.iter().copied().collect();
                    crate::modify::apply_rotation(state, &targets, center, state.modify.angle.to_radians());
                }
                if ui.button(t!("opt-set-center")).clicked() {
                    state.interaction.placing_center = true;
                }
                ui.label(theme::muted(t!("hint-rotate")));
            }
            Tool::Offset => {
                ui.label(t!("opt-distance"));
                ui.add(crate::properties::length_drag(&mut state.modify.offset).range(0.001..=100.0));
                ui.label(theme::muted(t!("hint-offset")));
            }
            Tool::TrimCorner | Tool::TrimExtend | Tool::Split | Tool::Align | Tool::ToggleJoin => {
                let key = match state.tool {
                    Tool::TrimCorner => "hint-trim-corner",
                    Tool::TrimExtend => "hint-trim-extend",
                    Tool::Split => "hint-split",
                    Tool::ToggleJoin => "hint-toggle-join",
                    _ => "hint-align",
                };
                ui.label(theme::muted(crate::i18n::tr(key)));
            }
            Tool::Measure => {
                ui.label(theme::muted(t!("hint-measure")));
            }
            Tool::PushPull => {
                let key = if state.interaction.push.is_some() { "hint-push-pull-drag" } else { "hint-push-pull" };
                ui.label(theme::muted(crate::i18n::tr(key)));
            }
            Tool::JoinGeometry { unjoin } => {
                let key = match (state.interaction.targets.is_empty(), unjoin) {
                    (true, _) => "hint-join-pick",
                    (false, false) => "hint-join-second",
                    (false, true) => "hint-unjoin-second",
                };
                ui.label(theme::muted(crate::i18n::tr(key)));
            }
            Tool::AttachTop => {
                ui.label(theme::muted(t!("hint-attach-top")));
            }
            Tool::JoinRoof => {
                ui.label(theme::muted(crate::i18n::tr(crate::modify::roof_join_hint(state))));
            }
            Tool::Paste => {
                ui.label(theme::muted(t!("hint-paste")));
            }
            Tool::MatchType => {
                ui.label(theme::muted(t!("hint-match-type")));
            }
            Tool::Demolish => {
                ui.label(theme::muted(t!("hint-demolish")));
            }
            Tool::CurtainGrid => crate::curtain_ui::options_bar(ui, state),
            Tool::Paint { remove } => crate::paint_ui::options_bar(ui, state, remove),
            Tool::Pick { .. } => {
                ui.label(theme::muted(crate::plugin_form::prompt(state)));
            }
            Tool::Mirror { draw } => {
                ui.checkbox(&mut state.modify.mirror_copy, t!("opt-copy"));
                ui.label(theme::muted(if draw { t!("hint-mirror-draw") } else { t!("hint-mirror-pick") }));
            }
            Tool::Sweep { type_id } => {
                let o = &mut state.sweep;
                ui.label(t!("prop-base-offset"));
                ui.add(crate::properties::length_drag(&mut o.base_offset).range(-100.0..=100.0));
                ui.checkbox(&mut o.closed, t!("opt-sweep-closed"));
                ui.checkbox(&mut o.flip, t!("opt-sweep-flip"));
                let n = state.interaction.points.len();
                let needed = if state.sweep.closed { 3 } else { 2 };
                if ui.add_enabled(n >= needed, Button::new(t!("sketch-finish"))).clicked()
                    && let Some(level) = state.active_level()
                {
                    crate::tools::finish_sweep(state, level, type_id);
                }
                if ui.add_enabled(n > 0, Button::new(t!("action-cancel"))).clicked() {
                    state.interaction.points.clear();
                }
                ui.label(theme::muted(t!("hint-sweep")));
            }
            Tool::Railing { type_id } => {
                ui.checkbox(&mut state.railing_host.on, t!("opt-railing-on-host"));
                if state.railing_host.on {
                    state.interaction.points.clear();
                    ui.checkbox(&mut state.railing_host.both, t!("opt-railing-both-sides"));
                    ui.label(theme::muted(t!("hint-railing-host")));
                    return;
                }
                let n = state.interaction.points.len();
                if ui.add_enabled(n >= 2, Button::new(t!("sketch-finish"))).clicked()
                    && let Some(level) = state.active_level()
                {
                    crate::tools::finish_railing(state, level, type_id);
                }
                if ui.add_enabled(n > 0, Button::new(t!("action-cancel"))).clicked() {
                    state.interaction.points.clear();
                }
                ui.label(theme::muted(t!("hint-railing")));
            }
        }
    });
}

/// Temp-data key of the selection the filter was opened on.
const FILTER_BASE: &str = "selection-filter-base";

/// The selection's filter: the categories of the selection it was opened
/// on, each with its count, checked while its elements are selected;
/// unchecking one deselects them, checking it selects them again, and
/// *Only* keeps that category alone.
fn selection_filter(ui: &mut Ui, state: &mut AppState) {
    let base: std::collections::BTreeSet<cc_core::ElementId> =
        ui.data(|d| d.get_temp(egui::Id::new(FILTER_BASE))).unwrap_or_else(|| state.selection.clone());
    let mut by_category: std::collections::BTreeMap<Category, Vec<cc_core::ElementId>> = Default::default();
    for id in base.iter().take(100_000) {
        if let Some(e) = state.project().element(*id) {
            by_category.entry(e.category()).or_default().push(*id);
        }
    }
    ui.set_min_width(240.0);
    ui.label(RichText::new(t!("selection-filter-title")).family(theme::semibold()).color(PALETTE.text));
    let mut change = None;
    for (category, ids) in &by_category {
        let mut on = ids.iter().any(|id| state.selection.contains(id));
        ui.horizontal(|ui| {
            let label = format!("{} ({})", actions::t_category(*category), ids.len());
            if ui.checkbox(&mut on, label).changed() {
                change = Some((ids.clone(), on, false));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button(t!("selection-filter-only")).clicked() {
                    change = Some((ids.clone(), true, true));
                }
            });
        });
    }
    match change {
        Some((ids, _, true)) => state.selection = ids.into_iter().collect(),
        Some((ids, true, false)) => state.selection.extend(ids),
        Some((ids, false, false)) => {
            for id in ids {
                state.selection.remove(&id);
            }
        }
        None => {}
    }
}

pub fn status_bar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if let Some(c) = state.cursor {
            let coords = format!("X {:>9}  Y {:>9}", format::meters(c.x), format::meters(c.y));
            ui.label(RichText::new(coords).monospace().color(PALETTE.text_muted));
            ui.separator();
        }
        if !state.typed.length.is_empty() {
            ui.label(RichText::new(t!("status-length", "value" => state.typed.length)).strong());
            ui.separator();
        }
        if !state.typed.angle.is_empty() || state.typed.angle_active {
            ui.label(RichText::new(t!("status-angle", "value" => state.typed.angle)).strong());
            ui.separator();
        }
        let n = state.selection.len();
        if n > 0 {
            // The count opens the selection's filter by category.
            let r = ui.add(Button::new(t!("status-selected", "count" => n)).frame(false));
            let r = r.on_hover_text(t!("selection-filter-hint"));
            if r.clicked() {
                let base = state.selection.clone();
                ui.data_mut(|d| d.insert_temp(egui::Id::new(FILTER_BASE), base));
            }
            egui::Popup::menu(&r)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| selection_filter(ui, state));
            ui.separator();
        }
        // What the pointer pre-selects stands in for the message meanwhile.
        ui.label(theme::muted(state.hint.shown().unwrap_or(&state.message)));
        let rev = (state.active_view, state.session.doc.revision());
        let cache = egui::Id::new("title-block-error");
        let error = match ui.data(|d| d.get_temp::<((Option<cc_core::ViewId>, u64), Option<String>)>(cache)) {
            Some((r, e)) if r == rev => e,
            _ => {
                let e = crate::sheet_props::title_block_error(state);
                ui.data_mut(|d| d.insert_temp(cache, (rev, e.clone())));
                e
            }
        };
        if let Some(e) = error {
            ui.separator();
            ui.label(RichText::new(e).color(PALETTE.warning));
        }
        if state.updates.available.is_some() {
            ui.separator();
            crate::update::notice(ui, &mut state.updates);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let file = state
                .session
                .doc
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| t!("status-unsaved-file"));
            let dirty = if state.session.doc.is_dirty() { " *" } else { "" };
            ui.label(RichText::new(format!("{file}{dirty}")).color(PALETTE.text_faint));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_titles_take_their_own_width_and_squeeze_evenly() {
        let texts = [30.0, 80.0, 50.0];
        // Room to spare: the full padding, so each tab is its text plus 28.
        assert_eq!(tab_padding(&texts, 1000.0), TAB_PAD);
        // A little short: the padding shrinks so the titles just fit.
        assert_eq!(tab_padding(&texts, 160.0 + 3.0 * 10.0), 10.0);
        // Far too narrow: never below the least padding.
        assert_eq!(tab_padding(&texts, 50.0), TAB_PAD_MIN);
        assert_eq!(tab_padding(&[], f32::NAN), TAB_PAD);
    }

    #[test]
    fn long_labels_wrap_on_two_balanced_lines() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let mut lines = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::proportional(12.0);
            lines = [
                "Níveis",
                "Unidades do projeto",
                "Informações do projeto",
                "Relatório de perda de pressão da tubulação",
                "Supercalifragilístico",
            ]
            .map(|l| label_lines(ui, l, &font))
            .to_vec();
        });
        out.textures_delta.clear();
        assert_eq!(lines[0], vec!["Níveis"]);
        // Long labels break at a space into two lines that keep all the words.
        for (i, label) in [
            (1, "Unidades do projeto"),
            (2, "Informações do projeto"),
            (3, "Relatório de perda de pressão da tubulação"),
        ] {
            assert_eq!(lines[i].len(), 2, "{label}");
            assert_eq!(lines[i].join(" "), label);
            assert!(lines[i].iter().all(|l| l.len() < label.len()));
        }
        // A single long word cannot wrap.
        assert_eq!(lines[4], vec!["Supercalifragilístico"]);
    }

    #[test]
    fn labels_also_wrap_after_a_slash_or_a_hyphen() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let mut lines = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::proportional(12.0);
            lines = ["Visibilidade/gráficos", "Contra-flecha estrutural", "Importar IFC…"]
                .map(|l| label_lines(ui, l, &font))
                .to_vec();
        });
        out.textures_delta.clear();
        // The slash stays at the end of the first line.
        assert_eq!(lines[0], vec!["Visibilidade/", "gráficos"]);
        assert_eq!(lines[1].len(), 2);
        assert_eq!(lines[2], vec!["Importar", "IFC…"]);
        // Never an empty line: no break before the first or after the last character.
        assert!(break_points("/a").is_empty());
        assert!(break_points("a-").is_empty());
        assert!(break_points(" a ").is_empty());
        assert_eq!(break_points("a/b c"), vec![(2, 2), (3, 4)]);
    }

    #[test]
    fn large_buttons_keep_a_least_width_and_even_padding() {
        // Short labels share one width, so their icons stand evenly apart.
        assert_eq!(button_width(20.0, 0.0), BUTTON_MIN_WIDTH);
        assert_eq!(button_width(40.0, 0.0), BUTTON_MIN_WIDTH);
        // Longer ones get the same room at each side.
        assert_eq!(button_width(70.0, 0.0), 70.0 + 2.0 * BUTTON_PAD);
        assert_eq!(button_width(70.0, 11.0), 81.0 + 2.0 * BUTTON_PAD);
    }

    #[test]
    fn families_of_two_or_four_use_two_rows() {
        assert_eq!([1, 2, 3, 4, 5, 6, 7].map(stack_rows), [3, 2, 3, 2, 3, 3, 3]);
    }
}
