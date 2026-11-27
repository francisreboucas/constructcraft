//! Parametric trusses: `truss_create` makes a host that keeps its chords,
//! diagonals and posts in step with its parameters.

use cc_core::{ElementId, ElementKind};
use cc_engine::Session;
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

fn types_named(s: &Session, name: &str) -> usize {
    s.project().types.values().filter(|t| t.name == name).count()
}

fn trusses(s: &Session) -> usize {
    s.project().elements.values().filter(|e| matches!(e.kind, ElementKind::Truss(_))).count()
}

fn derived(s: &mut Session, host: u64) -> Value {
    run(s, "element_get", json!({"id": host}))["derived"].clone()
}

const SPAN: [[f64; 2]; 2] = [[0.0, 0.0], [6.0, 0.0]];

#[test]
fn steel_defaults_make_the_catalog_types_once() {
    let mut s = Session::default();
    let depth = s.doc.undo_depth();
    let out = run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "panels": 4}));
    assert_eq!(s.doc.undo_depth(), depth + 1, "one undo step");
    assert_eq!(out["members"].as_array().unwrap().len(), 11, "two chords, posts at both supports");
    assert_eq!(types_named(&s, "Ue 150x60x20x2,00"), 1, "the bottom chord's beam type");
    assert_eq!(types_named(&s, "Ue 100x50x17x2,00"), 2, "the web's beam type and the posts' column type");
    // The chords are channels on their sides, open toward the web: the top one is its own, mirrored type.
    let placed = |s: &Session, role: &str| {
        let p = s.project();
        let Some(ElementKind::Truss(t)) = p.element(ElementId(id(&out))).map(|e| &e.kind) else { panic!("a truss") };
        let ty = if role == "top" { t.top_chord_type } else { t.bottom_chord_type };
        match p.element_type(ty).map(|t| &t.data) {
            Some(cc_core::TypeData::Beam { profile_rotation_deg, profile_mirror, .. }) => {
                (*profile_rotation_deg, *profile_mirror)
            }
            _ => panic!("a beam type"),
        }
    };
    assert_eq!((placed(&s, "top"), placed(&s, "bottom")), ((90.0, true), (90.0, false)));
    let types = s.project().types.len();
    run(&mut s, "truss_create", json!({"start": [0, 5], "end": [6, 5], "height": 1.0, "material": "steel"}));
    assert_eq!(s.project().types.len(), types, "the same types again");
    assert_eq!(trusses(&s), 2);
    run(&mut s, "undo", json!({}));
    run(&mut s, "undo", json!({}));
    assert_eq!(types_named(&s, "Ue 150x60x20x2,00"), 0, "undone with the first truss");
}

#[test]
fn timber_defaults_and_concrete_refused() {
    let mut s = Session::default();
    let args = json!({"start": SPAN[0], "end": SPAN[1], "height": 1.2, "material": "timber", "shape": "gable",
        "pattern": "kingpost", "panels": 4, "copies": 3, "spacing": 0.6});
    let out = run(&mut s, "truss_create", args);
    assert_eq!(out["members"].as_array().unwrap().len(), 18, "6 per copy");
    assert_eq!(types_named(&s, "Peça 6x16"), 1);
    assert_eq!(types_named(&s, "Peça 6x12"), 2);
    let d = derived(&mut s, id(&out));
    assert_eq!(d["pattern_used"], json!("kingpost"));
    assert_eq!(d["members"]["verticals"].as_array().unwrap().len(), 3);
    let depth = s.doc.undo_depth();
    let concrete = json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "material": "concrete"});
    assert!(s.execute("truss_create", concrete).is_err());
    assert_eq!(s.doc.undo_depth(), depth);
    assert_eq!(trusses(&s), 1);
}

#[test]
fn bad_values_are_refused_before_anything_is_made() {
    let mut s = Session::default();
    for extra in [
        json!({"pattern": "k"}),
        json!({"shape": "dome"}),
        json!({"shape": "gable", "panels": 5}),
        json!({"panels": 1}),
        json!({"copies": 0}),
        json!({"spacing": 0.1}),
        json!({"material": "glass"}),
        json!({"post_type": "no such type"}),
    ] {
        let mut args = json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0});
        args.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert!(s.execute("truss_create", args.clone()).is_err(), "{args}");
    }
    assert!(s.execute("truss_create", json!({"start": [0, 0], "end": [0.1, 0], "height": 1.0})).is_err());
    assert_eq!(trusses(&s), 0);
    assert_eq!(types_named(&s, "Ue 150x60x20x2,00"), 0);
}

#[test]
fn element_set_of_panels_regenerates_in_one_step() {
    let mut s = Session::default();
    let host = id(&run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "panels": 4})));
    let before = derived(&mut s, host)["members"].clone();
    let depth = s.doc.undo_depth();
    run(&mut s, "element_set", json!({"id": host, "params": {"panels": 6}}));
    assert_eq!(s.doc.undo_depth(), depth + 1);
    let after = derived(&mut s, host)["members"].clone();
    assert_eq!(after["bottom_chords"], before["bottom_chords"], "one continuous chord, the same id");
    assert_eq!(after["diagonals"].as_array().unwrap().len(), 6);
    assert_eq!(after["diagonals"][0], before["diagonals"][0], "same id for the first panel");
    let params = &run(&mut s, "element_get", json!({"id": host}))["params"];
    assert_eq!((params["panels"].clone(), params["shape"].clone()), (json!(6), json!("Flat")));
    for bad in [json!({"members": []}), json!({"shape": "Gable", "panels": 5}), json!({"height": 0})] {
        assert!(s.execute("element_set", json!({"id": host, "params": bad})).is_err(), "{bad}");
    }
    assert_eq!(s.doc.undo_depth(), depth + 1);
    run(&mut s, "undo", json!({}));
    assert_eq!(derived(&mut s, host)["members"], before);
}

#[test]
fn deleting_a_member_detaches_it() {
    let mut s = Session::default();
    let host = id(&run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "panels": 4})));
    let post = derived(&mut s, host)["members"]["verticals"][0].as_u64().unwrap();
    run(&mut s, "elements_delete", json!({"ids": [post]}));
    assert_eq!(derived(&mut s, host)["detached"], json!(1));
    run(&mut s, "element_set", json!({"id": host, "params": {"height": 1.5}}));
    assert!(s.project().element(ElementId(post)).is_none(), "not made again");
    assert_eq!(derived(&mut s, host)["members"]["verticals"].as_array().unwrap().len(), 4);
}

#[test]
fn ifc_export_writes_the_members_and_not_the_truss() {
    let mut s = Session::default();
    let host = id(&run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "copies": 2})));
    let text = cc_ifc::export(s.project(), &cc_ifc::ExportOptions::default());
    let p = s.project();
    let count = |f: fn(&ElementKind) -> bool| p.elements.values().filter(|e| f(&e.kind)).count();
    assert_eq!(text.matches("IFCCOLUMN(").count(), count(|k| matches!(k, ElementKind::Column(_))));
    assert_eq!(text.matches("IFCBEAM(").count(), count(|k| matches!(k, ElementKind::Beam(_))));
    let uid = cc_ifc::guid::encode(p.element(ElementId(host)).unwrap().uid.0);
    assert!(!text.contains(&uid), "the truss itself is not exported");
}

#[test]
fn a_default_flat_truss_is_braced_at_its_supports() {
    let mut s = Session::default();
    run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "bottom_offset": 2.0}));
    let geom = s.geometry().clone();
    let model = cc_geom::analytical::analytical_model(s.project(), &geom);
    for x in [0.0, 6.0] {
        let at = cc_core::DVec3::new(x, 0.0, 2.0);
        let node = model.nodes.iter().position(|n| n.position.distance(at) < 1e-6);
        let node = node.unwrap_or_else(|| panic!("a node at the support {x}"));
        let bars = model.members.iter().filter(|m| m.start == node || m.end == node).count();
        assert!(bars >= 2, "{bars} bar(s) at the support {x}");
    }
}

#[test]
fn the_members_meet_at_the_panel_points_whatever_their_sections() {
    // Steel posts stand inside the chords' channels, timber ones between
    // the chords' faces, and the chords run on over the end posts: the
    // analytical model still has one node per panel point and no stubs.
    for material in ["steel", "timber"] {
        let mut s = Session::default();
        let args = json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "panels": 4, "bottom_offset": 2.0,
            "material": material});
        run(&mut s, "truss_create", args);
        let geom = s.geometry().clone();
        let model = cc_geom::analytical::analytical_model(s.project(), &geom);
        assert_eq!(model.nodes.len(), 10, "{material}");
        // Each chord split into 4, 5 posts and 4 diagonals.
        assert_eq!(model.members.len(), 17, "{material}");
        assert!(
            model.nodes.iter().all(|n| n.support.is_none()),
            "{material}: posts stand on the chord, not on supports"
        );
        for n in &model.nodes {
            let (x, z) = (n.position.x / 1.5, n.position.z);
            assert!((x - x.round()).abs() < 1e-9 && ((z - 2.0).abs() < 1e-9 || (z - 3.0).abs() < 1e-9), "{n:?}");
        }
    }
}

#[test]
fn the_chords_close_the_ends_over_the_posts() {
    let mut s = Session::default();
    let args = json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0, "panels": 4, "bottom_offset": 2.0});
    let host = id(&run(&mut s, "truss_create", args));
    let members = derived(&mut s, host)["members"].clone();
    let geom = s.geometry().clone();
    // Ue 100x50 posts, 50 mm along the truss: the chords' bodies reach 25 mm past the span.
    for chord in ["top_chords", "bottom_chords"] {
        let g = &geom.beams[&ElementId(members[chord][0].as_u64().unwrap())];
        assert!((g.length - 6.05).abs() < 1e-9 && (g.origin.x + 0.025).abs() < 1e-9, "{chord}");
        assert!((g.start_ext.0, g.end_ext.0) == (0.0, g.length), "{chord}: not cut at the posts");
    }
    // The posts stand inside the channels, on their webs (2 mm), the last one turned around.
    let posts: Vec<_> = members["verticals"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    for (i, post) in posts.iter().enumerate() {
        let Some(ElementKind::Column(c)) = s.project().element(ElementId(*post)).map(|e| &e.kind) else {
            panic!("a column")
        };
        assert!((c.base_offset - 1.972).abs() < 1e-6 && (c.height - 1.056).abs() < 1e-6, "{c:?}");
        assert_eq!(c.rotation_deg, if i + 1 == posts.len() { 180.0 } else { 0.0 });
    }
}

#[test]
fn a_truss_over_the_member_limit_is_refused() {
    let mut s = Session::default();
    let depth = s.doc.undo_depth();
    let args = json!({"start": [0, 0], "end": [100, 0], "height": 2.0, "panels": 64, "copies": 200});
    let err = s.execute("truss_create", args).unwrap_err();
    assert!(err.message.contains("members"), "{err:?}");
    assert_eq!(s.doc.undo_depth(), depth, "no undo step");
    assert_eq!(trusses(&s), 0);
    assert_eq!(types_named(&s, "Ue 150x60x20x2,00"), 0);
    let host = id(&run(&mut s, "truss_create", json!({"start": SPAN[0], "end": SPAN[1], "height": 1.0})));
    let depth = s.doc.undo_depth();
    let set = json!({"id": host, "params": {"end": [100, 0], "panels": 64, "copies": 200}});
    assert!(s.execute("element_set", set).is_err());
    assert_eq!(s.doc.undo_depth(), depth);
    assert!(s.execute("element_set", json!({"id": host, "params": {"base_offset": 1e300}})).is_err());
}
