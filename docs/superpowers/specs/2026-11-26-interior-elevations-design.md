# Interior elevations of rooms (issue #477)

Issue #477 asks for interior elevations of a room, "with four side
option". The user (2026-11-26) added that the marker in plan should let
one choose which of its views are shown, and that one elevation per wall
of the room is worth having besides the four sides, and asked that the
behaviour follow what desktop BIM tools commonly do with interior
elevations (their public documentation; no product's symbols or
wording). This file keeps what exists, the principles and the plan.

An interior elevation is the view of a room's wall seen from inside the
room: kitchens, bathrooms, tiling layouts, joinery. A marker in the room
points at the walls shown; each view is cropped to the room, between its
side walls and from the floor to the room's height.

## What exists

- **Elevations** (`ViewKind::Elevation { direction }`) are exterior only:
  the whole model seen from one of four sides, with no position or depth.
  Their markers are drawn in plans by a view mark type
  (`cc_core::view_mark`, `MarkKind::Elevation`, `MarkStyle`), moved per
  plan with `View::mark_offsets`, and their texts take `{Number}`,
  `{Sheet}`, `{Name}`, `{Letter}`.
- **Sections** (`ViewKind::Section { start, end, depth }`) already cut
  along a line, look to its right and stop at a depth;
  `cc_views::projected` draws both.
- **Crop** (`View::crop`, `CropBox`, `callout::crop`) clips a section or an
  elevation to a rectangle of its drawing.
- **Rooms** (`ElementKind::Room`: `position`, `number`, `name`, `height`)
  have a derived boundary: `cc_geom::rooms::room_boundary` finds the
  polygon around the room's point among the walls. It does not say which
  wall each edge belongs to.

## Principles

- **The view is a query over the room.** An interior elevation stores the
  room and the face it shows, never a copied line, depth or crop. Where it
  cuts, how deep it looks and how it is cropped are worked out from the
  room's current boundary each time the view is generated, so moving a
  wall or changing the room's height updates the views. Nothing is edited
  by hand that the room already says (see issue #475).
- **Engine first.** Creating, choosing faces and deleting are `cc_engine`
  commands with tests in `crates/cc_engine/tests`; the app calls them.
- **Reuse.** The projection, the crop and the mark types of sections and
  elevations; no second renderer.
- **General conventions only**: a marker with one arrow per view, arrows
  switched on and off from the marker's properties. No product's symbols
  or wording.

## Model

- **`ElementKind::InteriorMarker(InteriorMarker)`**, category
  `InteriorMarker` (in plans only, never in 3D output or IFC):
  `position: DVec2`, `room: Option<ElementId>` (the room it was placed in,
  which names the views and gives their height; none in a space with no
  room, such as a corridor), `mode: FaceMode`. The space the marker looks
  around is the boundary of walls (and room separation lines) around its
  position, found as rooms find theirs, so it works with or without a
  room element.
  - `FaceMode::Directions { rotation }`: up to four faces at right angles
    (north, east, south and west of the marker), one arrow each. The
    marker turns as a whole (`rotation`, degrees) to face slanted walls.
    An L-shaped space still has the four: each view takes the space's
    extent in that direction. The cut plane of each view passes through
    the marker.
  - `FaceMode::Walls { distance, min_length }`: one face per edge of the
    boundary at least `min_length` long (default 0.30 m; shorter edges
    such as a column's face are left out), each keeping its wall (the
    views are numbered, see *Decisions*). Each view's cut plane stands
    `distance` in front of its wall (default 1.00 m, never past the middle
    of the space), so it shows the wall and what stands against it. A
    curved wall is one face, its view on the chord.
- **`ViewKind::InteriorElevation { marker: ElementId, face: Face }`**,
  browser category *Elevations* (a subgroup *Interior*), where
  `Face::Direction(Direction)` or `Face::Wall(ElementId)`. A wall face
  keeps the wall's id, not an edge index, so it survives other edits; if
  the wall no longer bounds the room the view says so and draws nothing
  until the wall is back or the face is removed.
- **Derived each generation** from the space's boundary (`cc_views`, with
  the walls' faces from `cc_geom`):
  - the cut plane: parallel to the face, through the marker (directions)
    or `distance` in front of the wall (walls); what is behind it is not
    shown;
  - the looking direction: towards the face;
  - the depth: up to the wall's far face, so the wall's finish, openings
    and what is fixed to it show, and nothing behind it;
  - the crop, snapped to the space: across, between the boundary's
    extents along the face (the side walls' inner faces); vertically,
    from the floor of the level to the ceiling over the space if there is
    one, else to the underside of the floor or roof above, else to the
    room's height (or the level above when there is no room);
  - a margin around the crop for annotation, set on the view.
- **Automatic extents.** Each view has `auto_extents: bool` (default on):
  on, the crop follows the space as above; off, the view keeps a crop the
  user drew (`View::crop`), as for sections. This is a choice of the
  drawing, not of the building's data.
- `cc_geom::rooms` gains a boundary with the source of each edge (wall id
  and face, or room separation line), which `Walls` mode needs.
- **Marks**: each view's arrow is drawn by its mark type, with the same
  texts as elevations (`{Number}` and `{Sheet}` once on a sheet); clicking
  an arrow in the app opens its view.

## Commands

- `interior_elevations_create { position | room, mode, faces, rotation,
  distance, min_length }`: the marker (at `position`, or at the point of
  `room`) and a view per face picked (default: all four directions), one
  undo step; returns the marker and the views. Refused where no walls
  close a space around the position.
- `interior_elevation_faces_set { marker, faces }`: the faces shown; adds
  the views of new faces and deletes those of faces taken away (their
  sheets' viewports go with them, as when a view is deleted).
- `element_get` of a marker lists its available faces and the view of
  each; `element_set` moves or turns it or changes its mode and options
  (views of faces that no longer exist are deleted).
- `view_set` takes `auto_extents`.
- Deleting the marker deletes its views (the app warns first, naming
  them). Deleting the room keeps the markers and views: they go on from
  the walls, and the views keep the names they had. `Project::repair`
  clears a marker's dangling room and drops interior views whose marker
  is gone.

## Stages (one pull request each)

1. **Directions.** The model, `FaceMode::Directions` with rotation, the
   derived cut, depth and crop snapped to the space (floor, ceiling or
   structure above), `auto_extents`, the views in the browser, the
   commands. Plans draw the marker with its arrows. Tests: a rectangular
   room gives four views showing each wall's face and its doors; an
   L-shaped room; a turned marker in a room with slanted walls; moving a
   wall moves the crop; a ceiling lowers the crop; a marker with no room;
   deleting the room keeps the views.
2. **Marker in the app.** *View → Interior Elevation*: a click in a space
   places the marker with its four views; right-click on a room does the
   same at the room's point. The marker's properties have a checkbox per
   face (`interior_elevation_faces_set`), rotation and mode; the marker
   drags and turns with grips; double-click on an arrow opens its view; the
   view's properties have *Automatic extents*.
3. **One view per wall.** The boundary with its sources, `FaceMode::Walls`
   with `distance` and `min_length`, the views numbered clockwise from
   the boundary's first wall, the marker drawing an arrow in front of
   each wall pointing at it. Tests: short edges left out; the cut plane
   never past the middle of a narrow space.
4. **Finishing touches.** The floor and ceiling lines of the space, a view
   template for interior elevations in the project templates, and the
   views in schedules of views and sheets.

## Decisions

Following what desktop BIM tools commonly do (2026-11-26):

- **Cut plane**: at the marker for the four directions, where the arrow
  stands; at a set distance in front of each wall for one view per wall.
  Nothing between the viewer and the cut plane is shown.
- **Short edges**: left out below a minimum length set on the marker.
- **Vertical crop**: snapped to the floor and to the ceiling over the
  space, else the structure above; the user may turn the automatic
  extents off for a view and crop it by hand.
- **Without a room**: allowed; the marker finds the walls around it and
  the views take the level above as their top.
- **Turning the marker**: the four arrows turn together to face slanted
  walls.

- **Names**: numbered per marker, the room's name and a number
  ("Kitchen 1", "Kitchen 2"…; "Interior Elevation 1"… with no room),
  clockwise from the first arrow (north for directions, the first wall
  of the boundary for walls). Short, so the arrows' texts fit in a small
  room; a number freed by a face taken away is reused by the next one
  added. The mark texts give the drawing number once on a sheet.
