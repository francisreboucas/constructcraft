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
    let out = run(&mut s, "interior_elevation_faces_set", json!({"marker": marker, "sides": ["east", "west"]}));
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

/// The walls `element_get` lists for a marker, clockwise from north.
fn marker_walls(s: &mut Session, marker: u64) -> Vec<u64> {
    run(s, "element_get", json!({"id": marker}))["derived"]["walls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["wall"].as_u64().unwrap())
        .collect()
}

/// The wall of the kitchen whose line lies at `y` (horizontal) or `x` (vertical).
fn wall_at(s: &mut Session, walls: &[u64], axis: usize, value: f64) -> u64 {
    walls
        .iter()
        .copied()
        .find(|w| {
            let p = run(s, "element_get", json!({"id": w}))["params"].clone();
            let c = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
            (c(&p["start"][axis]) - value).abs() < 1e-6 && (c(&p["end"][axis]) - value).abs() < 1e-6
        })
        .unwrap()
}

#[test]
fn the_walls_mode_makes_a_view_per_wall_cut_in_front_of_it() {
    let mut s = Session::default();
    let (walls, room) = kitchen(&mut s);
    let t = wall_thickness(&mut s, walls[0]);
    let (north, east, south, west) = (
        wall_at(&mut s, &walls, 1, 4.0),
        wall_at(&mut s, &walls, 0, 6.0),
        wall_at(&mut s, &walls, 1, 0.0),
        wall_at(&mut s, &walls, 0, 0.0),
    );
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "mode": "walls"}));
    let marker = out["marker"].as_u64().unwrap();
    // One view per wall, numbered clockwise from north.
    let order: Vec<u64> = out["views"].as_array().unwrap().iter().map(|v| v["wall"].as_u64().unwrap()).collect();
    assert_eq!(order, [north, east, south, west]);
    assert_eq!(marker_walls(&mut s, marker), order);
    let names: Vec<&str> = out["views"].as_array().unwrap().iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Kitchen 1", "Kitchen 2", "Kitchen 3", "Kitchen 4"]);

    // The north wall's view is cut 1 m in front of its inner face (y = 4 - t/2), looking north.
    let view = ViewId(out["views"][0]["view"].as_u64().unwrap());
    let cc_core::ViewKind::InteriorElevation { origin, look, .. } = s.project().view(view).unwrap().kind else {
        panic!()
    };
    assert!((look - DVec2::Y).length() < 1e-9, "{look}");
    assert!((origin - DVec2::new(3.0, 4.0 - t / 2.0 - 1.0)).length() < 1e-9, "{origin}");
    // It shows the wall's stretch of the room: 6 m less the side walls.
    let (ids, ext) = drawn(&mut s, view);
    assert!(ids.contains(&north), "{ids:?}");
    let (lo, hi) = ext.unwrap();
    assert!((hi.x - lo.x - (6.0 - t)).abs() < 0.05, "{lo} {hi}");

    // A distance larger than the marker's own puts the cut plane through the marker, not behind it.
    run(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "walls", "distance": 5.0}));
    let cc_core::ViewKind::InteriorElevation { origin, .. } = s.project().view(view).unwrap().kind else { panic!() };
    assert!((origin - DVec2::new(3.0, 2.0)).length() < 1e-9, "never behind the marker: {origin}");

    // Faces: keep only the east and west walls; then back to directions, with its four sides.
    let out = run(&mut s, "interior_elevation_faces_set", json!({"marker": marker, "walls": [east, west]}));
    assert_eq!(out["views"].as_array().unwrap().len(), 2);
    assert_eq!(
        fails(&mut s, "interior_elevation_faces_set", json!({"marker": marker, "sides": ["north"]})),
        ErrorCode::InvalidArguments
    );
    let out = run(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "directions"}));
    let sides: Vec<&str> = out["views"].as_array().unwrap().iter().map(|v| v["side"].as_str().unwrap()).collect();
    assert_eq!(sides, ["north", "east", "south", "west"]);
    assert!(s.project().view(view).is_none(), "the walls' views went with the mode");
}

/// Walls along the closed outline `pts`, and a room named Hall at `at`.
fn outline_room(s: &mut Session, pts: &[[f64; 2]], at: [f64; 2]) -> (Vec<u64>, u64) {
    let curves: Vec<Value> =
        (0..pts.len()).map(|i| json!({"kind": "line", "start": pts[i], "end": pts[(i + 1) % pts.len()]})).collect();
    let walls: Vec<u64> = run(s, "walls_create", json!({"curves": curves}))["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let room = run(s, "room_create", json!({"point": at, "name": "Hall"}))["id"].as_u64().unwrap();
    (walls, room)
}

fn view_kind(s: &Session, view: u64) -> (DVec2, DVec2) {
    let cc_core::ViewKind::InteriorElevation { origin, look, .. } = s.project().view(ViewId(view)).unwrap().kind else {
        panic!("an interior elevation")
    };
    (origin, look)
}

#[test]
fn short_walls_are_left_out_and_a_deleted_wall_takes_its_view() {
    let mut s = Session::default();
    // A room with a 0.25 m return in its north side: six walls, one of them short.
    let pts = [[0.0, 0.0], [6.0, 0.0], [6.0, 4.0], [3.0, 4.0], [3.0, 4.25], [0.0, 4.25]];
    let (walls, room) = outline_room(&mut s, &pts, [3.0, 2.0]);
    let short = walls[3];
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "mode": "walls"}));
    let marker = out["marker"].as_u64().unwrap();
    // The default minimum length (0.30 m) leaves the return out.
    let shown = marker_walls(&mut s, marker);
    assert_eq!(shown.len(), 5, "{shown:?}");
    assert!(!shown.contains(&short));
    assert_eq!(out["views"].as_array().unwrap().len(), 5);
    // A lower one lets it in (to be added with faces_set); the views stay.
    let out = run(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "walls", "min_length": 0.1}));
    assert_eq!(out["views"].as_array().unwrap().len(), 5);
    assert!(marker_walls(&mut s, marker).contains(&short));

    // With a minimum length of 5 m only the long south wall stays; one larger than
    // every wall is refused, so the marker never loses its last view.
    let out = run(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "walls", "min_length": 5.0}));
    assert_eq!(out["views"].as_array().unwrap().len(), 1, "{out}");
    assert_eq!(
        fails(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "walls", "min_length": 50.0})),
        ErrorCode::Rejected
    );
    assert_eq!(run(&mut s, "element_get", json!({"id": marker}))["derived"]["views"].as_array().unwrap().len(), 1);
    let south = walls[0];
    let view = out["views"][0]["view"].as_u64().unwrap();
    assert_eq!(out["views"][0]["wall"].as_u64(), Some(south));
    // Deleting the wall deletes its view; undo brings both back.
    run(&mut s, "elements_delete", json!({"ids": [south]}));
    assert!(s.project().view(ViewId(view)).is_none());
    run(&mut s, "undo", json!({}));
    assert!(s.project().view(ViewId(view)).is_some());
}

/// In an L-shaped room the wall of the re-entrant corner is seen from
/// inside the room, whichever side of its line the marker stands on.
#[test]
fn a_wall_is_seen_from_the_space_it_bounds() {
    let mut s = Session::default();
    // [0,6]x[0,3] and [0,3]x[3,6]: the wall on x = 3 for y in 3..6 bounds the upper arm on its west face.
    let pts = [[0.0, 0.0], [6.0, 0.0], [6.0, 3.0], [3.0, 3.0], [3.0, 6.0], [0.0, 6.0]];
    let (walls, _) = outline_room(&mut s, &pts, [1.5, 1.5]);
    let inner = walls[3];
    // The marker stands east of that wall's line, in the lower arm.
    let out =
        run(&mut s, "interior_elevations_create", json!({"position": [4.5, 1.5], "mode": "walls", "walls": [inner]}));
    let view = out["views"][0]["view"].as_u64().unwrap();
    let (origin, look) = view_kind(&s, view);
    assert!((look - DVec2::X).length() < 1e-9, "it looks east at the wall's west face: {look}");
    assert!(origin.x < 3.0, "from inside the upper arm: {origin}");
    let (ids, ext) = drawn(&mut s, ViewId(view));
    assert!(ids.contains(&inner) && ext.is_some(), "{ids:?}");
}

/// A round room of two half-circle walls: both are shown from the middle.
#[test]
fn curved_walls_are_seen_from_their_hollow_side() {
    let mut s = Session::default();
    let circle = json!({"kind": "circle", "center": [0, 0], "radius": 3});
    let walls = run(&mut s, "walls_create", json!({"curves": [circle]}))["ids"].clone();
    assert_eq!(walls.as_array().unwrap().len(), 2, "two half circles");
    let out = run(&mut s, "interior_elevations_create", json!({"position": [0.0, 0.0], "mode": "walls"}));
    let views = out["views"].as_array().unwrap();
    assert_eq!(views.len(), 2, "{out}");
    for v in views {
        let view = v["view"].as_u64().unwrap();
        let (origin, _) = view_kind(&s, view);
        assert!(origin.length() < 3.0, "cut inside the room: {origin}");
        let (ids, ext) = drawn(&mut s, ViewId(view));
        assert!(ids.contains(&v["wall"].as_u64().unwrap()) && ext.is_some(), "{ids:?}");
    }
}

/// A wall that stops bounding the space keeps its view, which says why it
/// is empty; the marker's other faces can still be changed.
#[test]
fn a_wall_out_of_the_space_keeps_its_view() {
    let mut s = Session::default();
    let (walls, room) = kitchen(&mut s);
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "mode": "walls"}));
    let marker = out["marker"].as_u64().unwrap();
    let east = wall_at(&mut s, &walls, 0, 6.0);
    let east_view = out["views"].as_array().unwrap().iter().find(|v| v["wall"].as_u64() == Some(east)).unwrap()["view"]
        .as_u64()
        .unwrap();
    // A partition closes the space short of the east wall.
    let partition = run(&mut s, "wall_create", json!({"start": [4.5, 0.0], "end": [4.5, 4.0]}))["id"].as_u64().unwrap();
    assert!(s.project().view(ViewId(east_view)).is_some(), "the view stays");
    let d = s.drawing(ViewId(east_view)).unwrap();
    assert!(d.prims.iter().all(|p| p.element().is_none()), "nothing of the model is drawn");
    assert!(d.prims.iter().any(|p| matches!(p, Prim::Text { .. })), "it says why");
    // The other faces can still be set, the stale one kept or dropped.
    let keep: Vec<u64> = vec![east, partition];
    let out = run(&mut s, "interior_elevation_faces_set", json!({"marker": marker, "walls": keep}));
    let shown: Vec<u64> = out["views"].as_array().unwrap().iter().map(|v| v["wall"].as_u64().unwrap()).collect();
    assert!(shown.contains(&east) && shown.contains(&partition) && shown.len() == 2, "{shown:?}");
    // Changing the distance keeps it too.
    run(&mut s, "interior_elevation_mode_set", json!({"marker": marker, "mode": "walls", "distance": 0.5}));
    assert!(s.project().view(ViewId(east_view)).is_some());
}

/// `element_set` changing a marker's mode deletes the views of the other
/// kind; a file of format 36 (a view's side written alone) still reads.
#[test]
fn modes_set_as_data_drop_stray_views_and_old_files_read() {
    let mut s = Session::default();
    let (_, room) = kitchen(&mut s);
    let out = run(&mut s, "interior_elevations_create", json!({"room": room}));
    let marker = out["marker"].as_u64().unwrap();
    // A file as stage 1 wrote it: `side: North`, no `left`, format 36.
    let text = cc_core::to_ron(s.project()).unwrap();
    assert!(text.contains("face: Side(North)"), "the new form");
    let old = text
        .replace("face: Side(North)", "side: North")
        .replace("face: Side(East)", "side: East")
        .replace("face: Side(South)", "side: South")
        .replace("face: Side(West)", "side: West")
        .replace(&format!("format_version: {}", cc_core::FORMAT_VERSION), "format_version: 36");
    let (read, repairs) = cc_core::from_ron_with_repairs(&old).unwrap();
    assert_eq!(repairs, 0);
    assert_eq!(read.marker_faces(cc_core::ElementId(marker)).len(), 4);

    let walls_mode = json!({"mode": {"Walls": {"distance": 1.0, "min_length": 0.3}}});
    run(&mut s, "element_set", json!({"id": marker, "params": walls_mode}));
    assert!(s.project().marker_faces(cc_core::ElementId(marker)).is_empty(), "the sides' views went");
}

/// An interior elevation draws the floor and the ceiling of its space as
/// cut lines across what it shows, and follows a view template like the
/// other elevations.
#[test]
fn interior_elevations_draw_their_floor_and_ceiling_and_take_templates() {
    let mut s = Session::default();
    let (walls, room) = kitchen(&mut s);
    let t = wall_thickness(&mut s, walls[0]);
    let ceiling = json!({"kind": "rectangle", "corner": [0, 0], "opposite": [6, 4]});
    run(&mut s, "ceiling_create", json!({"curves": [ceiling], "offset": 2.4}));
    let out = run(&mut s, "interior_elevations_create", json!({"room": room, "sides": ["north"]}));
    let view = view_of(&out, "north");
    let d = s.drawing(view).unwrap();
    // Two cut lines of no element, as wide as the space: at the level and at the ceiling.
    let mut heights: Vec<f64> = d
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Line { a, b, style: cc_views::LineStyle::Cut, element: None, .. }
                if (a.y - b.y).abs() < 1e-9 && ((a.x - b.x).abs() - (6.0 - t)).abs() < 0.05 =>
            {
                Some(a.y)
            }
            _ => None,
        })
        .collect();
    heights.sort_by(f64::total_cmp);
    assert_eq!(heights.len(), 2, "{heights:?}");
    assert!(heights[0].abs() < 1e-6 && (heights[1] - 2.4).abs() < 1e-6, "{heights:?}");

    // With the automatic extents off the crop is the user's: no space lines.
    run(
        &mut s,
        "view_set",
        json!({"view": view.0, "params": {"auto_extents": false}, "crop": [[-2.0, 0.0], [2.0, 2.0]]}),
    );
    let d = s.drawing(view).unwrap();
    assert!(!d.prims.iter().any(|p| matches!(p, Prim::Line { style: cc_views::LineStyle::Cut, element: None, .. })));

    // A view template applies: its scale becomes the view's.
    let template = run(&mut s, "template_create", json!({"name": "Interiors"}))["id"].as_u64().unwrap();
    run(&mut s, "template_set", json!({"template": template, "scale": 20}));
    run(&mut s, "view_template_assign", json!({"views": [view.0], "template": template}));
    assert_eq!(s.project().view(view).unwrap().scale, 20);
}
