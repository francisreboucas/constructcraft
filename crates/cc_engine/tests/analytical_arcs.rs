//! Arc beams in the analytical model: their members follow the arc (bulge,
//! center, radius, sweep and the arc's length), split where other members
//! connect along it, or as straight chords with `arc_segments`.

use cc_engine::{ErrorCode, Session};
use serde_json::{Value, json};
use std::f64::consts::{FRAC_PI_2, PI};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn fails(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    s.execute(name, args).expect_err("should fail").code
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ccad-analytical-arcs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

const R: f64 = 2.5;

/// Two 3 m columns at (2.5, 0) and (0, 2.5) and a quarter ring beam of
/// radius 2.5 m around the origin between them, counter-clockwise; the ring's id.
fn quarter_ring(s: &mut Session) -> u64 {
    for p in [[R, 0.0], [0.0, R]] {
        run(s, "column_create", json!({"position": p, "height": 3.0}));
    }
    let ring =
        run(s, "beam_create", json!({"start": [R, 0], "end": [0, R], "bulge": (PI / 8.0).tan(), "top_offset": 3.0}));
    ring["id"].as_u64().unwrap()
}

/// The members of element `id`.
fn members_of(m: &Value, id: u64) -> Vec<Value> {
    m["members"].as_array().unwrap().iter().filter(|x| x["element"] == json!(id)).cloned().collect()
}

/// The plan position of node `i`.
fn node(m: &Value, i: &Value) -> (f64, f64) {
    let n = &m["nodes"][i.as_u64().unwrap() as usize];
    (n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap())
}

fn on_ring((x, y): (f64, f64)) -> bool {
    close(x.hypot(y), R)
}

#[test]
fn a_quarter_ring_beam_exports_the_arc_length() {
    let mut s = Session::default();
    let ring = quarter_ring(&mut s);
    let path = scratch("quarter.analytical.json");
    run(&mut s, "analytical_export", json!({"path": path}));
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(m["version"], json!(4));
    let members = members_of(&m, ring);
    assert_eq!(members.len(), 1, "{m}");
    let member = &members[0];
    // The arc's length, 2.5 π / 2 = 3.927 m, not the chord's 3.536 m.
    let length = member["length"].as_f64().unwrap();
    assert!(close(length, R * FRAC_PI_2), "{length}");
    assert!((length - 3.927).abs() < 5e-4 && (length - R * 2f64.sqrt()).abs() > 0.3);
    let arc = &member["arc"];
    assert!(close(arc["bulge"].as_f64().unwrap(), (PI / 8.0).tan()), "{arc}");
    assert!(close(arc["radius"].as_f64().unwrap(), R));
    assert!(close(arc["sweep"].as_f64().unwrap(), 90.0));
    assert!(close(arc["center"]["x"].as_f64().unwrap(), 0.0) && close(arc["center"]["y"].as_f64().unwrap(), 0.0));
    // From column axis to column axis.
    assert_eq!(node(&m, &member["start"]), (R, 0.0));
    let end = node(&m, &member["end"]);
    assert!(close(end.0, 0.0) && close(end.1, R), "{end:?}");
}

#[test]
fn a_connecting_member_splits_the_arc_on_it() {
    let mut s = Session::default();
    let ring = quarter_ring(&mut s);
    // A radial beam from the center to the middle of the arc.
    let mid = [R * (PI / 4.0).cos(), R * (PI / 4.0).sin()];
    let radial = run(&mut s, "beam_create", json!({"start": [0, 0], "end": mid, "top_offset": 3.0}));
    let m = run(&mut s, "analytical_model", json!({}));
    let parts = members_of(&m, ring);
    assert_eq!(parts.len(), 2, "{m}");
    // The parts' lengths add up to the arc's, each the arc of 45°.
    let total: f64 = parts.iter().map(|p| p["length"].as_f64().unwrap()).sum();
    assert!(close(total, R * FRAC_PI_2), "{total}");
    for p in &parts {
        assert!(close(p["length"].as_f64().unwrap(), R * PI / 4.0));
        assert!(close(p["arc"]["bulge"].as_f64().unwrap(), (PI / 16.0).tan()), "{p}");
        assert!(close(p["arc"]["sweep"].as_f64().unwrap(), 45.0));
        assert!(on_ring(node(&m, &p["start"])) && on_ring(node(&m, &p["end"])));
    }
    // The shared node is on the arc, where the radial beam ends.
    assert_eq!(parts[0]["end"], parts[1]["start"]);
    let joint = node(&m, &parts[0]["end"]);
    assert!(close(joint.0, mid[0]) && close(joint.1, mid[1]), "{joint:?}");
    let spoke = members_of(&m, radial["id"].as_u64().unwrap());
    assert_eq!(spoke.len(), 1);
    assert_eq!(spoke[0]["end"], parts[0]["end"]);
    assert!(spoke[0].get("arc").is_none(), "a straight member has no arc");
}

#[test]
fn arc_segments_split_arcs_into_chords_on_the_arc() {
    let mut s = Session::default();
    let ring = quarter_ring(&mut s);
    let path = scratch("chords.analytical.json");
    let out = run(&mut s, "analytical_export", json!({"path": path, "arc_segments": 4}));
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(out["members"].as_u64(), Some(m["members"].as_array().unwrap().len() as u64));
    let chords = members_of(&m, ring);
    assert_eq!(chords.len(), 4, "{m}");
    // Each 22.5°: 2 r sin(11.25°), straight, between nodes on the arc, in a chain.
    let chord = 2.0 * R * (PI / 16.0).sin();
    for (i, c) in chords.iter().enumerate() {
        assert!(c.get("arc").is_none(), "{c}");
        assert!(close(c["length"].as_f64().unwrap(), chord), "{c}");
        assert!(on_ring(node(&m, &c["start"])) && on_ring(node(&m, &c["end"])));
        if i > 0 {
            assert_eq!(chords[i - 1]["end"], c["start"]);
        }
    }
    // The query takes it too; it is checked.
    let q = run(&mut s, "analytical_model", json!({"arc_segments": 4}));
    assert_eq!(members_of(&q, ring).len(), 4);
    assert_eq!(fails(&mut s, "analytical_model", json!({"arc_segments": 0})), ErrorCode::InvalidArguments);
    assert_eq!(
        fails(&mut s, "analytical_export", json!({"path": path, "arc_segments": 1000})),
        ErrorCode::InvalidArguments
    );
}

#[test]
fn straight_beams_are_unchanged() {
    let mut s = Session::default();
    for x in [0, 6] {
        run(&mut s, "column_create", json!({"position": [x, 0], "height": 3.0}));
    }
    let beam = run(&mut s, "beam_create", json!({"start": [0, 0], "end": [6, 0], "top_offset": 3.0}));
    let beam = beam["id"].as_u64().unwrap();
    let m = run(&mut s, "analytical_model", json!({}));
    let split = run(&mut s, "analytical_model", json!({"arc_segments": 8}));
    // Splitting arcs leaves a model with none as it is.
    assert_eq!(m, split);
    let members = members_of(&m, beam);
    assert_eq!(members.len(), 1);
    assert!(close(members[0]["length"].as_f64().unwrap(), 6.0));
    assert!(m["members"].as_array().unwrap().iter().all(|x| x.get("arc").is_none()), "{m}");
}
