//! The 2D form of 3D reinforcement views (on sheets and in exports): the
//! bars over the model drawn faint, as the 3D view shows them, and bars
//! seen end-on as dots.

use cc_engine::Session;
use cc_views::{LineStyle, Prim};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap_or_else(|| panic!("no id in {v}"))
}

#[test]
fn reinforcement_views_draw_the_model_faint() {
    let mut s = Session::default();
    let column = id(&run(&mut s, "column_create", json!({"position": [0, 0]})));
    let bars = id(&run(&mut s, "rebar_create", json!({"host": column, "layout": "perimeter", "count": 4})));
    let count = s.geometry().rebar[&cc_core::ElementId(bars)].bars.len();
    let view = id(&run(&mut s, "view_create", json!({"kind": "rebar"})));
    let d = s.drawing(cc_core::ViewId(view)).unwrap();
    // The column's edges, faint and only faint, under the bars.
    let column_lines = |style: fn(&LineStyle) -> bool| {
        d.prims
            .iter()
            .any(|p| matches!(p, Prim::Line { element: Some(x), style: st, .. } if x.0 == column && style(st)))
    };
    assert!(column_lines(|st| *st == LineStyle::Faint));
    // Drawn over the model: every prim of the bars comes after the model's.
    let last_model = d.prims.iter().rposition(|p| matches!(p, Prim::Line { element: Some(x), .. } if x.0 == column));
    let first_bar = d.prims.iter().position(|p| matches!(p, Prim::Line { element: Some(x), .. } if x.0 == bars));
    assert!(last_model < first_bar, "{last_model:?} {first_bar:?}");
    assert!(!column_lines(|st| matches!(st, LineStyle::Cut | LineStyle::Projection)));
    let lines = d
        .prims
        .iter()
        .filter(|p| matches!(p, Prim::Line { element: Some(x), style: LineStyle::Projection, .. } if x.0 == bars))
        .count();
    assert_eq!(lines, count);
    // Seen from above, the vertical bars are dots.
    let camera = json!({"target": [0, 0, 1.5], "yaw_deg": 0, "pitch_deg": 90, "distance": 10});
    run(&mut s, "view_set", json!({"view": view, "params": {"camera": camera}}));
    let d = s.drawing(cc_core::ViewId(view)).unwrap();
    let dots: Vec<&Vec<glam::DVec2>> = d
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Fill { pts, element: Some(x), .. } if x.0 == bars => Some(pts),
            _ => None,
        })
        .collect();
    assert_eq!(dots.len(), count);
    // As wide as the bar, at least 0.35 mm on paper (a thin bar at 1:100 here).
    let diameter = s.geometry().rebar[&cc_core::ElementId(bars)].diameter;
    let scale = s.project().view(cc_core::ViewId(view)).unwrap().scale as f64;
    let want = (diameter / 2.0).max(cc_views::rebar_view::MIN_DOT * scale / 1000.0);
    for pts in dots {
        let c = pts.iter().copied().sum::<glam::DVec2>() / pts.len() as f64;
        let r = pts.iter().map(|q| q.distance(c)).fold(0.0, f64::max);
        assert!((r - want).abs() < 1e-9, "{r} {want}");
    }
}

#[test]
fn a_starter_bar_seen_from_above_is_a_dot_and_its_foot() {
    let mut s = Session::default();
    let wall = id(&run(&mut s, "wall_create", json!({"start": [0, 0], "end": [2, 0], "height": 1.0})));
    let args = json!({"walls": [wall], "faces": "exterior", "vertical": {"bottom": {"across": 0.3}}, "max_length": 0});
    let created = run(&mut s, "wall_rebar_create", args)["created"].clone();
    let vertical = created
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .find(|x| {
            matches!(
                &s.project().element(cc_core::ElementId(*x)).unwrap().kind,
                cc_core::ElementKind::Rebar(r) if matches!(r.layout, cc_core::RebarLayout::WallVertical { .. })
            )
        })
        .unwrap();
    let count = s.geometry().rebar[&cc_core::ElementId(vertical)].bars.len();
    let view = id(&run(&mut s, "view_create", json!({"kind": "rebar"})));
    let camera = json!({"target": [1, 0, 0.5], "yaw_deg": 0, "pitch_deg": 90, "distance": 10});
    run(&mut s, "view_set", json!({"view": view, "params": {"camera": camera}}));
    let d = s.drawing(cc_core::ViewId(view)).unwrap();
    let of = |fill: bool| {
        d.prims
            .iter()
            .filter(|p| match p {
                Prim::Fill { element: Some(x), .. } => fill && x.0 == vertical,
                Prim::Line { element: Some(x), style: LineStyle::Projection, .. } => !fill && x.0 == vertical,
                _ => false,
            })
            .count()
    };
    // The vertical leg seen end-on is a dot, whatever its foot, which is drawn as a line.
    assert_eq!(of(true), count);
    assert!(of(false) >= count);
}
