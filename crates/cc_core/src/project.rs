use crate::dim_style::DimensionStyle;
use crate::material::Material;
use crate::{
    CameraState, Category, ColumnSection, Direction, Element, ElementId, ElementKind, ElementType, ElementTypeId,
    FoundationKind, Layer, LayerFunction, LevelId, SectionShape, TypeData, View, ViewCategory, ViewId, ViewKind,
    Viewport, WallFunction,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A project value named by the user (shown by title blocks as `{project:Name}`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Persistent unique id of the project.
    #[serde(default)]
    pub uid: crate::Uid,
    pub name: String,
    pub number: String,
    pub client: String,
    pub address: String,
    pub author: String,
    /// Fields named by the user, in the order they were added.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ProjectField>,
    /// Where the project stands on Earth (see [`crate::site`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<crate::site::SiteLocation>,
}

impl ProjectInfo {
    /// The value of the field called `name`.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub id: LevelId,
    pub name: String,
    /// Elevation in meters.
    pub elevation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub info: ProjectInfo,
    /// Sorted by elevation.
    pub levels: Vec<Level>,
    pub materials: Vec<Material>,
    pub types: BTreeMap<ElementTypeId, ElementType>,
    pub elements: BTreeMap<ElementId, Element>,
    pub views: BTreeMap<ViewId, View>,
    /// How quantities are displayed and typed.
    #[serde(default)]
    pub units: crate::Units,
    /// Elements protected from moving and deleting.
    #[serde(default)]
    pub pinned: std::collections::BTreeSet<ElementId>,
    /// Joined geometry as `(cutting, cut)` pairs: where the two overlap,
    /// the first keeps the volume and their cut outlines merge.
    #[serde(default)]
    pub joins: std::collections::BTreeSet<(ElementId, ElementId)>,
    /// Named sets of elements selected, moved and copied together.
    #[serde(default)]
    pub groups: BTreeMap<crate::GroupId, Group>,
    /// Plumbing systems and electrical circuits.
    #[serde(default)]
    pub systems: BTreeMap<crate::SystemId, crate::MepSystem>,
    /// Named view filters (categories and rules), applied by views.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub filters: BTreeMap<crate::FilterId, crate::ViewFilter>,
    /// View templates.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub templates: BTreeMap<crate::TemplateId, crate::ViewTemplate>,
    /// Construction phases, in time order (see [`crate::phase`]).
    #[serde(default)]
    pub phases: Vec<crate::Phase>,
    /// Phase filters views choose from.
    #[serde(default)]
    pub phase_filters: BTreeMap<crate::PhaseFilterId, crate::PhaseFilter>,
    /// Graphics of each phase status, where a phase filter overrides it.
    #[serde(default, skip_serializing_if = "crate::phase::phase_graphics_is_default")]
    pub phase_graphics: crate::PhaseGraphics,
    /// Elements made by visual programming graphs, and the call that made
    /// each (see [`crate::graph_binding`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub graph_bindings: BTreeMap<ElementId, crate::graph_binding::GraphBinding>,
    /// The project's own line styles (see [`crate::detail`]), used by detail
    /// lines and region boundaries next to the built-in ones.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub line_styles: crate::detail::LineStyles,
    /// The project's own fill patterns (see [`crate::fill_pattern`]), used
    /// by materials, filled regions and view overrides next to the built-in ones.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fill_patterns: crate::fill_pattern::FillPatterns,
    /// Named structural profiles (tracked as settings).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<crate::ProfileId, crate::structural_profile::ProfileDef>,
    /// Design option sets (see [`crate::design_option`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_sets: Vec<crate::OptionSet>,
    /// How the project browser groups the views (see [`crate::browser`]).
    #[serde(default, skip_serializing_if = "crate::browser::BrowserOrganization::is_default")]
    pub browser: crate::browser::BrowserOrganization,
    /// How reinforcing bars are marked (see [`crate::bar_marks`]).
    #[serde(default, skip_serializing_if = "crate::bar_marks::BarMarking::is_default")]
    pub bar_marking: crate::bar_marks::BarMarking,
    /// Named sun settings views can take (see [`crate::sun`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sun_presets: Vec<crate::sun::SunPreset>,
    /// The design option new elements go into and views without a choice of
    /// their own show for its set: session state, not saved nor undone (set
    /// through [`crate::Document::set_active_option`]).
    #[serde(skip)]
    pub active_option: Option<crate::OptionId>,
    next_id: u64,
}

/// Elements kept together: selecting one selects them all. An element
/// belongs to one group at most.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub members: std::collections::BTreeSet<ElementId>,
}

impl Project {
    /// An empty project with no levels, types or views.
    pub fn empty() -> Self {
        Self {
            info: ProjectInfo::default(),
            levels: Vec::new(),
            materials: Vec::new(),
            types: BTreeMap::new(),
            elements: BTreeMap::new(),
            views: BTreeMap::new(),
            units: crate::Units::default(),
            pinned: Default::default(),
            joins: Default::default(),
            groups: BTreeMap::new(),
            systems: BTreeMap::new(),
            filters: BTreeMap::new(),
            templates: BTreeMap::new(),
            phases: Vec::new(),
            phase_filters: BTreeMap::new(),
            phase_graphics: crate::PhaseGraphics::default(),
            graph_bindings: BTreeMap::new(),
            line_styles: BTreeMap::new(),
            fill_patterns: BTreeMap::new(),
            profiles: BTreeMap::new(),
            option_sets: Vec::new(),
            browser: Default::default(),
            bar_marking: Default::default(),
            sun_presets: Vec::new(),
            active_option: None,
            next_id: 1,
        }
    }

    /// The sheets by number (natural order), then id.
    pub fn sheets_in_order(&self) -> Vec<ViewId> {
        let mut sheets: Vec<(&str, ViewId)> = self
            .views
            .values()
            .filter_map(|v| match &v.kind {
                ViewKind::Sheet { number, .. } => Some((number.as_str(), v.id)),
                _ => None,
            })
            .collect();
        sheets.sort_by(|a, b| natural_cmp(a.0, b.0).then(a.1.cmp(&b.1)));
        sheets.into_iter().map(|(_, id)| id).collect()
    }

    /// The title block types, by id.
    pub fn title_blocks(&self) -> Vec<ElementTypeId> {
        self.types.values().filter(|t| t.category() == Category::TitleBlock).map(|t| t.id).collect()
    }

    /// A new project from the default architectural template (Brazilian Portuguese names).
    pub fn from_template() -> Self {
        Self::from_template_localized(&|_| None)
    }

    /// A new project from the metric template, naming levels, materials,
    /// types and views with `translate(key)` (keys `tpl-*`), falling back to Portuguese.
    pub fn from_template_localized(translate: &dyn Fn(&str) -> Option<String>) -> Self {
        Self::from_template_system(crate::MeasurementSystem::Metric, translate)
    }

    /// A new project from the template of `system`, named like
    /// [`Self::from_template_localized`]. Both templates have the same
    /// levels, types and views in the same order; the imperial one shows feet
    /// and inches and sizes its types in round inches.
    pub fn from_template_system(system: crate::MeasurementSystem, translate: &dyn Fn(&str) -> Option<String>) -> Self {
        let s = crate::template_spec::TemplateSpec::of(system);
        let n = |key: &str| {
            let key = s.library_key(key);
            translate(key).unwrap_or_else(|| template_default(key).to_string())
        };
        let mut p = Self::empty();
        p.units = system.units();
        p.info = ProjectInfo {
            uid: crate::Uid::default(),
            name: n("tpl-project-name"),
            number: "0001".into(),
            client: String::new(),
            address: String::new(),
            author: String::new(),
            fields: Vec::new(),
            site: None,
        };
        let ground = LevelId(p.alloc_id());
        let first = LevelId(p.alloc_id());
        p.levels = vec![
            Level { id: ground, name: n("tpl-level-ground"), elevation: 0.0 },
            Level { id: first, name: n("tpl-level-first"), elevation: s.first_level },
        ];
        use crate::material::MaterialClass as C;
        p.materials = [
            (&n("tpl-mat-masonry"), [196, 120, 92], C::Masonry),
            (&n("tpl-mat-plaster"), [232, 226, 214], C::Mortar),
            (&n("tpl-mat-concrete"), [170, 170, 165], C::Concrete),
            (&n("tpl-mat-wood"), [150, 104, 62], C::Wood),
            (&n("tpl-mat-glass"), [150, 200, 225], C::Glass),
            (&n("tpl-mat-gypsum"), [245, 245, 240], C::Gypsum),
            (&n("tpl-mat-roof-tile"), [168, 70, 48], C::Ceramic),
            (&n("tpl-mat-screed"), [190, 185, 175], C::Mortar),
            (&n("tpl-mat-fabric"), [120, 132, 150], C::Fabric),
            (&n("tpl-mat-porcelain"), [240, 240, 236], C::Ceramic),
            (&n("tpl-mat-steel"), [140, 146, 152], C::Metal),
            (&n("tpl-mat-pvc"), [236, 236, 230], C::Plastic),
            (&n("tpl-mat-earth"), crate::topography::EARTH_COLOR, C::Earth),
        ]
        .into_iter()
        .map(|(name, color, class)| {
            let mut m = Material::new(name.as_str(), color);
            crate::material::template_properties(&mut m, class);
            m
        })
        .collect();

        let layer = |m: &str, t: f64| Layer { material: m.into(), thickness: t, function: LayerFunction::Structure };
        let finish = |m: &str, t: f64| Layer { material: m.into(), thickness: t, function: LayerFunction::Finish };
        let names = crate::openings::OpeningNames::new(&n);
        let door = |w: f64, h: f64, m: &str| TypeData::Component {
            component: names.door.clone(),
            def: crate::openings::door_def(&names, w, h, m),
        };
        let window = |w: f64, h: f64, sill: f64, m: &str| TypeData::Component {
            component: names.window.clone(),
            def: crate::openings::window_def(&names, w, h, sill, m),
        };
        let operable = |size: (f64, f64, f64), m: &str, op: crate::WindowOperation| TypeData::Component {
            component: names.window_for(op).clone(),
            def: crate::openings::operable_window_def(&names, size, m, op),
        };
        let masonry = |(key, plaster, core): (&str, f64, f64)| {
            let layers = vec![
                finish(&n("tpl-mat-plaster"), plaster),
                layer(&n("tpl-mat-masonry"), core),
                finish(&n("tpl-mat-plaster"), plaster),
            ];
            (n(key), TypeData::Wall { layers, function: WallFunction::Exterior, curtain: None })
        };
        let column = |key: &str, section: ColumnSection, material: &str| {
            let material = n(material);
            let data =
                TypeData::Column { section, material, profile: None, profile_rotation_deg: 0.0, profile_mirror: false };
            (n(key), data)
        };
        let beam = |key: &str, (width, height): (f64, f64), shape: SectionShape, material: &str| {
            let material = n(material);
            let data = TypeData::Beam {
                width,
                height,
                material,
                shape,
                profile: None,
                profile_rotation_deg: 0.0,
                profile_mirror: false,
            };
            (n(key), data)
        };

        // The types in the order they get their ids (the same in every system).
        let (partition, partition_thickness) = s.partition;
        let mut types = vec![
            masonry(s.masonry[0]),
            masonry(s.masonry[1]),
            (
                n(partition),
                TypeData::Wall {
                    layers: vec![layer(&n("tpl-mat-gypsum"), partition_thickness)],
                    function: WallFunction::Interior,
                    curtain: None,
                },
            ),
        ];
        for (key, w, h) in s.doors {
            types.push((n(key), door(w, h, &n("tpl-mat-wood"))));
        }
        for (key, w, h, sill) in s.windows {
            types.push((n(key), window(w, h, sill, &n("tpl-mat-glass"))));
        }
        use crate::WindowOperation as W;
        let operations = [W::SideHungLeft, W::DoubleSideHung, W::TopHung, W::Sliding];
        for ((key, w, h, sill), op) in s.operable.into_iter().zip(operations) {
            types.push((n(key), operable((w, h, sill), &n("tpl-mat-glass"), op)));
        }
        let (slab, screed, concrete) = s.slab;
        let (column_rect, side) = s.column_rect;
        let (column_round, diameter) = s.column_round;
        let (concrete_beam, beam_width, beam_height) = s.beam;
        let (h_column, h_size, h_web, h_flange) = s.column_steel_h;
        let (tube, tube_size, tube_wall) = s.column_steel_tube;
        let (beam_i, [i_width, i_height, i_web, i_flange]) = s.beam_steel_i;
        types.extend([
            (
                n(slab),
                TypeData::Floor {
                    layers: vec![finish(&n("tpl-mat-screed"), screed), layer(&n("tpl-mat-concrete"), concrete)],
                },
            ),
            (n("tpl-type-ceiling-gypsum"), TypeData::Ceiling { thickness: s.ceiling, material: n("tpl-mat-gypsum") }),
            column(column_rect, ColumnSection::Rectangular { width: side, depth: side }, "tpl-mat-concrete"),
            column(column_round, ColumnSection::Round { diameter }, "tpl-mat-concrete"),
            beam(concrete_beam, (beam_width, beam_height), SectionShape::Solid, "tpl-mat-concrete"),
            column(
                h_column,
                ColumnSection::Profile {
                    width: h_size,
                    depth: h_size,
                    shape: SectionShape::I { web: h_web, flange: h_flange },
                },
                "tpl-mat-steel",
            ),
            column(
                tube,
                ColumnSection::Profile {
                    width: tube_size,
                    depth: tube_size,
                    shape: SectionShape::Tube { wall: tube_wall },
                },
                "tpl-mat-steel",
            ),
            beam(beam_i, (i_width, i_height), SectionShape::I { web: i_web, flange: i_flange }, "tpl-mat-steel"),
        ]);
        for (key, diameter) in s.rebars {
            types.push((n(key), TypeData::rebar(diameter, n("tpl-mat-steel"))));
        }
        let kinds = [FoundationKind::Pad, FoundationKind::Strip, FoundationKind::Slab];
        for ((key, width, length, thickness), kind) in s.footings.into_iter().zip(kinds) {
            let material = n("tpl-mat-concrete");
            types.push((
                n(key),
                TypeData::Foundation { kind, width, length, thickness, material, haunch: None, pile_cap: None },
            ));
        }
        let (stair, stair_width, max_riser, tread) = s.stair;
        let (railing, railing_height, post_spacing) = s.railing;
        types.extend([
            (
                n(stair),
                TypeData::Stair {
                    width: stair_width,
                    max_riser,
                    tread,
                    material: n("tpl-mat-concrete"),
                    build: Default::default(),
                    rules: crate::template_spec::Defaults::of(system).stair_rules,
                },
            ),
            (n(railing), TypeData::Railing { height: railing_height, post_spacing, material: n("tpl-mat-wood") }),
        ]);
        for (key, diameter) in s.pipes {
            types.push((n(key), TypeData::Pipe { diameter, material: n("tpl-mat-pvc") }));
        }
        for (key, diameter) in s.conduits {
            types.push((n(key), TypeData::Conduit { diameter, material: n("tpl-mat-pvc") }));
        }
        types.extend([
            (n("tpl-type-room"), TypeData::Room {}),
            (n(s.grid.0), TypeData::Grid { bubble: s.grid.1 }),
            (n(s.dimension.0), TypeData::Dimension { text_size: s.dimension.1, style: DimensionStyle::of(system) }),
        ]);
        for (key, size) in s.texts {
            types.push((n(key), TypeData::Text { size, symbol: crate::LeaderSymbol::Arrow }));
        }
        types.push((
            n("tpl-type-roof-tile"),
            TypeData::Roof {
                layers: vec![finish(&n("tpl-mat-roof-tile"), s.roof.0), layer(&n("tpl-mat-wood"), s.roof.1)],
                covering: None,
                framing: Default::default(),
            },
        ));
        for (name, data) in types {
            let id = ElementTypeId(p.alloc_id());
            p.types.insert(id, ElementType { id, name, data, info: Default::default(), uid: Default::default() });
        }
        for (component, name, def) in s.sized_library(&n, crate::component::library(&n)) {
            let id = ElementTypeId(p.alloc_id());
            let data = TypeData::Component { component, def };
            p.types.insert(id, ElementType { id, name, data, info: Default::default(), uid: Default::default() });
        }

        p.add_default_systems(&n);

        let add_view = |p: &mut Project, name: &str, scale: u32, kind: ViewKind| {
            let id = ViewId(p.alloc_id());
            p.views.insert(id, View::new(id, name, scale, kind));
            id
        };
        let (plan_scale, model_scale) = (s.plan_scale, s.model_scale);
        let plan = add_view(&mut p, &n("tpl-level-ground"), plan_scale, ViewKind::floor_plan(ground));
        add_view(&mut p, &n("tpl-level-first"), plan_scale, ViewKind::floor_plan(first));
        add_view(&mut p, &n("tpl-level-ground"), plan_scale, ViewKind::ceiling_plan(ground));
        add_view(&mut p, &n("tpl-level-first"), plan_scale, ViewKind::ceiling_plan(first));
        let threed = add_view(
            &mut p,
            "{3D}",
            model_scale,
            ViewKind::ThreeD { camera: CameraState::default(), analytical: false, rebar: false },
        );
        for (name, direction) in [
            (&n("tpl-elev-north"), Direction::North),
            (&n("tpl-elev-south"), Direction::South),
            (&n("tpl-elev-east"), Direction::East),
            (&n("tpl-elev-west"), Direction::West),
        ] {
            add_view(&mut p, name, model_scale, ViewKind::Elevation { direction });
        }
        let all_types: Vec<_> = p.types.values().filter(|t| t.category().is_model()).map(|t| t.id).collect();
        add_view(&mut p, &n("tpl-legend"), plan_scale, ViewKind::Legend { types: all_types });
        for (name, category) in [
            (&n("tpl-schedule-doors"), Category::Door),
            (&n("tpl-schedule-windows"), Category::Window),
            (&n("tpl-schedule-walls"), Category::Wall),
            (&n("tpl-schedule-rooms"), Category::Room),
        ] {
            add_view(&mut p, name, 1, ViewKind::Schedule { category, def: Default::default() });
        }
        let (size, [plan_at, threed_at]) = s.sheet;
        add_view(
            &mut p,
            &n("tpl-sheet-plans"),
            1,
            ViewKind::Sheet {
                number: "A101".into(),
                size,
                portrait: false,
                title_block: None,
                fields: Default::default(),
                issue_date: String::new(),
                viewports: vec![Viewport { view: plan, center: plan_at }, Viewport { view: threed, center: threed_at }],
            },
        );
        let tb = p.add_standard_title_block(&n);
        p.give_sheets_title_block(tb);
        p.ensure_phases(&n);
        p.add_french_doors(&names, &n, &s.french_doors);
        p.ensure_uids();
        // Last, so the ids of everything above stay as they were.
        p.convert_sections_to_profiles(&n);
        p
    }

    /// The template's French doors (glazed, from the floor), of one leaf
    /// and of two, shown in its legend. Added last, so the ids of what came
    /// before stay as they were.
    fn add_french_doors(
        &mut self,
        names: &crate::openings::OpeningNames,
        n: &dyn Fn(&str) -> String,
        doors: &[crate::template_spec::Sized2; 2],
    ) {
        use crate::DoorOperation as D;
        let (wood, glass) = (n("tpl-mat-wood"), n("tpl-mat-glass"));
        let mut added = Vec::new();
        for ((key, width, height), op) in doors.iter().copied().zip([D::SingleSwingLeft, D::DoubleDoor]) {
            let size = (width, height);
            let def = crate::openings::french_door_def(
                names,
                (size.0, size.1, crate::openings::BOTTOM_RAIL),
                (&wood, &glass),
                op,
            );
            let data = TypeData::Component { component: names.door_for(op, true).clone(), def };
            let id = ElementTypeId(self.alloc_id());
            self.types
                .insert(id, ElementType { id, name: n(key), data, info: Default::default(), uid: Default::default() });
            added.push(id);
        }
        for view in self.views.values_mut() {
            if let ViewKind::Legend { types } = &mut view.kind {
                types.extend(added.iter().copied());
            }
        }
    }

    /// Add the standard title block type (named by `n`) and return its id.
    pub fn add_standard_title_block(&mut self, n: &dyn Fn(&str) -> String) -> ElementTypeId {
        let id = ElementTypeId(self.alloc_id());
        let name = n(crate::title_block::STANDARD_KEY);
        let data = TypeData::Component { component: name.clone(), def: crate::title_block::standard(n) };
        self.types.insert(id, ElementType { id, name, data, info: Default::default(), uid: Default::default() });
        id
    }

    /// Point every sheet without a title block at `id`; returns how many.
    pub fn give_sheets_title_block(&mut self, id: ElementTypeId) -> usize {
        let mut n = 0;
        for v in self.views.values_mut() {
            if let ViewKind::Sheet { title_block: t @ None, .. } = &mut v.kind {
                *t = Some(id);
                n += 1;
            }
        }
        n
    }

    /// Assign persistent unique ids to the project, types and elements that
    /// lack one, and replace duplicates (e.g. from copied elements).
    pub fn ensure_uids(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut fresh = |uid: &mut crate::Uid| {
            if uid.is_nil() || !seen.insert(*uid) {
                *uid = crate::Uid::generate();
                seen.insert(*uid);
            }
        };
        fresh(&mut self.info.uid);
        self.types.values_mut().for_each(|t| fresh(&mut t.uid));
        self.elements.values_mut().for_each(|e| fresh(&mut e.uid));
    }

    /// Add default room, grid and dimension types to projects created before
    /// those categories existed, sized and named (in Portuguese) like the
    /// template of the project's measurement system.
    pub fn ensure_annotation_types(&mut self) {
        let s = crate::template_spec::TemplateSpec::of(self.measurement_system());
        let n = |key: &str| template_default(s.library_key(key)).to_string();
        let concrete = n("tpl-mat-concrete");
        let (column_key, side) = s.column_rect;
        let (beam_key, beam_width, beam_height) = s.beam;
        let (stair_key, stair_width, max_riser, tread) = s.stair;
        let (railing_key, railing_height, post_spacing) = s.railing;
        let defaults = [
            (
                Category::Column,
                column_key,
                TypeData::Column {
                    section: ColumnSection::Rectangular { width: side, depth: side },
                    material: concrete.clone(),
                    profile: None,
                    profile_rotation_deg: 0.0,
                    profile_mirror: false,
                },
            ),
            (
                Category::Beam,
                beam_key,
                TypeData::Beam {
                    width: beam_width,
                    height: beam_height,
                    material: concrete.clone(),
                    shape: SectionShape::Solid,
                    profile: None,
                    profile_rotation_deg: 0.0,
                    profile_mirror: false,
                },
            ),
            (
                Category::Stair,
                stair_key,
                TypeData::Stair {
                    width: stair_width,
                    max_riser,
                    tread,
                    material: concrete.clone(),
                    build: Default::default(),
                    rules: crate::template_spec::Defaults::of(self.measurement_system()).stair_rules,
                },
            ),
            (
                Category::Railing,
                railing_key,
                TypeData::Railing { height: railing_height, post_spacing, material: n("tpl-mat-wood") },
            ),
            (Category::Room, "tpl-type-room", TypeData::Room {}),
            (Category::Grid, s.grid.0, TypeData::Grid { bubble: s.grid.1 }),
            (
                Category::Dimension,
                s.dimension.0,
                TypeData::Dimension { text_size: s.dimension.1, style: DimensionStyle::of(self.measurement_system()) },
            ),
            (Category::Text, s.texts[0].0, TypeData::Text { size: s.texts[0].1, symbol: crate::LeaderSymbol::Arrow }),
        ];
        for (category, key, data) in defaults {
            if self.default_type(category).is_none() {
                let id = ElementTypeId(self.alloc_id());
                self.types.insert(
                    id,
                    ElementType { id, name: n(key), data, info: Default::default(), uid: crate::Uid::generate() },
                );
            }
        }
        // Projects created before foundations get a pad, a strip and a slab type.
        if self.default_type(Category::Foundation).is_none() {
            let kinds = [crate::FoundationKind::Pad, crate::FoundationKind::Strip, crate::FoundationKind::Slab];
            for ((key, width, length, thickness), kind) in s.footings.into_iter().zip(kinds) {
                let id = ElementTypeId(self.alloc_id());
                let data = TypeData::Foundation {
                    kind,
                    width,
                    length,
                    thickness,
                    material: concrete.clone(),
                    haunch: None,
                    pile_cap: None,
                };
                self.types.insert(
                    id,
                    ElementType { id, name: n(key), data, info: Default::default(), uid: crate::Uid::generate() },
                );
            }
        }
        // Projects created before reinforcement get the usual bar diameters, in steel.
        if self.default_type(Category::Rebar).is_none() {
            let steel = n("tpl-mat-steel");
            if !self.materials.iter().any(|m| m.name == steel) {
                let mut m = Material::new(steel.as_str(), [140, 146, 152]);
                crate::material::template_properties(&mut m, crate::material::MaterialClass::Metal);
                self.materials.push(m);
            }
            for (key, diameter) in s.rebars {
                let id = ElementTypeId(self.alloc_id());
                let data = TypeData::rebar(diameter, steel.clone());
                self.types.insert(
                    id,
                    ElementType { id, name: n(key), data, info: Default::default(), uid: crate::Uid::generate() },
                );
            }
        }
        // Projects created before building services get pipes, conduits, the
        // fixtures and devices of the library and the usual plumbing systems.
        if self.default_type(Category::Pipe).is_none() {
            let pvc = n("tpl-mat-pvc");
            if !self.materials.iter().any(|m| m.name == pvc) {
                let mut m = Material::new(pvc.as_str(), [236, 236, 230]);
                crate::material::template_properties(&mut m, crate::material::MaterialClass::Plastic);
                self.materials.push(m);
            }
            // Two supply pipes and two drain pipes.
            for (key, diameter) in [0, 1, 4, 6].into_iter().filter_map(|i| s.pipes.get(i).copied()) {
                let id = ElementTypeId(self.alloc_id());
                let data = TypeData::Pipe { diameter, material: pvc.clone() };
                self.types.insert(
                    id,
                    ElementType { id, name: n(key), data, info: Default::default(), uid: crate::Uid::generate() },
                );
            }
            // Projects without the library get all of it below.
            if self.default_type(Category::Component).is_some() {
                for (component, name, def) in s.sized_library(&n, crate::component::mep_library(&n)) {
                    let id = ElementTypeId(self.alloc_id());
                    let data = TypeData::Component { component, def };
                    self.types.insert(
                        id,
                        ElementType { id, name, data, info: Default::default(), uid: crate::Uid::generate() },
                    );
                }
            }
            if self.systems.is_empty() {
                self.add_default_systems(&n);
            }
        }
        if self.default_type(Category::Conduit).is_none() {
            let pvc = n("tpl-mat-pvc");
            for (key, diameter) in s.conduits.into_iter().take(2) {
                let id = ElementTypeId(self.alloc_id());
                let data = TypeData::Conduit { diameter, material: pvc.clone() };
                self.types.insert(
                    id,
                    ElementType { id, name: n(key), data, info: Default::default(), uid: crate::Uid::generate() },
                );
            }
        }
        // Projects created before components get the default library.
        if self.default_type(Category::Component).is_none() {
            use crate::material::MaterialClass as C;
            for (key, color, class) in
                [("tpl-mat-fabric", [120, 132, 150], C::Fabric), ("tpl-mat-porcelain", [240, 240, 236], C::Ceramic)]
            {
                let name = n(key);
                if !self.materials.iter().any(|m| m.name == name) {
                    let mut m = Material::new(name, color);
                    crate::material::template_properties(&mut m, class);
                    self.materials.push(m);
                }
            }
            for (component, name, def) in s.sized_library(&n, crate::component::library(&n)) {
                let id = ElementTypeId(self.alloc_id());
                let data = TypeData::Component { component, def };
                self.types
                    .insert(id, ElementType { id, name, data, info: Default::default(), uid: crate::Uid::generate() });
            }
        }
    }

    /// The measurement system of the project: imperial when its lengths
    /// show in an imperial unit (see [`crate::Units::system`]).
    pub fn measurement_system(&self) -> crate::MeasurementSystem {
        self.units.system()
    }

    /// The sizes and rules things made outside the template get in this
    /// project's measurement system.
    pub fn defaults(&self) -> &'static crate::Defaults {
        crate::Defaults::of(self.measurement_system())
    }

    /// Add the usual plumbing systems (names and codes from `n`, keys `tpl-sys-*`).
    fn add_default_systems(&mut self, n: &dyn Fn(&str) -> String) {
        use crate::SystemKind as K;
        for (name, code, kind) in [
            ("tpl-sys-cold-water", "tpl-sys-cold-water-code", K::ColdWater),
            ("tpl-sys-hot-water", "tpl-sys-hot-water-code", K::HotWater),
            ("tpl-sys-sanitary", "tpl-sys-sanitary-code", K::Sanitary),
            ("tpl-sys-vent", "tpl-sys-vent-code", K::Vent),
            ("tpl-sys-rainwater", "tpl-sys-rainwater-code", K::Rainwater),
        ] {
            let id = crate::SystemId(self.alloc_id());
            self.systems.insert(id, crate::MepSystem::new(n(name), n(code), kind));
        }
    }

    pub fn system(&self, id: crate::SystemId) -> Option<&crate::MepSystem> {
        self.systems.get(&id)
    }

    /// Display color of a pipe, conduit, cable tray or device: its (first)
    /// system's, or gray; a valve takes its pipe's.
    pub fn system_color(&self, element: &Element) -> [u8; 3] {
        let element = match &element.kind {
            ElementKind::Valve(v) => self.valve_host(v).map_or(element, |(host, _)| host),
            _ => element,
        };
        // Lightning protection keeps its color outside any system.
        let lightning = matches!(element.kind, ElementKind::LightningConductor(_))
            || self
                .mep_data(element)
                .and_then(|m| m.device())
                .is_some_and(|d| d.class() == crate::SystemClass::Lightning);
        let fallback = if lightning { crate::SystemKind::Lightning.default_color() } else { [128, 128, 128] };
        element.systems().first().and_then(|s| self.system(*s)).map_or(fallback, crate::MepSystem::color)
    }

    /// The MEP data of a component element's type.
    pub fn mep_data(&self, element: &Element) -> Option<crate::MepData> {
        match (&element.kind, self.element_type(element.type_id).map(|t| &t.data)) {
            (ElementKind::Component(_), Some(TypeData::Component { def, .. })) => def.mep,
            _ => None,
        }
    }

    /// The board or rack feeding a system (its `panel`, when that is still a
    /// component element).
    pub fn feeder(&self, system: crate::SystemId) -> Option<&Element> {
        let e = self.element(self.system(system)?.panel?)?;
        matches!(e.kind, ElementKind::Component(_)).then_some(e)
    }

    /// Where a component element stands in the world: its insertion point at
    /// its base (`None` for other elements).
    pub fn component_point(&self, element: &Element) -> Option<glam::DVec3> {
        let ElementKind::Component(c) = &element.kind else { return None };
        Some(c.position.extend(self.level_elevation(element.level) + c.base_offset))
    }

    /// The estimated length of the horizontal cables of a data outlet: one
    /// per jack, each routed to the rack of its data system as
    /// [`crate::mep::cable_length`] says (up to the ceiling of the outlet's
    /// level, across and down to the rack). `None` for elements that are not
    /// data outlets cabled to a rack.
    pub fn outlet_cable(&self, element: &Element) -> Option<(u32, f64)> {
        let mep = self.mep_data(element)?;
        let ports = mep.cables();
        let ElementKind::Component(c) = &element.kind else { return None };
        if ports == 0 {
            return None;
        }
        let system = c.system.filter(|s| self.system(*s).is_some_and(|x| x.kind == crate::SystemKind::Data))?;
        let rack = self.feeder(system)?;
        let (outlet, rack) = (self.component_point(element)?, self.component_point(rack)?);
        let ceiling = self.level_elevation(element.level) + self.level_height(element.level, 2.8);
        Some((ports, crate::mep::cable_length(outlet, rack, ceiling)))
    }

    /// A pipe, conduit or cable tray run as seen from `level`, with its elevations above
    /// that level: the level's own runs, and the vertical runs of other levels
    /// that pass through it (risers between floors, reaching at least its floor
    /// and starting below its top). `None` for other elements and runs.
    pub fn segment_at(&self, element: &Element, level: LevelId) -> Option<crate::MepSegment> {
        let s = element.as_segment()?;
        if element.level == level {
            return Some(s.clone());
        }
        if !s.is_vertical() || self.level(level).is_none() {
            return None;
        }
        let shift = self.level_elevation(element.level) - self.level_elevation(level);
        let (lo, hi) = (s.start_offset.min(s.end_offset) + shift, s.start_offset.max(s.end_offset) + shift);
        let top = self.level_height(level, 3.0);
        (hi >= -0.01 && lo < top - 0.01 && shift.is_finite()).then(|| crate::MepSegment {
            start_offset: s.start_offset + shift,
            end_offset: s.end_offset + shift,
            ..s.clone()
        })
    }

    /// The pipe, conduit and cable tray runs seen from `level` (see [`Project::segment_at`]).
    pub fn segments_at(&self, level: LevelId) -> impl Iterator<Item = (&Element, crate::MepSegment)> {
        self.elements.values().filter_map(move |e| Some((e, self.segment_at(e, level)?)))
    }

    /// Elements in system `id`: its pipes, the conduits carrying the circuit
    /// and the components connected to it.
    pub fn system_members(&self, id: crate::SystemId) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(move |e| e.systems().contains(&id))
    }

    /// The level above `id`, or `id` itself plus `fallback` meters: the default upper limit of rooms.
    pub fn level_height(&self, id: LevelId, fallback: f64) -> f64 {
        self.level_above(id).map_or(fallback, |above| above.elevation - self.level_elevation(id))
    }

    /// Base elevation and top elevation of a column (top at least 1 cm above).
    pub fn column_span(&self, element: &Element, column: &crate::Column) -> (f64, f64) {
        let base = self.level_elevation(element.level) + column.base_offset;
        let top = match column.top {
            crate::WallTop::Unconnected => base + column.height,
            crate::WallTop::Level { level, offset } => self.level_elevation(level) + offset,
        };
        (base, top.max(base + 0.01))
    }

    /// Base and top elevation of a stair (the top at least one riser above).
    pub fn stair_span(&self, element: &Element, stair: &crate::Stair) -> (f64, f64) {
        let base = self.level_elevation(element.level) + stair.base_offset;
        let top = match stair.top {
            crate::WallTop::Unconnected => base + stair.height,
            crate::WallTop::Level { level, offset } => self.level_elevation(level) + offset,
        };
        (base, top.max(base + 0.05))
    }

    /// Base and top elevation of a ramp (the top at least 1 cm above).
    pub fn ramp_span(&self, element: &Element, ramp: &crate::Ramp) -> (f64, f64) {
        let base = self.level_elevation(element.level) + ramp.base_offset;
        let top = match ramp.top {
            crate::WallTop::Unconnected => base + ramp.height,
            crate::WallTop::Level { level, offset } => self.level_elevation(level) + offset,
        };
        (base, top.max(base + 0.01))
    }

    /// Elevation of a wall's base.
    pub fn wall_base_z(&self, element: &Element, wall: &crate::Wall) -> f64 {
        self.level_elevation(element.level) + wall.base_offset
    }

    /// Elevation of a wall's top, from its top constraint (at least 1 cm above the base).
    pub fn wall_top_z(&self, element: &Element, wall: &crate::Wall) -> f64 {
        let base = self.wall_base_z(element, wall);
        let top = match wall.top {
            crate::WallTop::Unconnected => base + wall.height,
            crate::WallTop::Level { level, offset } => self.level_elevation(level) + offset,
        };
        top.max(base + 0.01)
    }

    /// Effective height of a wall.
    pub fn wall_height(&self, element: &Element, wall: &crate::Wall) -> f64 {
        self.wall_top_z(element, wall) - self.wall_base_z(element, wall)
    }

    /// Start and end of a wall's body centerline: the location line shifted
    /// across the wall according to `location_line` and the type's layers
    /// (listed from the exterior face, on the left of the line). An arc
    /// wall's centerline is concentric with its location line and has the
    /// same bulge.
    pub fn wall_axis(&self, element: &Element, wall: &crate::Wall) -> (DVec2, DVec2) {
        wall.offset_line(-self.wall_line_offset(element, wall))
    }

    /// Length of a wall's body centerline (the location line's length for
    /// straight walls; arc walls measure along the centerline arc).
    pub fn wall_length(&self, element: &Element, wall: &crate::Wall) -> f64 {
        let (start, end) = self.wall_axis(element, wall);
        crate::Wall { start, end, ..wall.clone() }.length()
    }

    /// Position of a wall's location line across the wall, measured from the
    /// centerline towards the exterior (left) face.
    fn wall_line_offset(&self, element: &Element, wall: &crate::Wall) -> f64 {
        self.element_type(element.type_id).map_or(0.0, |ty| ty.location_offset(wall.location_line))
    }

    /// The beam's body centerline: its location line moved across by the
    /// lateral justification.
    pub fn beam_axis(&self, element: &Element, beam: &crate::Beam) -> (DVec2, DVec2) {
        let width = match self.element_type(element.type_id).map(|t| &t.data) {
            Some(crate::TypeData::Beam { width, .. }) => *width,
            _ => 0.0,
        };
        // The location line runs along the left face: the body lies to its right.
        let side = match beam.lateral {
            crate::BeamLateral::Center => 0.0,
            crate::BeamLateral::Left => -width * 0.5,
            crate::BeamLateral::Right => width * 0.5,
        };
        match beam.arc() {
            // A concentric arc: the same sweep, so the same bulge.
            Some(arc) if arc.radius_at(side) > 1e-6 => (arc.at(0.0, side), arc.at(arc.length(), side)),
            Some(_) => (beam.start, beam.end),
            None => {
                let shift = (beam.end - beam.start).normalize_or_zero().perp() * side;
                (beam.start + shift, beam.end + shift)
            }
        }
    }

    /// Bottom and top elevations of a beam at its start, from its location
    /// elevation and vertical justification. A sloped beam's section stays
    /// square to its axis, so its vertical depth is a little more than its height.
    pub fn beam_z(&self, element: &Element, beam: &crate::Beam) -> (f64, f64) {
        let height = match self.element_type(element.type_id).map(|t| &t.data) {
            Some(crate::TypeData::Beam { height, .. }) => *height,
            _ => 0.0,
        };
        let length = beam.length();
        let slope = if length > 1e-9 && beam.arc().is_none() { beam.rise() / length } else { 0.0 };
        let height = height * slope.hypot(1.0);
        let z = self.level_elevation(element.level) + beam.top_offset;
        match beam.vertical {
            crate::BeamVertical::Top => (z - height, z),
            crate::BeamVertical::Center => (z - height * 0.5, z + height * 0.5),
            crate::BeamVertical::Bottom => (z, z + height),
        }
    }

    /// Allocate a new unique id (shared by all id kinds).
    /// The id the next new element, view or type gets (every id after it is
    /// newer).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// A copy of the project with `types` in place of its own (which are not copied).
    pub(crate) fn copy_with_types(&self, types: BTreeMap<ElementTypeId, ElementType>) -> Project {
        // Destructured so that a new field is not forgotten.
        let Project {
            info,
            levels,
            materials,
            types: _,
            elements,
            views,
            units,
            pinned,
            joins,
            groups,
            systems,
            filters,
            templates,
            phases,
            phase_filters,
            phase_graphics,
            graph_bindings,
            line_styles,
            fill_patterns,
            option_sets,
            browser,
            bar_marking,
            profiles,
            sun_presets,
            active_option,
            next_id,
        } = self;
        Project {
            info: info.clone(),
            levels: levels.clone(),
            materials: materials.clone(),
            types,
            elements: elements.clone(),
            views: views.clone(),
            units: units.clone(),
            pinned: pinned.clone(),
            joins: joins.clone(),
            groups: groups.clone(),
            systems: systems.clone(),
            filters: filters.clone(),
            templates: templates.clone(),
            phases: phases.clone(),
            phase_filters: phase_filters.clone(),
            phase_graphics: *phase_graphics,
            graph_bindings: graph_bindings.clone(),
            line_styles: line_styles.clone(),
            fill_patterns: fill_patterns.clone(),
            option_sets: option_sets.clone(),
            browser: browser.clone(),
            bar_marking: bar_marking.clone(),
            profiles: profiles.clone(),
            sun_presets: sun_presets.clone(),
            active_option: *active_option,
            next_id: *next_id,
        }
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// The largest id any object of the project uses (0 if none).
    pub fn max_id(&self) -> u64 {
        let levels = self.levels.iter().map(|l| l.id.0);
        let phases = self.phases.iter().map(|p| p.id.0);
        let types = self.types.keys().map(|k| k.0);
        let elements = self.elements.keys().map(|k| k.0);
        let views = self.views.keys().map(|k| k.0);
        let groups = self.groups.keys().map(|k| k.0);
        let systems = self.systems.keys().map(|k| k.0);
        let filters = self.filters.keys().map(|k| k.0);
        let templates = self.templates.keys().map(|k| k.0);
        let phase_filters = self.phase_filters.keys().map(|k| k.0);
        let line_styles = self.line_styles.keys().map(|k| k.0);
        let fill_patterns = self.fill_patterns.keys().map(|k| k.0);
        let profiles = self.profiles.keys().map(|k| k.0);
        let option_sets = self.option_sets.iter().map(|s| s.id.0);
        let options = self.option_sets.iter().flat_map(|s| s.options.iter().map(|o| o.id.0));
        let schemes = self.browser.schemes.iter().map(|s| s.id.0);
        levels
            .chain(phases)
            .chain(types)
            .chain(elements)
            .chain(views)
            .chain(groups)
            .chain(systems)
            .chain(filters)
            .chain(templates)
            .chain(phase_filters)
            .chain(line_styles)
            .chain(fill_patterns)
            .chain(profiles)
            .chain(option_sets)
            .chain(options)
            .chain(schemes)
            .max()
            .unwrap_or(0)
    }

    /// Move the id counter past every id in use, so new objects never take
    /// the id of an existing one (a file may carry a wrong counter), and back
    /// from the end of the range, where it could not move on. Returns
    /// whether it moved.
    pub fn ensure_next_id(&mut self) -> bool {
        let next = self.max_id().saturating_add(1);
        if self.next_id < next || (self.next_id == u64::MAX && next < u64::MAX) {
            self.next_id = next;
            true
        } else {
            false
        }
    }

    pub fn level(&self, id: LevelId) -> Option<&Level> {
        self.levels.iter().find(|l| l.id == id)
    }

    pub fn level_elevation(&self, id: LevelId) -> f64 {
        self.level(id).map_or(0.0, |l| l.elevation)
    }

    pub fn level_name(&self, id: LevelId) -> &str {
        self.level(id).map_or("?", |l| l.name.as_str())
    }

    pub fn is_pinned(&self, id: ElementId) -> bool {
        self.pinned.contains(&id)
    }

    /// The join between two elements as `(cutting, cut)`, if they are joined.
    pub fn join_of(&self, a: ElementId, b: ElementId) -> Option<(ElementId, ElementId)> {
        [(a, b), (b, a)].into_iter().find(|pair| self.joins.contains(pair))
    }

    /// The group an element belongs to.
    pub fn group_of(&self, id: ElementId) -> Option<crate::GroupId> {
        self.groups.iter().find(|(_, g)| g.members.contains(&id)).map(|(gid, _)| *gid)
    }

    /// `ids` plus every member of the groups they belong to.
    pub fn with_groups(&self, ids: impl IntoIterator<Item = ElementId>) -> std::collections::BTreeSet<ElementId> {
        let mut out = std::collections::BTreeSet::new();
        for id in ids {
            match self.group_of(id).and_then(|g| self.groups.get(&g)) {
                Some(group) => out.extend(group.members.iter().copied()),
                None => {
                    out.insert(id);
                }
            }
        }
        out
    }

    /// Elements whose geometry is joined with `id`.
    pub fn joined_with(&self, id: ElementId) -> impl Iterator<Item = ElementId> + '_ {
        self.joins.iter().filter_map(move |&(a, b)| (a == id).then_some(b).or((b == id).then_some(a)))
    }

    pub fn element(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    pub fn element_type(&self, id: ElementTypeId) -> Option<&ElementType> {
        self.types.get(&id)
    }

    pub fn type_name(&self, id: ElementTypeId) -> &str {
        self.types.get(&id).map_or("?", |t| t.name.as_str())
    }

    /// Wall thickness for an element's type (0 if not a layered type).
    pub fn thickness_of(&self, element: &Element) -> f64 {
        self.element_type(element.type_id).map_or(0.0, |t| t.thickness())
    }

    pub fn types_of(&self, category: Category) -> impl Iterator<Item = &ElementType> {
        self.types.values().filter(move |t| t.category() == category)
    }

    pub fn default_type(&self, category: Category) -> Option<ElementTypeId> {
        self.types_of(category).next().map(|t| t.id)
    }

    pub fn material_color(&self, name: &str) -> [u8; 3] {
        self.material(name).map_or([200, 200, 200], |m| m.shading_color())
    }

    /// Transparency of material `name` (0 opaque, 1 invisible).
    pub fn material_transparency(&self, name: &str) -> f64 {
        self.material(name).map_or(0.0, |m| m.graphics.transparency.clamp(0.0, 1.0))
    }

    pub fn material(&self, name: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.name == name)
    }

    /// Types that use the material `name`.
    pub fn material_users(&self, name: &str) -> Vec<ElementTypeId> {
        self.types.values().filter(|t| t.data.uses_material(name)).map(|t| t.id).collect()
    }

    /// Components whose own value of a material parameter is `name`.
    pub fn material_instance_users(&self, name: &str) -> Vec<ElementId> {
        self.elements
            .values()
            .filter(|e| {
                let crate::ElementKind::Component(c) = &e.kind else { return false };
                let Some(crate::TypeData::Component { def, .. }) = self.element_type(e.type_id).map(|t| &t.data) else {
                    return false;
                };
                c.text_overrides.iter().any(|(param, v)| {
                    v == name && def.param(param).is_some_and(|p| p.kind == crate::ParamKind::Material)
                })
            })
            .map(|e| e.id)
            .collect()
    }

    /// Elements with a face painted in material `name`.
    pub fn material_paint_users(&self, name: &str) -> Vec<ElementId> {
        self.elements.values().filter(|e| e.paints.iter().any(|p| p.material == name)).map(|e| e.id).collect()
    }

    /// Doors and windows hosted by `wall`.
    pub fn hosted_by(&self, wall: ElementId) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(move |e| e.as_hosted().is_some_and(|h| h.host == wall))
    }

    /// The railings that follow stair `host`.
    pub fn railings_of(&self, host: ElementId) -> impl Iterator<Item = &Element> {
        self.elements
            .values()
            .filter(move |e| matches!(&e.kind, ElementKind::Railing(r) if r.stair.is_some_and(|h| h.host == host)))
    }

    /// The reinforcement sets of beam or column `host`.
    pub fn rebar_of(&self, host: ElementId) -> impl Iterator<Item = (&Element, &crate::Rebar)> {
        self.elements.values().filter_map(move |e| match &e.kind {
            ElementKind::Rebar(r) if r.host == host => Some((e, r)),
            _ => None,
        })
    }

    /// The bar tags of reinforcement set `set`.
    pub fn bar_tags_of(&self, set: ElementId) -> impl Iterator<Item = (&Element, &crate::bar_tag::BarTag)> {
        self.elements.values().filter_map(move |e| match &e.kind {
            ElementKind::BarTag(t) if t.set == set => Some((e, t.as_ref())),
            _ => None,
        })
    }

    /// The valves on pipe `host`.
    pub fn valves_of(&self, host: ElementId) -> impl Iterator<Item = (&Element, &crate::Valve)> {
        self.elements.values().filter_map(move |e| match &e.kind {
            ElementKind::Valve(v) if v.host == host => Some((e, v)),
            _ => None,
        })
    }

    /// A valve's pipe element and its run.
    pub fn valve_host(&self, valve: &crate::Valve) -> Option<(&Element, &crate::MepSegment)> {
        let host = self.element(valve.host)?;
        match &host.kind {
            ElementKind::Pipe(s) => Some((host, s)),
            _ => None,
        }
    }

    pub fn elements_of(&self, category: Category) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(move |e| e.category() == category)
    }

    pub fn walls(&self) -> impl Iterator<Item = (&Element, &crate::Wall)> {
        self.elements.values().filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some((e, w)),
            _ => None,
        })
    }

    pub fn views_of(&self, category: ViewCategory) -> impl Iterator<Item = &View> {
        self.views.values().filter(move |v| v.category() == category)
    }

    pub fn view(&self, id: ViewId) -> Option<&View> {
        self.views.get(&id)
    }

    pub fn filter(&self, id: crate::FilterId) -> Option<&crate::ViewFilter> {
        self.filters.get(&id)
    }

    pub fn line_style(&self, id: crate::LineStyleId) -> Option<&crate::LineStyleDef> {
        self.line_styles.get(&id)
    }

    /// The detail lines and regions drawn with a project line style.
    pub fn line_style_users(&self, id: crate::LineStyleId) -> Vec<ElementId> {
        let wanted = Some(crate::LineStyleRef::Project(id));
        self.elements
            .values()
            .filter(|e| match &e.kind {
                ElementKind::DetailLine(l) => Some(l.style) == wanted,
                ElementKind::FilledRegion(r) | ElementKind::MaskingRegion(r) => r.boundary == wanted,
                _ => false,
            })
            .map(|e| e.id)
            .collect()
    }

    pub fn template(&self, id: crate::TemplateId) -> Option<&crate::ViewTemplate> {
        self.templates.get(&id)
    }

    /// A fresh mark for a new element: the next number among its category.
    pub fn next_mark(&self, category: Category) -> String {
        let max = self.elements_of(category).filter_map(|e| e.mark.parse::<u32>().ok()).max().unwrap_or(0);
        (max + 1).to_string()
    }

    /// The level directly above `id`, if any.
    pub fn level_above(&self, id: LevelId) -> Option<&Level> {
        let elevation = self.level_elevation(id);
        self.levels.iter().filter(|l| l.elevation > elevation + 1e-9).min_by(|a, b| a.elevation.total_cmp(&b.elevation))
    }

    /// Before `e` moves from its level to `to`: a top attached to a level
    /// moves up or down as many levels as the base (a column from the ground
    /// floor to the first floor pasted on the first floor reaches the second).
    /// When there is no such level, or it is not above the new base, the top
    /// is unconnected at the element's height.
    pub fn move_top_with_base(&self, e: &mut crate::Element, to: LevelId) {
        let from = e.level;
        if from == to {
            return;
        }
        let p = self;
        let span = |(base, top): (f64, f64)| top - base;
        let height = match &e.kind {
            crate::ElementKind::Wall(w) => p.wall_height(e, w),
            crate::ElementKind::Column(c) => span(p.column_span(e, c)),
            crate::ElementKind::Stair(s) => span(p.stair_span(e, s)),
            crate::ElementKind::Ramp(r) => span(p.ramp_span(e, r)),
            _ => return,
        };
        let (top, own_height, base_offset) = match &mut e.kind {
            crate::ElementKind::Wall(w) => (&mut w.top, &mut w.height, w.base_offset),
            crate::ElementKind::Column(c) => (&mut c.top, &mut c.height, c.base_offset),
            crate::ElementKind::Stair(s) => (&mut s.top, &mut s.height, s.base_offset),
            crate::ElementKind::Ramp(r) => (&mut r.top, &mut r.height, r.base_offset),
            _ => return,
        };
        let crate::WallTop::Level { level: top_level, offset } = *top else { return };
        let mut order: Vec<&crate::Level> = p.levels.iter().collect();
        order.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let index = |id: LevelId| order.iter().position(|l| l.id == id);
        let shifted = match (index(from), index(top_level), index(to)) {
            (Some(f), Some(t), Some(n)) => (n + t).checked_sub(f).and_then(|i| order.get(i)),
            _ => None,
        };
        let base_z = p.level_elevation(to) + base_offset;
        match shifted {
            Some(l) if l.elevation + offset > base_z + 1e-6 => *top = crate::WallTop::Level { level: l.id, offset },
            _ => {
                *top = crate::WallTop::Unconnected;
                if height.is_finite() && height > 0.0 {
                    *own_height = height;
                }
            }
        }
    }

    pub fn sort_levels(&mut self) {
        self.levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    }
}

/// Portuguese names of the default template, keyed like the translation files.
pub fn template_default(key: &str) -> &'static str {
    match key {
        "tpl-project-name" => "Novo projeto",
        "tpl-level-ground" => "Térreo",
        "tpl-level-first" => "Pavimento 1",
        "tpl-type-masonry-15" => "Alvenaria 15 cm",
        "tpl-type-masonry-25" => "Alvenaria 25 cm",
        "tpl-type-drywall-10" => "Divisória gesso 10 cm",
        "tpl-type-door-80" => "Porta 80x210",
        "tpl-type-door-90" => "Porta 90x210",
        "tpl-type-window-120" => "Janela 120x120",
        "tpl-type-window-150" => "Janela 150x100",
        "tpl-type-window-casement-60" => "Janela de abrir 60x100",
        "tpl-type-window-double-120" => "Janela de abrir 2 folhas 120x120",
        "tpl-comp-window-casement" => "Janela de abrir",
        "tpl-comp-window-double" => "Janela de abrir 2 folhas",
        "tpl-comp-window-top-hung" => "Janela maxim-ar",
        "tpl-comp-window-pivot-h" => "Janela basculante",
        "tpl-comp-window-pivot-v" => "Janela pivotante",
        "tpl-comp-window-sliding" => "Janela de correr",
        "tpl-type-window-top-hung-60" => "Janela maxim-ar 60x60",
        "tpl-type-window-sliding-150" => "Janela de correr 150x120",
        "tpl-comp-door" => "Porta de giro",
        "tpl-comp-door-double" => "Porta de giro 2 folhas",
        "tpl-comp-french-door" => "Porta-janela",
        "tpl-comp-french-door-double" => "Porta-janela 2 folhas",
        "tpl-param-bottom-rail" => "Rodapé",
        "tpl-type-french-door-80" => "Porta-janela 80x220",
        "tpl-type-french-door-double-140" => "Porta-janela 2 folhas 140x220",
        "tpl-conn-sewer" => "Esgoto",
        "tpl-conn-cold-water" => "Água fria",
        "tpl-conn-hot-water" => "Água quente",
        "tpl-comp-window" => "Janela",
        "tpl-param-sill" => "Peitoril",
        "tpl-piece-frame" => "Batente",
        "tpl-piece-leaf" => "Folha",
        "tpl-piece-sash" => "Caixilho",
        "tpl-piece-glass" => "Vidro",
        "tpl-piece-sill-board" => "Pingadeira",
        "tpl-type-slab-12" => "Laje 12 cm",
        "tpl-type-ceiling-gypsum" => "Forro de gesso",
        "tpl-type-roof-tile" => "Telhado cerâmico",
        "tpl-mat-roof-tile" => "Telha cerâmica",
        "tpl-mat-masonry" => "Alvenaria",
        "tpl-mat-plaster" => "Reboco",
        "tpl-mat-concrete" => "Concreto",
        "tpl-mat-wood" => "Madeira",
        "tpl-mat-glass" => "Vidro",
        "tpl-mat-gypsum" => "Gesso",
        "tpl-mat-screed" => "Contrapiso",
        "tpl-mat-earth" => "Terra",
        "tpl-type-topography" => "Terreno",
        "tpl-type-pad" => "Platô",
        "tpl-elev-north" => "Norte",
        "tpl-elev-south" => "Sul",
        "tpl-elev-east" => "Leste",
        "tpl-elev-west" => "Oeste",
        "tpl-legend" => "Legenda de tipos",
        "tpl-schedule-doors" => "Tabela de portas",
        "tpl-schedule-windows" => "Tabela de janelas",
        "tpl-schedule-walls" => "Tabela de paredes",
        "tpl-sheet-plans" => "Plantas",
        "tpl-tb-standard" => "Carimbo padrão",
        "tpl-tb-project" => "PROJETO",
        "tpl-tb-sheet" => "FOLHA",
        "tpl-tb-client" => "CLIENTE",
        "tpl-tb-number" => "NÚMERO",
        "tpl-tb-project-number" => "Proj. nº",
        "tpl-tb-author" => "Autor:",
        "tpl-type-room" => "Ambiente",
        "tpl-type-column-rect" => "Pilar 20 x 20 cm",
        "tpl-type-column-round" => "Pilar circular 30 cm",
        "tpl-type-beam" => "Viga 15 x 40 cm",
        "tpl-type-stair" => "Escada 1,00 m",
        "tpl-type-railing" => "Guarda-corpo 1,10 m",
        "tpl-type-grid" => "Eixo 8 mm",
        "tpl-type-dimension" => "Cota linear 2,5 mm",
        "tpl-type-text" => "Texto 2,5 mm",
        "tpl-type-text-large" => "Texto 3,5 mm",
        "tpl-schedule-rooms" => "Tabela de ambientes",
        "tpl-room-name" => "Ambiente",
        "tpl-mat-fabric" => "Tecido",
        "tpl-mat-porcelain" => "Louça",
        "tpl-mat-steel" => "Aço",
        "tpl-type-column-steel-h" => "Pilar metálico H 200 x 200",
        "tpl-type-column-steel-tube" => "Pilar metálico tubo 150 x 150 x 6",
        "tpl-type-beam-steel-i" => "Viga metálica I 300 x 150",
        "tpl-type-footing-pad" => "Sapata 100 x 100 x 40 cm",
        "tpl-type-footing-strip" => "Viga baldrame 20 x 40 cm",
        "tpl-type-footing-slab" => "Radier 30 cm",
        "tpl-type-rebar-5" => "Barra Ø 5,0 mm",
        "tpl-type-rebar-8" => "Barra Ø 8,0 mm",
        "tpl-type-rebar-10" => "Barra Ø 10,0 mm",
        "tpl-type-rebar-12" => "Barra Ø 12,5 mm",
        "tpl-type-rebar-16" => "Barra Ø 16,0 mm",
        "tpl-param-width" => "Largura",
        "tpl-param-depth" => "Profundidade",
        "tpl-param-height" => "Altura",
        "tpl-param-seat-height" => "AlturaAssento",
        "tpl-part-top" => "Tampo",
        "tpl-part-leg" => "Perna",
        "tpl-part-seat" => "Assento",
        "tpl-part-back" => "Encosto",
        "tpl-part-arm" => "Braço",
        "tpl-part-mattress" => "Colchão",
        "tpl-part-headboard" => "Cabeceira",
        "tpl-part-bowl" => "Bacia",
        "tpl-part-tank" => "Caixa acoplada",
        "tpl-part-basin" => "Cuba",
        "tpl-part-pedestal" => "Coluna",
        "tpl-part-body" => "Corpo",
        "tpl-comp-table" => "Mesa",
        "tpl-comp-chair" => "Cadeira",
        "tpl-comp-bed" => "Cama",
        "tpl-comp-sofa" => "Sofá",
        "tpl-comp-toilet" => "Bacia sanitária",
        "tpl-comp-washbasin" => "Lavatório",
        "tpl-comp-wardrobe" => "Armário",
        "tpl-type-table" => "Mesa 120 x 80",
        "tpl-type-chair" => "Cadeira simples",
        "tpl-type-bed" => "Cama casal 140 x 190",
        "tpl-type-sofa" => "Sofá 3 lugares",
        "tpl-type-toilet" => "Bacia com caixa acoplada",
        "tpl-type-washbasin" => "Lavatório de coluna",
        "tpl-type-wardrobe" => "Armário 160 cm",
        "tpl-mat-pvc" => "PVC",
        "tpl-type-pipe-20" => "Tubo PVC soldável 20 mm",
        "tpl-type-pipe-25" => "Tubo PVC soldável 25 mm",
        "tpl-type-pipe-32" => "Tubo PVC soldável 32 mm",
        "tpl-type-pipe-sewer-40" => "Tubo PVC esgoto 40 mm",
        "tpl-type-pipe-sewer-50" => "Tubo PVC esgoto 50 mm",
        "tpl-type-pipe-sewer-75" => "Tubo PVC esgoto 75 mm",
        "tpl-type-pipe-sewer-100" => "Tubo PVC esgoto 100 mm",
        "tpl-type-conduit-20" => "Eletroduto flexível 20 mm",
        "tpl-type-conduit-25" => "Eletroduto flexível 25 mm",
        "tpl-type-conduit-32" => "Eletroduto flexível 32 mm",
        "tpl-sys-cold-water" => "Água fria",
        "tpl-sys-cold-water-code" => "AF",
        "tpl-sys-hot-water" => "Água quente",
        "tpl-sys-hot-water-code" => "AQ",
        "tpl-sys-sanitary" => "Esgoto sanitário",
        "tpl-sys-sanitary-code" => "ES",
        "tpl-sys-vent" => "Ventilação",
        "tpl-sys-vent-code" => "V",
        "tpl-sys-rainwater" => "Águas pluviais",
        "tpl-sys-rainwater-code" => "AP",
        "tpl-part-head" => "Crivo",
        "tpl-part-grate" => "Grelha",
        "tpl-comp-shower" => "Chuveiro",
        "tpl-comp-kitchen-sink" => "Pia de cozinha",
        "tpl-comp-floor-drain" => "Ralo sifonado",
        "tpl-comp-outlet" => "Tomada de uso geral",
        "tpl-comp-dedicated-outlet" => "Tomada de uso específico",
        "tpl-comp-switch" => "Interruptor",
        "tpl-comp-wall-light" => "Arandela",
        "tpl-comp-panel" => "Quadro de distribuição",
        "tpl-comp-ceiling-light" => "Ponto de luz no teto",
        "tpl-type-shower" => "Chuveiro de parede",
        "tpl-type-kitchen-sink" => "Pia inox 120 cm",
        "tpl-type-floor-drain" => "Ralo sifonado 150 mm",
        "tpl-type-outlet" => "Tomada 2P+T 10 A",
        "tpl-type-dedicated-outlet" => "Tomada 2P+T 20 A",
        "tpl-type-switch" => "Interruptor simples",
        "tpl-type-wall-light" => "Arandela 60 VA",
        "tpl-type-panel" => "Quadro de distribuição de embutir",
        "tpl-type-ceiling-light" => "Luminária de teto 100 VA",
        "tpl-phase-existing" => "Existente",
        "tpl-phase-new" => "Construção nova",
        "tpl-phase-filter-all" => "Mostrar tudo",
        "tpl-phase-filter-complete" => "Mostrar completo",
        "tpl-phase-filter-new" => "Mostrar novo",
        "tpl-phase-filter-previous-demo" => "Mostrar anterior + demolição",
        "tpl-phase-filter-previous-new" => "Mostrar anterior + novo",
        "tpl-phase-filter-previous" => "Mostrar fase anterior",
        "tpl-phase-filter-demo-new" => "Mostrar demolição + novo",
        // The imperial template's names.
        "tpl-imp-type-masonry-6" => "Alvenaria 6\"",
        "tpl-imp-type-masonry-10" => "Alvenaria 10\"",
        "tpl-imp-type-drywall-4" => "Divisória gesso 4 7/8\"",
        "tpl-imp-type-door-32" => "Porta 2'-8\" x 6'-8\"",
        "tpl-imp-type-door-36" => "Porta 3'-0\" x 6'-8\"",
        "tpl-imp-type-window-48" => "Janela 4'-0\" x 4'-0\"",
        "tpl-imp-type-window-60" => "Janela 5'-0\" x 3'-6\"",
        "tpl-imp-type-window-casement-24" => "Janela de abrir 2'-0\" x 3'-6\"",
        "tpl-imp-type-window-double-48" => "Janela de abrir 2 folhas 4'-0\" x 4'-0\"",
        "tpl-imp-type-window-top-hung-24" => "Janela maxim-ar 2'-0\" x 2'-0\"",
        "tpl-imp-type-window-sliding-60" => "Janela de correr 5'-0\" x 4'-0\"",
        "tpl-imp-type-slab-5" => "Laje 5\"",
        "tpl-imp-type-column-rect" => "Pilar 8\" x 8\"",
        "tpl-imp-type-column-round" => "Pilar circular 12\"",
        "tpl-imp-type-beam" => "Viga 6\" x 16\"",
        "tpl-imp-type-column-steel-h" => "Pilar metálico H 8\" x 8\"",
        "tpl-imp-type-column-steel-tube" => "Pilar metálico tubo 6\" x 6\" x 1/4\"",
        "tpl-imp-type-beam-steel-i" => "Viga metálica I 12\" x 6 1/2\"",
        "tpl-imp-type-rebar-3" => "Barra #3",
        "tpl-imp-type-rebar-4" => "Barra #4",
        "tpl-imp-type-rebar-5" => "Barra #5",
        "tpl-imp-type-rebar-6" => "Barra #6",
        "tpl-imp-type-rebar-8" => "Barra #8",
        "tpl-imp-type-footing-pad" => "Sapata 3'-0\" x 3'-0\" x 12\"",
        "tpl-imp-type-footing-strip" => "Viga baldrame 8\" x 16\"",
        "tpl-imp-type-footing-slab" => "Radier 12\"",
        "tpl-imp-type-stair" => "Escada 3'-0\"",
        "tpl-imp-type-railing" => "Guarda-corpo 3'-6\"",
        "tpl-imp-type-pipe-1-2" => "Tubo PVC soldável 1/2\"",
        "tpl-imp-type-pipe-3-4" => "Tubo PVC soldável 3/4\"",
        "tpl-imp-type-pipe-1" => "Tubo PVC soldável 1\"",
        "tpl-imp-type-pipe-sewer-1-1-2" => "Tubo PVC esgoto 1 1/2\"",
        "tpl-imp-type-pipe-sewer-2" => "Tubo PVC esgoto 2\"",
        "tpl-imp-type-pipe-sewer-3" => "Tubo PVC esgoto 3\"",
        "tpl-imp-type-pipe-sewer-4" => "Tubo PVC esgoto 4\"",
        "tpl-imp-type-conduit-1-2" => "Eletroduto flexível 1/2\"",
        "tpl-imp-type-conduit-3-4" => "Eletroduto flexível 3/4\"",
        "tpl-imp-type-conduit-1" => "Eletroduto flexível 1\"",
        "tpl-imp-type-grid" => "Eixo 3/8\"",
        "tpl-imp-type-dimension" => "Cota linear 3/32\"",
        "tpl-imp-type-text" => "Texto 3/32\"",
        "tpl-imp-type-text-large" => "Texto 1/8\"",
        "tpl-imp-type-french-door-32" => "Porta-janela 2'-8\" x 6'-8\"",
        "tpl-imp-type-french-door-double-60" => "Porta-janela 2 folhas 5'-0\" x 6'-8\"",
        "tpl-imp-type-table" => "Mesa 4'-0\" x 2'-6\"",
        "tpl-imp-type-bed" => "Cama casal 5'-0\" x 6'-8\"",
        "tpl-imp-type-wardrobe" => "Armário 5'-0\"",
        "tpl-imp-type-kitchen-sink" => "Pia inox 4'-0\"",
        "tpl-imp-type-floor-drain" => "Ralo sifonado 6\"",
        _ => "?",
    }
}

/// Whether elements of `category` have a body whose geometry can be joined.
pub fn can_join_geometry(category: crate::Category) -> bool {
    use crate::Category::*;
    matches!(category, Wall | Floor | Ceiling | Roof | Column | Beam | Stair | Railing | Component)
}

/// How strongly an element cuts the elements it is joined with: of two
/// joined elements, the higher cuts the lower. Structural columns and beams
/// cut hosts (walls, roofs, ceilings, floors); floors, ceilings and roofs cut
/// walls; walls cut architectural columns.
pub fn join_priority(element: &crate::Element) -> u8 {
    use crate::ElementKind::*;
    match &element.kind {
        Column(c) if c.structural => 7,
        Beam(_) => 6,
        Floor(_) => 5,
        Roof(_) => 4,
        Ceiling(_) => 3,
        Wall(_) => 2,
        Column(_) => 1,
        _ => 0,
    }
}

/// Compare texts with their digit runs as numbers (A2 before A10).
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (take(&mut x), take(&mut y));
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = m.len().cmp(&n.len()).then_with(|| m.cmp(n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.cmp(&d);
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

#[cfg(test)]
mod sheet_order_tests {
    use super::*;
    use crate::SheetSize;

    #[test]
    fn sheets_sort_by_number_naturally() {
        assert_eq!(natural_cmp("A2", "A10"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("A10", "A10"), std::cmp::Ordering::Equal);
        assert_eq!(natural_cmp("B1", "A99"), std::cmp::Ordering::Greater);
        let mut p = Project::from_template();
        p.views.retain(|_, v| !matches!(v.kind, ViewKind::Sheet { .. }));
        for (id, n) in [(9001, "A10"), (9002, "A2"), (9003, "A1")] {
            let kind = ViewKind::Sheet {
                number: n.into(),
                size: SheetSize::A3,
                portrait: false,
                viewports: Vec::new(),
                title_block: None,
                fields: Default::default(),
                issue_date: String::new(),
            };
            p.views.insert(ViewId(id), View::new(ViewId(id), n, 100, kind));
        }
        assert_eq!(p.sheets_in_order(), vec![ViewId(9003), ViewId(9002), ViewId(9001)]);
    }
}
