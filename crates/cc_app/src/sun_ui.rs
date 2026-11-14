//! The Sun Settings dialog (*View → Sun Settings*, and the *Sun* row of a
//! view's properties): how a view places its sun (one moment, a study over
//! one or several days, or a light from a direction given by hand), the
//! project's sun presets, the site's time zone and what is drawn (the sun
//! path, shadows, a ground plane). Every change runs a command as it is
//! made (`view_set`, `sun_preset_*`, `site_set`), so the view behind shows
//! it at once; OK keeps them all as one undo step, Cancel undoes them
//! (`dialog::session`, through [`crate::graphics_dialog::Edits`]).
//!
//! The Site Location window is not opened from here: it keeps its own
//! dialog session, which would end this one's. A hint names where it is.

use crate::graphics_dialog::{Edits, key, run};
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::table::{row, section};
use crate::theme::PALETTE;
use cc_core::sun::{
    Daylight, MAX_MINUTE, MAX_YEAR, MIN_YEAR, SunDate, SunInterval, SunMode, SunSettings, SunSite, UTC_OFFSET_RANGE,
    days_in_month, view_forward,
};
use cc_core::{LevelId, ViewId, ViewKind};
use cc_engine::Ref;
use cc_engine::commands::site::SiteSet;
use cc_engine::commands::sun::{SunArg, SunPresetDelete, SunPresetRename, SunPresetSave};
use cc_engine::commands::views::ViewSet;
use egui::{Context, DragValue, RichText, Ui};

/// Width of the dialog's body.
const WIDTH: f32 = 660.0;
/// Width of the left column (mode and presets).
const LEFT: f32 = 220.0;
/// Height of the preset list.
const LIST_HEIGHT: f32 = 170.0;
/// The year of the built-in presets when the view's sun has no date.
const DEFAULT_YEAR: i32 = 2026;
/// Minutes a dragged time moves by.
const TIME_STEP: f64 = 5.0;
/// Hours a dragged time zone moves by.
const OFFSET_STEP: f64 = 0.25;

/// The open Sun Settings dialog.
pub struct SunDialog {
    /// The view whose sun is edited.
    view: ViewId,
    edits: Edits,
    /// Undo depth when the dialog opened: OK folds what follows into one step.
    start: usize,
    /// The document was saved when the dialog opened.
    clean: bool,
    /// The project preset last clicked or saved (several may share a mode).
    picked: Option<String>,
    /// A preset name being typed.
    naming: Option<Naming>,
    /// Why the last save or rename was refused.
    error: Option<String>,
    /// The date of the last dated sun, so a switch to lighting and back keeps it.
    last_date: Option<SunDate>,
}

/// The inline name field of *Save…* and *Rename*.
struct Naming {
    /// The preset renamed (`None`: a new one is saved).
    renaming: Option<String>,
    text: String,
    /// Take the keyboard focus in the next frame.
    focus: bool,
}

/// Open the dialog on the active view (when it takes a sun).
pub fn open(state: &mut AppState) {
    if let Some(view) = state.active_view {
        open_for(state, view);
    }
}

/// Open the dialog on `view`. A view without a sun gets the default one,
/// inside the dialog's edits (Cancel takes it away again).
pub fn open_for(state: &mut AppState, view: ViewId) {
    if state.dialogs.sun.is_some() {
        return;
    }
    let Some(v) = state.project().view(view) else { return };
    if !v.kind.takes_sun() {
        state.message = t!("why-sun-view");
        return;
    }
    let had_sun = v.sun.is_some();
    let clean = !state.session.doc.is_dirty();
    let edits = Edits::begin(state);
    let start = state.session.doc.undo_depth();
    if !had_sun {
        set_sun(state, view, SunArg::default());
    }
    let last_date = state.project().view(view).and_then(|v| v.sun.as_ref()).and_then(|s| date_of(&s.mode));
    state.dialogs.sun =
        Some(SunDialog { view, edits, start, clean, picked: None, naming: None, error: None, last_date });
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.dialogs.sun.take() else { return };
    // The view went away (or can no longer have a sun): nothing left to edit.
    if !state.project().view(d.view).is_some_and(|v| v.kind.takes_sun()) {
        finish(state, d, true);
        return;
    }
    let (mut ok, mut remove) = (0, usize::MAX);
    let response = crate::dialog::Dialog::new("sun-settings", t!("action-sun-settings")).width(WIDTH).show(
        ctx,
        |ui| body(ui, state, &mut d),
        |f| {
            remove = f.extra(t!("sun-remove"));
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    let close = match response.clicked {
        Some(i) if i == remove => {
            crate::engine_edit::view(state, d.view, None, |v| v.sun = None);
            Some(true)
        }
        Some(i) => Some(i == ok),
        None if response.dismissed => Some(false),
        None => None,
    };
    match close {
        Some(keep) => finish(state, d, keep),
        None => state.dialogs.sun = Some(d),
    }
}

/// Close the dialog: keep its edits as one undo step (OK) or undo them
/// (Cancel). Edits that cancel out leave no step (a sun given on opening
/// and removed), and a saved document left as it was stays saved.
fn finish(state: &mut AppState, d: SunDialog, keep: bool) {
    let kept = keep && state.session.doc.squash_since(d.start, t!("action-sun-settings"));
    d.edits.finish(state, keep);
    if d.clean && !kept && state.session.doc.undo_depth() <= d.start {
        state.session.doc.mark_saved();
    }
}

// ---------------------------------------------------------------- changes

/// Change the view's sun by `arg` (`view_set`).
fn set_sun(state: &mut AppState, view: ViewId, arg: SunArg) -> bool {
    run(state, None, ViewSet { view: Ref::Id(view.0), sun: Some(Some(arg)), ..Default::default() })
}

/// Change the view's sun as `f` changes a copy of it (`view_set`), in one
/// undo step per `merge` key.
fn edit_sun(state: &mut AppState, view: ViewId, merge: Option<u64>, f: impl FnOnce(&mut SunSettings)) -> bool {
    crate::engine_edit::view(state, view, merge, |v| f(v.sun.get_or_insert_with(SunSettings::default)))
}

/// Another kind of sun: the engine starts it from its defaults keeping the date.
/// The engine carries the date from one dated kind to another; one coming
/// back from lighting takes the date the dialog last saw.
fn set_mode(state: &mut AppState, d: &mut SunDialog, from: &SunMode, name: &str) -> bool {
    if let Some(date) = date_of(from) {
        d.last_date = Some(date);
    }
    let mut arg = SunArg { mode: Some(name.to_string()), frame: Some(0), ..Default::default() };
    if !from.is_dated()
        && let Some(date) = d.last_date
    {
        match name {
            "still" | "single_day" => arg.date = Some(date.iso()),
            "multi_day" => {
                arg.from_date = Some(date.iso());
                arg.to_date = Some(SunDate::new(date.year, 12, 31).unwrap_or(date).iso());
            }
            _ => {}
        }
    }
    set_sun(state, d.view, arg)
}

/// Take `mode` from a preset, from the first frame.
fn apply_mode(state: &mut AppState, view: ViewId, mode: SunMode) -> bool {
    edit_sun(state, view, None, |s| {
        s.mode = mode;
        s.frame = 0;
    })
}

/// Set the site's offset from UTC (`None`: the longitude's time zone).
fn set_offset(state: &mut AppState, merge: Option<u64>, offset: Option<f64>) -> bool {
    run(state, merge, SiteSet { utc_offset: Some(offset), ..Default::default() })
}

/// A drag or a typed value ended: the next change starts a new step.
fn done(state: &mut AppState, ended: bool) {
    if ended {
        state.session.doc.break_merge();
    }
}

fn merge(view: ViewId, field: &str) -> Option<u64> {
    Some(key((view.0, "sun", field)))
}

// ---------------------------------------------------------------- body

fn body(ui: &mut Ui, state: &mut AppState, d: &mut SunDialog) {
    crate::dialog::two_columns(ui, [LEFT, WIDTH - LEFT - 9.0], |ui, column| {
        if column == 0 {
            left(ui, state, d);
        } else {
            right(ui, state, d.view);
        }
    });
    ui.add_space(6.0);
    status(ui, state, d.view);
}

/// The view's sun (the default when it has none yet).
fn current(state: &AppState, view: ViewId) -> Option<SunSettings> {
    state.project().view(view).map(|v| v.sun.clone().unwrap_or_default())
}

/// A mode's date (a multi-day study's first one).
fn date_of(mode: &SunMode) -> Option<SunDate> {
    match mode {
        SunMode::Still { date, .. } | SunMode::SingleDay { date, .. } => Some(*date),
        SunMode::MultiDay { from_date, .. } => Some(*from_date),
        SunMode::Lighting { .. } => None,
    }
}

fn left(ui: &mut Ui, state: &mut AppState, d: &mut SunDialog) {
    let Some(sun) = current(state, d.view) else { return };
    section(ui, "sun-study", t!("sun-study"), |ui| {
        crate::table::padded(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for name in SunMode::NAMES {
                let Some(mode) = SunMode::from_name(name) else { continue };
                let on = sun.mode.name() == name;
                if ui.radio(on, tr(mode.key())).clicked() && !on {
                    set_mode(state, d, &sun.mode, name);
                }
            }
        });
    });
    section(ui, "sun-presets", t!("sun-presets"), |ui| presets(ui, state, d, &sun));
}

/// The built-in presets of a kind of sun (`SunMode::name`), dated in `year`.
fn builtins(kind: &str, year: i32) -> Vec<(String, SunMode)> {
    let date = |month: u8, day: u8| SunDate::new(year, month, day).unwrap_or_default();
    let days = [
        (t!("sun-preset-june-solstice"), date(6, 21)),
        (t!("sun-preset-december-solstice"), date(12, 21)),
        (t!("sun-preset-march-equinox"), date(3, 20)),
        (t!("sun-preset-september-equinox"), date(9, 22)),
    ];
    let week = |from: SunDate, to: SunDate| SunMode::MultiDay {
        from_date: from,
        to_date: to,
        from: 360,
        to: 1080,
        sunrise_to_sunset: true,
        interval: SunInterval::Hour,
    };
    let light = |azimuth: f64| SunMode::Lighting { azimuth, altitude: 35.0, relative_to_view: true };
    match kind {
        "still" => days.into_iter().map(|(name, date)| (name, SunMode::Still { date, time: 720 })).collect(),
        "single_day" => days
            .into_iter()
            .map(|(name, date)| {
                let mode = SunMode::SingleDay {
                    date,
                    from: 360,
                    to: 1080,
                    sunrise_to_sunset: true,
                    interval: SunInterval::Hour,
                };
                (name, mode)
            })
            .collect(),
        "multi_day" => vec![
            (
                t!("sun-preset-whole-year"),
                SunMode::MultiDay {
                    from_date: date(1, 1),
                    to_date: date(12, 31),
                    from: 720,
                    to: 720,
                    sunrise_to_sunset: false,
                    interval: SunInterval::Month,
                },
            ),
            (t!("sun-preset-june-week"), week(date(6, 18), date(6, 24))),
            (t!("sun-preset-december-week"), week(date(12, 18), date(12, 24))),
        ],
        "lighting" => vec![(t!("sun-preset-light-right"), light(135.0)), (t!("sun-preset-light-left"), light(225.0))],
        _ => Vec::new(),
    }
}

/// The project preset shown selected: the one picked if it still has the
/// view's mode, else the first with it.
fn selected_preset(state: &AppState, d: &SunDialog, sun: &SunSettings) -> Option<String> {
    let presets = &state.project().sun_presets;
    d.picked
        .as_ref()
        .filter(|n| presets.iter().any(|p| &p.name == *n && p.mode == sun.mode))
        .cloned()
        .or_else(|| presets.iter().find(|p| p.mode == sun.mode).map(|p| p.name.clone()))
}

fn presets(ui: &mut Ui, state: &mut AppState, d: &mut SunDialog, sun: &SunSettings) {
    let kind = sun.mode.name();
    let year = date_of(&sun.mode).map_or(DEFAULT_YEAR, |date| date.year);
    let builtins = builtins(kind, year);
    let own: Vec<(String, SunMode)> = state
        .project()
        .sun_presets
        .iter()
        .filter(|p| p.mode.name() == kind)
        .map(|p| (p.name.clone(), p.mode.clone()))
        .collect();
    let chosen = selected_preset(state, d, sun);
    let mut apply: Option<(SunMode, Option<String>)> = None;
    egui::ScrollArea::vertical().id_salt("sun-presets").max_height(LIST_HEIGHT).auto_shrink([false, false]).show(
        ui,
        |ui| {
            for (name, mode) in &builtins {
                if ui.selectable_label(*mode == sun.mode, name).clicked() {
                    apply = Some((mode.clone(), None));
                }
            }
            if !own.is_empty() {
                ui.separator();
            }
            for (name, mode) in &own {
                if ui.selectable_label(chosen.as_ref() == Some(name), name).clicked() {
                    apply = Some((mode.clone(), Some(name.clone())));
                }
            }
        },
    );
    if let Some((mode, name)) = apply {
        apply_mode(state, d.view, mode);
        d.picked = name;
    }
    match d.naming.as_mut() {
        Some(naming) => name_field(ui, naming).apply(state, d),
        None => {
            crate::table::button_row(ui, |ui| {
                if ui.button(t!("sun-preset-save")).clicked() {
                    d.naming = Some(Naming { renaming: None, text: String::new(), focus: true });
                    d.error = None;
                }
                let rename = ui.add_enabled(chosen.is_some(), egui::Button::new(t!("sun-preset-rename")));
                if rename.clicked()
                    && let Some(name) = &chosen
                {
                    d.naming = Some(Naming { renaming: Some(name.clone()), text: name.clone(), focus: true });
                    d.error = None;
                }
                let delete = ui.add_enabled(chosen.is_some(), egui::Button::new(t!("sun-preset-delete")));
                if delete.clicked()
                    && let Some(name) = chosen.clone()
                {
                    run(state, None, SunPresetDelete { name });
                    d.picked = None;
                    d.error = None;
                }
            });
        }
    }
    if let Some(error) = &d.error {
        ui.label(RichText::new(error).size(11.5).color(PALETTE.warning));
    }
}

/// What the name field asks for this frame.
enum NameAction {
    None,
    Confirm,
    Cancel,
}

impl NameAction {
    fn apply(self, state: &mut AppState, d: &mut SunDialog) {
        match self {
            NameAction::None => {}
            NameAction::Cancel => {
                d.naming = None;
                d.error = None;
            }
            NameAction::Confirm => commit_name(state, d),
        }
    }
}

fn name_field(ui: &mut Ui, naming: &mut Naming) -> NameAction {
    ui.add_space(4.0);
    let r = ui.add(
        egui::TextEdit::singleline(&mut naming.text)
            .hint_text(t!("sun-preset-name-hint"))
            .desired_width(ui.available_width()),
    );
    if naming.focus {
        r.request_focus();
        naming.focus = false;
    }
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let confirm_label = if naming.renaming.is_some() { t!("sun-preset-rename") } else { t!("sun-preset-store") };
    let (confirm, cancel) = crate::table::button_row(ui, |ui| {
        let confirm = ui.add_enabled(!naming.text.trim().is_empty(), egui::Button::new(confirm_label)).clicked();
        let cancel = ui.button("×").on_hover_text(t!("action-cancel")).clicked();
        (confirm, cancel)
    });
    if cancel {
        NameAction::Cancel
    } else if confirm || (enter && !naming.text.trim().is_empty()) {
        NameAction::Confirm
    } else {
        NameAction::None
    }
}

/// Save the view's sun as a preset, or rename one, by the name typed; a
/// refused name keeps the field open with the engine's reason.
fn commit_name(state: &mut AppState, d: &mut SunDialog) {
    let Some(naming) = d.naming.as_ref() else { return };
    let result = match &naming.renaming {
        None => state.session.run(SunPresetSave {
            name: naming.text.clone(),
            view: Some(Ref::Id(d.view.0)),
            sun: None,
            replace: false,
        }),
        Some(old) => state.session.run(SunPresetRename { name: old.clone(), new_name: naming.text.clone() }),
    };
    match result {
        Ok(out) => {
            d.picked = out["name"].as_str().map(String::from);
            d.naming = None;
            d.error = None;
        }
        Err(e) => {
            state.message = e.message.clone();
            d.error = Some(e.message);
        }
    }
}

// ---------------------------------------------------------------- settings

fn right(ui: &mut Ui, state: &mut AppState, id: ViewId) {
    let Some(view) = state.project().view(id).cloned() else { return };
    let sun = view.sun.clone().unwrap_or_default();
    let site = state.project().sun_site();
    section(ui, "sun-settings", t!("sun-settings"), |ui| {
        if sun.mode.is_dated() {
            location_rows(ui, state, id);
        }
        match &sun.mode {
            SunMode::Still { date, time } => still_rows(ui, state, id, *date, *time),
            SunMode::SingleDay { .. } | SunMode::MultiDay { .. } => study_rows(ui, state, id, &sun, &site),
            SunMode::Lighting { azimuth, altitude, relative_to_view } => {
                lighting_rows(ui, state, id, *azimuth, *altitude, *relative_to_view)
            }
        }
        ui.add_space(6.0);
        common_rows(ui, state, id, &sun, matches!(view.kind, ViewKind::ThreeD { .. }));
    });
}

/// A checkbox reporting `label` as its name (the row's label is painted).
fn check(ui: &mut Ui, on: &mut bool, label: &str) -> egui::Response {
    let r = crate::table::checkbox(ui, on);
    let (enabled, value, label) = (ui.is_enabled(), *on, label.to_string());
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, value, &label));
    r
}

/// `12,3°` with the project's decimal symbol.
fn degrees(value: f64, decimals: usize) -> String {
    format!("{}°", cc_views::format::number(value, decimals))
}

/// `UTC−3`, `UTC+5:30`, `UTC+0`.
fn utc_label(hours: f64) -> String {
    let minutes = (hours * 60.0).round() as i64;
    let sign = if minutes < 0 { '−' } else { '+' };
    let (h, m) = (minutes.abs() / 60, minutes.abs() % 60);
    if m == 0 { format!("UTC{sign}{h}") } else { format!("UTC{sign}{h}:{m:02}") }
}

/// Reads `−3`, `+5:30`, `UTC-4,5`, `UTC+5.75`.
fn parse_utc(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_prefix("UTC").or_else(|| t.strip_prefix("utc")).unwrap_or(t).trim().replace('−', "-");
    let (negative, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim().to_string()),
        None => (false, t.trim_start_matches('+').trim().to_string()),
    };
    let hours = match body.split_once(':') {
        Some((h, m)) => {
            let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
            (m < 60).then(|| f64::from(h) + f64::from(m) / 60.0)?
        }
        None => body.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())?,
    };
    Some(if negative { -hours } else { hours })
}

fn location_rows(ui: &mut Ui, state: &mut AppState, id: ViewId) {
    let site = state.project().info.site.clone();
    row(ui, &t!("sun-location"), |ui| {
        ui.add_space(4.0);
        match &site {
            Some(s) if !s.place.trim().is_empty() => ui.label(s.place.trim()).on_hover_text(s.place.trim()),
            Some(s) => ui.label(format!("{}, {}", degrees(s.latitude, 4), degrees(s.longitude, 4))),
            None => ui.label(RichText::new(t!("sun-location-none")).color(PALETTE.warning)),
        };
    });
    let auto = site.as_ref().is_none_or(|s| s.utc_offset.is_none());
    let used = site.as_ref().map_or(0.0, |s| s.offset_hours());
    let known = site.is_some();
    row(ui, &t!("sun-time-zone"), |ui| {
        ui.spacing_mut().interact_size.x = 64.0;
        let mut on = auto;
        let r = ui
            .add_enabled(known, crate::table::boxed(egui::Checkbox::new(&mut on, t!("sun-time-zone-auto"))))
            .on_disabled_hover_text(t!("sun-time-zone-no-site"));
        if r.changed() {
            set_offset(state, None, if on { None } else { Some(used) });
        }
        if auto {
            ui.label(RichText::new(utc_label(used)).color(PALETTE.text_muted));
        } else {
            let mut hours = used;
            let (lo, hi) = (*UTC_OFFSET_RANGE.start(), *UTC_OFFSET_RANGE.end());
            let r = ui.add(
                DragValue::new(&mut hours)
                    .range(lo..=hi)
                    .speed(0.05)
                    .custom_formatter(|v, _| utc_label(v))
                    .custom_parser(parse_utc),
            );
            if r.changed() {
                let hours = ((hours / OFFSET_STEP).round() * OFFSET_STEP).clamp(lo, hi);
                if (hours - used).abs() > 1e-9 {
                    set_offset(state, merge(id, "utc"), Some(hours));
                }
            }
            done(state, r.drag_stopped() || r.lost_focus());
        }
    });
    let path = format!("{} → {}", t!("ribbon-manage"), t!("dlg-site-location"));
    crate::table::padded(ui, |ui| {
        ui.label(RichText::new(t!("sun-site-hint", "path" => path)).size(11.0).color(PALETTE.text_faint));
    });
}

/// The order a language writes dates in, and the mark between the parts.
#[derive(Clone, Copy)]
enum Part {
    Day,
    Month,
    Year,
}

fn date_order() -> ([Part; 3], char) {
    let text = crate::i18n::format_date(2000, 11, 22);
    let at = |s: &str| text.find(s).unwrap_or(usize::MAX);
    let mut parts = [(at("2000"), Part::Year), (at("11"), Part::Month), (at("22"), Part::Day)];
    parts.sort_by_key(|(at, _)| *at);
    let mark = text.chars().find(|c| !c.is_ascii_digit()).unwrap_or('/');
    let [(_, a), (_, b), (_, c)] = parts;
    ([a, b, c], mark)
}

/// A date as day, month and year fields in the language's order: the new
/// date when one changed, and whether a drag or an edit ended.
fn date_field(ui: &mut Ui, date: SunDate) -> (Option<SunDate>, bool) {
    let (mut year, mut month, mut day) = (date.year, date.month, date.day);
    let (mut changed, mut ended) = (false, false);
    ui.spacing_mut().interact_size.x = 22.0;
    ui.spacing_mut().item_spacing.x = 2.0;
    let (order, mark) = date_order();
    for (i, part) in order.into_iter().enumerate() {
        if i > 0 {
            ui.label(RichText::new(mark.to_string()).color(PALETTE.text_faint));
        }
        let r = match part {
            Part::Day => ui.add(DragValue::new(&mut day).range(1..=days_in_month(year, month)).speed(0.1)),
            Part::Month => ui.add(DragValue::new(&mut month).range(1..=12).speed(0.05)),
            Part::Year => ui.add(DragValue::new(&mut year).range(MIN_YEAR..=MAX_YEAR).speed(0.05)),
        };
        changed |= r.changed();
        ended |= r.drag_stopped() || r.lost_focus();
    }
    let new = SunDate::new(year, month, day.min(days_in_month(year, month))).filter(|n| changed && *n != date);
    (new, ended)
}

/// `HH:MM` of minutes after midnight (rounded, within the day).
fn clock(minutes: f64) -> String {
    let m = if minutes.is_finite() { minutes.round().clamp(0.0, f64::from(MAX_MINUTE)) } else { 0.0 } as u16;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Reads `HH:MM`, `H:MM`, `HHhMM` or a whole hour.
fn parse_clock(text: &str) -> Option<f64> {
    let t = text.trim();
    let (h, m) = t.split_once([':', 'h', 'H', '.']).unwrap_or((t, ""));
    let h: u16 = h.trim().parse().ok()?;
    let m: u16 = if m.trim().is_empty() { 0 } else { m.trim().parse().ok()? };
    (h < 24 && m < 60).then(|| f64::from(h * 60 + m))
}

/// A time of day: the new time when it changed (in steps of five minutes
/// while dragged, as typed otherwise), and whether a drag or an edit ended.
fn time_drag(ui: &mut Ui, minutes: u16, enabled: bool) -> (Option<u16>, bool) {
    let mut v = f64::from(minutes);
    let r = ui.add_enabled(
        enabled,
        DragValue::new(&mut v)
            .range(0.0..=f64::from(MAX_MINUTE))
            .speed(1.0)
            .custom_formatter(|v, _| clock(v))
            .custom_parser(parse_clock),
    );
    let mut new = None;
    if r.changed() {
        let v = if r.dragged() { (v / TIME_STEP).round() * TIME_STEP } else { v.round() };
        let m = v.clamp(0.0, f64::from(MAX_MINUTE)) as u16;
        if m != minutes {
            new = Some(m);
        }
    }
    (new, r.drag_stopped() || r.lost_focus())
}

/// From and to times of day: the new pair when one changed (the other end
/// follows so `from` never passes `to`), and whether an edit ended.
fn time_span(ui: &mut Ui, from: u16, to: u16, enabled: bool) -> (Option<(u16, u16)>, bool) {
    ui.spacing_mut().interact_size.x = 44.0;
    ui.label(RichText::new(t!("sun-from")).color(PALETTE.text_muted));
    let (a, e1) = time_drag(ui, from, enabled);
    ui.label(RichText::new(t!("sun-to")).color(PALETTE.text_muted));
    let (b, e2) = time_drag(ui, to, enabled);
    let new = match (a, b) {
        (Some(a), _) => Some((a, to.max(a))),
        (None, Some(b)) => Some((from.min(b), b)),
        (None, None) => None,
    };
    (new, e1 || e2)
}

fn still_rows(ui: &mut Ui, state: &mut AppState, id: ViewId, date: SunDate, time: u16) {
    let (new, ended) = row(ui, &t!("sun-date"), |ui| date_field(ui, date));
    if let Some(x) = new {
        edit_sun(state, id, merge(id, "date"), |s| {
            if let SunMode::Still { date, .. } = &mut s.mode {
                *date = x;
            }
        });
    }
    done(state, ended);
    let (new, ended) = row(ui, &t!("sun-time"), |ui| {
        ui.spacing_mut().interact_size.x = 44.0;
        time_drag(ui, time, true)
    });
    if let Some(x) = new {
        edit_sun(state, id, merge(id, "time"), |s| {
            if let SunMode::Still { time, .. } = &mut s.mode {
                *time = x;
            }
        });
    }
    done(state, ended);
}

/// Sunrise and sunset on `date`, as whole minutes within the day (the whole
/// day under a polar day, noon under a polar night).
fn daylight_span(site: &SunSite, date: SunDate) -> (u16, u16) {
    let last = f64::from(MAX_MINUTE);
    match site.daylight(date) {
        Daylight::Normal { sunrise, sunset } => {
            let m = |v: f64| if v.is_finite() { v.round().clamp(0.0, last) as u16 } else { 0 };
            (m(sunrise), m(sunset))
        }
        Daylight::PolarDay => (0, MAX_MINUTE),
        Daylight::PolarNight => (720, 720),
    }
}

/// The rows of a study over one or several days.
fn study_rows(ui: &mut Ui, state: &mut AppState, id: ViewId, sun: &SunSettings, site: &SunSite) {
    // The (first) date, a multi-day study's last date, the times of day.
    let (first, last, (t_from, t_to), sunrise_to_sunset, interval) = match &sun.mode {
        SunMode::SingleDay { date, from, to, sunrise_to_sunset, interval } => {
            (*date, None, (*from, *to), *sunrise_to_sunset, *interval)
        }
        SunMode::MultiDay { from_date, to_date, from, to, sunrise_to_sunset, interval } => {
            (*from_date, Some(*to_date), (*from, *to), *sunrise_to_sunset, *interval)
        }
        SunMode::Still { .. } | SunMode::Lighting { .. } => return,
    };
    match last {
        None => {
            let (new, ended) = row(ui, &t!("sun-date"), |ui| date_field(ui, first));
            if let Some(x) = new {
                edit_sun(state, id, merge(id, "date"), |s| {
                    if let SunMode::SingleDay { date, .. } = &mut s.mode {
                        *date = x;
                    }
                });
            }
            done(state, ended);
        }
        Some(last) => {
            let (new, ended) = row(ui, &t!("sun-date-from"), |ui| date_field(ui, first));
            if let Some(x) = new {
                edit_sun(state, id, merge(id, "from-date"), |s| {
                    if let SunMode::MultiDay { from_date, to_date, .. } = &mut s.mode {
                        *from_date = x;
                        *to_date = (*to_date).max(x);
                    }
                });
            }
            done(state, ended);
            let (new, ended) = row(ui, &t!("sun-date-to"), |ui| date_field(ui, last));
            if let Some(x) = new {
                edit_sun(state, id, merge(id, "to-date"), |s| {
                    if let SunMode::MultiDay { from_date, to_date, .. } = &mut s.mode {
                        *to_date = x;
                        *from_date = (*from_date).min(x);
                    }
                });
            }
            done(state, ended);
        }
    }
    // With sunrise to sunset the times are the day's (the first day's), not editable.
    let (shown_from, shown_to) = if sunrise_to_sunset { daylight_span(site, first) } else { (t_from, t_to) };
    let (new, ended) = row(ui, &t!("sun-time"), |ui| time_span(ui, shown_from, shown_to, !sunrise_to_sunset));
    if let Some((a, b)) = new {
        edit_sun(state, id, merge(id, "times"), |s| {
            if let SunMode::SingleDay { from, to, .. } | SunMode::MultiDay { from, to, .. } = &mut s.mode {
                (*from, *to) = (a, b);
            }
        });
    }
    done(state, ended);
    let label = t!("sun-sunrise-to-sunset");
    let mut on = sunrise_to_sunset;
    if row(ui, &label, |ui| check(ui, &mut on, &label)).changed() {
        edit_sun(state, id, None, |s| {
            if let SunMode::SingleDay { sunrise_to_sunset, .. } | SunMode::MultiDay { sunrise_to_sunset, .. } =
                &mut s.mode
            {
                *sunrise_to_sunset = on;
            }
        });
    }
    let choices: &[SunInterval] = if last.is_some() { &SunInterval::ALL } else { &SunInterval::SUB_DAY };
    let mut picked = interval;
    row(ui, &t!("sun-interval"), |ui| {
        egui::ComboBox::from_id_salt(("sun-interval", id.0)).selected_text(tr(interval.key())).show_ui(ui, |ui| {
            for &i in choices {
                ui.selectable_value(&mut picked, i, tr(i.key()));
            }
        });
    });
    if picked != interval {
        edit_sun(state, id, None, |s| {
            if let SunMode::SingleDay { interval, .. } | SunMode::MultiDay { interval, .. } = &mut s.mode {
                *interval = picked;
            }
            s.frame = 0;
        });
    }
    frame_rows(ui, state, id, sun, site);
}

/// The number of frames of a study and the one shown.
fn frame_rows(ui: &mut Ui, state: &mut AppState, id: ViewId, sun: &SunSettings, site: &SunSite) {
    let frames = sun.frames(site);
    let count = u32::try_from(frames.len()).unwrap_or(u32::MAX).max(1);
    crate::table::readonly_row(ui, &t!("sun-frames"), count.to_string());
    let shown = sun.frame.min(count - 1);
    let mut n = shown + 1;
    let text = frames
        .get(usize::try_from(shown).unwrap_or(usize::MAX))
        .map(|f| {
            let date = crate::i18n::format_date(f.date.year, u32::from(f.date.month), u32::from(f.date.day));
            format!("{n} · {date} {}", clock(f.minutes))
        })
        .unwrap_or_default();
    let r = row(ui, &t!("sun-frame"), |ui| {
        ui.spacing_mut().slider_width = 80.0;
        // Table cells are flat; the slider's rail needs the theme's fill to show.
        let theme = ui.ctx().style_of(ui.ctx().theme()).visuals.widgets.inactive;
        let inactive = &mut ui.visuals_mut().widgets.inactive;
        (inactive.bg_fill, inactive.weak_bg_fill) = (theme.bg_fill, theme.weak_bg_fill);
        inactive.bg_stroke = egui::Stroke::new(1.0, PALETTE.border_strong);
        let r = ui.add(egui::Slider::new(&mut n, 1..=count).show_value(false));
        ui.label(RichText::new(text).color(PALETTE.text_muted));
        r
    });
    if r.changed() && n.saturating_sub(1) != shown {
        edit_sun(state, id, merge(id, "frame"), |s| s.frame = n.saturating_sub(1));
    }
    done(state, r.drag_stopped() || r.lost_focus());
}

/// Degrees from 0 to `max`, written as the status line writes them.
fn angle_drag(value: &mut f64, max: f64) -> DragValue<'_> {
    DragValue::new(value).range(0.0..=max).custom_formatter(|v, _| degrees(v, 1)).custom_parser(|text| {
        let t = text.trim().trim_end_matches('°').trim().replace(',', ".");
        t.parse::<f64>().ok().filter(|v| v.is_finite())
    })
}

fn lighting_rows(ui: &mut Ui, state: &mut AppState, id: ViewId, azimuth: f64, altitude: f64, relative: bool) {
    let mut a = azimuth;
    let r = row(ui, &t!("sun-azimuth"), |ui| ui.add(angle_drag(&mut a, 360.0).speed(1.0)));
    if r.changed() && a != azimuth {
        edit_sun(state, id, merge(id, "azimuth"), |s| {
            if let SunMode::Lighting { azimuth, .. } = &mut s.mode {
                *azimuth = a;
            }
        });
    }
    done(state, r.drag_stopped() || r.lost_focus());
    let mut h = altitude;
    let r = row(ui, &t!("sun-altitude"), |ui| ui.add(angle_drag(&mut h, 90.0).speed(0.5)));
    if r.changed() && h != altitude {
        edit_sun(state, id, merge(id, "altitude"), |s| {
            if let SunMode::Lighting { altitude, .. } = &mut s.mode {
                *altitude = h;
            }
        });
    }
    done(state, r.drag_stopped() || r.lost_focus());
    let label = t!("sun-relative-to-view");
    let mut on = relative;
    if row(ui, &label, |ui| check(ui, &mut on, &label)).changed() {
        edit_sun(state, id, None, |s| {
            if let SunMode::Lighting { relative_to_view, .. } = &mut s.mode {
                *relative_to_view = on;
            }
        });
    }
}

/// What every kind of sun has: the sun path, shadows and the ground plane.
fn common_rows(ui: &mut Ui, state: &mut AppState, id: ViewId, sun: &SunSettings, three_d: bool) {
    let label = t!("sun-show-path");
    let mut path = sun.show_path;
    if row(ui, &label, |ui| check(ui, &mut path, &label)).changed() {
        edit_sun(state, id, None, |s| s.show_path = path);
    }
    let label = t!("sun-shadows");
    let mut shadows = sun.shadows;
    let r = row(ui, &label, |ui| {
        ui.add_enabled_ui(three_d, |ui| check(ui, &mut shadows, &label))
            .inner
            .on_disabled_hover_text(t!("sun-shadows-3d"))
    });
    if r.changed() {
        edit_sun(state, id, None, |s| s.shadows = shadows);
    }
    let levels: Vec<(LevelId, String, f64)> =
        state.project().levels.iter().map(|l| (l.id, l.name.clone(), l.elevation)).collect();
    let lowest = levels.iter().min_by(|a, b| a.2.total_cmp(&b.2)).map(|l| l.0);
    let ground = sun.ground.filter(|g| levels.iter().any(|l| l.0 == *g));
    let label = t!("sun-ground");
    let mut on = ground.is_some();
    let mut level = ground;
    row(ui, &label, |ui| {
        ui.add_enabled_ui(lowest.is_some(), |ui| check(ui, &mut on, &label));
        if let Some(current) = ground {
            let name = levels.iter().find(|l| l.0 == current).map(|l| l.1.clone()).unwrap_or_default();
            ui.spacing_mut().combo_width = (ui.available_width() - 8.0).max(40.0);
            egui::ComboBox::from_id_salt(("sun-ground", id.0)).selected_text(name).show_ui(ui, |ui| {
                for (l, name, _) in &levels {
                    ui.selectable_value(&mut level, Some(*l), name);
                }
            });
        }
    });
    let wanted = if on { level.or(lowest) } else { None };
    if wanted != ground {
        edit_sun(state, id, None, |s| s.ground = wanted);
    }
}

/// Where the sun stands now, and when it rises and sets that day.
fn status(ui: &mut Ui, state: &AppState, id: ViewId) {
    let p = state.project();
    let Some(view) = p.view(id) else { return };
    let sun = view.sun.clone().unwrap_or_default();
    let site = p.sun_site();
    let now = sun.state(&site, view_forward(view));
    let mut text = t!("sun-status", "azimuth" => degrees(now.azimuth, 1), "altitude" => degrees(now.altitude, 1));
    if let Some(frame) = now.frame {
        let day = match site.daylight(frame.date) {
            Daylight::Normal { sunrise, sunset } => {
                t!("sun-status-daylight", "sunrise" => clock(sunrise), "sunset" => clock(sunset))
            }
            Daylight::PolarDay => t!("sun-polar-day"),
            Daylight::PolarNight => t!("sun-polar-night"),
        };
        text = format!("{text} · {day}");
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text).color(PALETTE.text_muted));
        if sun.mode.is_dated() && !now.above_horizon {
            ui.label(RichText::new(format!("· {}", t!("sun-below-horizon"))).color(PALETTE.warning));
        }
    });
}

// ---------------------------------------------------------------- properties

/// The sun rows of a view's graphics section: the *Sun* button (it opens
/// the dialog) and, when the view has a sun, the sun path and shadows.
pub fn view_rows(ui: &mut Ui, state: &mut AppState, id: ViewId) {
    let Some(view) = state.project().view(id).cloned() else { return };
    if !view.kind.takes_sun() {
        return;
    }
    let text = view.sun.as_ref().map_or_else(|| t!("sun-none"), |s| tr(s.mode.key()));
    if row(ui, &t!("prop-sun"), |ui| ui.button(text).clicked()) {
        open_for(state, id);
    }
    let Some(sun) = view.sun else { return };
    let label = t!("prop-sun-path");
    let mut path = sun.show_path;
    if row(ui, &label, |ui| check(ui, &mut path, &label)).changed() {
        edit_sun(state, id, None, |s| s.show_path = path);
    }
    if matches!(view.kind, ViewKind::ThreeD { .. }) {
        let label = t!("sun-shadows");
        let mut shadows = sun.shadows;
        if row(ui, &label, |ui| check(ui, &mut shadows, &label)).changed() {
            edit_sun(state, id, None, |s| s.shadows = shadows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_offsets_read_back() {
        assert_eq!(clock(390.0), "06:30");
        assert_eq!(clock(-5.0), "00:00");
        assert_eq!(clock(f64::NAN), "00:00");
        assert_eq!(parse_clock("6:30"), Some(390.0));
        assert_eq!(parse_clock("18"), Some(1080.0));
        assert_eq!(parse_clock("24:00"), None);
        assert_eq!(parse_clock("ab"), None);
        assert_eq!(utc_label(-3.0), "UTC−3");
        assert_eq!(utc_label(5.5), "UTC+5:30");
        assert_eq!(utc_label(0.0), "UTC+0");
        for h in [-12.0, -3.0, -4.5, 0.0, 5.75, 14.0] {
            assert_eq!(parse_utc(&utc_label(h)), Some(h));
        }
        assert_eq!(parse_utc("-4,5"), Some(-4.5));
        assert_eq!(parse_utc("x"), None);
    }

    #[test]
    fn built_in_presets_are_valid_and_of_their_kind() {
        for kind in SunMode::NAMES {
            let list = builtins(kind, 2030);
            assert!(!list.is_empty(), "{kind}");
            for (name, mode) in list {
                assert!(!name.is_empty());
                assert_eq!(mode.name(), kind);
                assert!(mode.is_valid(), "{name}");
                if let Some(d) = date_of(&mode) {
                    assert_eq!(d.year, 2030);
                }
            }
        }
    }
}
