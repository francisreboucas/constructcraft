//! Detail lines (issue #149): view-specific lines, arcs, circles, polylines
//! and ellipses with a line style, drawn only in their view, edited with the
//! generic modify commands, exported to DXF on their style's layer and left
//! out of IFC.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_engine::{ErrorCode, Session};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn fails(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    s.execute(name, args).map(|v| panic!("{name} should fail, gave {v}")).unwrap_err().code
}

fn ids(v: &Value) -> Vec<u64> {
    v["created"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect()
}

/// The elements a view's drawing shows.
fn drawn(s: &mut Session, view: u64) -> std::collections::BTreeSet<u64> {
    let d = s.drawing(cc_core::ViewId(view)).unwrap();
    d.prims.iter().filter_map(|p| p.element()).map(|e| e.0).collect()
}

fn first_plan(s: &mut Session) -> u64 {
    run(s, "views_list", json!({"category": "floor_plan"}))[0]["id"].as_u64().unwrap()
}

fn curve(s: &mut Session, id: u64) -> Value {
    run(s, "element_get", json!({"id": id}))["params"]["curve"].clone()
}

#[test]
fn detail_lines_are_drawn_only_in_their_view() {
    let mut s = Session::default();
    let plan = first_plan(&mut s);
    let drafting = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();

    // A polyline (three segments), an arc, a circle and an ellipse in one step.
    let out = run(
        &mut s,
        "detail_line_create",
        json!({
            "view": drafting,
            "style": "wide",
            "curves": [
                {"kind": "line", "start": [0, 0], "end": [1, 0]},
                {"kind": "line", "start": [1, 0], "end": [1, 1]},
                {"kind": "line", "start": [1, 1], "end": [0, 1]},
                {"kind": "arc", "start": [2, 0], "end": [3, 0], "through": [2.5, 0.5]},
                {"kind": "circle", "center": [5, 0], "radius": 0.5},
                {"kind": "ellipse", "center": [7, 0], "radii": [1, 0.5], "rotation": 30},
            ]
        }),
    );
    let made = ids(&out);
    assert_eq!(made.len(), 6, "one element per curve");
    let shown = drawn(&mut s, drafting);
    assert!(made.iter().all(|id| shown.contains(id)));
    // Only in the view that owns them.
    assert!(made.iter().all(|id| !drawn(&mut s, plan).contains(id)));
    run(&mut s, "undo", json!({}));
    assert!(drawn(&mut s, drafting).is_empty(), "all of it in one undo step");
    run(&mut s, "redo", json!({}));

    // element_get shows the curve and style; element_set changes them.
    let got = run(&mut s, "element_get", json!({"id": made[0]}));
    assert_eq!(got["category"], "detailline");
    assert_eq!(got["params"]["style"], json!({"BuiltIn": "Wide"}));
    assert_eq!(got["derived"]["style_name"], "wide");
    assert!((got["derived"]["length"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    run(&mut s, "element_set", json!({"id": made[0], "params": {"style": "hidden"}}));
    assert_eq!(run(&mut s, "element_get", json!({"id": made[0]}))["params"]["style"], json!({"BuiltIn": "Hidden"}));
    run(&mut s, "element_set", json!({"id": made[0], "params": {"curve": {"Line": {"a": [0, 0], "b": [2, 0]}}}}));
    assert_eq!(curve(&mut s, made[0]), json!({"Line": {"a": [0.0, 0.0], "b": [2.0, 0.0]}}));
    // A curve without size, another view or an unknown style are refused.
    let bad = json!({"id": made[0], "params": {"curve": {"Line": {"a": [0, 0], "b": [0, 0]}}}});
    assert_eq!(fails(&mut s, "element_set", bad), ErrorCode::InvalidArguments);
    assert_eq!(
        fails(&mut s, "element_set", json!({"id": made[0], "params": {"view": plan}})),
        ErrorCode::InvalidArguments
    );
    assert_eq!(
        fails(&mut s, "element_set", json!({"id": made[0], "params": {"style": "bold"}})),
        ErrorCode::InvalidArguments
    );

    // Plans take them too, on top of the model; 3D views and schedules do not.
    let wall = run(&mut s, "wall_create", json!({"start": [0, 0], "end": [5, 0]}))["id"].as_u64().unwrap();
    let on_plan = ids(&run(
        &mut s,
        "detail_line_create",
        json!({"view": plan, "curves": [{"kind": "line", "start": [0, -1], "end": [5, -1]}]}),
    ));
    let d = s.drawing(cc_core::ViewId(plan)).unwrap();
    let first = |id: u64| d.prims.iter().position(|p| p.element().map(|e| e.0) == Some(id)).unwrap();
    assert!(first(on_plan[0]) > first(wall), "drawn over the model");
    let three_d = run(&mut s, "views_list", json!({"category": "three_d"}))[0]["id"].clone();
    let line = json!([{"kind": "line", "start": [0, 0], "end": [1, 0]}]);
    assert_eq!(
        fails(&mut s, "detail_line_create", json!({"view": three_d, "curves": line})),
        ErrorCode::InvalidArguments
    );
    assert_eq!(fails(&mut s, "detail_line_create", json!({"view": plan, "curves": []})), ErrorCode::InvalidArguments);

    // Deleting the view deletes its lines.
    run(&mut s, "view_delete", json!({"view": drafting}));
    assert!(s.project().element(cc_core::ElementId(made[1])).is_none());
}

#[test]
fn detail_lines_move_and_edit_with_the_modify_commands() {
    let mut s = Session::default();
    let plan = first_plan(&mut s);
    let make =
        |s: &mut Session, curves: Value| ids(&run(s, "detail_line_create", json!({"view": plan, "curves": curves})));
    let a = make(&mut s, json!([{"kind": "line", "start": [0, 0], "end": [2, 0]}]))[0];
    let b = make(&mut s, json!([{"kind": "line", "start": [3, 1], "end": [3, 3]}]))[0];

    // Move, rotate and mirror move the curve.
    run(&mut s, "elements_move", json!({"ids": [a], "delta": [0, 1]}));
    assert_eq!(curve(&mut s, a), json!({"Line": {"a": [0.0, 1.0], "b": [2.0, 1.0]}}));
    run(&mut s, "elements_move", json!({"ids": [a], "delta": [0, -1]}));
    let copy = run(&mut s, "elements_copy", json!({"ids": [a], "delta": [0, 5]}));
    assert_eq!(copy["created"].as_array().unwrap().len(), 1);

    // Trim corner: both lines meet at (3, 0).
    run(&mut s, "trim_corner", json!({"a": a, "pick_a": [1, 0], "b": b, "pick_b": [3, 2]}));
    assert_eq!(curve(&mut s, a), json!({"Line": {"a": [0.0, 0.0], "b": [3.0, 0.0]}}));
    assert_eq!(curve(&mut s, b), json!({"Line": {"a": [3.0, 0.0], "b": [3.0, 3.0]}}));

    // Split: the new element is the part after the point.
    let split = run(&mut s, "element_split", json!({"id": a, "point": [1, 0]}));
    let second = split["created"].as_u64().unwrap();
    assert_eq!(curve(&mut s, a), json!({"Line": {"a": [0.0, 0.0], "b": [1.0, 0.0]}}));
    assert_eq!(curve(&mut s, second), json!({"Line": {"a": [1.0, 0.0], "b": [3.0, 0.0]}}));

    // Offset: a parallel copy to the left; a circle's copy is concentric.
    let off = run(&mut s, "element_offset", json!({"id": a, "distance": 0.5}))["id"].as_u64().unwrap();
    assert_eq!(curve(&mut s, off), json!({"Line": {"a": [0.0, 0.5], "b": [1.0, 0.5]}}));
    let circle = make(&mut s, json!([{"kind": "circle", "center": [10, 0], "radius": 1}]))[0];
    let inner = run(&mut s, "element_offset", json!({"id": circle, "distance": 0.25}))["id"].as_u64().unwrap();
    assert_eq!(curve(&mut s, inner), json!({"Circle": {"center": [10.0, 0.0], "radius": 0.75}}));

    // Trim/extend to a circle: the line from (0, 5) runs on to the circle.
    let c = make(&mut s, json!([{"kind": "line", "start": [6, 0], "end": [8, 0]}]))[0];
    run(&mut s, "trim_extend", json!({"id": c, "pick": [7, 0], "reference": circle}));
    assert_eq!(curve(&mut s, c), json!({"Line": {"a": [6.0, 0.0], "b": [9.0, 0.0]}}));
    // And to a wall's line.
    let wall = run(&mut s, "wall_create", json!({"start": [-1, -5], "end": [-1, 5]}))["id"].as_u64().unwrap();
    run(&mut s, "trim_extend", json!({"id": a, "pick": [0.5, 0], "reference": wall}));
    assert_eq!(curve(&mut s, a), json!({"Line": {"a": [-1.0, 0.0], "b": [1.0, 0.0]}}));
    // A wall is not trimmed to a corner with a detail line.
    let mixed = json!({"a": a, "pick_a": [0, 0], "b": wall, "pick_b": [-1, 1]});
    assert!(s.execute("trim_corner", mixed).is_err());
}

#[test]
fn dxf_puts_detail_lines_on_their_style_layer_and_ifc_leaves_them_out() {
    let mut s = Session::default();
    let view = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();
    run(
        &mut s,
        "detail_line_create",
        json!({"view": view, "style": "center", "curves": [{"kind": "line", "start": [0, 0], "end": [1, 0]}]}),
    );
    let dir = std::env::temp_dir().join(format!("ccad-detail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dxf = dir.join("detail.dxf");
    run(&mut s, "view_export", json!({"view": view, "path": dxf}));
    let text = std::fs::read_to_string(&dxf).unwrap();
    assert!(text.contains("\nCENTER\n"), "a layer named after the style");

    let ifc = dir.join("detail.ifc");
    run(&mut s, "ifc_export", json!({"path": ifc}));
    let text = std::fs::read_to_string(&ifc).unwrap();
    // Neither the lines nor their type are written.
    assert!(!text.contains("Detail line") && !text.contains("Linha de detalhe"));
}

#[test]
fn files_with_bad_detail_lines_are_repaired() {
    let mut s = Session::default();
    let view = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();
    let made = ids(&run(
        &mut s,
        "detail_line_create",
        json!({"view": view, "curves": [{"kind": "line", "start": [0, 0], "end": [1, 0]}, {"kind": "circle", "center": [0, 0], "radius": 1}]}),
    ));
    let mut project = s.project().clone();
    if let Some(cc_core::ElementKind::DetailLine(l)) =
        project.elements.get_mut(&cc_core::ElementId(made[1])).map(|e| &mut e.kind)
    {
        l.curve = cc_core::sketch::SketchCurve::Circle { center: glam::DVec2::ZERO, radius: f64::NAN };
    }
    let text = cc_core::to_ron(&project).unwrap();
    let (loaded, repairs) = cc_core::from_ron_with_repairs(&text).unwrap();
    assert!(repairs >= 1);
    assert!(loaded.element(cc_core::ElementId(made[0])).is_some());
    assert!(loaded.element(cc_core::ElementId(made[1])).is_none());
}

/// The detail lines of `view` as their curves (JSON).
fn curves_of(s: &mut Session, view: u64) -> Vec<Value> {
    let list = run(s, "elements_list", json!({"category": "detailline"}));
    let ids: Vec<u64> = list.as_array().unwrap().iter().filter_map(|e| e["id"].as_u64()).collect();
    ids.into_iter()
        .filter_map(|id| {
            let got = run(s, "element_get", json!({"id": id}));
            (got["params"]["view"].as_u64() == Some(view)).then(|| got["params"]["curve"].clone())
        })
        .collect()
}

/// `radius` rounds a rectangle's corners and `offset` moves it outward, as
/// for walls.
#[test]
fn detail_lines_take_an_offset_and_a_corner_radius() {
    let mut s = Session::default();
    let view = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();
    let rect = json!({"kind": "rectangle", "corner": [0, 0], "opposite": [4, 2]});
    let made = ids(&run(&mut s, "detail_line_create", json!({"view": view, "curves": [rect], "radius": 0.5})));
    assert_eq!(made.len(), 8, "four sides and four corner arcs");
    let arcs = curves_of(&mut s, view).iter().filter(|c| c.get("Arc").is_some()).count();
    assert_eq!(arcs, 4);

    let other = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();
    run(&mut s, "detail_line_create", json!({"view": other, "curves": [rect], "offset": 1.0}));
    // Outward: the sides now run from -1 to 5 and from -1 to 3.
    let mut xs = Vec::new();
    for c in curves_of(&mut s, other) {
        for end in ["a", "b"] {
            xs.push(c["Line"][end][0].as_f64().unwrap());
        }
    }
    let (lo, hi) = xs.iter().fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(*x), hi.max(*x)));
    assert!((lo + 1.0).abs() < 1e-9 && (hi - 5.0).abs() < 1e-9, "{lo} {hi}");

    assert_eq!(
        fails(&mut s, "detail_line_create", json!({"view": view, "curves": [rect], "radius": -1})),
        ErrorCode::InvalidArguments
    );
}

/// `detail_line_fillet` rounds the corner of two straight detail lines with
/// an arc tangent to both, in one undo step.
#[test]
fn detail_line_fillet_rounds_a_corner() {
    let mut s = Session::default();
    let view = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].as_u64().unwrap();
    let lines = json!([
        {"kind": "line", "start": [0, 0], "end": [3, 0]},
        {"kind": "line", "start": [3, 0], "end": [3, 3]},
    ]);
    let made = ids(&run(&mut s, "detail_line_create", json!({"view": view, "curves": lines, "style": "wide"})));
    let out = run(
        &mut s,
        "detail_line_fillet",
        json!({"a": made[0], "pick_a": [1, 0], "b": made[1], "pick_b": [3, 1], "radius": 1.0}),
    );
    let arc = out["created"].as_u64().unwrap();
    let at = |v: &Value, x: f64, y: f64| {
        let (vx, vy) = (v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
        assert!((vx - x).abs() < 1e-9 && (vy - y).abs() < 1e-9, "{v} is not ({x}, {y})");
    };
    at(&curve(&mut s, made[0])["Line"]["b"], 2.0, 0.0);
    at(&curve(&mut s, made[1])["Line"]["a"], 3.0, 1.0);
    let got = run(&mut s, "element_get", json!({"id": arc}));
    assert_eq!(got["params"]["style"], json!({"BuiltIn": "Wide"}), "the first line's style");
    let a = &got["params"]["curve"]["Arc"];
    at(&a["a"], 2.0, 0.0);
    at(&a["b"], 3.0, 1.0);
    run(&mut s, "undo", json!({}));
    assert_eq!(curves_of(&mut s, view).len(), 2, "one undo step");

    // Too large a radius, the same line twice, or an arc is refused.
    let big = json!({"a": made[0], "pick_a": [1, 0], "b": made[1], "pick_b": [3, 1], "radius": 10.0});
    assert_eq!(fails(&mut s, "detail_line_fillet", big), ErrorCode::InvalidArguments);
    let same = json!({"a": made[0], "pick_a": [1, 0], "b": made[0], "pick_b": [2, 0], "radius": 1.0});
    assert_eq!(fails(&mut s, "detail_line_fillet", same), ErrorCode::InvalidArguments);
}
