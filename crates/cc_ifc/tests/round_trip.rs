//! Export → import round trips: the imported project must reproduce the
//! levels and the geometry of every importable element.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_core::{Category, ColumnSection, Element, ElementKind, FoundationShape, Project, RoofShape, TypeData, Uid};
use cc_ifc::{ExportOptions, export, import};
use glam::DVec2;
use std::collections::HashMap;

fn load(text: &str) -> Project {
    cc_core::from_ron(text).unwrap()
}

pub(crate) fn round_trip(original: &Project) -> (Project, cc_ifc::ImportReport) {
    let text =
        export(original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    import(&text, Project::from_template()).unwrap()
}

fn by_uid(p: &Project) -> HashMap<Uid, &Element> {
    p.elements.values().map(|e| (e.uid, e)).collect()
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b}");
}

fn close2(a: DVec2, b: DVec2, tol: f64, what: &str) {
    assert!(a.distance(b) <= tol, "{what}: {a} vs {b}");
}

pub(crate) fn check(original: &Project) {
    let (imported, report) = round_trip(original);
    assert_eq!(report.schema, "IFC4");
    assert_eq!(imported.info.name, original.info.name);
    assert_eq!(imported.info.number, original.info.number);
    assert_eq!(imported.info.uid, original.info.uid);
    assert_eq!(imported.levels.len(), original.levels.len());
    for (a, b) in original.levels.iter().zip(&imported.levels) {
        assert_eq!(a.name, b.name);
        close(a.elevation, b.elevation, 1e-9, "level elevation");
    }
    let lookup = by_uid(&imported);
    let level_name = |p: &Project, e: &Element| p.level_name(e.level).to_string();
    const TOL: f64 = 1e-6;
    for e in original.elements.values() {
        let importable = matches!(
            e.category(),
            Category::Wall
                | Category::Door
                | Category::Window
                | Category::Floor
                | Category::Ceiling
                | Category::Roof
                | Category::Room
                | Category::Column
                | Category::Beam
                | Category::Component
        ) || matches!(&e.kind, ElementKind::Foundation(f) if matches!(f.shape, FoundationShape::Slab { .. }));
        if !importable {
            continue;
        }
        let i = lookup.get(&e.uid).unwrap_or_else(|| panic!("{:?} {} not imported", e.category(), e.id));
        assert_eq!(i.category(), e.category());
        assert_eq!(level_name(&imported, i), level_name(original, e), "level of {}", e.id);
        assert_eq!(imported.type_name(i.type_id), original.type_name(e.type_id), "type of {}", e.id);

        if e.category() != Category::Room {
            assert_eq!(i.mark, e.mark);
        }
        match (&e.kind, &i.kind) {
            (ElementKind::Wall(a), ElementKind::Wall(b)) => {
                let (s, t) = original.wall_axis(e, a);
                close2(b.start, s, TOL, "wall start");
                close2(b.end, t, TOL, "wall end");
                close(original.wall_height(e, a), imported.wall_height(i, b), TOL, "wall height");
                close(original.wall_base_z(e, a), imported.wall_base_z(i, b), TOL, "wall base");
                close(original.thickness_of(e), imported.thickness_of(i), TOL, "wall thickness");
                close(a.bulge, b.bulge, TOL, "wall bulge");
                assert_eq!(a.structural, b.structural);
            }
            (ElementKind::Door(a), ElementKind::Door(b)) | (ElementKind::Window(a), ElementKind::Window(b)) => {
                let host = |p: &Project, id| p.element(id).map(|h| h.uid);
                assert_eq!(host(original, a.host), host(&imported, b.host), "host of {}", e.id);
                close(a.offset, b.offset, TOL, "opening offset");
                close(a.sill_height, b.sill_height, TOL, "sill");
            }
            (ElementKind::Floor(a), ElementKind::Floor(b)) => {
                close(cc_geom::polygon::area(&a.boundary), cc_geom::polygon::area(&b.boundary), TOL, "floor area");
                assert_eq!(a.openings.len(), b.openings.len(), "floor openings");
                close(a.area(), b.area(), TOL, "floor net area");
                close(a.height_offset, b.height_offset, TOL, "floor offset");
            }
            (ElementKind::Foundation(a), ElementKind::Foundation(b)) => {
                let area = |s: &FoundationShape| match s {
                    FoundationShape::Slab { boundary } => cc_geom::polygon::area(boundary),
                    other => panic!("foundation came back as {other:?}"),
                };
                close(area(&a.shape), area(&b.shape), TOL, "foundation slab area");
                close(a.top_offset, b.top_offset, TOL, "foundation slab top");
                close(original.thickness_of(e), imported.thickness_of(i), TOL, "foundation slab thickness");
            }
            (ElementKind::Ceiling(a), ElementKind::Ceiling(b)) => {
                close(cc_geom::polygon::area(&a.boundary), cc_geom::polygon::area(&b.boundary), TOL, "ceiling area");
                close(a.height_offset, b.height_offset, TOL, "ceiling offset");
            }
            (ElementKind::Roof(a), ElementKind::Roof(b)) => {
                let fp = cc_geom::roofs::plan_area;
                assert_eq!(a.holes.len(), b.holes.len(), "roof openings");
                match (a.shape, b.shape) {
                    (RoofShape::Flat, RoofShape::Flat) => {
                        close(fp(a), fp(b), TOL, "flat roof area");
                        close(a.base_offset, b.base_offset, TOL, "roof base");
                    }
                    (
                        RoofShape::Shed { slope: s0, direction_deg: d0 },
                        RoofShape::Shed { slope: s1, direction_deg: d1 },
                    ) => {
                        close(s0, s1, TOL, "shed slope");
                        close(d0, d1, 1e-6, "shed direction");
                        close(fp(a), fp(b), TOL, "shed footprint");
                    }
                    (RoofShape::Gable { slope: s0, .. }, RoofShape::Gable { slope: s1, .. })
                    | (RoofShape::Hip { slope: s0 }, RoofShape::Hip { slope: s1 }) => {
                        // Pitched roofs come back from their B-rep: slope and outline (with the overhang).
                        close(s0, s1, 1e-6, "pitched slope");
                        close(fp(a), fp(b), 1e-6 * fp(a), "pitched footprint");
                    }
                    (x, y) => panic!("roof shape {x:?} came back as {y:?}"),
                }
            }
            (ElementKind::Room(a), ElementKind::Room(b)) => {
                assert_eq!(a.number, b.number);
                assert_eq!(a.name, b.name);
                close(a.height, b.height, TOL, "room height");
            }
            (ElementKind::Column(a), ElementKind::Column(b)) => {
                close2(a.position, b.position, TOL, "column position");
                let (b0, t0) = original.column_span(e, a);
                let (b1, t1) = imported.column_span(i, b);
                close(b0, b1, TOL, "column base");
                close(t0, t1, TOL, "column top");
                let section = |p: &Project, x: &Element| match p.element_type(x.type_id).map(|t| &t.data) {
                    Some(TypeData::Column { section, .. }) => *section,
                    _ => panic!("column without column type"),
                };
                match (section(original, e), section(&imported, i)) {
                    (ColumnSection::Rectangular { .. }, ColumnSection::Rectangular { .. }) => {
                        let diff = (a.rotation_deg - b.rotation_deg).rem_euclid(180.0);
                        assert!(
                            diff < 1e-6 || diff > 180.0 - 1e-6,
                            "column rotation {} vs {}",
                            a.rotation_deg,
                            b.rotation_deg
                        );
                    }
                    (ColumnSection::Round { diameter: d0 }, ColumnSection::Round { diameter: d1 }) => {
                        close(d0, d1, TOL, "diameter")
                    }
                    (
                        ColumnSection::Pipe { diameter: d0, wall: w0 },
                        ColumnSection::Pipe { diameter: d1, wall: w1 },
                    ) => {
                        close(d0, d1, TOL, "diameter");
                        close(w0, w1, TOL, "wall");
                    }
                    (x, y) => panic!("section {x:?} came back as {y:?}"),
                }
            }
            (ElementKind::Component(a), ElementKind::Component(b)) => {
                close2(a.position, b.position, TOL, "component position");
                let diff = (a.rotation_deg - b.rotation_deg).rem_euclid(360.0);
                assert!(diff < 1e-6 || diff > 360.0 - 1e-6, "rotation {} vs {}", a.rotation_deg, b.rotation_deg);
                close(a.base_offset, b.base_offset, TOL, "component base");
                let in_base = Project::from_template().types.values().any(|t| t.name == original.type_name(e.type_id));
                if in_base {
                    // The parametric type is reused, with the instance overrides.
                    assert!(matches!(
                        imported.element_type(i.type_id).map(|t| &t.data),
                        Some(TypeData::Component { def, .. }) if !def.params.is_empty()
                    ));
                    assert_eq!(a.overrides.keys().collect::<Vec<_>>(), b.overrides.keys().collect::<Vec<_>>());
                    for (k, v) in &a.overrides {
                        close(*v, b.overrides[k], TOL, "override");
                    }
                }
                // Either way the pieces end up in the same place.
                let bounds = |p: &Project, x: &Element, c: &cc_core::Component| {
                    cc_geom::component_parts(p, x, c)
                        .iter()
                        .flat_map(|q| {
                            q.footprint
                                .iter()
                                .map(|v| v.extend(q.z0))
                                .chain(q.footprint.iter().map(|v| v.extend(q.z1)))
                                .collect::<Vec<_>>()
                        })
                        .fold(
                            (glam::DVec3::splat(f64::INFINITY), glam::DVec3::splat(f64::NEG_INFINITY)),
                            |(lo, hi), v| (lo.min(v), hi.max(v)),
                        )
                };
                let (lo0, hi0) = bounds(original, e, a);
                let (lo1, hi1) = bounds(&imported, i, b);
                assert!(
                    (lo0 - lo1).abs().max_element() < 1e-5 && (hi0 - hi1).abs().max_element() < 1e-5,
                    "{lo0} {hi0} vs {lo1} {hi1}"
                );
            }
            (ElementKind::Beam(a), ElementKind::Beam(b)) => {
                close2(a.start, b.start, TOL, "beam start");
                close2(a.end, b.end, TOL, "beam end");
                close(a.top_offset, b.top_offset, TOL, "beam top");
            }
            (x, y) => panic!("{x:?} came back as {y:?}"),
        }
    }
    // The imported project survives its own export and repair.
    let mut again = imported.clone();
    assert_eq!(again.repair(), 0, "imported project needed repairs");
    let text = export(&again, &ExportOptions::default());
    assert!(original.walls().next().is_none() || text.contains("IFCWALL("));
}

#[test]
fn sample_house_round_trips() {
    check(&load(include_str!("../../../examples/casa_simples.ccad")));
}

#[test]
fn feature_fixture_round_trips() {
    check(&load(include_str!("fixtures/features.ccad")));
}

#[test]
fn arc_walls_round_trip() {
    use cc_core::{Document, Hosted, Wall};
    // A half circle (with a window) closed by a straight wall, and a free
    // clockwise quarter circle drawn on its exterior face.
    let mut doc = Document::new(Project::from_template());
    doc.edit("arcs", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, category, kind| {
            let type_id = e.project().default_type(category).unwrap();
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: "A".into(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        };
        let arc = Wall { start: DVec2::new(4.0, 0.0), end: DVec2::new(-4.0, 0.0), bulge: 1.0, ..Default::default() };
        let arc = add(e, Category::Wall, ElementKind::Wall(arc));
        let chord = Wall { start: DVec2::new(-4.0, 0.0), end: DVec2::new(4.0, 0.0), ..Default::default() };
        add(e, Category::Wall, ElementKind::Wall(chord));
        let window = Hosted { host: arc, offset: 3.0, sill_height: 1.0, flip_facing: false, flip_hand: false };
        add(e, Category::Window, ElementKind::Window(window));
        let free = Wall {
            start: DVec2::new(10.0, 0.0),
            end: DVec2::new(13.0, -3.0),
            bulge: -(std::f64::consts::PI / 8.0).tan(),
            location_line: cc_core::LocationLine::FinishExterior,
            ..Default::default()
        };
        add(e, Category::Wall, ElementKind::Wall(free));
    });
    let original = doc.project;
    check(&original);
    // The export carries arc axes and arc profiles.
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    assert_eq!(text.matches("IFCTRIMMEDCURVE").count(), 2);
    assert!(text.contains("IFCARCINDEX"));
    // Imported walls have the same volumes (the window cut out of its arc).
    let (imported, _) = round_trip(&original);
    let (a, b) = (cc_geom::ModelGeometry::build(&original), cc_geom::ModelGeometry::build(&imported));
    let volumes = |g: &cc_geom::ModelGeometry, p: &Project| {
        let mut v: Vec<(Uid, f64)> = p.walls().map(|(e, _)| (e.uid, g.volume(e.id))).collect();
        v.sort_by_key(|(uid, _)| *uid);
        v
    };
    for ((ua, va), (ub, vb)) in volumes(&a, &original).iter().zip(&volumes(&b, &imported)) {
        assert_eq!(ua, ub);
        close(*va, *vb, 1e-6, "arc wall volume");
    }
}

#[test]
fn roof_fixture_round_trips() {
    check(&load(include_str!("fixtures/roofs.ccad")));
}

#[test]
fn curtain_fixture_round_trips() {
    check(&load(include_str!("fixtures/curtains.ccad")));
}

#[test]
fn profile_fixture_round_trips() {
    check(&load(include_str!("fixtures/profiles.ccad")));
}

#[test]
fn arc_fixture_round_trips() {
    check(&load(include_str!("fixtures/arcs.ccad")));
}

#[test]
fn reimport_reuses_template_types() {
    let original = load(include_str!("../../../examples/casa_simples.ccad"));
    let (imported, _) = round_trip(&original);
    // Types with the same name and size are not duplicated.
    for category in [Category::Wall, Category::Door, Category::Window] {
        let mut names: Vec<&str> = imported.types_of(category).map(|t| t.name.as_str()).collect();
        let n = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), n, "duplicate {category:?} type names");
    }
}

#[test]
fn framed_beams_round_trip() {
    use cc_core::{Beam, Column, Document, WallTop};
    // A column with a beam framed into it, mitered where it meets a second beam.
    let mut doc = Document::new(Project::from_template());
    doc.edit("framing", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, category, kind| {
            let type_id = e.project().default_type(category).unwrap();
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
        };
        let column = Column {
            position: DVec2::ZERO,
            rotation_deg: 0.0,
            base_offset: 0.0,
            height: 3.0,
            top: WallTop::Unconnected,
            structural: true,
            base_support: Default::default(),
        };
        add(e, Category::Column, ElementKind::Column(column));
        add(e, Category::Beam, ElementKind::Beam(Beam::new(DVec2::ZERO, DVec2::new(4.0, 0.0), 3.0)));
        add(e, Category::Beam, ElementKind::Beam(Beam::new(DVec2::new(4.0, 0.0), DVec2::new(4.0, 3.0), 3.0)));
        // Away from the frame (the check expects a wall).
        let wall = cc_core::Wall { start: DVec2::new(10.0, 0.0), end: DVec2::new(12.0, 0.0), ..Default::default() };
        add(e, Category::Wall, ElementKind::Wall(wall));
    });
    let original = doc.project;
    let before = cc_geom::ModelGeometry::build(&original);
    assert!(before.beams.values().any(|g| !g.is_square()), "a mitered beam");
    check(&original);
    // The imported beams frame the same way.
    let (imported, _) = round_trip(&original);
    let after = cc_geom::ModelGeometry::build(&imported);
    let footprints = |g: &cc_geom::ModelGeometry, p: &Project| {
        let mut v: Vec<(Uid, [DVec2; 4])> =
            g.beams.values().filter_map(|b| Some((p.element(b.element)?.uid, b.footprint()))).collect();
        v.sort_by_key(|(uid, _)| *uid);
        v
    };
    let (a, b) = (footprints(&before, &original), footprints(&after, &imported));
    assert_eq!(a.len(), 2);
    for ((ua, fa), (ub, fb)) in a.iter().zip(&b) {
        assert_eq!(ua, ub);
        for (p, q) in fa.iter().zip(fb) {
            close2(*p, *q, 1e-6, "framed beam corner");
        }
    }
}

/// An arc beam between two columns is written as its outline of true arcs,
/// extruded, with its axis on the arc, and comes back as an arc beam.
#[test]
fn arc_beams_round_trip() {
    use cc_core::{Beam, Column, Document, WallTop};
    let mut doc = Document::new(Project::from_template());
    let mut beam_id = cc_core::ElementId(0);
    doc.edit("ring", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, category, kind| {
            let type_id = e.project().default_type(category).unwrap();
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            id
        };
        for position in [DVec2::new(2.5, 0.0), DVec2::new(0.0, 2.5)] {
            let column = Column {
                position,
                rotation_deg: 0.0,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            };
            add(e, Category::Column, ElementKind::Column(column));
        }
        let mut beam = Beam::new(DVec2::new(2.5, 0.0), DVec2::new(0.0, 2.5), 3.0);
        beam.bulge = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
        beam_id = add(e, Category::Beam, ElementKind::Beam(beam));
    });
    let original = doc.project;
    let text = export(&original, &ExportOptions::default());
    let curve = text.lines().find(|l| l.contains("=IFCINDEXEDPOLYCURVE(")).expect("an outline of arcs");
    assert_eq!(curve.matches("IFCARCINDEX").count(), 2, "{curve}");
    let (imported, report) = round_trip(&original);
    assert_eq!(report.imported.get(&Category::Beam), Some(&1), "{report:?}");
    let uid = original.element(beam_id).unwrap().uid;
    let back = imported.elements.values().find(|e| e.uid == uid).expect("the beam");
    let (ElementKind::Beam(a), ElementKind::Beam(b)) = (&original.element(beam_id).unwrap().kind, &back.kind) else {
        panic!("came back as {:?}", back.kind);
    };
    close2(a.start, b.start, 1e-6, "arc beam start");
    close2(a.end, b.end, 1e-6, "arc beam end");
    close(a.bulge, b.bulge, 1e-6, "arc beam bulge");
    close(a.top_offset, b.top_offset, 1e-6, "arc beam elevation");
    // The same body: framed into the same columns, the same volume.
    let (before, after) = (cc_geom::ModelGeometry::build(&original), cc_geom::ModelGeometry::build(&imported));
    close(before.volume(beam_id), after.volume(back.id), 1e-6, "arc beam volume");
    assert!(matches!(after.beams[&back.id].start, cc_geom::BeamEnd::Column(_)));
}

/// A floor with a shaft and a stairwell, and a foundation slab, come back
/// as native elements; the floor's openings are voids of its profile.
#[test]
fn floor_openings_and_foundation_slabs_round_trip() {
    let mut project = Project::from_template();
    let level = project.levels[0].id;
    let floor_t = project.default_type(Category::Floor).unwrap();
    let slab_t = project
        .types_of(Category::Foundation)
        .find(|t| matches!(t.data, TypeData::Foundation { kind: cc_core::FoundationKind::Slab, .. }))
        .map(|t| t.id)
        .unwrap();
    let square = |x: f64, y: f64, s: f64| {
        vec![DVec2::new(x, y), DVec2::new(x + s, y), DVec2::new(x + s, y + s), DVec2::new(x, y + s)]
    };
    let mut floor = cc_core::Floor::new(square(0.0, 0.0, 10.0), 3.0);
    floor.openings = vec![square(1.0, 1.0, 2.0), square(5.0, 5.0, 3.0)];
    let raft = cc_core::Foundation::new(FoundationShape::Slab { boundary: square(0.0, 0.0, 12.0) }, -0.1);
    for (type_id, kind) in [(floor_t, ElementKind::Floor(floor)), (slab_t, ElementKind::Foundation(raft))] {
        let id = cc_core::ElementId(project.alloc_id());
        let e = Element {
            id,
            level,
            type_id,
            mark: String::new(),
            comments: String::new(),
            kind,
            uid: Uid::generate(),
            phase_created: Default::default(),
            phase_demolished: None,
            design_option: None,
            paints: Vec::new(),
            plugin_data: Default::default(),
        };
        project.elements.insert(id, e);
    }
    let text = export(&project, &ExportOptions::default());
    assert!(text.contains("IFCARBITRARYPROFILEDEFWITHVOIDS"));
    assert!(text.contains(".BASESLAB."));
    check(&project);
}

#[test]
fn structure_fixture_exports_foundations_profiles_and_cut_slabs() {
    let project = load(include_str!("fixtures/structure.ccad"));
    let text = export(&project, &ExportOptions::default());
    let lines = |entity: &str| text.lines().filter(|l| l.contains(&format!("={entity}("))).collect::<Vec<_>>();
    // Pads under the four columns and strips under the four walls, a straight
    // strip meeting an arc strip (#22); a raft as a base slab.
    let footings = lines("IFCFOOTING");
    assert_eq!(footings.len(), 10);
    assert_eq!(footings.iter().filter(|l| l.contains(".PAD_FOOTING.")).count(), 4);
    // The arc strip's body is its outline of true arcs, extruded.
    assert_eq!(lines("IFCINDEXEDPOLYCURVE").len(), 1);
    assert_eq!(lines("IFCARCINDEX").len(), 0, "arc indices are typed values inside the curve");
    let curve = lines("IFCINDEXEDPOLYCURVE")[0];
    assert_eq!(curve.matches("IFCARCINDEX").count(), 2, "{curve}");
    assert!(lines("IFCSLAB").iter().any(|l| l.contains(".BASESLAB.")));
    // Steel profiles as parametric profiles.
    for profile in [
        "IFCISHAPEPROFILEDEF",
        "IFCUSHAPEPROFILEDEF",
        "IFCLSHAPEPROFILEDEF",
        "IFCRECTANGLEHOLLOWPROFILEDEF",
        "IFCCIRCLEHOLLOWPROFILEDEF",
    ] {
        assert!(!lines(profile).is_empty(), "{profile}");
    }
    // The sloped beams and the slab cut by its supports are B-reps, in `Brep` representations only.
    let breps: Vec<String> =
        lines("IFCFACETEDBREP").iter().filter_map(|l| l.split('=').next()).map(str::to_string).collect();
    assert!(!breps.is_empty());
    let uses = |l: &str| breps.iter().any(|b| l.contains(&format!("{b},")) || l.contains(&format!("{b})")));
    let representations = lines("IFCSHAPEREPRESENTATION");
    assert!(representations.iter().filter(|l| uses(l)).all(|l| l.contains("'Brep'")));
}

#[test]
fn mep_fixture_exports_cable_trays_and_valves() {
    let project = load(include_str!("fixtures/mep.ccad"));
    let text = export(&project, &ExportOptions::default());
    let lines = |entity: &str| text.lines().filter(|l| l.contains(&format!("={entity}("))).collect::<Vec<_>>();
    // Two tray runs, typed as tray segments, meeting at a bend.
    let trays: Vec<&str> =
        lines("IFCCABLECARRIERSEGMENT").into_iter().filter(|l| l.contains(".CABLETRAYSEGMENT.")).collect();
    assert_eq!(trays.len(), 2);
    assert!(lines("IFCCABLECARRIERSEGMENTTYPE").iter().any(|l| l.contains(".CABLETRAYSEGMENT.")));
    assert!(lines("IFCCABLECARRIERFITTING").iter().any(|l| l.contains("'Bend'")));
    // The gate valve isolates, typed, as a tessellated body.
    let valves = lines("IFCVALVE");
    assert_eq!(valves.len(), 1);
    assert!(valves[0].contains(".ISOLATING.") && valves[0].contains("'V1'"), "{}", valves[0]);
    assert!(lines("IFCVALVETYPE").iter().any(|l| l.contains(".ISOLATING.")));
    // It is in its pipe's system.
    let valve_id = valves[0].split('=').next().unwrap();
    assert!(
        lines("IFCRELASSIGNSTOGROUP")
            .iter()
            .any(|l| l.contains(&format!("{valve_id},")) || l.contains(&format!("{valve_id})")))
    );
    // Imported back, they come in as components keeping their class.
    let (imported, report) = round_trip(&project);
    assert!(report.generic.contains_key("IfcValve"), "{:?}", report.generic);
    assert!(imported.elements.len() > project.elements.len() / 2);
}

/// Lightning protection and structured cabling (#34): conductors are cable
/// segments, air terminals protective devices, data outlets data outlets
/// and racks communications appliances, grouped in their systems.
#[test]
fn lightning_protection_and_cabling_export_with_their_classes() {
    use cc_core::{DeviceKind, ElementId, ElementTypeId, MepSystem, SystemId, SystemKind};
    let mut doc = cc_core::Document::new(Project::from_template());
    let level = doc.project.levels[0].id;
    let name = |key: &str| key.trim_start_matches("tpl-param-").replace('-', "_");
    doc.edit("t", |e| {
        let add_type = |e: &mut cc_core::EditCtx, data: TypeData| {
            let id = ElementTypeId(e.alloc_id());
            e.add_type(cc_core::ElementType {
                id,
                name: "t".into(),
                data,
                info: Default::default(),
                uid: Uid::generate(),
            })
        };
        let device = |e: &mut cc_core::EditCtx, d: DeviceKind| {
            let (component, _, def) = cc_core::component::device_library(d, &name).unwrap();
            let id = ElementTypeId(e.alloc_id());
            e.add_type(cc_core::ElementType {
                id,
                name: format!("{d:?}"),
                data: TypeData::Component { component, def },
                info: Default::default(),
                uid: Uid::generate(),
            })
        };
        let conductor_t = add_type(e, TypeData::LightningConductor { diameter: 0.008, material: String::new() });
        let (terminal_t, outlet_t, rack_t) =
            (device(e, DeviceKind::AirTerminal), device(e, DeviceKind::DataOutlet), device(e, DeviceKind::Rack));
        let lps = SystemId(e.alloc_id());
        e.systems_mut().insert(lps, MepSystem::new("LPS", "", SystemKind::Lightning));
        let add = |e: &mut cc_core::EditCtx, type_id, kind| {
            let id = ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                uid: Uid::generate(),
                kind,
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
            id
        };
        let mut run = cc_core::MepSegment::new(DVec2::ZERO, DVec2::new(6.0, 0.0), 3.0);
        run.systems = vec![lps];
        let role = cc_core::ConductorRole::Air;
        add(e, conductor_t, ElementKind::LightningConductor(cc_core::LightningConductor { run, role }));
        let rod = cc_core::Component {
            position: DVec2::new(3.0, 0.0),
            base_offset: 3.0,
            system: Some(lps),
            ..Default::default()
        };
        add(e, terminal_t, ElementKind::Component(rod));
        let rack =
            add(e, rack_t, ElementKind::Component(cc_core::Component { base_offset: 1.4, ..Default::default() }));
        let data = SystemId(e.alloc_id());
        let mut system = MepSystem::new("Cabling R1", "R1", SystemKind::Data);
        system.panel = Some(rack);
        e.systems_mut().insert(data, system);
        let outlet = cc_core::Component {
            position: DVec2::new(4.0, 2.0),
            base_offset: 0.3,
            system: Some(data),
            ..Default::default()
        };
        add(e, outlet_t, ElementKind::Component(outlet));
    });
    let text = export(&doc.project, &ExportOptions::default());
    let lines = |entity: &str| text.lines().filter(|l| l.contains(&format!("={entity}("))).collect::<Vec<_>>();
    let conductors = lines("IFCCABLESEGMENT");
    assert_eq!(conductors.len(), 1);
    assert!(conductors[0].contains(".CONDUCTORSEGMENT."), "{}", conductors[0]);
    assert!(lines("IFCCABLESEGMENTTYPE").iter().any(|l| l.contains(".CONDUCTORSEGMENT.")));
    let terminals = lines("IFCPROTECTIVEDEVICE");
    assert_eq!(terminals.len(), 1);
    assert!(terminals[0].contains("'AIRTERMINAL'") && terminals[0].contains(".USERDEFINED."), "{}", terminals[0]);
    assert!(lines("IFCPROTECTIVEDEVICETYPE").iter().any(|l| l.contains("'AIRTERMINAL'")));
    assert!(lines("IFCOUTLET").iter().any(|l| l.contains(".DATAOUTLET.")));
    assert!(lines("IFCCOMMUNICATIONSAPPLIANCE").iter().any(|l| l.contains("'RACK'")));
    let systems = lines("IFCDISTRIBUTIONSYSTEM");
    assert!(systems.iter().any(|l| l.contains(".LIGHTNINGPROTECTION.")), "{systems:?}");
    assert!(systems.iter().any(|l| l.contains(".DATA.") && l.contains("'R1'")), "{systems:?}");
}

/// Phases of an element by name: (created, demolished).
fn phase_names(p: &Project, e: &Element) -> (String, Option<String>) {
    let created = p.phases.get(p.created_index(e)).map(|ph| ph.name.clone()).unwrap_or_default();
    (created, e.phase_demolished.map(|d| p.phase_name(d)))
}

#[test]
fn phases_round_trip() {
    let original = load(include_str!("fixtures/phases.ccad"));
    check(&original);
    let text = export(&original, &ExportOptions::default());
    // Statuses relative to the last phase, in the common property sets.
    for status in ["'EXISTING'", "'DEMOLISH'", "'NEW'", "'TEMPORARY'"] {
        assert!(text.contains(&format!("IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL({status}))")), "{status}");
    }
    let (imported, _) = import(&text, Project::from_template()).unwrap();
    let names = |p: &Project| p.phases.iter().map(|ph| ph.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&imported), names(&original));
    let lookup = by_uid(&imported);
    let mut checked = 0;
    for e in original.elements.values().filter(|e| e.category().is_phased()) {
        let Some(i) = lookup.get(&e.uid) else { continue };
        assert_eq!(phase_names(&imported, i), phase_names(&original, e), "phases of {:?} {}", e.category(), e.id);
        checked += 1;
    }
    assert!(checked >= 12, "{checked}");
    assert_eq!(imported.clone().repair(), 0);
}

#[test]
fn ifc_status_maps_to_phases() {
    // Without this program's phase sets, `Status` alone decides: existing
    // elements are built in the phase before the last, demolished ones are
    // demolished in the last, new and temporary ones are built in it.
    let original = load(include_str!("fixtures/phases.ccad"));
    let text = export(&original, &ExportOptions::default())
        .replace("'ConstructCAD_Phasing'", "'Other_Phasing'")
        .replace("'ConstructCAD_Phases'", "'Other_Phases'");
    let (imported, _) = import(&text, Project::from_template()).unwrap();
    assert_eq!(imported.phases.len(), 2);
    let (first, last) = (imported.phases[0].id, imported.phases[1].id);
    let lookup = by_uid(&imported);
    let mut seen = std::collections::BTreeSet::new();
    for e in original.elements.values().filter(|e| matches!(e.category(), Category::Wall | Category::Door)) {
        let i = lookup[&e.uid];
        let expected = match cc_ifc::ifc_status(&original, e).unwrap() {
            cc_core::PhaseStatus::Existing => (first, None),
            cc_core::PhaseStatus::Demolished => (first, Some(last)),
            cc_core::PhaseStatus::New => (last, None),
            cc_core::PhaseStatus::Temporary => (last, Some(last)),
        };
        seen.insert(cc_ifc::ifc_status(&original, e).unwrap());
        assert_eq!((i.phase_created, i.phase_demolished), expected, "{:?} {}", e.category(), e.id);
    }
    assert_eq!(seen.len(), 4);
    // The status is read as phases, not kept as an imported property.
    assert!(imported.elements.values().all(|e| match &e.kind {
        ElementKind::Component(c) => c.properties.iter().all(|s| s.properties.iter().all(|p| p.name != "Status")),
        _ => true,
    }));
}

/// Roof framing: each member is an `IfcMember` aggregated in its roof, and
/// the covering and framing come back on the roof's type, the members drawn
/// again rather than imported as objects of their own.
#[test]
fn roof_framing_round_trips() {
    use cc_core::roofing::{FramingSystem, TileKind};
    let mut original = load(include_str!("fixtures/roofs.ccad"));
    original.materials.push(cc_core::Material::new("Pinus", [190, 150, 100]));
    for t in original.types.values_mut() {
        if let TypeData::Roof { covering, framing, .. } = &mut t.data {
            *covering = Some(TileKind::CeramicRoman);
            framing.system = FramingSystem::TimberTrusses;
            framing.material = "Pinus".into();
            framing.truss_spacing = 2.4;
            framing.rafter = cc_core::roofing::MemberSize::new(0.05, 0.07);
        }
    }
    let geom = cc_geom::ModelGeometry::build(&original);
    let members: usize = geom.roof_framing.values().map(|g| g.members.len()).sum();
    assert!(members > 0, "the fixture's roofs are framed");
    let text = export(&original, &ExportOptions { time_stamp: None, file_name: "t.ifc".into() });
    assert_eq!(text.matches("IFCMEMBER(").count(), members);
    assert!(
        text.contains(".PURLIN.") && text.contains(".RAFTER.") && text.contains(".CHORD.") && text.contains(".POST.")
    );
    assert!(text.contains("'ConstructCAD_Roofing'"));

    let (imported, report) = import(&text, Project::from_template()).unwrap();
    assert!(!report.skipped.contains_key("IFCMEMBER"), "{:?}", report.skipped);
    let count = |p: &Project, c: Category| p.elements.values().filter(|e| e.category() == c).count();
    assert_eq!(count(&imported, Category::Roof), count(&original, Category::Roof));
    assert_eq!(count(&imported, Category::Component), count(&original, Category::Component), "no members as objects");
    for e in imported.elements.values().filter(|e| e.category() == Category::Roof) {
        let Some(TypeData::Roof { covering, framing, .. }) = imported.element_type(e.type_id).map(|t| &t.data) else {
            panic!("a roof type");
        };
        assert_eq!(*covering, Some(TileKind::CeramicRoman));
        assert_eq!((framing.system, framing.material.as_str()), (FramingSystem::TimberTrusses, "Pinus"));
        close(framing.truss_spacing, 2.4, 1e-9, "truss spacing");
        close(framing.rafter.height, 0.07, 1e-9, "rafter height");
    }
    assert!(imported.materials.iter().any(|m| m.name == "Pinus"));
    let geom = cc_geom::ModelGeometry::build(&imported);
    assert_eq!(geom.roof_framing.values().map(|g| g.members.len()).sum::<usize>(), members);
}

/// Terrain: written as an `IfcGeographicElement` (`TERRAIN`) with its earth
/// as a B-rep, and rebuilt as terrain from the points and boundary it keeps
/// in `ConstructCAD_Topography`. Terrain from other programs stays generic.
#[test]
fn terrain_round_trips() {
    let original = load(include_str!("fixtures/topography.ccad"));
    let text = export(&original, &ExportOptions { time_stamp: None, file_name: "t.ifc".into() });
    let terrains: Vec<&Element> = original.elements.values().filter(|e| e.category() == Category::Topography).collect();
    assert_eq!(terrains.len(), 2);
    assert_eq!(text.matches("IFCGEOGRAPHICELEMENT(").count(), 2);
    assert!(text.contains(".TERRAIN.") && text.contains("'ConstructCAD_Topography'"));

    let (imported, report) = import(&text, Project::from_template()).unwrap();
    assert_eq!(report.imported.get(&Category::Topography), Some(&2), "{report:?}");
    let lookup = by_uid(&imported);
    for e in terrains {
        let (ElementKind::Topography(a), Some(back)) = (&e.kind, lookup.get(&e.uid)) else { panic!("terrain back") };
        let ElementKind::Topography(b) = &back.kind else { panic!("a terrain, not {:?}", back.category()) };
        assert_eq!(a.points, b.points);
        assert_eq!(a.boundary, b.boundary);
        close(a.base_depth, b.base_depth, 1e-12, "base depth");
        let (Some(ta), Some(tb)) = (original.element_type(e.type_id), imported.element_type(back.type_id)) else {
            panic!("types")
        };
        assert_eq!(ta.name, tb.name);
        assert_eq!(ta.data, tb.data);
    }
    // Its building pads, in ConstructCAD_Pads: outline, elevation, own slopes and type.
    assert!(text.contains("'ConstructCAD_Pads'"));
    let pads = |p: &Project| -> Vec<(cc_core::pad::Pad, String, TypeData)> {
        p.elements
            .values()
            .filter_map(|e| match &e.kind {
                ElementKind::Pad(pad) => {
                    let t = p.element_type(e.type_id)?;
                    Some((pad.clone(), t.name.clone(), t.data.clone()))
                }
                _ => None,
            })
            .collect()
    };
    let (a, b) = (pads(&original), pads(&imported));
    assert_eq!(a.len(), 1, "the fixture has a pad");
    assert_eq!(report.imported.get(&Category::Pad), Some(&1), "{report:?}");
    for ((pa, na, da), (pb, nb, db)) in a.iter().zip(&b) {
        assert_eq!(
            (&pa.outline, pa.elevation, pa.cut_slope, pa.fill_slope),
            (&pb.outline, pb.elevation, pb.cut_slope, pb.fill_slope)
        );
        assert_eq!((na, da), (nb, db));
        let host = imported.element(pb.host).map(|h| h.category());
        assert_eq!(host, Some(Category::Topography));
    }
    // The same graded body and earthworks: equal volumes.
    let (g0, g1) = (cc_geom::ModelGeometry::build(&original), cc_geom::ModelGeometry::build(&imported));
    let total = |g: &cc_geom::ModelGeometry| g.topographies.values().map(|t| t.volume).sum::<f64>();
    close(total(&g0), total(&g1), 1e-6, "terrain volume");
    let earth = |g: &cc_geom::ModelGeometry| {
        g.pads.values().map(|p| (p.cut, p.fill)).fold((0.0, 0.0), |s, v| (s.0 + v.0, s.1 + v.1))
    };
    close(earth(&g0).0, earth(&g1).0, 1e-6, "cut");
    close(earth(&g0).1, earth(&g1).1, 1e-6, "fill");

    // Without the property set (another program's terrain): a generic component.
    let other = text.replace("'ConstructCAD_Topography'", "'Pset_Other'");
    let (imported, _) = import(&other, Project::from_template()).unwrap();
    assert_eq!(imported.elements.values().filter(|e| e.category() == Category::Topography).count(), 0);
    assert!(imported.elements.values().filter(|e| e.category() == Category::Component).count() >= 2);
}

/// A project with curtain walls: one standing in a plain wall (with its own
/// lines, a removed segment and a solid panel), an arc one and a free one.
fn curtain_project() -> Project {
    use cc_core::curtain::{CurtainGrid, CurtainPanel, CurtainSystem, GridAxis, GridLayout, GridSegment, PanelSwap};
    use cc_core::{Document, ElementType, ElementTypeId, Wall};
    let mut doc = Document::new(Project::from_template());
    doc.edit("curtains", |e| {
        let level = e.project().levels[0].id;
        let glass = e.project().materials.iter().find(|m| m.name == "Vidro").unwrap().name.clone();
        let mut system = CurtainSystem::with_materials(&glass, &glass);
        system.vertical = CurtainGrid { layout: GridLayout::FixedDistance, spacing: 1.0, ..Default::default() };
        system.horizontal = CurtainGrid { layout: GridLayout::FixedNumber, count: 2, ..Default::default() };
        let type_id = ElementTypeId(e.alloc_id());
        let data = TypeData::Wall {
            layers: Vec::new(),
            function: cc_core::WallFunction::Exterior,
            curtain: Some(system),
            structural: false,
        };
        e.add_type(ElementType {
            id: type_id,
            name: "Glazing".into(),
            data,
            info: Default::default(),
            uid: Uid::default(),
        });
        let plain = e.project().default_type(Category::Wall).unwrap();
        let add = |e: &mut cc_core::EditCtx, type_id, wall: Wall| {
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind: ElementKind::Wall(wall),
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            });
        };
        add(e, plain, Wall { start: DVec2::new(0.0, 0.0), end: DVec2::new(10.0, 0.0), ..Default::default() });
        let mut embedded =
            Wall { start: DVec2::new(2.0, 0.0), end: DVec2::new(6.0, 0.0), height: 2.5, ..Default::default() };
        embedded.curtain.vertical = Some(vec![1.5, 2.5]);
        embedded.curtain.removed.push(GridSegment { axis: GridAxis::Vertical, at: 1.5, along: 0.5 });
        let solid = CurtainPanel::Solid { thickness: 0.03, material: glass.clone() };
        embedded.curtain.panels.push(PanelSwap { u: 3.5, z: 2.0, panel: solid });
        add(e, type_id, embedded);
        add(
            e,
            type_id,
            Wall { start: DVec2::new(4.0, 6.0), end: DVec2::new(-4.0, 6.0), bulge: 0.5, ..Default::default() },
        );
        add(
            e,
            type_id,
            Wall { start: DVec2::new(0.0, 12.0), end: DVec2::new(3.0, 12.0), height: 3.0, ..Default::default() },
        );
    });
    doc.project
}

/// Figures of each curtain wall's grid by uid: columns, rows, panels, glazed area, mullion length.
fn curtain_figures(p: &Project) -> Vec<(Uid, usize, usize, usize, f64, f64)> {
    let g = cc_geom::ModelGeometry::build(p);
    let mut out: Vec<_> = p
        .walls()
        .filter_map(|(e, _)| {
            let c = g.walls.get(&e.id)?.curtain.as_ref()?;
            let l = &c.layout;
            Some((e.uid, l.columns(), l.rows(), l.panel_count(), l.glazed_area(), l.mullion_length()))
        })
        .collect();
    out.sort_by_key(|f| f.0);
    out
}

#[test]
fn curtain_walls_round_trip() {
    let original = curtain_project();
    check(&original);
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    assert_eq!(text.matches("IFCCURTAINWALL(").count(), 3);
    assert!(text.contains("IFCCURTAINWALLTYPE(") && text.contains("IFCPLATETYPE(") && text.contains("IFCMEMBERTYPE("));
    assert!(text.contains(".CURTAIN_PANEL.") && text.contains(".MULLION."));
    // The plain wall opens for the curtain wall in it, with nothing filling the opening.
    assert_eq!(text.matches("IFCOPENINGELEMENT(").count(), 1);
    assert!(!text.contains("IFCWINDOW(") && !text.contains("IFCRELFILLSELEMENT("));

    // Imported: the same grids, panels and mullions, the wall's own lines,
    // removed segment and swapped panel kept; no plate or member on its own.
    let (imported, report) = round_trip(&original);
    assert_eq!(report.generic.len(), 0, "{:?}", report.generic);
    let (a, b) = (curtain_figures(&original), curtain_figures(&imported));
    assert_eq!(a.len(), 3);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.0, x.1, x.2, x.3), (y.0, y.1, y.2, y.3));
        close(x.4, y.4, 1e-6, "glazed area");
        close(x.5, y.5, 1e-6, "mullion length");
    }
    let edits = |p: &Project| {
        let mut v: Vec<_> =
            p.walls().filter(|(_, w)| !w.curtain.is_empty()).map(|(e, w)| (e.uid, w.curtain.clone())).collect();
        v.sort_by_key(|x| x.0);
        v
    };
    assert_eq!(edits(&original), edits(&imported));
    // The plain wall still opens for it.
    let volume = |p: &Project| {
        let g = cc_geom::ModelGeometry::build(p);
        p.walls()
            .filter(|(e, _)| p.element_type(e.type_id).is_some_and(|t| t.curtain().is_none()))
            .map(|(e, _)| g.volume(e.id))
            .sum::<f64>()
    };
    close(volume(&original), volume(&imported), 1e-6, "plain wall volume");
}

/// A curtain wall from another program (no grid property set): rebuilt from
/// its plates and members when they make a plain grid, else they come in one
/// by one as before.
#[test]
fn foreign_curtain_walls_rebuild_from_their_parts() {
    let original = curtain_project();
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    let foreign = text.replace("ConstructCAD_Curtain", "Other_Curtain");
    let (imported, report) = import(&foreign, Project::from_template()).unwrap();
    let (a, b) = (curtain_figures(&original), curtain_figures(&imported));
    // The free straight wall (a plain grid) comes back as a curtain wall of the same grid.
    let free = original.walls().find(|(_, w)| w.start.y == 12.0).map(|(e, _)| e.uid).unwrap();
    let (x, y) = (a.iter().find(|f| f.0 == free).unwrap(), b.iter().find(|f| f.0 == free).unwrap());
    assert_eq!((x.1, x.2, x.3), (y.1, y.2, y.3));
    close(x.4, y.4, 1e-6, "glazed area");
    close(x.5, y.5, 1e-6, "mullion length");
    // The merged panel and the arc have no plain grid: their parts come in as objects.
    assert_eq!(b.len(), 1, "{b:?}");
    assert!(report.generic.values().sum::<usize>() > 0, "{:?}", report.generic);
}

#[test]
fn site_location_round_trips() {
    let mut p = Project::from_template();
    let mut site = cc_core::site::SiteLocation::at(-3.731_862_5, -38.526_670_1);
    site.elevation = 21.25;
    site.true_north = 12.5;
    site.place = "Praça do Ferreira".into();
    site.address.city = "Fortaleza".into();
    site.address.region = "CE".into();
    site.address.postcode = "60025-000".into();
    site.address.country = "Brasil".into();
    p.info.address = "Rua Floriano Peixoto, 10".into();
    p.info.site = Some(site.clone());
    let (imported, _) = round_trip(&p);
    let got = imported.info.site.expect("the site location comes back");
    // Compound angles keep millionths of a second (about 3 cm).
    close(got.latitude, site.latitude, 1e-9, "latitude");
    close(got.longitude, site.longitude, 1e-9, "longitude");
    close(got.elevation, site.elevation, 1e-9, "elevation");
    close(got.true_north, site.true_north, 1e-6, "true north");
    assert_eq!(got.place, site.place);
    assert_eq!(got.address.city, "Fortaleza");
    assert_eq!(got.address.postcode, "60025-000");
    assert_eq!(imported.info.address, "Rua Floriano Peixoto, 10");
    // Without a location, none comes back.
    let (imported, _) = round_trip(&Project::from_template());
    assert!(imported.info.site.is_none());
}

/// Windows keep how their panels open (#239): the type's
/// `IfcWindowPanelProperties` and partitioning, read back into a project
/// that has no such window type.
#[test]
fn window_operations_round_trip() {
    use cc_core::{Document, Hosted, Wall, WindowOperation};
    let mut doc = Document::new(Project::from_template());
    let names = [
        "Janela 120x120",
        "Janela de abrir 60x100",
        "Janela de abrir 2 folhas 120x120",
        "Janela maxim-ar 60x60",
        "Janela de correr 150x120",
    ];
    doc.edit("windows", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, type_id, kind| {
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        };
        let wall_type = e.project().default_type(Category::Wall).unwrap();
        let wall = Wall { start: DVec2::new(0.0, 0.0), end: DVec2::new(16.0, 0.0), ..Default::default() };
        let host = add(e, wall_type, ElementKind::Wall(wall));
        for (i, name) in names.iter().enumerate() {
            let ty = e.project().types_of(Category::Window).find(|t| t.name == *name).unwrap().id;
            let at =
                Hosted { host, offset: 2.0 + 3.0 * i as f64, sill_height: 1.0, flip_facing: false, flip_hand: false };
            add(e, ty, ElementKind::Window(at));
        }
    });
    let original = doc.project;
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    for literal in [
        ".FIXEDCASEMENT.",
        ".SIDEHUNGLEFTHAND.",
        ".SIDEHUNGRIGHTHAND.",
        ".TOPHUNG.",
        ".SLIDINGHORIZONTAL.",
        ".DOUBLE_PANEL_VERTICAL.",
    ] {
        assert!(text.contains(literal), "{literal}");
    }
    assert_eq!(text.matches("IFCWINDOWPANELPROPERTIES(").count(), 7);
    let mut base = Project::from_template();
    base.types.retain(|_, t| t.window_operation() == WindowOperation::Fixed);
    let (imported, _) = import(&text, base).unwrap();
    let lookup = by_uid(&imported);
    for e in original.elements.values().filter(|e| e.category() == Category::Window) {
        let i = lookup.get(&e.uid).unwrap();
        let (a, b) = (original.element_type(e.type_id).unwrap(), imported.element_type(i.type_id).unwrap());
        assert_eq!(b.window_operation(), a.window_operation(), "{}", a.name);
        assert_eq!(b.opening_size(), a.opening_size());
    }
}

/// Doors keep how their leaves swing (#249): `IfcDoor` and `IfcDoorType`
/// carry the operation type (a flipped single door the other hand) and the
/// type its `IfcDoorPanelProperties`; import reads them back into a project
/// that has no such door type, flipping the hand of the mirrored one.
#[test]
fn door_operations_round_trip() {
    use cc_core::{Document, DoorOperation, Hosted, Wall};
    let mut doc = Document::new(Project::from_template());
    let doors = [("Porta 80x210", false), ("Porta-janela 80x220", true), ("Porta-janela 2 folhas 140x220", false)];
    doc.edit("doors", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, type_id, kind| {
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        };
        let wall_type = e.project().default_type(Category::Wall).unwrap();
        let wall = Wall { start: DVec2::new(0.0, 0.0), end: DVec2::new(12.0, 0.0), ..Default::default() };
        let host = add(e, wall_type, ElementKind::Wall(wall));
        for (i, (name, flip_hand)) in doors.iter().enumerate() {
            let ty = e.project().types_of(Category::Door).find(|t| t.name == *name).unwrap().id;
            let at = Hosted {
                host,
                offset: 2.0 + 3.0 * i as f64,
                sill_height: 0.0,
                flip_facing: false,
                flip_hand: *flip_hand,
            };
            add(e, ty, ElementKind::Door(at));
        }
    });
    let original = doc.project;
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    let doors_with = |op: &str| text.lines().filter(|l| l.contains("IFCDOOR(") && l.contains(op)).count();
    assert_eq!(doors_with(".SINGLE_SWING_LEFT."), 1, "the plain door");
    assert_eq!(doors_with(".SINGLE_SWING_RIGHT."), 1, "the flipped French door");
    assert_eq!(doors_with(".DOUBLE_DOOR_SINGLE_SWING."), 1, "the double French door");
    let types_with = |op: &str| text.lines().filter(|l| l.contains("IFCDOORTYPE(") && l.contains(op)).count();
    assert_eq!(types_with(".DOUBLE_DOOR_SINGLE_SWING."), 1);
    assert_eq!(types_with(".SINGLE_SWING_LEFT."), 2, "the plain and the French door");
    assert!(!text.lines().any(|l| l.contains("IFCDOORTYPE(") && l.contains(".SINGLE_SWING_RIGHT.")));
    // A panel per leaf of every door type placed: a plain door, a French door and a double one.
    assert_eq!(text.matches("IFCDOORPANELPROPERTIES(").count(), 4);
    assert!(text.contains(".SWINGING."));
    let mut base = Project::from_template();
    base.types.retain(|_, t| t.category() != Category::Door);
    let (imported, _) = import(&text, base).unwrap();
    let lookup = by_uid(&imported);
    for e in original.elements.values().filter(|e| e.category() == Category::Door) {
        let i = lookup.get(&e.uid).unwrap();
        let (a, b) = (original.element_type(e.type_id).unwrap(), imported.element_type(i.type_id).unwrap());
        assert_eq!(b.door_operation(), a.door_operation(), "{}", a.name);
        assert_eq!(b.opening_size(), a.opening_size());
        assert_eq!(i.as_hosted().map(|h| h.flip_hand), e.as_hosted().map(|h| h.flip_hand), "{}", a.name);
    }
    assert!(imported.types_of(Category::Door).any(|t| t.door_operation() == DoorOperation::DoubleDoor));
}

/// Occurrences of an IFC type whose name a different project type has
/// import as one renamed type, not one type each (#247), and the types an
/// import adds keep unique uids.
#[test]
fn clashing_type_names_import_as_one_type() {
    use cc_core::{Document, Hosted, Wall};
    // The project's "Janela 120x120" is 130 cm wide: the template's stays 120.
    let mut project = Project::from_template();
    let ty = project.types_of(Category::Window).find(|t| t.name == "Janela 120x120").unwrap().id;
    if let Some(TypeData::Component { def, .. }) = project.types.get_mut(&ty).map(|t| &mut t.data) {
        def.params[0].value = 1.3;
    }
    let mut doc = Document::new(project);
    doc.edit("windows", |e| {
        let level = e.project().levels[0].id;
        let add = |e: &mut cc_core::EditCtx, type_id, kind| {
            let id = cc_core::ElementId(e.alloc_id());
            e.add_element(Element {
                id,
                level,
                type_id,
                mark: String::new(),
                comments: String::new(),
                kind,
                uid: Uid::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            })
        };
        let wall_type = e.project().default_type(Category::Wall).unwrap();
        let wall = Wall { start: DVec2::new(0.0, 0.0), end: DVec2::new(10.0, 0.0), ..Default::default() };
        let host = add(e, wall_type, ElementKind::Wall(wall));
        for i in 0..3 {
            let at =
                Hosted { host, offset: 2.0 + 3.0 * i as f64, sill_height: 1.0, flip_facing: false, flip_hand: false };
            add(e, ty, ElementKind::Window(at));
        }
    });
    let (imported, _) = round_trip(&doc.project);
    let names: Vec<&str> = imported
        .elements
        .values()
        .filter(|e| e.category() == Category::Window)
        .map(|e| imported.type_name(e.type_id))
        .collect();
    assert_eq!(names, ["Janela 120x120 (IFC)"; 3]);
    let windows: Vec<_> =
        imported.types_of(Category::Window).filter(|t| t.name.starts_with("Janela 120x120")).collect();
    assert_eq!(windows.len(), 2, "the template's and the imported one");
    assert_eq!(windows.iter().find(|t| t.name.ends_with("(IFC)")).unwrap().opening_size(), Some((1.3, 1.2)));
    let mut uids: Vec<Uid> = imported.types.values().map(|t| t.uid).filter(|u| *u != Uid::default()).collect();
    let n = uids.len();
    uids.sort();
    uids.dedup();
    assert_eq!(uids.len(), n, "type uids are unique");
}

/// One profile per shape code (the sketch with an opening), each with a
/// column type and a beam type and an element of each; plus a turned and a
/// mirrored type. Types are built through the document, so their legacy
/// fields follow their profiles.
fn profiles_project() -> Project {
    use cc_core::structural_profile::{ParamSketch, ProfileDef, ProfileMaterial, ProfileShape};
    use cc_core::{
        Beam, Column, Document, ElementType, ElementTypeId, MaterialClass, ProfileId, ProfilePoint, WallTop,
    };
    let mut doc = Document::new(Project::from_template());
    doc.edit("profiles", |e| {
        let p = e.project();
        let level = p.levels[0].id;
        let material =
            |class: MaterialClass| p.materials.iter().find(|m| m.identity.class == class).unwrap().name.clone();
        let (steel, concrete) = (material(MaterialClass::Metal), material(MaterialClass::Concrete));
        let mut shapes: Vec<(String, ProfileShape, f64, bool)> = ProfileShape::CODES
            .iter()
            .map(|code| {
                let mut shape = ProfileShape::default_for(code).unwrap();
                if let ProfileShape::Sketch(s) = &mut shape {
                    let hole = [("-0.02", "-0.03"), ("-0.02", "0.01"), ("0.02", "0.01"), ("0.02", "-0.03")];
                    *s = ParamSketch {
                        openings: vec![hole.iter().map(|(u, v)| ProfilePoint::new(*u, *v)).collect()],
                        ..ParamSketch::square(0.1, 0.12)
                    };
                }
                (format!("P {code}"), shape, 0.0, false)
            })
            .collect();
        shapes.push(("P i turned".into(), ProfileShape::default_for("i").unwrap(), 30.0, false));
        shapes.push(("P channel mirrored".into(), ProfileShape::default_for("channel").unwrap(), 0.0, true));
        let mut add = Vec::new();
        for (i, (name, shape, rotation, mirror)) in shapes.into_iter().enumerate() {
            let concrete_shape = matches!(
                shape,
                ProfileShape::Rect { .. }
                    | ProfileShape::Round { .. }
                    | ProfileShape::ConcreteT { .. }
                    | ProfileShape::ConcreteL { .. }
                    | ProfileShape::AnnularSector { .. }
                    | ProfileShape::Sketch(_)
            );
            let (pm, mat) = if concrete_shape {
                (ProfileMaterial::Concrete, concrete.clone())
            } else if shape.is_cold_formed() {
                (ProfileMaterial::ColdFormed, steel.clone())
            } else {
                (ProfileMaterial::Steel, steel.clone())
            };
            let section = match shape {
                ProfileShape::Round { d } if rotation == 0.0 && !mirror => ColumnSection::Round { diameter: d },
                ProfileShape::RoundTube { d, t } if rotation == 0.0 && !mirror => {
                    ColumnSection::Pipe { diameter: d, wall: t }
                }
                _ => ColumnSection::Rectangular { width: 0.1, depth: 0.1 },
            };
            add.push((i, name, pm, mat, shape, section, rotation, mirror));
        }
        for (i, name, pm, mat, shape, section, rotation, mirror) in add {
            let pid = ProfileId(e.alloc_id());
            let def = ProfileDef {
                id: pid,
                uid: Uid::generate(),
                name: name.clone(),
                material: pm,
                shape,
                catalog: None,
                description: String::new(),
            };
            e.profiles_mut().insert(pid, def);
            let column_type = TypeData::Column {
                section,
                material: mat.clone(),
                profile: Some(pid),
                profile_rotation_deg: rotation,
                profile_mirror: mirror,
            };
            let beam_type = TypeData::Beam {
                width: 0.1,
                height: 0.1,
                material: mat,
                shape: cc_core::SectionShape::Solid,
                profile: Some(pid),
                profile_rotation_deg: rotation,
                profile_mirror: mirror,
            };
            let x = 6.0 * i as f64;
            let column = Column {
                position: DVec2::new(x, 0.0),
                rotation_deg: 15.0 * (i % 3) as f64,
                base_offset: 0.0,
                height: 3.0,
                top: WallTop::Unconnected,
                structural: true,
                base_support: Default::default(),
            };
            let beam = Beam::new(DVec2::new(x, 10.0), DVec2::new(x + 4.0, 10.0 + 0.5 * (i % 2) as f64), 3.0);
            for (type_name, data, kind) in [
                (format!("C {name}"), column_type, ElementKind::Column(column)),
                (format!("V {name}"), beam_type, ElementKind::Beam(beam)),
            ] {
                let type_id = ElementTypeId(e.alloc_id());
                e.add_type(ElementType {
                    id: type_id,
                    name: type_name,
                    data,
                    info: Default::default(),
                    uid: Uid::generate(),
                });
                let id = cc_core::ElementId(e.alloc_id());
                e.add_element(Element {
                    id,
                    level,
                    type_id,
                    mark: String::new(),
                    comments: String::new(),
                    kind,
                    uid: Uid::default(),
                    phase_created: Default::default(),
                    phase_demolished: None,
                    design_option: None,
                    paints: Vec::new(),
                    plugin_data: Default::default(),
                });
            }
        }
    });
    doc.project
}

/// Writes `fixtures/profiles.ccad` from [`profiles_project`]
/// (`cargo test -p cc_ifc --test round_trip write_profiles_fixture -- --ignored`).
#[test]
#[ignore]
fn write_profiles_fixture() {
    let text = cc_core::to_ron(&profiles_project()).unwrap();
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/profiles.ccad"), text).unwrap();
}

/// Bounds and volume of an element's body.
fn body_figures(g: &cc_geom::ModelGeometry, id: cc_core::ElementId) -> (glam::DVec3, glam::DVec3, f64) {
    let (lo, hi) = g
        .body_of(id)
        .flat_map(|s| s.faces.iter().flat_map(|f| f.verts.iter().copied()))
        .fold((glam::DVec3::splat(f64::INFINITY), glam::DVec3::splat(f64::NEG_INFINITY)), |(lo, hi), p| {
            (lo.min(p), hi.max(p))
        });
    (lo, hi, g.volume(id))
}

#[test]
fn profiles_round_trip() {
    use cc_core::structural_profile::ProfileShape;
    let original = profiles_project();
    let text =
        export(&original, &ExportOptions { time_stamp: Some("2026-01-01T00:00:00".into()), file_name: "t.ifc".into() });
    for entity in [
        "IFCISHAPEPROFILEDEF",
        "IFCUSHAPEPROFILEDEF",
        "IFCTSHAPEPROFILEDEF",
        "IFCLSHAPEPROFILEDEF",
        "IFCCSHAPEPROFILEDEF",
        "IFCRECTANGLEHOLLOWPROFILEDEF",
        "IFCCIRCLEHOLLOWPROFILEDEF",
        "IFCRECTANGLEPROFILEDEF",
        "IFCCIRCLEPROFILEDEF",
        "IFCARBITRARYCLOSEDPROFILEDEF",
        "IFCARBITRARYPROFILEDEFWITHVOIDS",
        "IFCMATERIALPROFILE",
        "IFCMATERIALPROFILESET",
        "IFCMATERIALPROFILESETUSAGE",
    ] {
        assert!(text.contains(&format!("={entity}(")), "{entity}");
    }
    assert!(text.contains("'P i'"), "profile names are written");
    check(&original);
    let (imported, _) = round_trip(&original);

    // Every type comes back with its profile: the same shape (parametric
    // kinds) or the same area (arbitrary ones), under the same name.
    let parametric = ["i", "channel", "t", "angle", "cold_lipped_u", "rect_tube", "round_tube", "rect", "round"];
    let mut types = 0;
    for t in original.types.values() {
        let (TypeData::Column { profile: Some(pid), .. } | TypeData::Beam { profile: Some(pid), .. }) = &t.data else {
            continue;
        };
        let def = original.profile(*pid).unwrap();
        if !def.name.starts_with("P ") {
            continue;
        }
        types += 1;
        let back = imported
            .types_of(t.category())
            .find(|x| x.name == t.name)
            .unwrap_or_else(|| panic!("type {} not imported", t.name));
        let (TypeData::Column { profile: Some(bid), .. } | TypeData::Beam { profile: Some(bid), .. }) = &back.data
        else {
            panic!("{} came back without a profile: {:?}", t.name, back.data);
        };
        let bdef = imported.profile(*bid).unwrap();
        assert_eq!(bdef.name, def.name, "profile name of {}", t.name);
        assert_eq!(bdef.material, def.material, "profile material of {}", t.name);
        let mirrored = def.name.ends_with("mirrored");
        // An annular sector is written with its arcs and read back as one.
        if (parametric.contains(&def.shape.code()) || def.shape.code() == "annular_sector") && !mirrored {
            assert_eq!(bdef.shape, def.shape, "shape of {}", t.name);
        } else {
            assert!(matches!(bdef.shape, ProfileShape::Sketch(_)), "{} is a sketch: {:?}", t.name, bdef.shape);
        }
        // The placed sections coincide: area, box and centroid.
        let a = original.type_section(&t.data).unwrap().outline;
        let b = imported.type_section(&back.data).unwrap().outline;
        let (pa, pb) = (a.properties(0.0), b.properties(0.0));
        assert!((pa.area - pb.area).abs() <= 0.005 * pa.area, "area of {}: {} vs {}", t.name, pa.area, pb.area);
        let ((la, ha), (lb, hb)) = (a.bbox(), b.bbox());
        close2(ha - la, hb - lb, 1e-5, &format!("box of {}", t.name));
        let centroid = |c: [f64; 2], lo: DVec2| DVec2::new(c[0], c[1]) - lo;
        close2(centroid(pa.centroid, la), centroid(pb.centroid, lb), 1e-5, &format!("centroid of {}", t.name));
    }
    assert_eq!(types, 2 * (ProfileShape::CODES.len() + 2));

    // The bodies stand where they stood: bounds and volume.
    let (g0, g1) = (cc_geom::ModelGeometry::build(&original), cc_geom::ModelGeometry::build(&imported));
    let lookup = by_uid(&imported);
    let mut bodies = 0;
    for e in original.elements.values().filter(|e| matches!(e.category(), Category::Column | Category::Beam)) {
        let i = lookup[&e.uid];
        let (lo0, hi0, v0) = body_figures(&g0, e.id);
        let (lo1, hi1, v1) = body_figures(&g1, i.id);
        let name = original.type_name(e.type_id);
        assert!(v0 > 0.0, "{name}");
        assert!((v0 - v1).abs() <= 0.005 * v0, "volume of {name}: {v0} vs {v1}");
        assert!(
            (lo0 - lo1).abs().max_element() < 1e-5 && (hi0 - hi1).abs().max_element() < 1e-5,
            "bounds of {name}: {lo0} {hi0} vs {lo1} {hi1}"
        );
        bodies += 1;
    }
    assert_eq!(bodies, 2 * (ProfileShape::CODES.len() + 2));
}

/// A project with a profile of `shape` named `name` and one column or beam
/// type of it, inserted as they are (no edit, so no repair or sync).
fn with_profile_type(
    p: &mut Project,
    name: &str,
    shape: cc_core::structural_profile::ProfileShape,
    data: impl FnOnce(cc_core::ProfileId, String) -> TypeData,
) -> cc_core::ElementTypeId {
    use cc_core::structural_profile::{ProfileDef, ProfileMaterial};
    let pid = cc_core::ProfileId(p.alloc_id());
    let def = ProfileDef {
        id: pid,
        uid: Uid::generate(),
        name: name.into(),
        material: ProfileMaterial::Steel,
        shape,
        catalog: None,
        description: String::new(),
    };
    p.profiles.insert(pid, def);
    let material = p.materials.iter().find(|m| m.identity.class == cc_core::MaterialClass::Metal).unwrap().name.clone();
    let tid = cc_core::ElementTypeId(p.alloc_id());
    let ty = cc_core::ElementType {
        id: tid,
        name: format!("T {name}"),
        data: data(pid, material),
        info: Default::default(),
        uid: Uid::generate(),
    };
    p.types.insert(tid, ty);
    tid
}

fn add_plain(p: &mut Project, type_id: cc_core::ElementTypeId, kind: ElementKind) -> cc_core::ElementId {
    let id = cc_core::ElementId(p.alloc_id());
    let e = Element {
        id,
        level: p.levels[0].id,
        type_id,
        mark: String::new(),
        comments: String::new(),
        kind,
        uid: Uid::generate(),
        phase_created: Default::default(),
        phase_demolished: None,
        design_option: None,
        paints: Vec::new(),
        plugin_data: Default::default(),
    };
    p.elements.insert(id, e);
    id
}

/// A column whose profile makes no outline (a hostile file, before repair)
/// is still written: as its type's box, with its type's material.
#[test]
fn a_column_without_a_section_keeps_its_box_and_material() {
    use cc_core::structural_profile::ProfileShape;
    let mut p = Project::from_template();
    // Flanges thicker than half the depth: no outline.
    let bad = ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.005, tf: 0.15, r: 0.0 };
    let mut material = String::new();
    let tid = with_profile_type(&mut p, "Bad", bad, |pid, m| {
        material = m.clone();
        TypeData::Column {
            section: ColumnSection::Rectangular { width: 0.2, depth: 0.3 },
            material: m,
            profile: Some(pid),
            profile_rotation_deg: 0.0,
            profile_mirror: false,
        }
    });
    assert!(p.type_section(&p.types[&tid].data).is_none());
    let column = cc_core::Column {
        position: DVec2::new(1.0, 2.0),
        rotation_deg: 0.0,
        base_offset: 0.0,
        height: 3.0,
        top: cc_core::WallTop::Unconnected,
        structural: true,
        base_support: Default::default(),
    };
    add_plain(&mut p, tid, ElementKind::Column(column));
    let text = export(&p, &ExportOptions::default());
    assert_eq!(text.lines().filter(|l| l.contains("=IFCCOLUMN(")).count(), 1);
    // Its body is the type's box.
    assert!(
        text.lines().any(|l| l.contains("=IFCRECTANGLEPROFILEDEF(.AREA.,$,") && l.ends_with(",0.2,0.3);")),
        "box profile"
    );
    let (imported, _) = round_trip(&p);
    let back = imported.elements.values().find(|e| e.category() == Category::Column).unwrap();
    let g = cc_geom::ModelGeometry::build(&imported);
    assert!((g.volume(back.id) - 0.2 * 0.3 * 3.0).abs() < 1e-9);
    // The type carries the plain material.
    let type_id = text.lines().find(|l| l.contains("=IFCCOLUMNTYPE(") && l.contains("'T Bad'")).unwrap();
    let type_id = type_id.split('=').next().unwrap();
    let association =
        text.lines().find(|l| l.contains("=IFCRELASSOCIATESMATERIAL(") && l.contains(&format!("({type_id})"))).unwrap();
    let material_id = association.rsplit(',').next().unwrap().trim_end_matches(");");
    assert!(text.lines().any(|l| l.starts_with(&format!("{material_id}=IFCMATERIAL("))), "{association}");
    let back_type = imported.element_type(back.type_id).unwrap();
    assert_eq!(back_type.main_material(), material);
}

/// A sloped profile beam goes out as a B-rep and comes back as a component
/// of the same body.
#[test]
fn sloped_profile_beams_come_back_as_their_body() {
    use cc_core::structural_profile::ProfileShape;
    let mut p = Project::from_template();
    let shape = ProfileShape::default_for("i").unwrap();
    let tid = with_profile_type(&mut p, "Sloped", shape, |pid, m| TypeData::Beam {
        width: 0.1,
        height: 0.2,
        material: m,
        shape: cc_core::SectionShape::Solid,
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    });
    let mut beam = cc_core::Beam::new(DVec2::ZERO, DVec2::new(4.0, 1.0), 3.0);
    beam.end_offset = Some(3.6);
    let id = add_plain(&mut p, tid, ElementKind::Beam(beam));
    assert_eq!(p.repair(), 0);
    let text = export(&p, &ExportOptions::default());
    assert!(text.contains("=IFCFACETEDBREP("));
    let (imported, _) = round_trip(&p);
    let uid = p.elements[&id].uid;
    let back = imported.elements.values().find(|e| e.uid == uid).unwrap();
    assert_eq!(back.category(), Category::Component);
    let (g0, g1) = (cc_geom::ModelGeometry::build(&p), cc_geom::ModelGeometry::build(&imported));
    let (lo0, hi0, v0) = body_figures(&g0, id);
    let (lo1, hi1, v1) = body_figures(&g1, back.id);
    assert!(v0 > 0.0);
    assert!((v0 - v1).abs() <= 1e-6 * v0.max(1.0) + 0.005 * v0, "volume {v0} vs {v1}");
    assert!(
        (lo0 - lo1).abs().max_element() < 1e-5 && (hi0 - hi1).abs().max_element() < 1e-5,
        "{lo0} {hi0} vs {lo1} {hi1}"
    );
}

/// A file whose section differs from a project profile of the same name only
/// by float noise reuses that profile instead of making "name 2".
#[test]
fn float_noise_reuses_the_profile_of_the_same_name() {
    use cc_core::structural_profile::ProfileShape;
    let shape = ProfileShape::I { d: 0.3, bf: 0.15, tw: 0.0071, tf: 0.0107, r: 0.0 };
    let mut p = Project::from_template();
    let tid = with_profile_type(&mut p, "Noisy", shape.clone(), |pid, m| TypeData::Beam {
        width: 0.15,
        height: 0.3,
        material: m,
        shape: cc_core::SectionShape::Solid,
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    });
    add_plain(&mut p, tid, ElementKind::Beam(cc_core::Beam::new(DVec2::ZERO, DVec2::new(4.0, 0.0), 3.0)));
    let text = export(&p, &ExportOptions::default());
    // The project imported into has "Noisy" a few nanometers off.
    let mut base = Project::from_template();
    let noisy = ProfileShape::I { d: 0.3 + 3e-9, bf: 0.15 - 2e-9, tw: 0.0071, tf: 0.0107 + 1e-9, r: 0.0 };
    with_profile_type(&mut base, "Noisy", noisy, |pid, m| TypeData::Beam {
        width: 0.15,
        height: 0.3,
        material: m,
        shape: cc_core::SectionShape::Solid,
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    });
    let before = base.profiles.len();
    let (imported, _) = import(&text, base).unwrap();
    assert!(imported.profile_named("Noisy 2").is_none(), "{:?}", imported.profiles.values().map(|d| &d.name));
    assert_eq!(imported.profiles.len(), before);
    // A real difference still makes a new profile.
    let mut base = Project::from_template();
    let other = ProfileShape::I { d: 0.31, bf: 0.15, tw: 0.0071, tf: 0.0107, r: 0.0 };
    with_profile_type(&mut base, "Noisy", other, |pid, m| TypeData::Beam {
        width: 0.15,
        height: 0.31,
        material: m,
        shape: cc_core::SectionShape::Solid,
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    });
    let (imported, _) = import(&text, base).unwrap();
    assert!(imported.profile_named("Noisy 2").is_some());
}
