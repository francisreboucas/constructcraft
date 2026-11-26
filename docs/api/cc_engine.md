# `cc_engine` API

{% raw %}
[← API reference](index.md) · Generated from the source by `cargo run -p api_doc`; edit the doc comments, not this page.

The ConstructCraft engine: every user-facing operation as a named command
that runs on a `Session` without a GUI.
A command is a struct of typed parameters (deserialized from JSON, with a
JSON Schema generated from the same type) and a `run` method. The
`catalog` lists them; `Session::execute` runs one by name. The
command line (`ccad`), the MCP server and, area by area, the desktop app
all go through this one catalog, so an operation behaves the same
wherever it is called from.
Lengths are in meters, angles in degrees, points are `[x, y]` in plan.
Edits go through `Document::edit`, so each command is one undo step.
Levels, types, views and materials can be named by id or by name;
elements by id.

## Modules

| Module | Items | Summary |
|---|---|---|
| [`cc_engine`](#mod-cc_engine) | 5 | The ConstructCraft engine: every user-facing operation as a named command that runs on a `Session` without a GUI. |
| [`cc_engine::commands`](#mod-cc_engine-commands) | 10 | The commands, by area. |
| [`cc_engine::commands::analytical`](#mod-cc_engine-commands-analytical) | 2 | The analytical model of the structure, for structural analysis programs. |
| [`cc_engine::commands::annotate`](#mod-cc_engine-commands-annotate) | 9 | Dimensions: place (associated with what they measure) and drive by value. |
| [`cc_engine::commands::bar_marks`](#mod-cc_engine-commands-bar_marks) | 13 | Bar marks, bending schedules, bar tags and the check of bar lengths (see `cc_core::bar_marks`, `cc_core::bar_tag`, `cc_geom::bar_marks` and `cc_views::bending`). |
| [`cc_engine::commands::bracing`](#mod-cc_engine-commands-bracing) | 4 | Parametric bracing: `bracing_create` makes a host that keeps the braces of one bay of a frame in step with its pattern, storeys and brace type. |
| [`cc_engine::commands::browser`](#mod-cc_engine-commands-browser) | 7 | Project browser organization (see `cc_core::browser`): named schemes that group the browser's views by view type, level, discipline or phase. |
| [`cc_engine::commands::cable_tray`](#mod-cc_engine-commands-cable_tray) | 5 | Cable trays: open rectangular runs carrying electrical circuits, joined like conduits (see `cc_core::mep`, `cc_geom::mep_net`). |
| [`cc_engine::commands::cabling`](#mod-cc_engine-commands-cabling) | 3 | Structured cabling: racks, data outlets and the horizontal cables between them. |
| [`cc_engine::commands::cad`](#mod-cc_engine-commands-cad) | 11 | Imported CAD drawings (DWG, DXF and SVG underlays, and pictures): importing a file into a 2D view and showing or hiding its layers. |
| [`cc_engine::commands::cad_explode`](#mod-cc_engine-commands-cad_explode) | 3 | Exploding an imported drawing (`cad_explode`): its visible primitives become native, editable elements of its view in one undo step. |
| [`cc_engine::commands::callout`](#mod-cc_engine-commands-callout) | 7 | Callouts and detail views (see `cc_core::callout`). |
| [`cc_engine::commands::component`](#mod-cc_engine-commands-component) | 18 | Editing a component's definition piece by piece: parameters, reference planes and lines, and dimensions between planes that parameters drive (see `cc_core::component_ref`). |
| [`cc_engine::commands::component_connector`](#mod-cc_engine-commands-component_connector) | 5 | Connectors of a component: the points where pipes and conduits join it (a toilet's water inlet and sewer outlet, an outlet's electrical connection), each with the kind of system it takes, a position and a diameter as expressions, and the direction runs leave it. |
| [`cc_engine::commands::component_piece`](#mod-cc_engine-commands-component_piece) | 9 | Pieces of a component drawn as sketches: an outline on the plan, front or side plane extruded along the plane's normal, revolved about an axis in it, swept along a path or blended into a second outline; solid or void. |
| [`cc_engine::commands::component_script`](#mod-cc_engine-commands-component_script) | 5 | Scripted components: writing a script piece (its declarations become the component's parameters and hotspots), reading and setting parameter values of a type or of placed elements, and moving hotspots. |
| [`cc_engine::commands::connection`](#mod-cc_engine-commands-connection) | 5 | Steel and timber connections (see `cc_core::connection`): placing them on member ends, the checks `element_set` runs on them and the values `element_get` derives. |
| [`cc_engine::commands::create`](#mod-cc_engine-commands-create) | 31 | Creating elements, with the same defaults as the drawing tools. |
| [`cc_engine::commands::curtain`](#mod-cc_engine-commands-curtain) | 7 | Curtain walls: walls of a type with a curtain grid (see `cc_core::curtain`). |
| [`cc_engine::commands::design_options`](#mod-cc_engine-commands-design_options) | 15 | Design options: option sets, their options and primary option, the option elements belong to, the option each view shows and the option being edited (see `cc_core::design_option`). |
| [`cc_engine::commands::detail`](#mod-cc_engine-commands-detail) | 5 | Detail lines: 2D lines, arcs, circles and ellipses drawn in one view (see `cc_core::detail`). |
| [`cc_engine::commands::detail_component`](#mod-cc_engine-commands-detail_component) | 7 | Detail components: reusable 2D components placed in one view (see `cc_core::detail_component`). |
| [`cc_engine::commands::devices`](#mod-cc_engine-commands-devices) | 7 | Device types the engine provides on demand: a project that has no type for a kind of device (a data outlet, a rack, an air terminal, an earth electrode…) gets the built-in one with the first device placed, in the same undo step (see `cc_core::component::device_library`). |
| [`cc_engine::commands::doors`](#mod-cc_engine-commands-doors) | 1 | Door types: swing doors of one or two leaves and French doors (glazed, from the floor), built from the definitions of `cc_core::openings`. |
| [`cc_engine::commands::edit`](#mod-cc_engine-commands-edit) | 4 | Changing element parameters (the Properties panel). |
| [`cc_engine::commands::fill_patterns`](#mod-cc_engine-commands-fill_patterns) | 13 | Fill patterns (hatches): the built-in patterns and the project's own (see `cc_core::fill_pattern`), which materials (cut and surface patterns), filled region types and view overrides use. |
| [`cc_engine::commands::foundation`](#mod-cc_engine-commands-foundation) | 6 | Foundations: pads under columns, strips (ground beams) under walls and slabs, placed one by one or under existing columns and walls. |
| [`cc_engine::commands::framing`](#mod-cc_engine-commands-framing) | 8 | Bracing and trusses: sloped members between points in space, braces between columns and trusses laid out from a span, a height and a pattern. |
| [`cc_engine::commands::graph`](#mod-cc_engine-commands-graph) | 8 | Visual programming graphs (see `crate::graph`): run a `.ccgraph`, check it, list the nodes a graph can use, and see or release the elements a graph made. |
| [`cc_engine::commands::graphics`](#mod-cc_engine-commands-graphics) | 28 | View graphics: category visibility and overrides, element overrides and hiding in a view, view filters, view templates and temporary hiding and isolation. |
| [`cc_engine::commands::grid_frame`](#mod-cc_engine-commands-grid_frame) | 3 | A multi-storey grid frame: on every storey of a range of levels, columns at the grid intersections (tops attached to the level above) and beams along the grids on the upper level, plus optional secondary beams, in one undo step. |
| [`cc_engine::commands::io`](#mod-cc_engine-commands-io) | 13 | Files: IFC import/export, SketchUp and OBJ import, sheet PDF/SVG, view PNG/SVG/PDF, schedule CSV, component files. |
| [`cc_engine::commands::level_mark`](#mod-cc_engine-commands-level_mark) | 2 | Level marks: the elevation of a point written in a plan, an elevation or a section (see `cc_core::level_mark`). |
| [`cc_engine::commands::levels`](#mod-cc_engine-commands-levels) | 5 | Levels: create (with their plans, also as a storey above or below another), rename, change elevation or storey height, delete. |
| [`cc_engine::commands::lightning`](#mod-cc_engine-commands-lightning) | 7 | Lightning protection: conductors on the roof, down the walls and in the ground (an earth ring), air terminals on the roof, earth electrodes, and a coverage check of the roofs (see `cc_core::lightning`, `cc_geom::lightning`). |
| [`cc_engine::commands::line_style`](#mod-cc_engine-commands-line_style) | 5 | Line styles: the built-in styles and the project's own (see `cc_core::detail`), used by detail lines and region boundaries. |
| [`cc_engine::commands::mass`](#mod-cc_engine-commands-mass) | 11 | Conceptual masses: creating them (an extrusion, or a loft to another outline) and choosing and reading their mass floors. |
| [`cc_engine::commands::materials`](#mod-cc_engine-commands-materials) | 8 | Materials: create, duplicate, rename, change, delete. |
| [`cc_engine::commands::mep`](#mod-cc_engine-commands-mep) | 21 | Building services: pipes and conduits, plumbing systems and electrical circuits, and what they add up to (lengths, loads, fixture units). |
| [`cc_engine::commands::modify`](#mod-cc_engine-commands-modify) | 25 | Modifying elements: delete, move, copy/array, rotate, mirror, pin, flip, joins, trim/extend, split, offset, align, end points. |
| [`cc_engine::commands::niche`](#mod-cc_engine-commands-niche) | 2 | Niches (issue #304): recesses in one face of a wall, hosted like doors and windows. |
| [`cc_engine::commands::pad`](#mod-cc_engine-commands-pad) | 5 | Building pads: flat areas graded into a terrain, with cut and fill slopes out to the existing ground (see `cc_core::pad`). |
| [`cc_engine::commands::paint`](#mod-cc_engine-commands-paint) | 4 | Painted faces: a material on one surface of an element (see `cc_core::paint` for how a paint finds its face after the model changes). |
| [`cc_engine::commands::phases`](#mod-cc_engine-commands-phases) | 18 | Construction phases: the project's phases, the phases of elements (built, demolished), phase filters and the graphics of each status (see `cc_core::phase`). |
| [`cc_engine::commands::piles`](#mod-cc_engine-commands-piles) | 12 | Piles and pile caps (see `cc_core::pile`): free piles, pile and pile cap types, caps placed one by one or under columns (their piles made with them), and tie beams between caps. |
| [`cc_engine::commands::profiles`](#mod-cc_engine-commands-profiles) | 12 | Structural profiles (see `cc_core::structural_profile`): the named cross-sections column and beam types are drawn and built with. |
| [`cc_engine::commands::project`](#mod-cc_engine-commands-project) | 10 | The session's project: new, open, save, information, undo, units, language. |
| [`cc_engine::commands::query`](#mod-cc_engine-commands-query) | 18 | Reading the model: categories, levels, types, elements, views, materials, schedules. |
| [`cc_engine::commands::ramp`](#mod-cc_engine-commands-ramp) | 6 | Ramps: creating them in any shape and checking them against the accessibility rules of their type. |
| [`cc_engine::commands::rebar`](#mod-cc_engine-commands-rebar) | 6 | Reinforcement: sets of bars in beams and columns, and copying a host's sets to others. |
| [`cc_engine::commands::region`](#mod-cc_engine-commands-region) | 6 | Filled and masking regions: 2D areas drawn in one view (see `cc_core::region`), sketched like floors (`curves`: an outline and the loops inside it as openings). |
| [`cc_engine::commands::render`](#mod-cc_engine-commands-render) | 1 | Photorealistic rendering of 3D views (path tracing, `cc_raytracing`). |
| [`cc_engine::commands::roofing`](#mod-cc_engine-commands-roofing) | 2 | Roof coverings and framing: checking a roof against its tiles. |
| [`cc_engine::commands::schedule`](#mod-cc_engine-commands-schedule) | 2 | Schedule definitions (see `cc_core::schedule`): the fields a schedule shows, its filters, its sorting and grouping, how each column is written and how the table looks on paper. |
| [`cc_engine::commands::shaft`](#mod-cc_engine-commands-shaft) | 3 | Shaft openings: vertical openings through the floors, ceilings and roofs between their base and top (see `cc_core::shaft`). |
| [`cc_engine::commands::sheets`](#mod-cc_engine-commands-sheets) | 14 | Sheets and their title blocks: paper, title block, fields and issue date of a sheet; title block components and their 2D pieces. |
| [`cc_engine::commands::site`](#mod-cc_engine-commands-site) | 4 | The site location (latitude, longitude, elevation, true north and the postal address) and the map data brought in around it (a site drawing and context masses from OpenStreetMap). |
| [`cc_engine::commands::sketch`](#mod-cc_engine-commands-sketch) | 7 | Sketched outlines: the curves of floors, ceilings, roofs and masses (see `cc_core::sketch`), given to the commands that create them as `curves`, read with `sketch_get` and drawn again with `sketch_set`. |
| [`cc_engine::commands::slab_rebar`](#mod-cc_engine-commands-slab_rebar) | 2 | Slab reinforcement: a mesh, or ring and radial bars, in the top and bottom layers of floors and foundation slabs, and trimming bars around their openings. |
| [`cc_engine::commands::stair`](#mod-cc_engine-commands-stair) | 6 | Stairs: creating them in any shape, checking their steps and adding the railings that follow them. |
| [`cc_engine::commands::steel_building`](#mod-cc_engine-commands-steel_building) | 6 | Parametric steel buildings (see `cc_core::steel_building`): the host element and the catalog types its members get by default. |
| [`cc_engine::commands::strip_rebar`](#mod-cc_engine-commands-strip_rebar) | 2 | Strip footing reinforcement: bars across and along the footing slab, and stirrups, side bars and longitudinal bars for the beam on top. |
| [`cc_engine::commands::structure`](#mod-cc_engine-commands-structure) | 11 | Structural layout from grids: columns at grid intersections and beams between the columns along grid lines or in a given order. |
| [`cc_engine::commands::structure_material`](#mod-cc_engine-commands-structure_material) | 9 | Column and beam types by structural material (concrete, steel, timber): the first type of a material, else a default made from the profile catalog (steel `W200x26,6`, timber `MLC 9x32`) or a 20 x 40 concrete rectangle, in one undo step. |
| [`cc_engine::commands::sun`](#mod-cc_engine-commands-sun) | 13 | The sun of a view (see `cc_core::sun`): its settings as commands show and take them (`view_get`, `view_set`), where it stands, the frames of a study, and the project's named presets. |
| [`cc_engine::commands::sweep`](#mod-cc_engine-commands-sweep) | 4 | Swept profiles: a profile (a closed outline held by the type) carried along a path in plan, for baseboards, crown moldings, fascias, gutters and copings. |
| [`cc_engine::commands::topography`](#mod-cc_engine-commands-topography) | 8 | Terrain: topography surfaces from measured points inside a boundary (see `cc_core::topography`). |
| [`cc_engine::commands::types`](#mod-cc_engine-commands-types) | 7 | Types and components: change, duplicate, delete, create components. |
| [`cc_engine::commands::valve`](#mod-cc_engine-commands-valve) | 4 | Valves on pipes: inline accessories hosted by a pipe at a distance along its run (see `cc_core::mep::Valve`). |
| [`cc_engine::commands::views`](#mod-cc_engine-commands-views) | 11 | Views and sheets: create, change, duplicate, delete, place on sheets. |
| [`cc_engine::commands::wall_framing`](#mod-cc_engine-commands-wall_framing) | 5 | Light steel and wood framing of walls (see `cc_core::wall_framing`): the host element and the catalog types its members get by default. |
| [`cc_engine::commands::wall_rebar`](#mod-cc_engine-commands-wall_rebar) | 6 | Wall reinforcement: horizontal (ring) and vertical bars near each face of a wall, and ties between the faces. |
| [`cc_engine::defaults`](#mod-cc_engine-defaults) | 11 | Defaults applied when elements are created (shared by the drawing tools). |
| [`cc_engine::dimensions`](#mod-cc_engine-dimensions) | 3 | Dimensions that follow and drive the model. |
| [`cc_engine::error`](#mod-cc_engine-error) | 3 |  |
| [`cc_engine::extension`](#mod-cc_engine-extension) | 3 | Commands added at run time (plugins). |
| [`cc_engine::gltf`](#mod-cc_engine-gltf) | 3 | glTF 2.0 models (`.glb`, or `.gltf` with its buffers embedded as data URIs or in files next to it) as component types: each node of the scene's root, with everything below it, becomes a type of fixed mesh pieces, one per material, placed by an element at the center of its footprint and the bottom of its extent. |
| [`cc_engine::graph`](#mod-cc_engine-graph) | 15 | Visual programming graphs: nodes joined by wires, evaluated as a dataflow over the engine's commands. |
| [`cc_engine::graph::builtin`](#mod-cc_engine-graph-builtin) | 11 | The built-in nodes: input values, math, lists, geometry (as the JSON values commands take: points `[x, y]` and curves) and element parameters. |
| [`cc_engine::graph::eval`](#mod-cc_engine-graph-eval) | 8 | Running a graph: nodes in dataflow order, lists mapped over, the elements edit nodes create bound to their calls. |
| [`cc_engine::graph::lace`](#mod-cc_engine-graph-lace) | 4 | Lacing: a node given lists where it takes single values runs once per item, and its outputs become lists of the same shape. |
| [`cc_engine::graph::ports`](#mod-cc_engine-graph-ports) | 4 | Ports of graph nodes: their types, read from command schemas, and how deep a value runs past what a port takes (the lists a node maps over). |
| [`cc_engine::i18n`](#mod-cc_engine-i18n) | 11 | Localization: a Fluent-compatible subset (`key = value` messages with `{ $var }` placeholders). |
| [`cc_engine::import_material`](#mod-cc_engine-import_material) | 7 | Materials of imported models (SketchUp, OBJ, glTF): name, color, opacity and texture. |
| [`cc_engine::obj`](#mod-cc_engine-obj) | 6 | Wavefront OBJ models as component types: each object (`o`, or `g` in a file without objects) becomes a type of fixed mesh pieces, one per material of its `.mtl` library, placed by an element at the center of its footprint and the bottom of its extent. |
| [`cc_engine::osm`](#mod-cc_engine-osm) | 13 | OpenStreetMap data around the project site: the JSON an Overpass query returns, turned into a site drawing (buildings, streets, paths, railways, water, green areas, trees and street names, in project coordinates) and building footprints with heights for context masses; plus the answers of the address search (Nominatim) and of an elevation service. |
| [`cc_engine::reference`](#mod-cc_engine-reference) | 1 | The command reference (`docs/commands.md`), generated from the catalog so it always matches the engine: `ccad reference > docs/commands.md`. |
| [`cc_engine::refs`](#mod-cc_engine-refs) | 32 | Naming model objects in commands: levels, types, views, materials and systems by numeric id or by name; elements by id. |
| [`cc_engine::registry`](#mod-cc_engine-registry) | 4 |  |
| [`cc_engine::script`](#mod-cc_engine-script) | 10 | Scripts (macros): a list of commands with their arguments, as a JSON array of `{"command": ..., "args": {...}}` objects or one such object per line (`//` lines are comments). |
| [`cc_engine::session`](#mod-cc_engine-session) | 4 |  |
| [`cc_engine::skp`](#mod-cc_engine-skp) | 9 | SketchUp (.skp) models as component types: the geometry of each placed component or group becomes a type of fixed mesh pieces (one per material), placed by an element at the instance's position and plan rotation. |

<a id="mod-cc_engine"></a>

## `cc_engine`

<sub>`crates/cc_engine/src/lib.rs`</sub>

**Re-exports**

- `pub use defaults::{ conduit_offset, default_roof_base, default_top, default_wall_face, device_height, drain_slope, next_number, pipe_offset, tray_offset, walls_loop, walls_loop_at, }`
- `pub use error::{EngineError, ErrorCode, Result}`
- `pub use extension::Extension`
- `pub use refs::{Ref, category_name}`
- `pub use registry::{Command, CommandInfo, CommandKind, catalog, command_kind, describe}`
- `pub use session::{Session, new_project, new_project_with}`

**Items:** [`params_of`](#cc_engine-params_of) · [`element_params`](#cc_engine-element_params) · [`params_patch`](#cc_engine-params_patch) · [`value_patch`](#cc_engine-value_patch) · [`element_params_patch`](#cc_engine-element_params_patch)

<a id="cc_engine-params_of"></a>

### fn `params_of`

```rust
pub fn params_of<T: serde::Serialize>(value: &T) -> serde_json::Value
```

The fields of an enum value's variant as `*_get` shows them in `params`.

<a id="cc_engine-element_params"></a>

### fn `element_params`

```rust
pub fn element_params(kind: &cc_core::ElementKind) -> serde_json::Value
```

An element's `params` as `element_get` shows them (an imported drawing
without the drawing itself, which is not a parameter).

<a id="cc_engine-params_patch"></a>

### fn `params_patch`

```rust
pub fn params_patch<T>(old: &T, new: &T) -> serde_json::Value where T: serde::Serialize + serde::de::DeserializeOwned + PartialEq
```

The `params` patch a `*_set` command needs to turn the enum value `old`
(a view kind, type data) into `new`. Unlike `params_of(new)`, it also
clears the fields `new` leaves out of its JSON form (an option set back
to none, a list emptied, a flag turned off), which a merge patch would
otherwise keep.

<a id="cc_engine-value_patch"></a>

### fn `value_patch`

```rust
pub fn value_patch<T>(old: &T, new: &T) -> serde_json::Value where T: serde::Serialize + serde::de::DeserializeOwned + PartialEq
```

The merge patch that turns a whole value `old` (a struct a command takes
as a patch, such as a template's `underlay`) into `new`, clearing the
fields `new` leaves out as `params_patch` does.

<a id="cc_engine-element_params_patch"></a>

### fn `element_params_patch`

```rust
pub fn element_params_patch(old: &cc_core::ElementKind, new: &cc_core::ElementKind) -> serde_json::Value
```

`params_patch` for an element's kind, as `element_set` applies it.

<a id="mod-cc_engine-commands"></a>

## `cc_engine::commands`

<sub>`crates/cc_engine/src/commands/mod.rs`</sub>

The commands, by area. Each is a parameter struct implementing
`crate::Command`; `crate::catalog` lists them all.

**Items:** [`edit`](#cc_engine-commands-edit) · [`check`](#cc_engine-commands-check) · [`add_element`](#cc_engine-commands-add_element) · [`variant`](#cc_engine-commands-variant) · [`merge`](#cc_engine-commands-merge) · [`clearing_patch`](#cc_engine-commands-clearing_patch) · [`patch_variant`](#cc_engine-commands-patch_variant) · [`patch_value`](#cc_engine-commands-patch_value) · [`created`](#cc_engine-commands-created) · [`ids_json`](#cc_engine-commands-ids_json)

<a id="cc_engine-commands-edit"></a>

### fn `edit`

```rust
pub(crate) fn edit<R>(s: &mut Session, tx: String, f: impl FnOnce(&mut EditCtx) -> Result<R>) -> Result<R>
```

Run an edit as one undo step, kept only if it succeeds and leaves a
project that needs no repair (no dangling references, no invalid numbers).

<a id="cc_engine-commands-check"></a>

### fn `check`

```rust
pub(crate) fn check(p: &Project) -> Result<()>
```

The project is consistent: repairing a copy would change nothing.

<a id="cc_engine-commands-add_element"></a>

### fn `add_element`

```rust
pub(crate) fn add_element(e: &mut EditCtx, level: LevelId, type_id: ElementTypeId, kind: ElementKind) -> ElementId
```

Add an element (doors and windows get the next free mark).

<a id="cc_engine-commands-variant"></a>

### fn `variant`

```rust
pub(crate) fn variant(v: Value) -> (String, Value)
```

The fields of an enum value's single variant: `{"Wall": {...}}` gives
`("Wall", {...})`; unit variants give an empty object.

<a id="cc_engine-commands-merge"></a>

### fn `merge`

```rust
pub(crate) fn merge(target: &mut Value, patch: &Value)
```

Merge patch: objects merge key by key, anything else (arrays, numbers,
`null` for an optional value) replaces. Unlike RFC 7396, `null` sets a
field to null rather than removing it (model fields are never absent).
An enum value given as another variant (`{"Round": ..}` over
`{"Rectangular": ..}`) replaces the old one.

<a id="cc_engine-commands-clearing_patch"></a>

### fn `clearing_patch`

```rust
pub(crate) fn clearing_patch(old: &Value, new: &Value, check: &dyn Fn(&Value) -> Option<bool>) -> Value
```

A merge patch from `old` to `new` (two JSON forms of one value, which
leave out empty fields): `new` itself, plus a value clearing each field
`old` has and `new` leaves out. `check` merges a whole patch into the
old value and tells whether the result is valid (`Some`) and is the new
value (`true`). Each missing field takes the first of `null`, `false`,
`[]`, `""` and `0` that keeps the patch valid, or, for an object, its
own fields cleared the same way. When that does not give `new` (a field
cannot be cleared this way), the patch is `new` as before.

<a id="cc_engine-commands-patch_variant"></a>

### fn `patch_variant`

```rust
pub(crate) fn patch_variant<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T, patch: &Value) -> Result<T>
```

Apply a merge patch to the fields of an enum value (keeping its variant).

<a id="cc_engine-commands-patch_value"></a>

### fn `patch_value`

```rust
pub(crate) fn patch_value<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T, patch: &Value) -> Result<T>
```

Apply a merge patch to a whole serializable value.

<a id="cc_engine-commands-created"></a>

### fn `created`

```rust
pub(crate) fn created(s: &mut Session, id: ElementId) -> Value
```

What a created or changed element looks like now.

<a id="cc_engine-commands-ids_json"></a>

### fn `ids_json`

```rust
pub(crate) fn ids_json(ids: &[ElementId]) -> Value
```

<a id="mod-cc_engine-commands-analytical"></a>

## `cc_engine::commands::analytical`

<sub>`crates/cc_engine/src/commands/analytical.rs`</sub>

The analytical model of the structure, for structural analysis programs.

**Items:** [`AnalyticalModelGet`](#cc_engine-commands-analytical-analyticalmodelget) · [`AnalyticalExport`](#cc_engine-commands-analytical-analyticalexport)

<a id="cc_engine-commands-analytical-analyticalmodelget"></a>

### struct `AnalyticalModelGet`

```rust
pub struct AnalyticalModelGet
```

The analytical model of the structure: structural columns and beams as
members along their center lines between shared nodes. Beams framed into
columns end on the column axis, beams ending on beams end on their line,
and members are split where others connect. Each member carries its
`section`: the profile's name, `width` and `height` (m), `area` (m²), `ix`, `iy` and `j` (m⁴).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-analytical-analyticalexport"></a>

### struct `AnalyticalExport`

```rust
pub struct AnalyticalExport
```

Write the analytical model (as `analytical_model` returns it) to a JSON file.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .json file to write.


<a id="mod-cc_engine-commands-annotate"></a>

## `cc_engine::commands::annotate`

<sub>`crates/cc_engine/src/commands/annotate.rs`</sub>

Dimensions: place (associated with what they measure) and drive by value.

**Items:** [`DimensionCreate`](#cc_engine-commands-annotate-dimensioncreate) · [`DimensionChainCreate`](#cc_engine-commands-annotate-dimensionchaincreate) · [`DimensionOrdinateCreate`](#cc_engine-commands-annotate-dimensionordinatecreate) · [`DimensionSetValue`](#cc_engine-commands-annotate-dimensionsetvalue) · [`LeaderArgs`](#cc_engine-commands-annotate-leaderargs) · [`TextCreate`](#cc_engine-commands-annotate-textcreate) · [`TextAddLeader`](#cc_engine-commands-annotate-textaddleader) · [`TextRemoveLeader`](#cc_engine-commands-annotate-textremoveleader) · [`TagMove`](#cc_engine-commands-annotate-tagmove)

<a id="cc_engine-commands-annotate-dimensioncreate"></a>

### struct `DimensionCreate`

```rust
pub struct DimensionCreate
```

Place a dimension in a 2D view (points in the view's drawing coordinates:
plan X/Y in plans; horizontal position and height in elevations and
sections). Its ends follow the elements (or level lines) their points
lie on, so it keeps measuring them. Aligned dimensions measure from `a`
to `b`, linear ones along a horizontal or vertical `direction`, ordinate
ones from the origin `a` to `b` along that direction (a running
dimension; see `dimension_ordinate_create`), angular ones the angle at
`vertex` between the rays through `a` and `b`, radial and diameter ones
the arc centered at `a` through `b`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `a: Point`

  First point (center for radial and diameter).

- `b: Point`

  Second point (a point on the arc for radial and diameter).

- `kind: Option<String>`

  `aligned` (default), `linear`, `ordinate`, `angular`, `radial` or `diameter`.

- `offset: Option<f64>`

  Distance of the dimension line from `a` (left of a→b positive), or the arc radius for angular ones (default 1).

- `vertex: Option<Point>`

  Vertex of an angular dimension.

- `direction: Option<String>`

  Linear and ordinate dimensions: `horizontal` or `vertical` (default: the larger extent).

- `type_ref: Option<Ref>`

  Dimension type id or name (default: the default dimension type).


<a id="cc_engine-commands-annotate-dimensionchaincreate"></a>

### struct `DimensionChainCreate`

```rust
pub struct DimensionChainCreate
```

Place a chain of dimensions on one dimension line (continuous
dimensioning): one dimension from each point to the next, all measured
along the same direction and drawn on the same line, in one undo step.
Points are in the view's drawing coordinates, as for `dimension_create`,
and each end follows the element (or level line) it lies on. In an
aligned chain, segments along the direction are aligned dimensions and
the others measure along that direction.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `points: Vec<Point>`

  The measured points in order (at least 2, at most 500).

- `kind: Option<String>`

  `aligned` (default: along the first two points) or `linear` (horizontal or vertical).

- `offset: Option<f64>`

  Distance of the dimension line from the first point, positive to the left of the direction (default 1).

- `direction: Option<String>`

  Linear chains: `horizontal` or `vertical` (default: the larger extent of the first two points).

- `angle: Option<f64>`

  Direction to measure along, degrees counter-clockwise from the X axis (overrides `direction` and the
  direction of the first two points).

- `type_ref: Option<Ref>`

  Dimension type id or name (default: the default dimension type).


<a id="cc_engine-commands-annotate-dimensionordinatecreate"></a>

### struct `DimensionOrdinateCreate`

```rust
pub struct DimensionOrdinateCreate
```

Place running (ordinate) dimensions on one dimension line: one dimension
from the origin (the first point) to each other point, measured along
the same direction, each showing its running total from the origin, in
one undo step. Points are in the view's drawing coordinates, as for
`dimension_create`, and each end follows the element (or level line) it
lies on, so the totals follow the model; giving one a new value with
`dimension_set_value` moves what its point measures, keeping the origin.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `points: Vec<Point>`

  The origin, then the measured points (at least 2 in all, at most 500).

- `offset: Option<f64>`

  Distance of the dimension line from the origin, positive to the left of the direction (default 1).

- `direction: Option<String>`

  `horizontal` or `vertical` (default: along the origin and the second point).

- `angle: Option<f64>`

  Direction to measure along, degrees counter-clockwise from the X axis (overrides `direction`).

- `type_ref: Option<Ref>`

  Dimension type id or name (default: the default dimension type).


<a id="cc_engine-commands-annotate-dimensionsetvalue"></a>

### struct `DimensionSetValue`

```rust
pub struct DimensionSetValue
```

Give a dimension a new value (meters; degrees for angular dimensions):
in plans what its moving end measures moves or stretches; between level
lines in elevations and sections the upper level moves. Radial and
diameter dimensions are read-only.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Dimension element id.

- `value: f64`

  New value: meters, or degrees for angular dimensions.


<a id="cc_engine-commands-annotate-leaderargs"></a>

### struct `LeaderArgs`

```rust
pub struct LeaderArgs
```

A leader of a new text note.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Clone`, `Default`

**Fields**

- `end: Point`

  The point the leader points at, where its symbol is drawn.

- `elbow: Option<Point>`

  A bend point that splits the leader in two segments (text → elbow → end).

- `symbol: Option<String>`

  End symbol: `Arrow`, `OpenArrow`, `Triangle`, `Dot`, `Circle`, `Square`, `FilledSquare`, `Tick`
  or `None` (default: the text type's symbol).


<a id="cc_engine-commands-annotate-textcreate"></a>

### struct `TextCreate`

```rust
pub struct TextCreate
```

Place a text note in a 2D view (points in the view's drawing coordinates,
as for dimensions). Without `width` it is single-line text (only line
breaks in `text` break it); with `width` (paper millimeters) it is
multiline text wrapped to that width. Leaders point from the text to
what it describes, each with one segment or two (with an `elbow`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `position: Point`

  Top left corner of the text.

- `text: String`

  The text; a line break starts a new line.

- `width: Option<f64>`

  Wrapping width in paper millimeters (multiline text).

- `rotation: Option<f64>`

  Rotation in degrees counter-clockwise.

- `type_ref: Option<Ref>`

  Text type id or name (default: the default text type).

- `leaders: Vec<LeaderArgs>`

  Leaders from the text.


<a id="cc_engine-commands-annotate-textaddleader"></a>

### struct `TextAddLeader`

```rust
pub struct TextAddLeader
```

Add a leader to a text note.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `id: u64`

  Text element id.

- `leader: LeaderArgs`

<a id="cc_engine-commands-annotate-textremoveleader"></a>

### struct `TextRemoveLeader`

```rust
pub struct TextRemoveLeader
```

Remove a leader from a text note.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Text element id.

- `index: usize`

  Index of the leader (from 0, as in `params.leaders`).


<a id="cc_engine-commands-annotate-tagmove"></a>

### struct `TagMove`

```rust
pub struct TagMove
```

Move the tag of an element in a plan (the tag of a door, window, beam,
column or footing): by `delta` from where it is, to `offset` from its
default spot, or back there with `reset`. A tag moved away from its
element gets a leader to it. Each plan keeps its own tag positions.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  Plan id or name.

- `id: u64`

  Id of the element the tag names.

- `delta: Option<Point>`

  Move by `[dx, dy]` meters in plan.

- `offset: Option<Point>`

  Place the tag `[dx, dy]` meters from its default spot.

- `reset: bool`

  Put the tag back in its default spot.


<a id="mod-cc_engine-commands-bar_marks"></a>

## `cc_engine::commands::bar_marks`

<sub>`crates/cc_engine/src/commands/bar_marks.rs`</sub>

Bar marks, bending schedules, bar tags and the check of bar lengths (see
`cc_core::bar_marks`, `cc_core::bar_tag`, `cc_geom::bar_marks` and
`cc_views::bending`).

**Items:** [`shape_json`](#cc_engine-commands-bar_marks-shape_json) · [`mark_json`](#cc_engine-commands-bar_marks-mark_json) · [`BarMarksList`](#cc_engine-commands-bar_marks-barmarkslist) · [`BarMarksUpdate`](#cc_engine-commands-bar_marks-barmarksupdate) · [`BarMarksRenumber`](#cc_engine-commands-bar_marks-barmarksrenumber) · [`BarMarkingSet`](#cc_engine-commands-bar_marks-barmarkingset) · [`BarLengthCheck`](#cc_engine-commands-bar_marks-barlengthcheck) · [`BendingScheduleCreate`](#cc_engine-commands-bar_marks-bendingschedulecreate) · [`takes_tags`](#cc_engine-commands-bar_marks-takes_tags) · [`TagParts`](#cc_engine-commands-bar_marks-tagparts) · [`BarTagCreate`](#cc_engine-commands-bar_marks-bartagcreate) · [`check_tag`](#cc_engine-commands-bar_marks-check_tag) · [`tag_text`](#cc_engine-commands-bar_marks-tag_text)

<a id="cc_engine-commands-bar_marks-shape_json"></a>

### fn `shape_json`

```rust
pub(crate) fn shape_json(s: &ShapeSummary) -> Value
```

A shape as the queries show it: lengths in meters, turns in degrees.

<a id="cc_engine-commands-bar_marks-mark_json"></a>

### fn `mark_json`

```rust
pub(crate) fn mark_json(m: &Mark) -> Value
```

A mark as the queries show it.

<a id="cc_engine-commands-bar_marks-barmarkslist"></a>

### struct `BarMarksList`

```rust
pub struct BarMarksList
```

Every bar mark of the model, sorted naturally (N2 before N10): its
diameter and grade, its shape (`code`, e.g. `L 30-470`, and the legs),
the length of one bar (`unit_length`, the mean of its bars), how many
bars have it, their total length and weight, the longest bar, the sets
using it, and where it comes from (`source`: `frozen`, `override` or
`numbered`). Frozen marks no bar uses are listed with a count of 0.
Also the marking settings. A mark names one shape: the same diameter,
grade, legs and bends within the length tolerance (default 5 mm)
and 1 degree, read from either end or mirrored. Lengths in meters,
angles in degrees, weights in kilograms.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-bar_marks-barmarksupdate"></a>

### struct `BarMarksUpdate`

```rust
pub struct BarMarksUpdate
```

Freeze the bar marks: the marks numbered for shapes the frozen marks
table does not hold yet are added to it, so they stay when the model
changes (entries already there are kept, overrides stay on their sets).
With `drop_unused`, entries no bar uses any more are dropped. Changes
nothing (and makes no undo step) when there is nothing to freeze or drop.
`bar_marks_list` shows the result.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `drop_unused: bool`

  Drop frozen marks that no bar uses (default false).


<a id="cc_engine-commands-bar_marks-barmarksrenumber"></a>

### struct `BarMarksRenumber`

```rust
pub struct BarMarksRenumber
```

Number the bar marks afresh: the frozen marks table is cleared, and so
are the sets' `bar_mark` overrides unless `keep_overrides`; then every
shape is numbered from `start` (default: the current one) with `prefix`
(default: the current one) in the stable order (partition, diameter
descending, length descending, shape) and the numbering is frozen. One
undo step; returns the new marks.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `prefix: Option<String>`

  Text before the numbers (e.g. "N"; at most 40 characters).

- `start: Option<u32>`

  First number (0 to 1000000).

- `keep_overrides: bool`

  Keep the marks the sets set themselves (default false: they are cleared).


<a id="cc_engine-commands-bar_marks-barmarkingset"></a>

### struct `BarMarkingSet`

```rust
pub struct BarMarkingSet
```

Change how bars are marked: the `prefix` and `start` of numbered marks
(used by marks numbered from now on; `bar_marks_renumber` renumbers the
frozen ones), the length tolerance within which bars are one shape, and
the stock length bars are bought in (`bar_length_check` and bending
schedules flag longer bars). One undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `prefix: Option<String>`

  Text before the numbers (at most 40 characters).

- `start: Option<u32>`

  First number (0 to 1000000).

- `length_tolerance: Option<f64>`

  Legs of bars of one mark differ by at most this, meters (0.0001 to 0.05; default 0.005).

- `stock_length: Option<f64>`

  Length bars are bought in, meters (1 to 100; default 12).


<a id="cc_engine-commands-bar_marks-barlengthcheck"></a>

### struct `BarLengthCheck`

```rust
pub struct BarLengthCheck
```

Bars longer than `max_length` (default: the project's stock length, 12 m
unless `bar_marking_set` changed it): the bars that need splices, listed
by set and mark with how many there are and the longest. A set with a
`max_length` of its own cuts its bars into lapped pieces, which then pass.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `max_length: Option<f64>`

  Longest bar allowed, meters (default: the stock length).


<a id="cc_engine-commands-bar_marks-bendingschedulecreate"></a>

### struct `BendingScheduleCreate`

```rust
pub struct BendingScheduleCreate
```

Create a bending schedule: a reinforcement schedule view with one row
per bar mark (mark, diameter, grade, a sketch of the shape with its
dimensions, the length of one bar in centimeters, the count, the total
length and the weight, from the type's mass per meter or the diameter;
rows of bars longer than the stock length noted) and, under it, a
summary per diameter and grade (total length and weight; with
`waste_percent`, the weight with the waste added). Totals add the bars
exactly. It is placed on sheets, exported (`view_export`) and written to
CSV (`schedule_export_csv`, shapes as codes such as `L 30-470`) like any
schedule; `view_set` changes its `def.bending` options later.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  View name (default: the language's).

- `partitions: Vec<String>`

  Only the sets of these partitions (default: every set; "" stands for sets without one).

- `group_by_partition: bool`

  Rows under a heading per partition, each partition's marks counted on their own (default false).

- `waste_percent: Option<f64>`

  Waste added in the summary, percent (0 to 100; default: no waste columns).

- `hide_totals: bool`

  Hide the totals row of the marks (default false).


<a id="cc_engine-commands-bar_marks-takes_tags"></a>

### fn `takes_tags`

```rust
pub(crate) fn takes_tags(view: &View) -> bool
```

Whether `view` takes bar tags: plans, elevations and sections.

<a id="cc_engine-commands-bar_marks-tagparts"></a>

### struct `TagParts`

```rust
pub struct TagParts
```

The parts of a tag's text to write.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Clone`, `Copy`

**Fields**

- `count: Option<bool>`

  The number of bars (default true).

- `mark: Option<bool>`

  The bar mark (default true).

- `diameter: Option<bool>`

  The diameter in millimeters (default true).

- `spacing: Option<bool>`

  The spacing in centimeters, for sets with one (default true).

- `length: Option<bool>`

  The length of one bar in centimeters (default true).


<a id="cc_engine-commands-bar_marks-bartagcreate"></a>

### struct `BarTagCreate`

```rust
pub struct BarTagCreate
```

Tag a reinforcement set in a plan, an elevation or a section: a text
such as `25 N1 Ø12.5 c/12 C=500` (the count, the mark, the diameter in
millimeters, the spacing in centimeters for sets with one, the length of
one bar in centimeters, `C=var` when they differ; a set of several marks
shows its first and `+`), derived from the set each time; `multiplier`
writes `2x` first for a set repeated on both faces. The view draws the
set's bars as thin lines (dots where seen end-on), so the tag points at
them. The tag goes with its view and its set. A project gets a bar tag
type (2.5 mm text) with its first tag; `element_get` gives the text
(`derived.text`) and `element_set` changes the tag's `params`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name: a floor or ceiling plan, an elevation or a section.

- `set: u64`

  The reinforcement set (or shaped bars) to tag.

- `position: Point`

  Where the text starts (its left end, on the baseline), in the view's drawing coordinates.

- `leader: Option<Point>`

  Where a leader from the text points (default: no leader).

- `multiplier: Option<u32>`

  How many times the set is repeated: 2 writes `2x` first (1 to 100; default 1).

- `rotation_deg: Option<f64>`

  Angle of the text, degrees counter-clockwise (default 0).

- `parts: Option<TagParts>`

  The parts of the text to write, `{count, mark, diameter, spacing, length}`, each true or false (default: all).

- `type_ref: Option<Ref>`

  Bar tag type id or name (default: the default bar tag type).


<a id="cc_engine-commands-bar_marks-check_tag"></a>

### fn `check_tag`

```rust
pub(crate) fn check_tag(p: &cc_core::Project, tag: &BarTag) -> Result<()>
```

Refuse a changed tag a view or set would not take, or with bad values.

<a id="cc_engine-commands-bar_marks-tag_text"></a>

### fn `tag_text`

```rust
pub(crate) fn tag_text(s: &mut Session, tag: &BarTag) -> Option<String>
```

The text of tag `tag` as its view writes it.

<a id="mod-cc_engine-commands-bracing"></a>

## `cc_engine::commands::bracing`

<sub>`crates/cc_engine/src/commands/bracing.rs`</sub>

Parametric bracing: `bracing_create` makes a host that keeps the braces
of one bay of a frame in step with its pattern, storeys and brace type.

**Items:** [`brace_row`](#cc_engine-commands-bracing-brace_row) · [`new_bracing_type`](#cc_engine-commands-bracing-new_bracing_type) · [`BracingCreate`](#cc_engine-commands-bracing-bracingcreate) · [`derived`](#cc_engine-commands-bracing-derived)

<a id="cc_engine-commands-bracing-brace_row"></a>

### fn `brace_row`

```rust
pub fn brace_row(m: StructuralMaterial) -> Option<(&'static str, &'static str)>
```

The catalog row the braces of a material get by default: (series,
designation); `None` for concrete (give a beam type).

<a id="cc_engine-commands-bracing-new_bracing_type"></a>

### fn `new_bracing_type`

```rust
pub fn new_bracing_type(id: ElementTypeId) -> ElementType
```

The type a project gets with its first bracing.

<a id="cc_engine-commands-bracing-bracingcreate"></a>

### struct `BracingCreate`

```rust
pub struct BracingCreate
```

Brace one bay of a frame, storey by storey: the bay runs from `start` to
`end` in plan (the axes of its two columns), or between two `columns`,
whose level, base and height it then takes. `pattern` is `x` (default:
both diagonals), `diagonal` (one, rising toward `end`; `flip` turns it,
`alternate` turns every other storey), `v` (from the top corners down to
the middle of the bay), `chevron` (from the bottom corners up to the
middle, an inverted V) or `k` (from the mid-height of each column to the
middle of the bay at the top and at the bottom of the storey). `stories`
storeys of `height` are braced one above the other from `base_offset`.
The braces are ordinary beams the bracing keeps in step with its
parameters (`element_set` regenerates them, keeping their ids); a brace
edited or deleted by hand is detached and left alone. The default brace
type comes from the profile catalog for the `material` (steel:
`SHS 100x100x4`; timber: `Peça 6x12`), made when missing; concrete needs
a `type`. Out-of-range values are refused. One undo step; returns the
bracing's `id` and its `members`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Option<Point>`

  One column axis of the bay in plan (not with `columns`).

- `end: Option<Point>`

  The other column axis, 0.5 to 200 m from the start (not with `columns`).

- `columns: Vec<u64>`

  The two column ids of the bay, instead of `start` and `end`.

- `pattern: Option<String>`

  `x` (default), `diagonal`, `v`, `chevron` or `k`.

- `height: Option<f64>`

  One storey, meters, 0.5 to 50 (default: the columns' common height over `stories`, else the level's height).

- `stories: Option<u32>`

  Storeys braced one above the other, 1 to 100 (default 1).

- `base_offset: Option<f64>`

  The foot of the lowest storey above the level, meters (default: the columns' base, else 0).

- `flip: bool`

  A `diagonal` rises toward the start instead of the end.

- `alternate: bool`

  `diagonal` braces change direction storey by storey.

- `level: Option<Ref>`

  Level id or name (default: the first column's level, else the lowest level).

- `type_ref: Option<Ref>`

  Beam type of the braces, id or name (default: the material's catalog row).

- `material: Option<String>`

  `steel` (default) or `timber`: the catalog row of the brace type when `type` is not given.


<a id="cc_engine-commands-bracing-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(b: &Bracing, out: &mut serde_json::Map<String, Value>)
```

`element_get`'s derived values of a bracing: its brace ids, how many
roles are detached, the bay width and the height of all its storeys.

<a id="mod-cc_engine-commands-browser"></a>

## `cc_engine::commands::browser`

<sub>`crates/cc_engine/src/commands/browser.rs`</sub>

Project browser organization (see `cc_core::browser`): named schemes
that group the browser's views by view type, level, discipline or phase.
The schemes and the one in use are project settings, changed in one undo
step each; `browser_tree` gives the tree the browser shows.

**Items:** [`NEW_SCHEME`](#cc_engine-commands-browser-new_scheme) · [`BrowserSchemesList`](#cc_engine-commands-browser-browserschemeslist) · [`BrowserSchemeCreate`](#cc_engine-commands-browser-browserschemecreate) · [`BrowserSchemeSet`](#cc_engine-commands-browser-browserschemeset) · [`BrowserSchemeDelete`](#cc_engine-commands-browser-browserschemedelete) · [`BrowserSchemeUse`](#cc_engine-commands-browser-browserschemeuse) · [`BrowserTree`](#cc_engine-commands-browser-browsertree)

<a id="cc_engine-commands-browser-new_scheme"></a>

### const `NEW_SCHEME`

```rust
pub const NEW_SCHEME: [GroupBy; 2]
```

The criteria a new scheme groups by when none are given.

<a id="cc_engine-commands-browser-browserschemeslist"></a>

### struct `BrowserSchemesList`

```rust
pub struct BrowserSchemesList
```

The project browser's organization schemes: each with its name and the
criteria it groups the views by, outermost first, and whether it is in
use. `in_use` is null while the browser lists the views by type alone
(the default), and `criteria` are the names `group_by` takes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-browser-browserschemecreate"></a>

### struct `BrowserSchemeCreate`

```rust
pub struct BrowserSchemeCreate
```

Create a project browser organization scheme: a named way to group the
browser's views. `group_by` lists the criteria from the outermost group
in (`view_type`, `level`, `discipline`, `phase`); a view without a value
for one (a section has no level) stays beside that criterion's groups.
The new scheme is put in use unless `use_it` is false.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique among the schemes).

- `group_by: Option<Vec<String>>`

  Criteria, outermost first: `view_type`, `level`, `discipline`,
  `phase` (default: `["view_type", "level"]`).

- `use_it: Option<bool>`

  Make the browser follow the new scheme (default true).


<a id="cc_engine-commands-browser-browserschemeset"></a>

### struct `BrowserSchemeSet`

```rust
pub struct BrowserSchemeSet
```

Change a project browser organization scheme: its name or the criteria
it groups the views by. The browser follows at once when it is in use.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `scheme: Ref`

  Scheme id or name.

- `name: Option<String>`

  New name.

- `group_by: Option<Vec<String>>`

  New criteria, outermost first: `view_type`, `level`, `discipline`, `phase`.


<a id="cc_engine-commands-browser-browserschemedelete"></a>

### struct `BrowserSchemeDelete`

```rust
pub struct BrowserSchemeDelete
```

Delete a project browser organization scheme. When it was in use the
browser goes back to listing the views by type.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `scheme: Ref`

  Scheme id or name.


<a id="cc_engine-commands-browser-browserschemeuse"></a>

### struct `BrowserSchemeUse`

```rust
pub struct BrowserSchemeUse
```

Choose the scheme the project browser follows. Without `scheme` (or
with null) the browser lists the views by type alone, the default.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `scheme: Option<Ref>`

  Scheme id or name; omitted or null for the default list by view type.


<a id="cc_engine-commands-browser-browsertree"></a>

### struct `BrowserTree`

```rust
pub struct BrowserTree
```

The project browser's tree: the views grouped as the scheme in use (or
`scheme`, or the criteria in `group_by`) says. A group row has `group`
(its criterion), `name`, `value` (the view type, level id, discipline or
phase id), how many `views` it holds and its `children`; a view row has
`view` (its id) and `name`. Plans come in the order of their levels.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `scheme: Option<Ref>`

  Scheme id or name (default: the scheme in use).

- `group_by: Option<Vec<String>>`

  Criteria to group by instead of a scheme's, outermost first:
  `view_type`, `level`, `discipline`, `phase`.


<a id="mod-cc_engine-commands-cable_tray"></a>

## `cc_engine::commands::cable_tray`

<sub>`crates/cc_engine/src/commands/cable_tray.rs`</sub>

Cable trays: open rectangular runs carrying electrical circuits, joined
like conduits (see `cc_core::mep`, `cc_geom::mep_net`).

**Items:** [`metal`](#cc_engine-commands-cable_tray-metal) · [`new_tray_type`](#cc_engine-commands-cable_tray-new_tray_type) · [`parse_style`](#cc_engine-commands-cable_tray-parse_style) · [`CableTrayCreate`](#cc_engine-commands-cable_tray-cabletraycreate) · [`CableTrayTypeCreate`](#cc_engine-commands-cable_tray-cabletraytypecreate)

<a id="cc_engine-commands-cable_tray-metal"></a>

### fn `metal`

```rust
pub(crate) fn metal(p: &Project) -> String
```

The material of new cable trays and valves: the project's first metal, else its first material.

<a id="cc_engine-commands-cable_tray-new_tray_type"></a>

### fn `new_tray_type`

```rust
pub fn new_tray_type(id: ElementTypeId, width: f64, height: f64, style: TrayStyle, material: String) -> ElementType
```

A cable tray type of `width` × `height` and `style`, named by its
section (in millimeters, or in inches in imperial units).

<a id="cc_engine-commands-cable_tray-parse_style"></a>

### fn `parse_style`

```rust
pub(crate) fn parse_style(name: &str) -> Result<TrayStyle>
```

<a id="cc_engine-commands-cable_tray-cabletraycreate"></a>

### struct `CableTrayCreate`

```rust
pub struct CableTrayCreate
```

Create a cable tray from `start` to `end` (or a vertical run at `start`
to `end_offset`) carrying `circuits`. Its center line runs `offset` above
the level (default: where it starts on another cable tray, such as the
end of a run; else 0.30 m under the level above). Trays meet other trays
at their ends or along them (elbows, tees and crosses are derived), not
conduits or pipes. A project without a cable tray type gets one (200 ×
100 mm, 8" × 4" in imperial projects; ladder).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the center line in plan.

- `end: Option<Point>`

  End in plan (default: the start, for a vertical run).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Cable tray type id or name (default: the first cable tray type).

- `circuits: Vec<Ref>`

  Circuits it carries: ids, names or codes.

- `offset: Option<f64>`

  Elevation of the start above the level, meters.

- `end_offset: Option<f64>`

  Elevation of the end above the level, meters (default: as the start).


<a id="cc_engine-commands-cable_tray-cabletraytypecreate"></a>

### struct `CableTrayTypeCreate`

```rust
pub struct CableTrayTypeCreate
```

Create a cable tray type: an open channel `width` × `height` (meters;
default 0.2 × 0.1, 8" × 4" in imperial projects; at most 2 × 1), built as `style` says (`ladder`, the
default, `solid_bottom` or `perforated`), in `material` (default: the
project's first metal). The name defaults to its section.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: by its section, e.g. "Cable tray 200×100").

- `width: Option<f64>`

  Outside width, meters.

- `height: Option<f64>`

  Outside height, meters.

- `style: Option<String>`

  `ladder`, `solid_bottom` or `perforated`.

- `material: Option<String>`

  Material name.


<a id="mod-cc_engine-commands-cabling"></a>

## `cc_engine::commands::cabling`

<sub>`crates/cc_engine/src/commands/cabling.rs`</sub>

Structured cabling: racks, data outlets and the horizontal cables
between them. Each rack feeds a data system (`SystemKind::Data`, its
`panel` the rack), created with the rack; outlets belong to it like
devices to a circuit, and each jack takes a cable to the rack, whose
length is estimated along the ceiling (see `cc_core::mep::cable_length`).
Cable trays and conduits carry the data systems like circuits.

**Items:** [`RackPlace`](#cc_engine-commands-cabling-rackplace) · [`DataOutletPlace`](#cc_engine-commands-cabling-dataoutletplace) · [`CablingList`](#cc_engine-commands-cabling-cablinglist)

<a id="cc_engine-commands-cabling-rackplace"></a>

### struct `RackPlace`

```rust
pub struct RackPlace
```

Place a communications rack at `position` (on the wall, `offset` above
the level, default 1.40 m, 4'-6" in imperial projects) and create the data system its outlets are
cabled to. The rack's mark and the system's code are `mark` (default
the next free `R1`, `R2`…). A project without a rack type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Insertion point in plan (the back of the rack).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `offset: Option<f64>`

  Elevation above the level, meters.

- `mark: Option<String>`

  Mark of the rack and code of its cabling.

- `type_ref: Option<Ref>`

  Rack type id or name.


<a id="cc_engine-commands-cabling-dataoutletplace"></a>

### struct `DataOutletPlace`

```rust
pub struct DataOutletPlace
```

Place a data (or voice) outlet at `position` (on the wall, `offset`
above the level, default 0.30 m, 1'-6" in imperial projects), cabled to `rack` (a rack element: its
data system) or to the data system `system`; by default to the
project's only rack, if it has one. `ports` picks a type with that many
jacks (one cable each), made when the project has none; a project
without a data outlet type gets one (two jacks).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Insertion point in plan (the back of the outlet).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `offset: Option<f64>`

  Elevation above the level, meters.

- `rack: Option<u64>`

  Rack element id.

- `system: Option<Ref>`

  Data system id, name or code.

- `ports: Option<u32>`

  Jacks per outlet (1 to 48).

- `type_ref: Option<Ref>`

  Data outlet type id or name.


<a id="cc_engine-commands-cabling-cablinglist"></a>

### struct `CablingList`

```rust
pub struct CablingList
```

The structured cabling per rack: for each data system, its rack, its
outlets with their jacks and the estimated length of each cable (up to
the ceiling of the outlet's level, across along the two axes, down to
the rack, plus 3 m of slack), the totals, and the cables longer than the
90 m of a permanent link. `rack` limits it to one rack.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `rack: Option<u64>`

  Rack element id.


<a id="mod-cc_engine-commands-cad"></a>

## `cc_engine::commands::cad`

<sub>`crates/cc_engine/src/commands/cad.rs`</sub>

Imported CAD drawings (DWG, DXF and SVG underlays, and pictures):
importing a file into a 2D view and showing or hiding its layers. The placement (units, scale,
position, rotation, colors) is edited with `element_set`; moving,
rotating, pinning and deleting use the generic commands.

**Items:** [`light`](#cc_engine-commands-cad-light) · [`params`](#cc_engine-commands-cad-params) · [`patch`](#cc_engine-commands-cad-patch) · [`derived`](#cc_engine-commands-cad-derived) · [`cad_type`](#cc_engine-commands-cad-cad_type) · [`CadImportFile`](#cc_engine-commands-cad-cadimportfile) · [`ImageImport`](#cc_engine-commands-cad-imageimport) · [`IMAGE_EXTENSIONS`](#cc_engine-commands-cad-image_extensions) · [`cad_of`](#cc_engine-commands-cad-cad_of) · [`CadLayers`](#cc_engine-commands-cad-cadlayers) · [`CadLayersSet`](#cc_engine-commands-cad-cadlayersset)

<a id="cc_engine-commands-cad-light"></a>

### fn `light`

```rust
pub(crate) fn light(kind: &ElementKind) -> std::borrow::Cow<'_, ElementKind>
```

The element kind without its drawing (what `params` show and patch):
the drawing is data read from the file, not a parameter.

<a id="cc_engine-commands-cad-params"></a>

### fn `params`

```rust
pub fn params(kind: &ElementKind) -> Value
```

`params` of an element kind (its variant's fields), without an imported drawing.

<a id="cc_engine-commands-cad-patch"></a>

### fn `patch`

```rust
pub(crate) fn patch(c: &CadImport, patch: &Value) -> Result<ElementKind>
```

Apply a `params` patch to an imported drawing's element, keeping its drawing.

<a id="cc_engine-commands-cad-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(c: &CadImport, out: &mut serde_json::Map<String, Value>)
```

Derived values of an imported drawing for `element_get`.

<a id="cc_engine-commands-cad-cad_type"></a>

### fn `cad_type`

```rust
pub(crate) fn cad_type(e: &mut cc_core::EditCtx) -> ElementTypeId
```

The imported drawing type, creating one if the project has none.

<a id="cc_engine-commands-cad-cadimportfile"></a>

### struct `CadImportFile`

```rust
pub struct CadImportFile
```

Import a DWG or DXF file into a 2D view (plan, ceiling plan, elevation,
section, legend or drafting view) as a drawing to trace over, in one undo step. DXF
(ASCII or binary, any version) is read entirely by ConstructCraft; DWG
(R12 to 2018 and later) through the acadrust library. Blocks are
inserted, curves kept as arcs, texts kept; entities that are not drawn
(3D solids, images, infinite lines…) are counted in `skipped`. The
element starts pinned, with the layers that are off or frozen in the
file hidden, and in the view's level. It is drawn in the view only,
under the model, and is left out of 3D views and IFC export.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .dwg or .dxf file.

- `view: Ref`

  View id or name.

- `position: Option<Point>`

  Where the drawing goes, view coordinates in meters: its origin
  (`align` "origin", the default; [0, 0] if not given) or the center of
  its extents (`align` "center"; the center of what the view shows if
  not given).

- `align: Option<String>`

  `origin` (the drawing's origin at `position`), `center`, or `auto`:
  the origin, unless that leaves the drawing more than 1 km from what
  the view shows (a drawing in survey coordinates), then the center.

- `units: Option<String>`

  Drawing units, overriding the file's: mm, cm, dm, m, km, in, ft, yd
  or mi. Unitless files are read as meters when small (up to 1500
  units across), else as millimeters.

- `rotation: Option<f64>`

  Rotation in degrees counter-clockwise.

- `scale: Option<f64>`

  Extra scale on top of the units (default 1).

- `color_mode: Option<String>`

  `grey` (default), `layers` (the file's colors) or `black`.

- `halftone: Option<bool>`

  Draw it faded (default false).

- `blocks: Option<bool>`

  Bring the file's blocks in as detail components (DWG and DXF): each
  block inserted by a move, turn, uniform scale or mirror becomes a
  detail component type named after it (its nested blocks as nested
  pieces), and each such insert a detail component placed in the view;
  everything else stays in the imported drawing (default false: blocks
  are flattened into it).


<a id="cc_engine-commands-cad-imageimport"></a>

### struct `ImageImport`

```rust
pub struct ImageImport
```

Import a picture (PNG, JPEG, GIF, BMP or WebP) or an SVG drawing into a
2D view (plan, ceiling plan, elevation, section, legend or drafting view), in one undo
step, as an imported drawing under the model: placed, scaled, rotated,
pinned, hidden and printed like an imported DWG or DXF. A picture is as
large as its resolution makes it on paper (96 dpi when the file declares
none), times `scale`, unless `width` sets its width in the view; the
file's bytes are kept in the project. An SVG keeps its paths (curves as
polylines), fills and embedded pictures in their own colors; its texts
are not read.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the picture or .svg file.

- `view: Ref`

  View id or name.

- `position: Option<Point>`

  Where it goes, view coordinates in meters: the center of the picture
  (`align` "center", the default; the center of what the view shows if
  not given) or its bottom-left corner for pictures and the page's for
  SVG (`align` "origin"; [0, 0] if not given).

- `align: Option<String>`

  `center` (default) or `origin`.

- `width: Option<f64>`

  Width in the view, meters (overrides `scale`).

- `scale: Option<f64>`

  Scale on top of its size on paper (default 1; 100 for a plan drawn at 1:100).

- `rotation: Option<f64>`

  Rotation in degrees counter-clockwise.

- `halftone: Option<bool>`

  Draw it faded (default false).


<a id="cc_engine-commands-cad-image_extensions"></a>

### const `IMAGE_EXTENSIONS`

```rust
pub const IMAGE_EXTENSIONS: [&str; 7]
```

File extensions read by `image_import`.

<a id="cc_engine-commands-cad-cad_of"></a>

### fn `cad_of`

```rust
pub(super) fn cad_of(s: &Session, id: u64) -> Result<(ElementId, CadImport)>
```

<a id="cc_engine-commands-cad-cadlayers"></a>

### struct `CadLayers`

```rust
pub struct CadLayers
```

The layers of an imported drawing: name, color, whether the view shows
it, whether it was on in the file, and how many primitives it holds.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Imported drawing element id.


<a id="cc_engine-commands-cad-cadlayersset"></a>

### struct `CadLayersSet`

```rust
pub struct CadLayersSet
```

Show or hide layers of an imported drawing (by name; all of them when
`layers` is empty).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Imported drawing element id.

- `layers: Vec<String>`

  Layer names (case-insensitive); empty for every layer.

- `visible: bool`

  Show (true) or hide (false) them.


<a id="mod-cc_engine-commands-cad_explode"></a>

## `cc_engine::commands::cad_explode`

<sub>`crates/cc_engine/src/commands/cad_explode.rs`</sub>

Exploding an imported drawing (`cad_explode`): its visible primitives
become native, editable elements of its view in one undo step. Lines,
polylines (bulged segments as arcs), arcs and circles become detail
lines, one per segment, in a project line style per layer; fills become
filled regions of a solid type per color; texts become text notes of a
text type per size; pictures stay in an imported drawing with the same
placement. The element's color mode and halftone are not carried over:
the line styles take the layer colors.

**Items:** [`MAX_ELEMENTS`](#cc_engine-commands-cad_explode-max_elements) · [`MAX_NEW_TYPES`](#cc_engine-commands-cad_explode-max_new_types) · [`CadExplode`](#cc_engine-commands-cad_explode-cadexplode)

<a id="cc_engine-commands-cad_explode-max_elements"></a>

### const `MAX_ELEMENTS`

```rust
pub const MAX_ELEMENTS: usize
```

Most elements one explode makes: a larger drawing is refused (explode
a part of it after hiding layers, or keep it as an imported drawing).

<a id="cc_engine-commands-cad_explode-max_new_types"></a>

### const `MAX_NEW_TYPES`

```rust
pub const MAX_NEW_TYPES: usize
```

Most types (text sizes and fill colors) one explode adds.

<a id="cc_engine-commands-cad_explode-cadexplode"></a>

### struct `CadExplode`

```rust
pub struct CadExplode
```

Explode an imported DWG, DXF or SVG drawing into native elements of
its view, in one undo step: its lines, polylines, arcs and circles
become detail lines (one element per segment, bulged segments as arcs;
ellipses and splines are stored as polylines and come out as their
segments), its fills filled regions (no boundary, a solid filled region
type of the fill's color, reused when the project has one), and its
texts text notes (same place, height, rotation and alignment; a text
type of that size on paper at the view's scale, reused when one exists).
Each layer becomes a project line style of its name and color (light
colors darkened for paper), reused when a style of that name exists.
Layers hidden in the element (which starts with the layers off or
frozen in the file) are left out; the placement (units, scale,
position, rotation, mirror) is applied. Pictures stay in an imported
drawing in the same view with the same placement; without them the
element is removed. The color mode and halftone are not carried over.
Refused above 100,000 elements.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Imported drawing element id.


<a id="mod-cc_engine-commands-callout"></a>

## `cc_engine::commands::callout`

<sub>`crates/cc_engine/src/commands/callout.rs`</sub>

Callouts and detail views (see `cc_core::callout`).

**Items:** [`detail_scale`](#cc_engine-commands-callout-detail_scale) · [`crop_box`](#cc_engine-commands-callout-crop_box) · [`rect_json`](#cc_engine-commands-callout-rect_json) · [`view_callout_json`](#cc_engine-commands-callout-view_callout_json) · [`CalloutCreate`](#cc_engine-commands-callout-calloutcreate) · [`CalloutRemove`](#cc_engine-commands-callout-calloutremove) · [`CalloutsList`](#cc_engine-commands-callout-calloutslist)

<a id="cc_engine-commands-callout-detail_scale"></a>

### fn `detail_scale`

```rust
pub fn detail_scale(system: MeasurementSystem, parent: u32) -> u32
```

The default scale of a detail of a view at 1:`parent`: the largest
standard scale of the project's `system` at most a fifth of it (1:50
gives 1:10; `1/4" = 1'-0"` gives `1 1/2" = 1'-0"`).

<a id="cc_engine-commands-callout-crop_box"></a>

### fn `crop_box`

```rust
pub(crate) fn crop_box(corners: [Point; 2]) -> Result<CropBox>
```

The rectangle with corners `a` and `b`, checked.

<a id="cc_engine-commands-callout-rect_json"></a>

### fn `rect_json`

```rust
pub(crate) fn rect_json(r: CropBox) -> Value
```

A rectangle as commands show it: `[[min x, min y], [max x, max y]]`.

<a id="cc_engine-commands-callout-view_callout_json"></a>

### fn `view_callout_json`

```rust
pub(crate) fn view_callout_json(p: &Project, v: &View) -> (Value, Value)
```

A view's crop and callout as `view_get` shows them.

<a id="cc_engine-commands-callout-calloutcreate"></a>

### struct `CalloutCreate`

```rust
pub struct CalloutCreate
```

Draw a callout in a plan, ceiling plan, section or elevation: a
rectangle (two opposite corners in the view's drawing coordinates)
standing for a detail view of that region. By default a new detail
view is made: of the parent's kind (it stays live with the model,
following the parent's level, section line or direction), cropped to
the rectangle, at a larger scale, with its own detail lines, regions,
detail components, text and dimensions. With `drafting`, the callout
points to that existing drafting view instead (no crop). The parent
draws the rectangle with rounded corners and a tag with the detail
number and the sheet number once the view is placed on a sheet.
Moving the callout is changing the detail view's `crop` (`view_set`);
deleting the detail view removes it (`callout_remove` keeps the view).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `parent: Ref`

  The view the callout is drawn in (id or name): a floor or ceiling plan, a section or an elevation.

- `rect: [Point; 2]`

  Two opposite corners of the rectangle, in the parent's drawing
  coordinates (meters; in sections and elevations `y` is the height).

- `scale: Option<refs::ScaleArg>`

  Scale of the detail view: the denominator, or text such as `"1:10"`
  or an imperial label like `"1 1/2\" = 1'-0\""` (default: a standard
  scale of the project's units about five times larger than the
  parent's, 1:10 under 1:50).

- `name: Option<String>`

  Name of the detail view (default "Detail N").

- `drafting: Option<Ref>`

  An existing drafting view (id or name) the callout points to instead of a new detail view.


<a id="cc_engine-commands-callout-calloutremove"></a>

### struct `CalloutRemove`

```rust
pub struct CalloutRemove
```

Remove a view's callout from its parent; the view stays (a detail view
keeps its crop and becomes an ordinary view of its kind).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  The detail or drafting view the callout stands for (id or name).


<a id="cc_engine-commands-callout-calloutslist"></a>

### struct `CalloutsList`

```rust
pub struct CalloutsList
```

The callouts drawn in a view: the detail or drafting view each stands
for and its rectangle.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  The parent view (id or name).


<a id="mod-cc_engine-commands-component"></a>

## `cc_engine::commands::component`

<sub>`crates/cc_engine/src/commands/component.rs`</sub>

Editing a component's definition piece by piece: parameters, reference
planes and lines, and dimensions between planes that parameters drive
(see `cc_core::component_ref`). Every command names a component type;
the change goes to every type of that component in one undo step, each
type keeping its own parameter values (a value set here changes only the
type named).

**Items:** [`edit_def`](#cc_engine-commands-component-edit_def) · [`check_expr`](#cc_engine-commands-component-check_expr) · [`number`](#cc_engine-commands-component-number) · [`sketch_plane`](#cc_engine-commands-component-sketch_plane) · [`ComponentGet`](#cc_engine-commands-component-componentget) · [`ComponentParamAdd`](#cc_engine-commands-component-componentparamadd) · [`ComponentParamSet`](#cc_engine-commands-component-componentparamset) · [`ComponentParamDelete`](#cc_engine-commands-component-componentparamdelete) · [`ComponentRefPlaneAdd`](#cc_engine-commands-component-componentrefplaneadd) · [`ComponentRefPlaneSet`](#cc_engine-commands-component-componentrefplaneset) · [`ComponentRefPlaneDelete`](#cc_engine-commands-component-componentrefplanedelete) · [`ComponentRefLineAdd`](#cc_engine-commands-component-componentreflineadd) · [`ComponentRefLineDelete`](#cc_engine-commands-component-componentreflinedelete) · [`ComponentDimensionAdd`](#cc_engine-commands-component-componentdimensionadd) · [`ComponentDimensionSet`](#cc_engine-commands-component-componentdimensionset) · [`ComponentDimensionDelete`](#cc_engine-commands-component-componentdimensiondelete) · [`ComponentEqualAdd`](#cc_engine-commands-component-componentequaladd) · [`ComponentEqualDelete`](#cc_engine-commands-component-componentequaldelete)

<a id="cc_engine-commands-component-edit_def"></a>

### fn `edit_def`

```rust
pub(crate) fn edit_def<T>(s: &mut Session, r: &Ref, renames: &[(String, String)], change: impl FnOnce(&mut ComponentDef) -> Result<T>) -> Result<T>
```

Change the definition of the component of type `r` with `change`, check
it and write it to every type of the component in one undo step. The
type named takes the new parameter values; the others keep theirs
(following `renames`, `[old, new]`), and take new parameters' values.

<a id="cc_engine-commands-component-check_expr"></a>

### fn `check_expr`

```rust
pub(crate) fn check_expr(text: &str) -> Result<String>
```

An expression that must evaluate: a number, or names of the component.

<a id="cc_engine-commands-component-number"></a>

### fn `number`

```rust
pub(crate) fn number(v: f64) -> String
```

A number written so the expression parser reads it back exactly.

<a id="cc_engine-commands-component-sketch_plane"></a>

### fn `sketch_plane`

```rust
pub(crate) fn sketch_plane(name: &str) -> Result<SketchPlane>
```

<a id="cc_engine-commands-component-componentget"></a>

### struct `ComponentGet`

```rust
pub struct ComponentGet
```

The component of a type, as the modeling environment shows it: the
definition, the value of every parameter and reference plane, the value
of each dimension, where each connector is, the problems found, and the
component's types with their parameter values. A title block's pictures
show as `format`, `width`, `height` (pixels) and `bytes`, not their bytes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.


<a id="cc_engine-commands-component-componentparamadd"></a>

### struct `ComponentParamAdd`

```rust
pub struct ComponentParamAdd
```

Add a parameter to a component (to every type, with the same value).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: String`

  Name used in expressions (letters, digits and `_`, not starting with a digit).

- `value: Option<f64>`

  Value (meters for lengths, degrees for angles; default 0).

- `kind: Option<String>`

  `length` (default), `angle`, `number` or `text` (a title block's
  per-sheet field, with `instance`).

- `instance: Option<bool>`

  Elements may override the value (sheets, for a title block's text parameter).

- `text: Option<String>`

  Value of a `text` parameter (default empty).


<a id="cc_engine-commands-component-componentparamset"></a>

### struct `ComponentParamSet`

```rust
pub struct ComponentParamSet
```

Change a component parameter: its value (for the type named only), kind,
whether elements may override it, or its name (every expression,
dimension and type follows a rename).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name (the value changes for this type).

- `name: String`

  The parameter's current name.

- `value: Option<f64>`

  New value for the type named.

- `kind: Option<String>`

  `length`, `angle` or `number`.

- `instance: Option<bool>`

  Elements may override the value.

- `rename: Option<String>`

  New name.


<a id="cc_engine-commands-component-componentparamdelete"></a>

### struct `ComponentParamDelete`

```rust
pub struct ComponentParamDelete
```

Remove a component parameter; refused while an expression reads it.
Dimensions it labels lose the label (their planes keep their place).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: String`

  The parameter's name.


<a id="cc_engine-commands-component-componentrefplaneadd"></a>

### struct `ComponentRefPlaneAdd`

```rust
pub struct ComponentRefPlaneAdd
```

Add a reference plane to a component: square to `axis` (`x`: a left/right
plane, `y`: front/back, `z`: a height), at the position `at`, an
expression that may read parameters and other planes (`Left + Width`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `axis: String`

  `x`, `y` or `z`.

- `at: String`

  Position along the axis (meters): a number or an expression.

- `name: Option<String>`

  Name (default `Plane1`, `Plane2`…).


<a id="cc_engine-commands-component-componentrefplaneset"></a>

### struct `ComponentRefPlaneSet`

```rust
pub struct ComponentRefPlaneSet
```

Move or rename a reference plane: `at` is its new position (an
expression); a rename follows into every expression and dimension.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: String`

  The plane's current name.

- `at: Option<String>`

  New position: a number or an expression.

- `rename: Option<String>`

  New name.


<a id="cc_engine-commands-component-componentrefplanedelete"></a>

### struct `ComponentRefPlaneDelete`

```rust
pub struct ComponentRefPlaneDelete
```

Remove a reference plane and the dimensions on it; refused while an
expression reads it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: String`

  The plane's name.


<a id="cc_engine-commands-component-componentreflineadd"></a>

### struct `ComponentRefLineAdd`

```rust
pub struct ComponentRefLineAdd
```

Add a reference line on one of the sketch planes (`plan`, `front` or
`side`, standing at `offset` along its normal axis): its ends are
`[u, v]` pairs of expressions in that plane.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `plane: String`

  `plan`, `front` or `side`.

- `start: [String; 2]`

  First end `[u, v]`: numbers or expressions.

- `end: [String; 2]`

  Second end `[u, v]`.

- `offset: Option<String>`

  Position of the sketch plane along its normal axis (default 0).

- `name: Option<String>`

  Name (default `Line1`, `Line2`…).


<a id="cc_engine-commands-component-componentreflinedelete"></a>

### struct `ComponentRefLineDelete`

```rust
pub struct ComponentRefLineDelete
```

Remove a reference line.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: String`

  The line's name.


<a id="cc_engine-commands-component-componentdimensionadd"></a>

### struct `ComponentDimensionAdd`

```rust
pub struct ComponentDimensionAdd
```

Dimension two parallel reference planes in a view (`plan`, `front` or
`side`, one that sees them edge-on). With `label`, a length parameter
drives it: the second plane moves to the first plus the parameter (the
parameter is created with the current distance when missing).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `view: String`

  `plan`, `front` or `side`.

- `from: String`

  The plane it measures from.

- `to: String`

  The plane it measures to (the one a label moves).

- `label: Option<String>`

  Parameter that drives the distance.

- `place: Option<f64>`

  Where the dimension line sits across the planes (meters, default 0).


<a id="cc_engine-commands-component-componentdimensionset"></a>

### struct `ComponentDimensionSet`

```rust
pub struct ComponentDimensionSet
```

Change a dimension between reference planes: label it with a parameter
(`label`, created when missing), take its label off (`label: ""`, the
planes stay), move its line (`place`) or give it a new `value` (the
label's parameter for this type, or the moving plane's position).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The dimension's index (from `component_get`).

- `label: Option<String>`

  Parameter that drives it; empty to take the label off.

- `value: Option<f64>`

  New distance (meters).

- `place: Option<f64>`

  Where the dimension line sits across the planes.


<a id="cc_engine-commands-component-componentdimensiondelete"></a>

### struct `ComponentDimensionDelete`

```rust
pub struct ComponentDimensionDelete
```

Remove a dimension between reference planes (the planes stay).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The dimension's index.


<a id="cc_engine-commands-component-componentequaladd"></a>

### struct `ComponentEqualAdd`

```rust
pub struct ComponentEqualAdd
```

Keep the plane `middle` halfway between the parallel planes `a` and `b`
(an equality, EQ): its position becomes `(a + b) / 2`, so it follows
whatever moves them. Shown in `view` (`plan`, `front` or `side`) as two
equal dimensions.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `view: String`

  `plan`, `front` or `side`.

- `a: String`

  One outer plane.

- `middle: String`

  The plane kept halfway (not an origin plane).

- `b: String`

  The other outer plane.

- `place: Option<f64>`

  Where the dimensions' line sits across the planes (meters, default 0).


<a id="cc_engine-commands-component-componentequaldelete"></a>

### struct `ComponentEqualDelete`

```rust
pub struct ComponentEqualDelete
```

Remove an equality (EQ); its middle plane stays halfway as written.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The equality's index (from `component_get`).


<a id="mod-cc_engine-commands-component_connector"></a>

## `cc_engine::commands::component_connector`

<sub>`crates/cc_engine/src/commands/component_connector.rs`</sub>

Connectors of a component: the points where pipes and conduits join it
(a toilet's water inlet and sewer outlet, an outlet's electrical
connection), each with the kind of system it takes, a position and a
diameter as expressions, and the direction runs leave it. A reference
plane's name in a coordinate locks the connector to that plane.

**Items:** [`DIRECTIONS`](#cc_engine-commands-component_connector-directions) · [`direction_name`](#cc_engine-commands-component_connector-direction_name) · [`ComponentConnectorAdd`](#cc_engine-commands-component_connector-componentconnectoradd) · [`ComponentConnectorSet`](#cc_engine-commands-component_connector-componentconnectorset) · [`ComponentConnectorDelete`](#cc_engine-commands-component_connector-componentconnectordelete)

<a id="cc_engine-commands-component_connector-directions"></a>

### const `DIRECTIONS`

```rust
pub const DIRECTIONS: [(&str, ConnectorDirection); 6]
```

Names of the connector directions in commands.

<a id="cc_engine-commands-component_connector-direction_name"></a>

### fn `direction_name`

```rust
pub fn direction_name(d: ConnectorDirection) -> &'static str
```

The command name of a direction.

<a id="cc_engine-commands-component_connector-componentconnectoradd"></a>

### struct `ComponentConnectorAdd`

```rust
pub struct ComponentConnectorAdd
```

Add a connector to a component: where a pipe or conduit of `system`
joins it. `x`, `y` and `z` place it in the component's frame (numbers or
expressions; a reference plane's name locks it to that plane), and
`direction` is the way runs leave it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `name: Option<String>`

  Name (default `Connector1`, `Connector2`…).

- `system: Option<String>`

  Kind of system: `cold_water` (default), `hot_water`, `sanitary`,
  `vent`, `rainwater`, `power` or `lighting`.

- `x: Option<ExprArg>`

  Position across the component (default 0).

- `y: Option<ExprArg>`

  Position front to back (default 0).

- `z: Option<ExprArg>`

  Height (default 0).

- `direction: Option<String>`

  Which way runs leave it: `down` (default), `up`, `front`, `back`, `left` or `right`.

- `diameter: Option<ExprArg>`

  Outside diameter of the run it takes, meters (default: unset).


<a id="cc_engine-commands-component_connector-componentconnectorset"></a>

### struct `ComponentConnectorSet`

```rust
pub struct ComponentConnectorSet
```

Change a connector of a component: its name, kind of system, position
(`x`, `y`, `z`, numbers or expressions; a reference plane's name locks it
to that plane), direction or diameter (empty to clear it).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The connector's index (from `component_get`).

- `name: Option<String>`

  New name.

- `system: Option<String>`

  Kind of system: `cold_water`, `hot_water`, `sanitary`, `vent`, `rainwater`, `power` or `lighting`.

- `x: Option<ExprArg>`

  Position across the component.

- `y: Option<ExprArg>`

  Position front to back.

- `z: Option<ExprArg>`

  Height.

- `direction: Option<String>`

  Which way runs leave it: `down`, `up`, `front`, `back`, `left` or `right`.

- `diameter: Option<ExprArg>`

  Outside diameter of the run it takes, meters (empty to clear it).


<a id="cc_engine-commands-component_connector-componentconnectordelete"></a>

### struct `ComponentConnectorDelete`

```rust
pub struct ComponentConnectorDelete
```

Remove a connector of a component.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The connector's index.


<a id="mod-cc_engine-commands-component_piece"></a>

## `cc_engine::commands::component_piece`

<sub>`crates/cc_engine/src/commands/component_piece.rs`</sub>

Pieces of a component drawn as sketches: an outline on the plan, front
or side plane extruded along the plane's normal, revolved about an axis
in it, swept along a path or blended into a second outline; solid or
void. Coordinates are expressions, so a reference
plane's name locks a point (or the start and end) to that plane.

**Items:** [`ExprArg`](#cc_engine-commands-component_piece-exprarg) · [`SketchPointArg`](#cc_engine-commands-component_piece-sketchpointarg) · [`BezierArg`](#cc_engine-commands-component_piece-bezierarg) · [`EllipseArg`](#cc_engine-commands-component_piece-ellipsearg) · [`profile_points`](#cc_engine-commands-component_piece-profile_points) · [`ComponentPieceAdd`](#cc_engine-commands-component_piece-componentpieceadd) · [`ComponentPieceSet`](#cc_engine-commands-component_piece-componentpieceset) · [`depth_end`](#cc_engine-commands-component_piece-depth_end) · [`ComponentPieceDelete`](#cc_engine-commands-component_piece-componentpiecedelete)

<a id="cc_engine-commands-component_piece-exprarg"></a>

### enum `ExprArg`

```rust
pub enum ExprArg
```

A number or an expression of the component.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Number(f64)`
- `Text(String)`

**Methods**

- `pub(crate) fn text(&self) -> Result<String>`

<a id="cc_engine-commands-component_piece-sketchpointarg"></a>

### struct `SketchPointArg`

```rust
pub struct SketchPointArg
```

A point of a sketch: `u`, `v` in its plane (numbers or expressions; a
plane's name locks the coordinate to it), and `radius` to make the
segment to the next point an arc (positive bulging outwards),
`ellipse` a part of an ellipse, or `bezier` a cubic Bézier curve (a
spline's segment) by its two handles.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `u: ExprArg`
- `v: ExprArg`
- `radius: Option<ExprArg>`
- `ellipse: Option<EllipseArg>`
- `bezier: Option<BezierArg>`

**Methods**

- `pub fn new(u: impl Into<String>, v: impl Into<String>, radius: &str) -> Self`

  A straight or arc point (`radius` empty: straight).

- `pub fn of(p: &ProfilePoint) -> Self`

  The argument that writes `p` back.


<a id="cc_engine-commands-component_piece-bezierarg"></a>

### struct `BezierArg`

```rust
pub struct BezierArg
```

The two handles of a cubic Bézier segment to the next point.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `u1: ExprArg`

  The handle leaving the point, along the plane's U.

- `v1: ExprArg`

  The handle leaving the point, along the plane's V.

- `u2: ExprArg`

  The handle arriving at the next point, along U.

- `v2: ExprArg`

  The handle arriving at the next point, along V.


<a id="cc_engine-commands-component_piece-ellipsearg"></a>

### struct `EllipseArg`

```rust
pub struct EllipseArg
```

The ellipse the segment to the next point runs along.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `u: ExprArg`

  Center, along the plane's U.

- `v: ExprArg`

  Center, along the plane's V.

- `rx: ExprArg`

  Semi-axis along the ellipse's own first axis.

- `ry: ExprArg`

  Semi-axis along its second axis.

- `rotation: Option<ExprArg>`

  Angle of the first axis from U, degrees (default 0).

- `clockwise: bool`

  Runs clockwise from the point to the next (default counter-clockwise).


<a id="cc_engine-commands-component_piece-profile_points"></a>

### fn `profile_points`

```rust
pub(crate) fn profile_points(points: &[SketchPointArg]) -> Result<Vec<ProfilePoint>>
```

<a id="cc_engine-commands-component_piece-componentpieceadd"></a>

### struct `ComponentPieceAdd`

```rust
pub struct ComponentPieceAdd
```

Add a piece drawn as a sketch on the `plan`, `front` or `side` plane: an
`extrusion` of the closed outline from `start` to `end` along the plane's
normal (Z for the plan, Y for the front, X for the side); a
`revolution` about the line u = `axis` of the plane, `angle` degrees
(default 360); a `sweep` of the closed outline (`points`: u to the left
of the path, v along the normal) along the open `path` drawn on the
plane at `start`, mitered at its corners; or a `blend` from the outline
at `start` to the `top` outline at `end`. Coordinates are numbers or expressions; a reference
plane's name locks a coordinate (or `start`, `end`, `axis`) to that
plane, so the piece follows it. With `void`, the piece is cut out of the
solid pieces it overlaps.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `kind: Option<String>`

  `extrusion` (default), `revolution`, `sweep` or `blend`.

- `plane: String`

  `plan`, `front` or `side`.

- `points: Vec<SketchPointArg>`

  The closed outline, in order (a sweep's section, a blend's bottom).

- `path: Option<Vec<SketchPointArg>>`

  Sweeps: the open path the section follows.

- `top: Option<Vec<SketchPointArg>>`

  Blends: the closed outline at `end`.

- `start: Option<ExprArg>`

  Where the sketch plane stands along its normal (default 0).

- `end: Option<ExprArg>`

  Extrusions and blends: where they end along the normal (default 0.1 past `start`).

- `axis: Option<ExprArg>`

  Revolutions: the axis, u = `axis` in the sketch plane.

- `angle: Option<ExprArg>`

  Revolutions: degrees turned (default 360).

- `void: bool`

  Cut out of the solid pieces it overlaps.

- `name: Option<String>`

  Name (default `Piece1`, `Piece2`…).

- `material: Option<String>`

  Project material name.


<a id="cc_engine-commands-component_piece-componentpieceset"></a>

### struct `ComponentPieceSet`

```rust
pub struct ComponentPieceSet
```

Change a piece of a component: its name, material, whether it is a void
or drawn in plans, and for sketched pieces their outline, `start`, `end`
(extrusions), `axis` and `angle` (revolutions).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The piece's index (from `component_get`).

- `name: Option<String>`

  New name.

- `material: Option<String>`

  Project material name (empty for the default color).

- `void: Option<bool>`

  Cut out of the solid pieces it overlaps.

- `in_plan: Option<bool>`

  Drawn in plans.

- `detail: Option<Vec<String>>`

  The detail levels of the views that show the piece: any of
  `coarse`, `medium` and `fine` (all three by default).

- `points: Option<Vec<SketchPointArg>>`

  New outline (a sweep's section, a blend's bottom).

- `path: Option<Vec<SketchPointArg>>`

  Sweeps: new path.

- `top: Option<Vec<SketchPointArg>>`

  Blends: new top outline.

- `start: Option<ExprArg>`

  Where the sketch plane stands along its normal.

- `end: Option<ExprArg>`

  Extrusions: where they end along the normal.

- `axis: Option<ExprArg>`

  Revolutions: the axis, u = `axis` in the sketch plane.

- `angle: Option<ExprArg>`

  Revolutions: degrees turned.


<a id="cc_engine-commands-component_piece-depth_end"></a>

### fn `depth_end`

```rust
pub fn depth_end(start: &str, depth: &str) -> String
```

Where an extrusion ends, as an expression: `start + depth`, read back
plainly when the depth was written as `end - start`.

<a id="cc_engine-commands-component_piece-componentpiecedelete"></a>

### struct `ComponentPieceDelete`

```rust
pub struct ComponentPieceDelete
```

Remove a piece of a component (it may be left with none while it is shaped).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `index: usize`

  The piece's index.


<a id="mod-cc_engine-commands-component_script"></a>

## `cc_engine::commands::component_script`

<sub>`crates/cc_engine/src/commands/component_script.rs`</sub>

Scripted components: writing a script piece (its declarations become the
component's parameters and hotspots), reading and setting parameter
values of a type or of placed elements, and moving hotspots. See
`cc_script::object` for the scripts themselves.

**Items:** [`ComponentScriptSet`](#cc_engine-commands-component_script-componentscriptset) · [`script_errors`](#cc_engine-commands-component_script-script_errors) · [`ComponentParamsGet`](#cc_engine-commands-component_script-componentparamsget) · [`ComponentParamsSet`](#cc_engine-commands-component_script-componentparamsset) · [`ComponentHotspotMove`](#cc_engine-commands-component_script-componenthotspotmove)

<a id="cc_engine-commands-component_script-componentscriptset"></a>

### struct `ComponentScriptSet`

```rust
pub struct ComponentScriptSet
```

Write the Lua source of a script piece of a component: the piece named,
or a new script piece. What the script declares becomes the component's
parameters and hotspots (values of those still declared are kept); a
script that does not load is refused, one that fails while building its
pieces is kept and its error returned.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the component: id or name.

- `source: String`

  The Lua source: `param.length { "width", default = 1 }` declarations and a
  `geometry(p)` function returning pieces (`box`, `cylinder`, `extrusion`,
  `profile`, `revolution`, `sweep`, `blend`, `mesh`).

- `piece: Option<String>`

  The script piece to change; a new piece when left out.

- `name: Option<String>`

  Name of a new piece (default `Script1`, `Script2`…).

- `material: Option<String>`

  Material of the pieces that set none (project material name).


<a id="cc_engine-commands-component_script-script_errors"></a>

### fn `script_errors`

```rust
pub(crate) fn script_errors(p: &Project, ty: ElementTypeId, def: &ComponentDef, element: Option<ElementId>) -> Vec<String>
```

The errors of a type's script pieces, run with the type's values or an
element's.

<a id="cc_engine-commands-component_script-componentparamsget"></a>

### struct `ComponentParamsGet`

```rust
pub struct ComponentParamsGet
```

The parameters and hotspots of a component type, with the type's values;
or of placed components (all of one component), with the value they
share or `mixed` where they differ.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Option<Ref>`

  A type of the component: id or name (leave out with `elements`).

- `elements: Vec<u64>`

  Placed components of one component (leave out with `type`).


<a id="cc_engine-commands-component_script-componentparamsset"></a>

### struct `ComponentParamsSet`

```rust
pub struct ComponentParamsSet
```

Set a parameter of a component type (every element of the type follows)
or the instance value of placed components (all of one component; the
parameter must be an instance one). Values: numbers (meters, degrees),
true/false, texts, choice keys or material names. One undo step; when
any target refuses the value nothing changes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Option<Ref>`

  A type of the component: id or name (leave out with `elements`).

- `elements: Vec<u64>`

  Placed components of one component (leave out with `type`).

- `name: String`

  The parameter's name.

- `value: Value`

  The new value.


<a id="cc_engine-commands-component_script-componenthotspotmove"></a>

### struct `ComponentHotspotMove`

```rust
pub struct ComponentHotspotMove
```

Move a hotspot of a placed component (what its script reads as
`p.<name>`). The point is in the component's frame (meters from the
insertion point, x to its right, y to its back, z up) unless `world`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: u64`

  The placed component.

- `name: String`

  The hotspot's name.

- `point: [f64; 3]`

  The new position `[x, y, z]`.

- `world: bool`

  The point is in project coordinates (z absolute) instead of the component's frame.


<a id="mod-cc_engine-commands-connection"></a>

## `cc_engine::commands::connection`

<sub>`crates/cc_engine/src/commands/connection.rs`</sub>

Steel and timber connections (see `cc_core::connection`): placing them
on member ends, the checks `element_set` runs on them and the values
`element_get` derives.

**Items:** [`new_connection_type`](#cc_engine-commands-connection-new_connection_type) · [`params_arg`](#cc_engine-commands-connection-params_arg) · [`ConnectionsCreate`](#cc_engine-commands-connection-connectionscreate) · [`check`](#cc_engine-commands-connection-check) · [`derived`](#cc_engine-commands-connection-derived)

<a id="cc_engine-commands-connection-new_connection_type"></a>

### fn `new_connection_type`

```rust
pub fn new_connection_type(id: ElementTypeId) -> ElementType
```

The type a project gets with its first connection.

<a id="cc_engine-commands-connection-params_arg"></a>

### fn `params_arg`

```rust
pub(crate) fn params_arg(kind: ConnectionKind, patch: &Value) -> Result<ConnectionParams>
```

`kind`'s defaults with `patch` merged in, every name known and every value in range.

<a id="cc_engine-commands-connection-connectionscreate"></a>

### struct `ConnectionsCreate`

```rust
pub struct ConnectionsCreate
```

Place steel or timber connections of one `kind` at every member end that
suits it: `base_plate` under columns; `end_plate` and `timber_bolts` at
beam ends that stop at a column; `fin_plate` at beam ends at a column or
another beam; `hanger` at beam ends at another beam. Ends that already
have a connection are skipped. Plates and bolts follow their member.
One undo step; returns the new ids (`created`) and how many ends were
`skipped`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `kind: String`

  `base_plate`, `end_plate`, `fin_plate`, `timber_bolts` or `hanger`.

- `members: Vec<u64>`

  Column ids (base plates) or beam ids (the other kinds); default: every one of `level`.

- `level: Option<Ref>`

  Level id or name when no `members` are given (default: the active level, else every level); ignored with `members`.

- `params: Option<Value>`

  Sizes to change from the kind's defaults, meters: `thickness`, `width` and `length` (0: from the
  member's section plus `edge`), `bolt_diameter`, `rows` and `columns` (1 to 10), `spacing`, `edge`,
  `anchor_length`. Lengths are 0.001 to 2.


<a id="cc_engine-commands-connection-check"></a>

### fn `check`

```rust
pub(crate) fn check(old: &Connection, new: &Connection) -> Result<()>
```

`element_set` on a connection: it keeps its member, end and kind, and
its sizes stay in range.

<a id="cc_engine-commands-connection-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(g: &cc_geom::connection::ConnectionGeom, out: &mut serde_json::Map<String, Value>)
```

`element_get`'s derived values of a connection.

<a id="mod-cc_engine-commands-create"></a>

## `cc_engine::commands::create`

<sub>`crates/cc_engine/src/commands/create.rs`</sub>

Creating elements, with the same defaults as the drawing tools.

**Items:** [`top`](#cc_engine-commands-create-top) · [`create`](#cc_engine-commands-create-create) · [`WallCreate`](#cc_engine-commands-create-wallcreate) · [`arc_bulge`](#cc_engine-commands-create-arc_bulge) · [`check_bulge`](#cc_engine-commands-create-check_bulge) · [`check_arc`](#cc_engine-commands-create-check_arc) · [`check_beam_arc`](#cc_engine-commands-create-check_beam_arc) · [`HostedArgs`](#cc_engine-commands-create-hostedargs) · [`hosted_at`](#cc_engine-commands-create-hosted_at) · [`DoorCreate`](#cc_engine-commands-create-doorcreate) · [`WindowCreate`](#cc_engine-commands-create-windowcreate) · [`SketchArgs`](#cc_engine-commands-create-sketchargs) · [`sketched`](#cc_engine-commands-create-sketched) · [`sketch`](#cc_engine-commands-create-sketch) · [`FloorCreate`](#cc_engine-commands-create-floorcreate) · [`CeilingCreate`](#cc_engine-commands-create-ceilingcreate) · [`RoofCreate`](#cc_engine-commands-create-roofcreate) · [`check_roof_holes`](#cc_engine-commands-create-check_roof_holes) · [`check_openings`](#cc_engine-commands-create-check_openings) · [`RoofEdgeSet`](#cc_engine-commands-create-roofedgeset) · [`FloorOpeningAdd`](#cc_engine-commands-create-flooropeningadd) · [`FloorOpeningRemove`](#cc_engine-commands-create-flooropeningremove) · [`RoofOpeningRemove`](#cc_engine-commands-create-roofopeningremove) · [`RoomCreate`](#cc_engine-commands-create-roomcreate) · [`grids`](#cc_engine-commands-create-grids) · [`GridCreate`](#cc_engine-commands-create-gridcreate) · [`ColumnCreate`](#cc_engine-commands-create-columncreate) · [`BeamCreate`](#cc_engine-commands-create-beamcreate) · [`justification`](#cc_engine-commands-create-justification) · [`RailingCreate`](#cc_engine-commands-create-railingcreate) · [`ComponentPlace`](#cc_engine-commands-create-componentplace)

<a id="cc_engine-commands-create-top"></a>

### fn `top`

```rust
pub(crate) fn top(p: &Project, top_level: Option<&Ref>, top_offset: Option<f64>) -> Result<Option<WallTop>>
```

A top constraint: attached to a level (with an offset) or a fixed height.

<a id="cc_engine-commands-create-create"></a>

### fn `create`

```rust
pub(crate) fn create(s: &mut Session, tx: String, level: LevelId, category: Category, ty: Option<&Ref>, kind: ElementKind) -> Result<Value>
```

<a id="cc_engine-commands-create-wallcreate"></a>

### struct `WallCreate`

```rust
pub struct WallCreate
```

Create a wall from `start` to `end` on a level: straight, or an arc
through `mid` (or with a `bulge` or a `radius`). Its top is `height` above its base,
or attached to `top_level` (plus `top_offset`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start point of the wall line.

- `end: Point`

  End point of the wall line.

- `mid: Option<Point>`

  A point on the arc between the ends: makes an arc wall through the three points.

- `bulge: Option<f64>`

  Arc walls: tan(sweep / 4), positive when the arc turns counter-clockwise from `start`
  to `end` (1 is a half circle; at most 10 either way). Ignored when `mid` is given.

- `radius: Option<f64>`

  Arc walls: the radius in meters, positive when the arc turns counter-clockwise from
  `start` to `end`, negative clockwise; at least half the distance between the ends.
  Ignored when `mid` or `bulge` is given.

- `major_arc: Option<bool>`

  With `radius`: take the longer of the two arcs (default: the shorter).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Wall type id or name (default: the default wall type).

- `height: Option<f64>`

  Unconnected height in meters (default 3; 10'-0" in imperial projects).

- `top_level: Option<Ref>`

  Level the top is attached to, instead of a height.

- `top_offset: Option<f64>`

  Offset of the top from `top_level`, meters.

- `base_offset: Option<f64>`

  Base offset from the level, meters.

- `location_line: Option<String>`

  Which line of the wall `start`-`end` is: `Centerline` (default), `CoreCenter`,
  `FinishExterior`, `FinishInterior`, `CoreExterior` or `CoreInterior`.


<a id="cc_engine-commands-create-arc_bulge"></a>

### fn `arc_bulge`

```rust
pub(crate) fn arc_bulge(start: glam::DVec2, end: glam::DVec2, mid: Option<Point>, bulge: Option<f64>, radius: Option<f64>, major: Option<bool>) -> Result<f64>
```

The bulge of the line or arc from `start` to `end`: through `mid`, else
`bulge`, else of `radius` (signed: positive counter-clockwise; the longer
arc when `major`), else 0 (straight).

<a id="cc_engine-commands-create-check_bulge"></a>

### fn `check_bulge`

```rust
pub(crate) fn check_bulge(bulge: f64) -> Result<f64>
```

A bulge within the range arcs accept.

<a id="cc_engine-commands-create-check_arc"></a>

### fn `check_arc`

```rust
pub(crate) fn check_arc(p: &Project, type_id: cc_core::ElementTypeId, wall: &Wall) -> Result<()>
```

An arc wall must leave room inside its curve for its thickness.

<a id="cc_engine-commands-create-check_beam_arc"></a>

### fn `check_beam_arc`

```rust
pub(crate) fn check_beam_arc(p: &Project, type_id: cc_core::ElementTypeId, beam: &Beam) -> Result<()>
```

An arc beam is level and must leave room inside its curve for its width.

<a id="cc_engine-commands-create-hostedargs"></a>

### struct `HostedArgs`

```rust
pub struct HostedArgs
```

Parameters shared by doors and windows.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `wall: Option<u64>`

  Id of the host wall; omit it to use the wall nearest `point`.

- `offset: Option<f64>`

  Distance along the wall from its start to the opening's center, meters.

- `point: Option<Point>`

  A point near the wall: the opening is centered at its projection, on its side.

- `sill: Option<f64>`

  Sill height (default: the type's default sill for windows, 0 for doors).

- `type_ref: Option<Ref>`

  Door or window type id or name (default: the default type).

- `flip_facing: Option<bool>`

  Face the other side of the wall.

- `flip_hand: Option<bool>`

  Hinge on the other jamb.


<a id="cc_engine-commands-create-hosted_at"></a>

### fn `hosted_at`

```rust
pub(crate) fn hosted_at(p: &Project, a: &HostedArgs, ty: &cc_core::ElementType) -> Result<(Hosted, LevelId)>
```

Where an element of type `ty` (a door, a window, a niche) goes in the
wall the arguments name, and that wall's level.

<a id="cc_engine-commands-create-doorcreate"></a>

### struct `DoorCreate`

```rust
pub struct DoorCreate
```

Place a door in a wall (centered on the wall unless `offset` or `point` is given).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `args: HostedArgs`

<a id="cc_engine-commands-create-windowcreate"></a>

### struct `WindowCreate`

```rust
pub struct WindowCreate
```

Place a window in a wall (centered on the wall unless `offset` or `point` is given).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `args: HostedArgs`

<a id="cc_engine-commands-create-sketchargs"></a>

### struct `SketchArgs`

```rust
pub struct SketchArgs
```

Parameters shared by floors, ceilings and roofs.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `boundary: Option<Vec<Point>>`

  Closed outline (at least 3 points); omit with `from_walls` or `curves`.

- `curves: Option<Vec<super::sketch::CurveArg>>`

  The outline drawn with curves (lines, arcs, circles, ellipses, rectangles,
  polygons) instead of `boundary`; loops inside the outline are openings
  (floors and roofs). The element keeps the curves (see `sketch_get`).

- `from_walls: bool`

  Follow the outer boundary of the walls of the level, leaving interior walls out
  (exterior faces; interior faces for ceilings).

- `wall_face: Option<String>`

  With `from_walls`: the face of each wall the outline follows, `FinishExterior`, `FinishInterior`,
  `CoreExterior`, `CoreInterior`, `CoreCenter` or `Centerline`, where exterior is the side outside the
  loop of walls and the core is a wall's structure layers. Each side goes to that face of its own wall.
  Without it the outline is the walls' boundary moved out (in, for ceilings) by half the thickest wall.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `offset: Option<f64>`

  Floors: top above the level (default 0). Ceilings: bottom above the level (default 2.6; 8'-0" in
  imperial projects).
  Roofs: base above the level (default: top of the tallest wall).


<a id="cc_engine-commands-create-sketched"></a>

### fn `sketched`

```rust
pub(crate) fn sketched(s: &Session, a: &SketchArgs, category: Category) -> Result<(LevelId, cc_core::sketch::Profile, Option<cc_core::sketch::Sketch>)>
```

The level, outline and openings of a sketched element, and the curves it
was drawn with (`curves`).

<a id="cc_engine-commands-create-sketch"></a>

### fn `sketch`

```rust
pub(crate) fn sketch(s: &Session, a: &SketchArgs, category: Category) -> Result<(LevelId, Vec<DVec2>)>
```

The level and outline of an element with no openings.

<a id="cc_engine-commands-create-floorcreate"></a>

### struct `FloorCreate`

```rust
pub struct FloorCreate
```

Create a floor slab from an outline (or the loop of walls).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `sketch: SketchArgs`
- `openings: Vec<Vec<Point>>`

  Openings through the slab (shafts, stairwells): closed outlines inside
  the boundary, apart from each other.

- `structural: Option<bool>`

  A structural slab, one that takes reinforcement (default false).


<a id="cc_engine-commands-create-ceilingcreate"></a>

### struct `CeilingCreate`

```rust
pub struct CeilingCreate
```

Create a ceiling from an outline (or the inside of the loop of walls).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `sketch: SketchArgs`

<a id="cc_engine-commands-create-roofcreate"></a>

### struct `RoofCreate`

```rust
pub struct RoofCreate
```

Create a roof over an outline (or the loop of walls): flat by default; change its
shape (shed, gable, hip) with `element_set`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `sketch: SketchArgs`
- `overhang: Option<f64>`

  Overhang beyond the outline, meters (default 0.5).

- `holes: Vec<Vec<Point>>`

  Openings through the roof (courtyards): closed outlines inside the boundary,
  apart from each other. Pitched roofs slope down to their eaves too.


<a id="cc_engine-commands-create-check_roof_holes"></a>

### fn `check_roof_holes`

```rust
pub(crate) fn check_roof_holes(roof: &Roof) -> Result<()>
```

A roof's openings must be outlines with an area, inside its footprint
(clear of its edges) and apart from each other.

<a id="cc_engine-commands-create-check_openings"></a>

### fn `check_openings`

```rust
pub(crate) fn check_openings(boundary: &[DVec2], openings: &[Vec<DVec2>]) -> Result<()>
```

Openings of a floor or roof must be outlines with an area, inside the
`boundary` (clear of its edges) and apart from each other.

<a id="cc_engine-commands-create-roofedgeset"></a>

### struct `RoofEdgeSet`

```rust
pub struct RoofEdgeSet
```

Set the slope of one edge of a gable or hip roof's outline (edge `i` runs
from outline point `i` to the next): `0` makes it a gable end, a positive
slope an eave rising at that slope, `null` returns it to the shape's own.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Roof id.

- `edge: usize`

  Edge index, from 0.

- `slope: Option<f64>`

  Slope as rise over run (0.5 = 50 %), `0` for a gable end, or `null` for the roof's slope.


<a id="cc_engine-commands-create-flooropeningadd"></a>

### struct `FloorOpeningAdd`

```rust
pub struct FloorOpeningAdd
```

Add an opening (a shaft, a stairwell) to a floor: a closed outline inside
its boundary, apart from its other openings.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Floor id.

- `outline: Vec<Point>`

  Outline of the opening, at least 3 points.


<a id="cc_engine-commands-create-flooropeningremove"></a>

### struct `FloorOpeningRemove`

```rust
pub struct FloorOpeningRemove
```

Remove one opening of a floor (by its index in the floor's `openings`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Floor id.

- `index: usize`

  Opening index, from 0.


<a id="cc_engine-commands-create-roofopeningremove"></a>

### struct `RoofOpeningRemove`

```rust
pub struct RoofOpeningRemove
```

Remove one opening of a roof (by its index in the roof's `holes`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Roof id.

- `index: usize`

  Opening index, from 0.


<a id="cc_engine-commands-create-roomcreate"></a>

### struct `RoomCreate`

```rust
pub struct RoomCreate
```

Create a room at a point enclosed by room-bounding walls (its boundary follows the walls).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `point: Point`

  A point inside the room.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `name: Option<String>`

  Default: the room default name.

- `number: Option<String>`

  Default: the next free number.

- `height: Option<f64>`

  Upper limit above the level (default: the level's height).


<a id="cc_engine-commands-create-grids"></a>

### fn `grids`

```rust
pub fn grids(p: &Project) -> impl Iterator<Item = (ElementId, &Grid)>
```

The grids of `p` with their ids.

<a id="cc_engine-commands-create-gridcreate"></a>

### struct `GridCreate`

```rust
pub struct GridCreate
```

Create a grid line. Without `name` it takes the next name of its series:
the one after the most recently placed grid parallel to it (`1` → `2`,
`A` → `B`, `Z` → `AA`, `A1` → `A2`, `1.1` → `1.2`); a grid across the
existing ones starts a new series (letters across numbered grids,
numbers otherwise). Names given this way are unique among the grids.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start point (where the bubble is).

- `end: Point`

  End point.

- `name: Option<String>`

  Grid name (default: the next name of its series).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).


<a id="cc_engine-commands-create-columncreate"></a>

### struct `ColumnCreate`

```rust
pub struct ColumnCreate
```

Place a column; its top goes to the level above (or `height` when there is none).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Center of the column in plan.

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `height: Option<f64>`

  Unconnected height (default: the level's height).

- `top_level: Option<Ref>`

  Level the top is attached to (default: the level above).

- `top_offset: Option<f64>`

  Offset of the top from `top_level`, meters.

- `structural: Option<bool>`

  Load-bearing (default true); an architectural column is cut by the walls it is joined with.


<a id="cc_engine-commands-create-beamcreate"></a>

### struct `BeamCreate`

```rust
pub struct BeamCreate
```

Create a beam from `start` to `end`, straight or, for an arc beam, through `mid` (or with a
`bulge` or a `radius`); its location line is `top_offset` above the level (default: the
level's height), at the top of the section unless `vertical` says otherwise. An arc beam is
level and joins columns and other beams at its ends like a straight one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the beam axis.

- `end: Point`

  End of the beam axis.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `top_offset: Option<f64>`

  Elevation of the location line above the level, meters (default: the level height).

- `end_offset: Option<f64>`

  Elevation of the location line at the end, when it differs from the start (a sloped beam).

- `lateral: Option<String>`

  Line across the section the axis follows: `center` (default), `left` or `right` face.

- `vertical: Option<String>`

  Height of the section at the location elevation: `top` (default), `center` or `bottom`.

- `mid: Option<Point>`

  Arc beams: a point the location line passes through between `start` and `end`.

- `bulge: Option<f64>`

  Arc beams: `tan(sweep / 4)`, positive when the arc turns counter-clockwise from `start`
  to `end` (1 is a half circle; at most 10 either way). Ignored when `mid` is given.

- `radius: Option<f64>`

  Arc beams: the radius in meters, positive when the arc turns counter-clockwise from
  `start` to `end`, negative when clockwise. Ignored when `mid` or `bulge` is given.

- `major_arc: Option<bool>`

  With `radius`: take the longer of the two arcs (default: the shorter).


<a id="cc_engine-commands-create-justification"></a>

### fn `justification`

```rust
pub(crate) fn justification<T: Copy>(value: Option<&str>, all: &[(&str, T)], what: &str) -> Result<Option<T>>
```

A beam justification by name.

<a id="cc_engine-commands-create-railingcreate"></a>

### struct `RailingCreate`

```rust
pub struct RailingCreate
```

Create a railing along a path (at least 2 points).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `path: Vec<Point>`

  Points of the railing path (at least 2).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `base_offset: Option<f64>`

  Base above the level, meters.


<a id="cc_engine-commands-create-componentplace"></a>

### struct `ComponentPlace`

```rust
pub struct ComponentPlace
```

Place a component (furniture, plumbing fixture, electrical device…) of a
component type. Electrical devices default to their usual mounting height
(outlets 0.30 m, switches 1.10 m, boards 1.30 m, wall lights 2.10 m,
ceiling lights at the level height, data outlets 0.30 m, racks 1.40 m,
air terminals on the roof under them, earth electrodes 0.50 m below the
level; in imperial projects outlets 1'-6", switches 4'-0", boards and
racks 4'-6", wall lights 7'-0") and may be put on a circuit or system of their kind; lightning
protection devices join the first lightning protection system by default.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `position: Point`

  Insertion point in plan.

- `type_ref: Ref`

  Component type id or name.

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `base_offset: Option<f64>`

  Base above the level, meters (default 0; for devices, their mounting height).

- `system: Option<Ref>`

  Circuit or system id, name or code to connect it to.


<a id="mod-cc_engine-commands-curtain"></a>

## `cc_engine::commands::curtain`

<sub>`crates/cc_engine/src/commands/curtain.rs`</sub>

Curtain walls: walls of a type with a curtain grid (see
`cc_core::curtain`). They are created with `wall_create` like other walls;
these commands make curtain wall types and adjust one wall's grid and panels.

**Items:** [`CurtainTypeCreate`](#cc_engine-commands-curtain-curtaintypecreate) · [`CurtainGet`](#cc_engine-commands-curtain-curtainget) · [`Axis`](#cc_engine-commands-curtain-axis) · [`CurtainGridSet`](#cc_engine-commands-curtain-curtaingridset) · [`PanelArg`](#cc_engine-commands-curtain-panelarg) · [`PanelKind`](#cc_engine-commands-curtain-panelkind) · [`CurtainPanelSet`](#cc_engine-commands-curtain-curtainpanelset)

<a id="cc_engine-commands-curtain-curtaintypecreate"></a>

### struct `CurtainTypeCreate`

```rust
pub struct CurtainTypeCreate
```

Create a curtain wall type: walls of it are a grid of panels and
mullions. `curtain` changes the defaults (vertical and horizontal lines
every 1.5 m, glass panels, 5 x 15 cm mullions inside and around; in
imperial projects lines every 5'-0" and 2" x 6" mullions) and is
shaped like `params.curtain` in `type_get`: `vertical` and `horizontal`
{layout: `None`, `FixedDistance`, `FixedNumber` or `MaxSpacing`,
spacing, count, justification: `Start`, `Center` or `End`}, `panel`
{kind: `Glass` or `Solid`, thickness, material} or {kind: `Empty`} or
{kind: `Component`, component: type id}, and `vertical_mullion`,
`horizontal_mullion`, `border_mullion` {width, depth, material} or null
for none. Place walls of it with `wall_create`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Type name (unique among wall types).

- `curtain: Option<Value>`

  Changes to the default grid, panel and mullions.


<a id="cc_engine-commands-curtain-curtainget"></a>

### struct `CurtainGet`

```rust
pub struct CurtainGet
```

Show a curtain wall's grid: its interior lines (`vertical`: distances
along the wall from its start; `horizontal`: heights above its base),
the panels by column and row (from the start and from the base; merged
cells give their lowest) with their extent, kind and whether they are
swapped, and its panel count, glazed area and mullion length.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `element: u64`

  The curtain wall's id.


<a id="cc_engine-commands-curtain-axis"></a>

### enum `Axis`

```rust
pub enum Axis
```

A direction of a curtain grid.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Vertical`

  Vertical lines, spaced along the wall.

- `Horizontal`

  Horizontal lines, spaced up from the base.


<a id="cc_engine-commands-curtain-curtaingridset"></a>

### struct `CurtainGridSet`

```rust
pub struct CurtainGridSet
```

Adjust one curtain wall's grid on one axis. Lines are numbered from 0 in
the order `curtain_get` lists them (`vertical` or `horizontal`); positions
are distances along the wall from its start for vertical lines and heights
above the base for horizontal ones. In order: `reset` goes back to the
type's lines; `lines` gives all of them; `move` moves lines `[index, to]`;
`remove` takes lines out; `add` puts lines in. Any of these gives the wall
its own lines (later type changes no longer move them). Segments are a
line's pieces between the lines across it (`[line, segment]`, segment
numbered from the base or from the start, as the grid was before this
command): `remove_segments` merges the cells on both sides into one panel
with no mullion between, `restore_segments` puts them back.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: u64`

  The curtain wall's id.

- `axis: Axis`

  Which lines: `Vertical` or `Horizontal`.

- `reset: bool`

  Go back to the type's lines (and drop removed segments of this axis).

- `lines: Option<Vec<f64>>`

  All the interior lines, replacing the current ones.

- `move_lines: Vec<(usize, f64)>`

  Lines to move, as `[index, new position]`.

- `remove: Vec<usize>`

  Indices of lines to remove.

- `add: Vec<f64>`

  Positions of new lines.

- `remove_segments: Vec<[usize; 2]>`

  Segments to remove, as `[line, segment]`.

- `restore_segments: Vec<[usize; 2]>`

  Removed segments to put back, as `[line, segment]`.


<a id="cc_engine-commands-curtain-panelarg"></a>

### struct `PanelArg`

```rust
pub struct PanelArg
```

What a curtain panel becomes.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `kind: PanelKind`

  `Glass`, `Solid`, `Empty` or `Component`.

- `thickness: Option<f64>`

  Panel thickness, meters (glass and solid; default 24 mm).

- `material: Option<Ref>`

  Material name or index (glass and solid; default the type panel's material).

- `component: Option<Ref>`

  The component type id or name (component panels): a door or window
  fills the panel's opening; `Width` and `Height` parameters take its size.


<a id="cc_engine-commands-curtain-panelkind"></a>

### enum `PanelKind`

```rust
pub enum PanelKind
```

The kinds of curtain panels.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Glass`
- `Solid`
- `Empty`
- `Component`

<a id="cc_engine-commands-curtain-curtainpanelset"></a>

### struct `CurtainPanelSet`

```rust
pub struct CurtainPanelSet
```

Swap panels of one curtain wall: the panels of cells `[column, row]`
(numbered from the start and from the base, as `curtain_get` lists them;
a cell of merged cells swaps all of them) become `panel`, or the type's
again when `panel` is null.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: u64`

  The curtain wall's id.

- `cells: Vec<[usize; 2]>`

  The cells, as `[column, row]`.

- `panel: Option<PanelArg>`

  The new panel (null: the type's).


<a id="mod-cc_engine-commands-design_options"></a>

## `cc_engine::commands::design_options`

<sub>`crates/cc_engine/src/commands/design_options.rs`</sub>

Design options: option sets, their options and primary option, the
option elements belong to, the option each view shows and the option
being edited (see `cc_core::design_option`).

**Items:** [`option_ref_json`](#cc_engine-commands-design_options-option_ref_json) · [`view_options_json`](#cc_engine-commands-design_options-view_options_json) · [`OptionsList`](#cc_engine-commands-design_options-optionslist) · [`OptionSetCreate`](#cc_engine-commands-design_options-optionsetcreate) · [`OptionSetRename`](#cc_engine-commands-design_options-optionsetrename) · [`OptionSetDelete`](#cc_engine-commands-design_options-optionsetdelete) · [`OptionCreate`](#cc_engine-commands-design_options-optioncreate) · [`OptionSetProps`](#cc_engine-commands-design_options-optionsetprops) · [`OptionDelete`](#cc_engine-commands-design_options-optiondelete) · [`OptionAcceptPrimary`](#cc_engine-commands-design_options-optionacceptprimary) · [`ElementsOptionSet`](#cc_engine-commands-design_options-elementsoptionset) · [`ViewOptionsSet`](#cc_engine-commands-design_options-viewoptionsset) · [`OptionChoice`](#cc_engine-commands-design_options-optionchoice) · [`choices`](#cc_engine-commands-design_options-choices) · [`OptionActiveSet`](#cc_engine-commands-design_options-optionactiveset)

<a id="cc_engine-commands-design_options-option_ref_json"></a>

### fn `option_ref_json`

```rust
pub(crate) fn option_ref_json(p: &Project, id: Option<OptionId>) -> Value
```

An option as other queries show it: `{id, name, set}`, or `null`.

<a id="cc_engine-commands-design_options-view_options_json"></a>

### fn `view_options_json`

```rust
pub(crate) fn view_options_json(p: &Project, v: &cc_core::View) -> Value
```

The option of each set a view shows, as `view_get` shows it: the
view's own choice (`null`: automatic) and the option shown.

<a id="cc_engine-commands-design_options-optionslist"></a>

### struct `OptionsList`

```rust
pub struct OptionsList
```

The project's design option sets with their options (the primary one
marked, how many elements each holds) and the option being edited.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-design_options-optionsetcreate"></a>

### struct `OptionSetCreate`

```rust
pub struct OptionSetCreate
```

Add a design option set: a named group of alternatives, of which views
show one at a time. It starts with the options named in `options` (the
first is primary), or one option.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique, without ':').

- `options: Vec<String>`

  Names of its options (default: one, "Option 1").


<a id="cc_engine-commands-design_options-optionsetrename"></a>

### struct `OptionSetRename`

```rust
pub struct OptionSetRename
```

Rename a design option set.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `set: Ref`

  Set id or name.

- `name: String`

  New name (unique, without ':').


<a id="cc_engine-commands-design_options-optionsetdelete"></a>

### struct `OptionSetDelete`

```rust
pub struct OptionSetDelete
```

Delete a design option set with all its options and the elements in
them (the main model stays).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `set: Ref`

  Set id or name.


<a id="cc_engine-commands-design_options-optioncreate"></a>

### struct `OptionCreate`

```rust
pub struct OptionCreate
```

Add an option to a design option set (at the end).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `set: Ref`

  Set id or name.

- `name: Option<String>`

  Name, unique in the set (default: "Option n").

- `primary: bool`

  Make it the set's primary option.


<a id="cc_engine-commands-design_options-optionsetprops"></a>

### struct `OptionSetProps`

```rust
pub struct OptionSetProps
```

Rename a design option, make it its set's primary option, or move it
to another place in the set's list (`index`, 0 = first).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `option: Ref`

  Option id, name, or "Set: Option".

- `name: Option<String>`

  New name (unique in the set, without ':').

- `primary: Option<bool>`

  `true` makes it the primary option (views showing the set
  automatically, and exports, then use it).

- `index: Option<usize>`

  New position in the set's list.


<a id="cc_engine-commands-design_options-optiondelete"></a>

### struct `OptionDelete`

```rust
pub struct OptionDelete
```

Delete a design option and the elements in it (with what they host).
The primary option cannot be deleted: make another option primary
first, or delete the whole set.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `option: Ref`

  Option id, name, or "Set: Option".


<a id="cc_engine-commands-design_options-optionacceptprimary"></a>

### struct `OptionAcceptPrimary`

```rust
pub struct OptionAcceptPrimary
```

Accept a set's primary option: its elements join the main model, and
the set is deleted with its other options and their elements.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `set: Ref`

  Set id or name.


<a id="cc_engine-commands-design_options-elementsoptionset"></a>

### struct `ElementsOptionSet`

```rust
pub struct ElementsOptionSet
```

Move elements into a design option, or back to the main model with
`null`. Doors, windows and other hosted elements follow their host's
option (moving a wall moves them); annotations belong to no option.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `option: Option<Ref>`

  Option id, name or "Set: Option"; `null` for the main model.


<a id="cc_engine-commands-design_options-viewoptionsset"></a>

### struct `ViewOptionsSet`

```rust
pub struct ViewOptionsSet
```

Choose the option a view (plan, elevation, section, 3D view or
schedule) shows of a design option set; `null` shows it automatically:
the option being edited when it is in that set, else the primary one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `set: Ref`

  Set id or name.

- `option: Option<Ref>`

  Option id or name (in that set), or `null` for automatic.


<a id="cc_engine-commands-design_options-optionchoice"></a>

### struct `OptionChoice`

```rust
pub struct OptionChoice
```

A choice of option for one set (templates).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `set: Ref`

  Set id or name.

- `option: Option<Ref>`

  Option id or name (in that set), or `null` for automatic.


<a id="cc_engine-commands-design_options-choices"></a>

### fn `choices`

```rust
pub(crate) fn choices(p: &Project, list: &[OptionChoice]) -> Result<BTreeMap<OptionSetId, OptionId>>
```

The choices as a template keeps them (automatic ones left out).

<a id="cc_engine-commands-design_options-optionactiveset"></a>

### struct `OptionActiveSet`

```rust
pub struct OptionActiveSet
```

Set the design option this session edits (`null`: the main model):
new model elements go into it, and views without a choice of their own
show it for its set. Display state: not saved, not undone.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `option: Option<Ref>`

  Option id, name or "Set: Option"; `null` for the main model.


<a id="mod-cc_engine-commands-detail"></a>

## `cc_engine::commands::detail`

<sub>`crates/cc_engine/src/commands/detail.rs`</sub>

Detail lines: 2D lines, arcs, circles and ellipses drawn in one view
(see `cc_core::detail`). They are moved, copied, rotated, mirrored,
trimmed, split, offset and deleted with the generic commands, and
`element_get`/`element_set` show and change their curve and style.

**Items:** [`MAX_CURVES`](#cc_engine-commands-detail-max_curves) · [`style_arg`](#cc_engine-commands-detail-style_arg) · [`DetailLineCreate`](#cc_engine-commands-detail-detaillinecreate) · [`normalize_patch`](#cc_engine-commands-detail-normalize_patch) · [`check`](#cc_engine-commands-detail-check)

<a id="cc_engine-commands-detail-max_curves"></a>

### const `MAX_CURVES`

```rust
pub const MAX_CURVES: usize
```

Most curves one `detail_line_create` makes.

<a id="cc_engine-commands-detail-style_arg"></a>

### fn `style_arg`

```rust
pub fn style_arg(p: &cc_core::Project, code: &str) -> Result<LineStyleRef>
```

A line style named by its code: a built-in style's code, or the name
of one of the project's line styles.

<a id="cc_engine-commands-detail-detaillinecreate"></a>

### struct `DetailLineCreate`

```rust
pub struct DetailLineCreate
```

Draw detail lines in a 2D view (plan, ceiling plan, elevation, section,
legend or drafting view): lines, arcs, circles and ellipses in the
view's drawing coordinates (meters, as for dimensions and text notes),
shown only in that view, on top of the model. Each curve becomes one
element (a polyline is its segments; rectangles and polygons their
sides), all in one undo step. Detail lines are left out of 3D views and
IFC export; DXF export puts them on a layer named after their style.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `curves: Vec<CurveArg>`

  The curves to draw, as for sketches (`line`, `arc`, `circle`,
  `ellipse`, `rectangle`, `polygon`).

- `style: Option<String>`

  Line style: thin (default), medium, wide, hidden, dashed, center or
  dotted, or the name of one of the project's line styles
  (`line_styles_list`).


<a id="cc_engine-commands-detail-normalize_patch"></a>

### fn `normalize_patch`

```rust
pub(crate) fn normalize_patch(p: &cc_core::Project, patch: &Value) -> Result<Value>
```

A `params` patch for a detail line with its style given by code
(`"style": "wide"`) turned into the model's form.

<a id="cc_engine-commands-detail-check"></a>

### fn `check`

```rust
pub(crate) fn check(old: &DetailLine, new: &DetailLine) -> Result<()>
```

Check a patched detail line: the view stays, the curve is valid.

<a id="mod-cc_engine-commands-detail_component"></a>

## `cc_engine::commands::detail_component`

<sub>`crates/cc_engine/src/commands/detail_component.rs`</sub>

Detail components: reusable 2D components placed in one view (see
`cc_core::detail_component`). `detail_component_create` makes a type
from 2D pieces or from detail lines, regions, text notes and detail
components of a view; `detail_component_place` places one. Pieces are
added, changed and deleted with the `component_sheet_piece_*` commands,
parameters with `component_param_*`, and placed elements move, rotate,
mirror, copy and delete with the generic commands; `element_set`
changes `position`, `rotation_deg`, `scale`, `mirrored` and `overrides`.
`cad_import` with `blocks` brings a DWG or DXF file's blocks in as
detail components (see `cad_blocks`).

**Items:** [`DetailComponentCreate`](#cc_engine-commands-detail_component-detailcomponentcreate) · [`DetailComponentPlace`](#cc_engine-commands-detail_component-detailcomponentplace) · [`check`](#cc_engine-commands-detail_component-check) · [`derived`](#cc_engine-commands-detail_component-derived) · [`CadBlocks`](#cc_engine-commands-detail_component-cadblocks) · [`cad_blocks`](#cc_engine-commands-detail_component-cad_blocks) · [`insert_place`](#cc_engine-commands-detail_component-insert_place)

<a id="cc_engine-commands-detail_component-detailcomponentcreate"></a>

### struct `DetailComponentCreate`

```rust
pub struct DetailComponentCreate
```

Make a detail component (a reusable 2D component for construction
details) as a new type named `name`, in one undo step: from `pieces`
(meters in its frame), or from `elements` of `view` (detail lines,
filled and masking regions, text notes and other detail components),
whose point `origin` (default: the lower left corner of their extents)
becomes the component's origin. With `replace` (the default when
elements are given) those elements are deleted and the new component is
placed where they were. Detail lines keep their pen, regions their fill
(solid or pattern; openings as outlines), text notes their text at its
printed height, and detail components become nested pieces.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  Name of the component and of its type.

- `pieces: Vec<SheetPieceArg>`

  Pieces, as `component_sheet_piece_add` takes them (meters).

- `view: Option<Ref>`

  View id or name of `elements`.

- `elements: Vec<u64>`

  Ids of the view's elements to make the component from.

- `origin: Option<Point>`

  The point of the view that becomes the component's origin.

- `replace: Option<bool>`

  Replace `elements` by the new component (default true).

- `description: Option<String>`

  Description of the type.


<a id="cc_engine-commands-detail_component-detailcomponentplace"></a>

### struct `DetailComponentPlace`

```rust
pub struct DetailComponentPlace
```

Place a detail component in a 2D view (plan, ceiling plan, elevation,
section, legend or drafting view): its origin at `position` (view
coordinates, meters), turned `rotation` degrees, sized by `scale` and
flipped across its own X axis first when `mirrored`. It is drawn only in
that view, over the model and its regions, and goes with the view.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A detail component type: id or name.

- `view: Ref`

  View id or name.

- `position: Point`

  Where its origin goes, [x, y] in meters.

- `rotation: Option<f64>`

  Degrees counter-clockwise (default 0).

- `scale: Option<f64>`

  Size factor (default 1).

- `mirrored: Option<bool>`

  Flipped across its own X axis before turning (default false).


<a id="cc_engine-commands-detail_component-check"></a>

### fn `check`

```rust
pub(crate) fn check(old: &DetailComponent, new: &DetailComponent) -> Result<()>
```

Check a patched detail component: the view stays, the placement is valid.

<a id="cc_engine-commands-detail_component-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(p: &Project, e: &cc_core::Element, out: &mut serde_json::Map<String, Value>)
```

Derived values of a placed detail component for `element_get`.

<a id="cc_engine-commands-detail_component-cadblocks"></a>

### struct `CadBlocks`

```rust
pub(crate) struct CadBlocks
```

What `cad_blocks` made.

**Traits:** `Debug`, `Default`

**Fields**

- `types: Vec<(String, ComponentDef)>`

  One type per block: its component name (and type name) and definition.

- `dropped: usize`

  Pieces left out past the limits.


<a id="cc_engine-commands-detail_component-cad_blocks"></a>

### fn `cad_blocks`

```rust
pub(crate) fn cad_blocks(read: &cc_cad::CadRead, layers: &[cc_core::CadLayer], f: f64, names: &std::collections::BTreeMap<String, String>) -> CadBlocks
```

The kept blocks of a DWG or DXF read as detail component definitions:
each block's primitives (on visible layers) scaled to meters by `f`, in
its color (its own or its layer's), and its inserts of other kept
blocks as nested pieces; `names` gives each block's component name.

<a id="cc_engine-commands-detail_component-insert_place"></a>

### fn `insert_place`

```rust
pub(crate) fn insert_place(i: &cc_cad::CadInsert, f: f64, import: &PlanTransform) -> Place
```

Where an insert of a kept block lands in the view, for an import whose
drawing units are `f` meters and whose placement is `import`.

<a id="mod-cc_engine-commands-devices"></a>

## `cc_engine::commands::devices`

<sub>`crates/cc_engine/src/commands/devices.rs`</sub>

Device types the engine provides on demand: a project that has no type
for a kind of device (a data outlet, a rack, an air terminal, an earth
electrode…) gets the built-in one with the first device placed, in the
same undo step (see `cc_core::component::device_library`).

**Items:** [`parse_device`](#cc_engine-commands-devices-parse_device) · [`type_mep`](#cc_engine-commands-devices-type_mep) · [`device_type`](#cc_engine-commands-devices-device_type) · [`add_device_type`](#cc_engine-commands-devices-add_device_type) · [`NewDevice`](#cc_engine-commands-devices-newdevice) · [`add_device`](#cc_engine-commands-devices-add_device) · [`DeviceTypeCreate`](#cc_engine-commands-devices-devicetypecreate)

<a id="cc_engine-commands-devices-parse_device"></a>

### fn `parse_device`

```rust
pub(crate) fn parse_device(name: &str) -> Result<DeviceKind>
```

<a id="cc_engine-commands-devices-type_mep"></a>

### fn `type_mep`

```rust
pub(crate) fn type_mep(p: &Project, id: ElementTypeId) -> Option<MepData>
```

The MEP data of a component type.

<a id="cc_engine-commands-devices-device_type"></a>

### fn `device_type`

```rust
pub(crate) fn device_type(p: &Project, device: DeviceKind, ports: Option<u32>) -> Option<ElementTypeId>
```

The first component type that is a `device` (and, for data outlets, has
`ports` jacks when given).

<a id="cc_engine-commands-devices-add_device_type"></a>

### fn `add_device_type`

```rust
pub(crate) fn add_device_type(e: &mut EditCtx, device: DeviceKind, ports: Option<u32>) -> Result<ElementTypeId>
```

The built-in type of `device` (data outlets with `ports` jacks, default
two) added to the project: its materials too, when missing.

<a id="cc_engine-commands-devices-newdevice"></a>

### struct `NewDevice`

```rust
pub(crate) struct NewDevice
```

A device to add to the project.

**Fields**

- `device: DeviceKind`
- `level: LevelId`
- `position: DVec2`
- `rotation: f64`
- `base_offset: f64`
- `system: Option<SystemId>`
- `type_id: Option<ElementTypeId>`

  Its type (default: the first of its kind, else the built-in one).

- `ports: Option<u32>`

  Data outlets: jacks of the type to use when no type is given.

- `size: Option<f64>`

  Value of the type's first instance parameter (an air terminal's
  height, an earth electrode's length).

- `mark: String`

<a id="cc_engine-commands-devices-add_device"></a>

### fn `add_device`

```rust
pub(crate) fn add_device(e: &mut EditCtx, d: NewDevice) -> Result<ElementId>
```

Add a device (and its type, when the project has none).

<a id="cc_engine-commands-devices-devicetypecreate"></a>

### struct `DeviceTypeCreate`

```rust
pub struct DeviceTypeCreate
```

Give the project the built-in component type of a kind of device, if it
has none: `device` is `outlet`, `dedicated_outlet`, `switch`,
`ceiling_light`, `wall_light`, `panel`, `data_outlet` (with `ports`
jacks, default 2), `rack`, `air_terminal` or `earth_electrode`. Returns
the type (the existing one when there is one).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `device: String`

  Kind of device.

- `ports: Option<u32>`

  Data outlets: jacks per outlet (1 to 48).


<a id="mod-cc_engine-commands-doors"></a>

## `cc_engine::commands::doors`

<sub>`crates/cc_engine/src/commands/doors.rs`</sub>

Door types: swing doors of one or two leaves and French doors (glazed,
from the floor), built from the definitions of `cc_core::openings`.

**Items:** [`DoorTypeCreate`](#cc_engine-commands-doors-doortypecreate)

<a id="cc_engine-commands-doors-doortypecreate"></a>

### struct `DoorTypeCreate`

```rust
pub struct DoorTypeCreate
```

Create a door type. `style` is `swing` (a solid leaf, the default) or
`french` (a French door: glazed leaves of stiles and rails around a pane,
over a solid bottom rail `bottom_rail` tall, a parameter of the type);
`leaves` is 1 (hinged on the left; flip an element's hand for the right)
or 2 (a double door, leaves hinged on either jamb meeting in the middle,
drawn with two swings in plans and written to IFC as
`DOUBLE_DOOR_SINGLE_SWING`). Doors stand on the floor (sill 0). Place them
with `door_create`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name, unique among door types (default: the component's name and size, in centimeters; in feet
  and inches in imperial projects).

- `style: Option<String>`

  `swing` (default) or `french`.

- `leaves: Option<u8>`

  Number of leaves: 1 (default) or 2.

- `width: Option<f64>`

  Width of the opening, meters (default 0.8 for one leaf, 1.4 for two; 2'-8" and 5'-0" in imperial
  projects).

- `height: Option<f64>`

  Height of the opening, meters (default 2.1, French doors 2.2; 6'-8" in imperial projects).

- `bottom_rail: Option<f64>`

  French doors: height of the solid bottom rail of each leaf, meters (default 0.4; 1'-4" in imperial
  projects).

- `material: Option<String>`

  Material of the leaves (default: the template's wood, if the project has it).

- `glass: Option<String>`

  French doors: material of the panes (default: the template's glass, if the project has it).


<a id="mod-cc_engine-commands-edit"></a>

## `cc_engine::commands::edit`

<sub>`crates/cc_engine/src/commands/edit.rs`</sub>

Changing element parameters (the Properties panel).

**Items:** [`ElementSet`](#cc_engine-commands-edit-elementset) · [`patch_kind`](#cc_engine-commands-edit-patch_kind) · [`ElementsSet`](#cc_engine-commands-edit-elementsset) · [`ElementsSetType`](#cc_engine-commands-edit-elementssettype)

<a id="cc_engine-commands-edit-elementset"></a>

### struct `ElementSet`

```rust
pub struct ElementSet
```

Change an element: its level (doors and windows of a wall follow it),
type (of the same category), mark, comments, and any of its `params`
as `element_get` shows them (merged: only the fields given change, e.g.
`{"height": 2.8}` for a wall or `{"shape": {"Gable": {"slope": 0.5,
"ridge_deg": 0}}}` for a roof). The edit is refused if it would leave the
model invalid.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.

- `level: Option<Ref>`

  New level id or name.

- `type_ref: Option<Ref>`

  New type id or name (same category).

- `mark: Option<String>`

  New mark.

- `comments: Option<String>`

  New comments.

- `params: Option<Value>`

  Fields of the element's parameters to change.


<a id="cc_engine-commands-edit-patch_kind"></a>

### fn `patch_kind`

```rust
pub(crate) fn patch_kind(kind: &cc_core::ElementKind, patch: &Value) -> Result<cc_core::ElementKind>
```

`kind` with `patch` merged into its parameters as `element_set` merges
them (an imported drawing keeps its drawing), before any checks.

<a id="cc_engine-commands-edit-elementsset"></a>

### struct `ElementsSet`

```rust
pub struct ElementsSet
```

Change several elements at once, in one undo step: a new level (doors
and windows of a wall follow it), the same `params` merged into each
(as `element_set` takes them, e.g. `{"base_offset": -1.5}` for columns),
and the same mark or comments. Every element must take the change, or
none changes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `level: Option<Ref>`

  New level id or name.

- `params: Option<Value>`

  Fields of the elements' parameters to change.

- `mark: Option<String>`

  New mark of every element.

- `comments: Option<String>`

  New comments of every element.


<a id="cc_engine-commands-edit-elementssettype"></a>

### struct `ElementsSetType`

```rust
pub struct ElementsSetType
```

Give elements a type (all must be of the type's category), like Match Type.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `type_ref: Ref`

  Type id or name.


<a id="mod-cc_engine-commands-fill_patterns"></a>

## `cc_engine::commands::fill_patterns`

<sub>`crates/cc_engine/src/commands/fill_patterns.rs`</sub>

Fill patterns (hatches): the built-in patterns and the project's own (see
`cc_core::fill_pattern`), which materials (cut and surface patterns),
filled region types and view overrides use. The project's patterns are
project settings: each command changes them in one undo step. They come
from `.pat` files (`fill_pattern_import`, see `cc_core::pat`) or are
drawn up line family by line family (`fill_pattern_create`).
Where a command takes a pattern by code it accepts a built-in code
(`diagonal`, `brick`…) or a project pattern's name; in model values
(`material_set`, view overrides, `type_set`) a project pattern is written
`{"Custom": <id>}`, as `fill_patterns_list` shows in `value`.

**Items:** [`builtin_code`](#cc_engine-commands-fill_patterns-builtin_code) · [`pattern_code`](#cc_engine-commands-fill_patterns-pattern_code) · [`pattern_arg`](#cc_engine-commands-fill_patterns-pattern_arg) · [`LineArg`](#cc_engine-commands-fill_patterns-linearg) · [`FillPatternsList`](#cc_engine-commands-fill_patterns-fillpatternslist) · [`FillPatternGet`](#cc_engine-commands-fill_patterns-fillpatternget) · [`FillPatternCreate`](#cc_engine-commands-fill_patterns-fillpatterncreate) · [`FillPatternSet`](#cc_engine-commands-fill_patterns-fillpatternset) · [`FillPatternDuplicate`](#cc_engine-commands-fill_patterns-fillpatternduplicate) · [`FillPatternDelete`](#cc_engine-commands-fill_patterns-fillpatterndelete) · [`OnClash`](#cc_engine-commands-fill_patterns-onclash) · [`FillPatternImport`](#cc_engine-commands-fill_patterns-fillpatternimport) · [`FillPatternExport`](#cc_engine-commands-fill_patterns-fillpatternexport)

<a id="cc_engine-commands-fill_patterns-builtin_code"></a>

### fn `builtin_code`

```rust
pub fn builtin_code(p: Pattern) -> String
```

A built-in pattern's code: `none`, `solid`, `diagonal`, `diagonal_back`,
`crosshatch`, `horizontal`, `vertical`, `grid`, `brick`, `concrete`,
`sand`, `insulation` or `earth` (`custom` for a project pattern).

<a id="cc_engine-commands-fill_patterns-pattern_code"></a>

### fn `pattern_code`

```rust
pub fn pattern_code(p: &Project, pattern: Pattern) -> String
```

A pattern's code: a built-in code, or a project pattern's name.

<a id="cc_engine-commands-fill_patterns-pattern_arg"></a>

### fn `pattern_arg`

```rust
pub fn pattern_arg(p: &Project, code: &str) -> Result<Pattern>
```

A pattern by code: a built-in code or a project pattern's name.

<a id="cc_engine-commands-fill_patterns-linearg"></a>

### struct `LineArg`

```rust
pub struct LineArg
```

One line family of a pattern, lengths in millimeters (on paper for a
drafting pattern, in the world for a model pattern).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `angle: f64`

  Direction of the lines, degrees counterclockwise from +x.

- `origin: Option<[f64; 2]>`

  A point the first line passes through, `[x, y]` mm (default [0, 0]).

- `offset: [f64; 2]`

  From one line to the next, `[along, across]` mm: the shift of the
  dashes along the line and the spacing between lines.

- `dashes: Option<Vec<f64>>`

  Dash lengths in turn, mm: positive drawn, negative a gap, 0 a dot
  (none: continuous lines).


<a id="cc_engine-commands-fill_patterns-fillpatternslist"></a>

### struct `FillPatternsList`

```rust
pub struct FillPatternsList
```

The fill patterns materials, filled regions and view overrides can use:
the built-in ones (by code) and the project's (by name, with their kind,
scale and line families), each with how many materials, types, views,
templates and phase graphics use it, and its `value` in model values.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-fill_patterns-fillpatternget"></a>

### struct `FillPatternGet`

```rust
pub struct FillPatternGet
```

One project fill pattern: its definition and everything that uses it
(materials, types, views, templates, phase graphics).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `pattern: Ref`

  Fill pattern id or name.


<a id="cc_engine-commands-fill_patterns-fillpatterncreate"></a>

### struct `FillPatternCreate`

```rust
pub struct FillPatternCreate
```

Create a project fill pattern from line families: a unique name (not a
built-in code), drafting (sized on paper, the default) or model (sized
in the world, scaling with the view), a scale and the families.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique, not a built-in pattern's code).

- `description: Option<String>`

  Description.

- `kind: Option<String>`

  `drafting` (default: millimeters on paper) or `model` (millimeters in the world).

- `scale: Option<f64>`

  Multiplies every length (0.01 to 100; default 1).

- `lines: Vec<LineArg>`

  The line families (1 to 128).


<a id="cc_engine-commands-fill_patterns-fillpatternset"></a>

### struct `FillPatternSet`

```rust
pub struct FillPatternSet
```

Change a project fill pattern: name, description, kind, scale or line
families. What is drawn with it follows.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `pattern: Ref`

  Fill pattern id or name.

- `name: Option<String>`

  New name.

- `description: Option<String>`

  New description.

- `kind: Option<String>`

  `drafting` or `model`.

- `scale: Option<f64>`

  Multiplies every length (0.01 to 100).

- `lines: Option<Vec<LineArg>>`

  New line families, in place of all of them.


<a id="cc_engine-commands-fill_patterns-fillpatternduplicate"></a>

### struct `FillPatternDuplicate`

```rust
pub struct FillPatternDuplicate
```

Copy a project fill pattern under a new name (default: the name with a
number).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `pattern: Ref`

  Fill pattern id or name.

- `name: Option<String>`

  Name of the copy.


<a id="cc_engine-commands-fill_patterns-fillpatterndelete"></a>

### struct `FillPatternDelete`

```rust
pub struct FillPatternDelete
```

Delete a project fill pattern. Refused while something uses it, unless
`replace_with` names the pattern (a built-in code or another project
pattern) that takes its place there, in the same undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `pattern: Ref`

  Fill pattern id or name.

- `replace_with: Option<String>`

  The pattern that takes its place where it is used.


<a id="cc_engine-commands-fill_patterns-onclash"></a>

### enum `OnClash`

```rust
pub enum OnClash
```

What to do with an imported pattern whose name the project already has.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Rename`

  Import it under the name with a number.

- `Replace`

  Put its lines in place of the project pattern's (what uses it follows).

- `Skip`

  Leave it out.


<a id="cc_engine-commands-fill_patterns-fillpatternimport"></a>

### struct `FillPatternImport`

```rust
pub struct FillPatternImport
```

Import fill patterns from a `.pat` file (the plain-text hatch pattern
format CAD programs share: `*NAME, description` headers, then lines of
`angle, x-origin, y-origin, delta-x, delta-y [, dashes…]`). All the
file's patterns or those named; each pattern is drafting or model as its
`;%TYPE=MODEL` marker says unless `kind` is given, lengths in the
file's `;%UNITS=` (else millimeters) unless `units` is given. Patterns
with a problem are left out and listed in `errors` with their line.
With `preview`, nothing changes: the result lists what the file holds.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `path: Option<PathBuf>`

  Path of the .pat file (or give `text`).

- `text: Option<String>`

  The file's text (in place of `path`).

- `names: Option<Vec<String>>`

  Names of the patterns to import (default: all).

- `on_clash: OnClash`

  A name the project already has: `rename` (default), `replace` or `skip`.

- `kind: Option<String>`

  `drafting` or `model` for every pattern, in place of the file's markers.

- `units: Option<String>`

  Units of the file's lengths: `auto` (default: its `;%UNITS=` marker,
  else millimeters), `mm` or `inch`.

- `scale: Option<f64>`

  The scale given to every imported pattern (0.01 to 100; default 1).

- `preview: bool`

  Only read the file and list its patterns.


<a id="cc_engine-commands-fill_patterns-fillpatternexport"></a>

### struct `FillPatternExport`

```rust
pub struct FillPatternExport
```

Write project fill patterns as `.pat` text (millimeters, each pattern's
scale applied, a `;%TYPE=` marker per pattern): to `path` when given;
the text is returned either way.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `patterns: Option<Vec<Ref>>`

  Fill pattern ids or names (default: all the project's).

- `path: Option<PathBuf>`

  Path of the .pat file to write.


<a id="mod-cc_engine-commands-foundation"></a>

## `cc_engine::commands::foundation`

<sub>`crates/cc_engine/src/commands/foundation.rs`</sub>

Foundations: pads under columns, strips (ground beams) under walls and
slabs, placed one by one or under existing columns and walls.

**Items:** [`FootingCreate`](#cc_engine-commands-foundation-footingcreate) · [`check_strip_arc`](#cc_engine-commands-foundation-check_strip_arc) · [`StripFootingCreate`](#cc_engine-commands-foundation-stripfootingcreate) · [`FoundationSlabCreate`](#cc_engine-commands-foundation-foundationslabcreate) · [`FootingsUnderColumns`](#cc_engine-commands-foundation-footingsundercolumns) · [`StripFootingsUnderWalls`](#cc_engine-commands-foundation-stripfootingsunderwalls)

<a id="cc_engine-commands-foundation-footingcreate"></a>

### struct `FootingCreate`

```rust
pub struct FootingCreate
```

Place a pad footing centered at `position`; its top is `top_offset` above the level (default 0).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Center of the pad in plan.

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Pad footing type id or name (default: the first pad type).

- `top_offset: Option<f64>`

  Top above the level, meters (default 0).


<a id="cc_engine-commands-foundation-check_strip_arc"></a>

### fn `check_strip_arc`

```rust
pub(crate) fn check_strip_arc(p: &Project, type_id: ElementTypeId, shape: &FoundationShape) -> Result<()>
```

An arc strip must leave room inside its curve for its width.

<a id="cc_engine-commands-foundation-stripfootingcreate"></a>

### struct `StripFootingCreate`

```rust
pub struct StripFootingCreate
```

Place a strip footing (a ground beam) centered on the line `start`-`end`,
or on an arc through `mid` (or with a `bulge` or a `radius`); its top is
`top_offset` above the level (default 0). Strips meet at corners and tees
like beams, arcs at the true crossings of their faces.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the strip's center line.

- `end: Point`

  End of the strip's center line.

- `mid: Option<Point>`

  A point on the arc between the ends: makes an arc strip through the three points.

- `bulge: Option<f64>`

  Arc strips: tan(sweep / 4), positive when the arc turns counter-clockwise from `start`
  to `end` (1 is a half circle; at most 10 either way). Ignored when `mid` is given.

- `radius: Option<f64>`

  Arc strips: the radius in meters, positive when the arc turns counter-clockwise from
  `start` to `end`, negative clockwise; at least half the distance between the ends.
  Ignored when `mid` or `bulge` is given.

- `major_arc: Option<bool>`

  With `radius`: take the longer of the two arcs (default: the shorter).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Strip footing type id or name (default: the first strip type).

- `top_offset: Option<f64>`

  Top above the level, meters (default 0).


<a id="cc_engine-commands-foundation-foundationslabcreate"></a>

### struct `FoundationSlabCreate`

```rust
pub struct FoundationSlabCreate
```

Create a foundation slab (a raft) from an outline, from `curves` (lines,
arcs, circles: a circular raft keeps its true curve, area and volume) or
under the loop of walls of the level; `offset` is its top above the level
(default 0).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `sketch: SketchArgs`

<a id="cc_engine-commands-foundation-footingsundercolumns"></a>

### struct `FootingsUnderColumns`

```rust
pub struct FootingsUnderColumns
```

Place a pad footing under each column (default: every column of the
level), centered and turned with it, its top at the column's base.
Columns that already stand on a pad are skipped.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `columns: Vec<u64>`

  Column ids (default: every column of `level`).

- `level: Option<Ref>`

  Level id or name, when no columns are given (default: the lowest level).

- `type_ref: Option<Ref>`

  Pad footing type id or name (default: the first pad type).


<a id="cc_engine-commands-foundation-stripfootingsunderwalls"></a>

### struct `StripFootingsUnderWalls`

```rust
pub struct StripFootingsUnderWalls
```

Place a strip footing under each wall (default: every wall of the level),
along its body's centerline (an arc under an arc wall), its top at the
wall's base. Walls that already have a strip, and arc walls too tight for
the strip's width, are skipped; strips meet at corners like beams.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `walls: Vec<u64>`

  Wall ids (default: every wall of `level`).

- `level: Option<Ref>`

  Level id or name, when no walls are given (default: the lowest level).

- `type_ref: Option<Ref>`

  Strip footing type id or name (default: the first strip type).


<a id="mod-cc_engine-commands-framing"></a>

## `cc_engine::commands::framing`

<sub>`crates/cc_engine/src/commands/framing.rs`</sub>

Bracing and trusses: sloped members between points in space, braces
between columns and trusses laid out from a span, a height and a pattern.

**Items:** [`Point3`](#cc_engine-commands-framing-point3) · [`BraceCreate`](#cc_engine-commands-framing-bracecreate) · [`BracesBetweenColumns`](#cc_engine-commands-framing-bracesbetweencolumns) · [`TrussRows`](#cc_engine-commands-framing-trussrows) · [`truss_rows`](#cc_engine-commands-framing-truss_rows) · [`new_truss_type`](#cc_engine-commands-framing-new_truss_type) · [`TrussCreate`](#cc_engine-commands-framing-trusscreate) · [`derived`](#cc_engine-commands-framing-derived)

<a id="cc_engine-commands-framing-point3"></a>

### type alias `Point3`

```rust
pub type Point3 = [f64; 3]
```

A point in space: `[x, y, z]`, `z` above the level.

<a id="cc_engine-commands-framing-bracecreate"></a>

### struct `BraceCreate`

```rust
pub struct BraceCreate
```

Create a brace: a member between two points in space (`[x, y, z]`, `z`
above the level), its axis through both. Its ends must not be one above the other.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point3`

  Start point `[x, y, z]`.

- `end: Point3`

  End point `[x, y, z]`.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Beam type id or name (default: the category's default type).


<a id="cc_engine-commands-framing-bracesbetweencolumns"></a>

### struct `BracesBetweenColumns`

```rust
pub struct BracesBetweenColumns
```

Brace the bay between two columns: `x` (default) crosses two members
from the base of each column to the top of the other, `diagonal` places
one from the base of the first to the top of the second.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `columns: Vec<u64>`

  The two column ids.

- `pattern: Option<String>`

  `x` or `diagonal`.

- `type_ref: Option<Ref>`

  Beam type id or name (default: the category's default type).


<a id="cc_engine-commands-framing-trussrows"></a>

### struct `TrussRows`

```rust
pub struct TrussRows
```

The catalog rows a truss of a material gets by default: (series,
designation) of the chords and of the web (the posts are a column type of
the web's row).

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `chord: (&'static str, &'static str)`
- `web: (&'static str, &'static str)`
- `flat: bool`

  Whether the rows are channels laid on their sides: the bottom chord
  open upward, the top chord and the diagonals open downward, so the
  web stands inside the chords.


<a id="cc_engine-commands-framing-truss_rows"></a>

### fn `truss_rows`

```rust
pub fn truss_rows(m: StructuralMaterial) -> Option<TrussRows>
```

Steel and timber rows; `None` for concrete (trusses are not offered).

<a id="cc_engine-commands-framing-new_truss_type"></a>

### fn `new_truss_type`

```rust
pub fn new_truss_type(id: ElementTypeId) -> ElementType
```

The type a project gets with its first truss.

<a id="cc_engine-commands-framing-trusscreate"></a>

### struct `TrussCreate`

```rust
pub struct TrussCreate
```

Lay out a parametric plane truss from `start` to `end`: a `flat`
(default), `mono` or `gable` outline `height` deep (the rise at the apex
or high end for mono and gable), `panels` bottom-chord panels and the web
of `pattern` (`pratt`, the default, `howe`, `warren`, or for gables
`kingpost`, `queenpost` and `fink`; a gable-only pattern on another shape
is laid out as Pratt). Chords and diagonals are beams, verticals are
columns, all ordinary elements the truss keeps in step with its
parameters (`element_set` regenerates them, keeping their ids); a member
edited or deleted by hand is detached and left alone. Each chord is one
continuous beam per straight run (two top chords on a gable). Verticals
stand between the chords: on the faces of solid chords, or inside a
channel that opens toward the web, and they are turned so the side
nearest the chords' width lies across the truss. Where the truss ends on
a vertical (both ends of a flat truss, the high end of a mono one) the
chords run on over it and the vertical at the far end is turned around,
so the ends are closed. `copies` trusses are made `spacing` apart to the
left of start → end. Default types come from the profile catalog for the
`material` (steel: `Ue 150x60x20x2,00` chords laid on their sides, the
bottom one open upward and the top one downward, `Ue 100x50x17x2,00`
diagonals laid the same way and posts inside the chords; timber:
`Peça 6x16` chords, `Peça 6x12` web and posts), made when missing;
concrete is refused.
Out-of-range values are refused. One undo step; returns the truss's `id`
and its `members`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the span line in plan (a support).

- `end: Point`

  End of the span line in plan (the other support), 0.5 to 200 m from the start.

- `height: f64`

  Chord axis to chord axis (flat), or the rise at the apex or high end (mono, gable), meters, 0.1 to 20.

- `panels: Option<u32>`

  Bottom-chord panels, 2 to 64, even on a gable (default: about one per 1.5 m).

- `pattern: Option<String>`

  `pratt` (default), `howe`, `warren`, `kingpost`, `queenpost` or `fink`.

- `bottom_offset: Option<f64>`

  Elevation of the bottom chord's axis above the level (default: the level height minus `height`).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `chord_type: Option<Ref>`

  Beam type of both chords (default: the material's catalog chord).

- `web_type: Option<Ref>`

  Beam type of the diagonals (default: the material's catalog web row).

- `post_type: Option<Ref>`

  Column type of the verticals (default: a column type of the material's catalog web row).

- `shape: Option<String>`

  `flat` (default), `mono` (rising toward `end`) or `gable` (rising toward mid-span).

- `top_chord_type: Option<Ref>`

  Beam type of the top chord (default: `chord_type`, else the catalog chord).

- `bottom_chord_type: Option<Ref>`

  Beam type of the bottom chord (default: `chord_type`, else the catalog chord).

- `copies: Option<u32>`

  How many trusses, 1 to 200 (default 1).

- `spacing: Option<f64>`

  Distance between copies, to the left of start → end, meters, 0.3 to 20 (default 1.5).

- `material: Option<String>`

  `steel` (default) or `timber`: the catalog rows of the types not given (a given `chord_type` does not change it, so give `material` with a timber chord).


<a id="cc_engine-commands-framing-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(t: &Truss, out: &mut serde_json::Map<String, Value>)
```

`element_get`'s derived values of a truss: its member ids by role kind,
how many roles are detached, its span length and the pattern it is laid
out with (a gable-only pattern falls back to Pratt).

<a id="mod-cc_engine-commands-graph"></a>

## `cc_engine::commands::graph`

<sub>`crates/cc_engine/src/commands/graph.rs`</sub>

Visual programming graphs (see `crate::graph`): run a `.ccgraph`, check
it, list the nodes a graph can use, and see or release the elements a
graph made.

**Items:** [`GraphRun`](#cc_engine-commands-graph-graphrun) · [`GraphCheck`](#cc_engine-commands-graph-graphcheck) · [`problems`](#cc_engine-commands-graph-problems) · [`NodeInfo`](#cc_engine-commands-graph-nodeinfo) · [`node_catalog`](#cc_engine-commands-graph-node_catalog) · [`GraphNodes`](#cc_engine-commands-graph-graphnodes) · [`GraphElements`](#cc_engine-commands-graph-graphelements) · [`GraphRelease`](#cc_engine-commands-graph-graphrelease)

<a id="cc_engine-commands-graph-graphrun"></a>

### struct `GraphRun`

```rust
pub struct GraphRun
```

Run a visual programming graph (a `.ccgraph` file, or the graph as
JSON) as one undo step. Elements its nodes create are bound to the node
and the item that made them: running the graph again changes them in
place (keeping their ids), makes the new ones and deletes the ones it no
longer makes. Returns each node's outputs or error, the values of its
`output` nodes and how many elements were created, changed and deleted.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `path: Option<String>`

  Path of the `.ccgraph` file.

- `graph: Option<Value>`

  The graph itself (the contents of a `.ccgraph` file), instead of `path`.

- `inputs: BTreeMap<String, Value>`

  Values of the graph's input nodes, by their names.

- `brief: bool`

  Leave each node's outputs out of the result (only errors, outputs and counts).

- `allow_files: bool`

  Let the graph's commands read and write files (open, save, import,
  export). Off by default; a graph file cannot turn it on, and it stays
  off when another command (a plugin's) runs the graph.


<a id="cc_engine-commands-graph-graphcheck"></a>

### struct `GraphCheck`

```rust
pub struct GraphCheck
```

Check a graph without running it: nodes that cannot run here (unknown
commands or nodes, file commands the graph does not allow), wires to
ports that do not exist or whose types do not fit, required inputs with
no value, and what reading the file fixed.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `path: Option<String>`

  Path of the `.ccgraph` file.

- `graph: Option<Value>`

  The graph itself, instead of `path`.

- `allow_files: bool`

  Check it as `graph_run` with `allow_files` would run it (file
  commands are then no problem).


<a id="cc_engine-commands-graph-problems"></a>

### fn `problems`

```rust
pub fn problems(s: &Session, graph: &Graph) -> Vec<Value>
```

The problems `graph_check` reports, as `{node, message}`.

<a id="cc_engine-commands-graph-nodeinfo"></a>

### struct `NodeInfo`

```rust
pub struct NodeInfo
```

A node of the catalog as `graph_nodes` lists it.

**Traits:** `Debug`, `Clone`, `Serialize`

**Fields**

- `kind: Value`

  The node's `kind` in a graph file (`{"command": name}`, `{"builtin": name}` or `{"lua": code}`).

- `name: String`
- `group: String`
- `description: String`
- `inputs: Vec<Port>`
- `outputs: Vec<Port>`
- `edits: bool`

  Whether it may change the model.


<a id="cc_engine-commands-graph-node_catalog"></a>

### fn `node_catalog`

```rust
pub fn node_catalog(s: &Session) -> Vec<NodeInfo>
```

Every node a graph can use on this session: built-in nodes, a Lua node,
then a node per command.

<a id="cc_engine-commands-graph-graphnodes"></a>

### struct `GraphNodes`

```rust
pub struct GraphNodes
```

The nodes a graph can use: built-in nodes (inputs, math, logic, lists,
geometry, element parameters, outputs), the Lua node and a node per
engine command, each with its inputs and outputs (name, type, whether
required, default). Filter by words of the name or description, or by
group.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `search: Option<String>`

  Words that must all appear in the node's name, group or description.

- `group: Option<String>`

  Only this group (`Math`, `Lists`, `Creating elements`…).


<a id="cc_engine-commands-graph-graphelements"></a>

### struct `GraphElements`

```rust
pub struct GraphElements
```

The elements a graph made, with the node and item (lacing path) that
made each.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `graph: String`

  The graph's uid (its file's `uid`, also in `graph_run`'s result).


<a id="cc_engine-commands-graph-graphrelease"></a>

### struct `GraphRelease`

```rust
pub struct GraphRelease
```

Release elements from the graph that made them: they become ordinary
elements, and the next run of the graph makes new ones in their place.
Without `elements`, every element of the graph is released.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `graph: String`

  The graph's uid.

- `elements: Option<Vec<u64>>`

  Ids of the elements to release (default: all of the graph's).


<a id="mod-cc_engine-commands-graphics"></a>

## `cc_engine::commands::graphics`

<sub>`crates/cc_engine/src/commands/graphics.rs`</sub>

View graphics: category visibility and overrides, element overrides and
hiding in a view, view filters, view templates and temporary hiding and
isolation.

**Items:** [`patched`](#cc_engine-commands-graphics-patched) · [`overrides_json`](#cc_engine-commands-graphics-overrides_json) · [`ViewGraphicsGet`](#cc_engine-commands-graphics-viewgraphicsget) · [`ViewGraphicsSet`](#cc_engine-commands-graphics-viewgraphicsset) · [`ElementOverrideSet`](#cc_engine-commands-graphics-elementoverrideset) · [`ElementHideInView`](#cc_engine-commands-graphics-elementhideinview) · [`RuleArg`](#cc_engine-commands-graphics-rulearg) · [`param`](#cc_engine-commands-graphics-param) · [`param_name`](#cc_engine-commands-graphics-param_name) · [`filter_json`](#cc_engine-commands-graphics-filter_json) · [`FiltersList`](#cc_engine-commands-graphics-filterslist) · [`FilterCreate`](#cc_engine-commands-graphics-filtercreate) · [`FilterSet`](#cc_engine-commands-graphics-filterset) · [`FilterDelete`](#cc_engine-commands-graphics-filterdelete) · [`FilterElements`](#cc_engine-commands-graphics-filterelements) · [`ViewFilterAdd`](#cc_engine-commands-graphics-viewfilteradd) · [`ViewFilterSet`](#cc_engine-commands-graphics-viewfilterset) · [`ViewFilterRemove`](#cc_engine-commands-graphics-viewfilterremove) · [`ViewFilterMove`](#cc_engine-commands-graphics-viewfiltermove) · [`template_json`](#cc_engine-commands-graphics-template_json) · [`TemplatesList`](#cc_engine-commands-graphics-templateslist) · [`TemplateCreate`](#cc_engine-commands-graphics-templatecreate) · [`TemplateSet`](#cc_engine-commands-graphics-templateset) · [`TemplateDelete`](#cc_engine-commands-graphics-templatedelete) · [`ViewTemplateApply`](#cc_engine-commands-graphics-viewtemplateapply) · [`ViewTemplateAssign`](#cc_engine-commands-graphics-viewtemplateassign) · [`ViewTemporary`](#cc_engine-commands-graphics-viewtemporary) · [`ViewTemporaryApply`](#cc_engine-commands-graphics-viewtemporaryapply)

<a id="cc_engine-commands-graphics-patched"></a>

### fn `patched`

```rust
pub(crate) fn patched(current: &GraphicOverride, patch: Option<&Value>, reset: bool) -> Result<GraphicOverride>
```

Apply a merge patch (`null` resets an attribute) to overrides.

<a id="cc_engine-commands-graphics-overrides_json"></a>

### fn `overrides_json`

```rust
pub(crate) fn overrides_json(o: &GraphicOverride) -> Value
```

<a id="cc_engine-commands-graphics-viewgraphicsget"></a>

### struct `ViewGraphicsGet`

```rust
pub struct ViewGraphicsGet
```

The visibility/graphics of a view (or of a view template): every
category's visibility and overrides, the applied filters in order, the
element overrides and hidden elements, the view's template and what it
controls, and what is temporarily hidden or isolated. Overrides are
objects like `{"cut_lines": {"color": [255, 0, 0], "weight": 0.5,
"pattern": "Dash"}, "projection_lines": {..}, "cut_fill": {"color": [..],
"pattern": "Solid"}, "surface_fill": {..}, "halftone": true,
"transparency": 0.5}` (absent attributes are not overridden).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.


<a id="cc_engine-commands-graphics-viewgraphicsset"></a>

### struct `ViewGraphicsSet`

```rust
pub struct ViewGraphicsSet
```

Show or hide a category in a view (or view template) and change its
graphic overrides: `overrides` is merged into the current ones (`null`
resets one attribute; see `view_graphics_get`), `reset` clears them
first. Refused when the view's template controls its categories.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.

- `category: String`

  Category (`wall`, `door`, `room`, `dimension`…).

- `visible: Option<bool>`

  Show (`true`) or hide (`false`) the category.

- `overrides: Option<Value>`

  Overrides to merge.

- `reset: bool`

  Clear the category's overrides before merging.


<a id="cc_engine-commands-graphics-elementoverrideset"></a>

### struct `ElementOverrideSet`

```rust
pub struct ElementOverrideSet
```

Override the graphics of elements in one view (they keep their
category's and filters' graphics for what is not overridden):
`overrides` is merged into each element's (`null` resets an attribute),
`reset` clears them first, so `reset` alone removes the overrides.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `ids: Vec<u64>`

  Element ids.

- `overrides: Option<Value>`

  Overrides to merge (see `view_graphics_get`).

- `reset: bool`

  Clear the elements' overrides before merging.


<a id="cc_engine-commands-graphics-elementhideinview"></a>

### struct `ElementHideInView`

```rust
pub struct ElementHideInView
```

Hide elements in one view (`hidden: false` shows them again). Hidden
elements stay in the model and in other views.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `ids: Vec<u64>`

  Element ids.

- `hidden: bool`

  Hide (default) or unhide.


<a id="cc_engine-commands-graphics-rulearg"></a>

### struct `RuleArg`

```rust
pub struct RuleArg
```

A rule as commands take it.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `param: String`

  Parameter: `elevation` (of the element's level) or a schedule field —
  `type`, `mark`, `comments`, `level`, `material`, `system`,
  `structural` (yes/no), `function`, `width`, `height`, `thickness`,
  `length`, `area`, `volume`, `diameter`, `sill`, `type_mark`,
  `description`, `manufacturer`, `model`, `fire_rating`, `name`,
  `number`, `component_name`…

- `op: String`

  `equals`, `not_equals`, `contains`, `not_contains`, `begins_with`,
  `not_begins_with`, `greater`, `greater_or_equal`, `less`,
  `less_or_equal`, `has_value` or `has_no_value`.

- `value: Option<Value>`

  Text, or a number in SI units (meters) for quantities.


<a id="cc_engine-commands-graphics-param"></a>

### fn `param`

```rust
pub(crate) fn param(name: &str) -> Result<FilterParam>
```

<a id="cc_engine-commands-graphics-param_name"></a>

### fn `param_name`

```rust
pub fn param_name(p: FilterParam) -> String
```

<a id="cc_engine-commands-graphics-filter_json"></a>

### fn `filter_json`

```rust
pub(crate) fn filter_json(p: &Project, f: &ViewFilter) -> Value
```

<a id="cc_engine-commands-graphics-filterslist"></a>

### struct `FiltersList`

```rust
pub struct FiltersList
```

The project's view filters with their categories and rules.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-graphics-filtercreate"></a>

### struct `FilterCreate`

```rust
pub struct FilterCreate
```

Create a view filter: the elements of `categories` (all when empty) that
pass its rules — all of them, or any with `any`; a filter without rules
matches every element of its categories. Apply it to views with
`view_filter_add`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique).

- `categories: Vec<String>`

  Categories (`wall`, `door`…); empty for all.

- `rules: Vec<RuleArg>`

  Rules on parameters.

- `any: bool`

  Match when any rule passes (default: all must pass).


<a id="cc_engine-commands-graphics-filterset"></a>

### struct `FilterSet`

```rust
pub struct FilterSet
```

Change a view filter: name, categories, rules (replaced as a whole) or
how rules combine. Views that apply it follow.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `filter: Ref`

  Filter id or name.

- `name: Option<String>`

  New name.

- `categories: Option<Vec<String>>`

  New categories (empty for all).

- `rules: Option<Vec<RuleArg>>`

  New rules (replace the old ones).

- `any: Option<bool>`

  Match when any rule passes (`false`: all must pass).


<a id="cc_engine-commands-graphics-filterdelete"></a>

### struct `FilterDelete`

```rust
pub struct FilterDelete
```

Delete a view filter (views and templates stop applying it).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `filter: Ref`

  Filter id or name.


<a id="cc_engine-commands-graphics-filterelements"></a>

### struct `FilterElements`

```rust
pub struct FilterElements
```

The elements a view filter matches (only those drawn in `view` when given).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `filter: Ref`

  Filter id or name.

- `view: Option<Ref>`

  Only elements drawn in this view.


<a id="cc_engine-commands-graphics-viewfilteradd"></a>

### struct `ViewFilterAdd`

```rust
pub struct ViewFilterAdd
```

Apply a filter to a view (or view template), at `index` in the list
(default last: the first matching filter wins for each attribute),
hiding what it matches (`visible: false`) or overriding its graphics.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.

- `filter: Ref`

  Filter id or name.

- `visible: Option<bool>`

  Show (default) or hide the matching elements.

- `overrides: Option<Value>`

  Overrides of the matching elements (see `view_graphics_get`).

- `index: Option<usize>`

  Position in the view's filter list (0 = first).


<a id="cc_engine-commands-graphics-viewfilterset"></a>

### struct `ViewFilterSet`

```rust
pub struct ViewFilterSet
```

Change how a view (or view template) applies a filter: visibility of the
matching elements, whether it is enabled, and its overrides (merged;
`reset` clears them first).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.

- `filter: Ref`

  Filter id or name.

- `visible: Option<bool>`

  Show (`true`) or hide (`false`) the matching elements.

- `enabled: Option<bool>`

  Apply the filter (`false` keeps it listed without effect).

- `overrides: Option<Value>`

  Overrides to merge (see `view_graphics_get`).

- `reset: bool`

  Clear the overrides before merging.


<a id="cc_engine-commands-graphics-viewfilterremove"></a>

### struct `ViewFilterRemove`

```rust
pub struct ViewFilterRemove
```

Stop applying a filter in a view (or view template).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.

- `filter: Ref`

  Filter id or name.


<a id="cc_engine-commands-graphics-viewfiltermove"></a>

### struct `ViewFilterMove`

```rust
pub struct ViewFilterMove
```

Move a filter to `index` in a view's (or view template's) filter list
(0 = first, the highest priority).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Option<Ref>`

  View id or name.

- `template: Option<Ref>`

  Or a view template id or name.

- `filter: Ref`

  Filter id or name.

- `index: usize`

  New position (0 = first).


<a id="cc_engine-commands-graphics-template_json"></a>

### fn `template_json`

```rust
pub(crate) fn template_json(p: &Project, t: &ViewTemplate) -> Value
```

<a id="cc_engine-commands-graphics-templateslist"></a>

### struct `TemplatesList`

```rust
pub struct TemplatesList
```

The project's view templates: the properties each holds, which of them
it controls (`include`) and the views that follow it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-graphics-templatecreate"></a>

### struct `TemplateCreate`

```rust
pub struct TemplateCreate
```

Create a view template, from the properties of `view` when given
(scale, category graphics, filters, plan cut height, building services
display, structural display), else with defaults. Every property is
controlled (`include`) until changed with `template_set`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique).

- `view: Option<Ref>`

  View to take the properties from.


<a id="cc_engine-commands-graphics-templateset"></a>

### struct `TemplateSet`

```rust
pub struct TemplateSet
```

Change a view template: name, properties and which of them it controls
(`include`, merged: `{"scale": false}` lets views keep their own scale).
Views that follow the template change with it, in the same undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `template: Ref`

  Template id or name.

- `name: Option<String>`

  New name.

- `scale: Option<refs::ScaleArg>`

  Scale: the denominator (e.g. 50 for 1:50), or text such as `"1:50"`
  or an imperial label like `"1/4\" = 1'-0\""` (48).

- `cut_height: Option<f64>`

  Plans: cut plane height, meters.

- `discipline: Option<String>`

  Floor plans: `none`, `plumbing`, `electrical`, `communications` or `lightning` (services over a
  faint architecture).

- `structural: Option<bool>`

  Floor plans: structural (formwork) display.

- `structure_tags: Option<bool>`

  Floor plans: tag beams and columns.

- `phase_filter: Option<Option<Ref>>`

  Phase filter (id or name), or `null` for none.

- `detail: Option<String>`

  Detail level: `coarse`, `medium` or `fine`.

- `visual_style: Option<String>`

  Visual style: `wireframe`, `hidden_line`, `shaded`,
  `consistent_colors`, `textures`, or `default` (each view's usual style).

- `underlay: Option<Value>`

  Plans: other levels' plans drawn under them, merged like `view_set`'s
  `underlay` (e.g. `{"below": {"level": "Next", "looking": "Down"}}`).

- `design_options: Option<Vec<super::design_options::OptionChoice>>`

  The option shown of each design option set (replacing the
  template's choices; sets left out show automatically).

- `include: Option<Value>`

  Which properties the template controls: `scale`, `categories`,
  `filters`, `cut_height`, `discipline`, `structural`, `phase_filter`,
  `detail`, `visual_style`, `underlay`, `design_options` (true/false).
  `design_options` is off unless set.


<a id="cc_engine-commands-graphics-templatedelete"></a>

### struct `TemplateDelete`

```rust
pub struct TemplateDelete
```

Delete a view template; views that follow it keep their current
properties and become free.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `template: Ref`

  Template id or name.


<a id="cc_engine-commands-graphics-viewtemplateapply"></a>

### struct `ViewTemplateApply`

```rust
pub struct ViewTemplateApply
```

Copy a template's properties into views once (they stay free to
change). Views that follow another template are refused.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `views: Vec<Ref>`

  View ids or names.

- `template: Ref`

  Template id or name.


<a id="cc_engine-commands-graphics-viewtemplateassign"></a>

### struct `ViewTemplateAssign`

```rust
pub struct ViewTemplateAssign
```

Make views follow a template (`template: null` frees them): the
properties it controls are copied now and whenever the template changes,
and cannot be changed in the view.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `views: Vec<Ref>`

  View ids or names.

- `template: Option<Ref>`

  Template id or name, or null to unassign.


<a id="cc_engine-commands-graphics-viewtemporary"></a>

### struct `ViewTemporary`

```rust
pub struct ViewTemporary
```

Temporarily hide or isolate elements or categories in a view, or reset.
This is display state, not a model change: it is not saved, not undone
and not printed (sheets and exports show the view as it is); the
application marks the view while it is active. `action`: `hide` and
`isolate` (the elements `ids`), `hide_category` and `isolate_category`
(`categories`, and the categories of `ids`), or `reset`. Hiding adds to
what is already hidden; isolating replaces it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `action: String`

  `hide`, `isolate`, `hide_category`, `isolate_category` or `reset`.

- `ids: Vec<u64>`

  Element ids.

- `categories: Vec<String>`

  Category names.


<a id="cc_engine-commands-graphics-viewtemporaryapply"></a>

### struct `ViewTemporaryApply`

```rust
pub struct ViewTemporaryApply
```

Make a view's temporary hiding permanent: hidden elements become hidden
in the view and hidden categories are turned off; when isolating, every
other element drawn in the view is hidden in it. The temporary state is
then reset. One undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name.


<a id="mod-cc_engine-commands-grid_frame"></a>

## `cc_engine::commands::grid_frame`

<sub>`crates/cc_engine/src/commands/grid_frame.rs`</sub>

A multi-storey grid frame: on every storey of a range of levels, columns
at the grid intersections (tops attached to the level above) and beams
along the grids on the upper level, plus optional secondary beams, in one
undo step.

**Items:** [`MAX_MEMBERS`](#cc_engine-commands-grid_frame-max_members) · [`SecondaryArg`](#cc_engine-commands-grid_frame-secondaryarg) · [`GridFrameCreate`](#cc_engine-commands-grid_frame-gridframecreate)

<a id="cc_engine-commands-grid_frame-max_members"></a>

### const `MAX_MEMBERS`

```rust
pub const MAX_MEMBERS: usize
```

Most members one run creates.

<a id="cc_engine-commands-grid_frame-secondaryarg"></a>

### struct `SecondaryArg`

```rust
pub struct SecondaryArg
```

Secondary beams in each bay, parallel to a grid.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `parallel_to: u64`

  The grid the secondary beams run parallel to (one of `grids`).

- `spacing: f64`

  The largest distance between beams, meters (at least 0.3).


<a id="cc_engine-commands-grid_frame-gridframecreate"></a>

### struct `GridFrameCreate`

```rust
pub struct GridFrameCreate
```

Frame a range of levels on the grids: on every storey (each pair of
consecutive levels from `base_level` to `top_level`), a column at each
grid intersection of the lower level with its top attached to the upper
one, and beams on the upper level along each grid between neighboring
columns. Columns and beams already modeled are skipped. With `secondary`,
each bay between two grids parallel to `parallel_to` and two crossing
grids gets beams parallel to it, at most `spacing` apart. One undo step;
returns the new `columns`, `beams` and `secondary` beams and how many
were `skipped` (and bays `skipped_bays`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `grids: Vec<u64>`

  Grid ids (default: every grid).

- `base_level: Option<Ref>`

  The lowest level (default: the lowest level of the project).

- `top_level: Option<Ref>`

  The highest level (default: the level above the base).

- `column_type: Option<Ref>`

  Column type id or name (default: the first of `material`, made when missing).

- `beam_type: Option<Ref>`

  Beam type id or name (default: the first of `material`, made when missing).

- `material: Option<String>`

  `concrete`, `steel` (default) or `timber`: the default types' material.

- `secondary: Option<SecondaryArg>`

  Secondary beams in each bay.


<a id="mod-cc_engine-commands-io"></a>

## `cc_engine::commands::io`

<sub>`crates/cc_engine/src/commands/io.rs`</sub>

Files: IFC import/export, SketchUp and OBJ import, sheet PDF/SVG, view PNG/SVG/PDF, schedule CSV, component files.

**Items:** [`IfcImport`](#cc_engine-commands-io-ifcimport) · [`IfcExport`](#cc_engine-commands-io-ifcexport) · [`SheetExport`](#cc_engine-commands-io-sheetexport) · [`ViewExport`](#cc_engine-commands-io-viewexport) · [`ScheduleExportCsv`](#cc_engine-commands-io-scheduleexportcsv) · [`ComponentLoad`](#cc_engine-commands-io-componentload) · [`SkpImport`](#cc_engine-commands-io-skpimport) · [`ObjUnits`](#cc_engine-commands-io-objunits) · [`ObjImport`](#cc_engine-commands-io-objimport) · [`GltfImport`](#cc_engine-commands-io-gltfimport) · [`SkpComponentLoad`](#cc_engine-commands-io-skpcomponentload) · [`ComponentLibraryList`](#cc_engine-commands-io-componentlibrarylist) · [`ComponentSave`](#cc_engine-commands-io-componentsave)

<a id="cc_engine-commands-io-ifcimport"></a>

### struct `IfcImport`

```rust
pub struct IfcImport
```

Import an IFC file (IFC2X3 or IFC4), replacing the session's project. The
result has no file yet; save it with `project_save` and a path.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .ifc file.


<a id="cc_engine-commands-io-ifcexport"></a>

### struct `IfcExport`

```rust
pub struct IfcExport
```

Export the project as IFC4.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .ifc file to write.


<a id="cc_engine-commands-io-sheetexport"></a>

### struct `SheetExport`

```rust
pub struct SheetExport
```

Export a sheet as vector PDF or SVG, or as DXF in paper millimeters (by
`format`, else by the file extension).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `sheet: Ref`

  Sheet id or name.

- `path: PathBuf`

  Path of the file to write (.pdf, .svg or .dxf).

- `format: Option<String>`

  `pdf`, `svg` or `dxf`.


<a id="cc_engine-commands-io-viewexport"></a>

### struct `ViewExport`

```rust
pub struct ViewExport
```

Export any view with a drawing (plan, elevation, section, 3D view as a
hidden-line axonometric, legend, drafting view, diagram or sheet) as a
PNG image, as vector SVG or PDF, or as DXF for other CAD programs (by
`format`, else by the file extension). The view is drawn at its scale as
printed, without temporary hiding, on a page that fits it; sheets keep
their paper size. A DXF holds the view at full size in millimeters (a
sheet in paper millimeters), each element on the layer of its category
(detail lines, and regions with a boundary, on the layer named after
their line style; region fills as `SOLID` triangles, their patterns as
lines), colors as the nearest color index and line patterns as line types.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `path: PathBuf`

  Path of the file to write (.png, .svg, .pdf or .dxf).

- `format: Option<String>`

  `png`, `svg`, `pdf` or `dxf`.

- `dpi: Option<f64>`

  Resolution of a PNG, in dots per inch (default 150).


**Methods**

- `pub fn page(s: &mut Session, view: Ref) -> Result<(cc_views::Drawing2D, glam::DVec2)>`

  The view drawn on paper (millimeters) and the paper size.


<a id="cc_engine-commands-io-scheduleexportcsv"></a>

### struct `ScheduleExportCsv`

```rust
pub struct ScheduleExportCsv
```

Export a schedule view as CSV (UTF-8; semicolons when the decimal comma is used).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  Schedule view id or name.

- `path: PathBuf`

  Path of the .csv file to write.


<a id="cc_engine-commands-io-componentload"></a>

### struct `ComponentLoad`

```rust
pub struct ComponentLoad
```

Load a component file (.ccomp): its types and missing materials join the project.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .ccomp file.


<a id="cc_engine-commands-io-skpimport"></a>

### struct `SkpImport`

```rust
pub struct SkpImport
```

Import a SketchUp model (.skp) into the project. Each placed component
or group becomes a component element on `level`, of a component type
holding its faces as fixed mesh pieces (one per material): placements
that differ only by a move or a plan rotation share a type, anything
else in their transform (tilt, scale, mirror) is built into the type.
Faces outside any component make one more element at the file's origin.
Missing materials are added by name, color, opacity and texture; hidden
objects and those on hidden layers are left out.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .skp file.

- `level: Option<Ref>`

  Level id or name (default: the lowest level); the file's ground is at its elevation.

- `position: Option<Point>`

  Where the file's origin goes, in plan (default [0, 0]).

- `rotation: Option<f64>`

  Rotation of the whole model in plan, degrees counter-clockwise (default 0).


<a id="cc_engine-commands-io-objunits"></a>

### enum `ObjUnits`

```rust
pub enum ObjUnits
```

Units of the coordinates in an OBJ file (the format does not say).

**Traits:** `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `M`
- `Cm`
- `Mm`
- `In`
- `Ft`

<a id="cc_engine-commands-io-objimport"></a>

### struct `ObjImport`

```rust
pub struct ObjImport
```

Import a Wavefront OBJ model (.obj) into the project. Each object (or,
in a file without objects, each group) becomes a component element on
`level`, of a component type holding its faces as fixed mesh pieces, one
per material, inserted at the center of its footprint and the bottom of
its extent. Materials come from the `.mtl` libraries next to the file:
color (`Kd`), opacity (`d`, `Tr`) and texture (`map_Kd`, the picture next
to the file, repeating as the faces' `vt` coordinates say); missing
materials are added by name. Faces not flat are split into triangles;
lines, points and normals are ignored.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .obj file.

- `level: Option<Ref>`

  Level id or name (default: the lowest level); the file's zero height is at its elevation.

- `position: Option<Point>`

  Where the file's origin goes, in plan (default [0, 0]).

- `rotation: Option<f64>`

  Rotation of the whole model in plan, degrees counter-clockwise (default 0).

- `units: ObjUnits`

  Units of the file's coordinates: "m" (default), "cm", "mm", "in" or "ft".

- `z_up: bool`

  The file's Z axis points up; by default Y does (the usual OBJ convention).

- `merge: bool`

  Make the whole file one component element, inserted at the file's origin.


<a id="cc_engine-commands-io-gltfimport"></a>

### struct `GltfImport`

```rust
pub struct GltfImport
```

Import a glTF 2.0 model (.glb, or .gltf with its buffers embedded or in
files next to it) into the project. Each node of the scene's root, with
everything below it, becomes a component element on `level`, of a
component type holding its triangles as fixed mesh pieces, one per
material, inserted at the center of its footprint and the bottom of its
extent. glTF is in meters with Y up. Materials take their base color
times its texture's average, the texture itself (repeating as the
texture coordinates say) and, when blended, their opacity; missing
materials are added by name. Lines and points are ignored, and
compressed meshes (Draco, meshopt) are refused.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .glb or .gltf file.

- `level: Option<Ref>`

  Level id or name (default: the lowest level); the file's zero height is at its elevation.

- `position: Option<Point>`

  Where the file's origin goes, in plan (default [0, 0]).

- `rotation: Option<f64>`

  Rotation of the whole model in plan, degrees counter-clockwise (default 0).

- `merge: bool`

  Make the whole scene one component element, inserted at the file's origin.


<a id="cc_engine-commands-io-skpcomponentload"></a>

### struct `SkpComponentLoad`

```rust
pub struct SkpComponentLoad
```

Load a SketchUp model (.skp) as one component type, named after the
file, to place like any component: its faces become fixed mesh pieces
(one per material) with the file's origin as the insertion point.
Missing materials are added by name and color.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .skp file.


<a id="cc_engine-commands-io-componentlibrarylist"></a>

### struct `ComponentLibraryList`

```rust
pub struct ComponentLibraryList
```

List the component library: the component files (.ccomp) in `folders`
and their subfolders, each with its component, kind, category (doors
and windows have their own), types, parameters and connectors; `search`
keeps those whose component, type names or descriptions contain it.
Files that do not load are listed with the reason.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `folders: Vec<PathBuf>`

  Folders to look in.

- `search: Option<String>`

  Text to look for (any case).


<a id="cc_engine-commands-io-componentsave"></a>

### struct `ComponentSave`

```rust
pub struct ComponentSave
```

Save a component (all its types and the materials they use) to a .ccomp file.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `component: String`

  Component name (as in a component type's `params.component`).

- `path: PathBuf`

  Path of the .ccomp file to write.


<a id="mod-cc_engine-commands-level_mark"></a>

## `cc_engine::commands::level_mark`

<sub>`crates/cc_engine/src/commands/level_mark.rs`</sub>

Level marks: the elevation of a point written in a plan, an elevation or
a section (see `cc_core::level_mark`).

**Items:** [`LevelMarkCreate`](#cc_engine-commands-level_mark-levelmarkcreate) · [`LevelMarkTypeCreate`](#cc_engine-commands-level_mark-levelmarktypecreate)

<a id="cc_engine-commands-level_mark-levelmarkcreate"></a>

### struct `LevelMarkCreate`

```rust
pub struct LevelMarkCreate
```

Place a level mark in a plan, an elevation or a section: the finished
level of a point above a shelf line and its raw (structural) level under
it, each after the type's datum (the value written for the project's
zero) is added. Plans draw a circle with two filled quarters on the
point and measure the top of the highest floor of the plan's level under
it (the level where there is none), with the top of that floor's
structure as the raw level; elevations and sections draw a triangle
pointing down to the point and measure its height (`position` y), with
the raw level of a floor whose top the point lies on, in sections.
`element_get` gives the levels and texts (`derived`); `element_set`
changes the mark's `params`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  View id or name: a floor or ceiling plan, an elevation or a section.

- `position: Point`

  The point measured, in the view's drawing coordinates (in elevations
  and sections `y` is the elevation).

- `type_ref: Option<Ref>`

  Level mark type id or name (default: the default level mark type).

- `flip: Option<bool>`

  Write the values on the left of the symbol (default: on the right).

- `finished: Option<f64>`

  Finished level to write instead of the measured one, meters (an
  elevation of the project, before the datum).

- `raw: Option<f64>`

  Raw level to write instead of the measured one, meters.

- `show_raw: Option<bool>`

  Write the raw level when there is one (default true).


<a id="cc_engine-commands-level_mark-levelmarktypecreate"></a>

### struct `LevelMarkTypeCreate`

```rust
pub struct LevelMarkTypeCreate
```

Create a level mark type: the text size, the datum (the value written
for the project's zero, e.g. 100 when the ground floor is +100.00) and
the suffixes after the finished and raw levels. A project gets one with
its first level mark; `type_set` changes them.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the language's).

- `text_size: Option<f64>`

  Text size, paper millimeters (default 2.5; 3/32", 2.38125, in imperial projects).

- `datum: Option<f64>`

  Value written for the project's zero, meters (default 0).

- `finished_suffix: Option<String>`

  Text after the finished level (default: the language's, e.g. "(N.A.)").

- `raw_suffix: Option<String>`

  Text after the raw level (default: the language's, e.g. "(N.O.)").


<a id="mod-cc_engine-commands-levels"></a>

## `cc_engine::commands::levels`

<sub>`crates/cc_engine/src/commands/levels.rs`</sub>

Levels: create (with their plans, also as a storey above or below
another), rename, change elevation or storey height, delete.

**Items:** [`height_above`](#cc_engine-commands-levels-height_above) · [`height_below`](#cc_engine-commands-levels-height_below) · [`LevelCreate`](#cc_engine-commands-levels-levelcreate) · [`LevelSet`](#cc_engine-commands-levels-levelset) · [`LevelDelete`](#cc_engine-commands-levels-leveldelete)

<a id="cc_engine-commands-levels-height_above"></a>

### fn `height_above`

```rust
pub fn height_above(p: &Project, id: LevelId) -> Option<f64>
```

The height from level `id` up to the next level, if there is one.

<a id="cc_engine-commands-levels-height_below"></a>

### fn `height_below`

```rust
pub fn height_below(p: &Project, id: LevelId) -> Option<f64>
```

The height from the level below `id` up to it, if there is one.

<a id="cc_engine-commands-levels-levelcreate"></a>

### struct `LevelCreate`

```rust
pub struct LevelCreate
```

Add a level (default: 3 m above the highest, named "Level n") with its
floor and ceiling plans. With `above` or `below` it is a new storey next
to that level: the levels beyond it move away by the new storey's height,
so the other storeys keep theirs and the order of the levels stays.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Level name (default: "Level n").

- `elevation: Option<f64>`

  Elevation in meters (not with `above` or `below`).

- `above: Option<Ref>`

  Add the level above this one; the levels above it move up by `height`.

- `below: Option<Ref>`

  Add the level below this one; the levels below it move down by `height`.

- `height: Option<f64>`

  With `above` or `below`: the distance to that level, meters (default:
  the height of the storey next to it on that side, else 3 m).

- `plans: Option<bool>`

  Also create its floor and ceiling plans (default true).


<a id="cc_engine-commands-levels-levelset"></a>

### struct `LevelSet`

```rust
pub struct LevelSet
```

Rename a level, change its elevation or the height up to the next level
(what is attached to a moved level moves along). A new `height` moves
every level above it by the difference, so the levels keep their order and
the other storeys their heights. Renaming it leaves its plans' names as
they are: a level and its views are named apart.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `level: Ref`
- `name: Option<String>`

  New name.

- `elevation: Option<f64>`

  New elevation, meters (not with `height`).

- `height: Option<f64>`

  New height from this level up to the next one, meters; the levels
  above move along. Refused for the highest level.


<a id="cc_engine-commands-levels-leveldelete"></a>

### struct `LevelDelete`

```rust
pub struct LevelDelete
```

Delete a level no element is on, with its plan views; walls rising to it
keep their current height, unconnected. The last level is kept.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `level: Ref`

<a id="mod-cc_engine-commands-lightning"></a>

## `cc_engine::commands::lightning`

<sub>`crates/cc_engine/src/commands/lightning.rs`</sub>

Lightning protection: conductors on the roof, down the walls and in the
ground (an earth ring), air terminals on the roof, earth electrodes, and
a coverage check of the roofs (see `cc_core::lightning`,
`cc_geom::lightning`). Elements placed without a system join the
project's first lightning protection system, created with the first of them.

**Items:** [`new_conductor_type`](#cc_engine-commands-lightning-new_conductor_type) · [`LpsConductorCreate`](#cc_engine-commands-lightning-lpsconductorcreate) · [`EarthRingCreate`](#cc_engine-commands-lightning-earthringcreate) · [`AirTerminalPlace`](#cc_engine-commands-lightning-airterminalplace) · [`EarthElectrodePlace`](#cc_engine-commands-lightning-earthelectrodeplace) · [`LpsConductorTypeCreate`](#cc_engine-commands-lightning-lpsconductortypecreate) · [`LpsCheck`](#cc_engine-commands-lightning-lpscheck)

<a id="cc_engine-commands-lightning-new_conductor_type"></a>

### fn `new_conductor_type`

```rust
pub fn new_conductor_type(id: ElementTypeId, diameter: f64, material: String) -> ElementType
```

A conductor type of `diameter`, named by it (in millimeters, or in
inches in imperial units).

<a id="cc_engine-commands-lightning-lpsconductorcreate"></a>

### struct `LpsConductorCreate`

```rust
pub struct LpsConductorCreate
```

Create a lightning protection conductor from `start` to `end`, or a
down conductor straight down from `start` (no `end`). Without offsets, a
conductor lies on the roof under its ends (else where it starts on
another conductor, else at the level); a down conductor runs from the
roof over `start` (else the level above) down to the earthing,
0.5 m below the level. `role` is `air` (a roof conductor), `down` or
`earth`; by default vertical runs are down conductors, runs below the
level earth conductors and the others roof conductors. Conductors join
each other where their ends meet. A project without a conductor type gets
one (8 mm, copper); without a lightning protection system, one is made.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start in plan.

- `end: Option<Point>`

  End in plan (default: the start, for a down conductor).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Conductor type id or name (default: the first conductor type).

- `role: Option<String>`

  `air`, `down` or `earth` (default: by where it runs).

- `offset: Option<f64>`

  Elevation of the start above the level, meters.

- `end_offset: Option<f64>`

  Elevation of the end above the level, meters.

- `system: Option<Ref>`

  Lightning protection system id, name or code (default: the first).


<a id="cc_engine-commands-lightning-earthringcreate"></a>

### struct `EarthRingCreate`

```rust
pub struct EarthRingCreate
```

Create an earth ring: buried conductors (role `earth`) around `points`
(a closed outline), or by default around the walls of the level,
`distance` (default 1 m) out from their outer faces, `depth` (default
0.5 m) below the level, one conductor per side.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `points: Option<Vec<Point>>`

  The outline in plan (default: around the walls of the level).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `depth: Option<f64>`

  Depth below the level, meters.

- `distance: Option<f64>`

  Distance out from the walls' outer faces, meters (without `points`).

- `type_ref: Option<Ref>`

  Conductor type id or name (default: the first conductor type).

- `system: Option<Ref>`

  Lightning protection system id, name or code (default: the first).


<a id="cc_engine-commands-lightning-airterminalplace"></a>

### struct `AirTerminalPlace`

```rust
pub struct AirTerminalPlace
```

Place an air terminal (a lightning rod) at `position`: on the roof (or
floor) surface under it by default, else at the level above; `height` is
the rod's height (default: its type's, 1 m for the built-in type). A
project without an air terminal type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Position in plan.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `height: Option<f64>`

  Rod height, meters.

- `offset: Option<f64>`

  Elevation of its base above the level, meters (default: the roof under it).

- `type_ref: Option<Ref>`

  Air terminal type id or name.

- `system: Option<Ref>`

  Lightning protection system id, name or code (default: the first).


<a id="cc_engine-commands-lightning-earthelectrodeplace"></a>

### struct `EarthElectrodePlace`

```rust
pub struct EarthElectrodePlace
```

Place an earth electrode (a driven rod) at `position`: its top `depth`
(default 0.5 m) below the level, `length` long (default: its type's,
2.4 m for the built-in type). A project without an earth electrode type
gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Position in plan.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `length: Option<f64>`

  Rod length, meters.

- `depth: Option<f64>`

  Depth of its top below the level, meters.

- `type_ref: Option<Ref>`

  Earth electrode type id or name.

- `system: Option<Ref>`

  Lightning protection system id, name or code (default: the first).


<a id="cc_engine-commands-lightning-lpsconductortypecreate"></a>

### struct `LpsConductorTypeCreate`

```rust
pub struct LpsConductorTypeCreate
```

Create a lightning protection conductor type: a round conductor of
`diameter` (meters, 2 to 50 mm; default 8 mm, 3/8" in imperial projects) in `material` (default:
copper, else the first metal), named by its diameter unless `name` is given.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `diameter: Option<f64>`

  Diameter, meters (default 0.008; 3/8" in imperial projects).

- `material: Option<String>`

  Material name.

- `name: Option<String>`

  Type name.


<a id="cc_engine-commands-lightning-lpscheck"></a>

### struct `LpsCheck`

```rust
pub struct LpsCheck
```

Check how the air termination (air terminals and roof conductors)
covers the roofs, for a protection `class` (`I` to `IV`, default `III`):
the exposed upper faces of roofs (and of floors above the lowest level
that nothing covers) are sampled about `spacing` meters apart (default
1 m; more on large roofs) and each point is protected when one air
termination point protects it by itself, by the rolling sphere method
(sphere radius 20, 30, 45 or 60 m by class) over a horizontal plane at
the roof's lowest exposed point, or, on flat parts, when it lies within
half the class's mesh width (5, 10, 15 or 20 m) of a roof conductor.
Mutual protection between terminals and the shielding by the building
itself are left out, which errs on the safe side. Returns the counts,
the share protected and up to 200 unprotected points.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `class: Option<String>`

  Protection class, `I` to `IV`.

- `spacing: Option<f64>`

  Distance between the sampled points, meters.


<a id="mod-cc_engine-commands-line_style"></a>

## `cc_engine::commands::line_style`

<sub>`crates/cc_engine/src/commands/line_style.rs`</sub>

Line styles: the built-in styles and the project's own (see
`cc_core::detail`), used by detail lines and region boundaries. The
project's styles are project settings: `line_style_create`,
`line_style_set` and `line_style_delete` change them in one undo step
each; deleting a style moves what uses it to another style.

**Items:** [`pattern_code`](#cc_engine-commands-line_style-pattern_code) · [`LineStylesList`](#cc_engine-commands-line_style-linestyleslist) · [`LineStyleCreate`](#cc_engine-commands-line_style-linestylecreate) · [`LineStyleSet`](#cc_engine-commands-line_style-linestyleset) · [`LineStyleDelete`](#cc_engine-commands-line_style-linestyledelete)

<a id="cc_engine-commands-line_style-pattern_code"></a>

### fn `pattern_code`

```rust
pub fn pattern_code(p: LinePattern) -> &'static str
```

A dash pattern's code: `solid`, `dash`, `short_dash`, `dot` or `dash_dot`.

<a id="cc_engine-commands-line_style-linestyleslist"></a>

### struct `LineStylesList`

```rust
pub struct LineStylesList
```

The line styles detail lines and region boundaries can use: the
built-in ones (by code) and the project's (by name), each with its
weight on paper (mm), color, dash pattern and how many elements use it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-line_style-linestylecreate"></a>

### struct `LineStyleCreate`

```rust
pub struct LineStyleCreate
```

Create a project line style for detail lines and region boundaries:
a unique name (not a built-in code), a weight on paper, a color and a
dash pattern. Commands then name it like a built-in style.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique, not a built-in style's code).

- `weight: Option<f64>`

  Width on paper, millimeters (0.05 to 5; default 0.25).

- `color: Option<[u8; 3]>`

  Color `[r, g, b]`, 0 to 255 (default black).

- `pattern: Option<String>`

  Dash pattern: solid (default), dash, short_dash, dot or dash_dot.


<a id="cc_engine-commands-line_style-linestyleset"></a>

### struct `LineStyleSet`

```rust
pub struct LineStyleSet
```

Change a project line style: name, weight, color or pattern. The lines
and boundaries drawn with it follow.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `style: Ref`

  Line style id or name.

- `name: Option<String>`

  New name.

- `weight: Option<f64>`

  Width on paper, millimeters (0.05 to 5).

- `color: Option<[u8; 3]>`

  Color `[r, g, b]`.

- `pattern: Option<String>`

  Dash pattern: solid, dash, short_dash, dot or dash_dot.


<a id="cc_engine-commands-line_style-linestyledelete"></a>

### struct `LineStyleDelete`

```rust
pub struct LineStyleDelete
```

Delete a project line style. The detail lines and region boundaries
drawn with it move to `replace_with` (a built-in code or another
project style's name; default `thin`) in the same undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `style: Ref`

  Line style id or name.

- `replace_with: Option<String>`

  The style that takes its place where it is used (default `thin`).


<a id="mod-cc_engine-commands-mass"></a>

## `cc_engine::commands::mass`

<sub>`crates/cc_engine/src/commands/mass.rs`</sub>

Conceptual masses: creating them (an extrusion, or a loft to another
outline) and choosing and reading their mass floors.

**Items:** [`MassCreate`](#cc_engine-commands-mass-masscreate) · [`spanned_levels`](#cc_engine-commands-mass-spanned_levels) · [`new_mass_type`](#cc_engine-commands-mass-new_mass_type) · [`MassFloorsSet`](#cc_engine-commands-mass-massfloorsset) · [`MassFloors`](#cc_engine-commands-mass-massfloors) · [`floors_json`](#cc_engine-commands-mass-floors_json) · [`MassTypeCreate`](#cc_engine-commands-mass-masstypecreate) · [`MassFaceMove`](#cc_engine-commands-mass-massfacemove) · [`MassWallsCreate`](#cc_engine-commands-mass-masswallscreate) · [`MassSlabsCreate`](#cc_engine-commands-mass-massslabscreate) · [`MassRoofCreate`](#cc_engine-commands-mass-massroofcreate)

<a id="cc_engine-commands-mass-masscreate"></a>

### struct `MassCreate`

```rust
pub struct MassCreate
```

Create a conceptual mass: `outline` extruded `height` up from its level
(plus `base_offset`), or lofted to another outline (`top`, or the base
outline set in by `top_inset`). Mass floors are taken at every level the
mass spans unless `floors` lists them. A project without a mass type
gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `outline: Vec<Point>`

  Base outline `[[x, y], …]` in plan, closed (at least three points); or `curves`.

- `curves: Option<Vec<super::sketch::CurveArg>>`

  The base outline drawn with curves (lines, arcs, circles, ellipses,
  rectangles, polygons) in one loop, instead of `outline`; the mass keeps them.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `height: Option<f64>`

  Height above the base, meters (default: up to the level above, else 3; 10'-0" in imperial projects).

- `base_offset: Option<f64>`

  Offset of the base from the level elevation, meters (default 0).

- `top: Option<Vec<Point>>`

  Top outline of a loft `[[x, y], …]` (default: the base outline, an extrusion).

- `top_inset: Option<f64>`

  Loft to the base outline set in by this distance, meters (instead of `top`).

- `floors: Option<Vec<Ref>>`

  Levels (ids or names) where mass floors are taken (default: every level the mass spans).


<a id="cc_engine-commands-mass-spanned_levels"></a>

### fn `spanned_levels`

```rust
pub fn spanned_levels(p: &Project, base: f64, top: f64) -> Vec<LevelId>
```

The levels a mass from `base` to `top` (elevations) spans: at or above
the base, below the top.

<a id="cc_engine-commands-mass-new_mass_type"></a>

### fn `new_mass_type`

```rust
pub fn new_mass_type(p: &Project, id: ElementTypeId) -> ElementType
```

A mass type for a project that has none, in its first material.

<a id="cc_engine-commands-mass-massfloorsset"></a>

### struct `MassFloorsSet`

```rust
pub struct MassFloorsSet
```

Choose the levels where a mass takes mass floors: `levels` (ids or
names), or every level it spans when `levels` is left out; an empty list
takes none.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.

- `levels: Option<Vec<Ref>>`

  Levels (ids or names); left out: every level the mass spans.


<a id="cc_engine-commands-mass-massfloors"></a>

### struct `MassFloors`

```rust
pub struct MassFloors
```

The mass floors of a mass: for each, its level, elevation, area and
perimeter (and outline), with the gross floor area and the volume.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.


<a id="cc_engine-commands-mass-floors_json"></a>

### fn `floors_json`

```rust
pub fn floors_json(p: &Project, g: &cc_geom::mass::MassGeom) -> Vec<Value>
```

A mass's floors as JSON.

<a id="cc_engine-commands-mass-masstypecreate"></a>

### struct `MassTypeCreate`

```rust
pub struct MassTypeCreate
```

Create a mass type, shown in `material` (default: the project's first material).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's mass name).

- `material: Option<String>`

  Material name.


<a id="cc_engine-commands-mass-massfacemove"></a>

### struct `MassFaceMove`

```rust
pub struct MassFaceMove
```

Push or pull a face of a mass `distance` meters outward (inward when
negative): `top` changes its height, `bottom` its base offset and
height, and `side` (with `side`, the index of the base outline's edge
from that point to the next) moves that side, the sides beside it
keeping their directions. A loft's sides are not moved.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.

- `face: String`

  `top`, `bottom` or `side`.

- `side: Option<usize>`

  The side's edge of the base outline (with `face: "side"`).

- `distance: f64`

  Distance outward, meters (negative: inward).


<a id="cc_engine-commands-mass-masswallscreate"></a>

### struct `MassWallsCreate`

```rust
pub struct MassWallsCreate
```

Walls by face: a wall over each side of a mass extruded straight up
(`sides`: the base outline's edges, by default all), its exterior face
on the mass's face, from the mass's base up its height. Walls meeting at
the mass's corners join. A loft's slanted sides take no walls.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.

- `type_ref: Option<Ref>`

  Wall type id or name (default: the category's default type).

- `sides: Option<Vec<usize>>`

  Edges of the base outline to put walls on (default: every edge).


<a id="cc_engine-commands-mass-massslabscreate"></a>

### struct `MassSlabsCreate`

```rust
pub struct MassSlabsCreate
```

Floors from mass floors: a floor on each mass floor of a mass (or those
at `levels`), its outline the mass's at that level and its top at the level.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.

- `type_ref: Option<Ref>`

  Floor type id or name (default: the category's default type).

- `levels: Option<Vec<Ref>>`

  Levels (ids or names) of the mass floors to build (default: all of them).


<a id="cc_engine-commands-mass-massroofcreate"></a>

### struct `MassRoofCreate`

```rust
pub struct MassRoofCreate
```

Roof by face: a flat roof on the top face of a mass, over its top
outline, on the mass's level at the height of its top.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Mass element id.

- `type_ref: Option<Ref>`

  Roof type id or name (default: the category's default type).

- `overhang: Option<f64>`

  How far the eaves reach past the top outline, meters (default 0).


<a id="mod-cc_engine-commands-materials"></a>

## `cc_engine::commands::materials`

<sub>`crates/cc_engine/src/commands/materials.rs`</sub>

Materials: create, duplicate, rename, change, delete.

**Items:** [`material_json`](#cc_engine-commands-materials-material_json) · [`MaterialCreate`](#cc_engine-commands-materials-materialcreate) · [`MaterialDuplicate`](#cc_engine-commands-materials-materialduplicate) · [`MaterialRename`](#cc_engine-commands-materials-materialrename) · [`MaterialSet`](#cc_engine-commands-materials-materialset) · [`MaterialDelete`](#cc_engine-commands-materials-materialdelete) · [`MaterialTextureSet`](#cc_engine-commands-materials-materialtextureset) · [`MaterialTextureClear`](#cc_engine-commands-materials-materialtextureclear)

<a id="cc_engine-commands-materials-material_json"></a>

### fn `material_json`

```rust
pub(crate) fn material_json(m: &Material) -> Value
```

A material's fields as JSON, its texture's image as a summary (format,
size in pixels and bytes) instead of its bytes.

<a id="cc_engine-commands-materials-materialcreate"></a>

### struct `MaterialCreate`

```rust
pub struct MaterialCreate
```

Create a material (default name "Material"; a number is added when taken).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: Option<String>`

  Name (default: "Material"; a number is added when taken).

- `color: Option<[u8; 3]>`

  Shading color `[r, g, b]`, 0–255 (default light gray).


<a id="cc_engine-commands-materials-materialduplicate"></a>

### struct `MaterialDuplicate`

```rust
pub struct MaterialDuplicate
```

Copy a material under a new name (default "<name> (copy)").

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`
- `name: Option<String>`

  Name of the copy (default: "<name> (copy)").


<a id="cc_engine-commands-materials-materialrename"></a>

### struct `MaterialRename`

```rust
pub struct MaterialRename
```

Rename a material; every type using it follows.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`
- `name: String`

  New name.


<a id="cc_engine-commands-materials-materialset"></a>

### struct `MaterialSet`

```rust
pub struct MaterialSet
```

Change a material's fields (merged into what `material_get` shows:
description, class, colors, patterns, transparency, finish, physical
values…). Use `material_rename` to rename it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`
- `values: Value`

  Fields to change, shaped like the result of `material_get`.


<a id="cc_engine-commands-materials-materialdelete"></a>

### struct `MaterialDelete`

```rust
pub struct MaterialDelete
```

Delete a material no type uses.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`

<a id="cc_engine-commands-materials-materialtextureset"></a>

### struct `MaterialTextureSet`

```rust
pub struct MaterialTextureSet
```

Give a material an image texture (shown on its faces in the textures
visual style of 3D views), or change how its texture is laid out. The
image comes from `path` or from `data` (base64); with neither, the
current texture's size, offset and rotation change. One repeat covers
`width` × `height` meters on the faces: on floors and other horizontal
faces along plan X and Y, on walls and slopes level across the face and
up it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`
- `path: Option<std::path::PathBuf>`

  Path of a PNG, JPEG, GIF, BMP or WebP file.

- `data: Option<String>`

  The image file's bytes as base64 (instead of `path`).

- `width: Option<f64>`

  Width of one repeat, meters (default: the current one, else 1 m).

- `height: Option<f64>`

  Height of one repeat, meters (default: the current one, else the
  width in the image's proportions).

- `offset: Option<[f64; 2]>`

  Where a repeat starts, `[x, y]` meters along the face (default: the current one, else `[0, 0]`).

- `rotation: Option<f64>`

  Rotation of the image on the faces, degrees counter-clockwise (default: the current one, else 0).


<a id="cc_engine-commands-materials-materialtextureclear"></a>

### struct `MaterialTextureClear`

```rust
pub struct MaterialTextureClear
```

Take a material's texture away (its faces show the appearance color).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`

<a id="mod-cc_engine-commands-mep"></a>

## `cc_engine::commands::mep`

<sub>`crates/cc_engine/src/commands/mep.rs`</sub>

Building services: pipes and conduits, plumbing systems and electrical
circuits, and what they add up to (lengths, loads, fixture units).

**Items:** [`parse_kind`](#cc_engine-commands-mep-parse_kind) · [`CABLED`](#cc_engine-commands-mep-cabled) · [`check_fits`](#cc_engine-commands-mep-check_fits) · [`mep_classes`](#cc_engine-commands-mep-mep_classes) · [`segment`](#cc_engine-commands-mep-segment) · [`joined_offset`](#cc_engine-commands-mep-joined_offset) · [`PipeCreate`](#cc_engine-commands-mep-pipecreate) · [`ConduitCreate`](#cc_engine-commands-mep-conduitcreate) · [`SystemsList`](#cc_engine-commands-mep-systemslist) · [`cabling_totals`](#cc_engine-commands-mep-cabling_totals) · [`SystemCreate`](#cc_engine-commands-mep-systemcreate) · [`new_system`](#cc_engine-commands-mep-new_system) · [`SystemSet`](#cc_engine-commands-mep-systemset) · [`SystemDelete`](#cc_engine-commands-mep-systemdelete) · [`SystemAssign`](#cc_engine-commands-mep-systemassign) · [`CircuitLoads`](#cc_engine-commands-mep-circuitloads) · [`MepFittings`](#cc_engine-commands-mep-mepfittings) · [`MepConnect`](#cc_engine-commands-mep-mepconnect) · [`nearest_on`](#cc_engine-commands-mep-nearest_on) · [`MepSizing`](#cc_engine-commands-mep-mepsizing) · [`MepSizeApply`](#cc_engine-commands-mep-mepsizeapply)

<a id="cc_engine-commands-mep-parse_kind"></a>

### fn `parse_kind`

```rust
pub(crate) fn parse_kind(name: &str) -> Result<SystemKind>
```

<a id="cc_engine-commands-mep-cabled"></a>

### const `CABLED`

```rust
pub(crate) const CABLED: &[SystemClass]
```

Runs of circuits and structured cabling: conduits and cable trays.

<a id="cc_engine-commands-mep-check_fits"></a>

### fn `check_fits`

```rust
pub(crate) fn check_fits(p: &Project, classes: &[SystemClass], system: SystemId) -> Result<()>
```

Whether `system` is of one of the families `classes` (any, when empty).

<a id="cc_engine-commands-mep-mep_classes"></a>

### fn `mep_classes`

```rust
pub(crate) fn mep_classes(mep: Option<MepData>) -> &'static [SystemClass]
```

The families of systems a component of this MEP data belongs to:
plumbing for fixtures, the device's own family for devices, any without.

<a id="cc_engine-commands-mep-segment"></a>

### fn `segment`

```rust
pub(crate) fn segment(start: Point, end: Option<Point>, offset: f64, end_offset: Option<f64>) -> Result<MepSegment>
```

Where a segment ends: `end`, or straight up or down from `start` (a riser).

<a id="cc_engine-commands-mep-joined_offset"></a>

### fn `joined_offset`

```rust
pub(crate) fn joined_offset(p: &Project, level: cc_core::LevelId, start: Point, category: Category, system: Option<SystemId>, wanted: f64) -> Option<f64>
```

The elevation above `level` where a new run starting at `start` joins a
run of its network there (runs of `category`: pipes of `system`, any
conduit or cable tray; risers of other floors included): the run's
elevation at that point, or on a riser the height nearest `wanted` within it.

<a id="cc_engine-commands-mep-pipecreate"></a>

### struct `PipeCreate`

```rust
pub struct PipeCreate
```

Create a pipe from `start` to `end` (or a vertical riser at `start` up or
down to `end_offset`), in a plumbing system. Its center line starts
`offset` above the level (default: where it starts on a pipe of its
system, such as the end of a run or a riser; else 0.60 m for supply and
vent pipes, -0.30 m for drains; 2'-0" and -1'-0" in imperial projects).
Drains fall along the flow by `slope` (percent; default 2 % up to 75 mm,
1 % above; in imperial projects 1/4" per foot up to 3", 1/8" above);
other pipes are level unless `end_offset` or `slope` says otherwise.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the center line in plan.

- `end: Option<Point>`

  End in plan (default: the start, for a riser).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Pipe type id or name (default: the first pipe type).

- `system: Option<Ref>`

  Plumbing system id, name or code (e.g. "AF").

- `offset: Option<f64>`

  Elevation of the start above the level, meters.

- `end_offset: Option<f64>`

  Elevation of the end above the level, meters (overrides `slope`).

- `slope: Option<f64>`

  Fall from the start to the end, percent of the plan length.


<a id="cc_engine-commands-mep-conduitcreate"></a>

### struct `ConduitCreate`

```rust
pub struct ConduitCreate
```

Create a conduit from `start` to `end` (or a vertical drop at `start` to
`end_offset`) carrying `circuits`. It runs `offset` above the level
(default: where it starts on another conduit, such as the end of a run or
a vertical drop; else in the slab above, at the level height).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the center line in plan.

- `end: Option<Point>`

  End in plan (default: the start, for a vertical drop).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Conduit type id or name (default: the first conduit type).

- `circuits: Vec<Ref>`

  Circuits it carries: ids, names or codes.

- `offset: Option<f64>`

  Elevation of the start above the level, meters.

- `end_offset: Option<f64>`

  Elevation of the end above the level, meters (default: as the start).


<a id="cc_engine-commands-mep-systemslist"></a>

### struct `SystemsList`

```rust
pub struct SystemsList
```

List the plumbing systems and electrical circuits with what is in them:
element count, pipe or conduit length, fixture units and supply weights
of plumbing fixtures, and the devices and load of circuits.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-mep-cabling_totals"></a>

### fn `cabling_totals`

```rust
pub(crate) fn cabling_totals(p: &Project, id: SystemId) -> serde_json::Map<String, Value>
```

What the structured cabling of a rack adds up to: its data outlets, the
cables (one per jack) and their estimated length.

<a id="cc_engine-commands-mep-systemcreate"></a>

### struct `SystemCreate`

```rust
pub struct SystemCreate
```

Create a plumbing system, an electrical circuit, the structured cabling
of a rack or a lightning protection system. `kind` is `cold_water`,
`hot_water`, `sanitary`, `vent`, `rainwater`, `power`, `lighting`, `data`
or `lightning`; circuits may name the board (`panel`, a component element;
default: the project's only board, if it has one) feeding them and their
voltage (default 127 V), and cabling the rack (`panel`; default: the only
rack) its outlets are cabled to. The code of a circuit or of cabling
defaults to the next free number on its board or rack.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `kind: String`

  What it carries.

- `name: Option<String>`

  Name (default: the code, or the kind's name).

- `code: Option<String>`

  Short code shown in tags (e.g. "AF", a circuit number).

- `color: Option<[u8; 3]>`

  Display color `[r, g, b]` (default: by kind).

- `panel: Option<u64>`

  Circuits: the distribution board feeding it (element id; default: the only board).

- `voltage: Option<f64>`

  Circuits: nominal voltage, volts.


<a id="cc_engine-commands-mep-new_system"></a>

### fn `new_system`

```rust
pub(crate) fn new_system(p: &Project, kind: SystemKind, panel: Option<ElementId>, code: Option<String>, name: Option<String>) -> Result<MepSystem>
```

A new system of `kind`: fed by `panel` (checked; default: the project's
only board or rack for circuits and cabling), with `code` (default: the
next free number on its board or rack) and `name` (default: after the
code or the kind).

<a id="cc_engine-commands-mep-systemset"></a>

### struct `SystemSet`

```rust
pub struct SystemSet
```

Change a system or circuit: `params` is a merge patch over the values
`systems_list` shows (`name`, `code`, `kind`, `color`, `panel`, `voltage`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `system: Ref`

  System id, name or code.

- `params: Value`

  Values to change.


<a id="cc_engine-commands-mep-systemdelete"></a>

### struct `SystemDelete`

```rust
pub struct SystemDelete
```

Delete a system or circuit; its pipes, conduits and devices stay, outside any system.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `system: Ref`

  System id, name or code.


<a id="cc_engine-commands-mep-systemassign"></a>

### struct `SystemAssign`

```rust
pub struct SystemAssign
```

Put elements in a system or circuit, or take them out (`remove`). A pipe
or a lightning conductor belongs to one system (it moves); a conduit or
cable tray carries several circuits and cabling systems (one is added); a
fixture or device connects to one (a data outlet to the cabling of a rack).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `elements: Vec<u64>`

  Pipe, conduit and component ids.

- `system: Ref`

  System id, name or code.

- `remove: bool`

  Take them out instead.


<a id="cc_engine-commands-mep-circuitloads"></a>

### struct `CircuitLoads`

```rust
pub struct CircuitLoads
```

The load schedule of the circuits (of one board, or all): per circuit its
devices, installed load (VA) and current at its voltage, with the total
per board.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `panel: Option<u64>`

  Distribution board element id (default: every circuit).


<a id="cc_engine-commands-mep-mepfittings"></a>

### struct `MepFittings`

```rust
pub struct MepFittings
```

The fittings where pipes and conduits meet, counted by category, kind,
angle (elbows), diameters and system: the fittings takeoff.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `level: Option<Ref>`

  Level id or name (default: every level).


<a id="cc_engine-commands-mep-mepconnect"></a>

### struct `MepConnect`

```rust
pub struct MepConnect
```

Connect plumbing fixtures and electrical devices to the nearest pipe or
conduit of their level (or riser of another floor passing through it). A fixture gets a branch to the nearest sanitary
drain (falling towards it, with a drop from the floor), and to the
nearest cold (and, when it takes it, hot) water pipe; a device gets a
conduit to the nearest conduit, with a drop from its mounting height,
carrying its circuit. A component with connectors gets a branch from
each connector to the nearest run of a system of the connector's kind,
from the connector's position and of its diameter (leaving vertically or
horizontally as the connector points). Branches end on the runs, which
forms tees. `system` connects only to that system or circuit.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `elements: Vec<u64>`

  Fixture and device ids.

- `system: Option<Ref>`

  Only to this system or circuit (id, name or code).


<a id="cc_engine-commands-mep-nearest_on"></a>

### fn `nearest_on`

```rust
pub(crate) fn nearest_on(s: &MepSegment, p: glam::DVec2, wanted_z: f64) -> (glam::DVec2, f64)
```

The nearest point in plan of run `s` to `p`, and the run's elevation there
(on a riser, the height nearest `wanted_z`).

<a id="cc_engine-commands-mep-mepsizing"></a>

### struct `MepSizing`

```rust
pub struct MepSizing
```

Size the building services from their networks (simplified tables of
common Brazilian practice): drains by the fixture units upstream, water
pipes by the flow from the fixtures' weights (at most 3 m/s), circuits
by their current (breaker, copper conductor) and the voltage drop to the
farthest device (at most 4 %). Diameters in mm, lengths in m.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-mep-mepsizeapply"></a>

### struct `MepSizeApply`

```rust
pub struct MepSizeApply
```

Give each sized pipe the smallest pipe type of the project at least as
wide as `mep_sizing` requires. Pipes already wide enough keep their type
unless `downsize` is set.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `downsize: bool`

  Also narrow pipes wider than required.


<a id="mod-cc_engine-commands-modify"></a>

## `cc_engine::commands::modify`

<sub>`crates/cc_engine/src/commands/modify.rs`</sub>

Modifying elements: delete, move, copy/array, rotate, mirror, pin, flip,
joins, trim/extend, split, offset, align, end points.

**Items:** [`ElementsDelete`](#cc_engine-commands-modify-elementsdelete) · [`ElementsMove`](#cc_engine-commands-modify-elementsmove) · [`ElementsCopy`](#cc_engine-commands-modify-elementscopy) · [`ElementsRotate`](#cc_engine-commands-modify-elementsrotate) · [`ElementsMirror`](#cc_engine-commands-modify-elementsmirror) · [`ElementsPin`](#cc_engine-commands-modify-elementspin) · [`ElementFlip`](#cc_engine-commands-modify-elementflip) · [`WallJoin`](#cc_engine-commands-modify-walljoin) · [`GeometryJoin`](#cc_engine-commands-modify-geometryjoin) · [`GeometryUnjoin`](#cc_engine-commands-modify-geometryunjoin) · [`GroupCreate`](#cc_engine-commands-modify-groupcreate) · [`GroupUngroup`](#cc_engine-commands-modify-groupungroup) · [`TrimCorner`](#cc_engine-commands-modify-trimcorner) · [`TrimExtend`](#cc_engine-commands-modify-trimextend) · [`ElementSplit`](#cc_engine-commands-modify-elementsplit) · [`ElementOffset`](#cc_engine-commands-modify-elementoffset) · [`ElementAlign`](#cc_engine-commands-modify-elementalign) · [`align_line`](#cc_engine-commands-modify-align_line) · [`ElementMoveEnd`](#cc_engine-commands-modify-elementmoveend) · [`GridsAlignEnds`](#cc_engine-commands-modify-gridsalignends) · [`ElementsTurn`](#cc_engine-commands-modify-elementsturn) · [`ElementsExport`](#cc_engine-commands-modify-elementsexport) · [`ElementsPaste`](#cc_engine-commands-modify-elementspaste) · [`retarget`](#cc_engine-commands-modify-retarget) · [`WallAttach`](#cc_engine-commands-modify-wallattach)

<a id="cc_engine-commands-modify-elementsdelete"></a>

### struct `ElementsDelete`

```rust
pub struct ElementsDelete
```

Delete elements (doors and windows of deleted walls go too, piles with their caps; pinned
elements stay, and so do the piles of a cap that is not deleted).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.


<a id="cc_engine-commands-modify-elementsmove"></a>

### struct `ElementsMove`

```rust
pub struct ElementsMove
```

Move elements by `delta` (walls take their doors and windows; doors and
windows alone slide along their wall).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `delta: Point`

  Displacement `[dx, dy]` in meters.


<a id="cc_engine-commands-modify-elementscopy"></a>

### struct `ElementsCopy`

```rust
pub struct ElementsCopy
```

Copy elements by `delta`; with `count` > 1, a linear array of that many
copies spaced by `delta`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `delta: Point`

  Displacement `[dx, dy]` of the first copy, meters (the next ones repeat it).

- `count: Option<u32>`

  Number of copies (default 1, at most 1000).


<a id="cc_engine-commands-modify-elementsrotate"></a>

### struct `ElementsRotate`

```rust
pub struct ElementsRotate
```

Rotate elements by `angle` degrees (counter-clockwise) about `center`
(default: the center of their extents); with `copy`, rotated copies.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `angle: f64`

  Angle in degrees, counter-clockwise.

- `center: Option<Point>`

  Center of rotation (default: the center of the elements' extents).

- `copy: bool`

  Rotate copies, keeping the originals.


<a id="cc_engine-commands-modify-elementsmirror"></a>

### struct `ElementsMirror`

```rust
pub struct ElementsMirror
```

Mirror elements across an axis: two points, or the line of a wall, grid,
beam or railing (`axis_element`). Copies by default (`copy: false` moves them).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `axis: Option<[Point; 2]>`

  Two points of the axis.

- `axis_element: Option<u64>`

  Id of an element whose line is the axis.

- `copy: Option<bool>`

  Keep the originals and mirror copies (default true).


<a id="cc_engine-commands-modify-elementspin"></a>

### struct `ElementsPin`

```rust
pub struct ElementsPin
```

Pin elements (they cannot be moved or deleted) or unpin them.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `pinned: bool`

  `true` to pin, `false` to unpin.


<a id="cc_engine-commands-modify-elementflip"></a>

### struct `ElementFlip`

```rust
pub struct ElementFlip
```

Flip a door or window to the other side of its wall (`facing`) and/or to the other jamb (`hand`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Id of the door or window.

- `facing: bool`

  Flip to the other side of the wall.

- `hand: bool`

  Flip the hinge to the other jamb.


<a id="cc_engine-commands-modify-walljoin"></a>

### struct `WallJoin`

```rust
pub struct WallJoin
```

Allow or disallow joining at one end of a wall or beam. A wall end joins
other walls; a beam end stops at the face of a structural column it lies
in, or meets other beams at the same depth (mitered corners, butt joints).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Wall or beam id.

- `end: String`

  `start` or `end`.

- `joined: bool`

  `true` to allow joining at that end, `false` to keep it free.


<a id="cc_engine-commands-modify-geometryjoin"></a>

### struct `GeometryJoin`

```rust
pub struct GeometryJoin
```

Join the geometry of two elements that touch or overlap (walls, floors,
ceilings, roofs, columns, beams, stairs, railings, components). Where they
overlap, one cuts the other: structural columns and beams cut walls,
floors, ceilings and roofs; floors, ceilings and roofs cut walls; walls
cut architectural columns; between equals the first cuts the second. The
shared volume is counted once, and plans and sections draw two elements
cut in the same material as one piece.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `first: u64`

  The first element.

- `second: u64`

  The second element.


<a id="cc_engine-commands-modify-geometryunjoin"></a>

### struct `GeometryUnjoin`

```rust
pub struct GeometryUnjoin
```

Unjoin the geometry of two joined elements: each keeps its whole body again.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `first: u64`

  The first element.

- `second: u64`

  The second element.


<a id="cc_engine-commands-modify-groupcreate"></a>

### struct `GroupCreate`

```rust
pub struct GroupCreate
```

Group elements: selecting one of them selects them all, so they move,
copy and rotate together. Elements already in a group leave it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  The elements (at least two).

- `name: Option<String>`

  Group name (default: "Group N").


<a id="cc_engine-commands-modify-groupungroup"></a>

### struct `GroupUngroup`

```rust
pub struct GroupUngroup
```

Ungroup: the group goes, its elements stay.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `group: u64`

  Group id (see `groups_list`, or `group` in `element_get`).


<a id="cc_engine-commands-modify-trimcorner"></a>

### struct `TrimCorner`

```rust
pub struct TrimCorner
```

Trim or extend two linear elements (walls, beams, grids) to meet at a
corner; each pick point marks the part of its element to keep. Arc walls
meet lines and other arcs where their circles cross them. Two detail
lines meet where their lines, circles or ellipses cross.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `a: u64`

  First element id.

- `pick_a: Point`

  A point on the part of `a` to keep.

- `b: u64`

  Second element id.

- `pick_b: Point`

  A point on the part of `b` to keep.


<a id="cc_engine-commands-modify-trimextend"></a>

### struct `TrimExtend`

```rust
pub struct TrimExtend
```

Trim or extend a linear element to the line of a reference element (the
circle of an arc wall); the pick point marks the part to keep. Arc walls
trim and extend along their circle. A detail line trims or extends to
another detail line (also a circle or an ellipse) or to the line of a
wall, beam or grid.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element to trim or extend.

- `pick: Point`

  A point on the part to keep.

- `reference: u64`

  Id of the element whose line is the boundary.


<a id="cc_engine-commands-modify-elementsplit"></a>

### struct `ElementSplit`

```rust
pub struct ElementSplit
```

Split a linear element (wall, beam, grid) or a detail line at a point;
returns the new element. A detail circle or whole ellipse splits into
halves, from the point and from the one opposite.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.

- `point: Point`

  Where to split (projected on the element's line).


<a id="cc_engine-commands-modify-elementoffset"></a>

### struct `ElementOffset`

```rust
pub struct ElementOffset
```

Copy a linear element parallel at `distance` (positive: to the left of
its direction). Detail lines too: an arc's or a circle's copy is
concentric (positive: towards the center of a circle); ellipses have no
offset.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.

- `distance: f64`

  Offset distance, meters: positive to the left of start→end, negative to the right.


<a id="cc_engine-commands-modify-elementalign"></a>

### struct `ElementAlign`

```rust
pub struct ElementAlign
```

Align an element onto a line of a reference element, in plan: the edge
picked on the element (a wall's face, a column's side) moves onto the
line picked on the reference (a face or side, or the location line of a
grid or of an element picked without a point). When the two are not
parallel the element first turns about the picked point of its edge. An
arc wall moves onto the circle of a concentric arc wall (its radius
changes). Alignment is in plan, so the elements may be on different
levels (a column above onto the face of the column below).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element to move.

- `reference: u64`

  Element whose line it moves onto.

- `pick: Option<Point>`

  Plan point `[x, y]` near the edge of `id` that moves onto the reference line (the edge of
  its plan outline nearest to it). Without it a linear element moves its location line, and
  other elements their insertion point, onto the line.

- `reference_pick: Option<Point>`

  Plan point `[x, y]` near the line of `reference` to align to (the edge of its plan outline
  nearest to it, such as a wall face). Without it the reference's location line is used,
  which a column or component does not have.


<a id="cc_engine-commands-modify-align_line"></a>

### fn `align_line`

```rust
pub fn align_line(project: &cc_core::Project, geom: &cc_geom::ModelGeometry, id: ElementId, pick: Option<DVec2>) -> Option<cc_core::wall_ops::Path>
```

The line of element `id` that Align uses: the edge of its plan outline
nearest to `pick` (a wall face, a column side), else its location line
(grids have only that). An arc wall gives its arc.

<a id="cc_engine-commands-modify-elementmoveend"></a>

### struct `ElementMoveEnd`

```rust
pub struct ElementMoveEnd
```

Move one end of a wall, beam or grid (joined wall ends follow).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.

- `end: String`

  `start` or `end`.

- `point: Point`

  New position of that end.


<a id="cc_engine-commands-modify-gridsalignends"></a>

### struct `GridsAlignEnds`

```rust
pub struct GridsAlignEnds
```

Give grids the extents of a reference grid, so their bubbles line up:
each grid parallel to the reference (within 1°) has its ends moved along
its own line onto the lines through the reference's ends perpendicular
to it (each end to the reference end on its side). Grids that are not
parallel and pinned grids are left alone.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `reference: u64`

  Id of the reference grid.

- `grids: Vec<u64>`

  Ids of the grids to align (default: every other grid parallel to the reference).

- `ends: Option<String>`

  Which ends move: `both` (default), `start` (the bubble end) or `end`.


<a id="cc_engine-commands-modify-elementsturn"></a>

### struct `ElementsTurn`

```rust
pub struct ElementsTurn
```

Turn elements a quarter: components and columns rotate 90° about their
insertion point; doors and windows flip to the other side of their wall
(the Space key on a selection). Others and pinned elements are left alone.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.


<a id="cc_engine-commands-modify-elementsexport"></a>

### struct `ElementsExport`

```rust
pub struct ElementsExport
```

The elements as a portable set for `elements_paste`: doors and windows of
the given walls come along. The set keeps type, level and view ids, so it
pastes into this project or one sharing them.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.


<a id="cc_engine-commands-modify-elementspaste"></a>

### struct `ElementsPaste`

```rust
pub struct ElementsPaste
```

Paste a set from `elements_export` moved by `delta`, on `level` when
given (else on their own levels). Elements whose type, level, view or
wall does not exist here are skipped. Returns the new ids.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `elements: Value`

  The `elements` of an `elements_export` result.

- `delta: Option<Point>`

  Displacement `[dx, dy]`, meters (default: none).

- `level: Option<crate::refs::Ref>`

  Level id or name to paste on (default: each element's own level).

- `view: Option<crate::refs::Ref>`

  View id or name to paste the elements drawn in a view into (detail
  lines, filled and masking regions, text notes, dimensions, level
  marks, detail components and imported drawings), at the same drawing
  coordinates and on its level (default: each one's own view). Every
  such element must be one the view takes. Elements of the model ignore it.


<a id="cc_engine-commands-modify-retarget"></a>

### fn `retarget`

```rust
pub(crate) fn retarget(p: &Project, set: &mut [cc_core::Element], view: cc_core::ViewId) -> Result<()>
```

Move the view-owned elements of a copied set (see
`cc_core::Element::owner_view`) into `view`, onto its level (or the
lowest). Refused unless the view takes every one of them, as the
commands that create them would. A dimension moved between views that
do not share a drawing frame (two plans) stops measuring elements and
stays where it is drawn.

<a id="cc_engine-commands-modify-wallattach"></a>

### struct `WallAttach`

```rust
pub struct WallAttach
```

Attach the tops of walls to a roof, ceiling or floor (`top`), or take
them off it (`detach`). Under the element a wall rises or drops to its
underside, following its slope (a gable wall ends in a triangle);
elsewhere it keeps its own top. The walls follow the element when it
changes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `ids: Vec<u64>`

  Wall ids.

- `top: Option<u64>`

  The roof, ceiling or floor to attach the tops to.

- `detach: bool`

  Take the tops off what they are attached to.


<a id="mod-cc_engine-commands-niche"></a>

## `cc_engine::commands::niche`

<sub>`crates/cc_engine/src/commands/niche.rs`</sub>

Niches (issue #304): recesses in one face of a wall, hosted like doors
and windows. A niche type is a component of `ComponentKind::Niche` whose
opening has a depth (`cc_core::openings::niche_def`); the wall's geometry
recesses its face there (`cc_geom::Recess`).

**Items:** [`NicheTypeCreate`](#cc_engine-commands-niche-nichetypecreate) · [`NicheCreate`](#cc_engine-commands-niche-nichecreate)

<a id="cc_engine-commands-niche-nichetypecreate"></a>

### struct `NicheTypeCreate`

```rust
pub struct NicheTypeCreate
```

Create a niche type: a recess `width` wide, `height` tall and `depth`
deep in one face of a wall, its bottom `sill` above the wall's base by
default. In a wall thinner than the depth plus 1 cm the niche stops 1 cm
short of the other face (an opening through a wall is a door or a
window). Place niches with `niche_create`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name, unique among niche types (default: the component's name and its width, height and depth,
  in centimeters; in feet and inches in imperial projects).

- `width: Option<f64>`

  Width along the wall, meters (default 0.6; 2'-0" in imperial projects).

- `height: Option<f64>`

  Height, meters (default 0.6; 2'-0" in imperial projects).

- `depth: Option<f64>`

  How far it goes into the wall from its face, meters (default 0.1; 4" in imperial projects).

- `sill: Option<f64>`

  Height of its bottom above the wall's base in new niches, meters (default 0.9; 3'-0" in imperial
  projects; 0 for a niche from the floor).


<a id="cc_engine-commands-niche-nichecreate"></a>

### struct `NicheCreate`

```rust
pub struct NicheCreate
```

Place a niche in a wall: a recess in one of its faces, centered on the
wall unless `offset` or `point` is given. It is in the wall's left face
(seen from its start to its end), in the right one with `flip_facing`,
and with `point` in the face on the point's side. A project with no niche
type gets a default one (see `niche_type_create`) in the same undo step.
The wall keeps standing behind the niche: it is not split, and rooms,
joins and framing ignore it. A niche that would overlap a door, a window
or another niche of its wall cuts nothing; `element_get` then shows no
`volume`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `wall: Option<u64>`

  Id of the host wall; omit it to use the wall nearest `point`.

- `offset: Option<f64>`

  Distance along the wall from its start to the niche's center, meters.

- `point: Option<crate::refs::Point>`

  A point near the wall: the niche is centered at its projection, in the face on its side.

- `sill: Option<f64>`

  Height of its bottom above the wall's base, meters (default: the type's).

- `type_ref: Option<Ref>`

  Niche type id or name (default: the first niche type; one is made when the project has none).

- `flip_facing: Option<bool>`

  In the wall's right face instead of its left one.


<a id="mod-cc_engine-commands-pad"></a>

## `cc_engine::commands::pad`

<sub>`crates/cc_engine/src/commands/pad.rs`</sub>

Building pads: flat areas graded into a terrain, with cut and fill slopes
out to the existing ground (see `cc_core::pad`).

**Items:** [`new_pad_type`](#cc_engine-commands-pad-new_pad_type) · [`default_host`](#cc_engine-commands-pad-default_host) · [`PadCreate`](#cc_engine-commands-pad-padcreate) · [`PadTypeCreate`](#cc_engine-commands-pad-padtypecreate) · [`derived`](#cc_engine-commands-pad-derived)

<a id="cc_engine-commands-pad-new_pad_type"></a>

### fn `new_pad_type`

```rust
pub fn new_pad_type(id: ElementTypeId) -> ElementType
```

A pad type for a project that has none: slopes of 1:1 in cut and 1:1.5 in fill.

<a id="cc_engine-commands-pad-default_host"></a>

### fn `default_host`

```rust
pub fn default_host(p: &Project, level: Option<cc_core::LevelId>, outline: &[DVec2]) -> Option<ElementId>
```

The terrain a pad with `outline` grades by default: the first (in id
order) on `level` (any level when `None`) whose boundary holds the
outline's center, else the first it overlaps.

<a id="cc_engine-commands-pad-padcreate"></a>

### struct `PadCreate`

```rust
pub struct PadCreate
```

Create a building pad: a flat area at `elevation` (meters above the
terrain's level) inside a closed `outline` (or one drawn with `curves`),
graded into a terrain (`host`) with cut and fill slopes out to the
existing ground. Slopes are the horizontal run per unit of rise
(`cut_slope` 1 is 1:1); without them the pad uses its type's. Pads of
one terrain apply in creation order. A project without a pad type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `host: Option<u64>`

  Terrain element id (default: the terrain on `level` whose boundary holds the outline's center).

- `outline: Vec<Point>`

  Outline `[[x, y], …]` in plan, closed (at least three points); or `curves`.

- `curves: Option<Vec<CurveArg>>`

  The outline drawn with curves (lines, arcs, circles, rectangles…).

- `level: Option<Ref>`

  Level id or name where the default terrain is looked for (default: any level).

- `elevation: f64`

  Elevation of the pad above the terrain's level, meters.

- `cut_slope: Option<f64>`

  Run per unit of rise of the cut slope (default: the type's).

- `fill_slope: Option<f64>`

  Run per unit of rise of the fill slope (default: the type's).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).


<a id="cc_engine-commands-pad-padtypecreate"></a>

### struct `PadTypeCreate`

```rust
pub struct PadTypeCreate
```

Create a building pad type: the run per unit of rise of its cut and fill slopes.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's pad name).

- `cut_slope: Option<f64>`

  Run per unit of rise of the cut slope (default 1).

- `fill_slope: Option<f64>`

  Run per unit of rise of the fill slope (default 1.5).


<a id="cc_engine-commands-pad-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(g: &cc_geom::pad::PadGeom, derived: &mut serde_json::Map<String, Value>)
```

The figures `element_get` shows for a pad, as `pad`.

<a id="mod-cc_engine-commands-paint"></a>

## `cc_engine::commands::paint`

<sub>`crates/cc_engine/src/commands/paint.rs`</sub>

Painted faces: a material on one surface of an element (see
`cc_core::paint` for how a paint finds its face after the model changes).

**Items:** [`PICK_TOLERANCE`](#cc_engine-commands-paint-pick_tolerance) · [`Paint`](#cc_engine-commands-paint-paint) · [`PaintRemove`](#cc_engine-commands-paint-paintremove) · [`PaintsList`](#cc_engine-commands-paint-paintslist)

<a id="cc_engine-commands-paint-pick_tolerance"></a>

### const `PICK_TOLERANCE`

```rust
pub const PICK_TOLERANCE: f64
```

How far from a face a point given to `paint` or `paint_remove` may be, meters.

<a id="cc_engine-commands-paint-paint"></a>

### struct `Paint`

```rust
pub struct Paint
```

Paint a face of an element with a material: the face at `point` (and
those touching it in its plane, or the rest of a curved surface). It shows
the material's colors, surface pattern and texture over the type's
material; painting a painted surface again replaces its paint. The paint
follows the face when the element changes (the face turned like it that
passes nearest its point).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: u64`

  Element id.

- `point: [f64; 3]`

  A point on the face, `[x, y, z]` meters (within 5 cm of it).

- `normal: Option<[f64; 3]>`

  The face's outward normal, to pick between faces meeting at the
  point (a 3D pick gives it); default: the nearest face.

- `material: Ref`

  Material name (or index in `materials_list`).


<a id="cc_engine-commands-paint-paintremove"></a>

### struct `PaintRemove`

```rust
pub struct PaintRemove
```

Take the paint off a face of an element (the one at `point`), or every
paint of the element when no point is given.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: u64`

  Element id.

- `point: Option<[f64; 3]>`

  A point on the painted face, `[x, y, z]` meters (default: every paint of the element).

- `normal: Option<[f64; 3]>`

  The face's outward normal (see `paint`).


<a id="cc_engine-commands-paint-paintslist"></a>

### struct `PaintsList`

```rust
pub struct PaintsList
```

The painted faces of an element (or of every element): each paint's
material, point and normal, and the faces and area it covers now (none
when no face is turned like it any more).

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `element: Option<u64>`

  Element id (default: every element with paints).


<a id="mod-cc_engine-commands-phases"></a>

## `cc_engine::commands::phases`

<sub>`crates/cc_engine/src/commands/phases.rs`</sub>

Construction phases: the project's phases, the phases of elements
(built, demolished), phase filters and the graphics of each status
(see `cc_core::phase`).

**Items:** [`element_phases_json`](#cc_engine-commands-phases-element_phases_json) · [`view_phase_json`](#cc_engine-commands-phases-view_phase_json) · [`PhasesList`](#cc_engine-commands-phases-phaseslist) · [`PhaseCreate`](#cc_engine-commands-phases-phasecreate) · [`PhaseSet`](#cc_engine-commands-phases-phaseset) · [`PhaseDelete`](#cc_engine-commands-phases-phasedelete) · [`ElementsPhaseSet`](#cc_engine-commands-phases-elementsphaseset) · [`ElementsDemolish`](#cc_engine-commands-phases-elementsdemolish) · [`PhaseCreationSet`](#cc_engine-commands-phases-phasecreationset) · [`display_name`](#cc_engine-commands-phases-display_name) · [`status_name`](#cc_engine-commands-phases-status_name) · [`phase_filter_json`](#cc_engine-commands-phases-phase_filter_json) · [`PhaseFiltersList`](#cc_engine-commands-phases-phasefilterslist) · [`PhaseFilterCreate`](#cc_engine-commands-phases-phasefiltercreate) · [`PhaseFilterSet`](#cc_engine-commands-phases-phasefilterset) · [`PhaseFilterDelete`](#cc_engine-commands-phases-phasefilterdelete) · [`PhaseGraphicsGet`](#cc_engine-commands-phases-phasegraphicsget) · [`PhaseGraphicsSet`](#cc_engine-commands-phases-phasegraphicsset)

<a id="cc_engine-commands-phases-element_phases_json"></a>

### fn `element_phases_json`

```rust
pub(crate) fn element_phases_json(p: &Project, e: &cc_core::Element) -> (Value, Value)
```

The phases of an element as `element_get` shows them.

<a id="cc_engine-commands-phases-view_phase_json"></a>

### fn `view_phase_json`

```rust
pub(crate) fn view_phase_json(p: &Project, v: &cc_core::View) -> (Value, Value)
```

A view's phase and phase filter as `view_get` shows them.

<a id="cc_engine-commands-phases-phaseslist"></a>

### struct `PhasesList`

```rust
pub struct PhasesList
```

The project's construction phases in time order, with how many
elements each builds and demolishes and how many views show it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-phases-phasecreate"></a>

### struct `PhaseCreate`

```rust
pub struct PhaseCreate
```

Add a construction phase, at the end (the latest) or before another.
Elements and views keep their phases.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique).

- `description: Option<String>`

  Description.

- `before: Option<Ref>`

  Insert before this phase (id or name; default: after the last).


<a id="cc_engine-commands-phases-phaseset"></a>

### struct `PhaseSet`

```rust
pub struct PhaseSet
```

Rename a phase, change its description, or move it to another place
in time (`index`, 0 = earliest). A move that would put an element's
demolition before its construction is refused.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `phase: Ref`

  Phase id or name.

- `name: Option<String>`

  New name.

- `description: Option<String>`

  New description.

- `index: Option<usize>`

  New position in time order (0 = earliest).


<a id="cc_engine-commands-phases-phasedelete"></a>

### struct `PhaseDelete`

```rust
pub struct PhaseDelete
```

Delete a phase. A phase that elements are built or demolished in, or
that views show, needs `merge_into`: they move to that phase (an
element then built and demolished in the same phase is temporary). The
last remaining phase cannot be deleted.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `phase: Ref`

  Phase id or name.

- `merge_into: Option<Ref>`

  Phase that takes over its elements and views.


<a id="cc_engine-commands-phases-elementsphaseset"></a>

### struct `ElementsPhaseSet`

```rust
pub struct ElementsPhaseSet
```

Set the phase elements are built in and/or demolished in (`null`: not
demolished). A demolition cannot come before the construction. Doors,
windows and rebar follow their hosts: they are never built before them
and are demolished with them (unless demolished earlier).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `created: Option<Ref>`

  Phase built in (id or name).

- `demolished: Option<Option<Ref>>`

  Phase demolished in (id or name), or `null` to keep it standing.


<a id="cc_engine-commands-phases-elementsdemolish"></a>

### struct `ElementsDemolish`

```rust
pub struct ElementsDemolish
```

Demolish elements in a phase: `phase` when given, else the phase of
`view`, else the phase new elements are built in (the last by default).
Their doors, windows and rebar go with them (unless demolished
earlier). Elements not built by then, or already demolished earlier,
are refused.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `ids: Vec<u64>`

  Element ids.

- `phase: Option<Ref>`

  Phase id or name.

- `view: Option<Ref>`

  Or the view whose phase to use.


<a id="cc_engine-commands-phases-phasecreationset"></a>

### struct `PhaseCreationSet`

```rust
pub struct PhaseCreationSet
```

Set the phase new elements are built in by this session (the active
view's phase in the application): display state, not saved, not undone.
`phase` (id or name), or the phase of `view`; neither resets it to the
last phase.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `phase: Option<Ref>`

  Phase id or name.

- `view: Option<Ref>`

  Or the view whose phase to use.


<a id="cc_engine-commands-phases-display_name"></a>

### fn `display_name`

```rust
pub fn display_name(d: PhaseDisplay) -> &'static str
```

<a id="cc_engine-commands-phases-status_name"></a>

### fn `status_name`

```rust
pub fn status_name(st: PhaseStatus) -> &'static str
```

<a id="cc_engine-commands-phases-phase_filter_json"></a>

### fn `phase_filter_json`

```rust
pub(crate) fn phase_filter_json(p: &Project, f: &PhaseFilter) -> Value
```

<a id="cc_engine-commands-phases-phasefilterslist"></a>

### struct `PhaseFiltersList`

```rust
pub struct PhaseFiltersList
```

The phase filters: for each status (`existing`, `demolished`, `new`,
`temporary`) whether views using the filter show it `by_category`,
`overridden` (with the status graphics, see `phase_graphics_get`) or
`not_displayed`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-phases-phasefiltercreate"></a>

### struct `PhaseFilterCreate`

```rust
pub struct PhaseFilterCreate
```

Create a phase filter; statuses not given show `by_category`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: String`

  Name (unique).

- `existing: Option<String>`

  Existing elements: `by_category`, `overridden` or `not_displayed`.

- `demolished: Option<String>`

  Demolished elements.

- `new: Option<String>`

  New elements.

- `temporary: Option<String>`

  Temporary elements.


<a id="cc_engine-commands-phases-phasefilterset"></a>

### struct `PhaseFilterSet`

```rust
pub struct PhaseFilterSet
```

Change a phase filter: its name or how it shows each status. Views
using it follow.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `filter: Ref`

  Phase filter id or name.

- `name: Option<String>`

  New name.

- `existing: Option<String>`

  Existing elements: `by_category`, `overridden` or `not_displayed`.

- `demolished: Option<String>`

  Demolished elements.

- `new: Option<String>`

  New elements.

- `temporary: Option<String>`

  Temporary elements.


<a id="cc_engine-commands-phases-phasefilterdelete"></a>

### struct `PhaseFilterDelete`

```rust
pub struct PhaseFilterDelete
```

Delete a phase filter; views and view templates using it are left
without one (showing every status by category).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `filter: Ref`

  Phase filter id or name.


<a id="cc_engine-commands-phases-phasegraphicsget"></a>

### struct `PhaseGraphicsGet`

```rust
pub struct PhaseGraphicsGet
```

The graphics of each phase status (`existing`, `demolished`, `new`,
`temporary`), used by phase filters that show a status `overridden`.
Overrides are shaped as in `view_graphics_get`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

<a id="cc_engine-commands-phases-phasegraphicsset"></a>

### struct `PhaseGraphicsSet`

```rust
pub struct PhaseGraphicsSet
```

Change the graphics of a phase status: `overrides` is merged into the
current ones (`null` resets an attribute; see `view_graphics_get`),
`reset` clears them first.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `status: String`

  `existing`, `demolished`, `new` or `temporary`.

- `overrides: Option<Value>`

  Overrides to merge.

- `reset: bool`

  Clear the status overrides before merging.


<a id="mod-cc_engine-commands-piles"></a>

## `cc_engine::commands::piles`

<sub>`crates/cc_engine/src/commands/piles.rs`</sub>

Piles and pile caps (see `cc_core::pile`): free piles, pile and pile cap
types, caps placed one by one or under columns (their piles made with
them), and tie beams between caps.

**Items:** [`toe_arg`](#cc_engine-commands-piles-toe_arg) · [`PileTypeCreate`](#cc_engine-commands-piles-piletypecreate) · [`PileCreate`](#cc_engine-commands-piles-pilecreate) · [`PileCapTypeCreate`](#cc_engine-commands-piles-pilecaptypecreate) · [`PileCapCreate`](#cc_engine-commands-piles-pilecapcreate) · [`CapRotation`](#cc_engine-commands-piles-caprotation) · [`PileCapsUnderColumns`](#cc_engine-commands-piles-pilecapsundercolumns) · [`TieBeamsCreate`](#cc_engine-commands-piles-tiebeamscreate) · [`derived`](#cc_engine-commands-piles-derived) · [`normalize_patch`](#cc_engine-commands-piles-normalize_patch) · [`check`](#cc_engine-commands-piles-check) · [`owned_piles`](#cc_engine-commands-piles-owned_piles)

<a id="cc_engine-commands-piles-toe_arg"></a>

### fn `toe_arg`

```rust
pub(crate) fn toe_arg(cut_off: f64, toe: Option<f64>, length: Option<f64>) -> Result<f64>
```

A pile's toe from a `toe` or a `length` below `cut_off` (default: the default length).

<a id="cc_engine-commands-piles-piletypecreate"></a>

### struct `PileTypeCreate`

```rust
pub struct PileTypeCreate
```

Create a pile type: round (`diameter`) or square (`side`), made as `kind` says.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: from the size, e.g. `Estaca Ø40`).

- `diameter: Option<f64>`

  Diameter of a round pile, meters (default 0.4).

- `side: Option<f64>`

  Side of a square pile, meters (instead of `diameter`).

- `kind: Option<String>`

  How the piles are made: `bored` (default), `cfa` (continuous flight auger), `micro`
  (micropiles and root piles), `precast` (precast concrete, driven), `steel` (driven), `other`.

- `material: Option<String>`

  Material name (default: the first foundation type's, the template's concrete).


<a id="cc_engine-commands-piles-pilecreate"></a>

### struct `PileCreate`

```rust
pub struct PileCreate
```

Create a free pile at `position`: its head (`cut_off`) above the level and
its toe given by `toe` or by a `length` below the head. A project without
a pile type gets one (round, 40 cm, in concrete).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Axis of the pile in plan.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Pile type id or name (default: the first pile type).

- `cut_off: Option<f64>`

  Head of the pile (its cut-off) above the level, meters (default 0).

- `toe: Option<f64>`

  Toe of the pile above the level, meters (negative: below it); or `length`.

- `length: Option<f64>`

  Length of the pile below its head, meters (default 12); or `toe`.

- `embedment: Option<f64>`

  How far the head enters a cap or slab above it, meters (default 0).

- `rotation: Option<f64>`

  Rotation of a square pile in plan, degrees (default 0).


<a id="cc_engine-commands-piles-pilecaptypecreate"></a>

### struct `PileCapTypeCreate`

```rust
pub struct PileCapTypeCreate
```

Create a pile cap type: the layout of its piles (`single`, `pair`,
`triangle`, `square`, `line` of `count`, `circle` of `count` on a
`radius`, a ring cap with `ring_width`), their spacing and edge distance,
the cap's thickness, and the type, length and embedment of its piles. The
cap's outline follows: a rectangle around one, two, four and a line of
piles; for three, a triangle with its corners cut square; on a circle, a
round cap or a ring. A project without a pile type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: from the number of piles, e.g. `Bloco 3 estacas`).

- `layout: Option<String>`

  `single`, `pair` (default), `triangle`, `square`, `line` or `circle`.

- `count: Option<u32>`

  Number of piles of a `line` (2 to 64) or a `circle` (3 to 64).

- `radius: Option<f64>`

  Radius of a `circle` of piles (their axes), meters.

- `ring_width: Option<f64>`

  Width of a ring cap centered on the `circle` of piles, meters (default: a round cap).

- `spacing: Option<f64>`

  Distance between neighboring piles' axes, meters (default: three pile sizes).

- `edge_distance: Option<f64>`

  Distance from a pile's axis to the cap's edge, meters (default: one pile size).

- `thickness: Option<f64>`

  Thickness of the cap, meters (default 0.8).

- `pile_type: Option<Ref>`

  The piles' type id or name (default: the first pile type).

- `pile_length: Option<f64>`

  Length of each pile below its head, meters (default 12).

- `embedment: Option<f64>`

  How far each pile's head enters the cap, meters (default 0.1).

- `material: Option<String>`

  Material name (default: the first foundation type's, the template's concrete).


<a id="cc_engine-commands-piles-pilecapcreate"></a>

### struct `PileCapCreate`

```rust
pub struct PileCapCreate
```

Place a pile cap centered at `position`, turned by `rotation` (its
layout's `u` axis along it), its top `top_offset` above the level. Its
piles are made with it, in the same undo step, and follow it: they move,
turn and are copied and deleted with it, and change with its type. A
project without a pile cap type gets one (two piles).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `position: Point`

  Center of the cap (the origin of its layout) in plan.

- `rotation: Option<f64>`

  Rotation in plan, degrees (default 0).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Pile cap type id or name (default: the first pile cap type).

- `top_offset: Option<f64>`

  Top of the cap above the level, meters (default 0).


<a id="cc_engine-commands-piles-caprotation"></a>

### enum `CapRotation`

```rust
pub enum CapRotation
```

How caps under columns are turned: as their column, by a fixed angle, or
pointing away from a center.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Angle(f64)`

  A fixed angle, degrees counter-clockwise.

- `Mode(String)`

  `column` (as each column turns) or `radial` (pointing away from `center`).


<a id="cc_engine-commands-piles-pilecapsundercolumns"></a>

### struct `PileCapsUnderColumns`

```rust
pub struct PileCapsUnderColumns
```

Place a pile cap under each column (default: every column of the level),
centered on it, its top at the column's base; its piles are made with it,
all in one undo step. `rotation` turns each cap: `column` (default) as its
column, an angle in degrees, or `radial`: its layout's `u` axis pointing
away from `center`, so caps under columns on a circle point outward (a
three-pile cap's first pile outside). Columns that already stand on a pad
footing or a pile cap are skipped.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `columns: Vec<u64>`

  Column ids (default: every column of `level`).

- `level: Option<Ref>`

  Level id or name, when no columns are given (default: the lowest level).

- `type_ref: Option<Ref>`

  Pile cap type id or name (default: the first pile cap type).

- `rotation: Option<CapRotation>`

  `column` (default), `radial`, or an angle in degrees.

- `center: Option<Point>`

  The center radial caps point away from (default: the middle of the columns).


<a id="cc_engine-commands-piles-tiebeamscreate"></a>

### struct `TieBeamsCreate`

```rust
pub struct TieBeamsCreate
```

Create tie beams between consecutive pile caps of `caps` (and from the
last back to the first when `closed`): straight, or arcs around `center`
(each between two caps at the same distance from it, the shorter way),
or of a signed `radius` (positive: counter-clockwise). Each beam runs
between the caps' centers and stops at their outlines where it frames
into their depth; its top is flush with the lower of its two caps' tops
unless `top_offset` is given. Tie beams are ordinary beams.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `caps: Vec<u64>`

  Pile cap ids, in order (at least two).

- `closed: bool`

  Also join the last cap to the first.

- `center: Option<Point>`

  Arc beams: the center of the circle the caps stand on.

- `radius: Option<f64>`

  Arc beams: the radius in meters, positive when the arcs turn counter-clockwise from cap to
  cap, negative clockwise (ignored with `center`).

- `type_ref: Option<Ref>`

  Beam type id or name (default: the first beam type).

- `level: Option<Ref>`

  Level id or name of the beams (default: the first cap's level).

- `top_offset: Option<f64>`

  Top of the beams above their level, meters (default: flush with the caps' tops).


<a id="cc_engine-commands-piles-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(p: &Project, e: &cc_core::Element, out: &mut serde_json::Map<String, Value>)
```

The derived values `element_get` shows for a pile or a pile cap.

<a id="cc_engine-commands-piles-normalize_patch"></a>

### fn `normalize_patch`

```rust
pub(crate) fn normalize_patch(e: &cc_core::Element, patch: &Value) -> Result<Value>
```

`element_set` params for a pile: `length` stands for the toe that far
below the head; a pile cap's piles follow their cap and take no params.

<a id="cc_engine-commands-piles-check"></a>

### fn `check`

```rust
pub(crate) fn check(pile: &Pile) -> Result<()>
```

Check a pile changed by `element_set`: a length within the limits.

<a id="cc_engine-commands-piles-owned_piles"></a>

### fn `owned_piles`

```rust
pub(crate) fn owned_piles(p: &Project, ids: &[ElementId]) -> Vec<ElementId>
```

Element ids of piles in `ids` that belong to a cap not in `ids` (they go with their cap only).

<a id="mod-cc_engine-commands-profiles"></a>

## `cc_engine::commands::profiles`

<sub>`crates/cc_engine/src/commands/profiles.rs`</sub>

Structural profiles (see `cc_core::structural_profile`): the named
cross-sections column and beam types are drawn and built with. A profile
is a standard parametric shape (`i`, `channel`, `rect_tube`, `rect`…) of
a material (`steel`, `cold_formed`, `timber`, `concrete`) or a sketch
whose points are expressions over its own parameters. Profiles are
project settings: each command changes them in one undo step, and the
types using a changed profile follow it (their width and height become
its bounding box).

**Items:** [`TypeSectionInfo`](#cc_engine-commands-profiles-typesectioninfo) · [`section`](#cc_engine-commands-profiles-section) · [`ProfilesList`](#cc_engine-commands-profiles-profileslist) · [`ProfileGet`](#cc_engine-commands-profiles-profileget) · [`SketchParamArg`](#cc_engine-commands-profiles-sketchparamarg) · [`ProfileCreate`](#cc_engine-commands-profiles-profilecreate) · [`ProfileSet`](#cc_engine-commands-profiles-profileset) · [`ProfileDuplicate`](#cc_engine-commands-profiles-profileduplicate) · [`ProfileDelete`](#cc_engine-commands-profiles-profiledelete) · [`ProfileCatalog`](#cc_engine-commands-profiles-profilecatalog) · [`ProfileImport`](#cc_engine-commands-profiles-profileimport) · [`ProfileTypeCreate`](#cc_engine-commands-profiles-profiletypecreate)

<a id="cc_engine-commands-profiles-typesectioninfo"></a>

### struct `TypeSectionInfo`

```rust
pub(crate) struct TypeSectionInfo
```

The section of a column or beam type as element and analytical queries show it.

**Fields**

- `profile: Option<String>`

  The profile's name; `None` for a type drawn from its own section.

- `properties: cc_core::section_props::SectionProperties`

  Properties with the density of the type's material.

- `has_density: bool`

  Whether the type's material has a usable density (else `linear_mass` is 0).


<a id="cc_engine-commands-profiles-section"></a>

### fn `section`

```rust
pub(crate) fn section(p: &Project, data: &TypeData) -> Option<TypeSectionInfo>
```

The section of a column or beam type: its profile's (turned and mirrored),
or its own legacy section's when it has none.

<a id="cc_engine-commands-profiles-profileslist"></a>

### struct `ProfilesList`

```rust
pub struct ProfilesList
```

The project's structural profiles, optionally of one material or shape:
id, name, material, shape, area (m²), mass per meter in steel (kg/m) and
the number of column and beam types using each.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `material: Option<String>`

  Only profiles of this material: `steel`, `cold_formed`, `timber` or `concrete`.

- `shape: Option<String>`

  Only profiles of this shape code (`i`, `channel`, `rect`, `sketch`…).


<a id="cc_engine-commands-profiles-profileget"></a>

### struct `ProfileGet`

```rust
pub struct ProfileGet
```

One structural profile: its material, shape and parameters (and the
sketch of a sketched one), the outline (outer loop counter-clockwise,
holes clockwise, meters, centered on its bounding box), its section
properties (SI units; mass per length with the usual density of its
material), its catalog entry and the names of the types using it.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `profile: Ref`

  Profile id or name.


<a id="cc_engine-commands-profiles-sketchparamarg"></a>

### struct `SketchParamArg`

```rust
pub struct SketchParamArg
```

A numeric parameter of a sketched profile.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `name: String`

  Name used in the points' expressions.

- `value: f64`

  Value, meters.


<a id="cc_engine-commands-profiles-profilecreate"></a>

### struct `ProfileCreate`

```rust
pub struct ProfileCreate
```

Create a structural profile: a standard shape of a material (its default
dimensions, changed by `params`), or a sketch drawn with `curves` (and
`openings`), whose points keep the drawn coordinates and which declares
`sketch_params`. Without a name it gets one from its shape and size.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Name (unique, ignoring case; default: from the shape and its size).

- `material: String`

  Material: `steel`, `cold_formed`, `timber` or `concrete`.

- `shape: Option<String>`

  Shape code: `i`, `i_asym`, `channel`, `t`, `angle`, `cold_u`,
  `cold_lipped_u`, `cold_z`, `cold_hat`, `rect_tube`, `round_tube`,
  `rect`, `round`, `concrete_t`, `concrete_l`, `annular_sector` (the piece of a ring between
  `r_in` and `r_out` turning by `sweep` degrees) or `sketch` (the default with `curves`;
  required without them).

- `params: Option<BTreeMap<String, f64>>`

  Parameter values by name, meters (degrees for `angle`), over the
  shape's defaults; see `profile_get` `params` for the names.

- `curves: Option<Vec<CurveArg>>`

  The closed outline of a sketched profile, in meters.

- `openings: Option<Vec<Vec<CurveArg>>>`

  The openings of a sketched profile: each a closed loop of curves inside the outline.

- `sketch_params: Option<Vec<SketchParamArg>>`

  Numeric parameters a sketched profile declares (lengths).


<a id="cc_engine-commands-profiles-profileset"></a>

### struct `ProfileSet`

```rust
pub struct ProfileSet
```

Change a structural profile: its name, material, parameter values or,
for a sketched one, its sketch (a merge patch over the `sketch` that
`profile_get` shows: `params`, `outline` and `openings`). A change that
leaves no valid outline is refused. The column and beam types using it
follow; a profile whose size or material changes is no longer its
catalog entry.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `profile: Ref`

  Profile id or name.

- `name: Option<String>`

  New name (unique, ignoring case).

- `material: Option<String>`

  New material: `steel`, `cold_formed`, `timber` or `concrete`.

- `params: Option<BTreeMap<String, f64>>`

  Parameter values by name, meters (degrees for `angle`).

- `sketch: Option<Value>`

  Merge patch over a sketched profile's sketch.


<a id="cc_engine-commands-profiles-profileduplicate"></a>

### struct `ProfileDuplicate`

```rust
pub struct ProfileDuplicate
```

Copy a structural profile under a new name (default: the name with a number).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `profile: Ref`

  Profile id or name.

- `name: Option<String>`

  Name of the copy (unique, ignoring case).


<a id="cc_engine-commands-profiles-profiledelete"></a>

### struct `ProfileDelete`

```rust
pub struct ProfileDelete
```

Delete a structural profile. Refused while column or beam types use it,
unless `replace_with` names the profile they take instead, in the same
undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `profile: Ref`

  Profile id or name.

- `replace_with: Option<Ref>`

  The profile (id or name) the types using it take instead.


<a id="cc_engine-commands-profiles-profilecatalog"></a>

### struct `ProfileCatalog`

```rust
pub struct ProfileCatalog
```

The built-in profile catalogs (standard steel, cold-formed, timber and
hollow sections). Without `series`: each series' `code`, `name`,
`material`, `shape` and row `count`. With it: the series' `code`,
`name` and `rows`, each a `designation`, the `name` an imported profile
gets and its `params` (meters; degrees for `angle`). Catalogs are not
part of the project; `profile_import` copies rows in.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `series: Option<String>`

  Series code (`W`, `HP`, `IPE`, `HEA`, `HEB`, `NBR6355-Ue`, `SHS`…; ignoring case).

- `search: Option<String>`

  Only rows whose designation contains this text (ignoring case).


<a id="cc_engine-commands-profiles-profileimport"></a>

### struct `ProfileImport`

```rust
pub struct ProfileImport
```

Copy rows of a built-in catalog series into the project as profiles that
remember their catalog entry, in one undo step (none when nothing is
imported). A row whose profile name is taken keeps the existing profile
(`on_clash: "keep"`, the default) or comes in under a free name
(`"rename"`). Returns the `imported` profiles (`id`, `name`), the names
`kept` and the designations the series does not have (`unknown`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `series: String`

  Series code (see `profile_catalog`).

- `designations: Vec<String>`

  Designations to import, as `profile_catalog` lists them (ignoring case).

- `on_clash: Option<String>`

  `keep` (default) or `rename`.


<a id="cc_engine-commands-profiles-profiletypecreate"></a>

### struct `ProfileTypeCreate`

```rust
pub struct ProfileTypeCreate
```

Make a column or beam type of a structural profile: a copy of the
category's first type given the profile, its width and height becoming
the profile's bounding box. Returns its `id` and `name`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `profile: Ref`

  Profile id or name.

- `category: String`

  `column` or `beam`.

- `name: Option<String>`

  Type name (unique in its category; default: the profile's name, made unique).

- `material: Option<String>`

  Material name (default: the first project material of the profile's
  class, metal, wood or concrete; else the template's concrete).


<a id="mod-cc_engine-commands-project"></a>

## `cc_engine::commands::project`

<sub>`crates/cc_engine/src/commands/project.rs`</sub>

The session's project: new, open, save, information, undo, units, language.

**Items:** [`ProjectNew`](#cc_engine-commands-project-projectnew) · [`ProjectOpen`](#cc_engine-commands-project-projectopen) · [`ProjectSave`](#cc_engine-commands-project-projectsave) · [`ProjectInfoGet`](#cc_engine-commands-project-projectinfoget) · [`ProjectInfoSet`](#cc_engine-commands-project-projectinfoset) · [`Undo`](#cc_engine-commands-project-undo) · [`Redo`](#cc_engine-commands-project-redo) · [`UnitsGet`](#cc_engine-commands-project-unitsget) · [`UnitsSet`](#cc_engine-commands-project-unitsset) · [`LanguageSet`](#cc_engine-commands-project-languageset)

<a id="cc_engine-commands-project-projectnew"></a>

### struct `ProjectNew`

```rust
pub struct ProjectNew
```

Start a new project from a template (unsaved changes are discarded).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `template: Option<String>`

  The template: `metric` (the default; meters, metric sizes) or
  `imperial` (feet and inches, round imperial sizes).


<a id="cc_engine-commands-project-projectopen"></a>

### struct `ProjectOpen`

```rust
pub struct ProjectOpen
```

Open a ConstructCraft project (.ccad), replacing the session's project.
Damaged files are repaired and the number of repairs reported.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: PathBuf`

  Path of the .ccad file.


<a id="cc_engine-commands-project-projectsave"></a>

### struct `ProjectSave`

```rust
pub struct ProjectSave
```

Save the project to its file, or to `path` (which becomes its file).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `path: Option<PathBuf>`

  Where to save; required when the project has no file yet.


<a id="cc_engine-commands-project-projectinfoget"></a>

### struct `ProjectInfoGet`

```rust
pub struct ProjectInfoGet
```

Project information, file, unsaved state and counts.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-project-projectinfoset"></a>

### struct `ProjectInfoSet`

```rust
pub struct ProjectInfoSet
```

Change the project information (title block fields).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: Option<String>`

  Project name.

- `number: Option<String>`

  Project number.

- `client: Option<String>`

  Client name.

- `address: Option<String>`

  Project address.

- `author: Option<String>`

  Author of the drawings.

- `fields: Option<serde_json::Map<String, Value>>`

  Project fields by name: a text sets the field (adding it; a new name holds no braces and at most 192 bytes), `null` removes it.


<a id="cc_engine-commands-project-undo"></a>

### struct `Undo`

```rust
pub struct Undo
```

Undo the last edit.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-project-redo"></a>

### struct `Redo`

```rust
pub struct Redo
```

Redo the last undone edit.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-project-unitsget"></a>

### struct `UnitsGet`

```rust
pub struct UnitsGet
```

The project units (length, area, volume, angle, slope, currency, number format).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-project-unitsset"></a>

### struct `UnitsSet`

```rust
pub struct UnitsSet
```

Change the project units: `values` is merged into the object `units_get`
returns (only the fields given change), after `preset` when given.
Length units are `Millimeter`, `Centimeter`, `Meter`, `Foot`, `Inch`,
`FeetInches` (`12'-6 1/2"`) and `FractionalInch`; for the last two,
`decimals` is the precision, 1/2^decimals of an inch.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `values: Value`

  Fields to change, shaped like the result of `units_get`.

- `preset: Option<String>`

  Start from `metric` (the defaults) or `imperial` (feet and inches to 1/16", square and cubic feet).


<a id="cc_engine-commands-project-languageset"></a>

### struct `LanguageSet`

```rust
pub struct LanguageSet
```

Language of default names, messages and sheet labels (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `lo`, `pl`, `uk-UA`, `ru`, `zh-TW`, `zh-CN` or `ja`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `language: String`

  Language code: `en`, `pt-BR`, `es`, `fr`, `de`, `it`, `lo`, `pl`, `uk-UA`, `ru`, `zh-TW`, `zh-CN` or `ja`.


<a id="mod-cc_engine-commands-query"></a>

## `cc_engine::commands::query`

<sub>`crates/cc_engine/src/commands/query.rs`</sub>

Reading the model: categories, levels, types, elements, views, materials, schedules.

**Items:** [`snake`](#cc_engine-commands-query-snake) · [`view_category`](#cc_engine-commands-query-view_category) · [`CategoriesList`](#cc_engine-commands-query-categorieslist) · [`LevelsList`](#cc_engine-commands-query-levelslist) · [`TypesList`](#cc_engine-commands-query-typeslist) · [`type_json`](#cc_engine-commands-query-type_json) · [`TypeGet`](#cc_engine-commands-query-typeget) · [`ElementsList`](#cc_engine-commands-query-elementslist) · [`element_json`](#cc_engine-commands-query-element_json) · [`ElementGet`](#cc_engine-commands-query-elementget) · [`view_json`](#cc_engine-commands-query-view_json) · [`ViewsList`](#cc_engine-commands-query-viewslist) · [`ViewGet`](#cc_engine-commands-query-viewget) · [`MaterialsList`](#cc_engine-commands-query-materialslist) · [`MaterialGet`](#cc_engine-commands-query-materialget) · [`ScheduleTable`](#cc_engine-commands-query-scheduletable) · [`GroupsList`](#cc_engine-commands-query-groupslist) · [`Measure`](#cc_engine-commands-query-measure)

<a id="cc_engine-commands-query-snake"></a>

### fn `snake`

```rust
pub(crate) fn snake(name: &str) -> String
```

`FloorPlan` → `floor_plan`.

<a id="cc_engine-commands-query-view_category"></a>

### fn `view_category`

```rust
pub(crate) fn view_category(name: &str) -> Result<ViewCategory>
```

<a id="cc_engine-commands-query-categorieslist"></a>

### struct `CategoriesList`

```rust
pub struct CategoriesList
```

Element categories with their display names and how many elements and types each has.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-query-levelslist"></a>

### struct `LevelsList`

```rust
pub struct LevelsList
```

Levels from the lowest, with their elevations (meters).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-query-typeslist"></a>

### struct `TypesList`

```rust
pub struct TypesList
```

Types, optionally of one category.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `category: Option<String>`

  Category name (`wall`, `door`, `component`…).


<a id="cc_engine-commands-query-type_json"></a>

### fn `type_json`

```rust
pub(crate) fn type_json(s: &Session, id: cc_core::ElementTypeId) -> Option<Value>
```

<a id="cc_engine-commands-query-typeget"></a>

### struct `TypeGet`

```rust
pub struct TypeGet
```

A type with all its parameters (`params`, editable with `type_set`) and identity data (`info`).
A title block's pictures show as `format`, `width`, `height` (pixels) and `bytes`, not their
bytes; `component_sheet_piece_set` replaces them.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  Type id or name.


<a id="cc_engine-commands-query-elementslist"></a>

### struct `ElementsList`

```rust
pub struct ElementsList
```

Elements, optionally of one category, level and/or type, or only those
shown in a view (all the instances of a type visible in it, for example),
with a short summary each.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `category: Option<String>`

  Category name (`wall`, `door`, `component`…); see `categories_list`.

- `level: Option<Ref>`

  Level id or name.

- `type_ref: Option<Ref>`

  Type id or name: only its instances.

- `view: Option<Ref>`

  View id or name: only the elements the view shows.


<a id="cc_engine-commands-query-element_json"></a>

### fn `element_json`

```rust
pub(crate) fn element_json(s: &mut Session, id: ElementId) -> Option<Value>
```

Full description of an element: everything `element_set` can change, plus derived values.

<a id="cc_engine-commands-query-elementget"></a>

### struct `ElementGet`

```rust
pub struct ElementGet
```

An element with all its parameters (`params`, editable with `element_set`) and derived values
(length, height, thickness, volume, room area and perimeter; for beams the length after joins
and what each end is framed into; for columns and beams the `section`: profile name, area,
mass per length with the material's density (null without one), `ix`, `iy`, `wx`, `wy`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.


<a id="cc_engine-commands-query-view_json"></a>

### fn `view_json`

```rust
pub(crate) fn view_json(s: &Session, id: ViewId) -> Option<Value>
```

<a id="cc_engine-commands-query-viewslist"></a>

### struct `ViewsList`

```rust
pub struct ViewsList
```

Views (plans, ceiling plans, elevations, sections, 3D, legends, drafting views, schedules, sheets),
optionally of one category.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `category: Option<String>`

  `floor_plan`, `ceiling_plan`, `three_d`, `elevation`, `section`, `legend`, `drafting`, `schedule`,
  `diagram` or `sheet`.


<a id="cc_engine-commands-query-viewget"></a>

### struct `ViewGet`

```rust
pub struct ViewGet
```

A view with all its parameters (`params`, editable with `view_set`), its
`scale` denominator and `scale_text`, the scale as the project writes it
(`1:50`, or `1/4" = 1'-0"` in imperial projects), and its `sun`: null,
or the settings `view_set` takes plus `derived` (azimuth and altitude,
`above_horizon`, the current frame's `date` and `time`, `sunrise` and
`sunset`, the number of `frames`, `site_known` and whether it is the
`conventional` sun), and its `shading` (`ambient` and `shadow`, null for
views that take no sun).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.


<a id="cc_engine-commands-query-materialslist"></a>

### struct `MaterialsList`

```rust
pub struct MaterialsList
```

Materials with their colors and how many types use each.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-query-materialget"></a>

### struct `MaterialGet`

```rust
pub struct MaterialGet
```

A material with all its fields (editable with `material_set`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `material: Ref`

  Material name (or index in `materials_list`).


<a id="cc_engine-commands-query-scheduletable"></a>

### struct `ScheduleTable`

```rust
pub struct ScheduleTable
```

The table of a schedule view: headers, one row per element (with its id) and totals,
formatted with the project units.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  Schedule view id or name.


<a id="cc_engine-commands-query-groupslist"></a>

### struct `GroupsList`

```rust
pub struct GroupsList
```

The project's groups with their members.

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-query-measure"></a>

### struct `Measure`

```rust
pub struct Measure
```

Measure between two points of a view (meters on its plane: plan
coordinates in plans): the distance, its horizontal and vertical parts
and its angle. When both points lie on lines (`line_a`, `line_b`, each two
points) that are parallel, the distance between the lines is given too.
Nothing is created.

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `a: [f64; 2]`

  First point `[x, y]`.

- `b: [f64; 2]`

  Second point `[x, y]`.

- `line_a: Option<[[f64; 2]; 2]>`

  The line the first point was taken on, as two points.

- `line_b: Option<[[f64; 2]; 2]>`

  The line the second point was taken on, as two points.


<a id="mod-cc_engine-commands-ramp"></a>

## `cc_engine::commands::ramp`

<sub>`crates/cc_engine/src/commands/ramp.rs`</sub>

Ramps: creating them in any shape and checking them against the
accessibility rules of their type.

**Items:** [`SHAPES`](#cc_engine-commands-ramp-shapes) · [`RampCreate`](#cc_engine-commands-ramp-rampcreate) · [`new_ramp_type`](#cc_engine-commands-ramp-new_ramp_type) · [`RampCheck`](#cc_engine-commands-ramp-rampcheck) · [`issue_text`](#cc_engine-commands-ramp-issue_text) · [`RampTypeCreate`](#cc_engine-commands-ramp-ramptypecreate)

<a id="cc_engine-commands-ramp-shapes"></a>

### const `SHAPES`

```rust
pub const SHAPES: [(&str, RampShape); 4]
```

Names of the ramp shapes in commands.

<a id="cc_engine-commands-ramp-rampcreate"></a>

### struct `RampCreate`

```rust
pub struct RampCreate
```

Create a ramp starting at `start` (the middle of its foot) and rising
along `direction` up to the level above (or `height`). Flights follow the
type's rules: at most its steepest slope and its highest rise per flight,
with landings between them; `shape` lays them out in line, L- or
U-shaped (switchback), or as a helix. A project without a ramp type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Foot of the ramp (middle of its lower edge).

- `direction: Option<f64>`

  Direction of the first flight in plan, degrees counter-clockwise from +X.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `height: Option<f64>`

  Height when there is no level above (default 3; 10'-0" in imperial projects).

- `top_level: Option<Ref>`

  Level the ramp rises to (default: the level above).

- `shape: Option<String>`

  Plan shape: `straight` (default), `l_shaped`, `u_shaped` or `spiral`.

- `turn: Option<String>`

  Side the ramp turns to, going up: `left` (default) or `right`.

- `slope: Option<f64>`

  Slope of the flights, percent (default: the steepest the type allows).

- `landing: Option<f64>`

  Landing length, meters (default: the type's shortest landing).

- `gap: Option<f64>`

  Clear gap between the flights of a U-shaped ramp, meters (default 0.1; 4" in imperial projects).

- `inner_radius: Option<f64>`

  Radius of the inner edge of a spiral ramp, meters (default 1.5; 5'-0" in imperial projects).


**Methods**

- `pub fn layout(&self, defaults: &cc_core::Defaults) -> Result<RampLayout>`

  The plan layout these parameters ask for.


<a id="cc_engine-commands-ramp-new_ramp_type"></a>

### fn `new_ramp_type`

```rust
pub fn new_ramp_type(p: &Project, id: ElementTypeId) -> ElementType
```

A ramp type for a project that has none: 1.20 m wide, a 15 cm slab of
the stairs' material, the default rules (4'-0", a 6" slab and the
customary rules in imperial projects; see `cc_core::Defaults::ramp`).

<a id="cc_engine-commands-ramp-rampcheck"></a>

### struct `RampCheck`

```rust
pub struct RampCheck
```

Check a ramp: its slope, flights (count, rise and run of each) and the
rules of its type it breaks (too steep, a flight rising too high, a
short landing).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Ramp element id.


<a id="cc_engine-commands-ramp-issue_text"></a>

### fn `issue_text`

```rust
pub fn issue_text(issue: &cc_core::RampIssue) -> String
```

A broken ramp rule in words.

<a id="cc_engine-commands-ramp-ramptypecreate"></a>

### struct `RampTypeCreate`

```rust
pub struct RampTypeCreate
```

Create a ramp type: a slab `thickness` thick and `width` wide, within the
accessibility rules given (by default 8.33 % slope, 0.80 m rise per flight
and 1.20 m landings).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's ramp name).

- `width: Option<f64>`

  Width, meters (default 1.2; 4'-0" in imperial projects).

- `thickness: Option<f64>`

  Slab thickness, meters (default 0.15; 6" in imperial projects).

- `max_slope: Option<f64>`

  Steepest slope, percent (default 8.33).

- `max_flight_rise: Option<f64>`

  Highest rise of one flight, meters (default 0.80; 30" in imperial projects).

- `min_landing: Option<f64>`

  Shortest landing, meters (default 1.20; 60" in imperial projects).


<a id="mod-cc_engine-commands-rebar"></a>

## `cc_engine::commands::rebar`

<sub>`crates/cc_engine/src/commands/rebar.rs`</sub>

Reinforcement: sets of bars in beams and columns, and copying a host's sets to others.

**Items:** [`bar_type`](#cc_engine-commands-rebar-bar_type) · [`partition_arg`](#cc_engine-commands-rebar-partition_arg) · [`splice`](#cc_engine-commands-rebar-splice) · [`RebarCreate`](#cc_engine-commands-rebar-rebarcreate) · [`RebarTypical`](#cc_engine-commands-rebar-rebartypical) · [`RebarCopy`](#cc_engine-commands-rebar-rebarcopy)

<a id="cc_engine-commands-rebar-bar_type"></a>

### fn `bar_type`

```rust
pub(crate) fn bar_type(p: &Project, diameter: f64) -> Result<ElementTypeId>
```

The bar type nearest `diameter`.

<a id="cc_engine-commands-rebar-partition_arg"></a>

### fn `partition_arg`

```rust
pub(crate) fn partition_arg(text: Option<&str>) -> Result<String>
```

The lap splices of bars of `type_id` running along a beam: `max_length`
(default: the stock length; 0 for whole bars) and `lap` (default: 40 bar
diameters).
A partition as it is kept: trimmed, at most `cc_core::bar_marks::MAX_MARK_TEXT` characters.

<a id="cc_engine-commands-rebar-splice"></a>

### fn `splice`

```rust
pub(crate) fn splice(p: &Project, type_id: ElementTypeId, max_length: Option<f64>, lap: Option<f64>) -> Result<Option<Splice>>
```

<a id="cc_engine-commands-rebar-rebarcreate"></a>

### struct `RebarCreate`

```rust
pub struct RebarCreate
```

Add a set of bars to a beam or column: `layout` is `top` or `bottom`
(beams), `perimeter` (columns) with `count` bars, `sides` (beams: `count`
side bars on each side face, or as many as keep them `spacing` apart over
the height), or `stirrups` every `spacing`. The bars are `cover` inside
the faces (default 2.5 cm) and follow the host: along an arc beam they
follow the arc, each at its own radius, and the stirrups are radial. Top
and bottom bars that do not fit side by side (a clear distance of the bar
diameter, at least 2 cm) go in a second layer. Bars along a beam longer
than `max_length` are made of pieces overlapping by `lap`, the splices of
neighbouring bars staggered.
A column's `perimeter` bars are `count` spread around its section, or
given face by face with `faces` (a bar count for each face of the
section, its two corner bars included, the corner bars shared between
faces; the column's `derived.faces` lists the faces in order) or
`face_spacing`; on a curved face they stand at even angles. Its
`stirrups` follow the section's outline, and `ties` adds cross-ties every
`spacing`: a hooked bar from each bar between the corners to the bar
opposite. With `max_length` a column's vertical bars are cut in lifts
that lap by `lap`. `same_type` gives the set to every column (or beam) of
the host's type.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `host: u64`

  Beam or column id.

- `layout: String`

  `top`, `bottom`, `perimeter`, `sides`, `stirrups` or `ties`.

- `count: Option<u32>`

  Number of longitudinal bars (default: 2 in beams, 4 in columns), or of side bars on each
  side face (default 1, unless `spacing` is given).

- `spacing: Option<f64>`

  Stirrup spacing, meters (default 0.15; 6" in imperial projects), or the largest distance
  between side bars over the height when no `count` is given.

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type nearest 10 mm, or 5 mm for stirrups; 1/2" and 3/8" in imperial
  projects).

- `cover: Option<f64>`

  Concrete cover, meters (default 0.025; 1 1/2" in imperial projects).

- `max_length: Option<f64>`

  Longest bar along a beam, meters (default: the stock length, 12 m, or 40' in imperial
  projects): longer bars are lap spliced. 0 keeps whole bars.

- `lap: Option<f64>`

  Lap length of the splices, meters (default: 40 bar diameters).

- `lap_at_joins: Option<bool>`

  Beams, bars along the beam: run them half a lap past each end joined to another
  beam that carries on in the same line (the beams of a ring), so the bars of the
  two lap there (default false: they stop the cover short of the end). The lap is `lap`,
  or 40 bar diameters where bars are kept whole (`max_length` 0).

- `overlapping: Option<bool>`

  Columns, `stirrups`: several closed stirrups that overlap, one around each of the
  rectangles the section is made of (two in an L or a T, three in a U), instead of one
  following the whole outline. The section must have only square corners.

- `partition: Option<String>`

  The part of the structure the sets belong to (e.g. "FOUNDATION", "WALL"), which
  schedules filter and group by (default: none).

- `lifts: Option<Vec<f64>>`

  Columns, `perimeter`: heights above the column's base where its vertical bars are
  cut, rising (the construction joints): the bars of each lift run `lap` past the next
  height. Instead of the equal lifts of `max_length`.

- `faces: Option<Vec<u32>>`

  Columns, `perimeter`: the number of bars on each face of the section, in the order of
  the column's `derived.faces`, the two corner bars of a face included (at least 2).

- `face_spacing: Option<f64>`

  Columns, `perimeter`: instead of `faces`, the largest distance between bars on every
  face, meters.

- `same_type: Option<bool>`

  Also give the set to every other column (or beam) of the host's type (default false).


<a id="cc_engine-commands-rebar-rebartypical"></a>

### struct `RebarTypical`

```rust
pub struct RebarTypical
```

Give beams and columns without reinforcement a typical one: beams two
(three when 20 cm wide or more) 10 mm bars at the bottom, two 8 mm at the
top (both lap spliced past the stock length) and 5 mm stirrups every 15 cm; columns four 10 mm bars (six in round
ones) and 5 mm ties every 15 cm. Default: every beam and column of
`level` (or of the project).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `elements: Vec<u64>`

  Beam and column ids (default: all of `level`, or of the project).

- `level: Option<Ref>`

  Level id or name, when no elements are given.


<a id="cc_engine-commands-rebar-rebarcopy"></a>

### struct `RebarCopy`

```rust
pub struct RebarCopy
```

Copy the reinforcement sets of one host (all, or those of `sets`) to other
hosts of the same kind and section in one step: columns and beams of the
same type or of types with the same section, walls and strip footings of
the same type. Each copy keeps its bar type, layout, cover, splices,
partition and hand-given mark. Floors and slab foundations, whose sets keep
plan points, are not copied. With `replace`, each target's own sets go
first. Nothing is changed when a target does not match. Returns the
created set ids and those replaced.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `from: u64`

  The host whose sets are copied: a column, beam, wall or strip footing.

- `to: Vec<u64>`

  The hosts that receive them.

- `sets: Vec<u64>`

  The sets of `from` to copy (default all).

- `replace: Option<bool>`

  Delete the targets' own sets first (default false: add beside them).


<a id="mod-cc_engine-commands-region"></a>

## `cc_engine::commands::region`

<sub>`crates/cc_engine/src/commands/region.rs`</sub>

Filled and masking regions: 2D areas drawn in one view (see
`cc_core::region`), sketched like floors (`curves`: an outline and
the loops inside it as openings). They are moved, copied, rotated,
mirrored and deleted with the generic commands; `sketch_get`/`sketch_set`
read and redraw the boundary; `element_set` changes the boundary style
(`"boundary": "<style>"` or `"none"`) and `element_set`'s `type` (or
`type_set` on the type) the fill.

**Items:** [`FilledRegionCreate`](#cc_engine-commands-region-filledregioncreate) · [`MaskingRegionCreate`](#cc_engine-commands-region-maskingregioncreate) · [`FilledRegionTypeCreate`](#cc_engine-commands-region-filledregiontypecreate) · [`normalize_patch`](#cc_engine-commands-region-normalize_patch) · [`check`](#cc_engine-commands-region-check) · [`redraw`](#cc_engine-commands-region-redraw)

<a id="cc_engine-commands-region-filledregioncreate"></a>

### struct `FilledRegionCreate`

```rust
pub struct FilledRegionCreate
```

Draw a filled region in a 2D view (plan, ceiling plan, elevation,
section, legend or drafting view): the closed loops of `curves` in the
view's drawing coordinates (meters), the largest the outline and those
inside it openings, filled as its type says (a pattern over a
background, or a solid color) and edged in a line style. It is drawn
only in that view, over the model and under the view's annotations. A
project without a filled region type gets one (diagonal lines over white).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `curves: Vec<CurveArg>`

  The curves, closed into loops, as for sketches (`line`, `arc`,
  `circle`, `ellipse`, `rectangle`, `polygon`).

- `type_ref: Option<Ref>`

  Filled region type id or name (default: the project's first).

- `boundary: Option<String>`

  Boundary line style (a built-in code or a project style's name;
  default `thin`), or `none` for no boundary.


<a id="cc_engine-commands-region-maskingregioncreate"></a>

### struct `MaskingRegionCreate`

```rust
pub struct MaskingRegionCreate
```

Draw a masking region in a 2D view: an opaque white area (the closed
loops of `curves`, openings inside the outline) that hides the model's
linework of the view under it, while the view's annotations (detail
lines, dimensions, texts, tags) stay on top. Drawn only in that view.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  View id or name.

- `curves: Vec<CurveArg>`

  The curves, closed into loops, as for sketches.

- `boundary: Option<String>`

  Boundary line style (a built-in code or a project style's name), or
  `none` (the default) for no boundary.


<a id="cc_engine-commands-region-filledregiontypecreate"></a>

### struct `FilledRegionTypeCreate`

```rust
pub struct FilledRegionTypeCreate
```

Create a filled region type: a foreground pattern over a background
pattern, each with its color (a `solid` pattern fills with its color; a
`none` background leaves the model visible between the pattern lines).
Change it later with `type_set` (`{"fill": {...}}`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's filled region name).

- `foreground: Option<String>`

  Foreground pattern (default `diagonal`): none, solid, diagonal,
  diagonal_back, crosshatch, horizontal, vertical, grid, brick,
  concrete, sand, insulation or earth.

- `foreground_color: Option<[u8; 3]>`

  Foreground color `[r, g, b]` (default black).

- `background: Option<String>`

  Background pattern (default `solid`), as for `foreground`.

- `background_color: Option<[u8; 3]>`

  Background color `[r, g, b]` (default white).


<a id="cc_engine-commands-region-normalize_patch"></a>

### fn `normalize_patch`

```rust
pub(crate) fn normalize_patch(p: &Project, patch: &Value) -> Result<Value>
```

A `params` patch for a region with its boundary given by code
(`"boundary": "wide"`, or `"none"`) turned into the model's form.

<a id="cc_engine-commands-region-check"></a>

### fn `check`

```rust
pub(crate) fn check(old: &Region, new: &Region) -> Result<()>
```

Check a patched region: the view stays, the outline is valid, the
openings lie inside it apart.

<a id="cc_engine-commands-region-redraw"></a>

### fn `redraw`

```rust
pub(crate) fn redraw(r: &mut Region, sketch: cc_core::sketch::Sketch) -> Result<()>
```

Redraw a region's outline and openings from a sketch (`sketch_set`).

<a id="mod-cc_engine-commands-render"></a>

## `cc_engine::commands::render`

<sub>`crates/cc_engine/src/commands/render.rs`</sub>

Photorealistic rendering of 3D views (path tracing, `cc_raytracing`).

**Items:** [`ViewRender`](#cc_engine-commands-render-viewrender)

<a id="cc_engine-commands-render-viewrender"></a>

### struct `ViewRender`

```rust
pub struct ViewRender
```

Render a 3D view photorealistically (path tracing on the processor, no
GPU) and write the image: `.png`, `.jpg`/`.jpeg` exposed, tone mapped and
sRGB encoded, `.exr` and `.hdr` in linear light. The view's render
settings are used (the defaults when it has none), patched by `settings`
for this render only; its camera is the one kept in the view, fitted to
the model when it was never moved, as the 3D view does. The view is
rendered as printed, without temporary hiding. Changes nothing in the
model. Returns the path, the image size, the samples per pixel, the
seconds spent and the triangles rendered.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  3D view id or name.

- `path: PathBuf`

  Path of the image to write (.png, .jpg, .jpeg, .exr or .hdr).

- `settings: Option<Value>`

  Render settings for this render only, not kept in the view: the
  fields to change over the view's settings (or the defaults when it
  has none), shaped and bounded like `render` in `view_set`.

- `region: Option<[u32; 4]>`

  Render only this rectangle of the image, `[x, y, width, height]` in
  pixels from its top left corner (inside the image); the rest stays
  black.


<a id="mod-cc_engine-commands-roofing"></a>

## `cc_engine::commands::roofing`

<sub>`crates/cc_engine/src/commands/roofing.rs`</sub>

Roof coverings and framing: checking a roof against its tiles.

**Items:** [`RoofCheck`](#cc_engine-commands-roofing-roofcheck) · [`roof_json`](#cc_engine-commands-roofing-roof_json)

<a id="cc_engine-commands-roofing-roofcheck"></a>

### struct `RoofCheck`

```rust
pub struct RoofCheck
```

Check a roof against its covering: the least slope its tiles take and
the roof's slopes, the area covered, the number of tiles and their
weight, and its framing (purlins, rafters, battens and truss members:
lengths, the number of trusses and the volume). The covering and framing are set on the roof's type
(`type_set` with `covering` and `framing`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Roof element id.


<a id="cc_engine-commands-roofing-roof_json"></a>

### fn `roof_json`

```rust
pub fn roof_json(covering: Option<TileKind>, system: FramingSystem, g: &RoofFramingGeom) -> Value
```

A roof's covering and framing figures as JSON, with the rules it breaks.

<a id="mod-cc_engine-commands-schedule"></a>

## `cc_engine::commands::schedule`

<sub>`crates/cc_engine/src/commands/schedule.rs`</sub>

Schedule definitions (see `cc_core::schedule`): the fields a schedule
shows, its filters, its sorting and grouping, how each column is written
and how the table looks on paper.

**Items:** [`ScheduleGet`](#cc_engine-commands-schedule-scheduleget) · [`ScheduleSet`](#cc_engine-commands-schedule-scheduleset)

<a id="cc_engine-commands-schedule-scheduleget"></a>

### struct `ScheduleGet`

```rust
pub struct ScheduleGet
```

A schedule's definition: the `fields` it shows in order, its `filters`
(`field`, `op`, `value`), its `sorting` levels (`field`, `descending`,
`header`, `footer`, `blank_line`), whether every element has its own row
(`itemize`), whether the totals row shows (`totals`), each column's
format (`columns`: `heading`, `align`, `hidden`, `total`) and the table's
`appearance` on paper; and the fields `available` to schedules of its
category. `schedule_set` changes any of them.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  Schedule view id or name.


<a id="cc_engine-commands-schedule-scheduleset"></a>

### struct `ScheduleSet`

```rust
pub struct ScheduleSet
```

Change a schedule's definition; every part is optional and replaces the
part it names, the others staying as they are. `fields` are the columns
in order. `filters` keep the rows that pass every one: each compares a
`field` with a `value` by `op` (`equals`, `not_equals`, `contains`,
`not_contains`, `begins_with`, `not_begins_with`, `greater`,
`greater_or_equal`, `less`, `less_or_equal`, `has_value`,
`has_no_value`); numbers compare as numbers in SI units (lengths in
meters), texts without case. `sorting` orders the rows by up to four
levels, the first outermost; a level with `header`, `footer` or
`blank_line` groups the rows alike in it under a heading, over a footer
(the group's count and totals) or apart by a blank line. With `itemize`
false, the elements alike in every sorting level share one row: values
that differ among them read as varying and totaled columns add up (the
`Count` field says how many). `columns` formats the columns of the fields
named (`heading`, `align`: `Auto`, `Left`, `Center`, `Right`; `hidden`: a
hidden field still sorts and filters; `total`: add the column up in
footers and totals), `totals` shows or hides the totals row, and
`appearance` is how the table is drawn on sheets and exports
(`grid_lines`, `outline`, `show_title`, `show_headers`, and the
`title_size`, `header_size` and `text_size` in millimeters). One undo
step; returns the definition as `schedule_get` does.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`

  Schedule view id or name.

- `fields: Option<Vec<String>>`

  The fields shown, in order (names as `schedule_get` lists them in `available`).

- `filters: Option<Value>`

  The filters, `[{"field": "Level", "op": "equals", "value": "Térreo"}, …]` (at most 8; `[]` for none).

- `sorting: Option<Value>`

  The sorting levels, `[{"field": "Type", "header": true, "footer": true}, …]` (at most 4; `[]` for none).

- `itemize: Option<bool>`

  One row per element (default true); false gathers the elements alike in every sorting level in one row.

- `totals: Option<bool>`

  Show the totals row.

- `columns: Option<Value>`

  Column formats, `[{"field": "Area", "heading": "Net area", "align": "Right", "total": true}, …]`: the fields named take the format given, the others keep theirs (`[]` clears every format).

- `appearance: Option<Value>`

  How the table looks on paper: a patch over `{grid_lines, outline, show_title, show_headers, title_size, header_size, text_size}`.


<a id="mod-cc_engine-commands-shaft"></a>

## `cc_engine::commands::shaft`

<sub>`crates/cc_engine/src/commands/shaft.rs`</sub>

Shaft openings: vertical openings through the floors, ceilings and roofs
between their base and top (see `cc_core::shaft`).

**Items:** [`new_shaft_type`](#cc_engine-commands-shaft-new_shaft_type) · [`ShaftCreate`](#cc_engine-commands-shaft-shaftcreate) · [`ShaftTypeCreate`](#cc_engine-commands-shaft-shafttypecreate)

<a id="cc_engine-commands-shaft-new_shaft_type"></a>

### fn `new_shaft_type`

```rust
pub fn new_shaft_type(id: ElementTypeId) -> ElementType
```

The shaft type a project gets with its first shaft.

<a id="cc_engine-commands-shaft-shaftcreate"></a>

### struct `ShaftCreate`

```rust
pub struct ShaftCreate
```

Create a shaft: a vertical opening with `outline` in plan (or drawn
with `curves`), from its level plus `base_offset` up to `top_level`
(plus `top_offset`) or `height`. Every floor, ceiling and roof it passes
through is cut where the outline lies inside it, clear of its own
openings; the cuts follow the shaft when it moves, changes or is
deleted. A project without a shaft type gets one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `outline: Vec<Point>`

  Outline `[[x, y], …]` in plan, closed (at least three points); or `curves`.

- `curves: Option<Vec<CurveArg>>`

  The outline drawn with curves (lines, arcs, circles, rectangles…).

- `level: Option<Ref>`

  Level id or name of the bottom (default: the lowest level).

- `base_offset: Option<f64>`

  Bottom above the level, meters (default 0).

- `top_level: Option<Ref>`

  Level the shaft rises to (default: the highest level above its own).

- `top_offset: Option<f64>`

  Top above `top_level`, meters (default 0).

- `height: Option<f64>`

  Height when there is no level above, meters (default 3; 10'-0" in imperial projects).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).


<a id="cc_engine-commands-shaft-shafttypecreate"></a>

### struct `ShaftTypeCreate`

```rust
pub struct ShaftTypeCreate
```

Create a shaft type (shafts have no properties of their own; types name them).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's shaft name).


<a id="mod-cc_engine-commands-sheets"></a>

## `cc_engine::commands::sheets`

<sub>`crates/cc_engine/src/commands/sheets.rs`</sub>

Sheets and their title blocks: paper, title block, fields and issue
date of a sheet; title block components and their 2D pieces.

**Items:** [`SizeArg`](#cc_engine-commands-sheets-sizearg) · [`rename_sheet_fields`](#cc_engine-commands-sheets-rename_sheet_fields) · [`SheetSet`](#cc_engine-commands-sheets-sheetset) · [`SheetFields`](#cc_engine-commands-sheets-sheetfields) · [`TitleBlockCreate`](#cc_engine-commands-sheets-titleblockcreate) · [`SheetPieceArg`](#cc_engine-commands-sheets-sheetpiecearg) · [`pattern_arg`](#cc_engine-commands-sheets-pattern_arg) · [`def_json`](#cc_engine-commands-sheets-def_json) · [`push_pieces`](#cc_engine-commands-sheets-push_pieces) · [`ComponentSheetPieceAdd`](#cc_engine-commands-sheets-componentsheetpieceadd) · [`check_nested`](#cc_engine-commands-sheets-check_nested) · [`check_nested_in`](#cc_engine-commands-sheets-check_nested_in) · [`ComponentSheetPieceSet`](#cc_engine-commands-sheets-componentsheetpieceset) · [`ComponentSheetPieceDelete`](#cc_engine-commands-sheets-componentsheetpiecedelete)

<a id="cc_engine-commands-sheets-sizearg"></a>

### enum `SizeArg`

```rust
pub enum SizeArg
```

A paper size by name (`"A0"`…`"A4"`, `"ANSI A"`…`"ANSI E"`,
`"ARCH A"`…`"ARCH E1"`, `"ARCH E"`), or `{width, height}` in millimeters.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Iso(String)`

  A named size, any case, spaces or `_` optional: `"A0"`…`"A4"`,
  `"ANSI A"`…`"ANSI E"`, `"ARCH A"`…`"ARCH D"`, `"ARCH E1"`, `"ARCH E"`.

- `Custom { width: f64, height: f64, }`

  A custom paper, millimeters.


<a id="cc_engine-commands-sheets-rename_sheet_fields"></a>

### fn `rename_sheet_fields`

```rust
pub(crate) fn rename_sheet_fields(e: &mut EditCtx, types: &[ElementTypeId], renames: &[(String, String)])
```

Sheets on any of the title block `types` keep their per-sheet values
under the parameters' new names (`renames`, `[old, new]` in order).

<a id="cc_engine-commands-sheets-sheetset"></a>

### struct `SheetSet`

```rust
pub struct SheetSet
```

Change a sheet: its number, paper (`size`, `portrait`), title block,
per-sheet field values and issue date.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `sheet: Ref`

  The sheet: id or name.

- `number: Option<String>`

  Sheet number.

- `size: Option<SizeArg>`

  Paper: `"A0"`…`"A4"`, `"ANSI A"`…`"ANSI E"`, `"ARCH A"`…`"ARCH E1"`, `"ARCH E"`,
  or `{width, height}` in millimeters (50 to 5000).

- `portrait: Option<bool>`

  The long side runs up.

- `title_block: Option<Option<Ref>>`

  The title block type (id or name), or `null` for none.

- `fields: Option<serde_json::Map<String, Value>>`

  Values of the title block's per-sheet fields by parameter name: a text sets one, `null` clears it.

- `issue_date: Option<String>`

  Issue date, `YYYY-MM-DD`; empty to clear.


<a id="cc_engine-commands-sheets-sheetfields"></a>

### struct `SheetFields`

```rust
pub struct SheetFields
```

A sheet's title block fields: every field key (`fields`, as `{key}`
in title block texts reads it) with its value on this sheet, the title
block's per-sheet parameters (`params`: name, label, the sheet's value
and the type's default), and the title block's pieces that do not
resolve on this paper (`errors`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `sheet: Ref`

  The sheet: id or name.


<a id="cc_engine-commands-sheets-titleblockcreate"></a>

### struct `TitleBlockCreate`

```rust
pub struct TitleBlockCreate
```

Create a title block component: a copy of the standard one, or empty
with `empty`. Sheets use it through `sheet_set` `title_block`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  Name of the component and of its first type.

- `empty: bool`

  Start with no pieces.


<a id="cc_engine-commands-sheets-sheetpiecearg"></a>

### enum `SheetPieceArg`

```rust
pub enum SheetPieceArg
```

A 2D piece of a title block or a detail component. A title block's
coordinates are millimeters from the paper's bottom-left corner
(numbers or expressions over the parameters, `SheetWidth` and
`SheetHeight`); a detail component's are meters in its own frame, text
heights too (numbers or expressions over its parameters).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Stroke { points: Vec<SketchPointArg>, #[serde(default)] closed: bool, #[serde(default)] weight: Option<ExprArg>, #[serde(default)] color: Option<[u8; 3]>, #[serde(default)] fill: Option<[u8; 3]>, #[serde(default)] pattern: Option<String>, }`

  Lines and arcs through `points`.

- `Nested { component: String, x: ExprArg, y: ExprArg, #[serde(default)] rotation: Option<ExprArg>, #[serde(default)] scale: Option<ExprArg>, #[serde(default)] mirrored: bool, }`

  Another detail component placed inside this one (detail components only).

- `Text { x: ExprArg, y: ExprArg, text: String, #[serde(default)] size: Option<ExprArg>, #[serde(default)] anchor: Option<String>, #[serde(default)] rotation: Option<ExprArg>, }`

  Text with `{fields}`; line breaks start new lines.

- `Image { x: ExprArg, y: ExprArg, width: ExprArg, height: ExprArg, #[serde(default)] image_path: Option<PathBuf>, #[serde(default)] image: Option<String>, }`

  A PNG, JPEG, GIF, BMP or WebP picture filling a rectangle from its bottom-left corner (x, y).


**Methods**

- `pub(crate) fn shape(&self, s: &Session) -> Result<SheetShape>`

<a id="cc_engine-commands-sheets-pattern_arg"></a>

### fn `pattern_arg`

```rust
pub(crate) fn pattern_arg(code: &str) -> Result<cc_core::LinePattern>
```

A dash pattern by its code.

<a id="cc_engine-commands-sheets-def_json"></a>

### fn `def_json`

```rust
pub(crate) fn def_json(def: &ComponentDef) -> Value
```

A definition as queries show it: each title block picture as its
format, size in pixels and `bytes`, not the bytes themselves.

<a id="cc_engine-commands-sheets-push_pieces"></a>

### fn `push_pieces`

```rust
pub(crate) fn push_pieces(def: &mut ComponentDef, shapes: Vec<SheetShape>, name: Option<&str>) -> Result<Vec<usize>>
```

Push `shapes` onto a title block's or detail component's pieces, the
first named `name` (the others `Piece1`, `Piece2`…), and return their
indices; refused whole when they would pass the most pieces it holds.

<a id="cc_engine-commands-sheets-componentsheetpieceadd"></a>

### struct `ComponentSheetPieceAdd`

```rust
pub struct ComponentSheetPieceAdd
```

Add 2D pieces to a title block or detail component (every type of it), in one step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the title block or detail component: id or name.

- `piece: Option<SheetPieceArg>`

  The piece, by `kind`: `stroke` (`points` as [{u, v, radius}], `closed`,
  `weight`, `color`, `fill`, `pattern`), `text` (`x`, `y`, `text` with
  `{fields}`, `size`, `anchor`: left, center, right or middle,
  `rotation`), `image` (`x`, `y`, `width`, `height`, and `image_path` or
  base64 `image`) or, in a detail component, `nested` (`component`,
  `x`, `y`, `rotation`, `scale`, `mirrored`). A title block's are
  millimeters from the paper's bottom-left corner, as numbers or
  expressions over the parameters, `SheetWidth` and `SheetHeight`; a
  detail component's are meters in its frame.

- `pieces: Vec<SheetPieceArg>`

  More pieces (each as `piece`), added after it in the same step: all
  of them or, when one is refused, none.

- `name: Option<String>`

  Name of the first piece added (default `Piece1`, `Piece2`…).


<a id="cc_engine-commands-sheets-check_nested"></a>

### fn `check_nested`

```rust
pub(crate) fn check_nested(p: &Project, type_ref: &Ref, shapes: &[SheetShape]) -> Result<()>
```

Nested pieces must name detail components that do not hold the
component of `type_ref` (a cycle).

<a id="cc_engine-commands-sheets-check_nested_in"></a>

### fn `check_nested_in`

```rust
pub(crate) fn check_nested_in(p: &Project, component: &str, shapes: &[SheetShape]) -> Result<()>
```

Nested pieces of `shapes` must name detail components that do not hold `component`.

<a id="cc_engine-commands-sheets-componentsheetpieceset"></a>

### struct `ComponentSheetPieceSet`

```rust
pub struct ComponentSheetPieceSet
```

Change a 2D piece of a title block or detail component: `piece` is a merge patch over its
form in `component_get` `def.sheet` (e.g. `{"Text": {"size": "4"}}`);
an image piece's picture is replaced with `image_path` or `image`, not
by the patch.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the title block: id or name.

- `index: usize`

  The piece's index (in `component_get` `def.sheet`).

- `piece: Option<Value>`

  Merge patch over the piece's shape (without a picture).

- `name: Option<String>`

  New name.

- `image_path: Option<PathBuf>`

  A new picture for an image piece: its file (PNG, JPEG, GIF, BMP or WebP).

- `image: Option<String>`

  A new picture for an image piece: its bytes, base64.

- `add: Vec<SheetPieceArg>`

  Pieces added in the same step (each as `component_sheet_piece_add`
  `piece`; e.g. the other chains of a redrawn sketch); the result lists
  their indices in `added`.


<a id="cc_engine-commands-sheets-componentsheetpiecedelete"></a>

### struct `ComponentSheetPieceDelete`

```rust
pub struct ComponentSheetPieceDelete
```

Delete 2D pieces of a title block or detail component, in one step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  A type of the title block: id or name.

- `index: Option<usize>`

  The piece's index (in `component_get` `def.sheet`).

- `indices: Vec<usize>`

  More pieces' indices, deleted with `index` in the same step: all of
  them or, when one does not exist, none.


<a id="mod-cc_engine-commands-site"></a>

## `cc_engine::commands::site`

<sub>`crates/cc_engine/src/commands/site.rs`</sub>

The site location (latitude, longitude, elevation, true north and the
postal address) and the map data brought in around it (a site drawing
and context masses from OpenStreetMap).

**Items:** [`site_json`](#cc_engine-commands-site-site_json) · [`SiteGet`](#cc_engine-commands-site-siteget) · [`SiteSet`](#cc_engine-commands-site-siteset) · [`SiteContextImport`](#cc_engine-commands-site-sitecontextimport)

<a id="cc_engine-commands-site-site_json"></a>

### fn `site_json`

```rust
pub fn site_json(s: &Session) -> Value
```

The site as commands show it (`null` when the project has no location).

<a id="cc_engine-commands-site-siteget"></a>

### struct `SiteGet`

```rust
pub struct SiteGet
```

The project's site location: latitude and longitude of the project
origin (degrees, WGS 84), its elevation above sea level (meters), the
angle from project north to true north (degrees counter-clockwise),
the place name, the parts of the postal address, and local time's
offset from UTC in hours (`utc_offset`, null when not set, and
`utc_offset_used`: the one set, else the longitude's nominal time zone,
which the sun uses); `site` is null when no location is set. `address` is the project address line.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-site-siteset"></a>

### struct `SiteSet`

```rust
pub struct SiteSet
```

Set the project's site location, in one undo step: the latitude and
longitude of the project origin (both needed when the project has no
location yet), its elevation above sea level, true north and the
place and address found by a map search, and the offset from UTC.
Fields left out keep their values. When address parts are given the project address (title
blocks, IFC) is rewritten from them unless `project_address` is false.
`clear` removes the location.

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `latitude: Option<f64>`

  Latitude of the project origin, degrees (WGS 84, -90 to 90, north positive).

- `longitude: Option<f64>`

  Longitude of the project origin, degrees (WGS 84, -180 to 180, east positive).

- `elevation: Option<f64>`

  Height of the project's elevation 0 above sea level, meters (-500 to 9000).

- `true_north: Option<f64>`

  Angle from project north (+Y) to true north, degrees counter-clockwise.

- `place: Option<String>`

  The place as the map search named it.

- `address: Option<serde_json::Map<String, Value>>`

  Address parts by key (`street`, `number`, `district`, `city`, `region`, `postcode`, `country`); others are refused.

- `project_address: Option<bool>`

  Rewrite the project address from the address parts (default true when `address` is given).

- `utc_offset: Option<Option<f64>>`

  Local time's offset from UTC, hours (-12 to 14; fractions allowed),
  or `null` for the longitude's nominal time zone.

- `clear: Option<bool>`

  Remove the site location (nothing else may be given).


<a id="cc_engine-commands-site-sitecontextimport"></a>

### struct `SiteContextImport`

```rust
pub struct SiteContextImport
```

Bring OpenStreetMap data around the site into a floor plan, in one undo
step: a site drawing (an imported drawing in meters, pinned, under the
model: layers for buildings, streets, paths, railways, water, green
areas, trees and street names) and, unless `masses` is false, a
context mass per building footprint (height from the map's `height` or
`building:levels`, else `default_levels` storeys; no mass floors) of
the "Context" mass type. `data` is the JSON an Overpass query answers
(`[out:json]`, `out geom` or ways with their nodes; the program builds
the query for the square it shows); points are placed with the site
location, which must be set. Map data © OpenStreetMap contributors (ODbL).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `data: Option<String>`

  Overpass JSON text (or `path`).

- `path: Option<PathBuf>`

  Path of a file holding Overpass JSON (or `data`).

- `view: Option<Ref>`

  Floor plan id or name (default: the first floor plan of the lowest level).

- `extent: Option<f64>`

  Half the side of the square kept around the project origin, meters (default: everything given).

- `masses: Option<bool>`

  Make context masses from the building footprints (default true).

- `level_height: Option<f64>`

  Height of a storey, meters, for `building:levels` (default 3).

- `default_levels: Option<f64>`

  Storeys of a building whose height the map does not give (default 2).

- `text_height: Option<f64>`

  Height of street names, meters (default 2).


<a id="mod-cc_engine-commands-sketch"></a>

## `cc_engine::commands::sketch`

<sub>`crates/cc_engine/src/commands/sketch.rs`</sub>

Sketched outlines: the curves of floors, ceilings, roofs and masses
(see `cc_core::sketch`), given to the commands that create them as
`curves`, read with `sketch_get` and drawn again with `sketch_set`.

**Items:** [`CurveArg`](#cc_engine-commands-sketch-curvearg) · [`sketch_of`](#cc_engine-commands-sketch-sketch_of) · [`sketch_error`](#cc_engine-commands-sketch-sketch_error) · [`profile`](#cc_engine-commands-sketch-profile) · [`outline`](#cc_engine-commands-sketch-outline) · [`SketchGet`](#cc_engine-commands-sketch-sketchget) · [`SketchSet`](#cc_engine-commands-sketch-sketchset)

<a id="cc_engine-commands-sketch-curvearg"></a>

### enum `CurveArg`

```rust
pub enum CurveArg
```

A curve of a sketch. Rectangles and polygons become their sides (lines).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`

**Variants**

- `Line { start: Point, end: Point, }`

  A straight line.

- `Arc { start: Point, end: Point, #[serde(default)] bulge: Option<f64>, #[serde(default)] through: Option<Point>, }`

  A circular arc between two points: by its `bulge` (tan of a quarter of
  the turn, positive counter-clockwise, 1 for a half circle) or a point
  it passes `through`.

- `Circle { center: Point, radius: f64, }`

  A whole circle.

- `Ellipse { center: Point, radii: Point, #[serde(default)] rotation: f64, #[serde(default)] start: f64, #[serde(default)] sweep: Option<f64>, }`

  An ellipse, or a part of it from `start` turning by `sweep` degrees
  (of its parameter angle).

- `Rectangle { corner: Point, opposite: Point, #[serde(default)] rotation: f64, }`

  A rectangle from a corner to the opposite one, turned `rotation`
  degrees about the first corner: four lines.

- `Polygon { center: Point, radius: f64, sides: usize, #[serde(default = "yes")] inscribed: bool, #[serde(default)] rotation: f64, }`

  A regular polygon: its sides as lines.

- `Spline { #[serde(default)] through: Option<Vec<Point>>, #[serde(default)] control: Option<Vec<Point>>, #[serde(default)] closed: bool, }`

  A smooth curve: `through` the points given (at least 2), or by the
  `control` points of its cubic Bézier segments (`p0, c1, c2, p1, c1,
  c2, p2…`, 3n + 1 of them, as `sketch_get` gives it); closed back to
  its first point with `closed` (through points only).


**Methods**

- `pub fn curves(&self) -> Result<Vec<SketchCurve>>`

  The sketch curves this argument draws.

- `pub fn of(curve: &SketchCurve) -> Self`

  The argument that gives back `curve`.


<a id="cc_engine-commands-sketch-sketch_of"></a>

### fn `sketch_of`

```rust
pub fn sketch_of(args: &[CurveArg]) -> Result<Sketch>
```

The sketch the arguments draw.

<a id="cc_engine-commands-sketch-sketch_error"></a>

### fn `sketch_error`

```rust
pub fn sketch_error(e: SketchError) -> EngineError
```

A sketch's error as a command error.

<a id="cc_engine-commands-sketch-profile"></a>

### fn `profile`

```rust
pub fn profile(sketch: &Sketch) -> Result<Profile>
```

The outline and openings of a sketch.

<a id="cc_engine-commands-sketch-outline"></a>

### fn `outline`

```rust
pub fn outline(sketch: &Sketch) -> Result<Vec<DVec2>>
```

The outline of a sketch that takes no openings.

<a id="cc_engine-commands-sketch-sketchget"></a>

### struct `SketchGet`

```rust
pub struct SketchGet
```

The curves an element's outline is drawn with: its sketch, or lines along
its outline and openings when it has none (floors, ceilings, roofs,
masses and the other sketched elements, regions among them).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id.


<a id="cc_engine-commands-sketch-sketchset"></a>

### struct `SketchSet`

```rust
pub struct SketchSet
```

Draw an element's outline again from curves (edit its boundary): floors,
roofs and regions take the loops inside the outline as their openings; ceilings
and masses take one loop. The element keeps the curves, so `sketch_get`
gives them back. A terrain's measured points can change in the same
step (`points`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Element id (a floor, ceiling, roof, mass, shaft, terrain, pad, or a
  filled or masking region).

- `curves: Vec<CurveArg>`

  The curves, closed into loops.

- `points: Option<Vec<super::topography::Point3>>`

  Terrain only: measured points `[[x, y, z], …]` replacing the current
  ones (points that repeat another in plan are dropped).


<a id="mod-cc_engine-commands-slab_rebar"></a>

## `cc_engine::commands::slab_rebar`

<sub>`crates/cc_engine/src/commands/slab_rebar.rs`</sub>

Slab reinforcement: a mesh, or ring and radial bars, in the top and
bottom layers of floors and foundation slabs, and trimming bars around
their openings.

**Items:** [`TrimBars`](#cc_engine-commands-slab_rebar-trimbars) · [`SlabRebarCreate`](#cc_engine-commands-slab_rebar-slabrebarcreate)

<a id="cc_engine-commands-slab_rebar-trimbars"></a>

### struct `TrimBars`

```rust
pub struct TrimBars
```

Trimming bars around the openings of a slab.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type of the slab's bars).

- `count: Option<u32>`

  Bars along each side of an opening (default 2).

- `spacing: Option<f64>`

  Distance between them, meters (default 0.05; 2" in imperial projects).

- `anchorage: Option<f64>`

  How far each bar reaches past the opening, meters (default: 40 bar diameters).

- `diagonals: Option<bool>`

  A diagonal bar across each corner of the opening (default true).


<a id="cc_engine-commands-slab_rebar-slabrebarcreate"></a>

### struct `SlabRebarCreate`

```rust
pub struct SlabRebarCreate
```

Reinforce slabs: floors and foundation slabs of any boundary. `layout`
is `mesh` (default): straight bars in two directions, `angle` and square
to it, `spacing` (and `cross_spacing`) apart; or `radial`: ring bars
`spacing` apart around `center` and `radial_count` radial bars, with
shorter bars from `intermediate` between them. The bars go in the `top`
and `bottom` layers (`layers`), `cover` inside the faces of the slab's
structure, its edge and its openings: each bar is clipped to the true
boundary (arcs and circles as drawn), so every bar has its own length
(`element_get` lists a set's `derived.bars` by length). `hook` bends the
ends of mesh bars at the slab's edge. Bars longer than `max_length` are
lap spliced, neighbouring splices staggered. A slab with openings gets
trimming bars around them (`trim`; `false` for none). One set per layer
and direction, in one undo step; the bars follow the slab. Without
`slabs`, the structural floors (`floor_create` `structural`) and the
foundation slabs are reinforced.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `slabs: Vec<u64>`

  Floor and foundation slab ids (default: the structural floors and the foundation slabs
  of `level`, or of the project).

- `level: Option<Ref>`

  Level id or name, when no slabs are given.

- `layout: Option<String>`

  `mesh` (default) or `radial`.

- `layers: Option<String>`

  The layers reinforced: `both` (default), `top` or `bottom`.

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type nearest 10 mm; 1/2" in imperial projects).

- `cross_type: Option<Ref>`

  Bar type of the second direction of a mesh, or of the radial bars (default: `type`).

- `spacing: Option<f64>`

  Distance between the bars of a mesh, or between ring bars, meters (default 0.15; 6" in
  imperial projects).

- `cross_spacing: Option<f64>`

  Distance between the bars of the second direction of a mesh (default: `spacing`).

- `angle: Option<f64>`

  Direction of the first bars of a mesh, degrees from the plan's X axis (default 0); the
  second direction is square to it. The first direction is the outer layer.

- `hook: Option<f64>`

  Bend at the ends of mesh bars on the slab's edge, meters: positive towards the middle of
  the slab (down for top bars, up for bottom ones), negative out of it (into the wall
  above or below). Default 0: straight ends.

- `center: Option<[f64; 2]>`

  `radial`: the center of the rings and radial bars (default: the middle of the slab).

- `radial_count: Option<u32>`

  `radial`: the number of radial bars (default: as many as keep them `spacing` apart at
  the edge).

- `start: Option<f64>`

  `radial`: the radius the radial bars start at, meters (default 0: the center).

- `intermediate: Option<f64>`

  `radial`: the radius from which a shorter bar runs between each two radial bars (default
  0: none).

- `cover: Option<f64>`

  Concrete cover, meters (default 0.025; 1 1/2" in imperial projects).

- `trim: Option<Value>`

  Trimming bars around openings: their values, or `false` for none (default: two bars along
  each side reaching 40 diameters past the opening, and a diagonal across each corner).

- `max_length: Option<f64>`

  Longest bar, meters (default: the stock length, 12 m, or 40' in imperial projects):
  longer bars are lap spliced. 0 keeps whole bars.

- `lap: Option<f64>`

  Lap length of the splices, meters (default: 40 bar diameters).

- `partition: Option<String>`

  The part of the structure the sets belong to (e.g. "FOUNDATION", "BOTTOM SLAB"), which
  schedules filter and group by (default: none).


<a id="mod-cc_engine-commands-stair"></a>

## `cc_engine::commands::stair`

<sub>`crates/cc_engine/src/commands/stair.rs`</sub>

Stairs: creating them in any shape, checking their steps and adding the
railings that follow them.

**Items:** [`SHAPES`](#cc_engine-commands-stair-shapes) · [`StairCreate`](#cc_engine-commands-stair-staircreate) · [`StairCheck`](#cc_engine-commands-stair-staircheck) · [`issue_text`](#cc_engine-commands-stair-issue_text) · [`StairRailingsAdd`](#cc_engine-commands-stair-stairrailingsadd) · [`RailingHostSet`](#cc_engine-commands-stair-railinghostset)

<a id="cc_engine-commands-stair-shapes"></a>

### const `SHAPES`

```rust
pub const SHAPES: [(&str, StairShape); 6]
```

Names of the stair shapes in commands.

<a id="cc_engine-commands-stair-staircreate"></a>

### struct `StairCreate`

```rust
pub struct StairCreate
```

Create a stair starting at `start` (the middle of its first riser) and
rising along `direction` up to the level above (or `height`). Risers and
treads follow the type (the fewest risers within its maximum, and a fixed
tread or one from Blondel's formula `2 × riser + tread`) unless `risers`
or `tread` set them: the riser height is then the height over the risers.
`shape` lays it out straight, with a landing, L- or U-shaped, in three
flights round two landings, or around a center. The top floor is the last
step: a stair ends with a riser, not a tread.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `start: Point`

  Start of the run (center of the first step's edge).

- `direction: Option<f64>`

  Direction of the first flight in plan, degrees counter-clockwise from +X.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `height: Option<f64>`

  Height when there is no level above (default 3; 10'-0" in imperial projects).

- `top_level: Option<Ref>`

  Level the stair rises to (default: the level above).

- `shape: Option<String>`

  Plan shape: `straight` (default), `straight_landing`, `l_shaped`,
  `u_shaped`, `three_flight` (up, across and back, with a landing at
  each corner) or `spiral`.

- `turn: Option<String>`

  Side the stair turns to, walking up: `left` (default) or `right`.

- `risers: Option<u32>`

  Riser count (default: the fewest within the type's maximum riser).

- `tread: Option<f64>`

  Tread depth, meters (default: the type's).

- `split: Option<u32>`

  Risers of the first flight (default: half of them; a third on a three-flight stair).

- `split2: Option<u32>`

  Risers of the middle flight of a three-flight stair (default: half of those left).

- `landing: Option<f64>`

  Landing depth along the first flight, meters (default: the stair width).

- `gap: Option<f64>`

  Clear gap between the flights of a U-shaped stair, meters (default 0.1; 4" in imperial projects).

- `winders: Option<u32>`

  Winder treads in the corner of an L-shaped stair instead of a landing (2 to 6).

- `inner_radius: Option<f64>`

  Radius of the inner edge of a spiral stair, meters (default 0.15; 6" in imperial projects).

- `posts: Option<Vec<Vec<String>>>`

  Posts under the landings of an assembled stair, down to its base: a
  list per landing (up the stair) of places round its edge, named
  walking onto it (front: where the stair arrives; inner: the side it
  turns to): `front_outer`, `front`, `front_inner`, `inner`,
  `back_inner`, `back`, `back_outer`, `outer`. Only places its beams
  reach take a post (see `stair_check`); none by default.

- `numbers: Option<bool>`

  Number the steps in plans: the treads (winders included, landings
  not) and the floor the stair arrives at (default false).

- `number_start: Option<i32>`

  Number of the first tread (default 1), from -9999 to 9999: another
  value continues the count of a stair below, for instance.


**Methods**

- `pub fn layout(&self, defaults: &cc_core::Defaults) -> Result<StairLayout>`

  The plan layout these parameters ask for.


<a id="cc_engine-commands-stair-staircheck"></a>

### struct `StairCheck`

```rust
pub struct StairCheck
```

Check a stair's steps: riser count and height, tread on the walking line,
Blondel's step `2 × riser + tread` and the rules of its type they break
(riser out of range, short tread, step out of the Blondel range, short
landing). `landings` lists each landing's elevation and the places round
its edge where a post can stand under an assembled stair (where its
beams reach the edge), with their plan points and whether one stands there.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Stair element id.


<a id="cc_engine-commands-stair-issue_text"></a>

### fn `issue_text`

```rust
pub fn issue_text(issue: &cc_core::StairIssue) -> String
```

A broken stair rule in words.

<a id="cc_engine-commands-stair-stairrailingsadd"></a>

### struct `StairRailingsAdd`

```rust
pub struct StairRailingsAdd
```

Add railings along the sides of a stair or a ramp: they follow its
flights, landings and turns (at the pitch of the nosings on stairs), and
go with it when it changes, moves or is deleted.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Stair or ramp element id.

- `side: Option<String>`

  `left`, `right` or `both` (default), walking up.

- `type_ref: Option<Ref>`

  Railing type id or name (default: the category's default type).

- `inset: Option<f64>`

  Distance in from the edge of the stair, meters (default 0.05).

- `near: Option<Point>`

  A point `[x, y]` in plan: without `side`, the railing goes on the
  side nearest it (where a click picked the stair).


<a id="cc_engine-commands-stair-railinghostset"></a>

### struct `RailingHostSet`

```rust
pub struct RailingHostSet
```

Put a railing on a stair or ramp (`host`), or take it off (`detach`).
On a host it follows the host's `side` (default: the side its path runs
nearest), `inset` in from the edge, and moves, turns and goes with it;
taken off, it keeps the path it had in plan, at its level plus
`base_offset`. A hosted railing can also change only its side or inset.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `id: u64`

  Railing element id.

- `host: Option<u64>`

  Stair or ramp element id to follow.

- `detach: bool`

  Take the railing off its host.

- `side: Option<String>`

  `left` or `right`, walking up.

- `inset: Option<f64>`

  Distance in from the edge of the host, meters (default 0.05, or the railing's own).


<a id="mod-cc_engine-commands-steel_building"></a>

## `cc_engine::commands::steel_building`

<sub>`crates/cc_engine/src/commands/steel_building.rs`</sub>

Parametric steel buildings (see `cc_core::steel_building`): the host
element and the catalog types its members get by default.

**Items:** [`FRAME_DEFAULT`](#cc_engine-commands-steel_building-frame_default) · [`SECONDARY_DEFAULT`](#cc_engine-commands-steel_building-secondary_default) · [`new_steel_building_type`](#cc_engine-commands-steel_building-new_steel_building_type) · [`SecondaryMembers`](#cc_engine-commands-steel_building-secondarymembers) · [`SteelBuildingCreate`](#cc_engine-commands-steel_building-steelbuildingcreate) · [`derived`](#cc_engine-commands-steel_building-derived)

<a id="cc_engine-commands-steel_building-frame_default"></a>

### const `FRAME_DEFAULT`

```rust
pub const FRAME_DEFAULT: (&str, &str)
```

Columns and rafters by default: (series, designation).

<a id="cc_engine-commands-steel_building-secondary_default"></a>

### const `SECONDARY_DEFAULT`

```rust
pub const SECONDARY_DEFAULT: (&str, &str)
```

Purlins and girts by default.

<a id="cc_engine-commands-steel_building-new_steel_building_type"></a>

### fn `new_steel_building_type`

```rust
pub fn new_steel_building_type(id: ElementTypeId) -> ElementType
```

The type a project gets with its first steel building.

<a id="cc_engine-commands-steel_building-secondarymembers"></a>

### struct `SecondaryMembers`

```rust
pub struct SecondaryMembers
```

Purlins or girts.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Beam type id or name (default: the catalog's `Ue 150x60x20x2,00`, made when missing).

- `spacing: Option<f64>`

  Distance between lines, meters, 0.3 to 10 (default 1.5): purlins
  along the rafter's plan run, girts up the wall.


<a id="cc_engine-commands-steel_building-steelbuildingcreate"></a>

### struct `SteelBuildingCreate`

```rust
pub struct SteelBuildingCreate
```

Create a parametric steel building: portal frames (two columns and the
rafters of a gable or mono-pitch roof) every `bay_spacing` along it,
purlins on the roof and girts on both long sides, as ordinary columns
and beams the building keeps in step with its parameters (`element_set`
regenerates them, keeping their ids). A member edited or deleted by hand
is detached and left alone. Default types come from the profile catalog
(`W310x38,7` for columns and rafters, `Ue 150x60x20x2,00` for purlins and
girts), made when missing. Out-of-range values are refused. One undo
step; returns the building's `id` and its `members`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `origin: Option<Point>`

  Plan point `[x, y]` of the first frame's left column (default `[0, 0]`).

- `rotation: Option<f64>`

  Turn of the span direction, degrees counter-clockwise (default 0).

- `span: Option<f64>`

  Column axis to column axis, meters, 2 to 100 (default 20).

- `eave_height: Option<f64>`

  Underside of the rafters at the columns above the level, meters, 1 to 50 (default 6).

- `slope_percent: Option<f64>`

  Roof slope, percent, 0 to 100 (default 10).

- `roof: Option<String>`

  `gable` (ridge at mid-span, default) or `mono` (rising toward the far columns).

- `bays: Option<u32>`

  Bays, 1 to 100 (default 5); there is one frame more.

- `bay_spacing: Option<f64>`

  Frame to frame, meters, 1 to 30 (default 6).

- `column_type: Option<Ref>`

  Column type id or name (default: catalog `W310x38,7`).

- `rafter_type: Option<Ref>`

  Rafter (beam) type id or name (default: catalog `W310x38,7`).

- `purlins: Option<Option<SecondaryMembers>>`

  Purlins `{ "type", "spacing" }`, or `null` for none (default: on).

- `girts: Option<Option<SecondaryMembers>>`

  Girts on both long sides `{ "type", "spacing" }`, or `null` for none (default: on).


<a id="cc_engine-commands-steel_building-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(p: &cc_core::Project, b: &SteelBuilding, out: &mut serde_json::Map<String, Value>)
```

`element_get`'s derived values of a building: its member ids by role
kind, how many roles are detached and whether the layout reached the
member limit (`at_limit`).

<a id="mod-cc_engine-commands-strip_rebar"></a>

## `cc_engine::commands::strip_rebar`

<sub>`crates/cc_engine/src/commands/strip_rebar.rs`</sub>

Strip footing reinforcement: bars across and along the footing slab,
and stirrups, side bars and longitudinal bars for the beam on top.

**Items:** [`StripBeamBars`](#cc_engine-commands-strip_rebar-stripbeambars) · [`StripRebarCreate`](#cc_engine-commands-strip_rebar-striprebarcreate)

<a id="cc_engine-commands-strip_rebar-stripbeambars"></a>

### struct `StripBeamBars`

```rust
pub struct StripBeamBars
```

The bars of the beam on top of a strip footing (or of the whole section
of a strip without one).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Bar type id or name of the longitudinal and side bars (default: the type nearest 10 mm;
  1/2" in imperial projects).

- `top: Option<u32>`

  Longitudinal bars along the top of the beam (default 2; 0 for none).

- `bottom: Option<u32>`

  Longitudinal bars along its bottom, in the footing (default 0: the footing's own bars).

- `sides: Option<u32>`

  Side bars on each side face (default 0).

- `stirrup_type: Option<Ref>`

  Stirrup type id or name (default: the type nearest 5 mm; 3/8" in imperial projects).

- `stirrups: Option<f64>`

  Stirrup spacing along the strip, meters (default 0.15; 6" in imperial projects; 0 for
  none). The stirrups run over the full height of the section, in the width of the beam.


<a id="cc_engine-commands-strip_rebar-striprebarcreate"></a>

### struct `StripRebarCreate`

```rust
pub struct StripRebarCreate
```

Reinforce strip footings, straight or arc: bars across the footing
(`transverse_spacing` apart along it; radial in an arc strip) and bars
along it (`longitudinal_spacing` apart across it; ring bars in an arc
strip, each as long as the arc at its own radius), in the bottom and
top `layers`. A top bar across the footing follows its top, up the
slopes of a haunched section and flat between the faces of its beam, and
its ends turn down to the bottom layer, closing the edges. The bars
along a haunched footing follow the slopes, and leave the beam to its
own: `beam` gives it stirrups over the full height of the section, top,
bottom and side bars (a haunched strip gets stirrups and two top bars
by default; `false` for none). Bars along the strip longer than
`max_length` are lap spliced, neighbouring splices staggered. One set
per layer and direction, in one undo step; the bars follow the strip.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `strips: Vec<u64>`

  Strip footing ids (default: those of `level`, or of the project).

- `level: Option<Ref>`

  Level id or name, when no strips are given.

- `layers: Option<String>`

  The layers of the footing reinforced: `both` (default), `top` or `bottom`.

- `type_ref: Option<Ref>`

  Bar type id or name of the bars across the footing (default: the type nearest 10 mm; 1/2"
  in imperial projects).

- `longitudinal_type: Option<Ref>`

  Bar type of the bars along the footing (default: `type`).

- `transverse_spacing: Option<f64>`

  Distance between the bars across the footing, along it, meters (default 0.15; 6" in
  imperial projects; 0 for none).

- `longitudinal_spacing: Option<f64>`

  Distance between the bars along the footing, across it, meters (default 0.15; 0 for
  none).

- `beam: Option<Value>`

  The bars of the beam part: their values, or `false` for none.

- `cover: Option<f64>`

  Concrete cover, meters (default 0.025; 1 1/2" in imperial projects).

- `max_length: Option<f64>`

  Longest bar along the strip, meters (default: the stock length, 12 m, or 40' in imperial
  projects): longer bars are lap spliced. 0 keeps whole bars.

- `lap: Option<f64>`

  Lap length of the splices, meters (default: 40 bar diameters).

- `lap_at_joins: Option<bool>`

  Run the bars along the strip half a lap past each end joined to another strip that
  carries on in the same line (the strips of a ring footing), so the bars of the two
  lap there (default false: they stop the cover short of the end). The lap is `lap`,
  or 40 bar diameters where bars are kept whole (`max_length` 0).

- `partition: Option<String>`

  The part of the structure the sets belong to (e.g. "FOUNDATION", "WALL"), which
  schedules filter and group by (default: none).


<a id="mod-cc_engine-commands-structure"></a>

## `cc_engine::commands::structure`

<sub>`crates/cc_engine/src/commands/structure.rs`</sub>

Structural layout from grids: columns at grid intersections and beams
between the columns along grid lines or in a given order.

**Items:** [`SAME`](#cc_engine-commands-structure-same) · [`grids_with_ids`](#cc_engine-commands-structure-grids_with_ids) · [`crossing`](#cc_engine-commands-structure-crossing) · [`intersections`](#cc_engine-commands-structure-intersections) · [`columns_on`](#cc_engine-commands-structure-columns_on) · [`beam_exists`](#cc_engine-commands-structure-beam_exists) · [`ColumnsAtGrids`](#cc_engine-commands-structure-columnsatgrids) · [`BeamsOnGrids`](#cc_engine-commands-structure-beamsongrids) · [`BeamsBetweenColumns`](#cc_engine-commands-structure-beamsbetweencolumns) · [`SlabJoinSupports`](#cc_engine-commands-structure-slabjoinsupports) · [`StructureNumber`](#cc_engine-commands-structure-structurenumber)

<a id="cc_engine-commands-structure-same"></a>

### const `SAME`

```rust
pub(crate) const SAME: f64
```

Points closer than this are the same place (1 cm).

<a id="cc_engine-commands-structure-grids_with_ids"></a>

### fn `grids_with_ids`

```rust
pub(crate) fn grids_with_ids(p: &Project, ids: &[u64]) -> Result<Vec<(ElementId, (DVec2, DVec2))>>
```

The grids named by `ids` with their lines, or every grid when none are given.

<a id="cc_engine-commands-structure-crossing"></a>

### fn `crossing`

```rust
pub(crate) fn crossing((a0, a1): (DVec2, DVec2), (b0, b1): (DVec2, DVec2)) -> Option<DVec2>
```

Where segments `a` and `b` cross (ends included), if they are not parallel.

<a id="cc_engine-commands-structure-intersections"></a>

### fn `intersections`

```rust
pub(crate) fn intersections(lines: &[(DVec2, DVec2)]) -> Vec<DVec2>
```

Every intersection of the lines, each place once.

<a id="cc_engine-commands-structure-columns_on"></a>

### fn `columns_on`

```rust
pub(crate) fn columns_on(p: &Project, level: LevelId) -> Vec<(ElementId, DVec2)>
```

Columns of `level` with their centers.

<a id="cc_engine-commands-structure-beam_exists"></a>

### fn `beam_exists`

```rust
pub(crate) fn beam_exists(p: &Project, level: LevelId, a: DVec2, b: DVec2) -> bool
```

Whether a beam of `level` already runs between `a` and `b` (either way).

<a id="cc_engine-commands-structure-columnsatgrids"></a>

### struct `ColumnsAtGrids`

```rust
pub struct ColumnsAtGrids
```

Place a column at every intersection of grid lines on a level, skipping
places that already have a column. Returns the new ids (`created`) and how
many intersections were skipped.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `grids: Vec<u64>`

  Grid ids (default: every grid).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Column type id or name (default: the category's default type).

- `rotation: Option<f64>`

  Rotation in plan, degrees.

- `height: Option<f64>`

  Unconnected height (default: the level's height).

- `top_level: Option<Ref>`

  Level the tops are attached to (default: the level above).

- `top_offset: Option<f64>`

  Offset of the tops from `top_level`, meters.

- `structural: Option<bool>`

  Load-bearing (default true).


<a id="cc_engine-commands-structure-beamsongrids"></a>

### struct `BeamsOnGrids`

```rust
pub struct BeamsOnGrids
```

Place beams along grid lines: on each grid, a beam between each pair of
neighboring columns of the level whose centers lie on it. Spans that
already have a beam are skipped.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `grids: Vec<u64>`

  Grid ids (default: every grid).

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Beam type id or name (default: the category's default type).

- `top_offset: Option<f64>`

  Top of the beams above the level, meters (default: the level height).


<a id="cc_engine-commands-structure-beamsbetweencolumns"></a>

### struct `BeamsBetweenColumns`

```rust
pub struct BeamsBetweenColumns
```

Place beams from column to column in the order given (a chain), on the
level of the first column. Spans that already have a beam are skipped.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `columns: Vec<u64>`

  Column ids, at least two, in the order to connect.

- `type_ref: Option<Ref>`

  Beam type id or name (default: the category's default type).

- `top_offset: Option<f64>`

  Top of the beams above the level, meters (default: the level height).


<a id="cc_engine-commands-structure-slabjoinsupports"></a>

### struct `SlabJoinSupports`

```rust
pub struct SlabJoinSupports
```

Join each slab with the structure that carries it: beams, structural
columns and structural walls whose bodies overlap it (default: every
floor, or every floor of `level`). Beams and columns cut the slab, the
slab cuts walls, and the shared volume counts once. Pairs already joined
are left as they are.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `floors: Vec<u64>`

  Floor ids (default: every floor, or those of `level`).

- `level: Option<Ref>`

  Level id or name, when no floors are given.


<a id="cc_engine-commands-structure-structurenumber"></a>

### struct `StructureNumber`

```rust
pub struct StructureNumber
```

Number the marks of beams, columns and pad footings (default: all three)
in reading order: horizontal beams top to bottom then left to right,
the other beams left to right then top to bottom, columns and footings
top to bottom then left to right. Marks are a prefix per category (`V`,
`P`, `S` in Portuguese; `B`, `C`, `F` in English) and a number.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `categories: Vec<String>`

  Categories to number: `beam`, `column` and/or `foundation` (default: all three).

- `level: Option<Ref>`

  Level id or name (default: every level, numbered as one sequence per category).


<a id="mod-cc_engine-commands-structure_material"></a>

## `cc_engine::commands::structure_material`

<sub>`crates/cc_engine/src/commands/structure_material.rs`</sub>

Column and beam types by structural material (concrete, steel, timber):
the first type of a material, else a default made from the profile
catalog (steel `W200x26,6`, timber `MLC 9x32`) or a 20 x 40 concrete
rectangle, in one undo step.

**Items:** [`material_arg`](#cc_engine-commands-structure_material-material_arg) · [`member_category`](#cc_engine-commands-structure_material-member_category) · [`structural_type`](#cc_engine-commands-structure_material-structural_type) · [`catalog_type`](#cc_engine-commands-structure_material-catalog_type) · [`catalog_type_turned`](#cc_engine-commands-structure_material-catalog_type_turned) · [`catalog_type_placed`](#cc_engine-commands-structure_material-catalog_type_placed) · [`StructuralTypeFor`](#cc_engine-commands-structure_material-structuraltypefor) · [`type_or_row`](#cc_engine-commands-structure_material-type_or_row) · [`type_or_turned_row`](#cc_engine-commands-structure_material-type_or_turned_row)

<a id="cc_engine-commands-structure_material-material_arg"></a>

### fn `material_arg`

```rust
pub(crate) fn material_arg(code: &str) -> Result<StructuralMaterial>
```

<a id="cc_engine-commands-structure_material-member_category"></a>

### fn `member_category`

```rust
pub(crate) fn member_category(code: &str) -> Result<Category>
```

<a id="cc_engine-commands-structure_material-structural_type"></a>

### fn `structural_type`

```rust
pub(crate) fn structural_type(s: &mut Session, category: Category, material: StructuralMaterial) -> Result<(ElementTypeId, bool)>
```

The type of `category` new `material` members get: the first one of the
material, else a default made in one undo step. Returns it and whether it was made.

<a id="cc_engine-commands-structure_material-catalog_type"></a>

### fn `catalog_type`

```rust
pub(crate) fn catalog_type(s: &mut Session, category: Category, series: &str, designation: &str) -> Result<ElementTypeId>
```

The first `category` type whose profile is catalog row `designation` of
`series`, its profile not turned, else one made from the row (the
profile imported when the project lacks it). What it makes takes undo
steps of its own: callers squash them into theirs.

<a id="cc_engine-commands-structure_material-catalog_type_turned"></a>

### fn `catalog_type_turned`

```rust
pub(crate) fn catalog_type_turned(s: &mut Session, category: Category, row: (&str, &str), rotation_deg: f64) -> Result<ElementTypeId>
```

`catalog_type` with the profile turned `rotation_deg` (90 lays a
beam's section on its side: its depth runs across, its width up).

<a id="cc_engine-commands-structure_material-catalog_type_placed"></a>

### fn `catalog_type_placed`

```rust
pub(crate) fn catalog_type_placed(s: &mut Session, category: Category, (series, designation): (&str, &str), rotation_deg: f64, mirror: bool) -> Result<ElementTypeId>
```

`catalog_type_turned` with the profile also mirrored when `mirror`
(before it is turned: a channel laid on its side with 90 opens upward,
and downward when mirrored).

<a id="cc_engine-commands-structure_material-structuraltypefor"></a>

### struct `StructuralTypeFor`

```rust
pub struct StructuralTypeFor
```

The column or beam type new members of a structural material get: the
first type of that material, else a default made in one undo step
(steel `W200x26,6`, timber `MLC 9x32` from the profile catalog, concrete
a 0.20 x 0.40 rectangle named `20x40`). Returns its `id`, `name` and
whether it was `created`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `category: String`

  `column` or `beam`.

- `material: String`

  `concrete`, `steel` or `timber`.


<a id="cc_engine-commands-structure_material-type_or_row"></a>

### fn `type_or_row`

```rust
pub(crate) fn type_or_row(s: &mut Session, given: Option<ElementTypeId>, category: Category, (series, row): (&str, &str)) -> Result<ElementTypeId>
```

The given type, else the catalog row's (made when missing).

<a id="cc_engine-commands-structure_material-type_or_turned_row"></a>

### fn `type_or_turned_row`

```rust
pub(crate) fn type_or_turned_row(s: &mut Session, given: Option<ElementTypeId>, category: Category, row: (&str, &str), rotation_deg: f64) -> Result<ElementTypeId>
```

`type_or_row` with the row's type turned `rotation_deg` (see `catalog_type_turned`).

<a id="mod-cc_engine-commands-sun"></a>

## `cc_engine::commands::sun`

<sub>`crates/cc_engine/src/commands/sun.rs`</sub>

The sun of a view (see `cc_core::sun`): its settings as commands show
and take them (`view_get`, `view_set`), where it stands, the frames of a
study, and the project's named presets.

**Items:** [`SunArg`](#cc_engine-commands-sun-sunarg) · [`apply_sun`](#cc_engine-commands-sun-apply_sun) · [`mode_json`](#cc_engine-commands-sun-mode_json) · [`sun_json`](#cc_engine-commands-sun-sun_json) · [`sun_arg`](#cc_engine-commands-sun-sun_arg) · [`view_sun_json`](#cc_engine-commands-sun-view_sun_json) · [`SunPosition`](#cc_engine-commands-sun-sunposition) · [`utc_offset`](#cc_engine-commands-sun-utc_offset) · [`SunFrames`](#cc_engine-commands-sun-sunframes) · [`SunPresetsList`](#cc_engine-commands-sun-sunpresetslist) · [`SunPresetSave`](#cc_engine-commands-sun-sunpresetsave) · [`SunPresetRename`](#cc_engine-commands-sun-sunpresetrename) · [`SunPresetDelete`](#cc_engine-commands-sun-sunpresetdelete)

<a id="cc_engine-commands-sun-sunarg"></a>

### struct `SunArg`

```rust
pub struct SunArg
```

A view's sun as commands take it (`view_set` `sun`, `sun_preset_save`):
every field is optional and changes only what it names. The fields a
mode has: `still` `date` and `time`; `single_day` `date`, `from`, `to`,
`sunrise_to_sunset` and `interval`; `multi_day` `from_date`, `to_date`,
`from`, `to`, `sunrise_to_sunset` and `interval`; `lighting` `azimuth`,
`altitude` and `relative_to_view`. `frame`, `show_path`, `shadows` and
`ground` belong to every mode. Dates and times are local to the site.

**Traits:** `Debug`, `Clone`, `Default`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `preset: Option<String>`

  Take the mode and its values from this project preset (by name); the other fields given apply on top.

- `mode: Option<String>`

  `still` (one moment), `single_day` (a study over one day), `multi_day`
  (a study over several days), `lighting` (a light from a direction
  given by hand) or `conventional`: the façade drawing's sun, a
  lighting relative to the view (azimuth 225, altitude atan(1/√2) ≈
  35.26) whose rays run along a cube's diagonal from over the viewer's
  left shoulder, so a shadow falls down and to the right as far as its
  caster stands in front of the face (`view_get` shows it as
  `lighting`, with `derived.conventional`). Another mode starts from
  its defaults, keeping the date; `conventional` always resets the
  light's direction.

- `date: Option<String>`

  Still and single day: the date, `YYYY-MM-DD`.

- `time: Option<String>`

  Still: the time of day, `HH:MM`.

- `from_date: Option<String>`

  Multi day: the first date, `YYYY-MM-DD`.

- `to_date: Option<String>`

  Multi day: the last date, `YYYY-MM-DD` (not before `from_date`).

- `from: Option<String>`

  Single and multi day: the first time of each day, `HH:MM` (with a
  day, week or month step, the time of every frame).

- `to: Option<String>`

  Single and multi day: the last time of each day, `HH:MM` (not before `from`).

- `sunrise_to_sunset: Option<bool>`

  Single and multi day: from sunrise to sunset instead of `from` to `to`
  (12:00 with a day, week or month step).

- `interval: Option<String>`

  Single and multi day: the step between frames: `minutes_15`,
  `minutes_30`, `minutes_45` or `hour`; several days also take `day`,
  `week` or `month`.

- `azimuth: Option<f64>`

  Lighting: degrees clockwise seen from above, from true north or, with
  `relative_to_view`, from the direction the view looks along (any
  number, kept within 0 to 360).

- `altitude: Option<f64>`

  Lighting: degrees above the horizon, 0 to 90.

- `relative_to_view: Option<bool>`

  Lighting: measure the azimuth from the direction the view looks along (default true).

- `frame: Option<u32>`

  The frame of a study shown, from 0 (kept within the frames when read).

- `show_path: Option<bool>`

  Draw the sun path diagram.

- `shadows: Option<bool>`

  Cast shadows: in 3D views, and in plans, elevations and sections
  drawn in the shaded, consistent colors or textures style.

- `ground: Option<Option<Ref>>`

  The level whose plane receives the shadows (id or name), or `null` for the lowest level.


<a id="cc_engine-commands-sun-apply_sun"></a>

### fn `apply_sun`

```rust
pub(crate) fn apply_sun(p: &Project, current: Option<&SunSettings>, arg: &SunArg) -> Result<SunSettings>
```

`current` (or the default sun) changed by `arg`, checked: unknown names,
unreadable dates and times, fields the mode does not have and values out
of range are refused, never repaired.

<a id="cc_engine-commands-sun-mode_json"></a>

### fn `mode_json`

```rust
pub fn mode_json(mode: &SunMode) -> Map<String, Value>
```

A mode in the friendly form: `mode` and the fields it has.

<a id="cc_engine-commands-sun-sun_json"></a>

### fn `sun_json`

```rust
pub fn sun_json(project: &Project, sun: &SunSettings) -> Value
```

A view's sun in the friendly form (what `view_get` shows and `view_set` takes).

<a id="cc_engine-commands-sun-sun_arg"></a>

### fn `sun_arg`

```rust
pub fn sun_arg(project: &Project, sun: &SunSettings) -> SunArg
```

The argument that makes `sun` from any settings: every field of its
mode and the common ones (the inverse of applying a `SunArg`).

<a id="cc_engine-commands-sun-view_sun_json"></a>

### fn `view_sun_json`

```rust
pub(crate) fn view_sun_json(p: &Project, v: &View) -> Value
```

A view's sun as `view_get` shows it: `null`, or the friendly form plus
`derived` (where it stands for the current frame).

<a id="cc_engine-commands-sun-sunposition"></a>

### struct `SunPosition`

```rust
pub struct SunPosition
```

Where the sun stands: for a view with a sun, at its current frame (or
its lighting direction); else on `date` at `time` (local) at the
project's site, or at `latitude`, `longitude` and `utc_offset` given
here (a longitude given without an offset takes its nominal time zone).
Returns the azimuth (degrees clockwise from true north), the altitude
(degrees above the horizon, no refraction), `above_horizon`, the unit
`direction` towards the sun in project coordinates (true north
applied), sunrise and sunset that day (`HH:MM`, or null with `polar`
`day` or `night`), and `site_known` (false: the equator at Greenwich).

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Option<Ref>`

  A view with a sun (id or name); or give `date` and `time`.

- `date: Option<String>`

  The date, `YYYY-MM-DD`.

- `time: Option<String>`

  The local time of day, `HH:MM`.

- `latitude: Option<f64>`

  Latitude, degrees (-90 to 90, north positive), instead of the site's.

- `longitude: Option<f64>`

  Longitude, degrees (-180 to 180, east positive), instead of the site's.

- `utc_offset: Option<f64>`

  Local time's offset from UTC, hours (-12 to 14), instead of the site's.


<a id="cc_engine-commands-sun-utc_offset"></a>

### fn `utc_offset`

```rust
pub(crate) fn utc_offset(v: f64) -> Result<f64>
```

An offset from UTC within range.

<a id="cc_engine-commands-sun-sunframes"></a>

### struct `SunFrames`

```rust
pub struct SunFrames
```

The frames of a view's sun study, in time order: `count` (all of them,
at most 5000; 0 for lighting, 1 for a still sun) and the first `limit`
as `{index, date, time, azimuth, altitude}`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  A view with a sun (id or name).

- `limit: Option<usize>`

  Most frames listed (default 500, at most 5000).


<a id="cc_engine-commands-sun-sunpresetslist"></a>

### struct `SunPresetsList`

```rust
pub struct SunPresetsList
```

The project's sun presets: each `name` with its mode in the form
`view_set` `sun` takes (a view takes one with `"sun": {"preset": name}`).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

<a id="cc_engine-commands-sun-sunpresetsave"></a>

### struct `SunPresetSave`

```rust
pub struct SunPresetSave
```

Save a sun preset, in one undo step: the mode and its values (dates,
times, step or light direction; not the frame, path, shadows or
ground) of a view's sun, or of `sun` applied to the default sun. A
preset of the same name is replaced only with `replace`; a project
keeps at most 200.

**Traits:** `Debug`, `Default`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  Name of the preset.

- `view: Option<Ref>`

  A view whose sun is saved (id or name); or give `sun`.

- `sun: Option<SunArg>`

  The sun to save, as `view_set` takes it.

- `replace: bool`

  Replace a preset of the same name (default false: refused).


<a id="cc_engine-commands-sun-sunpresetrename"></a>

### struct `SunPresetRename`

```rust
pub struct SunPresetRename
```

Rename a sun preset, in one undo step.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  The preset's name.

- `new_name: String`

  Its new name (no other preset's).


<a id="cc_engine-commands-sun-sunpresetdelete"></a>

### struct `SunPresetDelete`

```rust
pub struct SunPresetDelete
```

Delete a sun preset, in one undo step (views that took it keep their sun).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  The preset's name.


<a id="mod-cc_engine-commands-sweep"></a>

## `cc_engine::commands::sweep`

<sub>`crates/cc_engine/src/commands/sweep.rs`</sub>

Swept profiles: a profile (a closed outline held by the type) carried
along a path in plan, for baseboards, crown moldings, fascias, gutters and
copings. Creating them, creating their types and redrawing a type's profile.

**Items:** [`new_sweep_type`](#cc_engine-commands-sweep-new_sweep_type) · [`SweepCreate`](#cc_engine-commands-sweep-sweepcreate) · [`SweepTypeCreate`](#cc_engine-commands-sweep-sweeptypecreate) · [`SweepProfileSet`](#cc_engine-commands-sweep-sweepprofileset)

<a id="cc_engine-commands-sweep-new_sweep_type"></a>

### fn `new_sweep_type`

```rust
pub fn new_sweep_type(p: &Project, id: ElementTypeId) -> ElementType
```

The sweep type a project gets with its first sweep: a baseboard (10 cm
tall, 4" in imperial projects; see `cc_core::Defaults::baseboard`).

<a id="cc_engine-commands-sweep-sweepcreate"></a>

### struct `SweepCreate`

```rust
pub struct SweepCreate
```

Create a swept profile: the profile of its type carried along `path` in
plan, `base_offset` above the level, on the left of the path walking
along it (`flip` puts it on the right); `closed` runs the path back to
its first point (around a room). A project without a sweep type gets
one (a baseboard).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `path: Vec<Point>`

  Points of the path in plan (at least 2; 3 when closed).

- `closed: bool`

  Run the path back to its first point.

- `level: Option<Ref>`

  Level id or name (default: the lowest level).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).

- `base_offset: Option<f64>`

  Height of the path above the level, meters (default 0).

- `flip: bool`

  Put the profile on the right of the path.


<a id="cc_engine-commands-sweep-sweeptypecreate"></a>

### struct `SweepTypeCreate`

```rust
pub struct SweepTypeCreate
```

Create a sweep type: its profile drawn as points `[x, y]` (x across, to
the left of the path; y up from it, meters) or as `curves` (lines, arcs,
circles, rectangles… making one closed outline), shown in `material`.
Without a profile it is a baseboard 1.5 cm × 10 cm.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's baseboard name).

- `profile: Option<Vec<Point>>`

  The profile's points, in order (a closed outline that does not cross itself).

- `curves: Option<Vec<CurveArg>>`

  The profile drawn with curves instead of points.

- `material: Option<String>`

  Material name (default: the project's first material).


<a id="cc_engine-commands-sweep-sweepprofileset"></a>

### struct `SweepProfileSet`

```rust
pub struct SweepProfileSet
```

Redraw the profile of a sweep type, as points `[x, y]` or `curves`; every
sweep of the type follows. The type keeps the curves (`type_get` shows
them as its `sketch`), so arcs stay arcs when the profile is edited again.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

  Sweep type id or name.

- `profile: Option<Vec<Point>>`

  The profile's points, in order.

- `curves: Option<Vec<CurveArg>>`

  The profile drawn with curves instead of points.


<a id="mod-cc_engine-commands-topography"></a>

## `cc_engine::commands::topography`

<sub>`crates/cc_engine/src/commands/topography.rs`</sub>

Terrain: topography surfaces from measured points inside a boundary
(see `cc_core::topography`).

**Items:** [`Point3`](#cc_engine-commands-topography-point3) · [`checked_points`](#cc_engine-commands-topography-checked_points) · [`new_topography_type`](#cc_engine-commands-topography-new_topography_type) · [`TopographyCreate`](#cc_engine-commands-topography-topographycreate) · [`PointElevation`](#cc_engine-commands-topography-pointelevation) · [`TopographyPointsSet`](#cc_engine-commands-topography-topographypointsset) · [`TopographyTypeCreate`](#cc_engine-commands-topography-topographytypecreate) · [`derived`](#cc_engine-commands-topography-derived)

<a id="cc_engine-commands-topography-point3"></a>

### type alias `Point3`

```rust
pub type Point3 = [f64; 3]
```

A measured point `[x, y, z]`: plan position and elevation above the level, meters.

<a id="cc_engine-commands-topography-checked_points"></a>

### fn `checked_points`

```rust
pub(crate) fn checked_points(list: &[Point3]) -> Result<Vec<DVec3>>
```

A terrain's new points: at most `MAX_POINTS`, finite, and those that
repeat another in plan dropped.

<a id="cc_engine-commands-topography-new_topography_type"></a>

### fn `new_topography_type`

```rust
pub fn new_topography_type(id: ElementTypeId, material: String, system: cc_core::MeasurementSystem) -> ElementType
```

A terrain type for a project of `system` that has none, in `material`
(contours every meter, every foot in imperial projects).

<a id="cc_engine-commands-topography-topographycreate"></a>

### struct `TopographyCreate`

```rust
pub struct TopographyCreate
```

Create a terrain surface (topography): measured `points` `[x, y, z]`
inside a closed `boundary` (or one drawn with `curves`; with neither,
the convex hull of the points). The points are triangulated with the
boundary, whose vertices take the elevations the points give them; the
earth closes down to a flat base `base_depth` below the lowest point.
Points outside the boundary are kept: they give the boundary its
elevations but are not vertices of the surface.
A project without a terrain type gets one (and an earth material).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `points: Vec<Point3>`

  Measured points `[[x, y, z], …]`: plan position and elevation above the level, meters.

- `boundary: Vec<Point>`

  Boundary `[[x, y], …]` in plan, closed (at least three points); or `curves`.

- `curves: Option<Vec<CurveArg>>`

  The boundary drawn with curves (lines, arcs, circles, rectangles…).

- `level: Option<Ref>`

  Level id or name the elevations are measured from (default: the lowest level).

- `base_depth: Option<f64>`

  Depth of the flat base below the lowest point of the surface, meters (default 2; 6'-0" in imperial
  projects).

- `type_ref: Option<Ref>`

  Type id or name (default: the category's default type).


<a id="cc_engine-commands-topography-pointelevation"></a>

### struct `PointElevation`

```rust
pub struct PointElevation
```

A new elevation for one point.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Clone`, `Copy`

**Fields**

- `index: usize`

  Index of the point in the surface's `points`.

- `z: f64`

  Elevation above the level, meters.


<a id="cc_engine-commands-topography-topographypointsset"></a>

### struct `TopographyPointsSet`

```rust
pub struct TopographyPointsSet
```

Edit the measured points of a terrain surface: replace them all
(`points`), or change elevations (`set`), remove points (`remove`, by
index) and add new ones (`add`), in that order. Indices refer to the
points before `remove` and `add`: the current ones, or `points` when it
is given. Points that repeat another in plan
are dropped (the first is kept); the answer says how many.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `id: u64`

  Terrain element id.

- `points: Option<Vec<Point3>>`

  All the points `[[x, y, z], …]`, replacing the current ones.

- `set: Vec<PointElevation>`

  New elevations `[{index, z}, …]`.

- `remove: Vec<usize>`

  Indices of points to remove.

- `add: Vec<Point3>`

  Points `[[x, y, z], …]` to add.


<a id="cc_engine-commands-topography-topographytypecreate"></a>

### struct `TopographyTypeCreate`

```rust
pub struct TopographyTypeCreate
```

Create a terrain type: the earth `material` sections cut it in, and its
contour lines every `contour_interval` meters, every `major_every`-th a
major (heavier) one.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `name: Option<String>`

  Type name (default: the template's terrain name).

- `material: Option<String>`

  Material name (default: the project's earth material, added if missing).

- `contour_interval: Option<f64>`

  Spacing of the contour lines, meters (default 1; 1'-0" in imperial projects).

- `major_every: Option<u32>`

  Every this many contours, a major one (default 5).


<a id="cc_engine-commands-topography-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(g: &cc_geom::topography::TopoGeom, derived: &mut serde_json::Map<String, Value>)
```

The figures `element_get` shows for a terrain surface, as `topography`.

<a id="mod-cc_engine-commands-types"></a>

## `cc_engine::commands::types`

<sub>`crates/cc_engine/src/commands/types.rs`</sub>

Types and components: change, duplicate, delete, create components.

**Items:** [`validate`](#cc_engine-commands-types-validate) · [`type_out`](#cc_engine-commands-types-type_out) · [`TypeSet`](#cc_engine-commands-types-typeset) · [`TypeDuplicate`](#cc_engine-commands-types-typeduplicate) · [`TypeDelete`](#cc_engine-commands-types-typedelete) · [`ComponentCreate`](#cc_engine-commands-types-componentcreate) · [`starter_def`](#cc_engine-commands-types-starter_def)

<a id="cc_engine-commands-types-validate"></a>

### fn `validate`

```rust
pub(crate) fn validate(def: &ComponentDef) -> Result<()>
```

Parameter names valid and unique, sound reference planes, lines and
dimensions, the right kind of pieces (a title block: sheet pieces only,
within their limits; a detail component at least one), every piece
resolves. A component may have no pieces while it is shaped: it starts
empty, and `component_place` refuses it until it has one.

<a id="cc_engine-commands-types-type_out"></a>

### fn `type_out`

```rust
pub(crate) fn type_out(s: &Session, id: ElementTypeId) -> Value
```

<a id="cc_engine-commands-types-typeset"></a>

### struct `TypeSet`

```rust
pub struct TypeSet
```

Change a type: its name, its parameters (`params`, merged into what
`type_get` shows: layers, sizes, materials, a component's `def`…) and its
identity data (`info`: type mark, description, manufacturer, cost…).
A column or beam type takes `profile` (a structural profile's name or id;
null is refused: these types always point to a profile),
`profile_rotation_deg` and `profile_mirror`. `width`/`height` are the box of
the turned profile; changing them alone re-points the type to a profile of
that section, unturned and unmirrored.
Changing a component's definition updates every type of that component,
each keeping its own parameter values.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`
- `name: Option<String>`

  New name (unique in its category).

- `params: Option<Value>`

  Fields of the type's parameters to change, shaped like `params` in `type_get`.

- `info: Option<Value>`

  Fields of the identity data to change, shaped like `info` in `type_get`.

- `renames: Vec<[String; 2]>`

  Component parameters renamed by a new `def`, as `[old, new]` pairs in
  order, so the component's other types keep their values.


<a id="cc_engine-commands-types-typeduplicate"></a>

### struct `TypeDuplicate`

```rust
pub struct TypeDuplicate
```

Copy a type under a new name (default "<name> (copy)"); the given elements of the type switch to the copy.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`
- `name: Option<String>`

  Name of the copy (default: "<name> (copy)").

- `elements: Vec<u64>`

  Elements of the type to switch to the copy.


<a id="cc_engine-commands-types-typedelete"></a>

### struct `TypeDelete`

```rust
pub struct TypeDelete
```

Delete a type that no element uses (the last type of a category is kept,
except a title block: sheets on a deleted title block move to the first
other one, or to none).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `type_ref: Ref`

<a id="cc_engine-commands-types-componentcreate"></a>

### struct `ComponentCreate`

```rust
pub struct ComponentCreate
```

Create a component type: `def` holds its parameters and pieces, shaped
like a component type's `params.def` in `type_get`. A piece's `shape` is
a `Box` {x, y, z, width, depth, height}, a `Cylinder` {x, y, z, radius,
height}, an `Extrusion` {points: [[x, y], …], z, height} (a plan outline
extruded up), a `Profile` {plane: `Plan`, `Front` or `Side`, points:
[{u, v, radius}, …], offset, depth} (lines and arcs drawn on a plane:
u, v are x, y in plan, x, z from the front and y, z from the side; the
plane stands at `offset` on the remaining axis and extrudes `depth` along
it; a point's optional `radius` makes the segment to the next point an
arc, positive bulging outwards) or a `Revolution` {plane, points, offset, axis, angle}
(the profile turned about the line u = axis of its plane). Every field
is an expression over the parameters (meters, degrees). A piece with
`void: true` is cut out of the solid pieces it overlaps; one with
`plan_only: true` is a plan symbol (a door's swing), drawn in plans and
not a solid. A `kind` of
`Door` or `Window` with an `opening` {width, height, sill, own_symbol,
operation, door_operation} (expressions, then a window's `operation`:
`Fixed`, `SideHungLeft`, `SideHungRight` or `DoubleSideHung`, drawn as
swings in plans and written as IFC panel properties, and a door's
`door_operation`: `SingleSwingLeft` (default), `SingleSwingRight` or
`DoubleDoor`, one swing per leaf in plans and the IFC operation type;
`door_type_create` makes the built-in ones) makes a door or window type,
placed with `door_create` or `window_create`: it cuts that opening in its
wall; its frame has x along the wall from the opening's center, y across
it (towards the side a door or window opens to), z up from the sill, and
`WallThickness` is the wall's thickness.
`connectors` [{name, system: `ColdWater`, `HotWater`, `Sanitary`, `Vent`,
`Rainwater`, `Power` or `Lighting`, x, y, z, direction: `Down`, `Up`,
`Front` (-y), `Back` (+y), `Left` (-x) or `Right` (+x), diameter}] are
where `mep_connect` joins pipes and conduits (a toilet's inlet and outlet).
With a `kind` in place of `def` the component starts from that kind's
own parameters and pieces, to be shaped in the component environment or
with the `component_*` commands.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `name: String`

  Name of the component and of its first type.

- `def: Value`

  The definition: `params` (name, value, kind, instance) and `items` (pieces), shaped like a component type's `params.def` in `type_get`. Left out, the component starts from its `kind`.

- `kind: Option<String>`

  The kind a component made without a `def` starts from: `Furniture`, `Casework`, `Plumbing` and `Electrical` (a box sized by width, depth and height parameters), `Lighting` (a cylinder sized by diameter and height), `Door` (width and height) and `Window` (width, height and sill), which cut their opening in a wall, or `Generic` (the default: a fixed box and no parameters, which are yours to add). Not given together with `def`, whose own `kind` says it.


<a id="cc_engine-commands-types-starter_def"></a>

### fn `starter_def`

```rust
pub fn starter_def(kind: ComponentKind) -> Option<ComponentDef>
```

The definition a new component of `kind` starts from: the kind's own
parameters, named in the current language, and pieces sized by them. A
generic component has no parameters (they are the user's to add) and one
fixed box, since a component needs a piece. `None` for the 2D kinds,
which are made by `title_block_create` and `detail_component_create`.

<a id="mod-cc_engine-commands-valve"></a>

## `cc_engine::commands::valve`

<sub>`crates/cc_engine/src/commands/valve.rs`</sub>

Valves on pipes: inline accessories hosted by a pipe at a distance along
its run (see `cc_core::mep::Valve`). A valve takes its pipe's diameter
and system, is a node of its network, and moves, is copied and is
deleted with the pipe.

**Items:** [`parse_kind`](#cc_engine-commands-valve-parse_kind) · [`new_valve_type`](#cc_engine-commands-valve-new_valve_type) · [`ValvePlace`](#cc_engine-commands-valve-valveplace) · [`ValveTypeCreate`](#cc_engine-commands-valve-valvetypecreate)

<a id="cc_engine-commands-valve-parse_kind"></a>

### fn `parse_kind`

```rust
pub(crate) fn parse_kind(name: &str) -> Result<ValveKind>
```

<a id="cc_engine-commands-valve-new_valve_type"></a>

### fn `new_valve_type`

```rust
pub fn new_valve_type(id: ElementTypeId, kind: ValveKind, material: String) -> ElementType
```

A valve type of `kind`, named after it.

<a id="cc_engine-commands-valve-valveplace"></a>

### struct `ValvePlace`

```rust
pub struct ValvePlace
```

Place a valve on a pipe, at `point` (the nearest point of the pipe in
plan, within 0.5 m) or `distance` meters along the pipe from its start
(default: its middle). `type` picks a valve type; else `kind` (`gate`,
`ball`, `check` or `globe`) picks the first type of that kind, created
when the project has none; else the first valve type (a gate valve type
is created when there is none). The valve takes the pipe's diameter and
system and splits its network there; it moves, is copied and is deleted
with the pipe.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `pipe: u64`

  Pipe element id.

- `point: Option<Point>`

  A point in plan on (or near) the pipe.

- `distance: Option<f64>`

  Distance from the pipe's start along it, meters.

- `kind: Option<String>`

  `gate`, `ball`, `check` or `globe`.

- `type_ref: Option<Ref>`

  Valve type id or name.


<a id="cc_engine-commands-valve-valvetypecreate"></a>

### struct `ValveTypeCreate`

```rust
pub struct ValveTypeCreate
```

Create valve types, one per kind in `kinds` (`gate`, `ball`, `check`,
`globe`; default: a gate valve), named after their kind (or `name`, for
one kind), in `material` (default: the project's first metal).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `kinds: Vec<String>`

  Kinds of valve to create a type for.

- `name: Option<String>`

  Type name (only with one kind).

- `material: Option<String>`

  Material name.


<a id="mod-cc_engine-commands-views"></a>

## `cc_engine::commands::views`

<sub>`crates/cc_engine/src/commands/views.rs`</sub>

Views and sheets: create, change, duplicate, delete, place on sheets.

**Items:** [`view_out`](#cc_engine-commands-views-view_out) · [`ViewCreate`](#cc_engine-commands-views-viewcreate) · [`ViewSet`](#cc_engine-commands-views-viewset) · [`apply_render`](#cc_engine-commands-views-apply_render) · [`apply_shading`](#cc_engine-commands-views-apply_shading) · [`ViewDuplicate`](#cc_engine-commands-views-viewduplicate) · [`ViewDelete`](#cc_engine-commands-views-viewdelete) · [`sheet_of`](#cc_engine-commands-views-sheet_of) · [`sheet`](#cc_engine-commands-views-sheet) · [`SheetPlaceView`](#cc_engine-commands-views-sheetplaceview) · [`SheetRemoveViewport`](#cc_engine-commands-views-sheetremoveviewport)

<a id="cc_engine-commands-views-view_out"></a>

### fn `view_out`

```rust
pub(crate) fn view_out(s: &Session, id: ViewId) -> Value
```

<a id="cc_engine-commands-views-viewcreate"></a>

### struct `ViewCreate`

```rust
pub struct ViewCreate
```

Create a view. `kind` is `floor_plan` or `ceiling_plan` (with `level`),
`section` (with `start`, `end` and optional `depth`), `elevation` (with
`direction`: north, south, east or west), `structural_plan` (with `level`:
a formwork plan of the structure, tagged), `plumbing_plan`,
`electrical_plan`, `communications_plan` and `lightning_plan` (with
`level`: the pipes, the conduits and devices, the structured cabling, or
the lightning protection, over the architecture, tagged), `three_d`, `analytical` (a 3D
view of the analytical model), `rebar` (a 3D view of the reinforcement), `legend` (all types),
`drafting` (a 2D view independent of the model, showing only the text,
dimensions and imported drawings placed in it; 1:10 by default), `schedule` (with `category`), `panel_diagram` (with `panel`: the
single-line diagram and load schedule of a distribution board; default
the only board) or `sheet` (A3, ANSI B in imperial projects, numbered after the others, with
the first title block type).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `kind: String`

  `floor_plan`, `ceiling_plan`, `structural_plan`, `plumbing_plan`, `electrical_plan`,
  `communications_plan`, `lightning_plan`, `section`, `elevation`, `three_d`, `analytical`, `rebar`, `legend`, `drafting`, `schedule`, `panel_diagram` or `sheet`.

- `name: Option<String>`

  View name (default: by kind).

- `level: Option<Ref>`

  Plans: level id or name (default: the lowest).

- `category: Option<String>`

  Schedules: the category scheduled (default `wall`).

- `start: Option<Point>`

  Sections: start of the cut line.

- `end: Option<Point>`

  Sections: end of the cut line.

- `depth: Option<f64>`

  Section depth beyond the cut line, meters (default 8).

- `direction: Option<String>`

  Elevations: `north`, `south` (default), `east` or `west`.

- `scale: Option<ScaleArg>`

  Scale: the denominator (e.g. 50 for 1:50), or text such as `"1:50"`
  or an imperial label like `"1/4\" = 1'-0\""` (48).

- `panel: Option<u64>`

  Distribution board element id, for a panel diagram.


<a id="cc_engine-commands-views-viewset"></a>

### struct `ViewSet`

```rust
pub struct ViewSet
```

Change a view: name, scale, phase and phase filter, and its `params` as
`view_get` shows them (merged: cut height, section depth, legend types,
schedule fields, sort, filter and totals, sheet number and size,
viewport positions…), the crop, the sun, the render settings and the shading.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `view: Ref`
- `name: Option<String>`

  New name. A plan's name is its own: its level and the level's other
  plans keep theirs.

- `scale: Option<ScaleArg>`

  Scale: the denominator (e.g. 100 for 1:100), or text such as
  `"1:100"` or an imperial label like `"1/8\" = 1'-0\""` (96).

- `params: Option<Value>`

  Fields of the view's parameters to change, shaped like `params` in `view_get`.

- `phase: Option<Ref>`

  Phase the view shows (id or name; plans, elevations, sections, 3D views, schedules).

- `detail: Option<String>`

  Detail level: `coarse` (layered elements cut as one plain region;
  component pieces set for it), `medium` or `fine`.

- `visual_style: Option<String>`

  Visual style: `wireframe` (every edge, no faces), `hidden_line` (white
  faces), `shaded` (white faces greyed by the light, with the sun's
  shadows), `consistent_colors` (material colors, unlit),
  `textures` (appearance colors and surface patterns), or `default`
  (plans hidden line, other views shaded).

- `phase_filter: Option<Option<Ref>>`

  Phase filter (id or name), or `null` for none (every status by category).

- `crop: Option<Option<[Point; 2]>>`

  Crop rectangle: two opposite corners in the view's drawing
  coordinates (plans, sections and elevations draw nothing of the model
  outside it), or `null` to show everything. For a detail view this is
  its callout in the parent; for a drafting view a callout points to,
  it moves that callout.

- `sun: Option<Option<super::sun::SunArg>>`

  The view's sun (plans, ceiling plans, 3D views, elevations and
  sections): the fields to change, shaped like `sun` in `view_get`
  (a view without a sun starts from a still sun on 2026-06-21 at
  10:00), `{"preset": name}` to take a preset, or `null` to remove it.
  With `shadows`, the sun casts shadows in 3D views and, in the
  shaded, consistent colors and textures styles, in plans, elevations
  and sections (a floor plan from the whole model, a section from
  what lies beyond its line, neither onto what is indoors; ceiling
  plans draw none).

- `render: Option<Option<Value>>`

  The view's photorealistic render settings (3D views only): the fields
  to change, shaped like `render` in `view_get` (`samples` 1 to 16384,
  `max_depth` 1 to 64, `width`/`height` 16 to 8192 pixels and at most
  7680x4320 in all, `fov_deg` 10 to 120, `exposure` -10 to 10 stops,
  `tone_map` `Filmic` or `Standard` (or lowercase), `clamp` 0 (off) to 1000, `clay`,
  and `environment` with `sky_strength` and `sun_strength` 0 to 100,
  `ground_color` [r, g, b] and `ground_plane`); a view without settings
  starts from the defaults. `null` removes them.

- `shading: Option<Value>`

  How the view shades faces and casts shadows in the shaded,
  consistent colors and textures styles (plans, ceiling plans, 3D
  views, elevations and sections; not photorealistic renders): the
  fields to change, shaped like `shading` in `view_get`: `ambient`,
  the light of a face turned away from the light, 0.3 (dark grey) to
  1 (no shading; default 0.6), and `shadow`, how dark cast shadows
  are, 0 (none) to 1 (black; default 0.55, a shaded shadow's grey
  being `1 - shadow`).


<a id="cc_engine-commands-views-apply_render"></a>

### fn `apply_render`

```rust
pub(crate) fn apply_render(current: Option<&RenderSettings>, patch: &Value) -> Result<RenderSettings>
```

The render settings `patch` makes of `current` (the defaults when none):
a merge patch over their serde form, refused when it names an unknown
field or leaves a value out of range.

<a id="cc_engine-commands-views-apply_shading"></a>

### fn `apply_shading`

```rust
pub(crate) fn apply_shading(current: &ShadingSettings, patch: &Value) -> Result<ShadingSettings>
```

The shading `patch` makes of `current`: a merge patch over its serde
form, refused when it names an unknown field or leaves a value out of range.

<a id="cc_engine-commands-views-viewduplicate"></a>

### struct `ViewDuplicate`

```rust
pub struct ViewDuplicate
```

Copy a view (a copied sheet starts without viewports), in one undo step.
With `detailing`, what is drawn in it comes along: detail lines, filled
and masking regions, text notes, dimensions (still measuring the same
elements), level marks, detail components and imported drawings; the
result lists them in `copied`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

  The view to copy (id or name).

- `detailing: bool`

  Copy the elements drawn in the view too (default: only the view).


<a id="cc_engine-commands-views-viewdelete"></a>

### struct `ViewDelete`

```rust
pub struct ViewDelete
```

Delete a view (and its viewports on sheets; dimensions drawn in it go too).

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `view: Ref`

<a id="cc_engine-commands-views-sheet_of"></a>

### fn `sheet_of`

```rust
pub fn sheet_of(project: &Project, view: ViewId) -> Option<ViewId>
```

The sheet that shows `view`, if any.

<a id="cc_engine-commands-views-sheet"></a>

### fn `sheet`

```rust
pub(crate) fn sheet(p: &Project, r: &Ref) -> Result<ViewId>
```

<a id="cc_engine-commands-views-sheetplaceview"></a>

### struct `SheetPlaceView`

```rust
pub struct SheetPlaceView
```

Place a view on a sheet at `center` (paper millimeters; default left of
center). A view can be on only one sheet; sheets cannot be placed.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `sheet: Ref`
- `view: Ref`
- `center: Option<Point>`

  Center of the viewport on the sheet, paper millimeters.


<a id="cc_engine-commands-views-sheetremoveviewport"></a>

### struct `SheetRemoveViewport`

```rust
pub struct SheetRemoveViewport
```

Remove a view from a sheet.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Command`

**Fields**

- `sheet: Ref`
- `view: Ref`

<a id="mod-cc_engine-commands-wall_framing"></a>

## `cc_engine::commands::wall_framing`

<sub>`crates/cc_engine/src/commands/wall_framing.rs`</sub>

Light steel and wood framing of walls (see `cc_core::wall_framing`):
the host element and the catalog types its members get by default.

**Items:** [`FramingRows`](#cc_engine-commands-wall_framing-framingrows) · [`framing_rows`](#cc_engine-commands-wall_framing-framing_rows) · [`new_wall_framing_type`](#cc_engine-commands-wall_framing-new_wall_framing_type) · [`WallFramingCreate`](#cc_engine-commands-wall_framing-wallframingcreate) · [`derived`](#cc_engine-commands-wall_framing-derived)

<a id="cc_engine-commands-wall_framing-framingrows"></a>

### struct `FramingRows`

```rust
pub struct FramingRows
```

The catalog rows a system's members get by default: (series, designation).

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `stud: (&'static str, &'static str)`

  A column type of this row.

- `track: (&'static str, &'static str)`

  Beam types of these rows: the track turned 90 degrees to lie flat.

- `header: (&'static str, &'static str)`

<a id="cc_engine-commands-wall_framing-framing_rows"></a>

### fn `framing_rows`

```rust
pub fn framing_rows(system: FramingSystem) -> FramingRows
```

<a id="cc_engine-commands-wall_framing-new_wall_framing_type"></a>

### fn `new_wall_framing_type`

```rust
pub fn new_wall_framing_type(id: ElementTypeId) -> ElementType
```

The type a project gets with its first wall framing.

<a id="cc_engine-commands-wall_framing-wallframingcreate"></a>

### struct `WallFramingCreate`

```rust
pub struct WallFramingCreate
```

Frame straight walls in light steel or wood, one framing per wall. Studs
stand every `spacing` from the wall start (plus one flush with each end)
between a bottom track or plate and one top track (two plates for wood).
Each door or window gets king and jack studs, a header, a sill (windows)
and cripples. Blocking at mid-height is optional. The studs are columns
and the rest are beams, all ordinary elements that the framing keeps in
step with its parameters, its wall and the wall's doors and windows
(`element_set` on the framing regenerates them, keeping their ids). A
member edited or deleted by hand is detached and left alone, and
deleting the wall deletes its framing and members. The framing and its
members follow the wall's phases, design option and level. Default types come
from the profile catalog, made when missing:

- light steel: `Ue 90x40x12x0,95` studs, `U 92x38x0,95` tracks and `Ue 140x40x12x0,95` headers;
- wood: `Peça 6x8` studs and plates, and `Peça 6x16` headers.
The tracks and plates lie flat: their default beam type has the profile
turned 90 degrees, so its depth runs across the wall and its width
(38 mm, 6 cm) is their height. A given type is used as it draws: the
height of its section is the height of the track.
Curved walls, walls already framed and out-of-range values are refused
before anything is made. One undo step; returns `framings`, each with its
`id`, `wall` and `members`.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `walls: Vec<u64>`

  Ids of the straight walls to frame (each gets its own framing).

- `system: Option<String>`

  `light_steel` (default) or `wood`.

- `spacing: Option<f64>`

  Stud spacing on center from the wall start, meters, 0.2 to 1.2 (default 0.4 for light steel, 0.6 for wood).

- `stud_type: Option<Ref>`

  Column type of the studs, id or name (default: the system's catalog stud).

- `track_type: Option<Ref>`

  Beam type of the tracks or plates and of the blocking, id or name (default: the system's catalog track or plate).

- `header_type: Option<Ref>`

  Beam type of the headers and sills, id or name (default: the system's catalog header).

- `double_top: Option<bool>`

  An extra top plate (wood only; default on for wood).

- `blocking: Option<bool>`

  A row of blocking at mid-height between the studs (default off).


<a id="cc_engine-commands-wall_framing-derived"></a>

### fn `derived`

```rust
pub(crate) fn derived(p: &cc_core::Project, f: &WallFraming, out: &mut serde_json::Map<String, Value>)
```

`element_get`'s derived values of a framing: its member ids by role
kind, how many roles are detached, its wall's length and whether the
layout would pass the member limit, so it has no members (`at_limit`).

<a id="mod-cc_engine-commands-wall_rebar"></a>

## `cc_engine::commands::wall_rebar`

<sub>`crates/cc_engine/src/commands/wall_rebar.rs`</sub>

Wall reinforcement: horizontal (ring) and vertical bars near each face
of a wall, and ties between the faces.

**Items:** [`ZoneArg`](#cc_engine-commands-wall_rebar-zonearg) · [`HorizontalBars`](#cc_engine-commands-wall_rebar-horizontalbars) · [`EndArg`](#cc_engine-commands-wall_rebar-endarg) · [`VerticalBars`](#cc_engine-commands-wall_rebar-verticalbars) · [`TieBars`](#cc_engine-commands-wall_rebar-tiebars) · [`WallRebarCreate`](#cc_engine-commands-wall_rebar-wallrebarcreate)

<a id="cc_engine-commands-wall_rebar-zonearg"></a>

### struct `ZoneArg`

```rust
pub struct ZoneArg
```

The spacing of horizontal bars up to a height.

**Traits:** `Debug`, `Clone`, `Copy`, `Deserialize`, `Serialize`, `JsonSchema`

**Fields**

- `up_to: f64`

  Height above the wall's base up to which the spacing applies, meters.

- `spacing: f64`

  Distance between the bars that start below that height, meters.


<a id="cc_engine-commands-wall_rebar-horizontalbars"></a>

### struct `HorizontalBars`

```rust
pub struct HorizontalBars
```

A wall's horizontal bars (ring bars in an arc wall).

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type nearest 10 mm; 1/2" in imperial projects).

- `spacing: Option<f64>`

  Distance between the bars up the wall, meters (default 0.15; 6" in imperial projects).

- `zones: Vec<ZoneArg>`

  Instead of one `spacing`: spacings by height from the base, closest at the bottom, for
  example `[{"up_to": 2, "spacing": 0.10}, {"up_to": 4, "spacing": 0.12}, {"up_to": 6,
  "spacing": 0.15}]`; the last runs to the top of the wall.


<a id="cc_engine-commands-wall_rebar-endarg"></a>

### struct `EndArg`

```rust
pub struct EndArg
```

How an end of a vertical bar is bent.

**Traits:** `Debug`, `Clone`, `Copy`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `across: f64`

  Leg across the wall, meters: towards the outside of the bar's own face when positive
  (into the slab beyond it), towards the other face when negative.

- `along: f64`

  A second leg after it, meters: back beside the bar when positive (up again at the
  bottom end), on past its end when negative. 0 for an L hook.


<a id="cc_engine-commands-wall_rebar-verticalbars"></a>

### struct `VerticalBars`

```rust
pub struct VerticalBars
```

A wall's vertical bars.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type nearest 10 mm; 1/2" in imperial projects).

- `spacing: Option<f64>`

  Distance between the bars along the wall, on its centerline, meters (default 0.15; 6"
  in imperial projects).

- `offset: Option<f64>`

  Distance of the first bar after the start of the wall, meters (default 0): half the
  spacing puts the bars of an additional set between those of the first.

- `from: Option<f64>`

  Height above the base the bars start at, meters (default: the base).

- `to: Option<f64>`

  Height above the base the bars end at, meters (default: the top): `1.5` makes the
  additional bars of the bottom 1.5 m.

- `bottom: Option<EndArg>`

  The bottom end (default: straight).

- `top: Option<EndArg>`

  The top end (default: straight).


<a id="cc_engine-commands-wall_rebar-tiebars"></a>

### struct `TieBars`

```rust
pub struct TieBars
```

Ties between the two faces of a wall.

**Traits:** `Debug`, `Clone`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Fields**

- `type_ref: Option<Ref>`

  Bar type id or name (default: the type nearest 5 mm; 3/8" in imperial projects).

- `every_horizontal: Option<u32>`

  A tie at every this many horizontal bars (default 1: every one).

- `every_vertical: Option<u32>`

  A tie at every this many vertical bars (default 2: every other one).

- `hooks: Option<[f64; 2]>`

  The angles the tie's two ends are hooked by, degrees, 90 to 180: at the exterior
  face, then the interior (default `[135, 135]`; `[135, 90]` or `[135, 180]` are
  common). The hooks count in the tie's length.


<a id="cc_engine-commands-wall_rebar-wallrebarcreate"></a>

### struct `WallRebarCreate`

```rust
pub struct WallRebarCreate
```

Reinforce walls, straight or arc: `horizontal` bars up the wall (ring
bars in an arc wall, each as long as the arc at its own radius) and
`vertical` bars along it near each face, and `ties` between the faces.
With none of the three given, horizontal and vertical bars every 15 cm
on both faces. Bars lie `cover` inside the faces of the wall's structure
(its core layers), stop at its ends and around its doors and windows,
and those longer than `max_length` are lap spliced, neighbouring splices
staggered. Horizontal bars may close up towards the base (`zones`);
vertical bars may cover part of the height, start between those of
another set (`offset`) and have bent ends. Each call adds sets, so a
second call makes the additional bars of a zone. One set per family and
face, in one undo step; the bars follow the wall.

**Traits:** `Debug`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`, `Command`

**Fields**

- `walls: Vec<u64>`

  Wall ids (default: the structural walls of `level`, or of the project).

- `level: Option<Ref>`

  Level id or name, when no walls are given.

- `faces: Option<String>`

  The faces reinforced: `both` (default), `exterior` or `interior`.

- `cover: Option<f64>`

  Concrete cover of both faces, meters (default 0.025; 1 1/2" in imperial projects).

- `cover_exterior: Option<f64>`

  Cover of the exterior face, when it differs.

- `cover_interior: Option<f64>`

  Cover of the interior face, when it differs.

- `horizontal: Option<HorizontalBars>`

  The horizontal bars.

- `vertical: Option<VerticalBars>`

  The vertical bars.

- `ties: Option<TieBars>`

  The ties between the faces, at the horizontal and vertical bars of the exterior face.

- `outer: Option<String>`

  The bars nearest the faces: `horizontal` (default) or `vertical`; the other family lies
  inside them.

- `max_length: Option<f64>`

  Longest bar, meters (default: the stock length, 12 m, or 40' in imperial projects):
  longer bars are lap spliced. 0 keeps whole bars.

- `lap: Option<f64>`

  Lap length of the splices, meters (default: 40 bar diameters).

- `lap_at_joins: Option<bool>`

  Run the horizontal bars half a lap past each end joined to another wall that carries
  on in the same line (the walls of a cylinder), so the bars of the two lap there
  (default false: they meet end to end at the joint). The lap is `lap`,
  or 40 bar diameters where bars are kept whole (`max_length` 0).

- `partition: Option<String>`

  The part of the structure the sets belong to (e.g. "FOUNDATION", "WALL"), which
  schedules filter and group by (default: none).


<a id="mod-cc_engine-defaults"></a>

## `cc_engine::defaults`

<sub>`crates/cc_engine/src/defaults.rs`</sub>

Defaults applied when elements are created (shared by the drawing tools).

**Items:** [`default_top`](#cc_engine-defaults-default_top) · [`default_roof_base`](#cc_engine-defaults-default_roof_base) · [`pipe_offset`](#cc_engine-defaults-pipe_offset) · [`drain_slope`](#cc_engine-defaults-drain_slope) · [`conduit_offset`](#cc_engine-defaults-conduit_offset) · [`tray_offset`](#cc_engine-defaults-tray_offset) · [`device_height`](#cc_engine-defaults-device_height) · [`next_number`](#cc_engine-defaults-next_number) · [`walls_loop`](#cc_engine-defaults-walls_loop) · [`walls_loop_at`](#cc_engine-defaults-walls_loop_at) · [`default_wall_face`](#cc_engine-defaults-default_wall_face)

<a id="cc_engine-defaults-default_top"></a>

### fn `default_top`

```rust
pub fn default_top(project: &Project, level: LevelId) -> WallTop
```

Top constraint for new columns and stairs: the level above, if any.

<a id="cc_engine-defaults-default_roof_base"></a>

### fn `default_roof_base`

```rust
pub fn default_roof_base(project: &Project, level: LevelId) -> f64
```

Default roof base: the top of the tallest wall on the level (else a
story: 3 m, 10'-0" in imperial projects).

<a id="cc_engine-defaults-pipe_offset"></a>

### fn `pipe_offset`

```rust
pub fn pipe_offset(project: &Project, kind: Option<cc_core::SystemKind>) -> f64
```

Elevation of a new pipe's center line above its level: supply pipes run
in the walls (0.6 m up, 2'-0" in imperial projects), drains under the
floor (0.3 m down, 1'-0").

<a id="cc_engine-defaults-drain_slope"></a>

### fn `drain_slope`

```rust
pub fn drain_slope(project: &Project, diameter: f64) -> f64
```

Minimum fall of a horizontal drain of outside `diameter`, as in common
drainage practice: 2 % up to 75 mm, 1 % from 100 mm (in imperial
projects 1/4" per foot up to 3", 1/8" per foot from 4").

<a id="cc_engine-defaults-conduit_offset"></a>

### fn `conduit_offset`

```rust
pub fn conduit_offset(project: &Project, level: LevelId) -> f64
```

Elevation of a new conduit above its level: in the slab above (the level
height, else 2.80 m; 9'-0" in imperial projects).

<a id="cc_engine-defaults-tray_offset"></a>

### fn `tray_offset`

```rust
pub fn tray_offset(project: &Project, level: LevelId) -> f64
```

Elevation of a new cable tray's center line above its level: under the
slab above, clear of it by 0.3 m (1'-0" in imperial projects).

<a id="cc_engine-defaults-device_height"></a>

### fn `device_height`

```rust
pub fn device_height(project: &Project, level: LevelId, device: cc_core::DeviceKind) -> f64
```

Usual mounting height of an electrical device above its level (lights:
the ceiling, at the level height); round inches in imperial projects
(see `cc_core::Defaults::devices`).

<a id="cc_engine-defaults-next_number"></a>

### fn `next_number`

```rust
pub fn next_number<'a>(existing: impl Iterator<Item = &'a str>) -> String
```

Next free number among `existing` names (1 when none is numeric).

<a id="cc_engine-defaults-walls_loop"></a>

### fn `walls_loop`

```rust
pub fn walls_loop(project: &Project, level: LevelId, outward: f64) -> Option<Vec<DVec2>>
```

Closed loop around the walls of `level`, offset to the exterior faces
(`outward > 0`) or interior faces (`outward < 0`): the outer boundary of
their location lines, so interior walls (even those meeting its corners)
and walls hanging off it are left out; with several separate loops, the
largest. The offset is half the thickest wall on the boundary.

<a id="cc_engine-defaults-walls_loop_at"></a>

### fn `walls_loop_at`

```rust
pub fn walls_loop_at(project: &Project, level: LevelId, face: LocationLine) -> Option<Vec<DVec2>>
```

Closed loop around the walls of `level` along one `face` of each wall on
it (issue #311): the outer boundary of their location lines, as
`walls_loop` finds it, with every side moved to that face of its own
wall, whatever its thickness, layers and location line. *Exterior* is
the side of a wall outside the loop and *interior* the side inside it
(not the wall's own exterior side, which depends on how it was drawn):
the finish faces are the wall's two faces, the core faces those of its
structure layers (the finish faces of a wall with no other layer).
Where walls in a line differ, the loop steps from one face to the next.

<a id="cc_engine-defaults-default_wall_face"></a>

### fn `default_wall_face`

```rust
pub fn default_wall_face(category: cc_core::Category) -> LocationLine
```

The face a loop of walls follows when none is asked for: the outside
faces, or the inside ones for ceilings.

<a id="mod-cc_engine-dimensions"></a>

## `cc_engine::dimensions`

<sub>`crates/cc_engine/src/dimensions.rs`</sub>

Dimensions that follow and drive the model.
Placing: each end of a new dimension follows the element (or, in
elevations and sections, the level line) whose linework in the view
passes through it. Editing the value: the moving end's element moves —
elements crossing the measured direction translate; walls, beams and
grids along it stretch their nearest end. In elevations and sections a
dimension between level lines raises or lowers the upper level.

**Items:** [`point_on_segment`](#cc_engine-dimensions-point_on_segment) · [`associate`](#cc_engine-dimensions-associate) · [`set_value`](#cc_engine-dimensions-set_value)

<a id="cc_engine-dimensions-point_on_segment"></a>

### fn `point_on_segment`

```rust
pub fn point_on_segment(a: DVec2, b: DVec2, p: DVec2, tol: f64) -> bool
```

<a id="cc_engine-dimensions-associate"></a>

### fn `associate`

```rust
pub fn associate(project: &Project, drawing: &Drawing2D, dim: &Dimension) -> Dimension
```

References for the ends of a dimension being placed: each end follows
the element whose linework passes through it, preferring lines across
the measured direction (the faces being measured) and, among those,
walls, columns and grids. Radial and diameter dimensions follow the
element of their arc; angular ones the elements along each measured line.

<a id="cc_engine-dimensions-set_value"></a>

### fn `set_value`

```rust
pub fn set_value(s: &mut Session, id: ElementId, value: f64) -> Result<()>
```

Give dimension `id` the measured `value` (meters, or radians for angles),
moving what its moving end touches (see the module docs). One undo step.

<a id="mod-cc_engine-error"></a>

## `cc_engine::error`

<sub>`crates/cc_engine/src/error.rs`</sub>

**Items:** [`ErrorCode`](#cc_engine-error-errorcode) · [`EngineError`](#cc_engine-error-engineerror) · [`Result`](#cc_engine-error-result)

<a id="cc_engine-error-errorcode"></a>

### enum `ErrorCode`

```rust
pub enum ErrorCode
```

What kind of failure a command met.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`

**Variants**

- `UnknownCommand`

  No command has this name.

- `InvalidArguments`

  The arguments do not match the command's schema.

- `NotFound`

  A referenced element, type, level, view or material does not exist.

- `Rejected`

  The edit would leave the model invalid, or is not allowed now.

- `Io`

  Reading or writing a file failed.

- `Internal`

  An internal error (a bug); the session is kept as it was.

- `Script`

  A Lua script or plugin failed: a syntax or run-time error, or one of
  its limits (instructions, memory) reached.


<a id="cc_engine-error-engineerror"></a>

### struct `EngineError`

```rust
pub struct EngineError
```

A command failure: a code for programs and a message for people.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `fmt::Display`, `std::error::Error`, `From<cc_core::PersistError>`, `From<cc_core::component_file::ComponentFileError>`, `From<cc_ifc::ImportError>`, `From<std::io::Error>`, `From<cc_core::wall_ops::EditError>`

**Fields**

- `code: ErrorCode`
- `message: String`

**Methods**

- `pub fn new(code: ErrorCode, message: impl Into<String>) -> Self`
- `pub fn invalid(message: impl Into<String>) -> Self`
- `pub fn not_found(message: impl Into<String>) -> Self`
- `pub fn rejected(message: impl Into<String>) -> Self`
- `pub fn io(message: impl Into<String>) -> Self`

<a id="cc_engine-error-result"></a>

### type alias `Result`

```rust
pub type Result<T> = std::result::Result<T, EngineError>
```

<a id="mod-cc_engine-extension"></a>

## `cc_engine::extension`

<sub>`crates/cc_engine/src/extension.rs`</sub>

Commands added at run time (plugins). An `Extension` set on a
`Session` lists its commands next to the built-in catalog and runs them
when `Session::execute` is asked for one of their names, so plugin
commands reach the GUI, the command line and the MCP server like the
built-in ones.
A command of an extension is one undo step however many commands it
runs, and a failed one leaves nothing behind.

**Items:** [`SEPARATOR`](#cc_engine-extension-separator) · [`Extension`](#cc_engine-extension-extension) · [`is_extension_name`](#cc_engine-extension-is_extension_name)

<a id="cc_engine-extension-separator"></a>

### const `SEPARATOR`

```rust
pub const SEPARATOR: &str
```

Separates an extension's id from its command's name in the full name
(`<id>__<name>`). Built-in names never contain it.

<a id="cc_engine-extension-extension"></a>

### trait `Extension`

```rust
pub trait Extension
```

A source of commands beyond the built-in catalog.

**Implemented by (1):** [`cc_plugins::plugin::PluginHost`](cc_plugins.md#cc_plugins-plugin-pluginhost)

**Required and provided items**

- `fn commands(&self) -> Vec<CommandInfo>`

  The commands it adds, with their full names.

- `fn run(&mut self, session: &mut Session, name: &str, args: Value) -> Option<Result<Value>>`

  Run the command `name` (one of `Extension::commands`) on `session`,
  or `None` when the name is not one of its own. The extension is taken
  out of the session while it runs, so commands it runs through
  `Session::execute` reach the built-in catalog only.

- `fn take_messages(&mut self) -> Vec<String>`

  Messages for the user since the last call (what plugins printed).


<a id="cc_engine-extension-is_extension_name"></a>

### fn `is_extension_name`

```rust
pub fn is_extension_name(name: &str) -> bool
```

Whether `name` could be the full name of an extension command.

<a id="mod-cc_engine-gltf"></a>

## `cc_engine::gltf`

<sub>`crates/cc_engine/src/gltf.rs`</sub>

glTF 2.0 models (`.glb`, or `.gltf` with its buffers embedded as data
URIs or in files next to it) as component types: each node of the scene's
root, with everything below it, becomes a type of fixed mesh pieces, one
per material, placed by an element at the center of its footprint and the
bottom of its extent. glTF lengths are meters and its Y axis points up,
so nothing needs asking. Materials take their base color and its
texture (a picture in a buffer, a data URI or a file next to the model),
repeating as the texture coordinates say (see `crate::import_material`),
and their opacity when blended.
Files are hostile input: every index, offset and count is checked, the
node graph is walked with a depth limit and a visited set, and the sizes
read are bounded. Compressed meshes (Draco, meshopt) are not read.

**Items:** [`MAX_BYTES`](#cc_engine-gltf-max_bytes) · [`GltfError`](#cc_engine-gltf-gltferror) · [`convert`](#cc_engine-gltf-convert)

<a id="cc_engine-gltf-max_bytes"></a>

### const `MAX_BYTES`

```rust
pub const MAX_BYTES: u64
```

Largest file read, in bytes (also each external buffer).

<a id="cc_engine-gltf-gltferror"></a>

### enum `GltfError`

```rust
pub enum GltfError
```

**Traits:** `Debug`, `Clone`, `PartialEq`

**Variants**

- `Invalid`

  Not a glTF 2.0 file, or its structure is broken.

- `Unsupported(String)`

  The meshes need an extension this reader lacks (its name).

- `MissingBuffer(String)`

  A buffer the meshes read could not be found or read (its URI).

- `TooLarge`

  More faces or objects than an import takes.


<a id="cc_engine-gltf-convert"></a>

### fn `convert`

```rust
pub fn convert(bytes: &[u8], dir: Option<&Path>, name: &str, merge: bool) -> Result<Converted, GltfError>
```

Convert the glTF file `bytes` (external buffers looked for in `dir`) to
types and placements; `merge` makes the whole scene one type at the
file's origin, named `name`.

<a id="mod-cc_engine-graph"></a>

## `cc_engine::graph`

<sub>`crates/cc_engine/src/graph/mod.rs`</sub>

Visual programming graphs: nodes joined by wires, evaluated as a
dataflow over the engine's commands.
Every command of the catalog is a node (its parameters are the inputs,
its result the outputs), next to built-in nodes (inputs, math, lists,
geometry as JSON values) and Lua nodes (pure functions run in the
sandbox). A node given lists where it takes single values runs once per
item (`lace`). Elements an edit node creates are bound to its call
(`cc_core::graph_binding`), so running the graph again changes them in
place, makes the new ones and deletes the ones no longer made (`eval`).
A graph is a `.ccgraph` file (JSON, `Graph`); files are hostile input:
`Graph::from_json` bounds sizes, drops dangling wires and refuses
cycles.

**Re-exports**

- `pub use eval::{Evaluator, NodeResult, RunReport}`
- `pub use lace::Lacing`
- `pub use ports::{Port, PortType}`

**Items:** [`GRAPH_FORMAT`](#cc_engine-graph-graph_format) · [`MAX_NODES`](#cc_engine-graph-max_nodes) · [`MAX_WIRES`](#cc_engine-graph-max_wires) · [`MAX_TEXT`](#cc_engine-graph-max_text) · [`MAX_FILE`](#cc_engine-graph-max_file) · [`MAX_LIST`](#cc_engine-graph-max_list) · [`MAX_CALLS`](#cc_engine-graph-max_calls) · [`RunMode`](#cc_engine-graph-runmode) · [`NodeKind`](#cc_engine-graph-nodekind) · [`Node`](#cc_engine-graph-node) · [`Wire`](#cc_engine-graph-wire) · [`Note`](#cc_engine-graph-note) · [`Group`](#cc_engine-graph-group) · [`Graph`](#cc_engine-graph-graph) · [`Repairs`](#cc_engine-graph-repairs)

<a id="cc_engine-graph-graph_format"></a>

### const `GRAPH_FORMAT`

```rust
pub const GRAPH_FORMAT: u32
```

Current `.ccgraph` format version.

<a id="cc_engine-graph-max_nodes"></a>

### const `MAX_NODES`

```rust
pub const MAX_NODES: usize
```

Most nodes in a graph.

<a id="cc_engine-graph-max_wires"></a>

### const `MAX_WIRES`

```rust
pub const MAX_WIRES: usize
```

Most wires in a graph.

<a id="cc_engine-graph-max_text"></a>

### const `MAX_TEXT`

```rust
pub const MAX_TEXT: usize
```

Largest text in a node (Lua code, notes), in bytes.

<a id="cc_engine-graph-max_file"></a>

### const `MAX_FILE`

```rust
pub const MAX_FILE: usize
```

Largest `.ccgraph` file, in bytes.

<a id="cc_engine-graph-max_list"></a>

### const `MAX_LIST`

```rust
pub const MAX_LIST: usize
```

Most items a list value may hold.

<a id="cc_engine-graph-max_calls"></a>

### const `MAX_CALLS`

```rust
pub const MAX_CALLS: usize
```

Most node calls in one run.

<a id="cc_engine-graph-runmode"></a>

### enum `RunMode`

```rust
pub enum RunMode
```

When a graph runs.

**Traits:** `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**Variants**

- `Manual`

  When asked to.

- `Automatic`

  After every change of the graph.


<a id="cc_engine-graph-nodekind"></a>

### enum `NodeKind`

```rust
pub enum NodeKind
```

What a node is.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`

**Variants**

- `Command(String)`

  An engine command (built-in, or a plugin's `<id>__<name>`).

- `Builtin(String)`

  A built-in node (`builtin::catalog`).

- `Lua(String)`

  A Lua function; its inputs and outputs are the node's `inputs` and
  `outputs`.


<a id="cc_engine-graph-node"></a>

### struct `Node`

```rust
pub struct Node
```

A box of the graph.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`

**Fields**

- `id: u64`

  Unique in the graph; bindings refer to it.

- `kind: NodeKind`
- `pos: [f32; 2]`

  Position on the canvas.

- `name: String`

  A label shown instead of the node's name; on an input node, the name
  of the graph input it is.

- `lacing: Lacing`
- `values: BTreeMap<String, Value>`

  Values of inputs left unwired (and the value of input nodes).

- `inputs: Vec<String>`

  Lua nodes: names of the inputs.

- `outputs: Vec<String>`

  Lua nodes: names of the outputs.

- `expanded: bool`

  Shows all its inputs (a command node otherwise shows the required,
  wired and set ones).


**Methods**

- `pub fn new(id: u64, kind: NodeKind, pos: [f32; 2]) -> Self`

<a id="cc_engine-graph-wire"></a>

### struct `Wire`

```rust
pub struct Wire
```

An output of one node joined to an input of another.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Serialize`, `Deserialize`

**Fields**

- `from: (u64, String)`

  Node id and output name.

- `to: (u64, String)`

  Node id and input name.


<a id="cc_engine-graph-note"></a>

### struct `Note`

```rust
pub struct Note
```

A text on the canvas.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`

**Fields**

- `pos: [f32; 2]`
- `text: String`

<a id="cc_engine-graph-group"></a>

### struct `Group`

```rust
pub struct Group
```

Nodes framed together under a title.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`

**Fields**

- `title: String`
- `nodes: BTreeSet<u64>`
- `color: [u8; 3]`

  Color as `[r, g, b]`.


<a id="cc_engine-graph-graph"></a>

### struct `Graph`

```rust
pub struct Graph
```

A visual programming graph (`.ccgraph`).

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `Default`

**Fields**

- `format: u32`
- `uid: Uid`

  Persistent id: the elements it made are bound to it.

- `name: String`
- `run_mode: RunMode`
- `allow_files: bool`

  Whether its commands may read and write files: a grant of the run
  (`graph_run`'s `allow_files`, the editor's checkbox), never read from
  or written to a file, so a graph cannot give itself file access. Old
  files that hold the field load with it ignored.

- `nodes: Vec<Node>`
- `wires: Vec<Wire>`
- `notes: Vec<Note>`
- `groups: Vec<Group>`

**Methods**

- `pub fn new(name: &str) -> Self`
- `pub fn from_json(text: &str) -> Result<(Graph, Repairs), String>`

  Read a graph (a file is hostile input) and fix what can be fixed.

- `pub fn from_value(value: Value) -> Result<(Graph, Repairs), String>`

  Read a graph from a JSON value (a command's argument).

- `pub fn to_json(&self) -> String`
- `pub fn repair(&mut self) -> Result<Repairs, String>`

  Make the graph consistent: bounded sizes, unique node ids, wires
  between existing nodes, one wire per input, no cycles.

- `pub fn node(&self, id: u64) -> Option<&Node>`
- `pub fn node_mut(&mut self, id: u64) -> Option<&mut Node>`
- `pub fn next_id(&self) -> u64`

  A free node id.

- `pub fn wire_into(&self, node: u64, input: &str) -> Option<&Wire>`

  The wire into an input, if any.

- `pub fn reaches(&self, from: u64, to: u64) -> bool`

  Whether `to` can be reached from `from` along wires.

- `pub fn connect(&mut self, from: (u64, String), to: (u64, String)) -> Result<(), String>`

  Join an output to an input, replacing the input's wire; refused when
  it would close a cycle.

- `pub fn remove_node(&mut self, id: u64)`

  Remove a node and its wires.

- `pub fn order(&self) -> Vec<u64>`

  The nodes in an order where every node comes after the nodes wired
  into it (the graph has no cycles once repaired; a cycle's nodes are
  left out).


<a id="cc_engine-graph-repairs"></a>

### struct `Repairs`

```rust
pub struct Repairs
```

What reading a graph file fixed.

**Traits:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

**Fields**

- `wires: usize`

  Wires dropped (dangling, repeated, a second wire into one input, or
  closing a cycle).

- `nodes: usize`

  Nodes dropped (repeated ids, oversized text).


**Methods**

- `pub fn total(&self) -> usize`

<a id="mod-cc_engine-graph-builtin"></a>

## `cc_engine::graph::builtin`

<sub>`crates/cc_engine/src/graph/builtin.rs`</sub>

The built-in nodes: input values, math, lists, geometry (as the JSON
values commands take: points `[x, y]` and curves) and element
parameters.

**Items:** [`Outputs`](#cc_engine-graph-builtin-outputs) · [`PureFn`](#cc_engine-graph-builtin-purefn) · [`SessionFn`](#cc_engine-graph-builtin-sessionfn) · [`Eval`](#cc_engine-graph-builtin-eval) · [`Builtin`](#cc_engine-graph-builtin-builtin) · [`GROUPS`](#cc_engine-graph-builtin-groups) · [`catalog`](#cc_engine-graph-builtin-catalog) · [`find`](#cc_engine-graph-builtin-find) · [`number`](#cc_engine-graph-builtin-number) · [`point`](#cc_engine-graph-builtin-point) · [`at_path`](#cc_engine-graph-builtin-at_path)

<a id="cc_engine-graph-builtin-outputs"></a>

### type alias `Outputs`

```rust
pub type Outputs = Result<Vec<Value>, String>
```

The outputs of one call, or why it failed.

<a id="cc_engine-graph-builtin-purefn"></a>

### type alias `PureFn`

```rust
pub type PureFn = fn(&[Option<Value>], &Node) -> Outputs
```

A node that reads only its inputs.

<a id="cc_engine-graph-builtin-sessionfn"></a>

### type alias `SessionFn`

```rust
pub type SessionFn = fn(&mut Session, &[Option<Value>], &Node) -> Outputs
```

A node that reads or changes the model.

<a id="cc_engine-graph-builtin-eval"></a>

### enum `Eval`

```rust
pub enum Eval
```

What running a built-in node needs.

**Traits:** `Clone`, `Copy`

**Variants**

- `Pure(PureFn)`

  A function of its inputs (and the node's stored values).

- `Query(SessionFn)`

  Reads the model.

- `Edit(SessionFn)`

  Changes the model (never creates elements).


<a id="cc_engine-graph-builtin-builtin"></a>

### struct `Builtin`

```rust
pub struct Builtin
```

A built-in node's definition.

**Traits:** `Clone`

**Fields**

- `name: &'static str`
- `group: &'static str`
- `description: &'static str`
- `inputs: Vec<Port>`
- `outputs: Vec<Port>`
- `eval: Eval`
- `input: bool`

  An input node: its value is `values.value`, and when the node has a
  name it is an input of the graph (`graph_run`'s `inputs`).


<a id="cc_engine-graph-builtin-groups"></a>

### const `GROUPS`

```rust
pub const GROUPS: [&str; 7]
```

Groups of built-in nodes, in the order the node search shows them.

<a id="cc_engine-graph-builtin-catalog"></a>

### fn `catalog`

```rust
pub fn catalog() -> Vec<Builtin>
```

Every built-in node.

<a id="cc_engine-graph-builtin-find"></a>

### fn `find`

```rust
pub fn find(name: &str) -> Option<Builtin>
```

One built-in node by name.

<a id="cc_engine-graph-builtin-number"></a>

### fn `number`

```rust
pub(crate) fn number(v: &Option<Value>) -> Result<f64, String>
```

<a id="cc_engine-graph-builtin-point"></a>

### fn `point`

```rust
pub(crate) fn point(v: &Option<Value>) -> Result<[f64; 2], String>
```

<a id="cc_engine-graph-builtin-at_path"></a>

### fn `at_path`

```rust
pub(crate) fn at_path<'a>(mut v: &'a Value, path: &str) -> Option<&'a Value>
```

The value at a dotted path (`params.height`, `ids.0`).

<a id="mod-cc_engine-graph-eval"></a>

## `cc_engine::graph::eval`

<sub>`crates/cc_engine/src/graph/eval.rs`</sub>

Running a graph: nodes in dataflow order, lists mapped over, the
elements edit nodes create bound to their calls.
A run is one undo step. For each call of a command that creates
elements, the elements made by the same call in an earlier run (the same
node and lacing path) take the new content and keep their ids, persistent
ids, marks, comments and phases, so what refers to them stays; new calls
make new elements, and after the run the elements of calls no longer
made are deleted. Running again with nothing changed changes nothing.

**Items:** [`DefKind`](#cc_engine-graph-eval-defkind) · [`NodeDef`](#cc_engine-graph-eval-nodedef) · [`command_def`](#cc_engine-graph-eval-command_def) · [`node_def`](#cc_engine-graph-eval-node_def) · [`NodeResult`](#cc_engine-graph-eval-noderesult) · [`RunReport`](#cc_engine-graph-eval-runreport) · [`Evaluator`](#cc_engine-graph-eval-evaluator) · [`is_identifier`](#cc_engine-graph-eval-is_identifier)

<a id="cc_engine-graph-eval-defkind"></a>

### enum `DefKind`

```rust
pub enum DefKind
```

What a node is, resolved against the session's catalog.

**Traits:** `Clone`

**Variants**

- `Command { name: String, kind: CommandKind, }`
- `Builtin(Builtin)`
- `Lua`
- `Missing(String)`

  A node that cannot run here (unknown, refused, a file command in a
  graph that does not allow them), and why.


<a id="cc_engine-graph-eval-nodedef"></a>

### struct `NodeDef`

```rust
pub struct NodeDef
```

A node's ports and what running it does.

**Traits:** `Clone`

**Fields**

- `title: String`
- `group: String`
- `description: String`
- `inputs: Vec<Port>`
- `outputs: Vec<Port>`
- `kind: DefKind`

**Methods**

- `pub fn edits(&self) -> bool`

  Whether running it may change the model.


<a id="cc_engine-graph-eval-command_def"></a>

### fn `command_def`

```rust
pub fn command_def(info: &crate::CommandInfo) -> NodeDef
```

The ports of a command, from its catalog entry.

<a id="cc_engine-graph-eval-node_def"></a>

### fn `node_def`

```rust
pub fn node_def(session: &Session, graph: &Graph, node: &Node) -> NodeDef
```

The definition of `node` in `graph`, against `session`'s commands.

<a id="cc_engine-graph-eval-noderesult"></a>

### struct `NodeResult`

```rust
pub struct NodeResult
```

How one node fared in a run.

**Traits:** `Debug`, `Clone`, `Default`, `Serialize`

**Fields**

- `outputs: BTreeMap<String, Value>`

  Its outputs by name.

- `error: Option<String>`

  Why it failed.

- `skipped: Option<String>`

  Not run, and why: an input failed or has no value.

- `cached: bool`

  Reused from the previous run (nothing it depends on changed).


<a id="cc_engine-graph-eval-runreport"></a>

### struct `RunReport`

```rust
pub struct RunReport
```

What a run did.

**Traits:** `Debug`, `Clone`, `Default`, `Serialize`

**Fields**

- `nodes: BTreeMap<u64, NodeResult>`
- `outputs: BTreeMap<String, Value>`

  The values of the graph's `output` nodes, by name.

- `created: usize`
- `changed: usize`
- `deleted: usize`
- `errors: usize`

  Nodes that failed.

- `messages: Vec<String>`

  What Lua nodes printed.


<a id="cc_engine-graph-eval-evaluator"></a>

### struct `Evaluator`

```rust
pub struct Evaluator
```

Runs graphs; keeps each node's last outputs, so a run after a change of
the graph alone re-runs only the nodes it affects.

**Traits:** `Default`

*Some fields are private.*

**Methods**

- `pub fn new() -> Self`
- `pub fn clear(&mut self)`

  Forget the cached outputs (the next run runs every node).

- `pub fn run(&mut self, session: &mut Session, graph: &Graph, inputs: &BTreeMap<String, Value>) -> RunReport`

  Run `graph` on `session` as one undo step; `inputs` set the graph's
  named input nodes.


<a id="cc_engine-graph-eval-is_identifier"></a>

### fn `is_identifier`

```rust
pub fn is_identifier(name: &str) -> bool
```

Whether `name` can be a Lua local's name.

<a id="mod-cc_engine-graph-lace"></a>

## `cc_engine::graph::lace`

<sub>`crates/cc_engine/src/graph/lace.rs`</sub>

Lacing: a node given lists where it takes single values runs once per
item, and its outputs become lists of the same shape.

**Items:** [`MAX_LEVELS`](#cc_engine-graph-lace-max_levels) · [`Lacing`](#cc_engine-graph-lace-lacing) · [`Arg`](#cc_engine-graph-lace-arg) · [`map`](#cc_engine-graph-lace-map)

<a id="cc_engine-graph-lace-max_levels"></a>

### const `MAX_LEVELS`

```rust
pub const MAX_LEVELS: usize
```

Deepest list nesting a node maps over.

<a id="cc_engine-graph-lace-lacing"></a>

### enum `Lacing`

```rust
pub enum Lacing
```

How the items of several lists pair up.

**Traits:** `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`, `JsonSchema`

**Variants**

- `Shortest`

  Item by item, as many times as the shortest list.

- `Longest`

  Item by item, as many times as the longest list (the shorter ones
  repeat their last item).

- `Cross`

  Every combination: one more list level per list.


<a id="cc_engine-graph-lace-arg"></a>

### struct `Arg`

```rust
pub struct Arg
```

An input of one mapping: its value and how many list levels it runs
past what its port takes.

**Traits:** `Debug`, `Clone`

**Fields**

- `value: Option<Value>`
- `levels: usize`

<a id="cc_engine-graph-lace-map"></a>

### fn `map`

```rust
pub fn map(args: Vec<Arg>, lacing: Lacing, outputs: usize, budget: &mut usize, call: &mut Call) -> Result<Vec<Value>, String>
```

Run `call` once per combination of items and gather its outputs
(`outputs` of them per call) into lists. `call` gets the values and the
path of indices of the call; `budget` counts calls down.

<a id="mod-cc_engine-graph-ports"></a>

## `cc_engine::graph::ports`

<sub>`crates/cc_engine/src/graph/ports.rs`</sub>

Ports of graph nodes: their types, read from command schemas, and how
deep a value runs past what a port takes (the lists a node maps over).

**Items:** [`PortType`](#cc_engine-graph-ports-porttype) · [`Port`](#cc_engine-graph-ports-port) · [`type_of_schema`](#cc_engine-graph-ports-type_of_schema) · [`depth`](#cc_engine-graph-ports-depth)

<a id="cc_engine-graph-ports-porttype"></a>

### enum `PortType`

```rust
pub enum PortType
```

What a port carries. Values are JSON; the type says how they read.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`

**Variants**

- `Any`

  Anything, passed through whole (never mapped over).

- `Number`
- `Integer`
- `Bool`
- `Text`
- `Point`

  A point `[x, y]` (or `[x, y, z]`).

- `Element`

  An element id.

- `Ref`

  A level, type, view or material: its id or its name.

- `Object`

  A JSON object (a command result, a curve).

- `List(Box<PortType>)`

**Methods**

- `pub fn rank(&self) -> Option<usize>`

  How many list levels the port takes as one value; `None` takes any
  value whole.

- `pub fn fits(&self, to: &PortType) -> bool`

  Whether an output of type `self` may be wired to an input of type
  `to`, counting lists the input maps over.

- `pub fn label(&self) -> String`

  A short name for the type (shown by the editor and `graph_nodes`).


<a id="cc_engine-graph-ports-port"></a>

### struct `Port`

```rust
pub struct Port
```

An input or output of a node.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Serialize`

**Fields**

- `name: String`
- `ty: PortType`
- `required: bool`

  An input the node cannot run without.

- `default: Option<Value>`

  The value of an input left unwired.

- `description: String`

**Methods**

- `pub fn new(name: &str, ty: PortType) -> Self`
- `pub fn required(mut self) -> Self`
- `pub fn default(mut self, v: Value) -> Self`
- `pub fn describe(mut self, d: &str) -> Self`

<a id="cc_engine-graph-ports-type_of_schema"></a>

### fn `type_of_schema`

```rust
pub fn type_of_schema(name: &str, schema: &Value) -> PortType
```

The type of a parameter from its JSON schema (draft 7, inlined).

<a id="cc_engine-graph-ports-depth"></a>

### fn `depth`

```rust
pub fn depth(value: &Value) -> usize
```

How deep `value` runs in lists (an empty list counts one level).

<a id="mod-cc_engine-i18n"></a>

## `cc_engine::i18n`

<sub>`crates/cc_engine/src/i18n.rs`</sub>

Localization: a Fluent-compatible subset (`key = value` messages with
`{ $var }` placeholders). All languages are embedded at build time and the
active one can be switched at runtime.

**Items:** [`Language`](#cc_engine-i18n-language) · [`language`](#cc_engine-i18n-language) · [`set_language`](#cc_engine-i18n-set_language) · [`parse_messages`](#cc_engine-i18n-parse_messages) · [`try_tr`](#cc_engine-i18n-try_tr) · [`tr`](#cc_engine-i18n-tr) · [`tr_args`](#cc_engine-i18n-tr_args) · [`format_date`](#cc_engine-i18n-format_date) · [`today`](#cc_engine-i18n-today) · [`civil_from_days`](#cc_engine-i18n-civil_from_days) · [`t`](#cc_engine-i18n-t)

<a id="cc_engine-i18n-language"></a>

### enum `Language`

```rust
pub enum Language
```

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

**Variants**

- `PtBr`
- `En`
- `Es`
- `Fr`
- `De`
- `It`
- `Lo`
- `Pl`
- `Uk`
- `ZhTw`
- `ZhCn`
- `Ja`

**Methods**

- `pub const ALL: [Language; 12]`
- `pub fn code(self) -> &'static str`

  BCP 47 code, used for persistence.

- `pub fn native_name(self) -> &'static str`

  Name in the language itself, so it is recognizable whatever is active.

- `pub fn from_code(code: &str) -> Option<Language>`
- `pub fn from_system() -> Language`

  Language of the operating system, English when unsupported.

- `pub fn source(self) -> &'static str`

  The embedded message file of the language.

- `pub fn decimal_comma(self) -> bool`

  Whether numbers use a decimal comma.


<a id="cc_engine-i18n-language"></a>

### fn `language`

```rust
pub fn language() -> Language
```

<a id="cc_engine-i18n-set_language"></a>

### fn `set_language`

```rust
pub fn set_language(lang: Language)
```

Switch the active language (also sets decimal and unit symbols).

<a id="cc_engine-i18n-parse_messages"></a>

### fn `parse_messages`

```rust
pub fn parse_messages(source: &str) -> HashMap<String, String>
```

Parse `key = value` lines; `#` starts a comment line.

<a id="cc_engine-i18n-try_tr"></a>

### fn `try_tr`

```rust
pub fn try_tr(key: &str) -> Option<String>
```

Translate `key` if the active language defines it.

<a id="cc_engine-i18n-tr"></a>

### fn `tr`

```rust
pub fn tr(key: &str) -> String
```

Translate `key`; falls back to Portuguese, then to the key itself so gaps are visible.

<a id="cc_engine-i18n-tr_args"></a>

### fn `tr_args`

```rust
pub fn tr_args(key: &str, args: &[(&str, String)]) -> String
```

Translate `key`, substituting `{ $name }` placeholders.

<a id="cc_engine-i18n-format_date"></a>

### fn `format_date`

```rust
pub fn format_date(y: i32, m: u32, d: u32) -> String
```

A date written the way the current language writes it.

<a id="cc_engine-i18n-today"></a>

### fn `today`

```rust
pub fn today() -> (i32, u32, u32)
```

Today's date (UTC), from the system clock.

<a id="cc_engine-i18n-civil_from_days"></a>

### fn `civil_from_days`

```rust
pub fn civil_from_days(z: i64) -> (i32, u32, u32)
```

Year, month, day of a count of days since 1970-01-01 (proleptic Gregorian).

<a id="cc_engine-i18n-t"></a>

### macro `t`

```rust
macro_rules! t
```

`t!("key")` or `t!("key", "name" => value, ...)`.

<a id="mod-cc_engine-import_material"></a>

## `cc_engine::import_material`

<sub>`crates/cc_engine/src/import_material.rs`</sub>

Materials of imported models (SketchUp, OBJ, glTF): name, color,
opacity and texture. A texture's picture becomes the material's
appearance texture, repeating every so many meters: the model's own
texture coordinates cannot be kept (textures are laid on each face's
plane), so the repeat comes from them — the meters one unit of the
coordinates spans on the faces, see `UvScale`. The material's color is
its color times the picture's average, so it looks right where textures
are not drawn.

**Items:** [`ImportedMaterial`](#cc_engine-import_material-importedmaterial) · [`texture_image`](#cc_engine-import_material-texture_image) · [`to_linear`](#cc_engine-import_material-to_linear) · [`to_srgb`](#cc_engine-import_material-to_srgb) · [`average_color`](#cc_engine-import_material-average_color) · [`textured`](#cc_engine-import_material-textured) · [`UvScale`](#cc_engine-import_material-uvscale)

<a id="cc_engine-import_material-importedmaterial"></a>

### struct `ImportedMaterial`

```rust
pub struct ImportedMaterial
```

A material a model uses.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `name: String`
- `color: [u8; 3]`

  sRGB.

- `opacity: f64`

  1 opaque, 0 invisible.

- `texture: Option<MaterialTexture>`

**Methods**

- `pub fn plain(name: impl Into<String>, color: [u8; 3]) -> Self`

  A plain, opaque material.

- `pub fn to_material(&self) -> cc_core::Material`

  The project material.


<a id="cc_engine-import_material-texture_image"></a>

### fn `texture_image`

```rust
pub fn texture_image(bytes: Vec<u8>) -> Option<RasterImage>
```

An image file usable as a texture (a picture format this reads, within
the texture limits).

<a id="cc_engine-import_material-to_linear"></a>

### fn `to_linear`

```rust
pub fn to_linear(c: u8) -> f64
```

sRGB byte to linear 0..1.

<a id="cc_engine-import_material-to_srgb"></a>

### fn `to_srgb`

```rust
pub fn to_srgb(v: f64) -> u8
```

Linear 0..1 to an sRGB byte (200 when not a number).

<a id="cc_engine-import_material-average_color"></a>

### fn `average_color`

```rust
pub fn average_color(image: &RasterImage) -> Option<[f64; 3]>
```

The average color of a picture (linear, weighted by alpha), or `None`
when it cannot be decoded or is wholly clear.

<a id="cc_engine-import_material-textured"></a>

### fn `textured`

```rust
pub fn textured(name: String, factor: [f64; 3], opacity: f64, image: Option<RasterImage>, repeat: Option<(f64, f64)>) -> ImportedMaterial
```

A material with color `factor` (linear) times the average of `image`,
which becomes its texture repeating every `repeat` meters (1 m when the
model gives no texture coordinates).

<a id="cc_engine-import_material-uvscale"></a>

### struct `UvScale`

```rust
pub struct UvScale
```

The meters a unit of texture coordinates spans on the faces, along u and
v, averaged over triangles by their area.

**Traits:** `Debug`, `Clone`, `Copy`, `Default`

*Some fields are private.*

**Methods**

- `pub fn add(&mut self, p: [DVec3; 3], uv: [DVec2; 3])`

  Add a triangle with its corners' texture coordinates.

- `pub fn repeat(&self) -> Option<(f64, f64)>`

  The repeat (width, height) in meters, when any triangle had coordinates.


<a id="mod-cc_engine-obj"></a>

## `cc_engine::obj`

<sub>`crates/cc_engine/src/obj.rs`</sub>

Wavefront OBJ models as component types: each object (`o`, or `g` in a
file without objects) becomes a type of fixed mesh pieces, one per
material of its `.mtl` library, placed by an element at the center of
its footprint and the bottom of its extent. OBJ has no units nor a fixed
up axis, so the caller gives both. Materials take `Kd`, `d`/`Tr` and the
picture of `map_Kd` (by file name next to the model), its repeat from
the faces' texture coordinates (`vt`, see `crate::import_material`).
Files are hostile input: unknown statements are skipped, faces with a
bad index are dropped, and the sizes read are bounded.

**Items:** [`MAX_BYTES`](#cc_engine-obj-max_bytes) · [`Frame`](#cc_engine-obj-frame) · [`ObjError`](#cc_engine-obj-objerror) · [`Mtl`](#cc_engine-obj-mtl) · [`read_mtl`](#cc_engine-obj-read_mtl) · [`convert`](#cc_engine-obj-convert)

<a id="cc_engine-obj-max_bytes"></a>

### const `MAX_BYTES`

```rust
pub const MAX_BYTES: u64
```

Largest .obj file read, in bytes.

<a id="cc_engine-obj-frame"></a>

### struct `Frame`

```rust
pub struct Frame
```

How the file's coordinates map to the model's.

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `scale: f64`

  Meters per file unit.

- `y_up: bool`

  The file's Y axis points up (the usual OBJ convention); else Z does.


<a id="cc_engine-obj-objerror"></a>

### enum `ObjError`

```rust
pub enum ObjError
```

**Traits:** `Debug`

**Variants**

- `TooLarge`

  More vertices, faces or objects than an import takes.


<a id="cc_engine-obj-mtl"></a>

### struct `Mtl`

```rust
pub struct Mtl
```

A material of a library: diffuse color (`Kd`, kept linear 0 to 1), opacity
(`d`, or `Tr` = 1 - d) and diffuse picture (`map_Kd`, a file name).

**Traits:** `Debug`, `Clone`, `PartialEq`, `Default`

**Fields**

- `color: [f64; 3]`
- `opacity: f64`
- `map: Option<String>`

<a id="cc_engine-obj-read_mtl"></a>

### fn `read_mtl`

```rust
pub fn read_mtl(text: &str) -> HashMap<String, Mtl>
```

The materials of a material library.

<a id="cc_engine-obj-convert"></a>

### fn `convert`

```rust
pub fn convert(text: &str, dir: Option<&Path>, frame: Frame, name: &str, merge: bool) -> Result<Converted, ObjError>
```

Convert the OBJ `text` (its material libraries looked for in `dir`) to
types and placements; `merge` makes the whole file one type at the origin
of the file, named `name`.

<a id="mod-cc_engine-osm"></a>

## `cc_engine::osm`

<sub>`crates/cc_engine/src/osm.rs`</sub>

OpenStreetMap data around the project site: the JSON an Overpass query
returns, turned into a site drawing (buildings, streets, paths,
railways, water, green areas, trees and street names, in project
coordinates) and building footprints with heights for context masses;
plus the answers of the address search (Nominatim) and of an elevation
service. Everything here is pure: the program downloads, the engine
reads. Map data is © OpenStreetMap contributors, under the ODbL.

**Items:** [`MAX_JSON`](#cc_engine-osm-max_json) · [`MAX_ELEMENTS`](#cc_engine-osm-max_elements) · [`ATTRIBUTION`](#cc_engine-osm-attribution) · [`Layer`](#cc_engine-osm-layer) · [`Building`](#cc_engine-osm-building) · [`Options`](#cc_engine-osm-options) · [`Site`](#cc_engine-osm-site) · [`read`](#cc_engine-osm-read) · [`overpass_query`](#cc_engine-osm-overpass_query) · [`Place`](#cc_engine-osm-place) · [`read_places`](#cc_engine-osm-read_places) · [`read_reverse`](#cc_engine-osm-read_reverse) · [`read_elevation`](#cc_engine-osm-read_elevation)

<a id="cc_engine-osm-max_json"></a>

### const `MAX_JSON`

```rust
pub const MAX_JSON: usize
```

Largest Overpass answer read, bytes.

<a id="cc_engine-osm-max_elements"></a>

### const `MAX_ELEMENTS`

```rust
pub const MAX_ELEMENTS: usize
```

Most OSM elements read from one answer.

<a id="cc_engine-osm-attribution"></a>

### const `ATTRIBUTION`

```rust
pub const ATTRIBUTION: &str
```

Attribution required with any use of OpenStreetMap data.

<a id="cc_engine-osm-layer"></a>

### enum `Layer`

```rust
pub enum Layer
```

The layers of a site drawing, in the order they are listed.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`

**Variants**

- `Buildings`
- `Streets`
- `Paths`
- `Railways`
- `Water`
- `Green`
- `Trees`
- `Names`

**Methods**

- `pub const ALL: [Layer; 8]`
- `pub fn key(self) -> &'static str`

  The localization key of its name.

- `pub fn code(self) -> &'static str`

  Short code in command results.


<a id="cc_engine-osm-building"></a>

### struct `Building`

```rust
pub struct Building
```

A building footprint for a context mass.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `outline: Vec<DVec2>`

  Footprint in project coordinates, open (first point not repeated).

- `height: f64`

  Height above its base, meters.

- `base: f64`

  Height of its base above the ground, meters (`min_height`).

- `measured: bool`

  Its height came from the map (`height` or `building:levels`), not the default.

- `label: String`

  Name, else the street address, if the map gives one.

- `source: String`

  `way/123` or `relation/45`.


<a id="cc_engine-osm-options"></a>

### struct `Options`

```rust
pub struct Options
```

How map data becomes a site drawing and masses.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Default`

**Fields**

- `extent: Option<f64>`

  Half the side of the square kept around the site, meters (`None`: everything given).

- `level_height: f64`

  Height of a storey, for `building:levels`.

- `default_levels: f64`

  Storeys of a building whose height the map does not give.

- `text_height: f64`

  Height of street names, meters.


<a id="cc_engine-osm-site"></a>

### struct `Site`

```rust
pub struct Site
```

A site drawing and the buildings for masses.

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `drawing: CadDrawing`
- `buildings: Vec<Building>`
- `counts: Vec<(Layer, usize)>`

  Primitives per layer.


<a id="cc_engine-osm-read"></a>

### fn `read`

```rust
pub fn read(json: &str, site: &SiteLocation, opts: &Options) -> Result<Site, String>
```

Read an Overpass answer (`[out:json]`, with `out geom` or with the
ways' nodes) into a site drawing in project coordinates around `site`.

<a id="cc_engine-osm-overpass_query"></a>

### fn `overpass_query`

```rust
pub fn overpass_query(site: &SiteLocation, extent: f64) -> String
```

The Overpass QL query for the map data of the square of half side
`extent` meters around `site` (its bounding box in latitude and
longitude, widened to hold the square under any north angle).

<a id="cc_engine-osm-place"></a>

### struct `Place`

```rust
pub struct Place
```

A place the address search found.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `name: String`

  The full name the service gives.

- `latitude: f64`
- `longitude: f64`
- `address: SiteAddress`

<a id="cc_engine-osm-read_places"></a>

### fn `read_places`

```rust
pub fn read_places(json: &str) -> Result<Vec<Place>, String>
```

The places of a Nominatim search answer (`format=jsonv2`, `addressdetails=1`).

<a id="cc_engine-osm-read_reverse"></a>

### fn `read_reverse`

```rust
pub fn read_reverse(json: &str) -> Result<Option<Place>, String>
```

The place of a Nominatim reverse answer (`format=jsonv2`), if any.

<a id="cc_engine-osm-read_elevation"></a>

### fn `read_elevation`

```rust
pub fn read_elevation(json: &str) -> Result<f64, String>
```

The first elevation of an elevation answer (`{"elevation": [12.0]}`), meters.

<a id="mod-cc_engine-reference"></a>

## `cc_engine::reference`

<sub>`crates/cc_engine/src/reference.rs`</sub>

The command reference (`docs/commands.md`), generated from the catalog
so it always matches the engine: `ccad reference > docs/commands.md`.

**Items:** [`markdown`](#cc_engine-reference-markdown)

<a id="cc_engine-reference-markdown"></a>

### fn `markdown`

```rust
pub fn markdown() -> String
```

The Markdown reference of every command.

<a id="mod-cc_engine-refs"></a>

## `cc_engine::refs`

<sub>`crates/cc_engine/src/refs.rs`</sub>

Naming model objects in commands: levels, types, views, materials and
systems by numeric id or by name; elements by id.

**Items:** [`Ref`](#cc_engine-refs-ref) · [`ScaleArg`](#cc_engine-refs-scalearg) · [`Point`](#cc_engine-refs-point) · [`point`](#cc_engine-refs-point) · [`points`](#cc_engine-refs-points) · [`finite`](#cc_engine-refs-finite) · [`positive`](#cc_engine-refs-positive) · [`level`](#cc_engine-refs-level) · [`browser_scheme`](#cc_engine-refs-browser_scheme) · [`level_or_first`](#cc_engine-refs-level_or_first) · [`element_type`](#cc_engine-refs-element_type) · [`type_of`](#cc_engine-refs-type_of) · [`view`](#cc_engine-refs-view) · [`material`](#cc_engine-refs-material) · [`system`](#cc_engine-refs-system) · [`element`](#cc_engine-refs-element) · [`elements`](#cc_engine-refs-elements) · [`category`](#cc_engine-refs-category) · [`view_category`](#cc_engine-refs-view_category) · [`category_name`](#cc_engine-refs-category_name) · [`filter`](#cc_engine-refs-filter) · [`template`](#cc_engine-refs-template) · [`phase`](#cc_engine-refs-phase) · [`option_set`](#cc_engine-refs-option_set) · [`design_option`](#cc_engine-refs-design_option) · [`line_style`](#cc_engine-refs-line_style) · [`fill_pattern`](#cc_engine-refs-fill_pattern) · [`profile`](#cc_engine-refs-profile) · [`phase_filter`](#cc_engine-refs-phase_filter) · [`nullable`](#cc_engine-refs-nullable) · [`detail_level`](#cc_engine-refs-detail_level) · [`visual_style`](#cc_engine-refs-visual_style)

<a id="cc_engine-refs-ref"></a>

### enum `Ref`

```rust
pub enum Ref
```

A level, type, view or material: its numeric id or its name.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`, `Default`

**Variants**

- `Id(u64)`
- `Name(String)`

<a id="cc_engine-refs-scalearg"></a>

### enum `ScaleArg`

```rust
pub enum ScaleArg
```

A drawing scale: the denominator `N` of 1:`N` (`48`), or text read by
`cc_core::scale::parse` (`"1:48"`, `"1/4\" = 1'-0\""`).

**Traits:** `Debug`, `Clone`, `PartialEq`, `Deserialize`, `Serialize`, `JsonSchema`, `From<u32>`

**Variants**

- `Ratio(u32)`
- `Text(String)`

**Methods**

- `pub fn ratio(&self) -> Result<u32>`

  The denominator, at least 1; an unreadable text is invalid.


<a id="cc_engine-refs-point"></a>

### type alias `Point`

```rust
pub type Point = [f64; 2]
```

A point in plan, `[x, y]` in meters.

<a id="cc_engine-refs-point"></a>

### fn `point`

```rust
pub fn point(p: Point) -> Result<DVec2>
```

<a id="cc_engine-refs-points"></a>

### fn `points`

```rust
pub fn points(ps: &[Point]) -> Result<Vec<DVec2>>
```

<a id="cc_engine-refs-finite"></a>

### fn `finite`

```rust
pub fn finite(v: f64, what: &str) -> Result<f64>
```

A finite number.

<a id="cc_engine-refs-positive"></a>

### fn `positive`

```rust
pub fn positive(v: f64, what: &str) -> Result<f64>
```

A positive finite number.

<a id="cc_engine-refs-level"></a>

### fn `level`

```rust
pub fn level(p: &Project, r: &Ref) -> Result<LevelId>
```

<a id="cc_engine-refs-browser_scheme"></a>

### fn `browser_scheme`

```rust
pub fn browser_scheme(p: &Project, r: &Ref) -> Result<cc_core::BrowserSchemeId>
```

A project browser organization scheme.

<a id="cc_engine-refs-level_or_first"></a>

### fn `level_or_first`

```rust
pub fn level_or_first(p: &Project, r: Option<&Ref>) -> Result<LevelId>
```

The given level, else the lowest one.

<a id="cc_engine-refs-element_type"></a>

### fn `element_type`

```rust
pub fn element_type(p: &Project, r: &Ref) -> Result<ElementTypeId>
```

<a id="cc_engine-refs-type_of"></a>

### fn `type_of`

```rust
pub fn type_of(p: &Project, category: Category, r: Option<&Ref>) -> Result<ElementTypeId>
```

A type of `category`: the given one (checked), else the category's default.

<a id="cc_engine-refs-view"></a>

### fn `view`

```rust
pub fn view(p: &Project, r: &Ref) -> Result<ViewId>
```

<a id="cc_engine-refs-material"></a>

### fn `material`

```rust
pub fn material(p: &Project, r: &Ref) -> Result<String>
```

A material's name (materials are keyed by name; an index also works).

<a id="cc_engine-refs-system"></a>

### fn `system`

```rust
pub fn system(p: &Project, r: &Ref) -> Result<cc_core::SystemId>
```

A plumbing system or electrical circuit by id, name, code or, when it is
the only one of its kind, kind (`cold_water`, `sanitary`…).

<a id="cc_engine-refs-element"></a>

### fn `element`

```rust
pub fn element(p: &Project, id: u64) -> Result<ElementId>
```

<a id="cc_engine-refs-elements"></a>

### fn `elements`

```rust
pub fn elements(p: &Project, ids: &[u64]) -> Result<Vec<ElementId>>
```

<a id="cc_engine-refs-category"></a>

### fn `category`

```rust
pub fn category(name: &str) -> Result<Category>
```

A category by its English name (`wall`, `door`…, `titleblock`), case-insensitive.

<a id="cc_engine-refs-view_category"></a>

### fn `view_category`

```rust
pub fn view_category(name: &str) -> Result<Category>
```

A category that views draw and filter (not title blocks), by name.

<a id="cc_engine-refs-category_name"></a>

### fn `category_name`

```rust
pub fn category_name(c: Category) -> String
```

<a id="cc_engine-refs-filter"></a>

### fn `filter`

```rust
pub fn filter(p: &Project, r: &Ref) -> Result<cc_core::FilterId>
```

A view filter by id or name.

<a id="cc_engine-refs-template"></a>

### fn `template`

```rust
pub fn template(p: &Project, r: &Ref) -> Result<cc_core::TemplateId>
```

A view template by id or name.

<a id="cc_engine-refs-phase"></a>

### fn `phase`

```rust
pub fn phase(p: &Project, r: &Ref) -> Result<cc_core::PhaseId>
```

A construction phase by id or name.

<a id="cc_engine-refs-option_set"></a>

### fn `option_set`

```rust
pub fn option_set(p: &Project, r: &Ref) -> Result<cc_core::OptionSetId>
```

A design option set by id or name.

<a id="cc_engine-refs-design_option"></a>

### fn `design_option`

```rust
pub fn design_option(p: &Project, r: &Ref) -> Result<cc_core::OptionId>
```

A design option by id, by name (unique in the project), or as
`"Set: Option"`.

<a id="cc_engine-refs-line_style"></a>

### fn `line_style`

```rust
pub fn line_style(p: &Project, r: &Ref) -> Result<cc_core::LineStyleId>
```

One of the project's line styles by id or name.

<a id="cc_engine-refs-fill_pattern"></a>

### fn `fill_pattern`

```rust
pub fn fill_pattern(p: &Project, r: &Ref) -> Result<cc_core::FillPatternId>
```

A custom fill pattern by id or name.

<a id="cc_engine-refs-profile"></a>

### fn `profile`

```rust
pub fn profile(p: &Project, r: &Ref) -> Result<cc_core::ProfileId>
```

A structural profile by id or name.

<a id="cc_engine-refs-phase_filter"></a>

### fn `phase_filter`

```rust
pub fn phase_filter(p: &Project, r: &Ref) -> Result<cc_core::PhaseFilterId>
```

A phase filter by id or name.

<a id="cc_engine-refs-nullable"></a>

### fn `nullable`

```rust
pub fn nullable<'de, D, T>(d: D) -> std::result::Result<Option<Option<T>>, D::Error> where D: serde::Deserializer<'de>, T: Deserialize<'de>
```

Deserialize an optional field that may also be `null`: absent gives
`None`, `null` gives `Some(None)` (use with `#[serde(default)]`).

<a id="cc_engine-refs-detail_level"></a>

### fn `detail_level`

```rust
pub fn detail_level(name: &str) -> Result<cc_core::DetailLevel>
```

A detail level by name: `coarse`, `medium` or `fine`.

<a id="cc_engine-refs-visual_style"></a>

### fn `visual_style`

```rust
pub fn visual_style(name: &str) -> Result<Option<cc_core::VisualStyle>>
```

A visual style by name (`wireframe`, `hidden_line`, `shaded`,
`consistent_colors`, `textures`), or `default` for the usual style of
the view's kind (`None`).

<a id="mod-cc_engine-registry"></a>

## `cc_engine::registry`

<sub>`crates/cc_engine/src/registry.rs`</sub>

**Items:** [`CommandKind`](#cc_engine-registry-commandkind) · [`Command`](#cc_engine-registry-command) · [`CommandInfo`](#cc_engine-registry-commandinfo) · [`describe`](#cc_engine-registry-describe)

<a id="cc_engine-registry-commandkind"></a>

### enum `CommandKind`

```rust
pub enum CommandKind
```

What a command does to the session.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`

**Variants**

- `Query`

  Reads the model; changes nothing.

- `Edit`

  Changes the model (one undo step).

- `File`

  Works with files or replaces the document.


<a id="cc_engine-registry-command"></a>

### trait `Command`

```rust
pub trait Command: DeserializeOwned + JsonSchema
```

A command: typed parameters and what running it does. The type's doc
comment is its description; its fields' doc comments describe the
parameters in the generated schema.

**Implemented by (344):** [`cc_engine::commands::analytical::AnalyticalExport`](cc_engine.md#cc_engine-commands-analytical-analyticalexport), [`cc_engine::commands::analytical::AnalyticalModelGet`](cc_engine.md#cc_engine-commands-analytical-analyticalmodelget), [`cc_engine::commands::annotate::DimensionChainCreate`](cc_engine.md#cc_engine-commands-annotate-dimensionchaincreate), [`cc_engine::commands::annotate::DimensionCreate`](cc_engine.md#cc_engine-commands-annotate-dimensioncreate), [`cc_engine::commands::annotate::DimensionOrdinateCreate`](cc_engine.md#cc_engine-commands-annotate-dimensionordinatecreate), [`cc_engine::commands::annotate::DimensionSetValue`](cc_engine.md#cc_engine-commands-annotate-dimensionsetvalue), [`cc_engine::commands::annotate::TagMove`](cc_engine.md#cc_engine-commands-annotate-tagmove), [`cc_engine::commands::annotate::TextAddLeader`](cc_engine.md#cc_engine-commands-annotate-textaddleader), [`cc_engine::commands::annotate::TextCreate`](cc_engine.md#cc_engine-commands-annotate-textcreate), [`cc_engine::commands::annotate::TextRemoveLeader`](cc_engine.md#cc_engine-commands-annotate-textremoveleader), [`cc_engine::commands::bar_marks::BarLengthCheck`](cc_engine.md#cc_engine-commands-bar_marks-barlengthcheck), [`cc_engine::commands::bar_marks::BarMarkingSet`](cc_engine.md#cc_engine-commands-bar_marks-barmarkingset), [`cc_engine::commands::bar_marks::BarMarksList`](cc_engine.md#cc_engine-commands-bar_marks-barmarkslist), [`cc_engine::commands::bar_marks::BarMarksRenumber`](cc_engine.md#cc_engine-commands-bar_marks-barmarksrenumber), [`cc_engine::commands::bar_marks::BarMarksUpdate`](cc_engine.md#cc_engine-commands-bar_marks-barmarksupdate), [`cc_engine::commands::bar_marks::BarTagCreate`](cc_engine.md#cc_engine-commands-bar_marks-bartagcreate), [`cc_engine::commands::bar_marks::BendingScheduleCreate`](cc_engine.md#cc_engine-commands-bar_marks-bendingschedulecreate), [`cc_engine::commands::bracing::BracingCreate`](cc_engine.md#cc_engine-commands-bracing-bracingcreate), [`cc_engine::commands::browser::BrowserSchemeCreate`](cc_engine.md#cc_engine-commands-browser-browserschemecreate), [`cc_engine::commands::browser::BrowserSchemeDelete`](cc_engine.md#cc_engine-commands-browser-browserschemedelete), [`cc_engine::commands::browser::BrowserSchemeSet`](cc_engine.md#cc_engine-commands-browser-browserschemeset), [`cc_engine::commands::browser::BrowserSchemeUse`](cc_engine.md#cc_engine-commands-browser-browserschemeuse), [`cc_engine::commands::browser::BrowserSchemesList`](cc_engine.md#cc_engine-commands-browser-browserschemeslist), [`cc_engine::commands::browser::BrowserTree`](cc_engine.md#cc_engine-commands-browser-browsertree), [`cc_engine::commands::cable_tray::CableTrayCreate`](cc_engine.md#cc_engine-commands-cable_tray-cabletraycreate), [`cc_engine::commands::cable_tray::CableTrayTypeCreate`](cc_engine.md#cc_engine-commands-cable_tray-cabletraytypecreate), [`cc_engine::commands::cabling::CablingList`](cc_engine.md#cc_engine-commands-cabling-cablinglist), [`cc_engine::commands::cabling::DataOutletPlace`](cc_engine.md#cc_engine-commands-cabling-dataoutletplace), [`cc_engine::commands::cabling::RackPlace`](cc_engine.md#cc_engine-commands-cabling-rackplace), [`cc_engine::commands::cad::CadImportFile`](cc_engine.md#cc_engine-commands-cad-cadimportfile), [`cc_engine::commands::cad::CadLayersSet`](cc_engine.md#cc_engine-commands-cad-cadlayersset), [`cc_engine::commands::cad::CadLayers`](cc_engine.md#cc_engine-commands-cad-cadlayers), [`cc_engine::commands::cad::ImageImport`](cc_engine.md#cc_engine-commands-cad-imageimport), [`cc_engine::commands::cad_explode::CadExplode`](cc_engine.md#cc_engine-commands-cad_explode-cadexplode), [`cc_engine::commands::callout::CalloutCreate`](cc_engine.md#cc_engine-commands-callout-calloutcreate), [`cc_engine::commands::callout::CalloutRemove`](cc_engine.md#cc_engine-commands-callout-calloutremove), [`cc_engine::commands::callout::CalloutsList`](cc_engine.md#cc_engine-commands-callout-calloutslist), [`cc_engine::commands::component::ComponentDimensionAdd`](cc_engine.md#cc_engine-commands-component-componentdimensionadd), [`cc_engine::commands::component::ComponentDimensionDelete`](cc_engine.md#cc_engine-commands-component-componentdimensiondelete), [`cc_engine::commands::component::ComponentDimensionSet`](cc_engine.md#cc_engine-commands-component-componentdimensionset), [`cc_engine::commands::component::ComponentEqualAdd`](cc_engine.md#cc_engine-commands-component-componentequaladd), [`cc_engine::commands::component::ComponentEqualDelete`](cc_engine.md#cc_engine-commands-component-componentequaldelete), [`cc_engine::commands::component::ComponentGet`](cc_engine.md#cc_engine-commands-component-componentget), [`cc_engine::commands::component::ComponentParamAdd`](cc_engine.md#cc_engine-commands-component-componentparamadd), [`cc_engine::commands::component::ComponentParamDelete`](cc_engine.md#cc_engine-commands-component-componentparamdelete), [`cc_engine::commands::component::ComponentParamSet`](cc_engine.md#cc_engine-commands-component-componentparamset), [`cc_engine::commands::component::ComponentRefLineAdd`](cc_engine.md#cc_engine-commands-component-componentreflineadd), [`cc_engine::commands::component::ComponentRefLineDelete`](cc_engine.md#cc_engine-commands-component-componentreflinedelete), [`cc_engine::commands::component::ComponentRefPlaneAdd`](cc_engine.md#cc_engine-commands-component-componentrefplaneadd), [`cc_engine::commands::component::ComponentRefPlaneDelete`](cc_engine.md#cc_engine-commands-component-componentrefplanedelete), [`cc_engine::commands::component::ComponentRefPlaneSet`](cc_engine.md#cc_engine-commands-component-componentrefplaneset), [`cc_engine::commands::component_connector::ComponentConnectorAdd`](cc_engine.md#cc_engine-commands-component_connector-componentconnectoradd), [`cc_engine::commands::component_connector::ComponentConnectorDelete`](cc_engine.md#cc_engine-commands-component_connector-componentconnectordelete), [`cc_engine::commands::component_connector::ComponentConnectorSet`](cc_engine.md#cc_engine-commands-component_connector-componentconnectorset), [`cc_engine::commands::component_piece::ComponentPieceAdd`](cc_engine.md#cc_engine-commands-component_piece-componentpieceadd), [`cc_engine::commands::component_piece::ComponentPieceDelete`](cc_engine.md#cc_engine-commands-component_piece-componentpiecedelete), [`cc_engine::commands::component_piece::ComponentPieceSet`](cc_engine.md#cc_engine-commands-component_piece-componentpieceset), [`cc_engine::commands::component_script::ComponentHotspotMove`](cc_engine.md#cc_engine-commands-component_script-componenthotspotmove), [`cc_engine::commands::component_script::ComponentParamsGet`](cc_engine.md#cc_engine-commands-component_script-componentparamsget), [`cc_engine::commands::component_script::ComponentParamsSet`](cc_engine.md#cc_engine-commands-component_script-componentparamsset), [`cc_engine::commands::component_script::ComponentScriptSet`](cc_engine.md#cc_engine-commands-component_script-componentscriptset), [`cc_engine::commands::connection::ConnectionsCreate`](cc_engine.md#cc_engine-commands-connection-connectionscreate), [`cc_engine::commands::create::BeamCreate`](cc_engine.md#cc_engine-commands-create-beamcreate), [`cc_engine::commands::create::CeilingCreate`](cc_engine.md#cc_engine-commands-create-ceilingcreate), [`cc_engine::commands::create::ColumnCreate`](cc_engine.md#cc_engine-commands-create-columncreate), [`cc_engine::commands::create::ComponentPlace`](cc_engine.md#cc_engine-commands-create-componentplace), [`cc_engine::commands::create::DoorCreate`](cc_engine.md#cc_engine-commands-create-doorcreate), [`cc_engine::commands::create::FloorCreate`](cc_engine.md#cc_engine-commands-create-floorcreate), [`cc_engine::commands::create::FloorOpeningAdd`](cc_engine.md#cc_engine-commands-create-flooropeningadd), [`cc_engine::commands::create::FloorOpeningRemove`](cc_engine.md#cc_engine-commands-create-flooropeningremove), [`cc_engine::commands::create::GridCreate`](cc_engine.md#cc_engine-commands-create-gridcreate), [`cc_engine::commands::create::RailingCreate`](cc_engine.md#cc_engine-commands-create-railingcreate), [`cc_engine::commands::create::RoofCreate`](cc_engine.md#cc_engine-commands-create-roofcreate), [`cc_engine::commands::create::RoofEdgeSet`](cc_engine.md#cc_engine-commands-create-roofedgeset), [`cc_engine::commands::create::RoofOpeningRemove`](cc_engine.md#cc_engine-commands-create-roofopeningremove), [`cc_engine::commands::create::RoomCreate`](cc_engine.md#cc_engine-commands-create-roomcreate), [`cc_engine::commands::create::WallCreate`](cc_engine.md#cc_engine-commands-create-wallcreate), [`cc_engine::commands::create::WindowCreate`](cc_engine.md#cc_engine-commands-create-windowcreate), [`cc_engine::commands::curtain::CurtainGet`](cc_engine.md#cc_engine-commands-curtain-curtainget), [`cc_engine::commands::curtain::CurtainGridSet`](cc_engine.md#cc_engine-commands-curtain-curtaingridset), [`cc_engine::commands::curtain::CurtainPanelSet`](cc_engine.md#cc_engine-commands-curtain-curtainpanelset), [`cc_engine::commands::curtain::CurtainTypeCreate`](cc_engine.md#cc_engine-commands-curtain-curtaintypecreate), [`cc_engine::commands::design_options::ElementsOptionSet`](cc_engine.md#cc_engine-commands-design_options-elementsoptionset), [`cc_engine::commands::design_options::OptionAcceptPrimary`](cc_engine.md#cc_engine-commands-design_options-optionacceptprimary), [`cc_engine::commands::design_options::OptionActiveSet`](cc_engine.md#cc_engine-commands-design_options-optionactiveset), [`cc_engine::commands::design_options::OptionCreate`](cc_engine.md#cc_engine-commands-design_options-optioncreate), [`cc_engine::commands::design_options::OptionDelete`](cc_engine.md#cc_engine-commands-design_options-optiondelete), [`cc_engine::commands::design_options::OptionSetCreate`](cc_engine.md#cc_engine-commands-design_options-optionsetcreate), [`cc_engine::commands::design_options::OptionSetDelete`](cc_engine.md#cc_engine-commands-design_options-optionsetdelete), [`cc_engine::commands::design_options::OptionSetProps`](cc_engine.md#cc_engine-commands-design_options-optionsetprops), [`cc_engine::commands::design_options::OptionSetRename`](cc_engine.md#cc_engine-commands-design_options-optionsetrename), [`cc_engine::commands::design_options::OptionsList`](cc_engine.md#cc_engine-commands-design_options-optionslist), [`cc_engine::commands::design_options::ViewOptionsSet`](cc_engine.md#cc_engine-commands-design_options-viewoptionsset), [`cc_engine::commands::detail::DetailLineCreate`](cc_engine.md#cc_engine-commands-detail-detaillinecreate), [`cc_engine::commands::detail_component::DetailComponentCreate`](cc_engine.md#cc_engine-commands-detail_component-detailcomponentcreate), [`cc_engine::commands::detail_component::DetailComponentPlace`](cc_engine.md#cc_engine-commands-detail_component-detailcomponentplace), [`cc_engine::commands::devices::DeviceTypeCreate`](cc_engine.md#cc_engine-commands-devices-devicetypecreate), [`cc_engine::commands::doors::DoorTypeCreate`](cc_engine.md#cc_engine-commands-doors-doortypecreate), [`cc_engine::commands::edit::ElementSet`](cc_engine.md#cc_engine-commands-edit-elementset), [`cc_engine::commands::edit::ElementsSetType`](cc_engine.md#cc_engine-commands-edit-elementssettype), [`cc_engine::commands::edit::ElementsSet`](cc_engine.md#cc_engine-commands-edit-elementsset), [`cc_engine::commands::fill_patterns::FillPatternCreate`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatterncreate), [`cc_engine::commands::fill_patterns::FillPatternDelete`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatterndelete), [`cc_engine::commands::fill_patterns::FillPatternDuplicate`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternduplicate), [`cc_engine::commands::fill_patterns::FillPatternExport`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternexport), [`cc_engine::commands::fill_patterns::FillPatternGet`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternget), [`cc_engine::commands::fill_patterns::FillPatternImport`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternimport), [`cc_engine::commands::fill_patterns::FillPatternSet`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternset), [`cc_engine::commands::fill_patterns::FillPatternsList`](cc_engine.md#cc_engine-commands-fill_patterns-fillpatternslist), [`cc_engine::commands::foundation::FootingCreate`](cc_engine.md#cc_engine-commands-foundation-footingcreate), [`cc_engine::commands::foundation::FootingsUnderColumns`](cc_engine.md#cc_engine-commands-foundation-footingsundercolumns), [`cc_engine::commands::foundation::FoundationSlabCreate`](cc_engine.md#cc_engine-commands-foundation-foundationslabcreate), [`cc_engine::commands::foundation::StripFootingCreate`](cc_engine.md#cc_engine-commands-foundation-stripfootingcreate), [`cc_engine::commands::foundation::StripFootingsUnderWalls`](cc_engine.md#cc_engine-commands-foundation-stripfootingsunderwalls), [`cc_engine::commands::framing::BraceCreate`](cc_engine.md#cc_engine-commands-framing-bracecreate), [`cc_engine::commands::framing::BracesBetweenColumns`](cc_engine.md#cc_engine-commands-framing-bracesbetweencolumns), [`cc_engine::commands::framing::TrussCreate`](cc_engine.md#cc_engine-commands-framing-trusscreate), [`cc_engine::commands::graph::GraphCheck`](cc_engine.md#cc_engine-commands-graph-graphcheck), [`cc_engine::commands::graph::GraphElements`](cc_engine.md#cc_engine-commands-graph-graphelements), [`cc_engine::commands::graph::GraphNodes`](cc_engine.md#cc_engine-commands-graph-graphnodes), [`cc_engine::commands::graph::GraphRelease`](cc_engine.md#cc_engine-commands-graph-graphrelease), [`cc_engine::commands::graph::GraphRun`](cc_engine.md#cc_engine-commands-graph-graphrun), [`cc_engine::commands::graphics::ElementHideInView`](cc_engine.md#cc_engine-commands-graphics-elementhideinview), [`cc_engine::commands::graphics::ElementOverrideSet`](cc_engine.md#cc_engine-commands-graphics-elementoverrideset), [`cc_engine::commands::graphics::FilterCreate`](cc_engine.md#cc_engine-commands-graphics-filtercreate), [`cc_engine::commands::graphics::FilterDelete`](cc_engine.md#cc_engine-commands-graphics-filterdelete), [`cc_engine::commands::graphics::FilterElements`](cc_engine.md#cc_engine-commands-graphics-filterelements), [`cc_engine::commands::graphics::FilterSet`](cc_engine.md#cc_engine-commands-graphics-filterset), [`cc_engine::commands::graphics::FiltersList`](cc_engine.md#cc_engine-commands-graphics-filterslist), [`cc_engine::commands::graphics::TemplateCreate`](cc_engine.md#cc_engine-commands-graphics-templatecreate), [`cc_engine::commands::graphics::TemplateDelete`](cc_engine.md#cc_engine-commands-graphics-templatedelete), [`cc_engine::commands::graphics::TemplateSet`](cc_engine.md#cc_engine-commands-graphics-templateset), [`cc_engine::commands::graphics::TemplatesList`](cc_engine.md#cc_engine-commands-graphics-templateslist), [`cc_engine::commands::graphics::ViewFilterAdd`](cc_engine.md#cc_engine-commands-graphics-viewfilteradd), [`cc_engine::commands::graphics::ViewFilterMove`](cc_engine.md#cc_engine-commands-graphics-viewfiltermove), [`cc_engine::commands::graphics::ViewFilterRemove`](cc_engine.md#cc_engine-commands-graphics-viewfilterremove), [`cc_engine::commands::graphics::ViewFilterSet`](cc_engine.md#cc_engine-commands-graphics-viewfilterset), [`cc_engine::commands::graphics::ViewGraphicsGet`](cc_engine.md#cc_engine-commands-graphics-viewgraphicsget), [`cc_engine::commands::graphics::ViewGraphicsSet`](cc_engine.md#cc_engine-commands-graphics-viewgraphicsset), [`cc_engine::commands::graphics::ViewTemplateApply`](cc_engine.md#cc_engine-commands-graphics-viewtemplateapply), [`cc_engine::commands::graphics::ViewTemplateAssign`](cc_engine.md#cc_engine-commands-graphics-viewtemplateassign), [`cc_engine::commands::graphics::ViewTemporaryApply`](cc_engine.md#cc_engine-commands-graphics-viewtemporaryapply), [`cc_engine::commands::graphics::ViewTemporary`](cc_engine.md#cc_engine-commands-graphics-viewtemporary), [`cc_engine::commands::grid_frame::GridFrameCreate`](cc_engine.md#cc_engine-commands-grid_frame-gridframecreate), [`cc_engine::commands::io::ComponentLibraryList`](cc_engine.md#cc_engine-commands-io-componentlibrarylist), [`cc_engine::commands::io::ComponentLoad`](cc_engine.md#cc_engine-commands-io-componentload), [`cc_engine::commands::io::ComponentSave`](cc_engine.md#cc_engine-commands-io-componentsave), [`cc_engine::commands::io::GltfImport`](cc_engine.md#cc_engine-commands-io-gltfimport), [`cc_engine::commands::io::IfcExport`](cc_engine.md#cc_engine-commands-io-ifcexport), [`cc_engine::commands::io::IfcImport`](cc_engine.md#cc_engine-commands-io-ifcimport), [`cc_engine::commands::io::ObjImport`](cc_engine.md#cc_engine-commands-io-objimport), [`cc_engine::commands::io::ScheduleExportCsv`](cc_engine.md#cc_engine-commands-io-scheduleexportcsv), [`cc_engine::commands::io::SheetExport`](cc_engine.md#cc_engine-commands-io-sheetexport), [`cc_engine::commands::io::SkpComponentLoad`](cc_engine.md#cc_engine-commands-io-skpcomponentload), [`cc_engine::commands::io::SkpImport`](cc_engine.md#cc_engine-commands-io-skpimport), [`cc_engine::commands::io::ViewExport`](cc_engine.md#cc_engine-commands-io-viewexport), [`cc_engine::commands::level_mark::LevelMarkCreate`](cc_engine.md#cc_engine-commands-level_mark-levelmarkcreate), [`cc_engine::commands::level_mark::LevelMarkTypeCreate`](cc_engine.md#cc_engine-commands-level_mark-levelmarktypecreate), [`cc_engine::commands::levels::LevelCreate`](cc_engine.md#cc_engine-commands-levels-levelcreate), [`cc_engine::commands::levels::LevelDelete`](cc_engine.md#cc_engine-commands-levels-leveldelete), [`cc_engine::commands::levels::LevelSet`](cc_engine.md#cc_engine-commands-levels-levelset), [`cc_engine::commands::lightning::AirTerminalPlace`](cc_engine.md#cc_engine-commands-lightning-airterminalplace), [`cc_engine::commands::lightning::EarthElectrodePlace`](cc_engine.md#cc_engine-commands-lightning-earthelectrodeplace), [`cc_engine::commands::lightning::EarthRingCreate`](cc_engine.md#cc_engine-commands-lightning-earthringcreate), [`cc_engine::commands::lightning::LpsCheck`](cc_engine.md#cc_engine-commands-lightning-lpscheck), [`cc_engine::commands::lightning::LpsConductorCreate`](cc_engine.md#cc_engine-commands-lightning-lpsconductorcreate), [`cc_engine::commands::lightning::LpsConductorTypeCreate`](cc_engine.md#cc_engine-commands-lightning-lpsconductortypecreate), [`cc_engine::commands::line_style::LineStyleCreate`](cc_engine.md#cc_engine-commands-line_style-linestylecreate), [`cc_engine::commands::line_style::LineStyleDelete`](cc_engine.md#cc_engine-commands-line_style-linestyledelete), [`cc_engine::commands::line_style::LineStyleSet`](cc_engine.md#cc_engine-commands-line_style-linestyleset), [`cc_engine::commands::line_style::LineStylesList`](cc_engine.md#cc_engine-commands-line_style-linestyleslist), [`cc_engine::commands::mass::MassCreate`](cc_engine.md#cc_engine-commands-mass-masscreate), [`cc_engine::commands::mass::MassFaceMove`](cc_engine.md#cc_engine-commands-mass-massfacemove), [`cc_engine::commands::mass::MassFloorsSet`](cc_engine.md#cc_engine-commands-mass-massfloorsset), [`cc_engine::commands::mass::MassFloors`](cc_engine.md#cc_engine-commands-mass-massfloors), [`cc_engine::commands::mass::MassRoofCreate`](cc_engine.md#cc_engine-commands-mass-massroofcreate), [`cc_engine::commands::mass::MassSlabsCreate`](cc_engine.md#cc_engine-commands-mass-massslabscreate), [`cc_engine::commands::mass::MassTypeCreate`](cc_engine.md#cc_engine-commands-mass-masstypecreate), [`cc_engine::commands::mass::MassWallsCreate`](cc_engine.md#cc_engine-commands-mass-masswallscreate), [`cc_engine::commands::materials::MaterialCreate`](cc_engine.md#cc_engine-commands-materials-materialcreate), [`cc_engine::commands::materials::MaterialDelete`](cc_engine.md#cc_engine-commands-materials-materialdelete), [`cc_engine::commands::materials::MaterialDuplicate`](cc_engine.md#cc_engine-commands-materials-materialduplicate), [`cc_engine::commands::materials::MaterialRename`](cc_engine.md#cc_engine-commands-materials-materialrename), [`cc_engine::commands::materials::MaterialSet`](cc_engine.md#cc_engine-commands-materials-materialset), [`cc_engine::commands::materials::MaterialTextureClear`](cc_engine.md#cc_engine-commands-materials-materialtextureclear), [`cc_engine::commands::materials::MaterialTextureSet`](cc_engine.md#cc_engine-commands-materials-materialtextureset), [`cc_engine::commands::mep::CircuitLoads`](cc_engine.md#cc_engine-commands-mep-circuitloads), [`cc_engine::commands::mep::ConduitCreate`](cc_engine.md#cc_engine-commands-mep-conduitcreate), [`cc_engine::commands::mep::MepConnect`](cc_engine.md#cc_engine-commands-mep-mepconnect), [`cc_engine::commands::mep::MepFittings`](cc_engine.md#cc_engine-commands-mep-mepfittings), [`cc_engine::commands::mep::MepSizeApply`](cc_engine.md#cc_engine-commands-mep-mepsizeapply), [`cc_engine::commands::mep::MepSizing`](cc_engine.md#cc_engine-commands-mep-mepsizing), [`cc_engine::commands::mep::PipeCreate`](cc_engine.md#cc_engine-commands-mep-pipecreate), [`cc_engine::commands::mep::SystemAssign`](cc_engine.md#cc_engine-commands-mep-systemassign), [`cc_engine::commands::mep::SystemCreate`](cc_engine.md#cc_engine-commands-mep-systemcreate), [`cc_engine::commands::mep::SystemDelete`](cc_engine.md#cc_engine-commands-mep-systemdelete), [`cc_engine::commands::mep::SystemSet`](cc_engine.md#cc_engine-commands-mep-systemset), [`cc_engine::commands::mep::SystemsList`](cc_engine.md#cc_engine-commands-mep-systemslist), [`cc_engine::commands::modify::ElementAlign`](cc_engine.md#cc_engine-commands-modify-elementalign), [`cc_engine::commands::modify::ElementFlip`](cc_engine.md#cc_engine-commands-modify-elementflip), [`cc_engine::commands::modify::ElementMoveEnd`](cc_engine.md#cc_engine-commands-modify-elementmoveend), [`cc_engine::commands::modify::ElementOffset`](cc_engine.md#cc_engine-commands-modify-elementoffset), [`cc_engine::commands::modify::ElementSplit`](cc_engine.md#cc_engine-commands-modify-elementsplit), [`cc_engine::commands::modify::ElementsCopy`](cc_engine.md#cc_engine-commands-modify-elementscopy), [`cc_engine::commands::modify::ElementsDelete`](cc_engine.md#cc_engine-commands-modify-elementsdelete), [`cc_engine::commands::modify::ElementsExport`](cc_engine.md#cc_engine-commands-modify-elementsexport), [`cc_engine::commands::modify::ElementsMirror`](cc_engine.md#cc_engine-commands-modify-elementsmirror), [`cc_engine::commands::modify::ElementsMove`](cc_engine.md#cc_engine-commands-modify-elementsmove), [`cc_engine::commands::modify::ElementsPaste`](cc_engine.md#cc_engine-commands-modify-elementspaste), [`cc_engine::commands::modify::ElementsPin`](cc_engine.md#cc_engine-commands-modify-elementspin), [`cc_engine::commands::modify::ElementsRotate`](cc_engine.md#cc_engine-commands-modify-elementsrotate), [`cc_engine::commands::modify::ElementsTurn`](cc_engine.md#cc_engine-commands-modify-elementsturn), [`cc_engine::commands::modify::GeometryJoin`](cc_engine.md#cc_engine-commands-modify-geometryjoin), [`cc_engine::commands::modify::GeometryUnjoin`](cc_engine.md#cc_engine-commands-modify-geometryunjoin), [`cc_engine::commands::modify::GridsAlignEnds`](cc_engine.md#cc_engine-commands-modify-gridsalignends), [`cc_engine::commands::modify::GroupCreate`](cc_engine.md#cc_engine-commands-modify-groupcreate), [`cc_engine::commands::modify::GroupUngroup`](cc_engine.md#cc_engine-commands-modify-groupungroup), [`cc_engine::commands::modify::TrimCorner`](cc_engine.md#cc_engine-commands-modify-trimcorner), [`cc_engine::commands::modify::TrimExtend`](cc_engine.md#cc_engine-commands-modify-trimextend), [`cc_engine::commands::modify::WallAttach`](cc_engine.md#cc_engine-commands-modify-wallattach), [`cc_engine::commands::modify::WallJoin`](cc_engine.md#cc_engine-commands-modify-walljoin), [`cc_engine::commands::niche::NicheCreate`](cc_engine.md#cc_engine-commands-niche-nichecreate), [`cc_engine::commands::niche::NicheTypeCreate`](cc_engine.md#cc_engine-commands-niche-nichetypecreate), [`cc_engine::commands::pad::PadCreate`](cc_engine.md#cc_engine-commands-pad-padcreate), [`cc_engine::commands::pad::PadTypeCreate`](cc_engine.md#cc_engine-commands-pad-padtypecreate), [`cc_engine::commands::paint::PaintRemove`](cc_engine.md#cc_engine-commands-paint-paintremove), [`cc_engine::commands::paint::Paint`](cc_engine.md#cc_engine-commands-paint-paint), [`cc_engine::commands::paint::PaintsList`](cc_engine.md#cc_engine-commands-paint-paintslist), [`cc_engine::commands::phases::ElementsDemolish`](cc_engine.md#cc_engine-commands-phases-elementsdemolish), [`cc_engine::commands::phases::ElementsPhaseSet`](cc_engine.md#cc_engine-commands-phases-elementsphaseset), [`cc_engine::commands::phases::PhaseCreate`](cc_engine.md#cc_engine-commands-phases-phasecreate), [`cc_engine::commands::phases::PhaseCreationSet`](cc_engine.md#cc_engine-commands-phases-phasecreationset), [`cc_engine::commands::phases::PhaseDelete`](cc_engine.md#cc_engine-commands-phases-phasedelete), [`cc_engine::commands::phases::PhaseFilterCreate`](cc_engine.md#cc_engine-commands-phases-phasefiltercreate), [`cc_engine::commands::phases::PhaseFilterDelete`](cc_engine.md#cc_engine-commands-phases-phasefilterdelete), [`cc_engine::commands::phases::PhaseFilterSet`](cc_engine.md#cc_engine-commands-phases-phasefilterset), [`cc_engine::commands::phases::PhaseFiltersList`](cc_engine.md#cc_engine-commands-phases-phasefilterslist), [`cc_engine::commands::phases::PhaseGraphicsGet`](cc_engine.md#cc_engine-commands-phases-phasegraphicsget), [`cc_engine::commands::phases::PhaseGraphicsSet`](cc_engine.md#cc_engine-commands-phases-phasegraphicsset), [`cc_engine::commands::phases::PhaseSet`](cc_engine.md#cc_engine-commands-phases-phaseset), [`cc_engine::commands::phases::PhasesList`](cc_engine.md#cc_engine-commands-phases-phaseslist), [`cc_engine::commands::piles::PileCapCreate`](cc_engine.md#cc_engine-commands-piles-pilecapcreate), [`cc_engine::commands::piles::PileCapTypeCreate`](cc_engine.md#cc_engine-commands-piles-pilecaptypecreate), [`cc_engine::commands::piles::PileCapsUnderColumns`](cc_engine.md#cc_engine-commands-piles-pilecapsundercolumns), [`cc_engine::commands::piles::PileCreate`](cc_engine.md#cc_engine-commands-piles-pilecreate), [`cc_engine::commands::piles::PileTypeCreate`](cc_engine.md#cc_engine-commands-piles-piletypecreate), [`cc_engine::commands::piles::TieBeamsCreate`](cc_engine.md#cc_engine-commands-piles-tiebeamscreate), [`cc_engine::commands::profiles::ProfileCatalog`](cc_engine.md#cc_engine-commands-profiles-profilecatalog), [`cc_engine::commands::profiles::ProfileCreate`](cc_engine.md#cc_engine-commands-profiles-profilecreate), [`cc_engine::commands::profiles::ProfileDelete`](cc_engine.md#cc_engine-commands-profiles-profiledelete), [`cc_engine::commands::profiles::ProfileDuplicate`](cc_engine.md#cc_engine-commands-profiles-profileduplicate), [`cc_engine::commands::profiles::ProfileGet`](cc_engine.md#cc_engine-commands-profiles-profileget), [`cc_engine::commands::profiles::ProfileImport`](cc_engine.md#cc_engine-commands-profiles-profileimport), [`cc_engine::commands::profiles::ProfileSet`](cc_engine.md#cc_engine-commands-profiles-profileset), [`cc_engine::commands::profiles::ProfileTypeCreate`](cc_engine.md#cc_engine-commands-profiles-profiletypecreate), [`cc_engine::commands::profiles::ProfilesList`](cc_engine.md#cc_engine-commands-profiles-profileslist), [`cc_engine::commands::project::LanguageSet`](cc_engine.md#cc_engine-commands-project-languageset), [`cc_engine::commands::project::ProjectInfoGet`](cc_engine.md#cc_engine-commands-project-projectinfoget), [`cc_engine::commands::project::ProjectInfoSet`](cc_engine.md#cc_engine-commands-project-projectinfoset), [`cc_engine::commands::project::ProjectNew`](cc_engine.md#cc_engine-commands-project-projectnew), [`cc_engine::commands::project::ProjectOpen`](cc_engine.md#cc_engine-commands-project-projectopen), [`cc_engine::commands::project::ProjectSave`](cc_engine.md#cc_engine-commands-project-projectsave), [`cc_engine::commands::project::Redo`](cc_engine.md#cc_engine-commands-project-redo), [`cc_engine::commands::project::Undo`](cc_engine.md#cc_engine-commands-project-undo), [`cc_engine::commands::project::UnitsGet`](cc_engine.md#cc_engine-commands-project-unitsget), [`cc_engine::commands::project::UnitsSet`](cc_engine.md#cc_engine-commands-project-unitsset), [`cc_engine::commands::query::CategoriesList`](cc_engine.md#cc_engine-commands-query-categorieslist), [`cc_engine::commands::query::ElementGet`](cc_engine.md#cc_engine-commands-query-elementget), [`cc_engine::commands::query::ElementsList`](cc_engine.md#cc_engine-commands-query-elementslist), [`cc_engine::commands::query::GroupsList`](cc_engine.md#cc_engine-commands-query-groupslist), [`cc_engine::commands::query::LevelsList`](cc_engine.md#cc_engine-commands-query-levelslist), [`cc_engine::commands::query::MaterialGet`](cc_engine.md#cc_engine-commands-query-materialget), [`cc_engine::commands::query::MaterialsList`](cc_engine.md#cc_engine-commands-query-materialslist), [`cc_engine::commands::query::Measure`](cc_engine.md#cc_engine-commands-query-measure), [`cc_engine::commands::query::ScheduleTable`](cc_engine.md#cc_engine-commands-query-scheduletable), [`cc_engine::commands::query::TypeGet`](cc_engine.md#cc_engine-commands-query-typeget), [`cc_engine::commands::query::TypesList`](cc_engine.md#cc_engine-commands-query-typeslist), [`cc_engine::commands::query::ViewGet`](cc_engine.md#cc_engine-commands-query-viewget), [`cc_engine::commands::query::ViewsList`](cc_engine.md#cc_engine-commands-query-viewslist), [`cc_engine::commands::ramp::RampCheck`](cc_engine.md#cc_engine-commands-ramp-rampcheck), [`cc_engine::commands::ramp::RampCreate`](cc_engine.md#cc_engine-commands-ramp-rampcreate), [`cc_engine::commands::ramp::RampTypeCreate`](cc_engine.md#cc_engine-commands-ramp-ramptypecreate), [`cc_engine::commands::rebar::RebarCopy`](cc_engine.md#cc_engine-commands-rebar-rebarcopy), [`cc_engine::commands::rebar::RebarCreate`](cc_engine.md#cc_engine-commands-rebar-rebarcreate), [`cc_engine::commands::rebar::RebarTypical`](cc_engine.md#cc_engine-commands-rebar-rebartypical), [`cc_engine::commands::region::FilledRegionCreate`](cc_engine.md#cc_engine-commands-region-filledregioncreate), [`cc_engine::commands::region::FilledRegionTypeCreate`](cc_engine.md#cc_engine-commands-region-filledregiontypecreate), [`cc_engine::commands::region::MaskingRegionCreate`](cc_engine.md#cc_engine-commands-region-maskingregioncreate), [`cc_engine::commands::render::ViewRender`](cc_engine.md#cc_engine-commands-render-viewrender), [`cc_engine::commands::roofing::RoofCheck`](cc_engine.md#cc_engine-commands-roofing-roofcheck), [`cc_engine::commands::schedule::ScheduleGet`](cc_engine.md#cc_engine-commands-schedule-scheduleget), [`cc_engine::commands::schedule::ScheduleSet`](cc_engine.md#cc_engine-commands-schedule-scheduleset), [`cc_engine::commands::shaft::ShaftCreate`](cc_engine.md#cc_engine-commands-shaft-shaftcreate), [`cc_engine::commands::shaft::ShaftTypeCreate`](cc_engine.md#cc_engine-commands-shaft-shafttypecreate), [`cc_engine::commands::sheets::ComponentSheetPieceAdd`](cc_engine.md#cc_engine-commands-sheets-componentsheetpieceadd), [`cc_engine::commands::sheets::ComponentSheetPieceDelete`](cc_engine.md#cc_engine-commands-sheets-componentsheetpiecedelete), [`cc_engine::commands::sheets::ComponentSheetPieceSet`](cc_engine.md#cc_engine-commands-sheets-componentsheetpieceset), [`cc_engine::commands::sheets::SheetFields`](cc_engine.md#cc_engine-commands-sheets-sheetfields), [`cc_engine::commands::sheets::SheetSet`](cc_engine.md#cc_engine-commands-sheets-sheetset), [`cc_engine::commands::sheets::TitleBlockCreate`](cc_engine.md#cc_engine-commands-sheets-titleblockcreate), [`cc_engine::commands::site::SiteContextImport`](cc_engine.md#cc_engine-commands-site-sitecontextimport), [`cc_engine::commands::site::SiteGet`](cc_engine.md#cc_engine-commands-site-siteget), [`cc_engine::commands::site::SiteSet`](cc_engine.md#cc_engine-commands-site-siteset), [`cc_engine::commands::sketch::SketchGet`](cc_engine.md#cc_engine-commands-sketch-sketchget), [`cc_engine::commands::sketch::SketchSet`](cc_engine.md#cc_engine-commands-sketch-sketchset), [`cc_engine::commands::slab_rebar::SlabRebarCreate`](cc_engine.md#cc_engine-commands-slab_rebar-slabrebarcreate), [`cc_engine::commands::stair::RailingHostSet`](cc_engine.md#cc_engine-commands-stair-railinghostset), [`cc_engine::commands::stair::StairCheck`](cc_engine.md#cc_engine-commands-stair-staircheck), [`cc_engine::commands::stair::StairCreate`](cc_engine.md#cc_engine-commands-stair-staircreate), [`cc_engine::commands::stair::StairRailingsAdd`](cc_engine.md#cc_engine-commands-stair-stairrailingsadd), [`cc_engine::commands::steel_building::SteelBuildingCreate`](cc_engine.md#cc_engine-commands-steel_building-steelbuildingcreate), [`cc_engine::commands::strip_rebar::StripRebarCreate`](cc_engine.md#cc_engine-commands-strip_rebar-striprebarcreate), [`cc_engine::commands::structure::BeamsBetweenColumns`](cc_engine.md#cc_engine-commands-structure-beamsbetweencolumns), [`cc_engine::commands::structure::BeamsOnGrids`](cc_engine.md#cc_engine-commands-structure-beamsongrids), [`cc_engine::commands::structure::ColumnsAtGrids`](cc_engine.md#cc_engine-commands-structure-columnsatgrids), [`cc_engine::commands::structure::SlabJoinSupports`](cc_engine.md#cc_engine-commands-structure-slabjoinsupports), [`cc_engine::commands::structure::StructureNumber`](cc_engine.md#cc_engine-commands-structure-structurenumber), [`cc_engine::commands::structure_material::StructuralTypeFor`](cc_engine.md#cc_engine-commands-structure_material-structuraltypefor), [`cc_engine::commands::sun::SunFrames`](cc_engine.md#cc_engine-commands-sun-sunframes), [`cc_engine::commands::sun::SunPosition`](cc_engine.md#cc_engine-commands-sun-sunposition), [`cc_engine::commands::sun::SunPresetDelete`](cc_engine.md#cc_engine-commands-sun-sunpresetdelete), [`cc_engine::commands::sun::SunPresetRename`](cc_engine.md#cc_engine-commands-sun-sunpresetrename), [`cc_engine::commands::sun::SunPresetSave`](cc_engine.md#cc_engine-commands-sun-sunpresetsave), [`cc_engine::commands::sun::SunPresetsList`](cc_engine.md#cc_engine-commands-sun-sunpresetslist), [`cc_engine::commands::sweep::SweepCreate`](cc_engine.md#cc_engine-commands-sweep-sweepcreate), [`cc_engine::commands::sweep::SweepProfileSet`](cc_engine.md#cc_engine-commands-sweep-sweepprofileset), [`cc_engine::commands::sweep::SweepTypeCreate`](cc_engine.md#cc_engine-commands-sweep-sweeptypecreate), [`cc_engine::commands::topography::TopographyCreate`](cc_engine.md#cc_engine-commands-topography-topographycreate), [`cc_engine::commands::topography::TopographyPointsSet`](cc_engine.md#cc_engine-commands-topography-topographypointsset), [`cc_engine::commands::topography::TopographyTypeCreate`](cc_engine.md#cc_engine-commands-topography-topographytypecreate), [`cc_engine::commands::types::ComponentCreate`](cc_engine.md#cc_engine-commands-types-componentcreate), [`cc_engine::commands::types::TypeDelete`](cc_engine.md#cc_engine-commands-types-typedelete), [`cc_engine::commands::types::TypeDuplicate`](cc_engine.md#cc_engine-commands-types-typeduplicate), [`cc_engine::commands::types::TypeSet`](cc_engine.md#cc_engine-commands-types-typeset), [`cc_engine::commands::valve::ValvePlace`](cc_engine.md#cc_engine-commands-valve-valveplace), [`cc_engine::commands::valve::ValveTypeCreate`](cc_engine.md#cc_engine-commands-valve-valvetypecreate), [`cc_engine::commands::views::SheetPlaceView`](cc_engine.md#cc_engine-commands-views-sheetplaceview), [`cc_engine::commands::views::SheetRemoveViewport`](cc_engine.md#cc_engine-commands-views-sheetremoveviewport), [`cc_engine::commands::views::ViewCreate`](cc_engine.md#cc_engine-commands-views-viewcreate), [`cc_engine::commands::views::ViewDelete`](cc_engine.md#cc_engine-commands-views-viewdelete), [`cc_engine::commands::views::ViewDuplicate`](cc_engine.md#cc_engine-commands-views-viewduplicate), [`cc_engine::commands::views::ViewSet`](cc_engine.md#cc_engine-commands-views-viewset), [`cc_engine::commands::wall_framing::WallFramingCreate`](cc_engine.md#cc_engine-commands-wall_framing-wallframingcreate), [`cc_engine::commands::wall_rebar::WallRebarCreate`](cc_engine.md#cc_engine-commands-wall_rebar-wallrebarcreate)

**Required and provided items**

- `const NAME: &'static str`
- `const KIND: CommandKind`
- `const READS_FILES: bool`

  Whether it may read a file named by one of its parameters (an import
  of an edit or query command); it then refuses the path when the run
  may not work with files (`Session::require_files`).

- `fn run(self, session: &mut Session) -> Result<Value>`

<a id="cc_engine-registry-commandinfo"></a>

### struct `CommandInfo`

```rust
pub struct CommandInfo
```

A command as listed by the catalog.

**Traits:** `Debug`, `Clone`, `Serialize`

**Fields**

- `name: String`
- `kind: CommandKind`
- `group: String`

  Area of the catalog (project, queries, elements…).

- `description: String`
- `input_schema: Value`

  JSON Schema of the parameters object: no references, valid under draft 7 and 2020-12.

- `reads_files: bool`

  May read a file named by a parameter (besides `File` commands, which
  all work with files).


<a id="cc_engine-registry-describe"></a>

### fn `describe`

```rust
pub fn describe(name: &str) -> Option<CommandInfo>
```

The catalog entry of one command.

<a id="mod-cc_engine-script"></a>

## `cc_engine::script`

<sub>`crates/cc_engine/src/script.rs`</sub>

Scripts (macros): a list of commands with their arguments, as a JSON
array of `{"command": ..., "args": {...}}` objects or one such object per
line (`//` lines are comments). `ccad script` runs them, the app records
and plays them as macros.
Arguments may hold two kinds of placeholders, resolved just before their
step runs: `{"$step": n}` (or `{"$step": n, "path": "/created/0"}`) is a
value of the result of the earlier step `n` (counted from 1; `path` is a
JSON pointer, `/id` by default), so a step can use what an earlier one
created; `{"$level": "active"}` is the active level (see
`Session::set_active_level`; without one, the lowest level). Recording
writes both (see `link` and `mark_level`).

**Items:** [`STEP_KEY`](#cc_engine-script-step_key) · [`LEVEL_KEY`](#cc_engine-script-level_key) · [`Step`](#cc_engine-script-step) · [`parse`](#cc_engine-script-parse) · [`to_text`](#cc_engine-script-to_text) · [`StepError`](#cc_engine-script-steperror) · [`run`](#cc_engine-script-run) · [`Made`](#cc_engine-script-made) · [`link`](#cc_engine-script-link) · [`mark_level`](#cc_engine-script-mark_level)

<a id="cc_engine-script-step_key"></a>

### const `STEP_KEY`

```rust
pub const STEP_KEY: &str
```

Key of a reference to an earlier step's result.

<a id="cc_engine-script-level_key"></a>

### const `LEVEL_KEY`

```rust
pub const LEVEL_KEY: &str
```

Key of the active level placeholder (its value is `"active"`).

<a id="cc_engine-script-step"></a>

### struct `Step`

```rust
pub struct Step
```

One step of a script.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `command: String`
- `args: Value`

**Methods**

- `pub fn to_json(&self) -> Value`

<a id="cc_engine-script-parse"></a>

### fn `parse`

```rust
pub fn parse(text: &str) -> Result<Vec<Step>>
```

Read a script.

<a id="cc_engine-script-to_text"></a>

### fn `to_text`

```rust
pub fn to_text(steps: &[Step]) -> String
```

A script with one step per line (the format macros are saved in).

<a id="cc_engine-script-steperror"></a>

### struct `StepError`

```rust
pub struct StepError
```

A failed step: its index (from 0) and the error.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `index: usize`
- `command: String`
- `error: EngineError`

<a id="cc_engine-script-run"></a>

### fn `run`

```rust
pub fn run(session: &mut Session, steps: &[Step]) -> std::result::Result<Vec<Value>, (Vec<Value>, StepError)>
```

Run the steps in order, stopping at the first error; returns each
completed step's result. Placeholders in the arguments are resolved first
(see the module documentation).

<a id="cc_engine-script-made"></a>

### struct `Made`

```rust
pub(crate) struct Made
```

What a recorded step made: the ids allocated while it ran (ids are shared
by every kind of object, so none of them names an older object) and its
result.

**Traits:** `Debug`, `Clone`

**Fields**

- `ids: std::ops::Range<u64>`
- `result: Value`

<a id="cc_engine-script-link"></a>

### fn `link`

```rust
pub(crate) fn link(steps: &mut [Step], made: &[Option<Made>])
```

Replace the ids that earlier steps created by references to their results
(`made[i]` is what `steps[i]` made). Integers at keys of counts and
indices, and ids no earlier result holds, stay as they are.

<a id="cc_engine-script-mark_level"></a>

### fn `mark_level`

```rust
pub(crate) fn mark_level(v: &mut Value, level: LevelId, name: &str, depth: usize)
```

Replace the references to `level` (by id or name) at `level` keys by the
active level placeholder, so the step plays on the level active then.

<a id="mod-cc_engine-session"></a>

## `cc_engine::session`

<sub>`crates/cc_engine/src/session.rs`</sub>

**Items:** [`Session`](#cc_engine-session-session) · [`new_project`](#cc_engine-session-new_project) · [`new_project_with`](#cc_engine-session-new_project_with) · [`guard`](#cc_engine-session-guard)

<a id="cc_engine-session-session"></a>

### struct `Session`

```rust
pub struct Session
```

An open project with its derived geometry and drawings, rebuilt lazily
when the document changes.

**Traits:** `Default`

**Fields**

- `doc: Document`

*Some fields are private.*

**Methods**

- `pub fn new(doc: Document) -> Self`
- `pub fn open(path: &std::path::Path) -> Result<(Self, usize)>`

  Open a `.ccad` file; returns the session and the number of repairs made.

- `pub fn replace(&mut self, doc: Document)`

  Replace the document (new, open, import), dropping derived data.

- `pub fn project(&self) -> &Project`
- `pub fn sync(&mut self) -> bool`

  Rebuild derived geometry if the document changed; returns whether it did.

- `pub fn view_geometry(&mut self, view: ViewId) -> &ModelGeometry`

  Geometry of the model as `view` shows it, in its phase: only the
  elements of that phase join, open walls and bound rooms. The whole
  model's geometry when the phase leaves nothing out.

- `pub fn geom_for(&self, view: ViewId) -> &ModelGeometry`

  `Session::view_geometry` as of the last sync (the GUI calls
  `Session::view_geometry` once a frame per open view first); falls
  back to the whole model's geometry when the phase's is not built.

- `pub fn element_geometry(&mut self, id: cc_core::ElementId) -> &ModelGeometry`

  Geometry of the model in which element `id` stands: the whole
  model's, or, for an element of a design option views do not show by
  default, the main model with that option (cached like a view's).

- `pub fn geom(&self) -> &ModelGeometry`

  Derived geometry as of the last `Session::sync` (callers that keep
  it current, like the GUI once a frame, read it without `&mut`).

- `pub fn geometry(&mut self) -> &ModelGeometry`

  Derived geometry of the current revision.

- `pub fn project_and_geometry(&mut self) -> (&Project, &ModelGeometry)`

  The project with its current geometry.

- `pub fn drawing(&mut self, view: ViewId) -> Option<Rc<Drawing2D>>`

  Drawing of a graphical view, with what is temporarily hidden or
  isolated in it left out (cached per revision).

- `pub fn printed_drawing(&mut self, view: ViewId) -> Option<Rc<Drawing2D>>`

  Drawing of a view as printed: without temporary hiding or isolation,
  and without its underlay unless the view prints it.

- `pub fn temporary(&self, view: ViewId) -> Option<&cc_core::TemporaryVisibility>`

  What is temporarily hidden or isolated in `view`.

- `pub fn set_temporary(&mut self, view: ViewId, temporary: Option<cc_core::TemporaryVisibility>)`

  Set (or, with `None` or an empty set, reset) what is temporarily
  hidden or isolated in `view`. Not an edit: nothing to undo or save.

- `pub fn set_view_camera(&mut self, view: ViewId, camera: cc_core::CameraState)`

  Keep the camera of 3D view `view` (not undoable). The model's geometry
  stays: only the view's drawing and the sheets showing it are redrawn.

- `pub fn temporary_revision(&self) -> u64`

  Changes whenever any view's temporary visibility changes.

- `pub fn invalidate_drawings(&mut self)`

  Drop cached drawings (after a language change alters their text).

- `pub fn sheet(&mut self, view: ViewId) -> Option<Rc<cc_views::Sheet>>`

  Composition of a sheet in paper millimeters (cached per revision).

- `pub fn schedule(&mut self, view: ViewId) -> Option<Rc<cc_views::Table>>`

  A schedule table of a schedule view, with the project's units: the
  elements its phase and phase filter show (cached per revision).

- `pub fn run<C: crate::Command + serde::Serialize>(&mut self, command: C) -> Result<Value>`

  Run a typed command (the way the GUI calls the engine): it goes
  through `Session::execute` like a command from the CLI or MCP.

- `pub fn run_merged<C: crate::Command + serde::Serialize>(&mut self, key: u64, command: C) -> Result<Value>`

  Run a typed command whose edit merges with the previous one of the
  same `key` into one undo step (a value being dragged); a different
  key, an unmerged edit or `Document::break_merge` starts a new step.

- `pub(crate) fn take_merge_key(&mut self) -> Option<u64>`

  The merge key for the edit being run.

- `pub fn start_recording(&mut self)`

  Start recording the commands that change the model (a macro).

- `pub fn is_recording(&self) -> bool`
- `pub fn stop_recording(&mut self) -> Vec<crate::script::Step>`

  Stop recording; returns the recorded steps (a dragged value counts once,
  with its final value). Ids of objects an earlier step created become
  `{"$step": n}` references and the active level `{"$level": "active"}`
  (see `crate::script`), so the macro plays on any level and project.

- `pub fn set_active_level(&mut self, level: Option<cc_core::LevelId>)`

  Set the level of the view the user works in (`None` outside plans);
  see `Session::active_level`.

- `pub fn level(&self, r: &crate::Ref) -> Result<cc_core::LevelId>`

  A level by id or name (`ccad script level=…`).

- `pub fn active_level(&self) -> Option<cc_core::LevelId>`

  The level of the view the user works in, if it exists: recording
  writes it as the active level placeholder, and scripts play that
  placeholder on it.

- `pub fn undo_to(&mut self, depth: usize)`

  Undo back to undo depth `depth` (cancelling a dialog), dropping what
  was recorded since.

- `pub fn set_extension(&mut self, extension: Option<Box<dyn crate::Extension>>) -> Option<Box<dyn crate::Extension>>`

  Set the source of commands beyond the built-in catalog (plugins), or
  remove it with `None`; returns the one it replaces.

- `pub fn extension(&self) -> Option<&dyn crate::Extension>`
- `pub fn take_extension_messages(&mut self) -> Vec<String>`

  The extension's messages for the user since the last call.

- `pub fn called_by_command(&self) -> bool`

  Whether the command running was called by another command (a plugin
  command), not by the user.

- `pub fn is_nested(&self) -> bool`

  Whether a command is running (a plugin command running others).

- `pub fn files_allowed(&self) -> bool`

  Whether the commands running may read and write files: true unless a
  caller without file access (a plugin without the `files` permission,
  a graph run without `allow_files`) runs them.

- `pub fn with_files<R>(&mut self, allowed: bool, f: impl FnOnce(&mut Session) -> R) -> R`

  Run `f` with file access narrowed to `allowed` (it is never widened:
  a caller without files cannot give them to what it runs).

- `pub fn require_files(&self, param: &str) -> Result<()>`

  Refuse a file path given to a command (in its parameter `param`) when
  the commands running may not work with files.

- `pub fn commands(&self) -> Vec<crate::CommandInfo>`

  Every command this session runs: the built-in catalog, then the
  extension's commands.

- `pub fn describe(&self, name: &str) -> Option<crate::CommandInfo>`

  The catalog entry of one command, built-in or from the extension.

- `pub fn kind_of(&self, name: &str) -> Option<crate::CommandKind>`

  The kind of a command, built-in or from the extension.

- `pub fn execute(&mut self, name: &str, args: Value) -> Result<Value>`

  Run the command `name` with JSON `args`. A panic inside the command
  becomes an internal error and the document is kept. Commands of the
  extension (names with `__`) are one undo step however many commands
  they run, and leave nothing behind when they fail.


<a id="cc_engine-session-new_project"></a>

### fn `new_project`

```rust
pub fn new_project() -> Project
```

A new project from the metric template, named in the active language.

<a id="cc_engine-session-new_project_with"></a>

### fn `new_project_with`

```rust
pub fn new_project_with(system: cc_core::MeasurementSystem) -> Project
```

A new project from the template of `system`, named in the active language.

<a id="cc_engine-session-guard"></a>

### fn `guard`

```rust
pub(crate) fn guard<T>(f: impl FnOnce() -> T) -> Result<T>
```

Last-resort guard: a panic becomes an internal error.

<a id="mod-cc_engine-skp"></a>

## `cc_engine::skp`

<sub>`crates/cc_engine/src/skp.rs`</sub>

SketchUp (.skp) models as component types: the geometry of each placed
component or group becomes a type of fixed mesh pieces (one per
material), placed by an element at the instance's position and plan
rotation. Whatever else the instance's transform holds (tilt, scale,
mirror) is baked into the type, so placements that differ only by a
move or a plan rotation share one type.

**Items:** [`MAX_FACES`](#cc_engine-skp-max_faces) · [`SkpType`](#cc_engine-skp-skptype) · [`SkpPlacement`](#cc_engine-skp-skpplacement) · [`Converted`](#cc_engine-skp-converted) · [`SkpError`](#cc_engine-skp-skperror) · [`MeshBuild`](#cc_engine-skp-meshbuild) · [`mesh_definition`](#cc_engine-skp-mesh_definition) · [`convert`](#cc_engine-skp-convert) · [`convert_single`](#cc_engine-skp-convert_single)

<a id="cc_engine-skp-max_faces"></a>

### const `MAX_FACES`

```rust
pub const MAX_FACES: usize
```

Faces written over the whole import.

<a id="cc_engine-skp-skptype"></a>

### struct `SkpType`

```rust
pub struct SkpType
```

A component type to add: its name and definition.

**Traits:** `Debug`, `Clone`

**Fields**

- `name: String`
- `def: ComponentDef`

<a id="cc_engine-skp-skpplacement"></a>

### struct `SkpPlacement`

```rust
pub struct SkpPlacement
```

An element placing `types[ty]`, in meters and degrees, before the
import's own move and rotation.

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `ty: usize`
- `position: DVec2`
- `z: f64`
- `rotation_deg: f64`

<a id="cc_engine-skp-converted"></a>

### struct `Converted`

```rust
pub struct Converted
```

What a file becomes: types, placements and the materials they use.

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `types: Vec<SkpType>`
- `placements: Vec<SkpPlacement>`
- `materials: Vec<crate::import_material::ImportedMaterial>`

  Materials the pieces use (keyed by index).

- `hidden: usize`

  Hidden instances and faces (or on hidden layers) left out.

- `faces: usize`

<a id="cc_engine-skp-skperror"></a>

### enum `SkpError`

```rust
pub enum SkpError
```

**Traits:** `Debug`

**Variants**

- `TooLarge`

  More faces than `MAX_FACES`, or more types than the import makes.


<a id="cc_engine-skp-meshbuild"></a>

### struct `MeshBuild`

```rust
pub(crate) struct MeshBuild
```

Faces of one piece, with vertices shared by position.

**Traits:** `Default`

*Some fields are private.*

**Methods**

- `pub(crate) fn add(&mut self, face: &[DVec3])`

<a id="cc_engine-skp-mesh_definition"></a>

### fn `mesh_definition`

```rust
pub(crate) fn mesh_definition(name: &str, pieces: BTreeMap<Option<usize>, MeshBuild>, materials: &[crate::import_material::ImportedMaterial]) -> Option<ComponentDef>
```

A component definition of fixed mesh pieces, one per entry of `pieces`
(keyed by the index in `materials`; `None` takes no material and is
named `name`); none when nothing was drawn.

<a id="cc_engine-skp-convert"></a>

### fn `convert`

```rust
pub fn convert(model: &SkpModel, loose_name: &str) -> Result<Converted, SkpError>
```

Every placed component and group of `model` as an element of a
component type; the root's loose faces make one more type, `loose_name`.

<a id="cc_engine-skp-convert_single"></a>

### fn `convert_single`

```rust
pub fn convert_single(model: &SkpModel, name: &str) -> Result<Converted, SkpError>
```

The whole of `model` as one component type named `name`.

{% endraw %}
