//! Interior elevations (issue #477): a marker in a room and a view of each
//! of its sides, looking from the marker at the wall on that side, cropped
//! to the room and following the walls, the marker and the room.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_core::ViewId;
use cc_engine::{ErrorCode, Session};
use cc_views::Prim;
use glam::DVec2;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn fails(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    s.execute(name, args).map(|v| panic!("{name} should fail, gave {v}")).unwrap_err().code
}

/// A 6 x 4 m room of four walls (centered on the rectangle), named Kitchen.
fn kitchen(s: &mut Session) -> (Vec<u64>, u64) {
    let rect = json!({"kind": "rectangle", "corner": [0, 0], "opposite": [6, 4]});
    let walls: Vec<u64> = run(s, "walls_create", json!({"curves": [rect]}))["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let room = run(s, "room_create", json!({"point": [3.0, 2.0], "name": "Kitchen"}))["id"].as_u64().unwrap();
    (walls, room)
}

/// The views of a marker by side, from `interior_elevations_create`'s result.
fn views(out: &Value) -> Vec<(String, u64, String)> {
    out["views"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (v["side"].as_str().unwrap().to_string(), v["view"].as_u64().unwrap(), v["name"].as_str().unwrap().into())
        })
        .collect()
}

fn view_of(out: &Value, side: &str) -> ViewId {
    ViewId(views(out).into_iter().find(|(s, ..)| s == side).unwrap().1)
}

/// The elements a view draws and the extents of their lines.
fn drawn(s: &mut Session, view: ViewId) -> (BTreeSet<u64>, Option<(DVec2, DVec2)>) {
    let d = s.drawing(view).unwrap();
    let mut ids = BTreeSet::new();
    let mut ext: Option<(DVec2, DVec2)> = None;
    for p in d.own_prims() {
        if let Prim::Line { a, b, element: Some(e), .. } = p {
            ids.insert(e.0);
            let (lo, hi) = ext.unwrap_or((*a, *a));
            ext = Some((lo.min(a.min(*b)), hi.max(a.max(*b))));
        }
    }
    (ids, ext)
}

/// The thickness of a wall's type, meters.
fn wall_thickness(s: &mut Session, wall: u64) -> f64 {
    let p = s.project();
    let e = p.element(cc_core::ElementId(wall)).unwrap();
    p.types.get(&e.type_id).unwrap().thickness()
}

#[test]
fn a_room_gets_four_numbered_views_cropped_to_it() {
    let mut s = Session::default();
    let (walls, room) = kitchen(&mut s);
    let t = wall_thickness(&mut s, walls[0]);
    // A door in the north wall (y = 4).
    let north_wall = walls
        .iter()
        .copied()
        .find(|w| {
            let p = run(&mut s, "element_get", json!({"id": w}))["params"].clone();
            let y = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
            (y(&p["start"][1]) - 4.0).abs() < 1e-6 && (y(&p["end"][1]) - 4.0).abs() < 1e-6
        })
        .unwrap();
    let door = run(&mut s, "door_create", json!({"wall": north_wall}))["id"].as_u64().unwrap();

    let out = run(&mut s, "interior_elevations_create", json!({"room": room}));
    let v = views(&out);
    let names: Vec<(&str, &str)> = v.iter().map(|(side, _, name)| (side.as_str(), name.as_str())).collect();
    assert_eq!(
        names,
        [("north", "Kitchen 1"), ("east", "Kitchen 2"), ("south", "Kitchen 3"), ("west", "Kitchen 4")],
        "numbered clockwise from north"
    );

    // The north view sees the north wall's door; the south one does not (it is behind the marker).
    let (north_ids, north_ext) = drawn(&mut s, view_of(&out, "north"));
    assert!(north_ids.contains(&door), "{north_ids:?}");
    let (south_ids, _) = drawn(&mut s, view_of(&out, "south"));
    assert!(!south_ids.contains(&door), "{south_ids:?}");

    // Across, the north view is cropped to the inner faces of the side walls
    // (x from the marker at x = 3, looking north: right is +x).
    let (lo, hi) = north_ext.unwrap();
    let half = 3.0 - t / 2.0;
    assert!(lo.x >= -half - 1e-6 && hi.x <= half + 1e-6, "{lo} {hi} (half width {half})");
    assert!(hi.x - lo.x > 2.0 * half - 0.05, "it reaches both side walls: {lo} {hi}");
    // Up, to the room's height at most.
    let height = run(&mut s, "element_get", json!({"id": room}))["params"]["height"].as_f64().unwrap();
    assert!(lo.y >= -1e-6 && hi.y <= height + 1e-6, "{lo} {hi} (height {height})");

    // The east and west views are as wide as the room is deep.
    let (_, east_ext) = drawn(&mut s, view_of(&out, "east"));
    let (lo, hi) = east_ext.unwrap();
    assert!(hi.x - lo.x <= 4.0 - t + 1e-6 && hi.x - lo.x > 4.0 - t - 0.05, "{lo} {hi}");

    // element_get lists the marker's views.
    let marker = out["marker"].as_u64().unwrap();
    let got = run(&mut s, "element_get", json!({"id": marker}));
    assert_eq!(got["derived"]["views"].as_array().unwrap().len(), 4);
}

#[test]
fn the_views_follow_the_walls_the_marker_and_the_room() {
    let mut s = Session::default();
    let (_, room) = kitchen(&mut s);
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "sides": ["north"]}));
    assert_eq!(views(&out).len(), 1);
    let north = view_of(&out, "north");
    let width = |s: &mut Session| {
        let (_, ext) = drawn(s, north);
        let (lo, hi) = ext.unwrap();
        hi.x - lo.x
    };
    let before = width(&mut s);
    // A partition at x = 4.5 closes the space nearer: the view narrows with it.
    run(&mut s, "wall_create", json!({"start": [4.5, 0.0], "end": [4.5, 4.0]}));
    let after = width(&mut s);
    assert!((before - after - 1.5).abs() < 0.05, "{before} -> {after}");

    // The marker moves: the view's projection follows it.
    let marker = out["marker"].as_u64().unwrap();
    run(&mut s, "elements_move", json!({"ids": [marker], "delta": [1.0, 0.0]}));
    let kind = &s.project().view(north).unwrap().kind;
    let cc_core::ViewKind::InteriorElevation { origin, .. } = kind else { panic!("{kind:?}") };
    assert!((origin.x - 4.0).abs() < 1e-9, "{origin}");

    // Sides: add east and west, drop north; numbers are reused from 1.
    let out = run(&mut s, "interior_elevation_sides_set", json!({"marker": marker, "sides": ["east", "west"]}));
    let names: Vec<String> = views(&out).into_iter().map(|(.., n)| n).collect();
    assert_eq!(names, ["Kitchen 1", "Kitchen 2"], "the north view's number 1 is reused");
    assert!(s.project().view(north).is_none(), "the north view is gone");

    // Deleting the room keeps the marker and its views, with no room.
    run(&mut s, "elements_delete", json!({"ids": [room]}));
    let got = run(&mut s, "element_get", json!({"id": marker}));
    assert!(got["params"]["room"].is_null(), "{got}");
    assert_eq!(got["derived"]["views"].as_array().unwrap().len(), 2);
    // Deleting the marker deletes its views, in one undo step.
    let ids: Vec<u64> = views(&out).into_iter().map(|(_, v, _)| v).collect();
    run(&mut s, "elements_delete", json!({"ids": [marker]}));
    assert!(ids.iter().all(|v| s.project().view(ViewId(*v)).is_none()));
    run(&mut s, "undo", json!({}));
    assert!(ids.iter().all(|v| s.project().view(ViewId(*v)).is_some()));
}

#[test]
fn a_ceiling_lowers_the_crop_and_a_turned_marker_faces_slanted_walls() {
    let mut s = Session::default();
    let (_, room) = kitchen(&mut s);
    let ceiling = json!({"kind": "rectangle", "corner": [0, 0], "opposite": [6, 4]});
    run(&mut s, "ceiling_create", json!({"curves": [ceiling], "offset": 2.4}));
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "sides": ["north"]}));
    let (_, ext) = drawn(&mut s, view_of(&out, "north"));
    let (_, hi) = ext.unwrap();
    assert!(hi.y <= 2.4 + 1e-6, "cropped at the ceiling: {hi}");

    // A marker with no room, turned 90°: its north side looks west.
    let out =
        run(&mut s, "interior_elevations_create", json!({"position": [2.0, 2.0], "sides": ["north"], "rotation": 90}));
    let view = view_of(&out, "north");
    let cc_core::ViewKind::InteriorElevation { look, .. } = s.project().view(view).unwrap().kind else { panic!() };
    assert!((look - DVec2::NEG_X).length() < 1e-9, "{look}");
    assert!(views(&out)[0].2.ends_with(" 1"), "named generically and numbered: {:?}", views(&out));

    // Outside every space there is nothing to look at.
    assert_eq!(fails(&mut s, "interior_elevations_create", json!({"position": [40.0, 40.0]})), ErrorCode::Rejected);
}

#[test]
fn files_with_dangling_interior_elevations_are_repaired() {
    let mut s = Session::default();
    let (_, room) = kitchen(&mut s);
    let out = run(&mut s, "interior_elevations_create", json!({"room": room}));
    let marker = cc_core::ElementId(out["marker"].as_u64().unwrap());
    let mut p = s.project().clone();
    p.elements.remove(&marker);
    let fixed = p.repair();
    assert!(fixed >= 4, "the four views of the missing marker go: {fixed}");
    assert!(!p.views.values().any(|v| matches!(v.kind, cc_core::ViewKind::InteriorElevation { .. })));
}

#[test]
fn an_l_shaped_room_keeps_its_whole_extent_in_each_direction() {
    let mut s = Session::default();
    let pts = [[0.0, 0.0], [6.0, 0.0], [6.0, 2.0], [3.0, 2.0], [3.0, 5.0], [0.0, 5.0]];
    let curves: Vec<Value> =
        (0..pts.len()).map(|i| json!({"kind": "line", "start": pts[i], "end": pts[(i + 1) % pts.len()]})).collect();
    let walls = run(&mut s, "walls_create", json!({"curves": curves}))["ids"].clone();
    let t = {
        let p = s.project();
        let e = p.element(cc_core::ElementId(walls[0].as_u64().unwrap())).unwrap();
        p.types.get(&e.type_id).unwrap().thickness()
    };
    let room = run(&mut s, "room_create", json!({"point": [1.5, 1.0], "name": "Hall"}))["id"].as_u64().unwrap();
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "sides": ["north", "east"]}));
    // Looking north from (1.5, 1): across, the whole 6 m arm.
    let (_, ext) = drawn(&mut s, view_of(&out, "north"));
    let (lo, hi) = ext.unwrap();
    assert!((hi.x - lo.x - (6.0 - t)).abs() < 0.05, "{lo} {hi}");
    // Looking east: across, the whole 5 m arm.
    let (_, ext) = drawn(&mut s, view_of(&out, "east"));
    let (lo, hi) = ext.unwrap();
    assert!((hi.x - lo.x - (5.0 - t)).abs() < 0.05, "{lo} {hi}");
}

#[test]
fn plans_draw_the_marker_with_an_arrow_per_view() {
    let mut s = Session::default();
    let (_, room) = kitchen(&mut s);
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "sides": ["north", "south"]}));
    let marker = out["marker"].as_u64().unwrap();
    let plans = run(&mut s, "views_list", json!({"category": "floor_plan"}));
    let plan = ViewId(plans[0]["id"].as_u64().unwrap());
    let d = s.drawing(plan).unwrap();
    // The circle and one filled arrow per view belong to the marker (to pick and move it).
    let fills = d.prims.iter().filter(|p| matches!(p, Prim::Fill { element: Some(e), .. } if e.0 == marker)).count();
    assert_eq!(fills, 3, "the circle and two arrows");
    let texts: Vec<String> = d
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"1".to_string()) && texts.contains(&"2".to_string()), "{texts:?}");
}
