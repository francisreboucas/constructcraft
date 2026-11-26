use crate::{
    Element, ElementId, ElementType, ElementTypeId, Level, Material, Project, ProjectInfo, View, ViewId, ViewKind,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Before/after state of one keyed object; `None` means absent.
type Change<T> = (Option<T>, Option<T>);

/// Project-wide settings tracked as a single unit.
#[derive(Debug, Clone, PartialEq)]
struct Settings {
    info: ProjectInfo,
    levels: Vec<Level>,
    materials: Vec<Material>,
    units: crate::Units,
    pinned: std::collections::BTreeSet<ElementId>,
    joins: std::collections::BTreeSet<(ElementId, ElementId)>,
    groups: BTreeMap<crate::GroupId, crate::Group>,
    systems: BTreeMap<crate::SystemId, crate::MepSystem>,
    filters: BTreeMap<crate::FilterId, crate::ViewFilter>,
    templates: BTreeMap<crate::TemplateId, crate::ViewTemplate>,
    phases: Vec<crate::Phase>,
    phase_filters: BTreeMap<crate::PhaseFilterId, crate::PhaseFilter>,
    phase_graphics: crate::PhaseGraphics,
    graph_bindings: BTreeMap<ElementId, crate::graph_binding::GraphBinding>,
    line_styles: crate::detail::LineStyles,
    fill_patterns: crate::fill_pattern::FillPatterns,
    profiles: BTreeMap<crate::ProfileId, crate::structural_profile::ProfileDef>,
    pipe_series: BTreeMap<crate::PipeSeriesId, crate::PipeSeries>,
    option_sets: Vec<crate::OptionSet>,
    browser: crate::browser::BrowserOrganization,
    bar_marking: crate::bar_marks::BarMarking,
    sun_presets: Vec<crate::sun::SunPreset>,
    plugin_data: crate::plugin_data::PluginData,
}

impl Settings {
    fn of(p: &Project) -> Self {
        Self {
            info: p.info.clone(),
            levels: p.levels.clone(),
            materials: p.materials.clone(),
            units: p.units.clone(),
            pinned: p.pinned.clone(),
            joins: p.joins.clone(),
            groups: p.groups.clone(),
            systems: p.systems.clone(),
            filters: p.filters.clone(),
            templates: p.templates.clone(),
            phases: p.phases.clone(),
            phase_filters: p.phase_filters.clone(),
            phase_graphics: p.phase_graphics,
            graph_bindings: p.graph_bindings.clone(),
            line_styles: p.line_styles.clone(),
            fill_patterns: p.fill_patterns.clone(),
            profiles: p.profiles.clone(),
            pipe_series: p.pipe_series.clone(),
            option_sets: p.option_sets.clone(),
            browser: p.browser.clone(),
            bar_marking: p.bar_marking.clone(),
            sun_presets: p.sun_presets.clone(),
            plugin_data: p.plugin_data.clone(),
        }
    }

    fn restore(self, p: &mut Project) {
        p.info = self.info;
        p.levels = self.levels;
        p.materials = self.materials;
        p.units = self.units;
        p.pinned = self.pinned;
        p.joins = self.joins;
        p.groups = self.groups;
        p.systems = self.systems;
        p.filters = self.filters;
        p.templates = self.templates;
        p.phases = self.phases;
        p.phase_filters = self.phase_filters;
        p.phase_graphics = self.phase_graphics;
        p.graph_bindings = self.graph_bindings;
        p.line_styles = self.line_styles;
        p.fill_patterns = self.fill_patterns;
        p.profiles = self.profiles;
        p.pipe_series = self.pipe_series;
        p.option_sets = self.option_sets;
        p.browser = self.browser;
        p.bar_marking = self.bar_marking;
        p.sun_presets = self.sun_presets;
        p.plugin_data = self.plugin_data;
    }
}

/// A recorded, reversible set of changes.
#[derive(Debug, Clone, Default)]
pub struct Transaction {
    pub name: String,
    elements: BTreeMap<ElementId, Change<Element>>,
    views: BTreeMap<ViewId, Change<View>>,
    types: BTreeMap<ElementTypeId, Change<ElementType>>,
    settings: Option<Change<Settings>>,
    /// Identity of the step in the undo history (see [`Document::undo_top_id`]).
    serial: u64,
}

fn put<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, value: Option<V>) {
    match value {
        Some(v) => {
            map.insert(key, v);
        }
        None => {
            map.remove(&key);
        }
    }
}

impl Transaction {
    /// Fold a newer transaction into this one, keeping the oldest "before" states.
    fn merge(&mut self, newer: Transaction) {
        fn fold<K: Ord, V>(into: &mut BTreeMap<K, Change<V>>, from: BTreeMap<K, Change<V>>) {
            for (k, (before, after)) in from {
                match into.get_mut(&k) {
                    Some(c) => c.1 = after,
                    None => {
                        into.insert(k, (before, after));
                    }
                }
            }
        }
        fold(&mut self.elements, newer.elements);
        fold(&mut self.views, newer.views);
        fold(&mut self.types, newer.types);
        if let Some((before, after)) = newer.settings {
            match &mut self.settings {
                Some(c) => c.1 = after,
                None => self.settings = Some((before, after)),
            }
        }
    }

    /// Drop the changes whose "after" state equals their "before" (an
    /// element added and removed again in merged steps).
    fn prune(&mut self) {
        self.elements.retain(|_, c| c.0 != c.1);
        self.views.retain(|_, c| c.0 != c.1);
        self.types.retain(|_, c| c.0 != c.1);
        if self.settings.as_ref().is_some_and(|c| c.0 == c.1) {
            self.settings = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() && self.views.is_empty() && self.types.is_empty() && self.settings.is_none()
    }

    /// Element ids touched by this transaction.
    pub fn changed_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.elements.keys().copied()
    }

    fn apply(&self, p: &mut Project, forward: bool) {
        fn pick<T: Clone>(c: &Change<T>, forward: bool) -> Option<T> {
            if forward { c.1.clone() } else { c.0.clone() }
        }
        for (id, c) in &self.elements {
            put(&mut p.elements, *id, pick(c, forward));
        }
        for (id, c) in &self.views {
            put(&mut p.views, *id, pick(c, forward));
        }
        for (id, c) in &self.types {
            put(&mut p.types, *id, pick(c, forward));
        }
        if let Some(c) = &self.settings
            && let Some(s) = pick(c, forward)
        {
            s.restore(p);
        }
    }
}

/// Mutable access to a project inside [`Document::edit`]; records the
/// original state of everything it touches.
pub struct EditCtx<'a> {
    project: &'a mut Project,
    tx: Transaction,
    /// The phase new elements are built in (else the last one).
    creation_phase: Option<crate::PhaseId>,
}

impl<'a> EditCtx<'a> {
    /// Whether this edit has changed element `id` so far (added, removed or changed).
    pub(crate) fn has_changed(&self, id: ElementId) -> bool {
        self.tx.elements.get(&id).is_some_and(|c| c.0.as_ref() != self.project.elements.get(&id))
    }

    pub fn project(&self) -> &Project {
        self.project
    }

    pub fn alloc_id(&mut self) -> u64 {
        self.project.alloc_id()
    }

    /// The elements this edit has changed so far, each with its state before
    /// the edit (`None`: it was added). Elements touched but left as they
    /// were are skipped.
    pub(crate) fn element_changes(&self) -> Vec<(ElementId, Option<&Element>)> {
        self.tx
            .elements
            .iter()
            .filter(|(id, c)| c.0.as_ref() != self.project.elements.get(id))
            .map(|(id, c)| (*id, c.0.as_ref()))
            .collect()
    }

    /// The types this edit has added, removed or changed so far.
    pub(crate) fn changed_types(&self) -> std::collections::BTreeSet<ElementTypeId> {
        self.tx.types.iter().filter(|(id, c)| c.0.as_ref() != self.project.types.get(id)).map(|(id, _)| *id).collect()
    }

    fn touch_element(&mut self, id: ElementId) {
        if !self.tx.elements.contains_key(&id) {
            let before = self.project.elements.get(&id).cloned();
            self.tx.elements.insert(id, (before, None));
        }
    }

    fn touch_view(&mut self, id: ViewId) {
        if !self.tx.views.contains_key(&id) {
            let before = self.project.views.get(&id).cloned();
            self.tx.views.insert(id, (before, None));
        }
    }

    fn touch_type(&mut self, id: ElementTypeId) {
        if !self.tx.types.contains_key(&id) {
            let before = self.project.types.get(&id).cloned();
            self.tx.types.insert(id, (before, None));
        }
    }

    /// Insert an element. Its `id` must come from [`EditCtx::alloc_id`].
    pub fn add_element(&mut self, mut element: Element) -> ElementId {
        let id = element.id;
        // Copies of an element get their own persistent id.
        if element.uid.is_nil() || self.project.elements.values().any(|e| e.uid == element.uid && e.id != id) {
            element.uid = crate::Uid::generate();
        }
        self.touch_element(id);
        self.project.elements.insert(id, element);
        id
    }

    pub fn element_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        if !self.project.elements.contains_key(&id) {
            return None;
        }
        self.touch_element(id);
        self.project.elements.get_mut(&id)
    }

    /// Remove an element and everything it hosts. Iterative with a visited set,
    /// so cyclic hosting in a crafted file cannot recurse forever.
    pub fn remove_element(&mut self, id: ElementId) {
        let mut pending = vec![id];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            pending.extend(self.project.hosted_by(id).map(|e| e.id));
            pending.extend(self.project.railings_of(id).map(|e| e.id));
            pending.extend(self.project.rebar_of(id).map(|(e, _)| e.id));
            pending.extend(self.project.bar_tags_of(id).map(|(e, _)| e.id));
            pending.extend(self.project.piles_of(id).map(|(e, _)| e.id));
            pending.extend(self.project.connections_of(id).map(|(e, _)| e.id));
            pending.extend(self.project.pads_of(id).map(|(e, _)| e.id));
            pending.extend(self.project.host_members_of(id));
            // A wall's framing goes with it (and the framing's members with the framing).
            pending.extend(self.project.framings_of(id));
            pending.extend(self.project.valves_of(id).map(|(e, _)| e.id));
            self.touch_element(id);
            self.project.elements.remove(&id);
            // A pinned element removed with its host or view takes its pin along.
            if self.project.pinned.contains(&id) {
                self.touch_settings();
                self.project.pinned.remove(&id);
            }
            if self.project.graph_bindings.contains_key(&id) {
                self.touch_settings();
                self.project.graph_bindings.remove(&id);
            }
            if self.project.joined_with(id).next().is_some() {
                self.touch_settings();
                self.project.joins.retain(|&(a, b)| a != id && b != id);
            }
            // A marker's interior elevations go with it; a room's markers stay, with no room.
            for view in self.project.interior_views_of(id) {
                self.remove_view(view);
            }
            let markers: Vec<ElementId> = self.project.markers_in_room(id).collect();
            for m in markers {
                if let Some(crate::ElementKind::InteriorMarker(marker)) = self.element_mut(m).map(|e| &mut e.kind) {
                    marker.room = None;
                }
            }
            // A board's diagrams go with it.
            let diagrams: Vec<ViewId> = self
                .project
                .views
                .values()
                .filter(|v| matches!(v.kind, ViewKind::Diagram { panel } if panel == id))
                .map(|v| v.id)
                .collect();
            for view in diagrams {
                self.remove_view(view);
            }
            if self.project.systems.values().any(|s| s.panel == Some(id)) {
                // Circuits fed by a removed board stay, unfed.
                for system in self.systems_mut().values_mut().filter(|s| s.panel == Some(id)) {
                    system.panel = None;
                }
            }
            if self.project.group_of(id).is_some() {
                self.touch_settings();
                for group in self.project.groups.values_mut() {
                    group.members.remove(&id);
                }
                self.project.groups.retain(|_, g| !g.members.is_empty());
            }
        }
    }

    pub fn add_view(&mut self, view: View) -> ViewId {
        let id = view.id;
        self.touch_view(id);
        self.project.views.insert(id, view);
        id
    }

    pub fn view_mut(&mut self, id: ViewId) -> Option<&mut View> {
        if !self.project.views.contains_key(&id) {
            return None;
        }
        self.touch_view(id);
        self.project.views.get_mut(&id)
    }

    /// Remove a view, the elements it owns (dimensions, text notes,
    /// imported drawings) and any sheet viewports showing it.
    pub fn remove_view(&mut self, id: ViewId) {
        let owned: Vec<ElementId> =
            self.project.elements.values().filter(|e| e.owner_view() == Some(id)).map(|e| e.id).collect();
        for e in owned {
            self.remove_element(e);
        }
        let sheets: Vec<_> = self
            .project
            .views
            .values()
            .filter(|v| match &v.kind {
                ViewKind::Sheet { viewports, .. } => viewports.iter().any(|vp| vp.view == id),
                _ => false,
            })
            .map(|v| v.id)
            .collect();
        for sheet in sheets {
            if let Some(ViewKind::Sheet { viewports, .. }) = self.view_mut(sheet).map(|v| &mut v.kind) {
                viewports.retain(|vp| vp.view != id);
            }
        }
        // Callouts drawn in it go; their views stay (a detail view keeps its crop).
        let details: Vec<ViewId> = self.project.callouts_in(id).map(|v| v.id).collect();
        for d in details {
            if let Some(v) = self.view_mut(d) {
                v.callout = None;
            }
        }
        self.touch_view(id);
        self.project.views.remove(&id);
    }

    pub fn add_type(&mut self, mut element_type: ElementType) -> ElementTypeId {
        let id = element_type.id;
        if element_type.uid.is_nil() || self.project.types.values().any(|t| t.uid == element_type.uid && t.id != id) {
            element_type.uid = crate::Uid::generate();
        }
        self.touch_type(id);
        self.project.types.insert(id, element_type);
        id
    }

    pub fn type_mut(&mut self, id: ElementTypeId) -> Option<&mut ElementType> {
        if !self.project.types.contains_key(&id) {
            return None;
        }
        self.touch_type(id);
        self.project.types.get_mut(&id)
    }

    /// Tag, grid and view mark types drawn as a detail component that is
    /// gone (or no longer a detail component) draw their plain symbol.
    fn sync_tag_types(&mut self) {
        for id in crate::tag::stale_symbols(&self.project.types) {
            if let Some(t) = self.type_mut(id) {
                crate::tag::reset_symbol(&mut t.data);
            }
        }
    }

    /// Remove a type (callers make sure no element uses it).
    pub fn remove_type(&mut self, id: ElementTypeId) {
        if self.project.types.contains_key(&id) {
            self.touch_type(id);
            self.project.types.remove(&id);
        }
    }

    fn touch_settings(&mut self) {
        if self.tx.settings.is_none() {
            self.tx.settings = Some((Some(Settings::of(self.project)), None));
        }
    }

    pub fn info_mut(&mut self) -> &mut ProjectInfo {
        self.touch_settings();
        &mut self.project.info
    }

    /// Levels; callers should keep them sorted by elevation.
    pub fn levels_mut(&mut self) -> &mut Vec<Level> {
        self.touch_settings();
        &mut self.project.levels
    }

    pub fn materials_mut(&mut self) -> &mut Vec<Material> {
        self.touch_settings();
        &mut self.project.materials
    }

    /// Rename a material and every type reference to it. Returns false
    /// (and changes nothing) when `new` is empty or already taken.
    pub fn rename_material(&mut self, old: &str, new: &str) -> bool {
        let new = new.trim();
        if new.is_empty() || old == new || self.project.material(new).is_some() || self.project.material(old).is_none()
        {
            return false;
        }
        for id in self.project.material_users(old) {
            if let Some(t) = self.type_mut(id) {
                t.data.rename_material(old, new);
            }
        }
        for id in self.project.material_instance_users(old) {
            if let Some(crate::ElementKind::Component(c)) = self.element_mut(id).map(|e| &mut e.kind) {
                c.text_overrides.values_mut().filter(|v| *v == old).for_each(|v| *v = new.to_string());
            }
        }
        for id in self.project.material_paint_users(old) {
            if let Some(e) = self.element_mut(id) {
                e.paints.iter_mut().filter(|p| p.material == old).for_each(|p| p.material = new.to_string());
            }
        }
        if let Some(m) = self.materials_mut().iter_mut().find(|m| m.name == old) {
            m.name = new.to_string();
        }
        true
    }

    /// Pin or unpin elements; returns how many changed.
    pub fn set_pinned(&mut self, ids: &[ElementId], pinned: bool) -> usize {
        let existing: Vec<ElementId> = ids
            .iter()
            .copied()
            .filter(|id| self.project.element(*id).is_some() && self.project.is_pinned(*id) != pinned)
            .collect();
        if existing.is_empty() {
            return 0;
        }
        self.touch_settings();
        for id in &existing {
            if pinned {
                self.project.pinned.insert(*id);
            } else {
                self.project.pinned.remove(id);
            }
        }
        existing.len()
    }

    /// Join the geometry of two elements, `cutting` keeping their common
    /// volume (an existing join between them is replaced), or unjoin them.
    /// Returns whether anything changed.
    pub fn set_joined(&mut self, cutting: ElementId, cut: ElementId, joined: bool) -> bool {
        let current = self.project.join_of(cutting, cut);
        let wanted = joined.then_some((cutting, cut));
        if current == wanted {
            return false;
        }
        self.touch_settings();
        if let Some(pair) = current {
            self.project.joins.remove(&pair);
        }
        if let Some(pair) = wanted {
            self.project.joins.insert(pair);
        }
        true
    }

    /// Group `members` (taking them out of any group they were in) under
    /// `name`; returns the new group's id.
    pub fn add_group(&mut self, name: String, members: std::collections::BTreeSet<ElementId>) -> crate::GroupId {
        self.touch_settings();
        let id = crate::GroupId(self.alloc_id());
        for group in self.project.groups.values_mut() {
            group.members.retain(|m| !members.contains(m));
        }
        self.project.groups.retain(|_, g| !g.members.is_empty());
        self.project.groups.insert(id, crate::Group { name, members });
        id
    }

    /// Dissolve a group (its elements stay); returns whether it existed.
    pub fn remove_group(&mut self, id: crate::GroupId) -> bool {
        if !self.project.groups.contains_key(&id) {
            return false;
        }
        self.touch_settings();
        self.project.groups.remove(&id);
        true
    }

    /// Plumbing systems and electrical circuits.
    pub fn systems_mut(&mut self) -> &mut BTreeMap<crate::SystemId, crate::MepSystem> {
        self.touch_settings();
        &mut self.project.systems
    }

    /// View filters.
    pub fn filters_mut(&mut self) -> &mut BTreeMap<crate::FilterId, crate::ViewFilter> {
        self.touch_settings();
        &mut self.project.filters
    }

    /// View templates (views that follow one are updated when the edit ends).
    pub fn templates_mut(&mut self) -> &mut BTreeMap<crate::TemplateId, crate::ViewTemplate> {
        self.touch_settings();
        &mut self.project.templates
    }

    /// Detail views follow their parents (see [`Project::detail_kind`]),
    /// details of details too, up to [`crate::callout::MAX_CHAIN`] links.
    fn sync_callouts(&mut self) {
        for _ in 0..crate::callout::MAX_CHAIN {
            let p = &*self.project;
            let changed: Vec<(ViewId, ViewKind)> =
                p.views.values().filter_map(|v| Some((v.id, p.detail_kind(v)?))).collect();
            if changed.is_empty() {
                break;
            }
            for (id, kind) in changed {
                if let Some(v) = self.view_mut(id) {
                    v.kind = kind;
                }
            }
        }
    }

    /// Interior elevations look from where their marker stands, the way it
    /// turns (see [`crate::interior`]).
    fn sync_interior_views(&mut self) {
        let p = &*self.project;
        let changed: Vec<(ViewId, glam::DVec2, glam::DVec2)> = p
            .views
            .values()
            .filter_map(|v| {
                let ViewKind::InteriorElevation { marker, side, origin, look, .. } = &v.kind else { return None };
                let Some(crate::ElementKind::InteriorMarker(m)) = p.element(*marker).map(|e| &e.kind) else {
                    return None;
                };
                let want = (m.position, m.look(*side));
                (want != (*origin, *look)).then_some((v.id, want.0, want.1))
            })
            .collect();
        for (id, at, dir) in changed {
            if let Some(ViewKind::InteriorElevation { origin, look, .. }) = self.view_mut(id).map(|v| &mut v.kind) {
                *origin = at;
                *look = dir;
            }
        }
    }

    /// Keep view graphics consistent with the model: drop overrides of
    /// missing elements and missing filters, unassign missing templates,
    /// and copy each template's controlled properties into its views.
    fn sync_views(&mut self) {
        // Templates drop filters that no longer exist.
        let filters = &self.project.filters;
        let stale = self
            .project
            .templates
            .values()
            .any(|t| t.graphics.filters.iter().any(|f| !filters.contains_key(&f.filter)));
        if stale {
            let filters = filters.clone();
            for t in self.templates_mut().values_mut() {
                t.graphics.filters.retain(|f| filters.contains_key(&f.filter));
            }
        }
        let elevations = self.project.elevation_ids();
        let level_ids: std::collections::BTreeSet<crate::LevelId> = self.project.levels.iter().map(|l| l.id).collect();
        let p = &*self.project;
        let changed: Vec<(ViewId, View)> = p
            .views
            .values()
            .filter(|v| {
                v.template.is_some()
                    || !v.graphics.is_empty()
                    || !v.tag_offsets.is_empty()
                    || !v.tags.is_empty()
                    || !v.fitting_placements.is_empty()
                    || v.fitting_tags
                    || !v.tag_types.is_empty()
                    || v.mark_type.is_some()
                    || !v.mark_offsets.is_empty()
                    || !v.level_heads.is_empty()
            })
            .filter_map(|v| {
                let mut new = v.clone();
                match new.template.map(|t| p.templates.get(&t)) {
                    Some(Some(t)) => t.apply(&mut new),
                    Some(None) => new.template = None,
                    None => {}
                }
                new.graphics.repair(&p.filters, Some(&|id| p.elements.contains_key(&id)));
                // Tags of deleted elements (and fittings of deleted runs) and deleted tag types go.
                crate::tag::repair_view_tags(&mut new, &p.elements, &p.types);
                // Mark types and moved markers of deleted types and views go.
                crate::view_mark::repair_view_marks(&mut new, &elevations, &p.types);
                // Heads of deleted levels go.
                crate::level_head::repair_view_level_heads(&mut new, &level_ids);
                (new != *v).then_some((v.id, new))
            })
            .collect();
        for (id, view) in changed {
            if let Some(v) = self.view_mut(id) {
                *v = view;
            }
        }
    }

    /// The project browser's schemes and the one in use.
    pub fn browser_mut(&mut self) -> &mut crate::browser::BrowserOrganization {
        self.touch_settings();
        &mut self.project.browser
    }

    /// How the project's reinforcing bars are marked.
    pub fn bar_marking_mut(&mut self) -> &mut crate::bar_marks::BarMarking {
        self.touch_settings();
        &mut self.project.bar_marking
    }

    /// The project's line styles.
    pub fn line_styles_mut(&mut self) -> &mut crate::detail::LineStyles {
        self.touch_settings();
        &mut self.project.line_styles
    }

    /// The project's sun presets (see [`crate::sun`]).
    pub fn sun_presets_mut(&mut self) -> &mut Vec<crate::sun::SunPreset> {
        self.touch_settings();
        &mut self.project.sun_presets
    }

    /// The data plugins keep on the project (see [`crate::plugin_data`]).
    pub fn plugin_data_mut(&mut self) -> &mut crate::plugin_data::PluginData {
        self.touch_settings();
        &mut self.project.plugin_data
    }

    /// Views whose sun's ground level is gone (deleted by this edit) lose it.
    fn sync_sun_ground(&mut self) {
        let p = &*self.project;
        let stale: Vec<ViewId> = p
            .views
            .values()
            .filter(|v| v.sun.as_ref().and_then(|s| s.ground).is_some_and(|l| p.level(l).is_none()))
            .map(|v| v.id)
            .collect();
        for id in stale {
            if let Some(sun) = self.view_mut(id).and_then(|v| v.sun.as_mut()) {
                sun.ground = None;
            }
        }
    }

    /// The project's fill patterns.
    pub fn fill_patterns_mut(&mut self) -> &mut crate::fill_pattern::FillPatterns {
        self.touch_settings();
        &mut self.project.fill_patterns
    }

    /// The project's structural profiles.
    pub fn profiles_mut(&mut self) -> &mut BTreeMap<crate::ProfileId, crate::structural_profile::ProfileDef> {
        self.touch_settings();
        &mut self.project.profiles
    }

    /// The project's pipe series.
    pub fn pipe_series_mut(&mut self) -> &mut BTreeMap<crate::PipeSeriesId, crate::PipeSeries> {
        self.touch_settings();
        &mut self.project.pipe_series
    }

    /// Construction phases, in time order.
    pub fn phases_mut(&mut self) -> &mut Vec<crate::Phase> {
        self.touch_settings();
        &mut self.project.phases
    }

    /// Phase filters.
    pub fn phase_filters_mut(&mut self) -> &mut BTreeMap<crate::PhaseFilterId, crate::PhaseFilter> {
        self.touch_settings();
        &mut self.project.phase_filters
    }

    /// Graphics of each phase status.
    /// Bind an element to the graph call that made it, or unbind it with `None`.
    pub fn set_graph_binding(&mut self, id: ElementId, binding: Option<crate::graph_binding::GraphBinding>) {
        if self.project.graph_bindings.get(&id) == binding.as_ref() {
            return;
        }
        self.touch_settings();
        match binding {
            Some(b) if self.project.elements.contains_key(&id) => {
                self.project.graph_bindings.insert(id, b);
            }
            _ => {
                self.project.graph_bindings.remove(&id);
            }
        }
    }

    pub fn phase_graphics_mut(&mut self) -> &mut crate::PhaseGraphics {
        self.touch_settings();
        &mut self.project.phase_graphics
    }

    /// Walls attached to an element this edit deleted (or turned into
    /// something that is no roof, ceiling or floor) keep their own top, and
    /// roofs joined to one that is no roof are unjoined
    /// ([`crate::Project::broken_roof_joins`]).
    fn sync_attachments(&mut self) {
        let p = &*self.project;
        let slab = |id: ElementId| {
            p.elements.get(&id).is_some_and(|e| {
                matches!(
                    e.kind,
                    crate::ElementKind::Floor(_) | crate::ElementKind::Ceiling(_) | crate::ElementKind::Roof(_)
                )
            })
        };
        let stale: Vec<ElementId> = p
            .elements
            .values()
            .filter(|e| matches!(&e.kind, crate::ElementKind::Wall(w) if w.attach_top.is_some_and(|t| !slab(t))))
            .map(|e| e.id)
            .collect();
        for id in stale {
            if let Some(crate::ElementKind::Wall(w)) = self.element_mut(id).map(|e| &mut e.kind) {
                w.attach_top = None;
            }
        }
        // Roofs joined to a roof this edit deleted (or to themselves, to what
        // is no roof, or round a loop) are built as drawn.
        for id in self.project.broken_roof_joins() {
            if let Some(crate::ElementKind::Roof(r)) = self.element_mut(id).map(|e| &mut e.kind) {
                r.join_to = None;
            }
        }
    }

    /// Wall profiles follow what this edit did to their walls (see
    /// [`crate::wall_profile`]): a wall whose length changed (its profile
    /// left as it was) refits it, kept where it was along the wall (a start
    /// moved along the wall's line shifts it back), cut at the ends and
    /// following an end it reached where the wall grows, dropped when
    /// nothing is left; a wall made curved or a curtain wall loses it; a
    /// sketch that no longer makes the outline goes.
    fn sync_wall_profiles(&mut self) {
        let changed: Vec<(ElementId, Option<crate::Wall>)> = self
            .element_changes()
            .into_iter()
            .map(|(id, before)| (id, before.and_then(|e| e.as_wall().cloned())))
            .collect();
        for (id, before) in changed {
            let p = &*self.project;
            let Some(e) = p.elements.get(&id) else { continue };
            let Some(w) = e.as_wall() else { continue };
            let Some(profile) = &w.profile else { continue };
            let curtain = p.element_type(e.type_id).is_some_and(|t| t.curtain().is_some());
            let fitted = if curtain || w.bulge != 0.0 || w.spline.is_some() {
                None
            } else {
                match before.filter(|b| b.profile == w.profile && b.bulge == 0.0) {
                    Some(b) if (b.length() - w.length()).abs() > 1e-9 => {
                        let (dir, moved) = (b.direction(), w.start - b.start);
                        // A start moved along the wall's own line: the profile stays where it was.
                        let along = w.direction().dot(dir) > 1.0 - 1e-9 && moved.dot(dir.perp()).abs() < 1e-6;
                        let shift = if along { moved.dot(dir) } else { 0.0 };
                        profile.refit(b.length(), shift, w.length())
                    }
                    _ => {
                        let mut kept = profile.clone();
                        kept.drop_stale_sketch();
                        Some(kept)
                    }
                }
            };
            if fitted.as_ref() == Some(profile) {
                continue;
            }
            if let Some(crate::ElementKind::Wall(w)) = self.element_mut(id).map(|e| &mut e.kind) {
                w.profile = fitted;
            }
        }
    }

    /// Spline walls (see [`crate::wall_spline`]) start and end where their
    /// curves do and have no bulge. An edit that moved a wall's ends and
    /// left its curve alone takes the curve along: the whole of it when
    /// both ends moved alike, else each end's knot with its handle.
    fn sync_wall_splines(&mut self) {
        let changed: Vec<(ElementId, Option<crate::Wall>)> = self
            .element_changes()
            .into_iter()
            .map(|(id, before)| (id, before.and_then(|e| e.as_wall().cloned())))
            .collect();
        for (id, before) in changed {
            let Some(w) = self.project.elements.get(&id).and_then(|e| e.as_wall()) else { continue };
            let Some(mut spline) = w.spline.clone() else { continue };
            if let Some(b) = before.filter(|b| b.spline.as_ref() == Some(&spline)) {
                let (at_start, at_end) = (w.start - b.start, w.end - b.end);
                if at_start.distance(at_end) < 1e-12 {
                    spline.map(|p| p + at_start);
                } else {
                    spline.move_end(true, at_start);
                    spline.move_end(false, at_end);
                }
            }
            let Some((start, end)) = spline.ends() else { continue };
            if w.spline.as_ref() == Some(&spline) && w.start == start && w.end == end && w.bulge == 0.0 {
                continue;
            }
            if let Some(crate::ElementKind::Wall(w)) = self.element_mut(id).map(|e| &mut e.kind) {
                (w.start, w.end, w.bulge) = (start, end, 0.0);
                w.spline = Some(spline);
            }
        }
    }

    /// Floors, ceilings and roofs take the cuts of the shafts that pass
    /// through them (see [`crate::shaft`]), in the same undo step as the
    /// edit that moved a shaft or them; unchanged cuts are left alone.
    fn sync_shafts(&mut self) {
        let p = &*self.project;
        let shafts = p.elements.values().any(|e| matches!(e.kind, crate::ElementKind::Shaft(_)));
        let changed: Vec<(ElementId, Vec<Vec<glam::DVec2>>)> = p
            .elements
            .values()
            .filter(|e| {
                matches!(
                    e.kind,
                    crate::ElementKind::Floor(_) | crate::ElementKind::Ceiling(_) | crate::ElementKind::Roof(_)
                )
            })
            .filter_map(|e| {
                let want = if shafts { p.shaft_cuts(e) } else { Vec::new() };
                (want.as_slice() != e.shaft_cuts()).then_some((e.id, want))
            })
            .collect();
        for (id, cuts) in changed {
            if let Some(slot) = self.element_mut(id).and_then(|e| e.shaft_cuts_mut()) {
                *slot = cuts;
            }
        }
    }

    /// Piles of pile caps stand where their caps want them, and caps have
    /// as many as their layouts hold (see [`crate::pile`]): missing ones are
    /// added, extra ones and those of no cap removed, the others moved.
    fn sync_piles(&mut self) {
        let plan = self.project.pile_sync();
        for (id, width, length) in plan.types {
            if let Some(crate::TypeData::Foundation { width: w, length: l, .. }) =
                self.type_mut(id).map(|t| &mut t.data)
            {
                (*w, *l) = (width, length);
            }
        }
        for id in plan.remove {
            self.remove_element(id);
        }
        for (id, level, type_id, pile) in plan.update {
            if let Some(e) = self.element_mut(id) {
                e.level = level;
                e.type_id = type_id;
                e.kind = crate::ElementKind::Pile(pile);
            }
        }
        // A pile the layout adds takes the reinforcement of its cap's piles: the
        // sets of the first of them (in the layout) that has any, as the piles
        // kept keep theirs and those removed take theirs along.
        let mut templates: BTreeMap<ElementId, Vec<(crate::ElementTypeId, crate::Rebar)>> = BTreeMap::new();
        for (_, _, pile) in &plan.add {
            let Some(cap) = pile.cap.map(|c| c.cap) else { continue };
            if templates.contains_key(&cap) {
                continue;
            }
            let mut piles: Vec<(u32, ElementId)> =
                self.project.piles_of(cap).map(|(e, p)| (p.cap.map_or(0, |c| c.index), e.id)).collect();
            piles.sort();
            let sets = piles
                .iter()
                .map(|(_, id)| self.project.rebar_of(*id).map(|(e, r)| (e.type_id, r.clone())).collect::<Vec<_>>())
                .find(|sets| !sets.is_empty())
                .unwrap_or_default();
            templates.insert(cap, sets);
        }
        for (level, type_id, pile) in plan.add {
            let id = ElementId(self.alloc_id());
            let mark = self.project.next_mark(crate::Category::Pile);
            let sets = pile.cap.and_then(|c| templates.get(&c.cap)).cloned().unwrap_or_default();
            self.add_element(Element {
                id,
                level,
                type_id,
                mark,
                comments: String::new(),
                kind: crate::ElementKind::Pile(pile),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            for (set_type, set) in sets {
                let set_id = ElementId(self.alloc_id());
                self.add_element(Element {
                    id: set_id,
                    level,
                    type_id: set_type,
                    mark: String::new(),
                    comments: String::new(),
                    kind: crate::ElementKind::Rebar(crate::Rebar { host: id, ..set }),
                    uid: Default::default(),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                });
            }
        }
    }

    /// Pads stand on their terrain's level (see [`crate::pad`]).
    fn sync_pads(&mut self) {
        for (id, level) in self.project.pad_levels() {
            if let Some(e) = self.element_mut(id) {
                e.level = level;
            }
        }
    }

    /// Elements (and sweep types) whose outline this edit changed without their sketch drop it
    /// (see [`crate::sketch`]).
    fn sync_sketches(&mut self) {
        let ids: Vec<ElementId> = self.tx.elements.keys().copied().collect();
        for id in ids {
            if let Some(e) = self.project.elements.get_mut(&id) {
                e.drop_stale_sketch();
            }
        }
        // Sweep types whose profile changed without its curves.
        let types: Vec<ElementTypeId> = self.tx.types.keys().copied().collect();
        for id in types {
            if let Some(crate::TypeData::Sweep { profile, sketch, .. }) =
                self.project.types.get_mut(&id).map(|t| &mut t.data)
                && sketch.as_ref().is_some_and(|s| !s.makes(profile, &[]))
            {
                *sketch = None;
            }
        }
    }

    /// Keep phases consistent: elements and views added by this edit get
    /// the creation phase (views: the last phase and the default phase
    /// filter), and hosted elements follow their hosts
    /// ([`Project::phase_fixes`]).
    fn sync_phases(&mut self) {
        let p = &*self.project;
        let Some(last) = p.last_phase() else { return };
        let creation = self.creation_phase.filter(|c| p.phase(*c).is_some()).unwrap_or(last);
        let added: Vec<ElementId> = self
            .tx
            .elements
            .iter()
            .filter(|(id, c)| c.0.is_none() && p.elements.get(id).is_some_and(|e| p.phase(e.phase_created).is_none()))
            .map(|(id, _)| *id)
            .collect();
        for id in added {
            if let Some(e) = self.project.elements.get_mut(&id) {
                e.phase_created = creation;
            }
        }
        let p = &*self.project;
        let filter = p.default_phase_filter();
        let views: Vec<ViewId> = self
            .tx
            .views
            .iter()
            .filter(|(id, c)| c.0.is_none() && p.views.get(id).is_some_and(|v| p.phase(v.phase).is_none()))
            .map(|(id, _)| *id)
            .collect();
        for id in views {
            if let Some(v) = self.project.views.get_mut(&id) {
                v.phase = last;
                if v.phase_filter.is_none() {
                    v.phase_filter = filter;
                }
            }
        }
        for (id, created, demolished) in self.project.phase_fixes() {
            if let Some(e) = self.element_mut(id) {
                e.phase_created = created;
                e.phase_demolished = demolished;
            }
        }
    }

    /// Design option sets (see [`crate::design_option`]).
    pub fn option_sets_mut(&mut self) -> &mut Vec<crate::OptionSet> {
        self.touch_settings();
        &mut self.project.option_sets
    }

    /// Keep design options consistent: model elements added by this edit
    /// without an option go into the active one, hosted elements follow
    /// their hosts and unknown options mean the main model
    /// ([`Project::option_fixes`]); views and templates drop choices of
    /// deleted sets and options.
    fn sync_options(&mut self) {
        let p = &*self.project;
        if p.option_sets.is_empty()
            && p.elements.values().all(|e| e.design_option.is_none())
            && p.views.values().all(|v| v.design_options.is_empty())
            && p.templates.values().all(|t| t.design_options.is_empty())
        {
            return;
        }
        if let Some(active) = p.active_option() {
            let added: Vec<ElementId> = self
                .tx
                .elements
                .iter()
                .filter(|(id, c)| {
                    c.0.is_none()
                        && p.elements.get(id).is_some_and(|e| {
                            e.design_option.is_none()
                                && e.category().is_phased()
                                && crate::phase::phase_host(e).is_none()
                        })
                })
                .map(|(id, _)| *id)
                .collect();
            for id in added {
                if let Some(e) = self.project.elements.get_mut(&id) {
                    e.design_option = Some(active);
                }
            }
        }
        for (id, option) in self.project.option_fixes() {
            if let Some(e) = self.element_mut(id) {
                e.design_option = option;
            }
        }
        let p = &*self.project;
        let views: Vec<ViewId> =
            p.views.values().filter(|v| p.stale_choices(&v.design_options)).map(|v| v.id).collect();
        let stale_templates = p.templates.values().any(|t| p.stale_choices(&t.design_options));
        for id in views {
            let mut choices = self.project.views.get(&id).map(|v| v.design_options.clone()).unwrap_or_default();
            self.project.clean_choices(&mut choices);
            if let Some(v) = self.view_mut(id) {
                v.design_options = choices;
            }
        }
        if stale_templates {
            let mut templates = std::mem::take(self.templates_mut());
            for t in templates.values_mut() {
                self.project.clean_choices(&mut t.design_options);
            }
            self.project.templates = templates;
        }
    }

    pub fn units_mut(&mut self) -> &mut crate::Units {
        self.touch_settings();
        &mut self.project.units
    }

    /// Column and beam types with a profile keep their legacy section as the
    /// profile's bounding box (a legacy section that is exactly the profile
    /// stays as it is); types whose profile is gone drop the reference and
    /// keep the box. A type whose legacy section this edit changed while
    /// leaving its profile reference alone points to a profile made from the
    /// new section instead (an equal one, else a new one); when both changed,
    /// the profile wins. Runs when this edit touched settings (profiles live
    /// there) or a column or beam type, and writes only what changes.
    fn sync_profiles(&mut self) {
        let settings = self.tx.settings.is_some();
        let ids: Vec<ElementTypeId> = self
            .project
            .types
            .values()
            .filter(|t| {
                matches!(
                    t.data,
                    crate::TypeData::Column { profile: Some(_), .. } | crate::TypeData::Beam { profile: Some(_), .. }
                ) && (settings || self.tx.types.contains_key(&t.id))
            })
            .map(|t| t.id)
            .collect();
        for id in ids {
            let Some(data) = self.project.types.get(&id).map(|t| &t.data) else { continue };
            // A direct edit of the legacy section (the profile reference left
            // as it was) is authoritative: the type points to a profile made
            // from the new section instead of getting the old box back.
            let before = self.tx.types.get(&id).and_then(|c| c.0.as_ref()).map(|t| &t.data);
            if let Some(before) = before
                && crate::profiles_project::type_profile(before) == crate::profiles_project::type_profile(data)
                && !crate::profiles_project::same_legacy(before, data)
            {
                let legacy = self.project.legacy_profile(data);
                self.touch_settings();
                let pid = legacy.and_then(|(material, shape)| self.project.section_profile(material, shape));
                // The new section is already placed: it is the profile as is.
                if let Some(
                    crate::TypeData::Column { profile, profile_rotation_deg, profile_mirror, .. }
                    | crate::TypeData::Beam { profile, profile_rotation_deg, profile_mirror, .. },
                ) = self.type_mut(id).map(|t| &mut t.data)
                {
                    *profile = pid;
                    *profile_rotation_deg = 0.0;
                    *profile_mirror = false;
                }
                continue;
            }
            let gone = match data {
                crate::TypeData::Column { profile: Some(p), .. } | crate::TypeData::Beam { profile: Some(p), .. } => {
                    self.project.profile(*p).is_none()
                }
                _ => continue,
            };
            if gone {
                if let Some(crate::TypeData::Column { profile, .. } | crate::TypeData::Beam { profile, .. }) =
                    self.type_mut(id).map(|t| &mut t.data)
                {
                    *profile = None;
                }
                continue;
            }
            let Some(boxed) = self.project.profile_box(data) else { continue };
            if let Some(t) = self.type_mut(id) {
                t.data = boxed;
            }
        }
    }

    /// Pipe types of a series keep their row's outside diameter; types whose
    /// series or size is gone drop the reference and keep the diameter. Runs
    /// when this edit touched settings (series live there) or a pipe type,
    /// and writes only what changes.
    fn sync_pipe_series(&mut self) {
        let settings = self.tx.settings.is_some();
        let fixes: Vec<(ElementTypeId, crate::TypeData)> = self
            .project
            .types
            .values()
            .filter(|t| settings || self.tx.types.contains_key(&t.id))
            .filter_map(|t| {
                let fix = crate::pipe_series_project::pipe_fix(self.project, &t.data)?;
                let mut data = t.data.clone();
                crate::pipe_series_project::apply_pipe_fix(&mut data, fix).then_some((t.id, data))
            })
            .collect();
        for (id, data) in fixes {
            if let Some(t) = self.type_mut(id) {
                t.data = data;
            }
        }
    }

    /// Fill in "after" states and drop no-op entries.
    fn finish(mut self) -> Transaction {
        // Pipe types follow their series rows.
        self.sync_pipe_series();
        // Legacy sections follow their profiles first, so what reads them below sees the new size.
        self.sync_profiles();
        // Member hosts (steel buildings, trusses) next: they read what the
        // edit itself did to elements, before the syncs below change any.
        self.sync_member_hosts();
        // Associative dimensions follow what this edit changed (elements or levels).
        self.sync_shafts();
        self.sync_pads();
        self.sync_piles();
        self.sync_attachments();
        self.sync_wall_splines();
        self.sync_wall_profiles();
        // Locked dimensions move what they lock before the dimensions follow.
        self.hold_locked_dimensions();
        self.sync_dimensions();
        self.sync_phases();
        self.sync_options();
        // Members take their host's phases and option once those are settled.
        self.sync_member_phases();
        self.sync_sketches();
        self.sync_callouts();
        self.sync_interior_views();
        self.sync_sun_ground();
        self.sync_tag_types();
        self.sync_views();
        let p = &*self.project;
        self.tx.elements.retain(|id, c| {
            c.1 = p.elements.get(id).cloned();
            c.0 != c.1
        });
        self.tx.views.retain(|id, c| {
            c.1 = p.views.get(id).cloned();
            c.0 != c.1
        });
        self.tx.types.retain(|id, c| {
            c.1 = p.types.get(id).cloned();
            c.0 != c.1
        });
        if let Some(c) = &mut self.tx.settings {
            c.1 = Some(Settings::of(p));
            if c.0 == c.1 {
                self.tx.settings = None;
            }
        }
        self.tx
    }
}

/// What a [`Document`] notes while its journal is on (see
/// [`Document::start_journal`]): each element's state before the first
/// change since, and whether anything else changed.
#[derive(Debug, Clone, Default)]
struct Journal {
    before: BTreeMap<ElementId, Option<Element>>,
    other: bool,
}

impl Journal {
    /// Note the state before `tx` is applied (`forward`: done or redone;
    /// otherwise undone).
    fn note(&mut self, tx: &Transaction, forward: bool) {
        for (id, (before, after)) in &tx.elements {
            self.before.entry(*id).or_insert_with(|| if forward { before.clone() } else { after.clone() });
        }
        if !tx.views.is_empty() || !tx.types.is_empty() || tx.settings.is_some() {
            self.other = true;
        }
    }
}

/// The net changes to a document's elements since its journal was started
/// ([`Document::take_journal`]): ids in ascending order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementChanges {
    pub added: Vec<ElementId>,
    pub modified: Vec<ElementId>,
    pub removed: Vec<ElementId>,
    /// Something other than elements changed too (types, views, levels,
    /// settings, plugin data of the project).
    pub other: bool,
}

impl ElementChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty() && !self.other
    }
}

/// A point of a [`Document`]'s undo history ([`Document::history_mark`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryMark {
    depth: usize,
    merge_key: Option<u64>,
}

impl HistoryMark {
    /// The undo depth then.
    pub fn depth(&self) -> usize {
        self.depth
    }
}

/// An open project plus its undo history and file state. Cloned to run a
/// long command (an import) on another thread while the app keeps drawing.
#[derive(Clone)]
pub struct Document {
    pub project: Project,
    pub path: Option<PathBuf>,
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    revision: u64,
    saved_revision: u64,
    /// A view state change (see [`Document::set_view_state`]) since saving.
    view_state_changed: bool,
    /// Merge key of the last recorded transaction, for coalescing.
    last_merge_key: Option<u64>,
    /// The last identity given to an undo step.
    serials: u64,
    /// The phase new elements are built in (the active view's; `None`:
    /// the last phase). Display state: not saved, not undone.
    pub creation_phase: Option<crate::PhaseId>,
    /// What changed since [`Document::start_journal`], while it is on.
    journal: Option<Journal>,
}

impl Document {
    pub fn new(mut project: Project) -> Self {
        project.ensure_uids();
        Self {
            project,
            path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 1,
            saved_revision: 1,
            view_state_changed: false,
            last_merge_key: None,
            serials: 0,
            creation_phase: None,
            journal: None,
        }
    }

    /// Start noting which elements the edits, undos and redos from now on
    /// change (anew, if it was on). Each change notes the state of the
    /// elements it touches before it, once per element (one clone), so the
    /// cost follows the size of the changes; nothing is noted while it is
    /// off.
    pub fn start_journal(&mut self) {
        self.journal = Some(Journal::default());
    }

    /// Stop the journal and return the net changes since it started: an
    /// element is added, removed or modified when its state now differs from
    /// its state then (compared with `PartialEq`), so changes that cancel out
    /// leave nothing. `None` when the journal was off.
    pub fn take_journal(&mut self) -> Option<ElementChanges> {
        let journal = self.journal.take()?;
        let mut changes = ElementChanges { other: journal.other, ..ElementChanges::default() };
        for (id, before) in journal.before {
            match (before, self.project.elements.get(&id)) {
                (None, Some(_)) => changes.added.push(id),
                (Some(_), None) => changes.removed.push(id),
                (Some(b), Some(a)) if b != *a => changes.modified.push(id),
                _ => {}
            }
        }
        Some(changes)
    }

    /// Note `tx` in the journal, if it is on (before it is applied).
    fn note(&mut self, tx: &Transaction, forward: bool) {
        if let Some(j) = &mut self.journal {
            j.note(tx, forward);
        }
    }

    /// Where the undo history stands: its depth and the merge key of its
    /// last step (see [`Document::fold_since`]).
    pub fn history_mark(&self) -> HistoryMark {
        HistoryMark { depth: self.undo.len(), merge_key: self.last_merge_key }
    }

    /// Fold the undo steps recorded since `mark` into the step that was on
    /// top of the history then, which keeps its name (follow-up edits that
    /// belong to it, like a plugin's event handlers keeping their data in
    /// step with the edit). Its merge key stays too, so the next edit merged
    /// with that key (a value still being dragged) still merges into it. The
    /// folded step gets a new identity ([`Document::undo_top_id`]). Returns
    /// whether anything was folded: nothing is when no step was recorded
    /// since, or when there was no step to fold into.
    pub fn fold_since(&mut self, mark: HistoryMark) -> bool {
        if mark.depth == 0 || mark.depth > self.undo.len() {
            return false;
        }
        if mark.depth == self.undo.len() {
            // Nothing recorded since (or what was is discarded): the merge
            // goes on as it was.
            self.last_merge_key = mark.merge_key;
            return false;
        }
        let newer = self.undo.split_off(mark.depth);
        let serial = self.next_serial();
        let Some(top) = self.undo.last_mut() else { return false };
        for tx in newer {
            top.merge(tx);
        }
        top.prune();
        top.serial = serial;
        if top.is_empty() {
            self.undo.pop();
        }
        self.last_merge_key = mark.merge_key;
        true
    }

    /// Undo the steps recorded since undo depth `depth` and drop them: they
    /// cannot be redone (edits that failed, like a plugin event handler's).
    pub fn discard_since(&mut self, depth: usize) {
        while self.undo.len() > depth {
            let Some(tx) = self.undo.pop() else { break };
            self.note(&tx, false);
            tx.apply(&mut self.project, false);
            self.last_merge_key = None;
            self.revision += 1;
        }
    }

    /// Apply a change that is saved but not undoable, and that no derived
    /// geometry depends on (a 3D view's camera). The revision stays, so caches
    /// keyed on it stay valid: callers drop what does depend on the change
    /// (see `Session::set_view_camera`).
    pub fn set_view_state(&mut self, f: impl FnOnce(&mut Project)) {
        f(&mut self.project);
        self.view_state_changed = true;
    }

    /// Set the design option new elements go into, which views without a
    /// choice of their own show for its set (`None`: the main model). Not
    /// undone nor saved, but views change: the revision moves on (without
    /// marking the document as changed).
    pub fn set_active_option(&mut self, option: Option<crate::OptionId>) {
        let option = option.filter(|o| self.project.option_set_of(*o).is_some());
        if self.project.active_option == option {
            return;
        }
        let clean = self.revision == self.saved_revision;
        self.project.active_option = option;
        self.revision += 1;
        if clean {
            self.saved_revision = self.revision;
        }
    }

    /// Monotonic counter bumped on every model change; use it to invalidate caches.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision || self.view_state_changed
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.revision;
        self.view_state_changed = false;
    }

    /// Treat the project as not saved (one restored from a recovery copy,
    /// whose changes never reached its file).
    pub fn mark_unsaved(&mut self) {
        self.saved_revision = 0;
    }

    /// Run `f` as one undoable transaction named `name`.
    pub fn edit<R>(&mut self, name: impl Into<String>, f: impl FnOnce(&mut EditCtx) -> R) -> R {
        self.edit_impl(name.into(), None, f)
    }

    /// Like [`Document::edit`], but consecutive edits with the same `merge_key`
    /// become a single undo step (e.g. while dragging a value).
    pub fn edit_merged<R>(&mut self, name: impl Into<String>, merge_key: u64, f: impl FnOnce(&mut EditCtx) -> R) -> R {
        self.edit_impl(name.into(), Some(merge_key), f)
    }

    /// Number of undoable transactions.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// Identity of the step on top of the undo history (`None` when there is
    /// nothing to undo). Each step recorded gets one never given before in
    /// this document, and a step another edit is folded into (a merged edit,
    /// a squash) gets a new one; a step undone and redone keeps its own. So a
    /// caller that kept the identity of a step it made can tell whether that
    /// step, unchanged, is still the one [`Document::undo`] would undo.
    pub fn undo_top_id(&self) -> Option<u64> {
        self.undo.last().map(|t| t.serial)
    }

    /// A new step identity.
    fn next_serial(&mut self) -> u64 {
        self.serials = self.serials.wrapping_add(1);
        self.serials
    }

    /// Fold the undo steps recorded since undo depth `depth` into one step
    /// named `name` (a command made of other commands, undone at once).
    /// Changes that cancel out are dropped, and so is the step if nothing is
    /// left; returns whether a step remains.
    pub fn squash_since(&mut self, depth: usize, name: impl Into<String>) -> bool {
        if depth >= self.undo.len() {
            return false;
        }
        let mut steps = self.undo.split_off(depth).into_iter();
        let Some(mut tx) = steps.next() else { return false };
        for newer in steps {
            tx.merge(newer);
        }
        tx.prune();
        tx.name = name.into();
        tx.serial = self.next_serial();
        self.last_merge_key = None;
        if tx.is_empty() {
            return false;
        }
        self.undo.push(tx);
        true
    }

    /// End coalescing so the next merged edit starts a new undo step.
    pub fn break_merge(&mut self) {
        self.last_merge_key = None;
    }

    fn edit_impl<R>(&mut self, name: String, merge_key: Option<u64>, f: impl FnOnce(&mut EditCtx) -> R) -> R {
        let mut ctx = EditCtx {
            project: &mut self.project,
            tx: Transaction { name, ..Default::default() },
            creation_phase: self.creation_phase,
        };
        let result = f(&mut ctx);
        let tx = ctx.finish();
        if !tx.is_empty() {
            self.note(&tx, true);
            let serial = self.next_serial();
            match self.undo.last_mut() {
                Some(last) if merge_key.is_some() && merge_key == self.last_merge_key && self.redo.is_empty() => {
                    last.merge(tx);
                    last.serial = serial;
                }
                _ => self.undo.push(Transaction { serial, ..tx }),
            }
            self.last_merge_key = merge_key;
            self.redo.clear();
            self.revision += 1;
        }
        result
    }

    /// Like [`Document::edit`], but the change is kept only if `f` succeeds
    /// and `check` accepts the resulting project; otherwise the project is
    /// restored and nothing is recorded (no undo step, redo kept).
    pub fn try_edit<R, E>(
        &mut self,
        name: impl Into<String>,
        f: impl FnOnce(&mut EditCtx) -> Result<R, E>,
        check: impl FnOnce(&Project) -> Result<(), E>,
    ) -> Result<R, E> {
        self.try_edit_merged(name, None, f, check)
    }

    /// [`Document::try_edit`] that merges into the previous step when
    /// `merge_key` matches its key (like [`Document::edit_merged`]).
    pub fn try_edit_merged<R, E>(
        &mut self,
        name: impl Into<String>,
        merge_key: Option<u64>,
        f: impl FnOnce(&mut EditCtx) -> Result<R, E>,
        check: impl FnOnce(&Project) -> Result<(), E>,
    ) -> Result<R, E> {
        let mut ctx = EditCtx {
            project: &mut self.project,
            tx: Transaction { name: name.into(), ..Default::default() },
            creation_phase: self.creation_phase,
        };
        let result = f(&mut ctx);
        let tx = ctx.finish();
        let outcome = result.and_then(|r| check(&self.project).map(|()| r));
        if outcome.is_err() {
            tx.apply(&mut self.project, false);
            return outcome;
        }
        if !tx.is_empty() {
            self.note(&tx, true);
            let serial = self.next_serial();
            match self.undo.last_mut() {
                Some(last) if merge_key.is_some() && merge_key == self.last_merge_key && self.redo.is_empty() => {
                    last.merge(tx);
                    last.serial = serial;
                }
                _ => self.undo.push(Transaction { serial, ..tx }),
            }
            self.last_merge_key = merge_key;
            self.redo.clear();
            self.revision += 1;
        }
        outcome
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.undo.last().map(|t| t.name.as_str())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo.last().map(|t| t.name.as_str())
    }

    pub fn undo(&mut self) -> bool {
        let Some(tx) = self.undo.pop() else { return false };
        self.note(&tx, false);
        tx.apply(&mut self.project, false);
        self.redo.push(tx);
        self.last_merge_key = None;
        self.revision += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(tx) = self.redo.pop() else { return false };
        self.note(&tx, true);
        tx.apply(&mut self.project, true);
        self.undo.push(tx);
        self.last_merge_key = None;
        self.revision += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, ElementKind, Hosted, Wall};
    use glam::DVec2;

    #[test]
    fn squashed_steps_undo_at_once() {
        let mut doc = Document::new(Project::from_template());
        let before = doc.project.clone();
        let depth = doc.undo_depth();
        doc.edit("a", |e| e.info_mut().name = "one".into());
        doc.edit("b", |e| e.info_mut().name = "two".into());
        assert!(doc.squash_since(depth, "both"));
        assert_eq!(doc.undo_depth(), depth + 1);
        assert!(doc.undo());
        assert_eq!(doc.project, before);
        assert!(doc.redo());
        assert_eq!(doc.project.info.name, "two");
        // Steps that cancel out leave no step.
        let depth = doc.undo_depth();
        let name = doc.project.info.name.clone();
        doc.edit("c", |e| e.info_mut().name = "three".into());
        doc.edit("d", |e| e.info_mut().name = name);
        assert!(!doc.squash_since(depth, "none"));
        assert_eq!(doc.undo_depth(), depth);
        assert!(!doc.squash_since(depth + 5, "past the end"));
    }

    #[test]
    fn rejected_edits_leave_no_trace() {
        let mut doc = Document::new(Project::from_template());
        let before = doc.project.clone();
        let (depth, rev) = (doc.undo_depth(), doc.revision());
        let failed: Result<(), &str> = doc.try_edit(
            "t",
            |e| {
                e.info_mut().name = "changed".into();
                Err("no")
            },
            |_| Ok(()),
        );
        assert!(failed.is_err());
        let refused: Result<(), &str> = doc.try_edit(
            "t",
            |e| {
                e.info_mut().name = "changed".into();
                Ok(())
            },
            |p| if p.info.name == "changed" { Err("refused") } else { Ok(()) },
        );
        assert!(refused.is_err());
        assert_eq!(doc.project, before);
        assert_eq!((doc.undo_depth(), doc.revision()), (depth, rev));
        let ok: Result<(), &str> = doc.try_edit(
            "t",
            |e| {
                e.info_mut().name = "kept".into();
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(ok.is_ok() && doc.can_undo() && doc.project.info.name == "kept");
    }

    fn add_wall(doc: &mut Document) -> ElementId {
        doc.edit("wall", |e| {
            let level = e.project().levels[0].id;
            let type_id = e.project().default_type(Category::Wall).unwrap();
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Wall(Wall {
                    start: DVec2::ZERO,
                    end: DVec2::new(5.0, 0.0),
                    base_offset: 0.0,
                    height: 3.0,
                    ..Default::default()
                }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        })
    }

    #[test]
    fn folded_edits_join_the_top_step_and_keep_its_merge() {
        let mut doc = Document::new(Project::from_template());
        let before = doc.project.clone();
        // Nothing to fold into.
        let mark = doc.history_mark();
        doc.edit("a", |e| e.info_mut().name = "a".into());
        assert!(!doc.fold_since(mark) && doc.undo_depth() == 1);
        // A dragged value: each tick merges, follow-up edits fold into it.
        let depth = doc.undo_depth();
        for i in 0..3 {
            doc.edit_merged("drag", 7, |e| e.info_mut().number = i.to_string());
            let mark = doc.history_mark();
            doc.edit("follow", |e| e.info_mut().client = format!("c{i}"));
            assert!(doc.fold_since(mark));
            assert_eq!((doc.undo_depth(), doc.undo_name()), (depth + 1, Some("drag")));
        }
        // Nothing recorded since: the merge goes on.
        let mark = doc.history_mark();
        assert!(!doc.fold_since(mark));
        doc.edit_merged("drag", 7, |e| e.info_mut().number = "9".into());
        assert_eq!(doc.undo_depth(), depth + 1);
        // One undo takes back the drag and what followed it.
        assert!(doc.undo());
        assert_eq!((&doc.project.info.number, &doc.project.info.client), (&before.info.number, &before.info.client));
        assert_eq!(doc.project.info.name, "a");
        assert!(doc.undo());
        assert_eq!(doc.project, before);
    }

    #[test]
    fn the_journal_reports_net_element_changes() {
        let mut doc = Document::new(Project::from_template());
        assert_eq!(doc.take_journal(), None);
        let kept = add_wall(&mut doc);
        doc.start_journal();
        let added = add_wall(&mut doc);
        doc.edit("move", |e| e.element_mut(kept).unwrap().translate(DVec2::new(1.0, 0.0)));
        let changes = doc.take_journal().unwrap();
        assert_eq!((changes.added, changes.modified, changes.removed), (vec![added], vec![kept], vec![]));
        assert!(!changes.other && doc.take_journal().is_none());
        // Undone: the wall added is removed, the moved one back where it was.
        doc.start_journal();
        assert!(doc.undo() && doc.undo());
        let changes = doc.take_journal().unwrap();
        assert_eq!((changes.added, changes.modified, changes.removed), (vec![], vec![kept], vec![added]));
        // Changes that cancel out, and changes to settings only.
        doc.start_journal();
        assert!(doc.redo() && doc.undo());
        doc.edit("name", |e| e.info_mut().name = "x".into());
        let changes = doc.take_journal().unwrap();
        assert!(changes.added.is_empty() && changes.modified.is_empty() && changes.removed.is_empty());
        assert!(changes.other && !changes.is_empty());
        // Discarded steps cannot be redone.
        let depth = doc.undo_depth();
        add_wall(&mut doc);
        doc.discard_since(depth);
        assert_eq!(doc.undo_depth(), depth);
        assert!(!doc.can_redo());
        assert_eq!(doc.project.elements.len(), 1);
    }

    #[test]
    fn undo_redo_create_and_modify() {
        let mut doc = Document::new(Project::from_template());
        let id = add_wall(&mut doc);
        doc.edit("move", |e| e.element_mut(id).unwrap().translate(DVec2::new(1.0, 0.0)));
        let wall = |d: &Document| d.project.element(id).and_then(|e| e.as_wall().cloned());
        assert_eq!(wall(&doc).unwrap().start, DVec2::new(1.0, 0.0));
        assert!(doc.undo());
        assert_eq!(wall(&doc).unwrap().start, DVec2::ZERO);
        assert!(doc.undo());
        assert!(wall(&doc).is_none());
        assert!(doc.redo());
        assert!(doc.redo());
        assert_eq!(wall(&doc).unwrap().start, DVec2::new(1.0, 0.0));
        assert!(!doc.redo());
    }

    #[test]
    fn deleting_wall_deletes_hosted_and_undo_restores() {
        let mut doc = Document::new(Project::from_template());
        let wall = add_wall(&mut doc);
        let door = doc.edit("door", |e| {
            let level = e.project().levels[0].id;
            let type_id = e.project().default_type(Category::Door).unwrap();
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: "1".into(),
                comments: String::new(),
                uid: Default::default(),
                kind: ElementKind::Door(Hosted {
                    host: wall,
                    offset: 2.0,
                    sill_height: 0.0,
                    flip_facing: false,
                    flip_hand: false,
                }),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        });
        doc.edit("delete", |e| e.remove_element(wall));
        assert!(doc.project.element(door).is_none());
        doc.undo();
        assert!(doc.project.element(door).is_some());
        assert!(doc.project.element(wall).is_some());
    }

    /// A crafted file can make doors host each other (or themselves); removing
    /// one must not recurse forever.
    #[test]
    fn removing_cyclic_hosted_elements_terminates() {
        let mut doc = Document::new(Project::from_template());
        let (a, b) = doc.edit("cycle", |e| {
            let level = e.project().levels[0].id;
            let type_id = e.project().default_type(Category::Door).unwrap();
            let (a, b) = (ElementId(e.alloc_id()), ElementId(e.alloc_id()));
            for (id, host) in [(a, b), (b, a)] {
                let kind = ElementKind::Door(Hosted {
                    host,
                    offset: 1.0,
                    sill_height: 0.0,
                    flip_facing: false,
                    flip_hand: false,
                });
                e.add_element(Element {
                    id,
                    level,
                    type_id,
                    mark: String::new(),
                    comments: String::new(),
                    kind,
                    uid: Default::default(),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                });
            }
            let selfie = ElementId(e.alloc_id());
            let kind = ElementKind::Door(Hosted {
                host: selfie,
                offset: 1.0,
                sill_height: 0.0,
                flip_facing: false,
                flip_hand: false,
            });
            e.add_element(Element {
                id: selfie,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            e.remove_element(selfie);
            (a, b)
        });
        doc.edit("delete", |e| e.remove_element(a));
        assert!(doc.project.element(a).is_none() && doc.project.element(b).is_none());
    }

    /// Deleting the roof a roof is joined to unjoins it in the same step;
    /// undoing the delete brings the join back.
    #[test]
    fn deleting_a_joined_roof_unjoins_its_wings() {
        let mut doc = Document::new(Project::from_template());
        let add_roof = |doc: &mut Document, to: Option<ElementId>| {
            doc.edit("roof", |e| {
                let level = e.project().levels[0].id;
                let type_id = e.project().default_type(Category::Roof).unwrap();
                let id = ElementId(e.alloc_id());
                let square = vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y];
                let mut roof = crate::Roof::new(square, 3.0, 0.0, crate::RoofShape::Hip { slope: 0.5 });
                roof.join_to = to;
                e.add_element(Element {
                    id,
                    level,
                    type_id,
                    mark: String::new(),
                    comments: String::new(),
                    uid: Default::default(),
                    kind: ElementKind::Roof(roof),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                })
            })
        };
        let main = add_roof(&mut doc, None);
        let wing = add_roof(&mut doc, Some(main));
        assert_eq!(doc.project.roof_join_target(wing), Some(main));
        let depth = doc.undo_depth();
        doc.edit("delete", |e| e.remove_element(main));
        assert_eq!(doc.undo_depth(), depth + 1);
        assert_eq!(doc.project.roof_join_target(wing), None);
        doc.undo();
        assert_eq!(doc.project.roof_join_target(wing), Some(main));
    }

    #[test]
    fn merged_edits_undo_together() {
        let mut doc = Document::new(Project::from_template());
        let id = add_wall(&mut doc);
        for _ in 0..5 {
            doc.edit_merged("drag", 7, |e| e.element_mut(id).unwrap().translate(DVec2::X));
        }
        let start = |d: &Document| d.project.element(id).unwrap().as_wall().unwrap().start;
        assert_eq!(start(&doc), DVec2::new(5.0, 0.0));
        doc.undo();
        assert_eq!(start(&doc), DVec2::ZERO);
        assert!(doc.can_undo(), "wall creation is still undoable");
    }

    #[test]
    fn noop_edit_is_not_recorded() {
        let mut doc = Document::new(Project::from_template());
        let rev = doc.revision();
        doc.edit("nothing", |e| {
            let _ = e.info_mut();
        });
        assert_eq!(doc.revision(), rev);
        assert!(!doc.can_undo());
    }

    #[test]
    fn renaming_a_material_updates_types_in_one_step() {
        let mut doc = Document::new(Project::from_template());
        let old = "Madeira";
        let users = doc.project.material_users(old);
        assert!(!users.is_empty());
        assert!(doc.edit("rename", |e| e.rename_material(old, "Carvalho")));
        assert!(doc.project.material(old).is_none());
        assert_eq!(doc.project.material_users("Carvalho"), users);
        // Taken or empty names are refused.
        assert!(!doc.edit("rename", |e| e.rename_material("Carvalho", "Concreto")));
        assert!(!doc.edit("rename", |e| e.rename_material("Carvalho", "  ")));
        doc.undo();
        assert_eq!(doc.project.material_users(old), users);
        assert!(doc.project.material("Carvalho").is_none());
    }

    #[test]
    fn undo_steps_have_identities() {
        let mut doc = Document::new(Project::from_template());
        assert_eq!(doc.undo_top_id(), None);
        doc.edit("a", |e| e.info_mut().name = "one".into());
        let a = doc.undo_top_id();
        assert!(a.is_some());
        // Undone and redone, a step keeps its identity.
        doc.undo();
        assert_eq!(doc.undo_top_id(), None);
        doc.redo();
        assert_eq!(doc.undo_top_id(), a);
        // An edit with no change records nothing.
        doc.edit("none", |_| ());
        assert_eq!(doc.undo_top_id(), a);
        // A merged edit changes the step, and with it its identity.
        doc.edit_merged("b", 7, |e| e.info_mut().name = "two".into());
        let b = doc.undo_top_id();
        assert_ne!(b, a);
        doc.edit_merged("b", 7, |e| e.info_mut().name = "three".into());
        assert_eq!(doc.undo_depth(), 2);
        assert_ne!(doc.undo_top_id(), b);
        // A step undone and replaced by another never has the same identity.
        doc.undo();
        doc.edit("c", |e| e.info_mut().name = "four".into());
        assert!(doc.undo_top_id().is_some_and(|c| Some(c) != a && Some(c) != b));
        // Squashing makes a new step.
        let before = doc.undo_top_id();
        assert!(doc.squash_since(0, "all"));
        assert_ne!(doc.undo_top_id(), before);
    }
}
