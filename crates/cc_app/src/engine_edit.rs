//! Property edits through the engine. Panels and dialogs describe a change
//! as a function on a copy of the object; these helpers send the result to
//! the engine's `*_set` command, merged into one undo step per `key` while a
//! value is dragged. Refusals (invalid values) go to the status bar.

use crate::state::AppState;
use cc_core::{
    Element, ElementId, ElementType, ElementTypeId, Level, LevelId, Material, SheetSize, View, ViewId, ViewKind,
};
use cc_engine::Ref;
use cc_engine::commands::{edit, levels, materials, project, sheets, types, views};

fn run<C: cc_engine::Command + serde::Serialize>(state: &mut AppState, key: Option<u64>, command: C) -> bool {
    let result = match key {
        Some(k) => state.session.run_merged(k, command),
        None => state.session.run(command),
    };
    match result {
        Ok(_) => true,
        Err(e) => {
            state.message = e.message;
            false
        }
    }
}

fn json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or_default()
}

/// Change elements `ids` at once (`elements_set`): a new `level` and the
/// same `params` merged into each, in one undo step per `key`.
pub fn elements(
    state: &mut AppState,
    ids: &[ElementId],
    key: Option<u64>,
    level: Option<LevelId>,
    params: Option<serde_json::Value>,
) -> bool {
    let command = edit::ElementsSet {
        ids: ids.iter().map(|id| id.0).collect(),
        level: level.map(|l| Ref::Id(l.0)),
        params,
        mark: None,
        comments: None,
    };
    run(state, key, command)
}

/// Give elements `ids` the same mark or comments (`elements_set`), in one
/// undo step per `key`.
pub fn elements_identity(
    state: &mut AppState,
    ids: &[ElementId],
    key: Option<u64>,
    mark: Option<String>,
    comments: Option<String>,
) -> bool {
    let command =
        edit::ElementsSet { ids: ids.iter().map(|id| id.0).collect(), level: None, params: None, mark, comments };
    run(state, key, command)
}

/// Change element `id` as `f` changes a copy of it (`element_set`).
pub fn element(state: &mut AppState, id: ElementId, key: Option<u64>, f: impl FnOnce(&mut Element)) -> bool {
    let Some(old) = state.project().element(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let command = edit::ElementSet {
        id: id.0,
        level: (new.level != old.level).then_some(Ref::Id(new.level.0)),
        type_ref: (new.type_id != old.type_id).then_some(Ref::Id(new.type_id.0)),
        mark: (new.mark != old.mark).then_some(new.mark),
        comments: (new.comments != old.comments).then_some(new.comments),
        params: (new.kind != old.kind).then(|| cc_engine::element_params_patch(&old.kind, &new.kind)),
    };
    run(state, key, command)
}

/// Change type `id` as `f` changes a copy of it (`type_set`).
pub fn element_type(
    state: &mut AppState,
    id: ElementTypeId,
    key: Option<u64>,
    f: impl FnOnce(&mut ElementType),
) -> bool {
    let Some(old) = state.project().element_type(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let command = types::TypeSet {
        type_ref: Ref::Id(id.0),
        name: (new.name != old.name).then_some(new.name),
        params: (new.data != old.data).then(|| cc_engine::params_patch(&old.data, &new.data)),
        info: (new.info != old.info).then(|| json(&new.info)),
        renames: Vec::new(),
    };
    run(state, key, command)
}

/// Change view `id` as `f` changes a copy of it (`view_set`).
pub fn view(state: &mut AppState, id: ViewId, key: Option<u64>, f: impl FnOnce(&mut View)) -> bool {
    let Some(old) = state.project().view(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let crop = (new.callout_rect() != old.callout_rect() || new.crop != old.crop)
        .then(|| new.callout_rect().or(new.crop).map(|r| [[r.min.x, r.min.y], [r.max.x, r.max.y]]));
    let command = views::ViewSet {
        view: Ref::Id(id.0),
        name: (new.name != old.name).then_some(new.name),
        scale: (new.scale != old.scale).then_some(new.scale.into()),
        params: (new.kind != old.kind).then(|| cc_engine::params_patch(&old.kind, &new.kind)),
        phase: (new.phase != old.phase).then_some(Ref::Id(new.phase.0)),
        detail: (new.detail != old.detail).then(|| new.detail.name().to_string()),
        visual_style: (new.visual_style != old.visual_style)
            .then(|| new.visual_style.map_or("default", |s| s.name()).to_string()),
        phase_filter: (new.phase_filter != old.phase_filter).then(|| new.phase_filter.map(|f| Ref::Id(f.0))),
        crop,
        sun: (new.sun != old.sun)
            .then(|| new.sun.as_ref().map(|sun| cc_engine::commands::sun::sun_arg(state.project(), sun))),
    };
    run(state, key, command)
}

/// Change sheet `id` as `f` changes a copy of it (`sheet_set`): its
/// number, paper, title block, per-sheet field values and issue date, in
/// one undo step per `key`.
pub fn sheet(state: &mut AppState, id: ViewId, key: Option<u64>, f: impl FnOnce(&mut View)) -> bool {
    let Some(old) = state.project().view(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    let (
        ViewKind::Sheet { number: n0, size: s0, portrait: p0, title_block: t0, fields: f0, issue_date: d0, .. },
        ViewKind::Sheet { number: n1, size: s1, portrait: p1, title_block: t1, fields: f1, issue_date: d1, .. },
    ) = (&old.kind, &new.kind)
    else {
        return false;
    };
    let mut fields = serde_json::Map::new();
    for (name, value) in f1 {
        if f0.get(name) != Some(value) {
            fields.insert(name.clone(), serde_json::Value::String(value.clone()));
        }
    }
    for name in f0.keys().filter(|n| !f1.contains_key(*n)) {
        fields.insert(name.clone(), serde_json::Value::Null);
    }
    let size = (s1 != s0).then(|| match *s1 {
        SheetSize::Custom { width, height } => sheets::SizeArg::Custom { width, height },
        iso => sheets::SizeArg::Iso(iso.label()),
    });
    let command = sheets::SheetSet {
        sheet: Ref::Id(id.0),
        number: (n1 != n0).then(|| n1.clone()),
        size,
        portrait: (p1 != p0).then_some(*p1),
        title_block: (t1 != t0).then(|| t1.map(|t| Ref::Id(t.0))),
        fields: (!fields.is_empty()).then_some(fields),
        issue_date: (d1 != d0).then(|| d1.clone()),
    };
    let nothing = command.number.is_none()
        && command.size.is_none()
        && command.portrait.is_none()
        && command.title_block.is_none()
        && command.fields.is_none()
        && command.issue_date.is_none();
    !nothing && run(state, key, command)
}

/// Change material `name` as `f` changes a copy of it (`material_set`; renaming has its own command).
pub fn material(state: &mut AppState, name: &str, key: Option<u64>, f: impl FnOnce(&mut Material)) -> bool {
    let Some(old) = state.project().material(name).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let command = materials::MaterialSet { material: Ref::Name(name.to_string()), values: json(&new) };
    run(state, key, command)
}

/// Change the project units as `f` changes a copy of them (`units_set`).
pub fn units(state: &mut AppState, key: Option<u64>, f: impl FnOnce(&mut cc_core::Units)) -> bool {
    let old = state.project().units.clone();
    let mut new = old.clone();
    f(&mut new);
    new != old && run(state, key, project::UnitsSet { values: json(&new), preset: None })
}

/// Change the project information as `f` changes a copy of it (`project_info_set`).
pub fn info(state: &mut AppState, key: Option<u64>, f: impl FnOnce(&mut cc_core::ProjectInfo)) -> bool {
    let old = state.project().info.clone();
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let changed = |a: &String, b: &String| (a != b).then(|| b.clone());
    let command = project::ProjectInfoSet {
        name: changed(&old.name, &new.name),
        number: changed(&old.number, &new.number),
        client: changed(&old.client, &new.client),
        address: changed(&old.address, &new.address),
        author: changed(&old.author, &new.author),
        fields: field_changes(&old, &new),
    };
    run(state, key, command)
}

/// Change plumbing system or circuit `id` as `f` changes a copy (`system_set`).
pub fn system(
    state: &mut AppState,
    id: cc_core::SystemId,
    key: Option<u64>,
    f: impl FnOnce(&mut cc_core::MepSystem),
) -> bool {
    let Some(old) = state.project().system(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let mut params = json(&new);
    // Unset optional values are left out of the model's form; the patch clears them.
    if let Some(o) = params.as_object_mut() {
        for field in ["panel", "color"] {
            o.entry(field).or_insert(serde_json::Value::Null);
        }
    }
    let command = cc_engine::commands::mep::SystemSet { system: Ref::Id(id.0), params };
    run(state, key, command)
}

/// Rename a level or change its elevation as `f` changes a copy (`level_set`).
pub fn level(state: &mut AppState, id: LevelId, key: Option<u64>, f: impl FnOnce(&mut Level)) -> bool {
    let Some(old) = state.project().level(id).cloned() else { return false };
    let mut new = old.clone();
    f(&mut new);
    if new == old {
        return false;
    }
    let command = levels::LevelSet {
        level: Ref::Id(id.0),
        name: (new.name != old.name).then_some(new.name),
        elevation: (new.elevation != old.elevation).then_some(new.elevation),
        height: None,
    };
    run(state, key, command)
}

/// Set the height from a level up to the next one; the levels above move along (`level_set`).
pub fn level_height(state: &mut AppState, id: LevelId, key: Option<u64>, height: f64) -> bool {
    let command = levels::LevelSet { level: Ref::Id(id.0), name: None, elevation: None, height: Some(height) };
    run(state, key, command)
}

/// The `fields` argument of `project_info_set` turning `old`'s project fields into `new`'s.
fn field_changes(
    old: &cc_core::ProjectInfo,
    new: &cc_core::ProjectInfo,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut map = serde_json::Map::new();
    for f in &new.fields {
        if old.field(&f.name) != Some(f.value.as_str()) {
            map.insert(f.name.clone(), serde_json::Value::String(f.value.clone()));
        }
    }
    for f in &old.fields {
        if new.field(&f.name).is_none() {
            map.insert(f.name.clone(), serde_json::Value::Null);
        }
    }
    (!map.is_empty()).then_some(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::{Document, SheetSize, ViewKind};

    fn sheet_state() -> (AppState, ViewId) {
        let state = AppState::new(Document::new(crate::actions::new_project()));
        let id =
            state.project().views.values().find(|v| matches!(v.kind, ViewKind::Sheet { .. })).map(|v| v.id).unwrap();
        (state, id)
    }

    fn kind(state: &AppState, id: ViewId) -> ViewKind {
        state.project().view(id).unwrap().kind.clone()
    }

    /// Issue #235: the properties palette and the template manager send the
    /// underlay through their own `view_set` / `template_set` patch (not
    /// [`view`]); setting a plan back to "None" there must clear it too, one
    /// plan at a time and both at once.
    #[test]
    fn the_palette_and_templates_set_an_underlay_back_to_none() {
        use crate::properties::underlay_patch;
        use cc_engine::commands::{graphics, views};
        let mut state = AppState::new(Document::new(crate::actions::new_project()));
        let plan = state.project().views.values().find(|v| v.is_plan()).map(|v| v.id).unwrap();
        let underlay = |state: &AppState| *kind(state, plan).underlay().unwrap();
        let set_view = |state: &mut AppState, new: cc_core::Underlay| {
            let old = underlay(state);
            let params = serde_json::json!({ "underlay": underlay_patch(&old, &new) });
            let command = views::ViewSet { view: Ref::Id(plan.0), params: Some(params), ..Default::default() };
            assert!(crate::actions::run_command(state, command).is_some(), "{}", state.message);
        };
        let plan_below = cc_core::UnderlayPlan::default();
        let both = cc_core::Underlay { below: Some(plan_below), above: Some(plan_below), printed: true };
        set_view(&mut state, both);
        assert_eq!(underlay(&state), both);
        set_view(&mut state, cc_core::Underlay { below: None, ..both });
        assert_eq!(underlay(&state), cc_core::Underlay { below: None, ..both });
        set_view(&mut state, cc_core::Underlay::default());
        assert!(underlay(&state).is_empty(), "{:?}", underlay(&state));

        // A template's underlay, from the template manager.
        let out = state.session.execute("template_create", serde_json::json!({ "name": "T" })).unwrap();
        let template = out["id"].as_u64().unwrap();
        let template_underlay = |state: &AppState| {
            state.project().templates.values().find(|t| t.id.0 == template).map(|t| t.underlay).unwrap()
        };
        let set_template = |state: &mut AppState, new: cc_core::Underlay| {
            let old = template_underlay(state);
            let command = graphics::TemplateSet {
                template: Ref::Id(template),
                underlay: Some(underlay_patch(&old, &new)),
                ..Default::default()
            };
            assert!(crate::actions::run_command(state, command).is_some(), "{}", state.message);
        };
        set_template(&mut state, both);
        assert_eq!(template_underlay(&state), both);
        set_template(&mut state, cc_core::Underlay { above: None, ..both });
        assert_eq!(template_underlay(&state).above, None);
        set_template(&mut state, cc_core::Underlay::default());
        assert!(template_underlay(&state).is_empty());
    }

    /// Issue #225: a plan's underlay set to "Next level below" could not be
    /// set back to "None" (the patch left the cleared field out, and the
    /// merge kept the old value). The same goes for any field that returns
    /// to its empty value.
    #[test]
    fn a_property_set_back_to_none_is_cleared() {
        let mut state = AppState::new(Document::new(crate::actions::new_project()));
        let plan = state.project().views.values().find(|v| v.is_plan()).map(|v| v.id).unwrap();
        let below = cc_core::UnderlayPlan::default();
        assert!(view(&mut state, plan, None, |v| {
            if let Some(u) = v.kind.underlay_mut() {
                (u.below, u.above, u.printed) = (Some(below), Some(below), true);
            }
        }));
        assert_eq!(kind(&state, plan).underlay().map(|u| (u.below, u.printed)), Some((Some(below), true)));
        // One back to none, then the whole underlay.
        assert!(view(&mut state, plan, None, |v| {
            if let Some(u) = v.kind.underlay_mut() {
                u.below = None;
            }
        }));
        assert_eq!(kind(&state, plan).underlay().map(|u| (u.below, u.above)), Some((None, Some(below))));
        assert!(view(&mut state, plan, None, |v| {
            if let Some(u) = v.kind.underlay_mut() {
                *u = Default::default();
            }
        }));
        assert!(kind(&state, plan).underlay().is_some_and(|u| u.is_empty()), "{:?}", kind(&state, plan));
        assert!(state.message.is_empty(), "{}", state.message);
    }

    #[test]
    fn sheet_changes_run_one_sheet_set() {
        let (mut state, id) = sheet_state();
        let depth = state.session.doc.undo_depth();
        let changed = sheet(&mut state, id, None, |v| {
            if let ViewKind::Sheet { size, portrait, issue_date, fields, number, .. } = &mut v.kind {
                *size = SheetSize::Custom { width: 1000.0, height: 594.0 };
                *portrait = true;
                *issue_date = "2026-11-06".into();
                *number = "B7".into();
                fields.insert("Drawn".into(), "Ana".into());
            }
        });
        assert!(changed, "{}", state.message);
        assert_eq!(state.session.doc.undo_depth(), depth + 1, "one undo step");
        let ViewKind::Sheet { size, portrait, issue_date, fields, number, title_block, .. } = kind(&state, id) else {
            panic!()
        };
        assert_eq!(size, SheetSize::Custom { width: 1000.0, height: 594.0 });
        assert!(portrait);
        assert_eq!((issue_date.as_str(), number.as_str()), ("2026-11-06", "B7"));
        assert_eq!(fields.get("Drawn").map(String::as_str), Some("Ana"));
        assert!(title_block.is_some(), "the template's sheet has the standard title block");

        // Clearing a field and the title block; ISO sizes go back as their names.
        assert!(sheet(&mut state, id, None, |v| {
            if let ViewKind::Sheet { fields, title_block, size, .. } = &mut v.kind {
                fields.clear();
                *title_block = None;
                *size = SheetSize::A1;
            }
        }));
        let ViewKind::Sheet { fields, title_block, size, .. } = kind(&state, id) else { panic!() };
        assert!(fields.is_empty() && title_block.is_none());
        assert_eq!(size, SheetSize::A1);

        // Nothing changed: no command. A refused value: false, and a message.
        assert!(!sheet(&mut state, id, None, |_| {}));
        state.message.clear();
        assert!(!sheet(&mut state, id, None, |v| {
            if let ViewKind::Sheet { issue_date, .. } = &mut v.kind {
                *issue_date = "31/02/2026".into();
            }
        }));
        assert!(!state.message.is_empty());
    }
}
