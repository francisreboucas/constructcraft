//! The Profile Manager (*Structure → Profiles*, also *Manage → Profiles*):
//! the project's structural profiles grouped by material, with a search;
//! the selected one's outline drawn to fit, its name, material and
//! dimensions (millimeters on screen, meters to the engine) and its section
//! properties. *New* makes a profile of a standard shape, *Duplicate* copies
//! one, *Delete* removes one (with the profile that takes its place in the
//! types using it) and *Import from catalog* copies rows of the built-in
//! series in. Every change is a `profile_*` command; OK keeps them, Cancel
//! undoes them all (`dialog::session`, through [`crate::graphics_dialog::Edits`]).
//! What is shown is read back with `profiles_list` and `profile_get`.

use crate::dialog::Dialog;
use crate::graphics_dialog::{Edits, key};
use crate::i18n::tr;
use crate::state::AppState;
use crate::t;
use crate::table;
use crate::theme::{self, PALETTE};
use cc_core::profile_catalog::{self, CatalogRow, ProfileLibrary, Series};
use cc_core::structural_profile::MAX_NAME;
use cc_core::{ProfileId, ProfileMaterial, ProfileShape};
use cc_engine::Ref;
use cc_engine::commands::profiles as cmd;
use egui::{Context, Pos2, RichText, Sense, Shape, Stroke, Ui, vec2};
use glam::DVec2;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Side of the outline preview, points.
const PREVIEW: f32 = 250.0;

/// The i18n key of a shape code's name (`profile-shape-cold-lipped-u`).
pub fn shape_key(code: &str) -> String {
    format!("profile-shape-{}", code.replace('_', "-"))
}

/// The i18n key of what a shape parameter measures (`profile-param-bf-top`).
pub fn param_key(name: &str) -> String {
    format!("profile-param-{}", name.replace('_', "-"))
}

/// A shape's name in the interface language.
fn shape_label(code: &str) -> String {
    tr(&shape_key(code))
}

/// The material a new profile of a shape starts with.
fn material_for(code: &str) -> ProfileMaterial {
    if code.starts_with("cold_") {
        ProfileMaterial::ColdFormed
    } else if code.starts_with("concrete_") || code == "rect" || code == "round" {
        ProfileMaterial::Concrete
    } else {
        ProfileMaterial::Steel
    }
}

/// One profile as `profiles_list` gives it.
struct Row {
    id: ProfileId,
    name: String,
    material: ProfileMaterial,
    shape: String,
    users: usize,
}

/// What the dialog shows, read back while the document revision is unchanged.
#[derive(Default)]
struct Cache {
    revision: Option<u64>,
    list: Vec<Row>,
    /// `profile_get` of the selected profile.
    detail: Option<(ProfileId, Value)>,
}

/// Which source the left column of the Profile Manager displays.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Brazil,
    Project,
    Other,
}

/// A built-in catalog row selected for preview / one-click add.
#[derive(Clone, PartialEq, Eq)]
struct CatalogSelection {
    series: String,
    designation: String,
}

/// The open Profile Manager.
pub struct ProfilesDialog {
    pub(crate) selected: Option<ProfileId>,
    tab: LeftTab,
    catalog_selected: Option<CatalogSelection>,
    catalog_preview: Option<(CatalogSelection, Value)>,
    expand_selected: bool,
    search: String,
    /// The profile that takes a deleted one's place in its types.
    replace_with: Option<ProfileId>,
    /// The last refused change, shown under the parameters.
    error: Option<String>,
    pub(crate) catalog: Option<Catalog>,
    /// *Edit sketch* was pressed: the dialog closes (keeping its edits)
    /// and the selected profile opens in a sketch.
    edit_sketch: bool,
    cache: Cache,
    edits: Edits,
}

/// One series in the catalog dialog.
pub struct CatalogSeries {
    code: String,
    name: String,
    shape: String,
    library: String,
}

/// *Import from catalog*: a series, a filter and the designations picked.
pub struct Catalog {
    /// Selected library filter (`""` = all, `"brazil"`, `"europe"`, `"timber"`).
    library: String,
    /// The series' code.
    pub(crate) series: String,
    filter: String,
    picked: BTreeSet<String>,
    /// Every series with metadata.
    series_list: Vec<CatalogSeries>,
    /// The designations shown, for (series, filter).
    rows: Option<(String, String, Vec<String>)>,
    /// Why the last *Add* was refused, shown in the catalog itself.
    error: Option<String>,
}

/// Open the Profile Manager; not while one of its profile sketches is open
/// (the manager comes back when that sketch closes).
pub fn open(state: &mut AppState) {
    if section_sketch_open(state) {
        state.message = t!("why-profile-sketch-open");
        return;
    }
    let edits = Edits::begin(state);
    open_with(state, edits);
}

/// Whether a structural profile's sketch is open (the manager waits for it).
pub fn section_sketch_open(state: &AppState) -> bool {
    matches!(state.sketch.as_ref().map(|s| s.target), Some(crate::sketch_mode::Target::Section { .. }))
}

/// Keep the manager's edits when its profile sketch closed without going
/// back to it (another tool, a palette command…), so the shared dialog
/// session ends and a later dialog's Cancel undoes only its own edits. The
/// one place that settles them, run when the tool changes and every frame.
pub fn settle_abandoned(state: &mut AppState) {
    if state.dialogs.profiles.is_none()
        && !section_sketch_open(state)
        && let Some(edits) = state.dialogs.profiles_resume.take()
    {
        edits.finish(state, true);
    }
}

/// Open the manager again on profile `id` after its sketch closes, with
/// `edits`, the ones it had when the sketch opened (a fresh start if none).
pub fn resume(state: &mut AppState, id: ProfileId, edits: Option<Edits>) {
    match edits {
        Some(edits) => open_with(state, edits),
        None => open(state),
    }
    if let Some(d) = state.dialogs.profiles.as_mut() {
        d.tab = LeftTab::Project;
        d.selected = Some(id);
        d.expand_selected = true;
    }
}

fn default_catalog_selection(library: ProfileLibrary) -> Option<CatalogSelection> {
    let series = profile_catalog::catalog()
        .iter()
        .find(|s| s.library == library && s.code == "W")
        .or_else(|| profile_catalog::catalog().iter().find(|s| s.library == library))?;
    let row = series.rows.first()?;
    Some(CatalogSelection { series: series.code.to_string(), designation: row.designation.clone() })
}

fn open_with(state: &mut AppState, edits: Edits) {
    state.dialogs.profiles = Some(ProfilesDialog {
        selected: None,
        tab: LeftTab::Brazil,
        catalog_selected: default_catalog_selection(ProfileLibrary::Brazil),
        catalog_preview: None,
        expand_selected: true,
        search: String::new(),
        replace_with: None,
        error: None,
        catalog: None,
        edit_sketch: false,
        cache: Cache::default(),
        edits,
    });
}

/// Run an edit; a refusal goes to the dialog's error line.
fn run<C: cc_engine::Command + serde::Serialize>(
    state: &mut AppState,
    error: &mut Option<String>,
    merge: Option<u64>,
    command: C,
) -> Option<Value> {
    let result = match merge {
        Some(k) => state.session.run_merged(k, command),
        None => state.session.run(command),
    };
    match result {
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

fn profile_id(v: &Value) -> Option<ProfileId> {
    v["id"].as_u64().map(ProfileId)
}

/// Read the list and the selected profile again if the model changed.
fn refresh(state: &mut AppState, d: &mut ProfilesDialog) {
    let revision = state.session.doc.revision();
    if d.cache.revision != Some(revision) {
        let out = state.session.run(cmd::ProfilesList::default()).unwrap_or_default();
        let mut list: Vec<Row> = out
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| {
                Some(Row {
                    id: profile_id(r)?,
                    name: r["name"].as_str()?.to_string(),
                    material: ProfileMaterial::from_code(r["material"].as_str()?)?,
                    shape: r["shape"].as_str().unwrap_or_default().to_string(),
                    users: r["users"].as_u64().unwrap_or(0) as usize,
                })
            })
            .collect();
        list.sort_by_key(|r| r.name.to_lowercase());
        d.cache = Cache { revision: Some(revision), list, detail: None };
    }
    if d.selected.is_none_or(|id| !d.cache.list.iter().any(|r| r.id == id)) {
        d.selected = d.cache.list.first().map(|r| r.id);
    }
    if d.cache.detail.as_ref().map(|(id, _)| *id) != d.selected {
        d.cache.detail = d
            .selected
            .and_then(|id| state.session.run(cmd::ProfileGet { profile: Ref::Id(id.0) }).ok().map(|v| (id, v)));
    }
    // The replacement is another profile, of the same material when there is one.
    let selected = d.selected.and_then(|id| d.cache.list.iter().find(|r| r.id == id));
    let valid = d.replace_with.is_some_and(|w| Some(w) != d.selected && d.cache.list.iter().any(|r| r.id == w));
    if !valid {
        let others = || d.cache.list.iter().filter(|r| Some(r.id) != d.selected);
        d.replace_with = others()
            .find(|r| selected.is_some_and(|s| s.material == r.material))
            .or_else(|| others().next())
            .map(|r| r.id);
    }
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.dialogs.profiles.take() else { return };
    refresh(state, &mut d);
    let mut ok = 0;
    let response = Dialog::new("profiles", t!("profiles-title")).width(920.0).show(
        ctx,
        |ui| {
            crate::dialog::two_columns(ui, [270.0, 640.0], |ui, column| {
                if column == 0 {
                    left_panel(ui, state, &mut d);
                } else if d.tab == LeftTab::Project {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(PREVIEW);
                            if let Some((_, detail)) = &d.cache.detail {
                                preview(ui, detail);
                            }
                        });
                        ui.add_space(8.0);
                        ui.vertical(|ui| details(ui, state, &mut d));
                    });
                } else {
                    ensure_catalog_preview(&mut d);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(PREVIEW);
                            if let Some((_, detail)) = &d.catalog_preview {
                                preview(ui, detail);
                            }
                        });
                        ui.add_space(8.0);
                        ui.vertical(|ui| catalog_details(ui, state, &mut d));
                    });
                }
            });
        },
        |f| {
            ok = f.primary(t!("action-ok"));
            f.secondary(t!("action-cancel"));
        },
    );
    catalog_dialog(ctx, state, &mut d);
    if std::mem::take(&mut d.edit_sketch)
        && let Some(id) = d.selected
    {
        // The manager's session stays open: the sketch's Finish and Cancel
        // open it again with these edits, so its Cancel undoes the sketch too.
        state.session.doc.break_merge();
        if crate::profile_sketch::open_section(state, id) {
            state.dialogs.profiles_resume = Some(d.edits);
            return;
        }
    }
    let close = match response.clicked {
        Some(i) => Some(i == ok),
        None if response.dismissed && d.catalog.is_none() => Some(false),
        None => None,
    };
    match close {
        Some(keep) => d.edits.finish(state, keep),
        None => state.dialogs.profiles = Some(d),
    }
}

// ---------------------------------------------------------------- the list

fn left_panel(ui: &mut Ui, state: &mut AppState, d: &mut ProfilesDialog) {
    let br_count: usize =
        profile_catalog::catalog().iter().filter(|s| s.library == ProfileLibrary::Brazil).map(|s| s.rows.len()).sum();
    let other_count: usize =
        profile_catalog::catalog().iter().filter(|s| s.library != ProfileLibrary::Brazil).map(|s| s.rows.len()).sum();
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(d.tab == LeftTab::Brazil, format!("Brasil ({br_count})")).clicked() {
            d.tab = LeftTab::Brazil;
            if d.catalog_selected.as_ref().is_none_or(|sel| {
                profile_catalog::series(&sel.series).is_none_or(|s| s.library != ProfileLibrary::Brazil)
            }) {
                d.catalog_selected = default_catalog_selection(ProfileLibrary::Brazil);
                d.catalog_preview = None;
            }
        }
        if ui.selectable_label(d.tab == LeftTab::Project, format!("Projeto ({})", d.cache.list.len())).clicked() {
            d.tab = LeftTab::Project;
        }
        if ui.selectable_label(d.tab == LeftTab::Other, format!("Europa / Madeira ({other_count})")).clicked() {
            d.tab = LeftTab::Other;
            if d.catalog_selected.as_ref().is_none_or(|sel| {
                profile_catalog::series(&sel.series).is_none_or(|s| s.library == ProfileLibrary::Brazil)
            }) {
                d.catalog_selected = default_catalog_selection(ProfileLibrary::Europe);
                d.catalog_preview = None;
            }
        }
    });
    ui.add_space(4.0);
    ui.add(egui::TextEdit::singleline(&mut d.search).hint_text(t!("profiles-search")).desired_width(f32::INFINITY));
    ui.add_space(4.0);
    match d.tab {
        LeftTab::Project => list(ui, state, d),
        LeftTab::Brazil => catalog_list(ui, state, d, true),
        LeftTab::Other => catalog_list(ui, state, d, false),
    }
}

fn force_section_open(ui: &Ui, id: &str) {
    let state_id = ui.make_persistent_id(("prop-section", id));
    ui.data_mut(|data| data.insert_persisted(state_id, true));
}

fn list(ui: &mut Ui, state: &mut AppState, d: &mut ProfilesDialog) {
    let search = d.search.trim().to_lowercase();
    let selected_shape = d.selected.and_then(|id| d.cache.list.iter().find(|r| r.id == id)).map(|r| r.shape.clone());
    let expand_selected = std::mem::take(&mut d.expand_selected);
    egui::Frame::new().stroke(Stroke::new(1.0, PALETTE.border)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt("profiles-list").max_height(380.0).auto_shrink([false, false]).show(
            ui,
            |ui| {
                let mut seen_shapes: Vec<String> = ProfileShape::CODES.iter().map(|c| (*c).to_string()).collect();
                for r in &d.cache.list {
                    if !seen_shapes.contains(&r.shape) {
                        seen_shapes.push(r.shape.clone());
                    }
                }
                for shape in seen_shapes {
                    let rows: Vec<(ProfileId, String)> = d
                        .cache
                        .list
                        .iter()
                        .filter(|r| r.shape == shape && r.name.to_lowercase().contains(&search))
                        .map(|r| (r.id, format!("{}  ({})", r.name, tr(r.material.key()))))
                        .collect();
                    if rows.is_empty() {
                        continue;
                    }
                    let sec_id = format!("profiles-shape-{shape}");
                    if !search.is_empty() || (expand_selected && selected_shape.as_deref() == Some(shape.as_str())) {
                        force_section_open(ui, &sec_id);
                    }
                    let label = if shape.is_empty() { t!("profiles-title") } else { shape_label(&shape) };
                    table::section(ui, &sec_id, label, |ui| {
                        for (id, name) in rows {
                            if ui.selectable_label(d.selected == Some(id), name).clicked() {
                                d.selected = Some(id);
                                d.replace_with = None;
                                d.error = None;
                            }
                        }
                    });
                }
            },
        );
    });
    ui.add_space(6.0);
    buttons(ui, state, d);
}

fn catalog_list(ui: &mut Ui, state: &mut AppState, d: &mut ProfilesDialog, brazil_only: bool) {
    let search = d.search.trim().to_lowercase();
    let series_list: Vec<&Series> = profile_catalog::catalog()
        .iter()
        .filter(|s| if brazil_only { s.library == ProfileLibrary::Brazil } else { s.library != ProfileLibrary::Brazil })
        .collect();
    let mut shapes: Vec<&str> = Vec::new();
    for s in &series_list {
        if !shapes.contains(&s.shape) {
            shapes.push(s.shape);
        }
    }
    let project_names: BTreeSet<&str> = d.cache.list.iter().map(|r| r.name.as_str()).collect();

    egui::Frame::new().stroke(Stroke::new(1.0, PALETTE.border)).show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(if brazil_only { "profiles-brazil-list" } else { "profiles-other-list" })
            .max_height(410.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for shape in shapes {
                    let matching_series: Vec<(&Series, Vec<&CatalogRow>)> = series_list
                        .iter()
                        .filter(|s| s.shape == shape)
                        .filter_map(|&s| {
                            let rows: Vec<&CatalogRow> = s
                                .rows
                                .iter()
                                .filter(|r| {
                                    search.is_empty()
                                        || r.designation.to_lowercase().contains(&search)
                                        || s.code.to_lowercase().contains(&search)
                                        || tr(s.name_key).to_lowercase().contains(&search)
                                })
                                .collect();
                            (!rows.is_empty()).then_some((s, rows))
                        })
                        .collect();
                    if matching_series.is_empty() {
                        continue;
                    }
                    let sec_id = format!("profiles-lib-{}-{shape}", if brazil_only { "br" } else { "other" });
                    if !search.is_empty() {
                        force_section_open(ui, &sec_id);
                    }
                    table::section(ui, &sec_id, shape_label(shape), |ui| {
                        for (s, rows) in matching_series {
                            let header_id = ui.make_persistent_id(("cat-series", s.code));
                            let title = format!("{} — {} ({})", s.code, tr(s.name_key), rows.len());
                            let mut header = egui::CollapsingHeader::new(title)
                                .id_salt(header_id)
                                .default_open(s.code == "W" || s.code == "IPE");
                            if !search.is_empty() {
                                header = header.open(Some(true));
                            }
                            header.show(ui, |ui| {
                                for row in rows {
                                    let is_sel = d
                                        .catalog_selected
                                        .as_ref()
                                        .is_some_and(|sel| sel.series == s.code && sel.designation == row.designation);
                                    let in_proj = project_names.contains(row.designation.as_str());
                                    let label = if in_proj {
                                        format!("✓ {}", row.designation)
                                    } else {
                                        row.designation.clone()
                                    };
                                    if ui.selectable_label(is_sel, label).clicked() {
                                        d.catalog_selected = Some(CatalogSelection {
                                            series: s.code.to_string(),
                                            designation: row.designation.clone(),
                                        });
                                        d.catalog_preview = None;
                                        d.error = None;
                                    }
                                }
                            });
                        }
                    });
                }
            });
    });
    ui.add_space(6.0);
    if let Some(sel) = d.catalog_selected.clone() {
        let in_project = d.cache.list.iter().find(|r| r.name == sel.designation).map(|r| r.id);
        ui.horizontal_wrapped(|ui| {
            if let Some(id) = in_project {
                if ui.button("Selecionar no projeto").clicked() {
                    d.tab = LeftTab::Project;
                    d.selected = Some(id);
                    d.expand_selected = true;
                }
            } else if ui.button("Adicionar ao projeto").clicked() {
                import_from_catalog(state, d, &sel.series, vec![sel.designation.clone()]);
            }
            if let Some(s) = profile_catalog::series(&sel.series)
                && ui.button(format!("Adicionar série {}", s.code)).clicked()
            {
                let all: Vec<String> = s.rows.iter().map(|r| r.designation.clone()).collect();
                import_from_catalog(state, d, s.code, all);
            }
        });
    }
}

fn import_from_catalog(state: &mut AppState, d: &mut ProfilesDialog, series: &str, designations: Vec<String>) {
    let command = cmd::ProfileImport { series: series.to_string(), designations, on_clash: None };
    if let Some(out) = run(state, &mut d.error, None, command)
        && let Some(id) = profile_id(&out["imported"][0])
    {
        d.selected = Some(id);
        d.expand_selected = true;
    }
}

fn material_density(m: ProfileMaterial) -> f64 {
    let mut probe = cc_core::Material::new("", [0, 0, 0]);
    cc_core::material::template_properties(&mut probe, m.material_class());
    probe.physical.density.unwrap_or(0.0)
}

fn catalog_preview_json(series: &Series, row: &CatalogRow) -> Option<Value> {
    let def = profile_catalog::row_to_def(series, row)?;
    let pt = |p: DVec2| [p.x, p.y];
    let params: serde_json::Map<String, Value> =
        def.shape.params().into_iter().map(|(n, v)| (n, serde_json::json!(v))).collect();
    let mut out = serde_json::json!({
        "name": def.name,
        "material": def.material.code(),
        "shape": def.shape.code(),
        "params": Value::Object(params),
        "used_by": [],
    });
    if let (Some(o), Ok(outline)) = (out.as_object_mut(), def.outline()) {
        let holes: Vec<Vec<[f64; 2]>> = outline.holes.iter().map(|h| h.iter().copied().map(pt).collect()).collect();
        o.insert(
            "outline".into(),
            serde_json::json!({ "outer": outline.outer.iter().copied().map(pt).collect::<Vec<_>>(), "holes": holes }),
        );
        o.insert("properties".into(), serde_json::json!(outline.properties(material_density(def.material))));
    }
    Some(out)
}

fn ensure_catalog_preview(d: &mut ProfilesDialog) {
    let Some(sel) = &d.catalog_selected else {
        d.catalog_preview = None;
        return;
    };
    if d.catalog_preview.as_ref().is_some_and(|(s, _)| s == sel) {
        return;
    }
    d.catalog_preview = profile_catalog::series(&sel.series)
        .and_then(|s| s.rows.iter().find(|r| r.designation == sel.designation).map(|r| (s, r)))
        .and_then(|(s, r)| catalog_preview_json(s, r))
        .map(|v| (sel.clone(), v));
}

fn catalog_details(ui: &mut Ui, _state: &mut AppState, d: &mut ProfilesDialog) {
    let Some((sel, detail)) = d.catalog_preview.clone() else { return };
    table::section(ui, "profiles-catalog-params", t!("profiles-params"), |ui| {
        table::readonly_row(ui, &t!("prop-name"), sel.designation.clone());
        let material = detail["material"].as_str().and_then(ProfileMaterial::from_code).unwrap_or_default();
        table::readonly_row(ui, &t!("prop-material"), tr(material.key()));
        let shape = detail["shape"].as_str().unwrap_or_default();
        table::readonly_row(ui, &t!("prop-shape"), shape_label(shape));
        for (param, value) in ordered_params(&detail) {
            let degrees = param == "angle";
            let shown = if degrees { value } else { value * 1000.0 };
            let suffix = if degrees { "°" } else { " mm" };
            let tip = crate::i18n::try_tr(&param_key(&param));
            table::row_tip(ui, &param, tip.as_deref(), |ui| {
                ui.label(format!("{} {suffix}", number(shown)));
            });
        }
    });
    if let Some(e) = &d.error {
        ui.label(RichText::new(e).color(PALETTE.danger));
    }
    ui.add_space(6.0);
    let p = &detail["properties"];
    let get = |k: &str| p[k].as_f64().unwrap_or(f64::NAN);
    table::section(ui, "profiles-catalog-properties", t!("profiles-properties"), |ui| {
        let rows = [
            ("profile-prop-area", get("area") * 1e4, "cm²"),
            ("profile-prop-mass", get("linear_mass"), "kg/m"),
            ("profile-prop-ix", get("ix") * 1e8, "cm⁴"),
            ("profile-prop-iy", get("iy") * 1e8, "cm⁴"),
            ("profile-prop-wx", get("wx") * 1e6, "cm³"),
            ("profile-prop-wy", get("wy") * 1e6, "cm³"),
            ("profile-prop-rx", get("rx") * 1e2, "cm"),
            ("profile-prop-ry", get("ry") * 1e2, "cm"),
            ("profile-prop-j", get("j") * 1e8, "cm⁴"),
        ];
        for (label, value, unit) in rows {
            table::readonly_row(ui, &tr(label), format!("{} {unit}", number(value)));
        }
    });
}

fn buttons(ui: &mut Ui, state: &mut AppState, d: &mut ProfilesDialog) {
    let selected = d.selected;
    let users = selected.and_then(|id| d.cache.list.iter().find(|r| r.id == id)).map_or(0, |r| r.users);
    ui.horizontal_wrapped(|ui| {
        ui.menu_button(t!("profiles-new"), |ui| {
            for code in ProfileShape::CODES {
                if ui.button(shape_label(code)).clicked() {
                    let command = cmd::ProfileCreate {
                        material: material_for(code).code().into(),
                        shape: Some(code.to_string()),
                        ..Default::default()
                    };
                    if let Some(out) = run(state, &mut d.error, None, command) {
                        d.selected = profile_id(&out);
                        d.expand_selected = true;
                    }
                    ui.close();
                }
            }
        });
        if ui.add_enabled(selected.is_some(), egui::Button::new(t!("profiles-duplicate"))).clicked()
            && let Some(id) = selected
            && let Some(out) =
                run(state, &mut d.error, None, cmd::ProfileDuplicate { profile: Ref::Id(id.0), name: None })
        {
            d.selected = profile_id(&out);
            d.expand_selected = true;
        }
        let can_delete = selected.is_some() && (users == 0 || d.replace_with.is_some());
        if ui.add_enabled(can_delete, egui::Button::new(t!("profiles-delete"))).clicked()
            && let Some(id) = selected
        {
            let replace_with = if users > 0 { d.replace_with.map(|w| Ref::Id(w.0)) } else { None };
            if run(state, &mut d.error, None, cmd::ProfileDelete { profile: Ref::Id(id.0), replace_with }).is_some() {
                d.selected = None;
            }
        }
        if ui.button(t!("profiles-import")).clicked() {
            open_catalog(state, d);
        }
    });
    if users > 0 {
        ui.horizontal(|ui| {
            ui.label(theme::muted(t!("profiles-replace-with")));
            let name = |id: Option<ProfileId>| {
                id.and_then(|id| d.cache.list.iter().find(|r| r.id == id)).map_or(String::new(), |r| r.name.clone())
            };
            let mut choice = d.replace_with;
            egui::ComboBox::from_id_salt("profiles-replace").selected_text(name(choice)).width(140.0).show_ui(
                ui,
                |ui| {
                    for r in d.cache.list.iter().filter(|r| Some(r.id) != selected) {
                        ui.selectable_value(&mut choice, Some(r.id), &r.name);
                    }
                },
            );
            d.replace_with = choice;
        });
    }
}

// ---------------------------------------------------------------- the selected profile

/// The parameters in the order the shape declares them (a sketch's as listed).
fn ordered_params(detail: &Value) -> Vec<(String, f64)> {
    let params = detail["params"].as_object();
    let values = |name: &str| params.and_then(|p| p.get(name)).and_then(Value::as_f64);
    match detail["shape"].as_str().and_then(ProfileShape::default_for) {
        Some(shape) if !matches!(shape, ProfileShape::Sketch(_)) => {
            shape.params().into_iter().filter_map(|(n, _)| Some((n.clone(), values(&n)?))).collect()
        }
        _ => params.into_iter().flatten().filter_map(|(n, v)| Some((n.clone(), v.as_f64()?))).collect(),
    }
}

/// A value with about four significant digits.
/// A profile's section properties as shown (label, value, unit), from
/// `get` (SI values): cm², cm⁴, cm³, cm and kg/m in metric projects; in²,
/// in⁴, in³, in and lb/ft in imperial ones.
fn section_rows(
    get: &dyn Fn(&str) -> f64,
    system: cc_core::MeasurementSystem,
) -> [(&'static str, f64, &'static str); 9] {
    let mass = cc_views::format::Quantity::LinearMass;
    let linear_mass = get("linear_mass") * mass.per_si(system);
    if system == cc_core::MeasurementSystem::Metric {
        return [
            ("profile-prop-area", get("area") * 1e4, "cm²"),
            ("profile-prop-mass", linear_mass, "kg/m"),
            ("profile-prop-ix", get("ix") * 1e8, "cm⁴"),
            ("profile-prop-iy", get("iy") * 1e8, "cm⁴"),
            ("profile-prop-wx", get("wx") * 1e6, "cm³"),
            ("profile-prop-wy", get("wy") * 1e6, "cm³"),
            ("profile-prop-rx", get("rx") * 1e2, "cm"),
            ("profile-prop-ry", get("ry") * 1e2, "cm"),
            ("profile-prop-j", get("j") * 1e8, "cm⁴"),
        ];
    }
    let inch = cc_core::units::INCH;
    [
        ("profile-prop-area", get("area") / inch.powi(2), "in²"),
        ("profile-prop-mass", linear_mass, "lb/ft"),
        ("profile-prop-ix", get("ix") / inch.powi(4), "in⁴"),
        ("profile-prop-iy", get("iy") / inch.powi(4), "in⁴"),
        ("profile-prop-wx", get("wx") / inch.powi(3), "in³"),
        ("profile-prop-wy", get("wy") / inch.powi(3), "in³"),
        ("profile-prop-rx", get("rx") / inch, "in"),
        ("profile-prop-ry", get("ry") / inch, "in"),
        ("profile-prop-j", get("j") / inch.powi(4), "in⁴"),
    ]
}

fn number(v: f64) -> String {
    let a = v.abs();
    if !v.is_finite() {
        "—".into()
    } else if a >= 1000.0 {
        format!("{v:.0}")
    } else if a >= 100.0 {
        format!("{v:.1}")
    } else if a >= 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.4}")
    }
}

fn details(ui: &mut Ui, state: &mut AppState, d: &mut ProfilesDialog) {
    let Some((id, detail)) = d.cache.detail.clone() else { return };
    let profile = || Ref::Id(id.0);
    let mut failed = false;
    table::section(ui, "profiles-params", t!("profiles-params"), |ui| {
        let name = detail["name"].as_str().unwrap_or_default().to_string();
        let typed =
            table::row(ui, &t!("prop-name"), |ui| table::commit_text(ui, ("profile-name", id), &name, 200.0, ""));
        if let Some(text) = typed {
            let command = cmd::ProfileSet { profile: profile(), name: Some(text), ..Default::default() };
            failed |= run(state, &mut d.error, None, command).is_none();
        }
        let material = detail["material"].as_str().and_then(ProfileMaterial::from_code).unwrap_or_default();
        let mut choice = material;
        table::row(ui, &t!("prop-material"), |ui| {
            egui::ComboBox::from_id_salt(("profile-material", id)).selected_text(tr(material.key())).show_ui(
                ui,
                |ui| {
                    for m in ProfileMaterial::ALL {
                        ui.selectable_value(&mut choice, m, tr(m.key()));
                    }
                },
            );
        });
        if choice != material {
            let command =
                cmd::ProfileSet { profile: profile(), material: Some(choice.code().into()), ..Default::default() };
            failed |= run(state, &mut d.error, None, command).is_none();
        }
        let shape = detail["shape"].as_str().unwrap_or_default();
        table::readonly_row(ui, &t!("prop-shape"), shape_label(shape));
        for (param, value) in ordered_params(&detail) {
            // Lengths in millimeters (inches in imperial projects); the Z
            // section's lip angle in degrees.
            let degrees = param == "angle";
            let inches = !degrees && cc_views::format::imperial();
            let mut shown = if degrees || inches { value } else { value * 1000.0 };
            // The symbol stays the label; what it measures is its tooltip.
            let tip = crate::i18n::try_tr(&param_key(&param));
            let r = table::row_tip(ui, &param, tip.as_deref(), |ui| {
                if inches {
                    return ui.add(crate::properties::size_drag(&mut shown).range(0.0..=f64::MAX));
                }
                let suffix = if degrees { "°" } else { " mm" };
                ui.add(egui::DragValue::new(&mut shown).speed(0.5).range(0.0..=f64::MAX).max_decimals(2).suffix(suffix))
            });
            if r.changed() {
                let v = if degrees || inches { shown } else { shown / 1000.0 };
                let command = cmd::ProfileSet {
                    profile: profile(),
                    params: Some(BTreeMap::from([(param.clone(), v)])),
                    ..Default::default()
                };
                failed |= run(state, &mut d.error, Some(key(("profile-param", id, &param))), command).is_none();
            }
            if r.drag_stopped() || r.lost_focus() {
                state.session.doc.break_merge();
            }
        }
    });
    if let Some(e) = &d.error {
        ui.label(RichText::new(e).color(PALETTE.danger));
    }
    ui.add_space(6.0);
    let p = &detail["properties"];
    let get = |k: &str| p[k].as_f64().unwrap_or(f64::NAN);
    table::section(ui, "profiles-properties", t!("profiles-properties"), |ui| {
        let rows = section_rows(&get, cc_views::format::system());
        for (label, value, unit) in rows {
            table::readonly_row(ui, &tr(label), format!("{} {unit}", number(value)));
        }
        let users: Vec<&str> = detail["used_by"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let users = if users.is_empty() { "—".to_string() } else { users.join(", ") };
        table::readonly_row(ui, &t!("profiles-used-by"), users);
    });
    if detail["shape"] == "sketch" {
        ui.add_space(6.0);
        // The sketch takes the central area, so it opens from the manager
        // alone (not over Type Properties).
        let free = d.edits.owns();
        let button = ui
            .add_enabled(free, egui::Button::new(t!("profiles-edit-sketch")))
            .on_disabled_hover_text(t!("profiles-edit-sketch-nested"));
        if button.clicked() {
            d.edit_sketch = true;
        }
    }
    if failed {
        // Read the profile back, so a refused value shows what it is.
        d.cache.revision = None;
    }
}

/// The outline from `profile_get`, scaled to fit with a 10% margin: filled
/// with the soft tint, holes left open, the centroid's cross and axes, and
/// the overall depth and width.
fn preview(ui: &mut Ui, detail: &Value) {
    let palette = theme::palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(PREVIEW, PREVIEW), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, palette.canvas);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
    let points = |v: &Value| -> Vec<DVec2> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| Some(DVec2::new(p[0].as_f64()?, p[1].as_f64()?)))
            .filter(|p| p.is_finite())
            .collect()
    };
    let outer = points(&detail["outline"]["outer"]);
    let holes: Vec<Vec<DVec2>> =
        detail["outline"]["holes"].as_array().into_iter().flatten().map(points).filter(|h| h.len() >= 3).collect();
    if outer.len() < 3 {
        return;
    }
    let (min, max) = outer.iter().fold((outer[0], outer[0]), |(a, b), p| (a.min(*p), b.max(*p)));
    let size = max - min;
    let area = rect.shrink(rect.width() * 0.1);
    let scale = (area.width() as f64 / size.x).min(area.height() as f64 / size.y);
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let mid = (min + max) * 0.5;
    let to = |p: DVec2| {
        let c = area.center();
        Pos2::new(c.x + ((p.x - mid.x) * scale) as f32, c.y - ((p.y - mid.y) * scale) as f32)
    };
    let mut mesh = egui::Mesh::default();
    for tri in cc_geom::polygon::triangulate_with_holes(&outer, &holes) {
        let base = mesh.vertices.len() as u32;
        for p in tri {
            mesh.colored_vertex(to(p), palette.accent_soft);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    painter.add(Shape::mesh(mesh));
    let edge = Stroke::new(1.5, palette.text);
    for ring in std::iter::once(&outer).chain(&holes) {
        painter.add(Shape::closed_line(ring.iter().map(|p| to(*p)).collect(), edge));
    }
    // The centroid: dashed axes across the drawing and a cross.
    let c = &detail["properties"]["centroid"];
    if let (Some(x), Some(y)) = (c[0].as_f64(), c[1].as_f64()) {
        let at = to(DVec2::new(x, y));
        let axis = Stroke::new(1.0, palette.text_faint);
        for line in [
            [Pos2::new(rect.left() + 6.0, at.y), Pos2::new(rect.right() - 6.0, at.y)],
            [Pos2::new(at.x, rect.top() + 6.0), Pos2::new(at.x, rect.bottom() - 6.0)],
        ] {
            painter.extend(Shape::dashed_line(&line, axis, 5.0, 3.0));
        }
        let cross = Stroke::new(1.5, palette.accent);
        painter.line_segment([at - vec2(5.0, 5.0), at + vec2(5.0, 5.0)], cross);
        painter.line_segment([at - vec2(5.0, -5.0), at + vec2(5.0, -5.0)], cross);
    }
    let font = egui::FontId::proportional(11.0);
    let mm = |v: f64| {
        if cc_views::format::imperial() { cc_views::format::inch_size(v) } else { format!("{} mm", number(v * 1000.0)) }
    };
    let bottom = Pos2::new(rect.center().x, rect.bottom() - 4.0);
    painter.text(bottom, egui::Align2::CENTER_BOTTOM, format!("bf = {}", mm(size.x)), font.clone(), palette.text_muted);
    let left = Pos2::new(rect.left() + 6.0, rect.top() + 4.0);
    painter.text(left, egui::Align2::LEFT_TOP, format!("d = {}", mm(size.y)), font, palette.text_muted);
}

// ---------------------------------------------------------------- the catalog

fn open_catalog(state: &mut AppState, d: &mut ProfilesDialog) {
    let out = state.session.run(cmd::ProfileCatalog::default()).unwrap_or_default();
    let series_list: Vec<CatalogSeries> = out
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let code = s["code"].as_str()?.to_string();
            let name = s["name"].as_str().map_or_else(|| code.clone(), str::to_string);
            let shape = s["shape"].as_str().unwrap_or_default().to_string();
            let library = s["library"].as_str().unwrap_or_default().to_string();
            Some(CatalogSeries { code, name, shape, library })
        })
        .collect();
    let default_lib = "brazil".to_string();
    let first =
        series_list.iter().find(|s| s.library == default_lib).or_else(|| series_list.first()).map(|s| s.code.clone());
    let Some(first) = first else { return };
    d.catalog = Some(Catalog {
        library: default_lib,
        series: first,
        filter: String::new(),
        picked: BTreeSet::new(),
        series_list,
        rows: None,
        error: None,
    });
}

/// The designations of the catalog's series that pass its filter, read again
/// when either changes (a new series also clears what was picked).
fn catalog_rows(state: &mut AppState, c: &mut Catalog) {
    let current = c.rows.as_ref().map(|(s, f, _)| (s.as_str(), f.as_str()));
    if current == Some((c.series.as_str(), c.filter.as_str())) {
        return;
    }
    if c.rows.as_ref().is_some_and(|(s, _, _)| *s != c.series) {
        c.picked.clear();
    }
    let search: String = c.filter.chars().take(MAX_NAME).collect();
    let command = cmd::ProfileCatalog { series: Some(c.series.clone()), search: Some(search) };
    let out = state.session.run(command).unwrap_or_default();
    let rows = out["rows"].as_array().into_iter().flatten().filter_map(|r| Some(r["designation"].as_str()?.into()));
    c.rows = Some((c.series.clone(), c.filter.clone(), rows.collect()));
}

fn catalog_dialog(ctx: &Context, state: &mut AppState, d: &mut ProfilesDialog) {
    let Some(mut c) = d.catalog.take() else { return };
    catalog_rows(state, &mut c);
    let nothing_picked = c.picked.is_empty();
    let mut add = 0;
    let response = Dialog::new("profiles-catalog", t!("profiles-catalog-title")).width(440.0).show(
        ctx,
        |ui| {
            ui.horizontal(|ui| {
                for (code, label) in [
                    ("brazil", "Brasil (NBR / Gerdau)".to_string()),
                    ("europe", "Europa (EN)".to_string()),
                    ("timber", tr("profile-mat-timber")),
                    ("", tr("tab-all")),
                ] {
                    if ui.selectable_label(c.library == code, label).clicked() && c.library != code {
                        c.library = code.to_string();
                        if let Some(first) =
                            c.series_list.iter().find(|s| c.library.is_empty() || s.library == c.library)
                        {
                            c.series = first.code.clone();
                        }
                    }
                }
            });
            ui.add_space(4.0);
            egui::Grid::new("profiles-catalog-options").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(theme::muted(t!("profiles-catalog-series")));
                let name = c.series_list.iter().find(|s| s.code == c.series).map(|s| s.name.clone());
                let mut series = c.series.clone();
                egui::ComboBox::from_id_salt("profiles-catalog-series")
                    .selected_text(name.unwrap_or_default())
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        let visible: Vec<&CatalogSeries> =
                            c.series_list.iter().filter(|s| c.library.is_empty() || s.library == c.library).collect();
                        let mut shapes: Vec<String> = Vec::new();
                        for s in &visible {
                            if !shapes.contains(&s.shape) {
                                shapes.push(s.shape.clone());
                            }
                        }
                        for sh in shapes {
                            let group_title = if sh.is_empty() { t!("profiles-title") } else { shape_label(&sh) };
                            ui.label(RichText::new(group_title).strong().small().color(PALETTE.text_muted));
                            for s in &visible {
                                if s.shape == sh {
                                    ui.selectable_value(&mut series, s.code.clone(), format!("  {}", s.name));
                                }
                            }
                        }
                    });
                c.series = series;
                ui.end_row();
                ui.label(theme::muted(t!("profiles-catalog-filter")));
                ui.add(egui::TextEdit::singleline(&mut c.filter).char_limit(MAX_NAME).desired_width(300.0));
                ui.end_row();
            });
            ui.add_space(4.0);
            let rows_len = c.rows.as_ref().map_or(0, |(_, _, r)| r.len());
            ui.horizontal(|ui| {
                if ui.add_enabled(rows_len > 0, egui::Button::new(t!("fp-select-all"))).clicked()
                    && let Some((_, _, rows)) = &c.rows
                {
                    c.error = None;
                    for d_name in rows {
                        c.picked.insert(d_name.clone());
                    }
                }
                if ui.add_enabled(!c.picked.is_empty(), egui::Button::new(t!("pref-clear"))).clicked() {
                    c.error = None;
                    c.picked.clear();
                }
            });
            ui.add_space(4.0);
            egui::Frame::new().stroke(Stroke::new(1.0, PALETTE.border)).inner_margin(4).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("profiles-catalog-rows").max_height(320.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let rows = c.rows.as_ref().map(|(_, _, r)| r.as_slice()).unwrap_or_default();
                    for designation in rows {
                        let mut on = c.picked.contains(designation);
                        if ui.checkbox(&mut on, designation).changed() {
                            c.error = None;
                            if on {
                                c.picked.insert(designation.clone());
                            } else {
                                c.picked.remove(designation);
                            }
                        }
                    }
                });
            });
            // Nothing picked yet, or why the import was refused.
            if let Some(e) = &c.error {
                ui.label(RichText::new(e).color(PALETTE.danger));
            } else if c.picked.is_empty() {
                ui.label(theme::muted(t!("profiles-catalog-none")));
            }
        },
        |f| {
            add = f.primary(t!("profiles-catalog-add"));
            if nothing_picked {
                f.disable(add);
            }
            f.secondary(t!("action-cancel"));
        },
    );
    let done = match response.clicked {
        Some(i) if i == add => {
            if c.picked.is_empty() {
                false
            } else {
                let command = cmd::ProfileImport {
                    series: c.series.clone(),
                    designations: c.picked.iter().cloned().collect(),
                    on_clash: None,
                };
                match run(state, &mut c.error, None, command) {
                    Some(out) => {
                        if let Some(id) = profile_id(&out["imported"][0]) {
                            d.selected = Some(id);
                            d.expand_selected = true;
                        }
                        true
                    }
                    None => false,
                }
            }
        }
        Some(_) => true,
        None => response.dismissed,
    };
    if !done {
        d.catalog = Some(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_keep_the_shape_order() {
        let detail =
            serde_json::json!({"shape": "i", "params": {"bf": 0.1, "d": 0.2, "r": 0.01, "tf": 0.008, "tw": 0.005}});
        let names: Vec<String> = ordered_params(&detail).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["d", "bf", "tw", "tf", "r"]);
    }

    #[test]
    fn shape_keys_use_hyphens() {
        assert_eq!(shape_key("cold_lipped_u"), "profile-shape-cold-lipped-u");
        assert_eq!(material_for("cold_z"), ProfileMaterial::ColdFormed);
    }

    #[test]
    fn section_properties_in_both_systems() {
        // A 1" x 1" square bar of steel.
        let inch = cc_core::units::INCH;
        let props = |k: &str| match k {
            "area" => inch * inch,
            "linear_mass" => 7850.0 * inch * inch,
            "ix" | "iy" => inch.powi(4) / 12.0,
            "wx" | "wy" => inch.powi(3) / 6.0,
            "rx" | "ry" => inch / 12f64.sqrt(),
            _ => 0.0,
        };
        let imperial = section_rows(&props, cc_core::MeasurementSystem::Imperial);
        let (_, area, unit) = imperial[0];
        assert!((area - 1.0).abs() < 1e-9 && unit == "in²");
        let (_, mass, unit) = imperial[1];
        assert_eq!((number(mass), unit), ("3.40".to_string(), "lb/ft"));
        assert!((imperial[2].1 - 1.0 / 12.0).abs() < 1e-9 && imperial[2].2 == "in⁴");
        let metric = section_rows(&props, cc_core::MeasurementSystem::Metric);
        assert_eq!((number(metric[0].1), metric[0].2), ("6.45".to_string(), "cm²"));
        assert_eq!(metric[1].2, "kg/m");
    }

    #[test]
    fn number_keeps_four_digits() {
        assert_eq!(number(28.4812), "28.48");
        assert_eq!(number(1943.2), "1943");
        assert_eq!(number(f64::NAN), "—");
    }
}
