//! The Pipe Catalog (*Systems → Plumbing → Pipe Catalog*, also *Manage →
//! Pipe Catalog*): the project's pipe series and the built-in ones, filtered
//! by region and use; the selected series' sizes (outside diameter, wall,
//! inside diameter) and the fittings sold for it. *Load series* copies a
//! built-in series in, *Create types* makes pipe types of the sizes
//! checked, *Import file…* and *Export…* read and write series files and
//! *Delete* removes a project series (its types become free). Every change
//! is a `pipe_*` command; OK keeps them, Cancel undoes them all
//! (`dialog::session`, through [`crate::graphics_dialog::Edits`]).
//!
//! Also here: the pipe tool's type list grouped by series
//! ([`grouped_types`]) and the fittings a new pipe makes, named in the status
//! bar ([`report_fittings`]).

use crate::dialog::Dialog;
use crate::graphics_dialog::Edits;
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::table;
use crate::theme::{self, PALETTE};
use cc_core::pipe_catalog::Region;
use cc_core::{ElementId, ElementTypeId, PipeSeriesId, Project, TypeData};
use cc_engine::Ref;
use cc_engine::commands::pipes as cmd;
use egui::{Context, RichText, Stroke, Ui};
use serde_json::Value;
use std::collections::BTreeSet;

/// A series in the list: one of the project's or a built-in one not yet loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Project(PipeSeriesId),
    BuiltIn(String),
}

/// One series of the list.
#[derive(Debug, Clone)]
struct ListRow {
    selection: Selection,
    code: String,
    name: String,
    /// The built-in region (a project series copied from the catalog has its region).
    region: Option<Region>,
    use_: String,
}

#[derive(Default)]
struct Cache {
    revision: Option<u64>,
    project: Vec<ListRow>,
    built_in: Vec<ListRow>,
    /// The selected series in the file form (millimeters).
    detail: Option<(Selection, Value)>,
}

/// The open Pipe Catalog.
pub struct PipeCatalogDialog {
    pub(crate) selected: Option<Selection>,
    region: Option<Region>,
    /// `supply`, `drain` or every use.
    use_: Option<&'static str>,
    /// The size labels checked for *Create types*.
    pub(crate) picked: BTreeSet<String>,
    error: Option<String>,
    cache: Cache,
    edits: Edits,
}

/// Open the Pipe Catalog (on series `select`, when given).
pub fn open(state: &mut AppState) {
    open_on(state, None);
}

/// Open the Pipe Catalog with `select` selected.
pub fn open_on(state: &mut AppState, select: Option<Selection>) {
    if state.dialogs.pipe_catalog.is_some() {
        return;
    }
    let edits = Edits::begin(state);
    state.dialogs.pipe_catalog = Some(PipeCatalogDialog {
        selected: select,
        region: None,
        use_: None,
        picked: BTreeSet::new(),
        error: None,
        cache: Cache::default(),
        edits,
    });
}

/// Run an edit; a refusal goes to the dialog's error line.
fn run<C: cc_engine::Command + serde::Serialize>(
    state: &mut AppState,
    error: &mut Option<String>,
    command: C,
) -> Option<Value> {
    match state.session.run(command) {
        Ok(out) => {
            *error = None;
            Some(out)
        }
        Err(e) => {
            *error = Some(e.message);
            None
        }
    }
}

fn region_of(code: &str) -> Option<Region> {
    cc_core::pipe_catalog::built_in_of(code).map(|b| b.region)
}

/// Read the lists and the selected series again when the model changed.
fn refresh(state: &mut AppState, d: &mut PipeCatalogDialog) {
    let revision = state.session.doc.revision();
    if d.cache.revision != Some(revision) {
        let out = state.session.run(cmd::PipeSeriesList::default()).unwrap_or_default();
        let project: Vec<ListRow> = out["series"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| {
                let code = s["code"].as_str()?.to_string();
                Some(ListRow {
                    selection: Selection::Project(PipeSeriesId(s["id"].as_u64()?)),
                    name: s["name"].as_str().unwrap_or(&code).to_string(),
                    region: region_of(&code),
                    use_: s["use"].as_str().unwrap_or("any").to_string(),
                    code,
                })
            })
            .collect();
        if d.cache.built_in.is_empty() {
            let out = state.session.run(cmd::PipeCatalog::default()).unwrap_or_default();
            d.cache.built_in = out["series"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|s| {
                    let code = s["code"].as_str()?.to_string();
                    Some(ListRow {
                        selection: Selection::BuiltIn(code.clone()),
                        name: s["name"].as_str().unwrap_or(&code).to_string(),
                        region: region_of(&code),
                        use_: s["use"].as_str().unwrap_or("any").to_string(),
                        code,
                    })
                })
                .collect();
        }
        d.cache.project = project;
        d.cache.revision = Some(revision);
        d.cache.detail = None;
        // A built-in series just loaded: select the project's copy.
        if let Some(Selection::BuiltIn(code)) = &d.selected
            && let Some(row) = d.cache.project.iter().find(|r| r.code == *code)
        {
            d.selected = Some(row.selection.clone());
        }
    }
    let exists = |s: &Selection| d.cache.project.iter().chain(&d.cache.built_in).any(|r| r.selection == *s);
    if d.selected.as_ref().is_none_or(|s| !exists(s)) {
        d.selected = d.cache.project.first().or(d.cache.built_in.first()).map(|r| r.selection.clone());
    }
    if d.cache.detail.as_ref().map(|(s, _)| s) != d.selected.as_ref() {
        d.picked.clear();
        d.cache.detail = d.selected.clone().and_then(|s| {
            let out = match &s {
                Selection::Project(id) => state.session.run(cmd::PipeSeriesGet { series: Ref::Id(id.0) }).ok()?,
                Selection::BuiltIn(code) => {
                    let out = state.session.run(cmd::PipeCatalog { series: Some(code.clone()) }).ok()?;
                    out["series"][0].clone()
                }
            };
            Some((s, out))
        });
    }
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.dialogs.pipe_catalog.take() else { return };
    refresh(state, &mut d);
    let mut ok = 0;
    let response = Dialog::new("pipe-catalog", t!("pipe-catalog-title")).width(940.0).show(
        ctx,
        |ui| {
            crate::dialog::two_columns(ui, [280.0, 650.0], |ui, column| {
                if column == 0 {
                    list(ui, state, &mut d);
                } else {
                    details(ui, state, &mut d);
                }
            });
        },
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    let close = match response.clicked {
        Some(i) => Some(i == ok),
        None if response.dismissed => Some(false),
        None => None,
    };
    match close {
        Some(keep) => d.edits.finish(state, keep),
        None => state.dialogs.pipe_catalog = Some(d),
    }
}

// ---------------------------------------------------------------- the list

fn use_label(code: &str) -> String {
    tr(&format!("pipe-use-{code}"))
}

fn list(ui: &mut Ui, state: &mut AppState, d: &mut PipeCatalogDialog) {
    egui::Grid::new("pipe-catalog-filters").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        ui.label(theme::muted(t!("pipe-catalog-region")));
        let shown = d.region.map_or_else(|| t!("pipe-catalog-all"), |r| tr(r.key()));
        egui::ComboBox::from_id_salt("pipe-catalog-region").selected_text(shown).width(170.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut d.region, None, t!("pipe-catalog-all"));
            for r in Region::ALL {
                ui.selectable_value(&mut d.region, Some(r), tr(r.key()));
            }
        });
        ui.end_row();
        ui.label(theme::muted(t!("pipe-catalog-use")));
        let shown = d.use_.map_or_else(|| t!("pipe-catalog-all"), use_label);
        egui::ComboBox::from_id_salt("pipe-catalog-use").selected_text(shown).width(170.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut d.use_, None, t!("pipe-catalog-all"));
            for u in ["supply", "drain"] {
                ui.selectable_value(&mut d.use_, Some(u), use_label(u));
            }
        });
        ui.end_row();
    });
    ui.add_space(4.0);
    let keep = |r: &&ListRow| {
        d.region.is_none_or(|g| r.region == Some(g)) && d.use_.is_none_or(|u| r.use_ == u || r.use_ == "any")
    };
    let project: Vec<ListRow> = d.cache.project.iter().filter(keep).cloned().collect();
    let loaded: BTreeSet<&str> = d.cache.project.iter().map(|r| r.code.as_str()).collect();
    let built_in: Vec<ListRow> =
        d.cache.built_in.iter().filter(keep).filter(|r| !loaded.contains(r.code.as_str())).cloned().collect();
    egui::Frame::new().stroke(Stroke::new(1.0, PALETTE.border)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt("pipe-catalog-list").max_height(420.0).auto_shrink([false, false]).show(
            ui,
            |ui| {
                for (id, title, rows) in [
                    ("pipe-catalog-project", t!("pipe-catalog-in-project"), project),
                    ("pipe-catalog-built-in", t!("pipe-catalog-built-in"), built_in),
                ] {
                    if rows.is_empty() {
                        continue;
                    }
                    table::section(ui, id, title, |ui| {
                        for r in rows {
                            let on = d.selected.as_ref() == Some(&r.selection);
                            if ui.selectable_label(on, &r.name).on_hover_text(&r.code).clicked() {
                                d.selected = Some(r.selection.clone());
                                d.error = None;
                            }
                        }
                    });
                }
            },
        );
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button(t!("pipe-catalog-import-file")).clicked() {
            import_file(state, d);
        }
        let project = match &d.selected {
            Some(Selection::Project(id)) => Some(*id),
            _ => None,
        };
        if ui.add_enabled(project.is_some(), egui::Button::new(t!("pipe-catalog-export"))).clicked()
            && let Some(id) = project
        {
            export_file(state, d, id);
        }
        let delete = ui
            .add_enabled(project.is_some(), egui::Button::new(t!("pipe-catalog-delete")))
            .on_hover_text(t!("pipe-catalog-delete-tip"));
        if delete.clicked()
            && let Some(id) = project
            && run(state, &mut d.error, cmd::PipeSeriesDelete { series: Ref::Id(id.0), detach: true }).is_some()
        {
            d.selected = None;
        }
    });
}

fn import_file(state: &mut AppState, d: &mut PipeCatalogDialog) {
    let dialog = rfd::FileDialog::new().add_filter(t!("file-filter-pipe-series"), &["json"]);
    let Some(path) = dialog.pick_file() else { return };
    let command = cmd::PipeSeriesImport { path: Some(path), on_clash: cmd::SeriesClash::Rename, ..Default::default() };
    if let Some(out) = run(state, &mut d.error, command)
        && let Some(id) = out["series"].as_u64()
    {
        d.selected = Some(Selection::Project(PipeSeriesId(id)));
    }
}

fn export_file(state: &mut AppState, d: &mut PipeCatalogDialog, id: PipeSeriesId) {
    let code = d.cache.project.iter().find(|r| r.selection == Selection::Project(id)).map(|r| r.code.clone());
    let dialog = rfd::FileDialog::new()
        .add_filter(t!("file-filter-pipe-series"), &["json"])
        .set_file_name(format!("{}.json", code.unwrap_or_else(|| "series".into()).to_lowercase()));
    let Some(path) = dialog.save_file() else { return };
    if run(state, &mut d.error, cmd::PipeSeriesExport { series: Ref::Id(id.0), path: Some(path.clone()) }).is_some() {
        state.message = t!("msg-exported", "path" => path.display());
    }
}

// ---------------------------------------------------------------- the selected series

/// A length of the file form (millimeters) as shown: millimeters, or inches
/// in imperial projects.
fn dim(v: &Value) -> String {
    match v.as_f64() {
        Some(mm) if cc_views::format::imperial() => format!("{:.3}\"", mm / 25.4),
        Some(mm) => format!("{}", (mm * 100.0).round() / 100.0),
        None => "—".into(),
    }
}

fn unit() -> &'static str {
    cc_views::format::unit_symbol(if cc_views::format::imperial() { "in" } else { "mm" })
}

/// The pipe types of the project that are sizes of series `id`: size label → type id.
fn types_of(project: &Project, id: PipeSeriesId) -> Vec<(String, ElementTypeId)> {
    project
        .types
        .values()
        .filter_map(|t| match &t.data {
            TypeData::Pipe { series: Some(s), size, .. } if *s == id => Some((size.clone(), t.id)),
            _ => None,
        })
        .collect()
}

fn details(ui: &mut Ui, state: &mut AppState, d: &mut PipeCatalogDialog) {
    let Some((selection, detail)) = d.cache.detail.clone() else {
        ui.label(theme::muted(t!("pipe-catalog-empty")));
        return;
    };
    let name = detail["name"].as_str().unwrap_or_default();
    ui.label(RichText::new(name).strong().size(15.0));
    let facts: Vec<String> = [
        detail["standard"].as_str().map(str::to_string),
        detail["material"].as_str().map(str::to_string),
        detail["pressure"].as_str().map(str::to_string),
        detail["use"].as_str().map(use_label),
        detail["code"].as_str().map(str::to_string),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();
    ui.label(theme::muted(facts.join(" · ")));
    ui.add_space(4.0);
    let typed: Vec<(String, ElementTypeId)> = match &selection {
        Selection::Project(id) => types_of(state.project(), *id),
        Selection::BuiltIn(_) => Vec::new(),
    };
    let pipes: Vec<Value> = detail["pipes"].as_array().cloned().unwrap_or_default();
    table::section(ui, "pipe-catalog-sizes", t!("pipe-catalog-sizes"), |ui| {
        egui::ScrollArea::vertical().id_salt("pipe-catalog-sizes-rows").max_height(220.0).show(ui, |ui| {
            egui::Grid::new("pipe-catalog-sizes-grid").num_columns(6).striped(true).spacing([14.0, 3.0]).show(
                ui,
                |ui| {
                    let u = unit();
                    for h in [
                        String::new(),
                        t!("col-pipe-size"),
                        format!("de ({u})"),
                        format!("e ({u})"),
                        format!("di ({u})"),
                        t!("pipe-catalog-type"),
                    ] {
                        ui.label(theme::muted(h));
                    }
                    ui.end_row();
                    for p in &pipes {
                        let size = p["size"].as_str().unwrap_or_default().to_string();
                        let has_type = typed.iter().any(|(s, _)| *s == size);
                        let mut on = d.picked.contains(&size);
                        let r = ui.add_enabled(!has_type, egui::Checkbox::without_text(&mut on));
                        if r.changed() {
                            if on {
                                d.picked.insert(size.clone());
                            } else {
                                d.picked.remove(&size);
                            }
                        }
                        ui.label(&size);
                        ui.label(dim(&p["de"]));
                        ui.label(dim(&p["e"]));
                        ui.label(dim(&p["di"]));
                        ui.label(if has_type { "✔" } else { "" });
                        ui.end_row();
                    }
                },
            );
        });
        table::button_row(ui, |ui| {
            let loaded = matches!(selection, Selection::Project(_));
            if !loaded && ui.button(t!("pipe-catalog-load")).clicked() {
                load(state, d, &selection, None);
            }
            let create = ui
                .add_enabled(!d.picked.is_empty(), egui::Button::new(t!("pipe-catalog-create-types")))
                .on_disabled_hover_text(t!("pipe-catalog-pick-sizes"));
            if create.clicked() {
                let sizes: Vec<String> = d.picked.iter().cloned().collect();
                load(state, d, &selection, Some(sizes));
            }
        });
    });
    if let Some(e) = &d.error {
        ui.label(RichText::new(e).color(PALETTE.danger));
    }
    ui.add_space(4.0);
    let fittings: Vec<Value> = detail["fittings"].as_array().cloned().unwrap_or_default();
    let title = format!("{} ({})", t!("pipe-catalog-fittings"), fittings.len());
    table::section_closed(ui, "pipe-catalog-fittings", title, |ui| {
        egui::ScrollArea::vertical().id_salt("pipe-catalog-fitting-rows").max_height(220.0).show(ui, |ui| {
            egui::Grid::new("pipe-catalog-fittings-grid").num_columns(6).striped(true).spacing([14.0, 3.0]).show(
                ui,
                |ui| {
                    let u = unit();
                    for h in [
                        t!("col-fitting"),
                        t!("col-pipe-size"),
                        format!("{} ({u})", t!("pipe-catalog-socket")),
                        format!("z ({u})"),
                        format!("z2 ({u})"),
                        format!("{} ({u})", t!("pipe-catalog-body")),
                    ] {
                        ui.label(theme::muted(h));
                    }
                    ui.end_row();
                    for f in &fittings {
                        let kind = tr(&format!("fitting-{}", f["kind"].as_str().unwrap_or_default()));
                        let angle = f["angle"].as_f64().map_or(String::new(), |a| format!(" {a}°"));
                        let size = f["size"].as_str().unwrap_or_default();
                        let sizes = match f["size2"].as_str() {
                            Some(s2) if s2 != size => format!("{size} × {s2}"),
                            _ => size.to_string(),
                        };
                        let derived = f["derived"].as_str().filter(|s| !s.is_empty());
                        let r = ui.label(format!("{kind}{angle}"));
                        if let Some(text) = derived {
                            r.on_hover_text(text);
                        }
                        ui.label(sizes);
                        ui.label(dim(&f["socket"]));
                        ui.label(dim(&f["z"]));
                        ui.label(dim(&f["z2"]));
                        ui.label(dim(&f["od"]));
                        ui.end_row();
                    }
                },
            );
        });
    });
}

/// Load the selected built-in series (and make types of `sizes`), or make
/// types of `sizes` of a project series.
fn load(state: &mut AppState, d: &mut PipeCatalogDialog, selection: &Selection, sizes: Option<Vec<String>>) {
    match selection {
        Selection::BuiltIn(code) => {
            let command = cmd::PipeSeriesImport { code: Some(code.clone()), sizes, ..Default::default() };
            if let Some(out) = run(state, &mut d.error, command)
                && let Some(id) = out["series"].as_u64()
            {
                d.selected = Some(Selection::Project(PipeSeriesId(id)));
            }
        }
        Selection::Project(id) => {
            for size in sizes.unwrap_or_default() {
                let command = cmd::PipeTypeCreate { series: Ref::Id(id.0), size, name: None, material: None };
                if run(state, &mut d.error, command).is_none() {
                    break;
                }
            }
        }
    }
    d.picked.clear();
}

// ---------------------------------------------------------------- the pipe tool

/// A type of the tool's list: id and name.
type TypeItem = (ElementTypeId, String);
/// A group of the tool's list: the series' name (None for the free types) and its types.
type TypeGroup = (Option<String>, Vec<TypeItem>);

/// The pipe types of `types` grouped by their series, in the series' order
/// (by name), each group headed by the series' name; free types last, with
/// no heading.
pub fn grouped_types(project: &Project, types: Vec<TypeItem>) -> Vec<TypeGroup> {
    let mut groups: Vec<TypeGroup> = Vec::new();
    let mut free = Vec::new();
    for (id, name) in types {
        let series = project.element_type(id).and_then(|t| project.pipe_row(&t.data)).map(|(s, row)| (s, row.de));
        match series {
            Some((s, _)) => match groups.iter_mut().find(|(g, _)| g.as_deref() == Some(s.name.as_str())) {
                Some((_, list)) => list.push((id, name)),
                None => groups.push((Some(s.name.clone()), vec![(id, name)])),
            },
            None => free.push((id, name)),
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    // Sizes from the smallest up within a series.
    let de = |id: ElementTypeId| {
        project.element_type(id).and_then(|t| project.pipe_row(&t.data)).map_or(0.0, |(_, row)| row.de)
    };
    for (_, list) in &mut groups {
        list.sort_by(|a, b| de(a.0).total_cmp(&de(b.0)));
    }
    if !free.is_empty() {
        groups.push((None, free));
    }
    groups
}

/// What the status bar says of the fittings at the ends of pipe `id`: the
/// catalog ones by name, and the first one the series does not sell with
/// the reason (None when the pipe meets no catalog fitting).
pub fn fittings_message(state: &mut AppState, id: ElementId) -> Option<(String, bool)> {
    use cc_geom::fitting_catalog::{MatchStatus, fitting_name, kind_key};
    let fittings: Vec<cc_geom::mep_net::Fitting> =
        state.session.geometry().mep.fittings.iter().filter(|f| f.branch_runs.contains(&id)).cloned().collect();
    let project = state.project();
    let mut names = Vec::new();
    let mut missing = None;
    for f in &fittings {
        let Some(m) = &f.catalog else { continue };
        let Some(series) = project.pipe_series.get(&m.series) else { continue };
        match m.status {
            MatchStatus::Missing(reason) if missing.is_none() => {
                missing = Some(format!("{}: {}", tr(f.kind.key()), tr(reason.key())));
            }
            MatchStatus::Missing(_) => {}
            _ => {
                for piece in &m.pieces {
                    if let Some(row) = series.fittings.get(piece.row) {
                        names.push(fitting_name(&tr(kind_key(row.kind)), row, series));
                    }
                }
            }
        }
    }
    if let Some(m) = missing {
        return Some((t!("pipe-fitting-missing", "fitting" => m), true));
    }
    (!names.is_empty()).then(|| (t!("pipe-fittings-placed", "names" => names.join(", ")), false))
}

/// Name in the status bar the fittings pipe `id` meets (a missing one first, as a warning).
pub fn report_fittings(state: &mut AppState, id: ElementId) {
    if let Some((text, _warning)) = fittings_message(state, id) {
        state.message = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_pipe_types_group_by_series_from_the_smallest() {
        let project = cc_engine::new_project_with(cc_core::MeasurementSystem::Metric);
        let types: Vec<(ElementTypeId, String)> =
            project.types_of(cc_core::Category::Pipe).map(|t| (t.id, t.name.clone())).collect();
        let groups = grouped_types(&project, types.clone());
        assert_eq!(groups.iter().map(|g| g.1.len()).sum::<usize>(), types.len());
        assert!(groups.len() >= 2, "{groups:?}");
        for (series, list) in &groups {
            assert!(series.is_some(), "the template's types are all sizes of series");
            let de: Vec<f64> =
                list.iter().filter_map(|(id, _)| project.element_type(*id).and_then(|t| t.diameter())).collect();
            assert!(de.windows(2).all(|w| w[0] <= w[1]), "{de:?}");
        }
    }
}
