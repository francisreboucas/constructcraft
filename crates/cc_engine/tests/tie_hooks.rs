//! #438: cross-ties keep their hooks through to the bending schedule. A wall
//! tie (`wall_rebar_create` `ties`) is an S, a column tie (`rebar_create`
//! `ties`) a C, both written `tie … hook … hook …`; stirrups stay stirrups.

use cc_core::{ElementId, ElementKind, RebarLayout};
use cc_engine::Session;
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap_or_else(|| panic!("no id in {v}"))
}

/// The mark of the bars of set `set`.
fn mark_of(s: &mut Session, set: u64) -> Value {
    let marks = run(s, "bar_marks_list", json!({}))["marks"].as_array().unwrap().clone();
    marks
        .into_iter()
        .find(|m| m["sets"].as_array().unwrap().iter().any(|x| x.as_u64() == Some(set)))
        .unwrap_or_else(|| panic!("no mark for set {set}"))
}

/// The bends (degrees) of a mark's legs, and which legs are hooks.
fn legs(mark: &Value) -> Vec<(f64, bool)> {
    let legs = mark["shape"]["legs"].as_array().unwrap();
    legs.iter().map(|l| (l["bend"].as_f64().unwrap(), l["hook"].as_bool().unwrap())).collect()
}

#[test]
fn wall_and_column_ties_reach_the_bending_schedule_with_their_hooks() {
    let mut s = Session::default();
    // A wall with ties: hooks of 135° at both faces, bent to opposite sides (an S).
    let wall = id(&run(&mut s, "wall_create", json!({"start": [0, 0], "end": [6, 0], "height": 3.0})));
    let args = json!({
        "walls": [wall], "horizontal": {"spacing": 0.25}, "vertical": {"spacing": 0.25},
        "ties": {"every_horizontal": 2, "every_vertical": 3},
    });
    let created = run(&mut s, "wall_rebar_create", args)["created"].clone();
    let wall_ties = created
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .find(|x| {
            matches!(&s.project().element(ElementId(*x)).unwrap().kind,
                ElementKind::Rebar(r) if matches!(r.layout, RebarLayout::WallTies { .. }))
        })
        .unwrap();
    let m = mark_of(&mut s, wall_ties);
    let code = m["code"].as_str().unwrap();
    assert!(code.starts_with("tie ") && code.matches("hook 135°").count() == 2, "{code}");
    let l = legs(&m);
    assert_eq!(l.iter().map(|(_, h)| *h).collect::<Vec<_>>(), [true, false, true], "{m}");
    assert!(l[0].0 * l[1].0 < 0.0 && (l[0].0.abs() - 135.0).abs() < 1e-6, "an S: {l:?}");

    // A column with bars between its corners, its ties 135° and 180°, both to one side (a C),
    // and its stirrups still read as stirrups.
    let column = id(&run(&mut s, "column_create", json!({"position": [10, 0]})));
    run(&mut s, "rebar_create", json!({"host": column, "layout": "perimeter", "count": 8}));
    let ties = id(&run(&mut s, "rebar_create", json!({"host": column, "layout": "ties", "spacing": 0.3})));
    let stirrups = id(&run(&mut s, "rebar_create", json!({"host": column, "layout": "stirrups", "spacing": 0.15})));
    let m = mark_of(&mut s, ties);
    let code = m["code"].as_str().unwrap();
    assert!(code.starts_with("tie ") && code.contains("hook 135°") && code.contains("hook 180°"), "{code}");
    let l = legs(&m);
    assert!(l[0].0 * l[1].0 > 0.0, "a C: {l:?}");
    let stirrup = mark_of(&mut s, stirrups);
    assert!(stirrup["code"].as_str().unwrap().starts_with("stirrup "), "{stirrup}");

    // The bending schedule writes the ties' codes.
    let view = id(&run(&mut s, "bending_schedule_create", json!({})));
    let path = std::env::temp_dir().join(format!("tie_hooks_{}.csv", std::process::id()));
    run(&mut s, "schedule_export_csv", json!({"view": view, "path": path}));
    let csv = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(csv.lines().filter(|l| l.contains("tie ") && l.contains("hook")).count(), 2, "{csv}");
    assert!(csv.contains("stirrup "), "{csv}");
}
