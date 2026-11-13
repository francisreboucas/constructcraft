//! Structural profiles: columns and beams built and drawn from their
//! profile, at the view's detail level.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_core::structural_profile::{ProfileDef, ProfileMaterial, ProfileShape};
use cc_core::{
    ColumnSection, Document, ElementId, ElementKind, ElementType, ElementTypeId, ProfileId, SectionShape, TypeData,
    Uid, ViewId,
};
use cc_engine::Session;
use cc_views::{LineStyle, Prim};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// IPE 200 (EN 10365): depth, flange width, web, flange, root radius.
const IPE200: ProfileShape = ProfileShape::I { d: 0.2, bf: 0.1, tw: 0.0056, tf: 0.0085, r: 0.012 };

/// A session whose project has a steel profile of `shape` and a column type
/// pointing to it (until profile commands exist).
fn with_column_type(shape: ProfileShape) -> (Session, ElementTypeId) {
    let mut p = cc_engine::new_project();
    let pid = ProfileId(p.alloc_id());
    p.profiles.insert(
        pid,
        ProfileDef {
            id: pid,
            uid: Uid::generate(),
            name: "IPE 200".into(),
            material: ProfileMaterial::Steel,
            shape,
            catalog: None,
            description: String::new(),
        },
    );
    let steel = p
        .types
        .values()
        .find(|t| matches!(t.data, TypeData::Column { section: ColumnSection::Profile { .. }, .. }))
        .unwrap()
        .clone();
    let tid = ElementTypeId(p.alloc_id());
    let TypeData::Column { material, .. } = steel.data else { panic!("not a column type") };
    let data = TypeData::Column {
        section: ColumnSection::Profile {
            width: 0.1,
            depth: 0.2,
            shape: SectionShape::I { web: 0.0056, flange: 0.0085 },
        },
        material,
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    };
    p.types.insert(
        tid,
        ElementType { id: tid, name: "IPE 200".into(), data, info: Default::default(), uid: Uid::generate() },
    );
    (Session::new(Document::new(p)), tid)
}

#[test]
fn a_column_with_an_ipe_profile_has_its_area_times_height() {
    let (mut s, tid) = with_ipe_column_type();
    let col = run(&mut s, "column_create", json!({"type": tid.0, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
    let id = ElementId(col);
    let (base, top) = {
        let p = s.project();
        let e = p.element(id).unwrap();
        let ElementKind::Column(c) = &e.kind else { panic!("not a column") };
        p.column_span(e, c)
    };
    let height = top - base;
    let volume = s.geometry().volume(id);
    let area =
        2.0 * 0.1 * 0.0085 + (0.2 - 0.017) * 0.0056 + 4.0 * (1.0 - std::f64::consts::FRAC_PI_4) * 0.012f64.powi(2);
    assert!((volume - area * height).abs() / (area * height) < 0.003, "{volume} vs {}", area * height);
}

/// Cut lines of element `id` in the view's drawing.
fn cut_lines(s: &mut Session, view: u64, id: u64) -> usize {
    let d = s.drawing(ViewId(view)).unwrap();
    d.prims
        .iter()
        .filter(|p| matches!(p, Prim::Line { style: LineStyle::Cut, element: Some(e), .. } if e.0 == id))
        .count()
}

#[test]
fn plans_cut_a_profile_column_at_their_detail_level() {
    let (mut s, tid) = with_ipe_column_type();
    let view = run(&mut s, "views_list", json!({"category": "floor_plan"}))[0]["id"].as_u64().unwrap();
    let col = run(&mut s, "column_create", json!({"type": tid.0, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
    run(&mut s, "view_set", json!({"view": view, "detail": "coarse"}));
    assert_eq!(cut_lines(&mut s, view, col), 4, "the bounding rectangle");
    run(&mut s, "view_set", json!({"view": view, "detail": "medium"}));
    assert_eq!(cut_lines(&mut s, view, col), 12, "the I without its root radii");
    run(&mut s, "view_set", json!({"view": view, "detail": "fine"}));
    assert!(cut_lines(&mut s, view, col) > 12, "the I with its root radii");
}

fn with_ipe_column_type() -> (Session, ElementTypeId) {
    with_column_type(IPE200)
}

#[test]
fn a_beam_ending_between_the_flanges_frames_into_the_column_box() {
    // The I stands with its web along y; the beam comes along x and ends
    // between the flanges, off the web by more than the join tolerance.
    let (mut s, tid) = with_ipe_column_type();
    run(&mut s, "column_create", json!({"type": tid.0, "position": [0.0, 0.0]}));
    let beam = run(&mut s, "beam_create", json!({"start": [5.0, 0.0], "end": [0.03, 0.0]}))["id"].as_u64().unwrap();
    let g = s.geometry().beams.get(&ElementId(beam)).cloned().unwrap();
    assert!(matches!(g.end, cc_geom::BeamEnd::Column(_)), "{:?}", g.end);
    // Trimmed at the face of the box, x = bf / 2.
    let face = 5.0 - 0.05;
    assert!((g.end_ext.0 - face).abs() < 1e-9 && (g.end_ext.1 - face).abs() < 1e-9, "{:?}", g.end_ext);
}

#[test]
fn a_tube_column_is_cut_with_its_hole() {
    let tube = ProfileShape::RectTube { h: 0.15, b: 0.15, t: 0.006, ro: 0.0 };
    let (mut s, tid) = with_column_type(tube);
    let view = run(&mut s, "views_list", json!({"category": "floor_plan"}))[0]["id"].as_u64().unwrap();
    let col = run(&mut s, "column_create", json!({"type": tid.0, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
    run(&mut s, "view_set", json!({"view": view, "detail": "fine"}));
    assert_eq!(cut_lines(&mut s, view, col), 8, "outside and hole outlines");
    let d = s.drawing(ViewId(view)).unwrap();
    let fills: Vec<&Vec<glam::DVec2>> = d
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Fill { pts, element: Some(e), .. } if e.0 == col => Some(pts),
            _ => None,
        })
        .collect();
    assert!(!fills.is_empty());
    let filled = |q: glam::DVec2| fills.iter().any(|f| cc_geom::polygon::contains(f, q));
    assert!(filled(glam::DVec2::new(0.072, 0.0)), "the wall is filled");
    assert!(!filled(glam::DVec2::ZERO), "the hole is not");
    assert!(!filled(glam::DVec2::new(0.05, 0.05)), "the hole is not");
}

/// W 310x38.7 (ASTM A6): depth, flange width, web, flange, root radius.
const W310: ProfileShape = ProfileShape::I { d: 0.31, bf: 0.165, tw: 0.0058, tf: 0.0097, r: 0.008 };

/// Add a steel profile of `shape` and a beam type pointing to it.
fn add_beam_type(s: &mut Session, shape: ProfileShape) -> ElementTypeId {
    let mut p = s.project().clone();
    let pid = ProfileId(p.alloc_id());
    p.profiles.insert(
        pid,
        ProfileDef {
            id: pid,
            uid: Uid::generate(),
            name: "W 310x38.7".into(),
            material: ProfileMaterial::Steel,
            shape: shape.clone(),
            catalog: None,
            description: String::new(),
        },
    );
    let material = p
        .types
        .values()
        .find_map(|t| match &t.data {
            TypeData::Beam { material, .. } => Some(material.clone()),
            _ => None,
        })
        .unwrap();
    let ProfileShape::I { d, bf, tw, tf, .. } = shape else { panic!("not an I") };
    let tid = ElementTypeId(p.alloc_id());
    let data = TypeData::Beam {
        width: bf,
        height: d,
        material,
        shape: SectionShape::I { web: tw, flange: tf },
        profile: Some(pid),
        profile_rotation_deg: 0.0,
        profile_mirror: false,
    };
    p.types.insert(
        tid,
        ElementType { id: tid, name: "W 310x38.7".into(), data, info: Default::default(), uid: Uid::generate() },
    );
    *s = Session::new(Document::new(p));
    tid
}

#[test]
fn beams_with_w_profiles_meet_in_a_mitered_l() {
    let (mut s, _) = with_ipe_column_type();
    let tid = add_beam_type(&mut s, W310);
    let make = |s: &mut Session, a: [f64; 2], b: [f64; 2]| {
        let id = run(s, "beam_create", json!({"type": tid.0, "start": a, "end": b}))["id"].as_u64().unwrap();
        ElementId(id)
    };
    let a = make(&mut s, [0.0, 0.0], [4.0, 0.0]);
    let b = make(&mut s, [4.0, 0.0], [4.0, 4.0]);
    // Volumes count the root radii whole, not as their chords.
    let area = W310.area().unwrap();
    let length = 4.0;
    let g = s.geometry();
    let (ga, gb) = (&g.beams[&a], &g.beams[&b]);
    assert_eq!((ga.end, gb.start), (cc_geom::BeamEnd::Beam, cc_geom::BeamEnd::Beam));
    assert!(ga.curved, "the root radii are arcs");
    let (va, vb) = (g.volume(a), g.volume(b));
    assert!(va > 0.0 && vb > 0.0, "{va} {vb}");
    // Mitered on their axes the symmetric W's keep area x axis length each:
    // the corner the square ends would overlap is cut away.
    let square = 2.0 * area * (length + ga.half);
    assert!(va + vb < square, "{} vs {square}", va + vb);
    assert!(
        (va + vb - 2.0 * area * length).abs() / (2.0 * area * length) < 1e-6,
        "{} vs {}",
        va + vb,
        2.0 * area * length
    );
}

/// The linear-mass cell of the only beam of a beams schedule.
fn beam_schedule_cells(s: &mut Session) -> Vec<String> {
    let schedule = run(s, "view_create", json!({"kind": "schedule", "category": "beam"}));
    run(
        s,
        "view_set",
        json!({"view": schedule["id"], "params": {"def": {"fields": ["Type", "Profile", "LinearMass"]}}}),
    );
    let table = run(s, "schedule_table", json!({"view": schedule["id"]}));
    let rows = table["rows"].as_array().unwrap();
    rows[0]["cells"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect()
}

#[test]
fn schedules_show_the_profile_and_its_linear_mass() {
    let (mut s, _) = with_ipe_column_type();
    let tid = add_beam_type(&mut s, W310);
    // Give the beam type the template's steel.
    let mut p = s.project().clone();
    let steel = p.materials.iter().find(|m| m.identity.class == cc_core::MaterialClass::Metal).unwrap().name.clone();
    let TypeData::Beam { material, .. } = &mut p.types.get_mut(&tid).unwrap().data else { panic!("not a beam") };
    *material = steel.clone();
    let mut s = Session::new(Document::new(p.clone()));
    run(&mut s, "beam_create", json!({"type": tid.0, "start": [0.0, 0.0], "end": [4.0, 0.0]}));
    // The template's steel carries a density of 7850 kg/m3.
    let cells = beam_schedule_cells(&mut s);
    assert!(cells[1].contains("W 310x38.7"), "{cells:?}");
    let expected = W310.outline().unwrap().properties(7850.0).linear_mass;
    let shown = cells[2].trim_end_matches(" kg/m").replace(',', ".");
    let got: f64 = shown.parse().unwrap_or_else(|_| panic!("{cells:?}"));
    assert!((got - expected).abs() / expected < 0.01, "{got} vs {expected}");

    // Without a density nothing is guessed.
    let mut p = s.project().clone();
    p.materials.iter_mut().find(|m| m.name == steel).unwrap().physical.density = None;
    let mut s = Session::new(Document::new(p));
    let cells = beam_schedule_cells(&mut s);
    assert_eq!(cells[2], "", "{cells:?}");
}

fn run_err(s: &mut Session, name: &str, args: Value) -> cc_engine::EngineError {
    match s.execute(name, args) {
        Ok(v) => panic!("{name} should fail, gave {v}"),
        Err(e) => e,
    }
}

#[test]
fn create_get_set_delete() {
    let mut s = Session::default();
    let out = run(
        &mut s,
        "profile_create",
        json!({"material": "steel", "shape": "i", "name": "PS 300",
            "params": {"d": 0.3, "bf": 0.15, "tw": 0.008, "tf": 0.0125, "r": 0.0}}),
    );
    let id = out["id"].as_u64().unwrap();
    let got = run(&mut s, "profile_get", json!({"profile": "ps 300"}));
    assert_eq!(got["id"].as_u64(), Some(id));
    assert_eq!(got["shape"], "i");
    assert_eq!(got["material"], "steel");
    let area = got["properties"]["area"].as_f64().unwrap();
    assert!((area - (2.0 * 0.15 * 0.0125 + 0.275 * 0.008)).abs() < 1e-9);

    let steps = s.doc.undo_depth();
    let err = run_err(&mut s, "profile_set", json!({"profile": id, "params": {"tw": 0.5}}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    assert_eq!(s.doc.undo_depth(), steps, "a refused change leaves no undo step");
    let err = run_err(&mut s, "profile_set", json!({"profile": id, "params": {"zz": 0.5}}));
    assert!(err.message.contains("tw"), "{}", err.message);
    run(&mut s, "profile_set", json!({"profile": id, "params": {"tw": 0.01}}));
    assert_eq!(run(&mut s, "profile_get", json!({"profile": id}))["params"]["tw"], 0.01);

    run(&mut s, "undo", json!({}));
    assert_eq!(run(&mut s, "profile_get", json!({"profile": id}))["params"]["tw"], 0.008);
    let list = run(&mut s, "profiles_list", json!({"shape": "I"}));
    let row = list.as_array().unwrap().iter().find(|p| p["id"].as_u64() == Some(id)).unwrap();
    assert!((row["linear_mass_steel"].as_f64().unwrap() - area * 7850.0).abs() < 1e-9);
    assert!(list.as_array().unwrap().iter().all(|p| p["shape"] == "i"));
    run(&mut s, "profile_delete", json!({"profile": id}));
    let list = run(&mut s, "profiles_list", json!({}));
    assert!(list.as_array().unwrap().iter().all(|p| p["id"].as_u64() != Some(id)));
}

#[test]
fn create_checks_its_arguments() {
    let mut s = Session::default();
    let err = run_err(&mut s, "profile_create", json!({"material": "gold", "shape": "i"}));
    assert!(err.message.contains("steel"), "{}", err.message);
    let err = run_err(&mut s, "profile_create", json!({"material": "steel", "shape": "zz"}));
    assert!(err.message.contains("cold_lipped_u"), "{}", err.message);
    // Defaults, a name of its own, case-insensitive codes.
    let a = run(&mut s, "profile_create", json!({"material": "Steel", "shape": "RECT"}));
    let b = run(&mut s, "profile_create", json!({"material": "steel", "shape": "rect"}));
    assert_ne!(a["name"], b["name"]);
    let err = run_err(&mut s, "profile_create", json!({"material": "steel", "shape": "rect", "name": a["name"]}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    let dup = run(&mut s, "profile_duplicate", json!({"profile": a["id"]}));
    assert_ne!(dup["id"], a["id"]);
    assert_ne!(dup["name"], a["name"]);
}

#[test]
fn sketch_profile_from_curves() {
    let mut s = Session::default();
    let out = run(
        &mut s,
        "profile_create",
        json!({"material": "steel", "name": "Box",
            "curves": [{"kind": "rectangle", "corner": [-0.1, -0.15], "opposite": [0.1, 0.15]}],
            "openings": [[{"kind": "rectangle", "corner": [-0.09, -0.14], "opposite": [0.09, 0.14]}]],
            "sketch_params": [{"name": "t", "value": 0.01}]}),
    );
    let got = run(&mut s, "profile_get", json!({"profile": out["id"]}));
    assert_eq!(got["shape"], "sketch");
    assert_eq!(got["outline"]["holes"].as_array().unwrap().len(), 1);
    assert_eq!(got["params"]["t"], 0.01);
    let area = got["properties"]["area"].as_f64().unwrap();
    assert!((area - (0.2 * 0.3 - 0.18 * 0.28)).abs() < 1e-9, "{area}");
    // The sketch is patched: no openings leaves the solid rectangle.
    run(&mut s, "profile_set", json!({"profile": out["id"], "sketch": {"openings": []}}));
    let got = run(&mut s, "profile_get", json!({"profile": out["id"]}));
    assert!(got["outline"]["holes"].as_array().unwrap().is_empty());
}

#[test]
fn deleting_a_used_profile_needs_a_replacement() {
    let mut s = Session::default();
    let used = run(&mut s, "profiles_list", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["users"].as_u64().unwrap() > 0)
        .unwrap()
        .clone();
    let other =
        run(&mut s, "profile_create", json!({"material": "concrete", "shape": "rect", "name": "R"}))["id"].clone();
    let err = run_err(&mut s, "profile_delete", json!({"profile": used["id"]}));
    assert!(err.message.contains("replace_with"), "{}", err.message);
    run(&mut s, "profile_delete", json!({"profile": used["id"], "replace_with": other}));
    let r = run(&mut s, "profile_get", json!({"profile": "R"}));
    assert!(!r["used_by"].as_array().unwrap().is_empty());
}

/// The legacy box of a column or beam type: (profile, width, height).
fn type_box(s: &Session, id: ElementTypeId) -> (Option<ProfileId>, f64, f64) {
    match &s.project().types[&id].data {
        TypeData::Beam { profile, width, height, .. } => (*profile, *width, *height),
        TypeData::Column { profile, section, .. } => {
            let (w, h) = match *section {
                ColumnSection::Rectangular { width, depth } | ColumnSection::Profile { width, depth, .. } => {
                    (width, depth)
                }
                ColumnSection::Round { diameter } | ColumnSection::Pipe { diameter, .. } => (diameter, diameter),
            };
            (*profile, w, h)
        }
        _ => panic!("not a column or beam type"),
    }
}

#[test]
fn types_follow_an_edited_profile() {
    let mut s = Session::default();
    let list = run(&mut s, "profiles_list", json!({"shape": "rect"}));
    let used = list.as_array().unwrap().iter().find(|p| p["users"].as_u64().unwrap() > 0).unwrap().clone();
    let pid = ProfileId(used["id"].as_u64().unwrap());
    let users = s.project().profile_users(pid);
    assert!(!users.is_empty());
    run(&mut s, "profile_set", json!({"profile": pid.0, "params": {"b": 0.33, "h": 0.55}}));
    for t in users {
        let (profile, w, h) = type_box(&s, t);
        assert_eq!(profile, Some(pid), "the type still points to the edited profile");
        assert!((w - 0.33).abs() < 1e-9 && (h - 0.55).abs() < 1e-9, "{w} x {h}");
    }
}

#[test]
fn undoing_a_replacing_delete_restores_the_profile_and_its_users() {
    let mut s = Session::default();
    let list = run(&mut s, "profiles_list", json!({"shape": "rect"}));
    let used = list.as_array().unwrap().iter().find(|p| p["users"].as_u64().unwrap() > 0).unwrap().clone();
    let pid = ProfileId(used["id"].as_u64().unwrap());
    let users = s.project().profile_users(pid);
    let before: Vec<_> = users.iter().map(|t| type_box(&s, *t)).collect();
    let other = run(
        &mut s,
        "profile_create",
        json!({"material": "concrete", "shape": "rect", "name": "R", "params": {"b": 0.17, "h": 0.71}}),
    )["id"]
        .clone();
    let steps = s.doc.undo_depth();
    run(&mut s, "profile_delete", json!({"profile": pid.0, "replace_with": other}));
    assert_eq!(s.doc.undo_depth(), steps + 1, "one undo step");
    for t in &users {
        let (_, w, h) = type_box(&s, *t);
        assert!((w - 0.17).abs() < 1e-9 && (h - 0.71).abs() < 1e-9, "{w} x {h}");
    }
    run(&mut s, "undo", json!({}));
    assert!(s.project().profile(pid).is_some(), "the profile is back");
    let after: Vec<_> = users.iter().map(|t| type_box(&s, *t)).collect();
    assert_eq!(after, before, "the users point to it again with their box");
}

#[test]
fn hostile_profile_set_is_refused() {
    let mut s = Session::default();
    let rect = run(&mut s, "profile_create", json!({"material": "steel", "shape": "rect", "name": "A"}))["id"].clone();
    let steps = s.doc.undo_depth();
    let err = run_err(&mut s, "profile_set", json!({"profile": rect, "sketch": {"openings": []}}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);

    let sk = run(
        &mut s,
        "profile_create",
        json!({"material": "steel", "name": "Sk",
            "curves": [{"kind": "rectangle", "corner": [-0.1, -0.1], "opposite": [0.1, 0.1]}]}),
    )["id"]
        .clone();
    let steps = steps + 1;
    assert_eq!(s.doc.undo_depth(), steps);
    for patch in [
        json!({"outline": 5}),
        json!({"params": [{"name": "1bad", "value": 0.1}]}),
        json!({"params": [{"name": "a", "value": 0.1}, {"name": "a", "value": 0.2}]}),
        json!({"params": [{"name": "a", "value": 0.1, "kind": "Text"}]}),
    ] {
        let err = run_err(&mut s, "profile_set", json!({"profile": sk, "sketch": patch}));
        assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments, "{patch}: {}", err.message);
        assert_eq!(s.doc.undo_depth(), steps, "{patch} leaves no undo step");
    }

    let err = run_err(&mut s, "profile_set", json!({"profile": sk, "name": "a"}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    run(&mut s, "profile_set", json!({"profile": sk, "name": "SK"}));
    assert_eq!(run(&mut s, "profile_get", json!({"profile": sk}))["name"], "SK");
}

#[test]
fn a_new_material_leaves_the_catalog_entry() {
    let mut s = Session::default();
    let id = run(&mut s, "profile_create", json!({"material": "steel", "shape": "rect", "name": "A"}))["id"].clone();
    let mut p = s.project().clone();
    p.profiles.get_mut(&ProfileId(id.as_u64().unwrap())).unwrap().catalog =
        Some(cc_core::structural_profile::CatalogRef { series: "X".into(), designation: "1".into() });
    let mut s = Session::new(Document::new(p));
    run(&mut s, "profile_set", json!({"profile": id, "material": "timber"}));
    assert_eq!(run(&mut s, "profile_get", json!({"profile": id}))["catalog"], Value::Null);
}

#[test]
fn import_from_catalog_and_make_types() {
    let mut s = Session::default();
    let series = run(&mut s, "profile_catalog", json!({}));
    assert!(series.as_array().unwrap().iter().any(|x| x["code"] == "IPE"));
    let rows = run(&mut s, "profile_catalog", json!({"series": "IPE", "search": "20"}));
    assert!(rows["rows"].as_array().unwrap().iter().any(|r| r["designation"] == "IPE 200"));

    let out = run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200", "IPE 270", "999"]}));
    assert_eq!(out["imported"].as_array().unwrap().len(), 2);
    assert_eq!(out["unknown"][0], "999");
    let again = run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    assert_eq!(again["kept"][0], "IPE 200");

    let t = run(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "beam"}));
    let ty = run(&mut s, "type_get", json!({"type": t["id"]}));
    assert_eq!(ty["params"]["profile"], "IPE 200");
    assert!((ty["params"]["height"].as_f64().unwrap() - 0.2).abs() < 1e-9, "bbox cached");

    let beam =
        run(&mut s, "beam_create", json!({"type": t["id"], "start": [0.0, 0.0], "end": [6.0, 0.0]}))["id"].clone();
    let e = run(&mut s, "element_get", json!({"id": beam}));
    assert!((e["derived"]["section"]["area"].as_f64().unwrap() - 0.00285).abs() / 0.00285 < 0.02);

    run(&mut s, "profile_import", json!({"series": "HEB", "designations": ["HEB 200"]}));
    run(&mut s, "type_set", json!({"type": t["id"], "params": {"profile": "HEB 200"}}));
    let ty = run(&mut s, "type_get", json!({"type": t["id"]}));
    assert!((ty["params"]["width"].as_f64().unwrap() - 0.2).abs() < 1e-9);

    // `analytical_export` writes `analytical_model`'s document to a file.
    let a = run(&mut s, "analytical_model", json!({}));
    let text = a.to_string();
    assert!(text.contains("HEB 200"), "members carry their section");
    let member = a["members"].as_array().unwrap().iter().find(|m| m["element"] == beam).unwrap();
    let sec = &member["section"];
    assert_eq!(sec["profile"], "HEB 200");
    for k in ["area", "ix", "iy", "j"] {
        assert!(sec[k].as_f64().unwrap() > 0.0, "{k}: {sec}");
    }
}

#[test]
fn catalog_import_options_and_undo() {
    let mut s = Session::default();
    let err = run_err(&mut s, "profile_catalog", json!({"series": "nope"}));
    assert_eq!(err.code, cc_engine::ErrorCode::NotFound);
    let ipe = run(&mut s, "profile_catalog", json!({"series": "ipe"}));
    assert_eq!(ipe["code"], "IPE");
    let row = ipe["rows"].as_array().unwrap().iter().find(|r| r["designation"] == "IPE 200").unwrap().clone();
    assert_eq!(row["name"], "IPE 200");
    assert!((row["params"]["d"].as_f64().unwrap() - 0.2).abs() < 1e-9, "{row}");
    let all = run(&mut s, "profile_catalog", json!({}));
    let entry = all.as_array().unwrap().iter().find(|x| x["code"] == "IPE").unwrap().clone();
    assert_eq!(entry["material"], "steel");
    assert_eq!(entry["shape"], "i");
    assert_eq!(entry["count"].as_u64().unwrap() as usize, ipe["rows"].as_array().unwrap().len());

    let steps = s.doc.undo_depth();
    let out =
        run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200", "IPE 220", "IPE 200"]}));
    assert_eq!(out["imported"].as_array().unwrap().len(), 2, "{out}");
    assert_eq!(out["kept"][0], "IPE 200", "a repeated designation is kept");
    assert_eq!(s.doc.undo_depth(), steps + 1, "one undo step");
    let got = run(&mut s, "profile_get", json!({"profile": "IPE 200"}));
    assert_eq!(got["catalog"]["series"], "IPE");
    assert_eq!(got["catalog"]["designation"], "IPE 200");
    // Nothing new: no undo step.
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200", "nope"]}));
    assert_eq!(s.doc.undo_depth(), steps + 1);
    let renamed =
        run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"], "on_clash": "rename"}));
    assert_eq!(renamed["imported"][0]["name"], "IPE 200 2");
    let err = run_err(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"], "on_clash": "x"}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    run(&mut s, "undo", json!({}));
    run(&mut s, "undo", json!({}));
    for id in out["imported"].as_array().unwrap() {
        assert!(s.project().profile(ProfileId(id["id"].as_u64().unwrap())).is_none());
    }
}

/// Import the first row of a catalog series; the new profile's name.
fn import_first(s: &mut Session, series: &str) -> Value {
    let first = run(s, "profile_catalog", json!({"series": series}))["rows"][0]["designation"].clone();
    run(s, "profile_import", json!({"series": series, "designations": [first]}))["imported"][0]["name"].clone()
}

#[test]
fn profile_types_take_a_material_of_their_class_and_a_free_name() {
    let mut s = Session::default();
    let name = import_first(&mut s, "Glulam");
    let a = run(&mut s, "profile_type_create", json!({"profile": name, "category": "column"}));
    let b = run(&mut s, "profile_type_create", json!({"profile": name, "category": "column"}));
    assert_eq!(a["name"], name);
    assert_ne!(a["name"], b["name"]);
    let ty = run(&mut s, "type_get", json!({"type": a["id"]}));
    let material = ty["params"]["material"].as_str().unwrap().to_string();
    let class = s.project().material(&material).unwrap().identity.class;
    assert_eq!(class, cc_core::MaterialClass::Wood, "{material}");
    let named = run(&mut s, "profile_type_create", json!({"profile": name, "category": "beam", "name": "Viga"}));
    assert_eq!(named["name"], "Viga");
    let err = run_err(&mut s, "profile_type_create", json!({"profile": name, "category": "beam", "name": "Viga"}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    let err = run_err(&mut s, "profile_type_create", json!({"profile": name, "category": "wall"}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    let err = run_err(&mut s, "profile_type_create", json!({"profile": "nope", "category": "beam"}));
    assert_eq!(err.code, cc_engine::ErrorCode::NotFound);

    // A project without beam types: a clear error.
    let mut p = s.project().clone();
    p.types.retain(|_, t| !matches!(t.data, TypeData::Beam { .. }));
    let mut s = Session::new(Document::new(p));
    let err = run_err(&mut s, "profile_type_create", json!({"profile": name, "category": "beam"}));
    assert!(err.message.contains("beam"), "{}", err.message);
}

#[test]
fn type_set_takes_a_profile_but_never_none() {
    let mut s = Session::default();
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let t = run(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "beam"}))["id"].clone();
    let steps = s.doc.undo_depth();
    let err = run_err(&mut s, "type_set", json!({"type": t, "params": {"profile": "nope"}}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    assert_eq!(s.doc.undo_depth(), steps);
    run(&mut s, "type_set", json!({"type": t, "params": {"profile_rotation_deg": 90.0, "profile_mirror": true}}));
    let ty = run(&mut s, "type_get", json!({"type": t}));
    assert_eq!(ty["params"]["profile_rotation_deg"], 90.0);
    assert_eq!(ty["params"]["profile_mirror"], true);
    assert!((ty["params"]["width"].as_f64().unwrap() - 0.2).abs() < 1e-9, "turned: {ty}");
    run(&mut s, "type_set", json!({"type": t, "params": {"profile_rotation_deg": 0.0}}));
    // Types always point to a profile: null is refused, without an undo step.
    let steps = s.doc.undo_depth();
    let err = run_err(&mut s, "type_set", json!({"type": t, "params": {"profile": null}}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    assert!(err.message.contains("must point to a profile"), "{}", err.message);
    assert_eq!(s.doc.undo_depth(), steps);
    assert_eq!(run(&mut s, "type_get", json!({"type": t}))["params"]["profile"], "IPE 200");
    // By id too.
    let pid = run(&mut s, "profile_get", json!({"profile": "IPE 200"}))["id"].clone();
    run(&mut s, "type_set", json!({"type": t, "params": {"profile": pid}}));
    assert_eq!(run(&mut s, "type_get", json!({"type": t}))["params"]["profile"], "IPE 200");
}

#[test]
fn element_section_mass_needs_a_density() {
    let mut s = Session::default();
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let t = run(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "column"}))["id"].clone();
    let col = run(&mut s, "column_create", json!({"type": t, "position": [0.0, 0.0]}))["id"].clone();
    let e = run(&mut s, "element_get", json!({"id": col}));
    let sec = &e["derived"]["section"];
    assert_eq!(sec["profile"], "IPE 200");
    let area = sec["area"].as_f64().unwrap();
    assert!((sec["linear_mass"].as_f64().unwrap() - area * 7850.0).abs() < 1e-6, "{sec}");
    for k in ["ix", "iy", "wx", "wy"] {
        assert!(sec[k].as_f64().unwrap() > 0.0, "{k}: {sec}");
    }
    let material = run(&mut s, "type_get", json!({"type": t}))["params"]["material"].as_str().unwrap().to_string();
    let mut p = s.project().clone();
    p.materials.iter_mut().find(|m| m.name == material).unwrap().physical.density = None;
    let mut s = Session::new(Document::new(p));
    let e = run(&mut s, "element_get", json!({"id": col}));
    assert!(e["derived"]["section"]["linear_mass"].is_null(), "{e}");
}

#[test]
fn a_type_without_a_profile_reads_its_legacy_section() {
    // As IFC import or a raw file may leave it.
    let (mut s, tid) = with_ipe_column_type();
    let mut p = s.project().clone();
    let TypeData::Column { profile, .. } = &mut p.types.get_mut(&tid).unwrap().data else { panic!("not a column") };
    *profile = None;
    s = Session::new(Document::new(p));
    let ty = run(&mut s, "type_get", json!({"type": tid.0}));
    assert!(ty["params"]["profile"].is_null(), "{ty}");
    let col = run(&mut s, "column_create", json!({"type": tid.0, "position": [0.0, 0.0]}))["id"].clone();
    let e = run(&mut s, "element_get", json!({"id": col}));
    let area = e["derived"]["section"]["area"].as_f64().unwrap();
    let legacy = 2.0 * 0.1 * 0.0085 + (0.2 - 0.017) * 0.0056;
    assert!((area - legacy).abs() < 1e-9, "{area} vs {legacy}");
}

#[test]
fn truss_posts_default_to_the_material_catalog_column_type() {
    let mut s = Session::default();
    let out = run(&mut s, "truss_create", json!({"start": [0, 0], "end": [6, 0], "height": 1.0, "panels": 4}));
    let posts: Vec<u64> = out["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap())
        .filter(|i| matches!(s.project().element(ElementId(*i)).unwrap().kind, ElementKind::Column(_)))
        .collect();
    assert_eq!(posts.len(), 5);
    for post in posts {
        let e = s.project().element(ElementId(post)).unwrap();
        let t = s.project().element_type(e.type_id).unwrap();
        assert_eq!((t.name.as_str(), t.category()), ("Ue 100x50x17x2,00", cc_core::Category::Column));
    }
    // A given post type wins.
    let column = run(&mut s, "profile_type_create", json!({"profile": "Ue 150x60x20x2,00", "category": "column"}));
    let out = run(
        &mut s,
        "truss_create",
        json!({"start": [0, 5], "end": [6, 5], "height": 1.0, "panels": 4, "post_type": column["id"]}),
    );
    let host = s.project().element(ElementId(out["id"].as_u64().unwrap())).unwrap();
    let ElementKind::Truss(t) = &host.kind else { panic!("a truss") };
    assert_eq!(t.post_type.0, column["id"].as_u64().unwrap());
}

#[test]
fn analytical_members_carry_the_type_box() {
    let mut s = Session::default();
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let t = run(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "beam"}));
    let beam =
        run(&mut s, "beam_create", json!({"type": t["id"], "start": [0.0, 0.0], "end": [6.0, 0.0]}))["id"].clone();
    let a = run(&mut s, "analytical_model", json!({}));
    let member = a["members"].as_array().unwrap().iter().find(|m| m["element"] == beam).unwrap();
    let sec = &member["section"];
    assert!((sec["width"].as_f64().unwrap() - 0.1).abs() < 1e-9, "{sec}");
    assert!((sec["height"].as_f64().unwrap() - 0.2).abs() < 1e-9, "{sec}");
}

#[test]
fn profile_command_inputs_are_bounded() {
    let mut s = Session::default();
    let long = "x".repeat(cc_core::structural_profile::MAX_NAME + 1);
    let err = run_err(&mut s, "profile_catalog", json!({"series": "IPE", "search": long}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    let err = run_err(&mut s, "profile_import", json!({"series": "IPE", "designations": [long]}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let err = run_err(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "beam", "name": long}));
    assert_eq!(err.code, cc_engine::ErrorCode::InvalidArguments);
}

/// The path Type Properties takes: the template's beam type pointed to an
/// imported IPE 200 by id (`type_set`); a beam of it builds a body of the
/// I's area along its length.
#[test]
fn a_template_beam_type_switched_to_ipe_200_builds_its_body() {
    let mut s = Session::default();
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let pid = run(&mut s, "profile_get", json!({"profile": "IPE 200"}))["id"].clone();
    let t = s.project().default_type(cc_core::Category::Beam).unwrap();
    run(&mut s, "type_set", json!({"type": t.0, "params": {"profile": pid}}));
    assert_eq!(run(&mut s, "type_get", json!({"type": t.0}))["params"]["profile"], "IPE 200");
    let beam = run(&mut s, "beam_create", json!({"type": t.0, "start": [0.0, 0.0], "end": [4.0, 0.0]}))["id"].clone();
    let id = ElementId(beam.as_u64().unwrap());
    let geom = s.geometry();
    let faces: usize = geom.body_of(id).map(|b| b.faces.len()).sum();
    assert!(faces > 0, "a body");
    let volume: f64 = geom.body_of(id).map(cc_geom::Solid::volume).sum();
    assert!((volume - 0.00285 * 4.0).abs() / (0.00285 * 4.0) < 0.03, "{volume}");
}

/// A concrete column type on a new profile of `shape` (`params` its values).
fn concrete_column_type(s: &mut Session, shape: &str, params: Value) -> Value {
    let p = run(s, "profile_create", json!({"material": "concrete", "shape": shape, "params": params}));
    run(s, "profile_type_create", json!({"profile": p["id"], "category": "column"}))["id"].clone()
}

#[test]
fn round_profile_columns_get_round_rebar() {
    for (shape, params) in [("round", json!({"d": 0.4})), ("round_tube", json!({"d": 0.4, "t": 0.1}))] {
        let mut s = Session::default();
        let t = concrete_column_type(&mut s, shape, params);
        let col = run(&mut s, "column_create", json!({"type": t, "position": [0.0, 0.0], "height": 3.0}))["id"].clone();
        let out = run(&mut s, "rebar_typical", json!({"elements": [col]}));
        let sets: Vec<Value> =
            out["created"].as_array().unwrap().iter().map(|i| run(&mut s, "element_get", json!({"id": i}))).collect();
        let bars = sets.iter().find(|x| x["params"]["layout"]["Longitudinal"].is_object()).unwrap();
        assert_eq!(bars["params"]["layout"]["Longitudinal"]["count"], json!(6), "{shape}: {bars}");
        // Bars on a circle and round ties.
        let geom = s.geometry();
        let id = |x: &Value| ElementId(x["id"].as_u64().unwrap());
        let ties = sets.iter().find(|x| x["params"]["layout"]["Stirrups"].is_object()).unwrap();
        let tie = &geom.rebar[&id(ties)].bars[0];
        assert!(tie.points.len() > 4, "{shape}: a round tie, {} points", tie.points.len());
        let r: Vec<f64> = geom.rebar[&id(bars)].bars.iter().map(|b| b.points[0].truncate().length()).collect();
        assert!(r.iter().all(|v| (v - r[0]).abs() < 1e-9), "{shape}: on a circle {r:?}");
    }
}

#[test]
fn schedules_tags_and_legends_read_profile_made_sections() {
    let mut s = Session::default();
    let round = concrete_column_type(&mut s, "round", json!({"d": 0.3}));
    let col = run(&mut s, "column_create", json!({"type": round, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
    let p = s.project().clone();
    let geom = s.geometry().clone();
    let e = p.element(ElementId(col)).unwrap();
    let section = cc_views::field_value(&p, &geom, e, cc_core::ScheduleField::Section).unwrap().0;
    assert!(section.starts_with('Ø'), "{section}");

    // A beam on an I profile is drawn in legends as an I, not its box.
    run(&mut s, "profile_import", json!({"series": "IPE", "designations": ["IPE 200"]}));
    let t =
        run(&mut s, "profile_type_create", json!({"profile": "IPE 200", "category": "beam"}))["id"].as_u64().unwrap();
    let d = cc_views::legend(s.project(), &[ElementTypeId(t)], 20);
    let cut = d.prims.iter().filter(|p| matches!(p, Prim::Line { style: LineStyle::Cut, .. })).count();
    assert!(cut > 4, "an I outline, not a box: {cut} cut lines");
}

#[test]
fn resizing_a_turned_type_keeps_the_typed_box() {
    let mut s = Session::default();
    let p =
        run(&mut s, "profile_create", json!({"material": "concrete", "shape": "rect", "params": {"b": 0.2, "h": 0.5}}));
    let t = run(&mut s, "profile_type_create", json!({"profile": p["id"], "category": "beam"}))["id"].clone();
    run(&mut s, "type_set", json!({"type": t, "params": {"profile_rotation_deg": 90.0, "profile_mirror": true}}));
    let ty = run(&mut s, "type_get", json!({"type": t}));
    assert!((ty["params"]["width"].as_f64().unwrap() - 0.5).abs() < 1e-9, "turned: {ty}");
    // A new width re-points the type to a profile of the typed box, as is.
    run(&mut s, "type_set", json!({"type": t, "params": {"width": 0.6}}));
    let ty = run(&mut s, "type_get", json!({"type": t}));
    let (w, h) = (ty["params"]["width"].as_f64().unwrap(), ty["params"]["height"].as_f64().unwrap());
    assert!((w - 0.6).abs() < 1e-9 && (h - 0.2).abs() < 1e-9, "{w} x {h}: {ty}");
    assert!(ty["params"]["profile_rotation_deg"].as_f64().unwrap_or(0.0) == 0.0, "{ty}");
    assert!(!ty["params"]["profile_mirror"].as_bool().unwrap_or(false), "{ty}");
    assert_ne!(ty["params"]["profile"], p["name"], "{ty}");
    // The section drawn is the typed box too.
    let data = &s.project().element_type(ElementTypeId(t.as_u64().unwrap())).unwrap().data;
    let (lo, hi) = s.project().type_section(data).unwrap().outline.bbox();
    assert!(((hi - lo) - glam::DVec2::new(0.6, 0.2)).abs().max_element() < 1e-9, "{:?}", hi - lo);
}

/// A round column's volume is the circle's, not its chorded outline's: IFC
/// export writes it as a quantity next to an exact `IfcCircleProfileDef`.
#[test]
fn round_columns_have_their_exact_volume() {
    let mut s = Session::default();
    for (name, shape, params) in
        [("R30", "round", json!({"d": 0.3})), ("T200", "round_tube", json!({"d": 0.2, "t": 0.008}))]
    {
        run(&mut s, "profile_create", json!({"material": "steel", "shape": shape, "name": name, "params": params}));
        let t = run(&mut s, "profile_type_create", json!({"profile": name, "category": "column"}))["id"].clone();
        let col = run(&mut s, "column_create", json!({"type": t, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
        let id = ElementId(col);
        let e = s.project().element(id).unwrap();
        let ElementKind::Column(c) = &e.kind else { panic!() };
        let (base, top) = s.project().column_span(e, c);
        let disk = |d: f64| std::f64::consts::PI * d * d / 4.0;
        let area = if shape == "round" { disk(0.3) } else { disk(0.2) - disk(0.184) };
        let volume = s.geometry().volume(id);
        let want = area * (top - base);
        assert!((volume - want).abs() / want < 1e-5, "{name}: {volume} vs {want}");
    }
}

/// A free beam of a round profile sweeps the circle's exact area along it.
#[test]
fn round_beams_have_their_exact_volume() {
    let mut s = Session::default();
    run(&mut s, "profile_create", json!({"material": "steel", "shape": "round", "name": "R30", "params": {"d": 0.3}}));
    let t = run(&mut s, "profile_type_create", json!({"profile": "R30", "category": "beam"}))["id"].clone();
    let id = run(&mut s, "beam_create", json!({"type": t, "start": [0.0, 0.0], "end": [4.0, 0.0]}))["id"].as_u64();
    let id = ElementId(id.unwrap());
    let want = std::f64::consts::PI * 0.0225 * 4.0;
    let volume = s.geometry().volume(id);
    assert!((volume - want).abs() / want < 1e-5, "{volume} vs {want}");
}

#[test]
fn brazil_profile_series_and_material_library_import() {
    let mut s = Session::default();
    let res = run(&mut s, "material_library_import", json!({"library": "brazil"}));
    let imported = res["imported"].as_array().unwrap();
    assert!(imported.len() >= 20, "imported steels, rebars and concretes: {}", imported.len());
    assert!(s.project().material("Steel MR250 (NBR 7007)").is_some());
    assert!(s.project().material("Concrete C30 (NBR 6118)").is_some());

    // Calling again keeps existing materials untouched.
    let res2 = run(&mut s, "material_library_import", json!({"library": "brazil", "kind": "steel"}));
    assert!(res2["imported"].as_array().unwrap().is_empty());
    assert!(!res2["kept"].as_array().unwrap().is_empty());

    // Import CS, CVS and BR profiles and create types with MR250 steel.
    let cs = run(&mut s, "profile_import", json!({"series": "CS", "designations": ["CS 250x43"]}));
    assert_eq!(cs["imported"].as_array().unwrap().len(), 1);
    let cvs = run(&mut s, "profile_import", json!({"series": "CVS", "designations": ["CVS 250x40"]}));
    assert_eq!(cvs["imported"].as_array().unwrap().len(), 1);
    let br = run(&mut s, "profile_import", json!({"series": "BR", "designations": ["BR 5/8\""]}));
    assert_eq!(br["imported"].as_array().unwrap().len(), 1);

    let col_ty = run(
        &mut s,
        "profile_type_create",
        json!({"profile": "CS 250x43", "category": "column", "material": "Steel MR250 (NBR 7007)"}),
    );
    let col = run(&mut s, "column_create", json!({"type": col_ty["id"], "position": [0.0, 0.0], "height": 4.0}));
    assert!(s.geometry().volume(ElementId(col["id"].as_u64().unwrap())) > 0.0);
}

#[test]
fn steel_connection_macros_create_and_auto_recognize_all_joint_kinds() {
    let mut s = Session::default();
    run(&mut s, "profile_import", json!({"series": "CS", "designations": ["CS 250x43"]}));
    run(&mut s, "profile_import", json!({"series": "CVS", "designations": ["CVS 250x40"]}));
    let col_ty = run(&mut s, "profile_type_create", json!({"profile": "CS 250x43", "category": "column"}))["id"]
        .as_u64()
        .unwrap();
    let beam_ty = run(&mut s, "profile_type_create", json!({"profile": "CVS 250x40", "category": "beam"}))["id"]
        .as_u64()
        .unwrap();

    // 1. Catalog query lists all 6 parametric macros.
    let catalog = run(&mut s, "steel_connection_macros", json!({}));
    let macros = catalog["macros"].as_array().unwrap();
    assert_eq!(macros.len(), 6);

    // 2. Build a portal frame with columns, primary beams, a spliced beam, a secondary beam, X-bracing and a truss.
    let c1 = run(&mut s, "column_create", json!({"type": col_ty, "position": [0.0, 0.0]}))["id"].as_u64().unwrap();
    let c2 = run(&mut s, "column_create", json!({"type": col_ty, "position": [0.0, 6.0]}))["id"].as_u64().unwrap();
    let c3 = run(&mut s, "column_create", json!({"type": col_ty, "position": [6.0, 0.0]}))["id"].as_u64().unwrap();

    // Beam along +Y (flange of column c1 & c2 -> end_plate)
    let b_frame = run(&mut s, "beam_create", json!({"type": beam_ty, "start": [0.0, 0.0], "end": [0.0, 6.0]}))["id"]
        .as_u64()
        .unwrap();

    // Beam along +X into column c1 web -> single_plate, spliced at x = 3.0 -> splice
    let b_web1 = run(&mut s, "beam_create", json!({"type": beam_ty, "start": [0.0, 0.0], "end": [3.0, 0.0]}))["id"]
        .as_u64()
        .unwrap();
    let b_web2 = run(&mut s, "beam_create", json!({"type": beam_ty, "start": [3.0, 0.0], "end": [6.0, 0.0]}))["id"]
        .as_u64()
        .unwrap();

    // Secondary beam framing into b_frame at mid-span (0.0, 3.0)
    let b_sec = run(&mut s, "beam_create", json!({"type": beam_ty, "start": [0.0, 3.0], "end": [3.0, 3.0]}))["id"]
        .as_u64()
        .unwrap();

    // Explicit creation of each macro kind and verification of 3D scripted geometry:
    let bp = run(
        &mut s,
        "steel_connection_create",
        json!({"elements": [c1], "kind": "base_plate", "base_mode": "fixed", "stiffeners": true, "anchors": 4}),
    );
    let bp_id = ElementId(bp["id"].as_u64().unwrap());
    assert_eq!(bp["kind"], "base_plate");
    assert!(s.geometry().volume(bp_id) > 0.0, "base_plate should produce 3D solid pieces");

    let ep = run(
        &mut s,
        "steel_connection_create",
        json!({"elements": [c1, b_frame], "kind": "end_plate", "stiffeners": true}),
    );
    let ep_id = ElementId(ep["id"].as_u64().unwrap());
    assert_eq!(ep["kind"], "end_plate");
    assert!(s.geometry().volume(ep_id) > 0.0, "end_plate should produce 3D solid pieces");

    let sp = run(&mut s, "steel_connection_create", json!({"elements": [c1, b_web1]}));
    let sp_id = ElementId(sp["id"].as_u64().unwrap());
    assert_eq!(sp["kind"], "single_plate", "beam into column web auto-detects single_plate");
    assert!(s.geometry().volume(sp_id) > 0.0, "single_plate should produce 3D solid pieces");

    let da = run(&mut s, "steel_connection_create", json!({"elements": [b_frame, b_sec], "kind": "double_angle"}));
    let da_id = ElementId(da["id"].as_u64().unwrap());
    assert_eq!(da["kind"], "double_angle");
    assert!(s.geometry().volume(da_id) > 0.0, "double_angle should produce 3D solid pieces");

    let spl = run(&mut s, "steel_connection_create", json!({"elements": [b_web1, b_web2]}));
    let spl_id = ElementId(spl["id"].as_u64().unwrap());
    assert_eq!(spl["kind"], "splice", "collinear beams auto-detect splice");
    assert!(s.geometry().volume(spl_id) > 0.0, "splice should produce 3D solid pieces");

    // Add X-bracing between c1 and c2, and a truss, then run steel_connections_auto.
    let braces = run(&mut s, "braces_between_columns", json!({"columns": [c1, c2], "pattern": "x", "type": beam_ty}));
    let brace_ids: Vec<u64> = braces["created"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let gp = run(&mut s, "steel_connection_create", json!({"elements": brace_ids}));
    let gp_id = ElementId(gp["id"].as_u64().unwrap());
    assert_eq!(gp["kind"], "gusset_plate", "crossing braces auto-detect gusset_plate");
    assert!(s.geometry().volume(gp_id) > 0.0, "gusset_plate should produce 3D solid pieces");

    let _truss = run(
        &mut s,
        "truss_create",
        json!({"start": [0.0, 8.0], "end": [8.0, 8.0], "height": 1.2, "panels": 4, "chord_type": beam_ty}),
    );

    let auto1 =
        run(&mut s, "steel_connections_auto", json!({"elements": [c1, c2, c3, b_frame, b_web1, b_web2, b_sec]}));
    assert!(
        auto1["created"].as_array().unwrap().len() >= 3,
        "should place remaining base/end/shear connections: {auto1}"
    );
    assert!(auto1["skipped"].as_u64().unwrap() >= 1, "already placed connections are skipped: {auto1}");

    // Second run with the same scope creates 0 duplicates.
    let auto2 =
        run(&mut s, "steel_connections_auto", json!({"elements": [c1, c2, c3, b_frame, b_web1, b_web2, b_sec]}));
    assert_eq!(auto2["created"].as_array().unwrap().len(), 0);
    assert!(auto2["skipped"].as_u64().unwrap() >= 4);
}
