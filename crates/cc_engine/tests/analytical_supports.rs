//! Supports, materials and column tops in the analytical model (format 4):
//! each support node carries its column's `base_support` and `restraint`,
//! the members' materials come with their mechanical properties, and a
//! column top that only beams frame into ends on their axis.

use cc_engine::{ErrorCode, Session};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn fails(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    s.execute(name, args).expect_err("should fail").code
}

fn node_at(m: &Value, x: f64, y: f64, z: f64) -> Option<&Value> {
    m["nodes"].as_array().unwrap().iter().find(|n| {
        (n["x"].as_f64().unwrap() - x).abs() < 1e-6
            && (n["y"].as_f64().unwrap() - y).abs() < 1e-6
            && (n["z"].as_f64().unwrap() - z).abs() < 1e-6
    })
}

fn members_of(m: &Value, element: &Value) -> usize {
    m["members"].as_array().unwrap().iter().filter(|x| &x["element"] == element).count()
}

/// A portal: 3 m columns at x = 0 (fixed) and x = 5 (pinned) and a 0.40 m
/// beam between them, its top at 3 m; the ids of the columns and the beam.
fn portal(s: &mut Session) -> (Value, Value, Value) {
    let a = run(s, "column_create", json!({"position": [0, 0], "height": 3.0}));
    let b = run(s, "column_create", json!({"position": [5, 0], "height": 3.0, "base_support": "pinned"}));
    let beam = run(s, "beam_create", json!({"start": [0, 0], "end": [5, 0], "top_offset": 3.0}));
    (a["id"].clone(), b["id"].clone(), beam["id"].clone())
}

#[test]
fn support_nodes_carry_their_restraint() {
    let mut s = Session::default();
    portal(&mut s);
    let m = run(&mut s, "analytical_model", json!({}));
    assert_eq!(m["version"], json!(4));
    let fixed = node_at(&m, 0.0, 0.0, 0.0).expect("the fixed base");
    assert_eq!(fixed["support"], json!(true));
    assert_eq!(fixed["base_support"], json!("fixed"));
    assert_eq!(fixed["restraint"], json!([true, true, true, true, true, true]));
    let pinned = node_at(&m, 5.0, 0.0, 0.0).expect("the pinned base");
    assert_eq!(pinned["base_support"], json!("pinned"));
    assert_eq!(pinned["restraint"], json!([true, true, true, false, false, false]));
    // Other nodes have no support and no restraint.
    let joint = node_at(&m, 0.0, 0.0, 2.8).expect("the joint on the beam's axis");
    assert_eq!(joint["support"], json!(false));
    assert!(joint.get("restraint").is_none());
    assert!(m["axes"].as_str().is_some_and(|a| a.contains("[ux, uy, uz, rx, ry, rz]")));
    assert_eq!(m["units"]["modulus"], json!("Pa"));
}

#[test]
fn a_column_base_support_is_set_like_any_parameter() {
    let mut s = Session::default();
    let (a, _, _) = portal(&mut s);
    run(&mut s, "element_set", json!({"id": a, "params": {"base_support": "Pinned"}}));
    let m = run(&mut s, "analytical_model", json!({}));
    assert_eq!(node_at(&m, 0.0, 0.0, 0.0).unwrap()["base_support"], json!("pinned"));
    assert_eq!(
        fails(&mut s, "column_create", json!({"position": [9, 0], "base_support": "roller"})),
        ErrorCode::InvalidArguments
    );
}

/// The columns rise 0.20 m above the beam's axis (2.80 m): that piece is
/// inside the joint, so each column is one member up to the axis, with no
/// free node at 3 m.
#[test]
fn a_column_top_ends_on_the_axis_of_the_beams_framed_into_it() {
    let mut s = Session::default();
    let (a, b, beam) = portal(&mut s);
    let m = run(&mut s, "analytical_model", json!({}));
    assert_eq!(members_of(&m, &a), 1);
    assert_eq!(members_of(&m, &b), 1);
    assert_eq!(members_of(&m, &beam), 1);
    assert!(node_at(&m, 0.0, 0.0, 3.0).is_none() && node_at(&m, 5.0, 0.0, 3.0).is_none());
    assert_eq!(m["nodes"].as_array().unwrap().len(), 4);
}

/// A column standing on the one below keeps the piece above the beam: the
/// joint and the upper column's base are both on the lower column.
#[test]
fn a_column_continued_above_keeps_its_top() {
    let mut s = Session::default();
    let (a, _, _) = portal(&mut s);
    run(&mut s, "column_create", json!({"position": [0, 0], "level": "Pavimento 1", "height": 3.0}));
    let m = run(&mut s, "analytical_model", json!({}));
    assert_eq!(members_of(&m, &a), 2, "split at the beam's axis, up to the upper column");
    assert!(node_at(&m, 0.0, 0.0, 3.0).is_some());
}

#[test]
fn materials_come_with_their_mechanical_properties() {
    let mut s = Session::default();
    portal(&mut s);
    let m = run(&mut s, "analytical_model", json!({}));
    let materials = m["materials"].as_array().unwrap();
    assert!(!materials.is_empty());
    for member in m["members"].as_array().unwrap() {
        let name = &member["material"];
        assert!(materials.iter().any(|x| &x["name"] == name), "{name} is listed");
    }
    // Every material has the fields, null where unknown; a known modulus is in Pa.
    for x in materials {
        for key in ["class", "modulus", "shear_modulus", "poisson", "density", "yield_strength", "compressive_strength"]
        {
            assert!(x.get(key).is_some(), "{key} in {x}");
        }
        if let Some(e) = x["modulus"].as_f64() {
            assert!(e > 1e9, "{e} Pa");
        }
    }
}
