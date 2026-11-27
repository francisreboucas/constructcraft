# `cc_views` API

{% raw %}
[← API reference](index.md) · Generated from the source by `cargo run -p api_doc`; edit the doc comments, not this page.

View generation for ConstructCraft.
Each `cc_core::View` is a query over the model. `generate` produces a
resolution-independent `Drawing2D` for graphical views; `schedule`
computes tables.

## Modules

| Module | Items | Summary |
|---|---|---|
| [`cc_views`](#mod-cc_views) | 4 | View generation for ConstructCraft. |
| [`cc_views::annotate`](#mod-cc_views-annotate) | 12 | Annotations drawn over views: room, door and window tags, grids and dimensions. |
| [`cc_views::bar_tags`](#mod-cc_views-bar_tags) | 3 | Bar tags in plans, elevations and sections (see `cc_core::bar_tag`): the text `2x 25 N1 Ø12.5 c/12 C=500` derived from the tagged set, with an optional leader, and the set's bars drawn as thin lines in that view (no view draws reinforcement otherwise, so a tag always points at its bars; bars seen end-on are dots). |
| [`cc_views::bending`](#mod-cc_views-bending) | 14 | Bending schedules: a reinforcement schedule with one row per bar mark (mark, diameter, grade, a sketch of the shape with its dimensions, the length of one bar, the count, the total length and the weight; rows of bars longer than the stock length noted) and a summary per diameter and grade under it (total length and weight, with the waste added when the schedule gives one). |
| [`cc_views::cad`](#mod-cc_views-cad) | 3 | Imported CAD drawings (underlays) in the view they belong to: pictures, then thin lines in their own pen (grey, black or the layer colors, faded when the element asks), fills and texts, all below the model's linework and picked only when nothing of the model is under the pointer. |
| [`cc_views::callout`](#mod-cc_views-callout) | 3 | Crop regions and callout marks (see `cc_core::callout`). |
| [`cc_views::component_view`](#mod-cc_views-component_view) | 6 | Views of a component definition for the modeling environment: the pieces seen from above, the front, the right or in a 3D axonometric, with the reference planes seen edge-on as dash-dot lines named at one end, the reference lines of that view and the dimensions between planes (labeled ones show their parameter). |
| [`cc_views::cut`](#mod-cc_views-cut) | 5 | Fills of cut regions (plans and sections) from the material's cut pattern. |
| [`cc_views::detail`](#mod-cc_views-detail) | 3 | Detail lines in the view they belong to: each curve as short segments in its style's own pen (width, color, dash pattern; built-in or one of the project's line styles), on top of the model's linework. |
| [`cc_views::detail_component`](#mod-cc_views-detail_component) | 5 | Detail components in the view they belong to (see `cc_core::detail_component`), in id order over the model and the view's regions, under its detail lines and annotations: each one's pieces and those of the components it nests, through its placement (position, rotation, mirror, scale). |
| [`cc_views::diagram`](#mod-cc_views-diagram) | 1 | The single-line diagram of a distribution board with its load schedule: the supply and main breaker, the busbar and one branch per circuit with its breaker, conductor and load, then a table of the circuits (from `cc_geom::sizing`). |
| [`cc_views::drawing`](#mod-cc_views-drawing) | 9 |  |
| [`cc_views::drawing_index`](#mod-cc_views-drawing_index) | 1 | Lookups over a drawing's primitives: the primitives of each element and a grid of the pickable ones, so picking under the pointer does not walk every primitive of a large drawing (an imported DXF holds 100 000). |
| [`cc_views::dxf`](#mod-cc_views-dxf) | 9 | DXF export of drawings, for exchanging 2D views with other CAD programs. |
| [`cc_views::export`](#mod-cc_views-export) | 6 | Vector export of drawings in paper millimeters (composed sheets) to SVG and PDF. |
| [`cc_views::fonts`](#mod-cc_views-fonts) | 4 | Fallback fonts for the scripts Inter lacks (Chinese, Japanese, Korean, Lao), found among the fonts operating systems usually ship and read once. |
| [`cc_views::format`](#mod-cc_views-format) | 53 | Number formatting for drawings, schedules and the interface. |
| [`cc_views::graphics`](#mod-cc_views-graphics) | 5 | View graphics applied to drawings: hidden categories, filters and elements leave the drawing, as do elements the view's phase and phase filter leave out, and the resolved overrides change the pens of lines and the colors and patterns of fills (see `cc_core::graphics` and `cc_core::phase`). |
| [`cc_views::hatch`](#mod-cc_views-hatch) | 9 | Hatch patterns: the line work of a material `Pattern` clipped to a polygon. |
| [`cc_views::joins`](#mod-cc_views-joins) | 1 | Joined geometry in cut views (plans and sections): the outline of the element that is cut stops where the cutting element begins, the cutting element is painted over it, and when both are cut in the same material the line between them goes, so they read as one piece. |
| [`cc_views::legend`](#mod-cc_views-legend) | 1 |  |
| [`cc_views::level_marks`](#mod-cc_views-level_marks) | 2 | Level marks: the finished level above a shelf line and the raw level under it, beside a symbol on the point measured. |
| [`cc_views::mep`](#mod-cc_views-mep) | 5 | Building services in plans: pipes and conduits as bands in their system's color with diameter, slope and circuit tags, cable trays as outlines as wide as they are, valves on pipes, and electrical devices as the usual plan symbols (sizes in paper millimeters). |
| [`cc_views::notes`](#mod-cc_views-notes) | 11 | Text notes and their leaders. |
| [`cc_views::plan`](#mod-cc_views-plan) | 8 |  |
| [`cc_views::projected`](#mod-cc_views-projected) | 12 | Orthographic projected views: elevations, sections and axonometrics, with painter's-algorithm hidden surface removal. |
| [`cc_views::raster`](#mod-cc_views-raster) | 4 | Imported pictures: reading their size and decoding their pixels. |
| [`cc_views::region`](#mod-cc_views-region) | 5 | Filled and masking regions in the view they belong to (see `cc_core::region`), in id order, over the model's linework and under the view's annotations: a filled region as its type's fill (a solid color, or a clear background under pattern lines), a masking region opaque white; then the boundary in its line style, when it has one. |
| [`cc_views::schedule`](#mod-cc_views-schedule) | 10 | Schedules: one row per element of a category, one column per field of the schedule definition, with filtering, sorting and totals. |
| [`cc_views::shadow`](#mod-cc_views-shadow) | 17 | The shadows a view's sun casts in projected views and plans, as exact polygons (see `crate::projected::render`). |
| [`cc_views::sheet`](#mod-cc_views-sheet) | 6 | Sheet composition: the title block and the placed viewports, as one drawing in paper millimeters (origin at the bottom-left corner). |
| [`cc_views::sheet_fields`](#mod-cc_views-sheet_fields) | 5 | `{fields}` in title block texts: sheet, project and date values, and the title block's per-sheet parameters. |
| [`cc_views::sun_path`](#mod-cc_views-sun_path) | 34 | The sun path diagram of a view with a sun (`cc_core::View::sun` with `show_path`): a compass ring turned to true north on the ground, the sky dome's path of the sun over the day shown (and, over several days, over the first and last), its whole hours, sunrise, sunset and the date, and the sun where it stands. |
| [`cc_views::topography`](#mod-cc_views-topography) | 1 | Terrain in floor plans: contour lines (majors heavier) and the boundary below the cut plane; where the cut plane crosses the surface, the earth above it hatched in the type's material and outlined as cut. |

<a id="mod-cc_views"></a>

## `cc_views`

<sub>`crates/cc_views/src/lib.rs`</sub>

**Re-exports**

- `pub use mep::discipline_of`
- `pub use annotate::{DimensionLayout, HIGHLIGHT_ONLY, dimension_layout, dimension_type, draw_dimension, draw_marks}`
- `pub use component_view::{ComponentView, ComponentViewKind, ViewConnector, component_view}`
- `pub use drawing::{Anchor, Drawing2D, LineStyle, Pen, Prim, ResolvedPen, SectionHandle, TagHandle, halftone}`
- `pub use drawing_index::DrawingIndex`
- `pub use legend::legend`
- `pub use plan::{is_structural, plan, plan_of, section_pieces}`
- `pub use projected::{Frame, Look, axonometric, elevation, section, view_frame}`
- `pub use schedule::{Cell, Row, Table, field_value, schedule, schedule_where, to_csv, totaled_by_default}`
- `pub use shadow::{Catch as ShadowCatch, Indoors, Shadows, Skip as ShadowSkip, Storey}`
- `pub use sheet::{MARGIN, PlacedViewport, Sheet, SheetEnv, compose as compose_sheet, title_block_drawing}`
- `pub use sun_path::{SunDiagram, SunLabel, SunLabelKind, SunLine, SunLineKind, SunText, diagram as sun_diagram}`

**Items:** [`generate`](#cc_views-generate) · [`generate_labeled`](#cc_views-generate_labeled) · [`generate_temporary`](#cc_views-generate_temporary) · [`hosts_cad`](#cc_views-hosts_cad)

<a id="cc_views-generate"></a>

### fn `generate`

```rust
pub fn generate(project: &Project, geom: &ModelGeometry, view: &View) -> Option<Drawing2D>
```

Drawing for any graphical view (`None` for schedules and sheets), with
the headings of diagrams left as their translation keys.

<a id="cc_views-generate_labeled"></a>

### fn `generate_labeled`

```rust
pub fn generate_labeled(project: &Project, geom: &ModelGeometry, view: &View, label: &dyn Fn(&str) -> String) -> Option<Drawing2D>
```

Drawing for any graphical view, with the view's graphics applied;
`label` translates the headings drawn by the view itself (diagrams).

<a id="cc_views-generate_temporary"></a>

### fn `generate_temporary`

```rust
pub fn generate_temporary(project: &Project, geom: &ModelGeometry, view: &View, label: &dyn Fn(&str) -> String, temporary: Option<&cc_core::TemporaryVisibility>) -> Option<Drawing2D>
```

Drawing for any graphical view with its graphics and, when given, the
elements temporarily hidden or isolated in it.

<a id="cc_views-hosts_cad"></a>

### fn `hosts_cad`

```rust
pub fn hosts_cad(kind: &ViewKind) -> bool
```

Whether a view can hold imported CAD drawings: plans, ceiling plans,
elevations, sections, legends and drafting views (2D views with a drawing plane).

<a id="mod-cc_views-annotate"></a>

## `cc_views::annotate`

<sub>`crates/cc_views/src/annotate.rs`</sub>

Annotations drawn over views: room, door and window tags, grids and
dimensions. Sizes are given in paper millimeters and converted with the
view scale, so annotations read the same at any scale.

**Items:** [`HIGHLIGHT_ONLY`](#cc_views-annotate-highlight_only) · [`TagOffsets`](#cc_views-annotate-tagoffsets) · [`plan_annotations`](#cc_views-annotate-plan_annotations) · [`section_label`](#cc_views-annotate-section_label) · [`structure_tags`](#cc_views-annotate-structure_tags) · [`projected_grids`](#cc_views-annotate-projected_grids) · [`dimension_type`](#cc_views-annotate-dimension_type) · [`dimensions`](#cc_views-annotate-dimensions) · [`draw_dimension`](#cc_views-annotate-draw_dimension) · [`draw_marks`](#cc_views-annotate-draw_marks) · [`DimensionLayout`](#cc_views-annotate-dimensionlayout) · [`dimension_layout`](#cc_views-annotate-dimension_layout)

<a id="cc_views-annotate-highlight_only"></a>

### const `HIGHLIGHT_ONLY`

```rust
pub const HIGHLIGHT_ONLY: [u8; 4]
```

Fill color that is invisible but highlights when the element is selected.

<a id="cc_views-annotate-tagoffsets"></a>

### type alias `TagOffsets`

```rust
pub type TagOffsets = std::collections::BTreeMap<cc_core::ElementId, DVec2>
```

Tags a view moved by hand: offsets in plan by the element they tag.

<a id="cc_views-annotate-plan_annotations"></a>

### fn `plan_annotations`

```rust
pub fn plan_annotations(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, level: LevelId, ceiling: bool, structural: bool, view: &cc_core::View)
```

Tags of rooms and openings, and grids, for a floor or ceiling plan of `level`.

<a id="cc_views-annotate-section_label"></a>

### fn `section_label`

```rust
pub fn section_label(project: &Project, data: &TypeData) -> Option<String>
```

A steel or cold-formed profile's name (`IPE 200`), else the section's size in
centimeters (inches in imperial units): `15×40`, `Ø30`.

<a id="cc_views-annotate-structure_tags"></a>

### fn `structure_tags`

```rust
pub fn structure_tags(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, level: LevelId, scale: u32, offsets: &TagOffsets)
```

Tags of the beams, columns, pad footings and pile caps of `level`: their mark and
section (`V1 15×40`). Beam tags run along the beam, beside it; column
and footing tags sit above and to the right.

<a id="cc_views-annotate-projected_grids"></a>

### fn `projected_grids`

```rust
pub fn projected_grids(d: &mut Drawing2D, project: &Project, frame: &crate::projected::Frame, scale: u32)
```

Grids in an elevation or a section looking through `frame` (issue #332):
each grid running square to the view's plane, as a vertical line where it
stands with its bubble on top, as tall as what the view draws. A section
shows the grids that cross what it sees: between the ends of its line,
and between its cut and its depth. They belong to their grid, so the
view's graphics hide them with the category or one by one.

<a id="cc_views-annotate-dimension_type"></a>

### fn `dimension_type`

```rust
pub fn dimension_type(project: &Project, type_id: cc_core::ElementTypeId) -> (f64, DimensionStyle)
```

The text size and style of a dimension type (the defaults for a missing one).

<a id="cc_views-annotate-dimensions"></a>

### fn `dimensions`

```rust
pub fn dimensions(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32)
```

Dimensions placed in `view`, in its drawing coordinates.

<a id="cc_views-annotate-draw_dimension"></a>

### fn `draw_dimension`

```rust
pub fn draw_dimension(d: &mut Drawing2D, layout: &DimensionLayout, style: &DimensionStyle, text_size: f64, scale: u32, id: Option<cc_core::ElementId>)
```

Draw a laid-out dimension: lines, marks, the box behind an opaque
text, and the text.

<a id="cc_views-annotate-draw_marks"></a>

### fn `draw_marks`

```rust
pub fn draw_marks(d: &mut Drawing2D, layout: &DimensionLayout, style: &DimensionStyle, scale: u32, id: Option<cc_core::ElementId>)
```

The marks at the ends of a laid-out dimension line.

<a id="cc_views-annotate-dimensionlayout"></a>

### struct `DimensionLayout`

```rust
pub struct DimensionLayout
```

Linework and text of a dimension, in its view's drawing coordinates.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `lines: Vec<(DVec2, DVec2, LineStyle)>`
- `marks: Vec<(DVec2, DVec2)>`

  Where the dimension line ends at a mark, and the direction (a unit
  vector) it arrives there in.

- `text: String`

  The value with the style's prefix and suffix.

- `text_pos: DVec2`

  Bottom center of the text, which is rotated by `text_angle` (radians).

- `text_angle: f64`

<a id="cc_views-annotate-dimension_layout"></a>

### fn `dimension_layout`

```rust
pub fn dimension_layout(dim: &Dimension, scale: u32, style: &DimensionStyle) -> Option<DimensionLayout>
```

Lay out a dimension at a view `scale` as `style` says; `None` when it measures nothing.

<a id="mod-cc_views-bar_tags"></a>

## `cc_views::bar_tags`

<sub>`crates/cc_views/src/bar_tags.rs`</sub>

Bar tags in plans, elevations and sections (see `cc_core::bar_tag`):
the text `2x 25 N1 Ø12.5 c/12 C=500` derived from the tagged set, with an
optional leader, and the set's bars drawn as thin lines in that view (no
view draws reinforcement otherwise, so a tag always points at its bars;
bars seen end-on are dots). Sizes are paper millimeters at the view scale.

**Items:** [`text`](#cc_views-bar_tags-text) · [`takes_tags`](#cc_views-bar_tags-takes_tags) · [`bar_tags`](#cc_views-bar_tags-bar_tags)

<a id="cc_views-bar_tags-text"></a>

### fn `text`

```rust
pub fn text(project: &Project, geom: &ModelGeometry, marks: &BarMarks, tag: &BarTag) -> Option<String>
```

The text of `tag`: its parts, from the tagged set's bars (in `geom`)
and their `marks`; `None` when the set has no bars there.

<a id="cc_views-bar_tags-takes_tags"></a>

### fn `takes_tags`

```rust
pub fn takes_tags(view: &View) -> bool
```

Whether `view` takes bar tags: plans, elevations and sections.

<a id="cc_views-bar_tags-bar_tags"></a>

### fn `bar_tags`

```rust
pub fn bar_tags(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, view: &View)
```

The bar tags of `view`, with the bars of the sets they tag.

<a id="mod-cc_views-bending"></a>

## `cc_views::bending`

<sub>`crates/cc_views/src/bending.rs`</sub>

Bending schedules: a reinforcement schedule with one row per bar mark
(mark, diameter, grade, a sketch of the shape with its dimensions, the
length of one bar, the count, the total length and the weight; rows of
bars longer than the stock length noted) and a summary per diameter and
grade under it (total length and weight, with the waste added when the
schedule gives one).
Every total is the exact sum of the bars it counts: a row adds its bars'
lengths and weights, the totals and the summary add the rows, so the
summary per diameter equals the sum of the individual bars.
The shape of a mark is written as a code (CSV, the app's table):
`straight 500`, `L 30-470`, `U 30-470-30`, `Z …`, `bent …`,
`stirrup 14x44`, `ring R=50 L=314`, `closed …`, `arc R=220 L=345`, legs
of arcs as `(R=… L=…)`; lengths as `format::bar_length` writes them.
Sheets draw a sketch instead (`BarSketch`): the bar's legs laid flat in
order, to proportion (very short straight legs drawn longer), normalized
into the cell, each leg labelled with its length.

**Items:** [`SKETCH_ROW`](#cc_views-bending-sketch_row) · [`SKETCH_WIDTH`](#cc_views-bending-sketch_width) · [`SKETCH_TEXT`](#cc_views-bending-sketch_text) · [`BarSketch`](#cc_views-bending-barsketch) · [`SketchLabel`](#cc_views-bending-sketchlabel) · [`shape_code`](#cc_views-bending-shape_code) · [`sketch_of`](#cc_views-bending-sketch_of) · [`draw_sketch`](#cc_views-bending-draw_sketch) · [`diameter_text`](#cc_views-bending-diameter_text) · [`BendingRow`](#cc_views-bending-bendingrow) · [`DiameterTotal`](#cc_views-bending-diametertotal) · [`Bending`](#cc_views-bending-bending) · [`bending`](#cc_views-bending-bending) · [`table`](#cc_views-bending-table)

<a id="cc_views-bending-sketch_row"></a>

### const `SKETCH_ROW`

```rust
pub const SKETCH_ROW: f64
```

Height of a row with a sketch, paper millimeters.

<a id="cc_views-bending-sketch_width"></a>

### const `SKETCH_WIDTH`

```rust
pub const SKETCH_WIDTH: f64
```

Narrowest column of sketches, paper millimeters.

<a id="cc_views-bending-sketch_text"></a>

### const `SKETCH_TEXT`

```rust
pub const SKETCH_TEXT: f64
```

Text height of the sketches' dimensions, paper millimeters.

<a id="cc_views-bending-barsketch"></a>

### struct `BarSketch`

```rust
pub struct BarSketch
```

A bar's shape drawn for a schedule: one polyline per leg in the
sketch's own frame (meters, schematic) and the dimensions.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Default`

**Fields**

- `legs: Vec<Vec<DVec2>>`
- `labels: Vec<SketchLabel>`

<a id="cc_views-bending-sketchlabel"></a>

### struct `SketchLabel`

```rust
pub struct SketchLabel
```

A dimension of a sketch: its text, the point of the leg it stands by and
the unit direction it stands off to.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `text: String`
- `at: DVec2`
- `away: DVec2`

<a id="cc_views-bending-shape_code"></a>

### fn `shape_code`

```rust
pub fn shape_code(shape: &ShapeSummary) -> String
```

The code of a shape (see the module's documentation).

<a id="cc_views-bending-sketch_of"></a>

### fn `sketch_of`

```rust
pub fn sketch_of(shape: &ShapeSummary) -> BarSketch
```

The sketch of a shape (see `BarSketch`): its legs laid flat one after
another, each bend turning the way the bar turns, the longest straight
leg along x and the rest above it, short straight legs of open bars
drawn longer, each leg labelled with its length (arcs also with their
radius).

<a id="cc_views-bending-draw_sketch"></a>

### fn `draw_sketch`

```rust
pub fn draw_sketch(d: &mut Drawing2D, sketch: &BarSketch, (lo, hi): (DVec2, DVec2), k: f64)
```

Draw `sketch` into the cell from `lo` to `hi` (paper millimeters, y up)
of a drawing at `k` meters per paper millimeter.

<a id="cc_views-bending-diameter_text"></a>

### fn `diameter_text`

```rust
pub fn diameter_text(diameter: f64) -> String
```

A rebar diameter as schedules write it (`Ø 12,5 mm`, `Ø 1/2"`).

<a id="cc_views-bending-bendingrow"></a>

### struct `BendingRow`

```rust
pub struct BendingRow
```

One row of a bending schedule: the bars of one mark (in one partition
when grouped), with exact sums.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `partition: String`

  The partition heading the row when grouped (else empty).

- `mark: usize`

  Index of the mark in the resolved `bar_marks::BarMarks`.

- `count: usize`
- `length: f64`

  The bars' lengths added up, meters.

- `weight: f64`

  Their weight, kilograms.

- `longest: f64`

  The longest bar, meters.

- `first: ElementId`

  The first set with bars of the row.


<a id="cc_views-bending-diametertotal"></a>

### struct `DiameterTotal`

```rust
pub struct DiameterTotal
```

The total of one diameter and grade: the rows of it added up.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `diameter: f64`
- `grade: String`
- `length: f64`
- `weight: f64`

<a id="cc_views-bending-bending"></a>

### struct `Bending`

```rust
pub struct Bending
```

A bending schedule's numbers: the marks, the rows in order and the
totals per diameter and grade (diameters ascending).

**Traits:** `Debug`, `Clone`, `PartialEq`, `Default`

**Fields**

- `marks: bar_marks::BarMarks`
- `rows: Vec<BendingRow>`
- `totals: Vec<DiameterTotal>`

<a id="cc_views-bending-bending"></a>

### fn `bending`

```rust
pub fn bending(project: &Project, geom: &ModelGeometry, options: &BendingOptions, keep: &mut dyn FnMut(&Element) -> bool) -> Bending
```

The rows and totals of a bending schedule of the reinforcement sets
`keep` accepts (those a view's phase and options show) and `options`
lets through. Marks are resolved over the whole model, so they read the
same in every schedule. Each row adds its bars in set and bar order; the
totals add the rows in order.

<a id="cc_views-bending-table"></a>

### fn `table`

```rust
pub fn table(project: &Project, geom: &ModelGeometry, def: &ScheduleDef, options: &BendingOptions, keep: &mut dyn FnMut(&Element) -> bool) -> Table
```

The bending schedule as a table (see `bending`): a row per mark (under
a heading per partition when grouped), the totals, and the summary per
diameter under it.

<a id="mod-cc_views-cad"></a>

## `cc_views::cad`

<sub>`crates/cc_views/src/cad.rs`</sub>

Imported CAD drawings (underlays) in the view they belong to: pictures,
then thin lines in their own pen (grey, black or the layer colors, faded
when the element asks), fills and texts, all below the model's linework
and picked only when nothing of the model is under the pointer.

**Items:** [`GREY`](#cc_views-cad-grey) · [`underlays`](#cc_views-cad-underlays) · [`draw`](#cc_views-cad-draw)

<a id="cc_views-cad-grey"></a>

### const `GREY`

```rust
pub const GREY: [u8; 3]
```

The grey of `CadColorMode::Grey`.

<a id="cc_views-cad-underlays"></a>

### fn `underlays`

```rust
pub fn underlays(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32)
```

Draw the imported drawings of `view` under what `d` holds.

<a id="cc_views-cad-draw"></a>

### fn `draw`

```rust
pub fn draw(d: &mut Drawing2D, c: &CadImport, id: ElementId, scale: u32)
```

One imported drawing, in view coordinates.

<a id="mod-cc_views-callout"></a>

## `cc_views::callout`

<sub>`crates/cc_views/src/callout.rs`</sub>

Crop regions and callout marks (see `cc_core::callout`).

**Items:** [`CalloutHandle`](#cc_views-callout-callouthandle) · [`marks`](#cc_views-callout-marks) · [`crop`](#cc_views-callout-crop)

<a id="cc_views-callout-callouthandle"></a>

### struct `CalloutHandle`

```rust
pub struct CalloutHandle
```

A callout's mark in its parent, as drawn (meters on the parent's plane).

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `view: ViewId`

  The detail (or drafting) view the callout stands for.

- `rect: CropBox`
- `tag: DVec2`

  Center of the tag's circle.

- `radius: f64`

**Methods**

- `pub fn hits(&self, p: DVec2, tol: f64) -> bool`

  Whether `p` lies on the mark: within `tol` of the rectangle's sides or in the tag.


<a id="cc_views-callout-marks"></a>

### fn `marks`

```rust
pub(crate) fn marks(d: &mut Drawing2D, project: &Project, view: &View)
```

The marks of the callouts drawn in `view`, each listed in `d.callouts`.

<a id="cc_views-callout-crop"></a>

### fn `crop`

```rust
pub(crate) fn crop(d: &mut Drawing2D, project: &Project, view: ViewId, rect: CropBox)
```

Crop the drawing of `view` to `rect`: lines and fills of the model (and
of anything not owned by the view) are clipped to it, texts and pictures
outside it dropped; what the view owns (detail lines, regions, notes,
dimensions…) is kept whole.

<a id="mod-cc_views-component_view"></a>

## `cc_views::component_view`

<sub>`crates/cc_views/src/component_view.rs`</sub>

Views of a component definition for the modeling environment: the
pieces seen from above, the front, the right or in a 3D axonometric,
with the reference planes seen edge-on as dash-dot lines named at one
end, the reference lines of that view and the dimensions between planes
(labeled ones show their parameter).
Coordinates are the view's own (u, v) in meters, as `SketchPlane`
defines them: plan x, y; front x, z; side y, z. Lines carry no element:
the environment picks planes and dimensions from the lists returned.

**Items:** [`ComponentViewKind`](#cc_views-component_view-componentviewkind) · [`ViewPlane`](#cc_views-component_view-viewplane) · [`ViewDimension`](#cc_views-component_view-viewdimension) · [`ViewConnector`](#cc_views-component_view-viewconnector) · [`ComponentView`](#cc_views-component_view-componentview) · [`component_view`](#cc_views-component_view-component_view)

<a id="cc_views-component_view-componentviewkind"></a>

### enum `ComponentViewKind`

```rust
pub enum ComponentViewKind
```

The views of the environment.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default`

**Variants**

- `Plan`
- `Front`
- `Side`
- `ThreeD`

  From the front, right and above.


**Methods**

- `pub const ALL: [ComponentViewKind; 4]`
- `pub fn plane(self) -> Option<SketchPlane>`

  The sketch plane drawn in this view (none in 3D).

- `pub fn key(self) -> &'static str`
- `pub fn project(self, p: DVec3) -> DVec3`

  A model point in this view: (u, v) and the depth toward the viewer.


<a id="cc_views-component_view-viewplane"></a>

### struct `ViewPlane`

```rust
pub struct ViewPlane
```

A reference plane seen edge-on: its name and the ends of its line.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `name: String`
- `a: DVec2`
- `b: DVec2`
- `origin: bool`

  One of the origin planes (cannot move).


<a id="cc_views-component_view-viewdimension"></a>

### struct `ViewDimension`

```rust
pub struct ViewDimension
```

A dimension between planes as drawn: where its text sits.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `index: usize`
- `text_pos: DVec2`
- `text_angle: f64`
- `value: f64`
- `label: Option<String>`

<a id="cc_views-component_view-viewconnector"></a>

### struct `ViewConnector`

```rust
pub struct ViewConnector
```

A connector as seen in a view: where it is and which way runs leave it.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `index: usize`

  Its index in the definition.

- `point: DVec2`
- `dir: DVec2`

  The direction in the view (zero length when it points at or away from the viewer).

- `facing: i8`

  Toward the viewer (+1), away (-1) or across the view (0).

- `system: cc_core::SystemKind`
- `radius: Option<f64>`

  Radius of the run it takes, when it has a diameter.


<a id="cc_views-component_view-componentview"></a>

### struct `ComponentView`

```rust
pub struct ComponentView
```

A view of a component: its drawing and what can be picked in it.

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `drawing: Drawing2D`
- `planes: Vec<ViewPlane>`
- `dimensions: Vec<ViewDimension>`
- `equals: Vec<ViewDimension>`

  Equalities (EQ) drawn in this view, by index: where each "EQ" text sits.

- `connectors: Vec<ViewConnector>`

  Connectors, drawn by the environment in their system's color.

- `pieces: Vec<(usize, Vec<(DVec2, DVec2)>)>`

  Each piece's edges in this view (by index in the definition), for picking.

- `bounds: Option<(DVec2, DVec2)>`

  Box of the pieces and planes, for zoom to fit.

- `errors: Vec<String>`

  Pieces that do not resolve, with why.


<a id="cc_views-component_view-component_view"></a>

### fn `component_view`

```rust
pub fn component_view(def: &ComponentDef, overrides: &BTreeMap<String, f64>, kind: ComponentViewKind, scale: u32) -> ComponentView
```

The component `def` (with instance `overrides`) in `kind`, drawn at a
view `scale` (for text and dimension sizes).

<a id="mod-cc_views-cut"></a>

## `cc_views::cut`

<sub>`crates/cc_views/src/cut.rs`</sub>

Fills of cut regions (plans and sections) from the material's cut pattern.

**Items:** [`POCHE`](#cc_views-cut-poche) · [`PATTERN_MM`](#cc_views-cut-pattern_mm) · [`spacing`](#cc_views-cut-spacing) · [`cut_fill`](#cc_views-cut-cut_fill) · [`cut_fill_with_holes`](#cc_views-cut-cut_fill_with_holes)

<a id="cc_views-cut-poche"></a>

### const `POCHE`

```rust
pub(crate) const POCHE: [u8; 4]
```

Neutral fill of cut regions whose material has no cut pattern.

<a id="cc_views-cut-pattern_mm"></a>

### const `PATTERN_MM`

```rust
pub(crate) const PATTERN_MM: f64
```

Base spacing of hatch patterns on paper, millimeters.

<a id="cc_views-cut-spacing"></a>

### fn `spacing`

```rust
pub(crate) fn spacing(scale: u32) -> f64
```

Hatch spacing in drawing units for a view at `1:scale`.

<a id="cc_views-cut-cut_fill"></a>

### fn `cut_fill`

```rust
pub(crate) fn cut_fill(d: &mut Drawing2D, project: &Project, material: &str, poly: Vec<DVec2>, spacing: f64, id: Option<ElementId>)
```

Fill `poly` with the cut pattern of `material` (or the neutral poche).

<a id="cc_views-cut-cut_fill_with_holes"></a>

### fn `cut_fill_with_holes`

```rust
pub(crate) fn cut_fill_with_holes(d: &mut Drawing2D, project: &Project, material: &str, outer: &[DVec2], holes: &[Vec<DVec2>], spacing: f64, id: Option<ElementId>)
```

`cut_fill` of a region with `holes`: one keyhole ring (triangles when
no ring can be made), the pattern lines clipped out of the holes.

<a id="mod-cc_views-detail"></a>

## `cc_views::detail`

<sub>`crates/cc_views/src/detail.rs`</sub>

Detail lines in the view they belong to: each curve as short segments
in its style's own pen (width, color, dash pattern; built-in or one of
the project's line styles), on top of the model's linework. View
graphics override them like any line.

**Items:** [`pen`](#cc_views-detail-pen) · [`draw`](#cc_views-detail-draw) · [`lines`](#cc_views-detail-lines)

<a id="cc_views-detail-pen"></a>

### fn `pen`

```rust
pub fn pen(style: LineStyleRef, styles: &LineStyles) -> Pen
```

The pen of a line style.

<a id="cc_views-detail-draw"></a>

### fn `draw`

```rust
pub fn draw(d: &mut Drawing2D, line: &DetailLine, styles: &LineStyles, id: Option<ElementId>)
```

Draw one detail line.

<a id="cc_views-detail-lines"></a>

### fn `lines`

```rust
pub fn lines(d: &mut Drawing2D, project: &Project, view: ViewId)
```

The detail lines of `view`.

<a id="mod-cc_views-detail_component"></a>

## `cc_views::detail_component`

<sub>`crates/cc_views/src/detail_component.rs`</sub>

Detail components in the view they belong to (see
`cc_core::detail_component`), in id order over the model and the
view's regions, under its detail lines and annotations: each one's
pieces and those of the components it nests, through its placement
(position, rotation, mirror, scale). Pictures first, then strokes (a
closed one's solid or pattern fill under its lines), then texts, as
title blocks draw theirs; lengths are meters, pen weights paper
millimeters, text heights meters turned into paper millimeters by the
view's scale.

**Items:** [`def_of`](#cc_views-detail_component-def_of) · [`components`](#cc_views-detail_component-components) · [`draw`](#cc_views-detail_component-draw) · [`draw_pieces`](#cc_views-detail_component-draw_pieces) · [`dxf_blocks`](#cc_views-detail_component-dxf_blocks)

<a id="cc_views-detail_component-def_of"></a>

### fn `def_of`

```rust
pub fn def_of<'a>(project: &'a Project, e: &cc_core::Element) -> Option<(&'a str, &'a ComponentDef)>
```

The definition and component name of an element's type, if it is a
detail component.

<a id="cc_views-detail_component-components"></a>

### fn `components`

```rust
pub fn components(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32)
```

The detail components of `view`, at `1:scale`.

<a id="cc_views-detail_component-draw"></a>

### fn `draw`

```rust
pub fn draw(d: &mut Drawing2D, project: &Project, name: &str, def: &ComponentDef, overrides: &std::collections::BTreeMap<String, f64>, place: Place, scale: u32, id: Option<ElementId>) -> bool
```

Draw the component `name` (`def`, with `overrides`) and what it nests
through `place`; returns whether nothing was left out.

<a id="cc_views-detail_component-draw_pieces"></a>

### fn `draw_pieces`

```rust
pub fn draw_pieces(d: &mut Drawing2D, patterns: &cc_core::fill_pattern::FillPatterns, pieces: &SheetPieces, place: &Place, scale: u32, id: Option<ElementId>)
```

Draw one resolution's own pieces (not what it nests) through `place`.

<a id="cc_views-detail_component-dxf_blocks"></a>

### fn `dxf_blocks`

```rust
pub fn dxf_blocks(project: &Project, view: ViewId, drawing: &Drawing2D, scale: u32, layer: &dyn Fn(&cc_core::Element) -> String) -> crate::dxf::DxfBlocks
```

The detail components of `view` that `drawing` shows, as DXF blocks:
one block per component (an element with its own parameter values gets
its own), drawn at `1:scale` with what it nests as inserts of further
blocks (a nested piece that would close a cycle is left out), and an
insert per element on the layer `layer` names.

<a id="mod-cc_views-diagram"></a>

## `cc_views::diagram`

<sub>`crates/cc_views/src/diagram.rs`</sub>

The single-line diagram of a distribution board with its load schedule:
the supply and main breaker, the busbar and one branch per circuit with
its breaker, conductor and load, then a table of the circuits (from
`cc_geom::sizing`). Sizes in paper millimeters; `label` translates the
headings (keys `diagram-*`).

**Items:** [`panel_diagram`](#cc_views-diagram-panel_diagram)

<a id="cc_views-diagram-panel_diagram"></a>

### fn `panel_diagram`

```rust
pub fn panel_diagram(project: &Project, geom: &ModelGeometry, panel: ElementId, scale: u32, label: &dyn Fn(&str) -> String) -> Drawing2D
```

<a id="mod-cc_views-drawing"></a>

## `cc_views::drawing`

<sub>`crates/cc_views/src/drawing.rs`</sub>

**Items:** [`LineStyle`](#cc_views-drawing-linestyle) · [`Pen`](#cc_views-drawing-pen) · [`ResolvedPen`](#cc_views-drawing-resolvedpen) · [`halftone`](#cc_views-drawing-halftone) · [`Anchor`](#cc_views-drawing-anchor) · [`Prim`](#cc_views-drawing-prim) · [`Drawing2D`](#cc_views-drawing-drawing2d) · [`SectionHandle`](#cc_views-drawing-sectionhandle) · [`TagHandle`](#cc_views-drawing-taghandle)

<a id="cc_views-drawing-linestyle"></a>

### enum `LineStyle`

```rust
pub enum LineStyle
```

Graphic weight of a line, resolved to pen widths by the renderer.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`

**Variants**

- `Cut`

  Element cut by the view plane.

- `Projection`

  Visible element beyond the cut plane.

- `Thin`

  Fine detail (layers, glass, door swings).

- `Hidden`

  Dashed: element above the cut plane or hidden.

- `Annotation`

  Levels, section marks, text leaders.

- `Center`

  Dash-dot: grid lines and center lines.

- `Faint`

  Background linework (the architecture under building services).

- `Hatch`

  Lines of a cut pattern (drawn like `Thin`; view graphics replace them
  when they override the cut pattern).


**Methods**

- `pub fn pen(self) -> (f64, [u8; 3])`

  The default pen of the style: width (paper mm) and color.

- `pub fn dashes(self) -> &'static [f64]`

  Dash pattern of the style, paper millimeters.

- `pub fn resolve(self, pen: Option<&Pen>) -> ResolvedPen`

  The pen of a line of this style with its overrides.


<a id="cc_views-drawing-pen"></a>

### struct `Pen`

```rust
pub struct Pen
```

Graphic overrides of one line (view graphics); `None` keeps the style's pen.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Default`

**Fields**

- `color: Option<[u8; 3]>`
- `weight: Option<f64>`

  Width on paper, millimeters.

- `pattern: Option<cc_core::LinePattern>`
- `halftone: bool`

  Mixed with white.


<a id="cc_views-drawing-resolvedpen"></a>

### struct `ResolvedPen`

```rust
pub struct ResolvedPen
```

A line's pen resolved: width in paper millimeters, color and dash lengths
in paper millimeters (empty: continuous).

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `width: f64`
- `color: [u8; 3]`
- `dashes: &'static [f64]`

<a id="cc_views-drawing-halftone"></a>

### fn `halftone`

```rust
pub fn halftone(c: [u8; 3]) -> [u8; 3]
```

A color mixed two thirds with white (faint drawing).

<a id="cc_views-drawing-anchor"></a>

### enum `Anchor`

```rust
pub enum Anchor
```

Which point of a text box sits at its position.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Default`

**Variants**

- `LeftBottom`
- `CenterBottom`
- `RightBottom`

  The end of the baseline (right-aligned).

- `Center`

<a id="cc_views-drawing-prim"></a>

### enum `Prim`

```rust
pub enum Prim
```

Drawing primitive, in view coordinates (meters on the view plane).

**Traits:** `Debug`, `Clone`, `PartialEq`

**Variants**

- `Line { a: DVec2, b: DVec2, style: LineStyle, element: Option<ElementId>, pen: Option<Pen>, }`
- `Fill { pts: Vec<DVec2>, color: [u8; 4], element: Option<ElementId>, cut: bool, }`

  Simple polygon fill (any winding, may be concave).

- `Text { pos: DVec2, text: String, size: f64, anchor: Anchor, angle: f64, element: Option<ElementId>, }`

  Text placed by `anchor` at `pos`, rotated by `angle` radians
  (counter-clockwise); `size` is paper height in millimeters.

- `Image { origin: DVec2, u: DVec2, v: DVec2, image: std::sync::Arc<cc_core::raster::RasterImage>, element: Option<ElementId>, faded: bool, }`

  A picture filling the parallelogram from `origin` (its bottom-left
  corner) along `u` (its width) and `v` (its height).


**Methods**

- `pub fn mapped(&self, f: impl Fn(DVec2) -> DVec2) -> Prim`

  The primitive with its points moved by the affine map `f` (an image's
  sides follow from where its corners land).

- `pub fn image_corners(origin: DVec2, u: DVec2, v: DVec2) -> [DVec2; 4]`

  The four corners of an image (bottom-left first, counter-clockwise when not mirrored).

- `pub fn element(&self) -> Option<ElementId>`

  The element a primitive belongs to, if any.


<a id="cc_views-drawing-drawing2d"></a>

### struct `Drawing2D`

```rust
pub struct Drawing2D
```

Ordered list of primitives, painted first to last.

**Traits:** `Debug`, `Clone`, `Default`, `PartialEq`

**Fields**

- `prims: Vec<Prim>`
- `fit_elements: bool`

  Fit the view to the geometry of elements only (plans: the markers of
  elevations and sections stand outside the model).

- `background: std::collections::BTreeSet<ElementId>`

  Elements drawn as background (imported CAD drawings): picked only
  where nothing else is.

- `tags: Vec<TagHandle>`

  Tags that can be dragged: where each is and how far it was moved.

- `underlay: usize`

  How many of the first `prims` are the plans of other levels drawn
  under a plan (`cc_core::Underlay`): halftone, with no element, not
  picked or snapped to.

- `overlay: usize`

  How many of the last `prims` are an overlay drawn over the view (the
  sun path diagram, `crate::sun_path`): with no element, not picked
  or snapped to, but part of the extents to fit the view to.

- `sections: Vec<SectionHandle>`

  The marks of sections in a plan: picked, moved and flipped there.

- `callouts: Vec<crate::callout::CalloutHandle>`

  The marks of callouts drawn in the view: picked, moved and resized there.

- `index: DrawingIndex`

  Lookups over `prims`, built the first time they are needed (see
  `DrawingIndex`); a copy of the drawing starts without them.


**Methods**

- `pub fn own_prims(&self) -> &[Prim]`

  The primitives of the view itself, without its underlay and its overlay.

- `pub fn snap_lines(&self) -> Vec<(DVec2, DVec2)>`

  The lines of the view itself that tools snap to by their ends: its
  linework without the pattern lines of hatches, which are not geometry.

- `pub fn overlay_prims(&self) -> &[Prim]`

  The overlay drawn over the view (see `Self::overlay`).

- `pub fn without_underlay(&self) -> Drawing2D`

  The drawing without its underlay (printed when the view says so).

- `pub fn section_at(&self, p: DVec2, tol: f64) -> Option<&SectionHandle>`

  The section mark at `p` (the last drawn on top), if any.

- `pub fn callout_at(&self, p: DVec2, tol: f64) -> Option<&crate::callout::CalloutHandle>`

  The callout mark at `p` (the last drawn on top), if any.

- `pub fn tag_at(&self, p: DVec2) -> Option<TagHandle>`

  The tag at `p` (the last drawn on top), if any.

- `pub fn line(&mut self, a: DVec2, b: DVec2, style: LineStyle, element: Option<ElementId>)`
- `pub fn line_pen(&mut self, a: DVec2, b: DVec2, style: LineStyle, pen: Option<Pen>, element: Option<ElementId>)`

  A line with its own pen (view graphics still override what they set).

- `pub fn polyline(&mut self, pts: &[DVec2], closed: bool, style: LineStyle, element: Option<ElementId>)`
- `pub fn fill(&mut self, pts: Vec<DVec2>, color: [u8; 4], element: Option<ElementId>)`
- `pub fn cut_fill(&mut self, pts: Vec<DVec2>, color: [u8; 4], element: Option<ElementId>)`

  A fill of a cut region.

- `pub fn text(&mut self, pos: DVec2, text: impl Into<String>, size: f64)`
- `pub fn text_at(&mut self, pos: DVec2, text: impl Into<String>, size: f64, anchor: Anchor, angle: f64)`
- `pub fn tag_text(&mut self, pos: DVec2, text: impl Into<String>, size: f64, anchor: Anchor, angle: f64, element: Option<ElementId>)`

  Text that annotates `element` (a tag, a dimension value…).

- `pub fn arc(&mut self, center: DVec2, radius: f64, a0: f64, a1: f64, style: LineStyle, element: Option<ElementId>)`

  Circular arc approximated with line segments.

- `pub fn dashed_arc(&mut self, center: DVec2, radius: f64, (a0, a1): (f64, f64), style: LineStyle, element: Option<ElementId>, scale: u32)`

  A circular arc in a dashed `style` at `1:scale`, its dashes laid
  along the whole arc (each a few chords, the gaps left empty), so the
  pattern shows however short the chords of `Self::arc` would be.

- `pub fn bounds(&self) -> Option<(DVec2, DVec2)>`

  Bounding box of geometry (text anchors included).

- `pub fn fit_bounds(&self) -> Option<(DVec2, DVec2)>`

  The extents to fit the view to: [`bounds`](Self::bounds), or only the
  elements' geometry and the overlay with `fit_elements` (all of it
  when there is none).

- `pub fn hit_test(&self, p: DVec2, tol: f64) -> Option<ElementId>`

  Topmost element under `p` within `tol` (view units). Background
  elements (imported drawings) are picked only where nothing else is.

- `pub fn prims_of(&self, ids: &std::collections::BTreeSet<ElementId>) -> Vec<usize>`

  The indices of the primitives of `ids`, in drawing order.

- `pub fn hit_test_line(&self, p: DVec2, tol: f64) -> Option<ElementId>`

  The element whose line passes within `tol` of `p` (the nearest),
  leaving out fills and background elements: what a drag starting at
  `p` grabs.

- `pub fn lines_near(&self, p: DVec2, tol: f64) -> Vec<(DVec2, DVec2, ElementId)>`

  The lines of elements passing within `tol` of `p`, with their
  element: the model as the view draws it, for snapping. The underlay,
  background elements (imported drawings), pattern lines, faded
  linework and annotations are left out.

- `pub(crate) fn hit_test_all(&self, p: DVec2, tol: f64) -> Option<ElementId>`

  `Self::hit_test` searching every primitive (tests of the index).

- `pub(crate) fn nearest_line_all(&self, p: DVec2, tol: f64) -> Option<ElementId>`

  `Self::hit_test_line` searching every primitive (tests of the index).

- `pub fn elements_in(&self, lo: DVec2, hi: DVec2, fully: bool) -> Vec<ElementId>`

  Elements taken by the rectangle `lo`..`hi`: with `fully` (a window)
  those whose lines, fills and texts all lie inside it, otherwise (a
  crossing) those that any of them reaches into: a line crossing it, a
  fill overlapping it, a text anchored in it. A crossing leaves out
  background elements (imported drawings), which lie under everything.


<a id="cc_views-drawing-sectionhandle"></a>

### struct `SectionHandle`

```rust
pub struct SectionHandle
```

A section's mark in a plan, as drawn (meters on the plan).

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `view: cc_core::ViewId`
- `start: DVec2`
- `end: DVec2`
- `segments: Option<[f64; 2]>`

  The view's split line (see `cc_core::ViewKind::Section`).

- `pieces: Vec<(DVec2, DVec2)>`

  The pieces of line drawn between the heads.

- `radius: f64`

  Radius of a head's circle.

- `head: f64`

  How far from each end the line starts (half the triangle's base).


**Methods**

- `pub fn hits(&self, p: DVec2, tol: f64) -> bool`

  Whether `p` lies on the mark: within `tol` of its line or in a head.

- `pub fn segment_ends(&self) -> Option<[DVec2; 2]>`

  The inner ends of a split line's pieces, next to the start and to
  the end: dragged closer to or away from their head.


<a id="cc_views-drawing-taghandle"></a>

### struct `TagHandle`

```rust
pub struct TagHandle
```

A tag of an element in a view, as drawn: picked within `half` of
`center`; `offset` is how far the view moved it from its default spot.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `element: ElementId`
- `center: DVec2`
- `half: DVec2`
- `offset: DVec2`

<a id="mod-cc_views-drawing_index"></a>

## `cc_views::drawing_index`

<sub>`crates/cc_views/src/drawing_index.rs`</sub>

Lookups over a drawing's primitives: the primitives of each element and
a grid of the pickable ones, so picking under the pointer does not walk
every primitive of a large drawing (an imported DXF holds 100 000).

**Items:** [`DrawingIndex`](#cc_views-drawing_index-drawingindex)

<a id="cc_views-drawing_index-drawingindex"></a>

### struct `DrawingIndex`

```rust
pub struct DrawingIndex
```

Built once for a drawing whose primitives no longer change: the cached
drawings of views are shared and read only. Each lookup checks that the
primitives are still the ones it was built from (their count and
storage); when they are not, callers fall back to searching them all.

**Traits:** `Default`, `Clone`, `PartialEq`, `std::fmt::Debug`

*Some fields are private.*

**Methods**

- `pub(crate) fn elements(&self, prims: &[Prim]) -> Option<&HashMap<ElementId, Vec<u32>>>`

  The primitives of each element, in drawing order.

- `pub(crate) fn candidates(&self, prims: &[Prim], p: DVec2, tol: f64) -> Option<Vec<u32>>`

  The pickable primitives (lines, fills and pictures of an element)
  that may lie within `tol` of `p`, sorted in drawing order; `None`
  when they should all be searched.


<a id="mod-cc_views-dxf"></a>

## `cc_views::dxf`

<sub>`crates/cc_views/src/dxf.rs`</sub>

DXF export of drawings, for exchanging 2D views with other CAD programs.
The writer is self-contained and writes ASCII DXF R12 (AC1009), the
version every CAD program reads: lines as `LINE` with their dash
patterns as line types, opaque fills as `SOLID` triangles, and texts as
`TEXT` with their alignment. Each primitive goes to the layer the
caller names (the element's category); colors are the nearest
AutoCAD Color Index of the pen's color, so a plot style by color can give
the pens their weights (R12 has no line weights).

**Items:** [`DxfScale`](#cc_views-dxf-dxfscale) · [`aci`](#cc_views-dxf-aci) · [`layer_name`](#cc_views-dxf-layer_name) · [`default_layer`](#cc_views-dxf-default_layer) · [`DxfBlock`](#cc_views-dxf-dxfblock) · [`DxfInsert`](#cc_views-dxf-dxfinsert) · [`DxfBlocks`](#cc_views-dxf-dxfblocks) · [`to_dxf`](#cc_views-dxf-to_dxf) · [`to_dxf_with`](#cc_views-dxf-to_dxf_with)

<a id="cc_views-dxf-dxfscale"></a>

### struct `DxfScale`

```rust
pub struct DxfScale
```

How a drawing is written: coordinates are multiplied by `unit`; text
heights and dash lengths, in paper millimeters, by `paper`.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `unit: f64`
- `paper: f64`

**Methods**

- `pub fn model(scale: u32) -> Self`

  A view in meters drawn at 1:`scale`, written in millimeters at full size.

- `pub const PAPER: Self`

  A drawing already in paper millimeters (a sheet).


<a id="cc_views-dxf-aci"></a>

### fn `aci`

```rust
pub fn aci(c: [u8; 3]) -> u8
```

The nearest index color of `c` (7, black on white paper, for dark colors).

<a id="cc_views-dxf-layer_name"></a>

### fn `layer_name`

```rust
pub fn layer_name(name: &str) -> String
```

A layer name DXF R12 accepts: letters, digits, `$`, `-` and `_`, at
most 31 characters, upper case.

<a id="cc_views-dxf-default_layer"></a>

### fn `default_layer`

```rust
pub fn default_layer(prim: &Prim) -> &'static str
```

The layer of a primitive with no element: annotation lines, texts, or the rest.

<a id="cc_views-dxf-dxfblock"></a>

### struct `DxfBlock`

```rust
pub struct DxfBlock
```

A block definition: primitives in its own frame (drawing units, as the
drawing's), on layer 0, and the blocks it inserts.

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `name: String`
- `prims: Vec<Prim>`
- `inserts: Vec<DxfInsert>`

<a id="cc_views-dxf-dxfinsert"></a>

### struct `DxfInsert`

```rust
pub struct DxfInsert
```

A block placed: its origin at `position` (drawing units), scaled by
`scale` along its own axes (a negative one mirrors), then turned
`rotation_deg` counter-clockwise.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `block: String`
- `position: DVec2`
- `rotation_deg: f64`
- `scale: DVec2`
- `layer: String`

<a id="cc_views-dxf-dxfblocks"></a>

### struct `DxfBlocks`

```rust
pub struct DxfBlocks
```

Blocks written with a drawing: their definitions, where the drawing
inserts them, and the elements they stand for (left out of the
drawing's own entities).

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `blocks: Vec<DxfBlock>`
- `inserts: Vec<DxfInsert>`
- `skip: std::collections::BTreeSet<cc_core::ElementId>`

<a id="cc_views-dxf-to_dxf"></a>

### fn `to_dxf`

```rust
pub fn to_dxf(drawing: &Drawing2D, scale: DxfScale, layer: &dyn Fn(&Prim) -> String) -> Vec<u8>
```

The DXF R12 document of `drawing` (in code page 1252), scaled by
`scale`, each primitive on the layer `layer` names (sanitized with
`layer_name`).

<a id="cc_views-dxf-to_dxf_with"></a>

### fn `to_dxf_with`

```rust
pub fn to_dxf_with(drawing: &Drawing2D, scale: DxfScale, layer: &dyn Fn(&Prim) -> String, blocks: &DxfBlocks) -> Vec<u8>
```

`to_dxf` with `blocks`: their definitions in the BLOCKS section and
their inserts in the entities, in place of the primitives of the
elements in `blocks.skip`. Block names are sanitized like layer names.

<a id="mod-cc_views-export"></a>

## `cc_views::export`

<sub>`crates/cc_views/src/export.rs`</sub>

Vector export of drawings in paper millimeters (composed sheets) to SVG
and PDF. Both writers are self-contained. The PDF uses standard
Helvetica and embeds Inter for lines with a character outside WinAnsi
that has an Inter glyph.

**Items:** [`to_svg`](#cc_views-export-to_svg) · [`to_pdf`](#cc_views-export-to_pdf) · [`MAX_PIXELS`](#cc_views-export-max_pixels) · [`view_on_paper`](#cc_views-export-view_on_paper) · [`view_on_pdf_paper`](#cc_views-export-view_on_pdf_paper) · [`to_png`](#cc_views-export-to_png)

<a id="cc_views-export-to_svg"></a>

### fn `to_svg`

```rust
pub fn to_svg(drawing: &Drawing2D, paper: DVec2) -> String
```

SVG document of `drawing` on a sheet of `paper` millimeters.

<a id="cc_views-export-to_pdf"></a>

### fn `to_pdf`

```rust
pub fn to_pdf(drawing: &Drawing2D, paper: DVec2) -> Vec<u8>
```

PDF document (one page) of `drawing` on a sheet of `paper` millimeters.

<a id="cc_views-export-max_pixels"></a>

### const `MAX_PIXELS`

```rust
pub const MAX_PIXELS: u64
```

The largest raster export, in pixels (about 20000 x 5000, or an A0
sheet at 300 dpi).

<a id="cc_views-export-view_on_paper"></a>

### fn `view_on_paper`

```rust
pub fn view_on_paper(drawing: &Drawing2D, scale: u32, margin: f64) -> (Drawing2D, DVec2)
```

A view's drawing (meters, drawn at 1:`scale`; text sizes in paper
millimeters) laid out on paper: the drawing in millimeters with `margin`
around it, and the paper size.

<a id="cc_views-export-view_on_pdf_paper"></a>

### fn `view_on_pdf_paper`

```rust
pub fn view_on_pdf_paper(drawing: &Drawing2D, scale: u32, margin: f64) -> (Drawing2D, DVec2)
```

Lay out a standalone PDF view, including the extents of embedded text.

<a id="cc_views-export-to_png"></a>

### fn `to_png`

```rust
pub fn to_png(drawing: &Drawing2D, paper: DVec2, dpi: f64) -> Result<Vec<u8>, String>
```

PNG image of `drawing` on a sheet of `paper` millimeters at `dpi` dots
per inch, on white; text is drawn as outlines of Inter (embedded), with
the system's fallback fonts (`fonts::fallbacks`: CJK, Lao) for the
characters Inter lacks.

<a id="mod-cc_views-fonts"></a>

## `cc_views::fonts`

<sub>`crates/cc_views/src/fonts.rs`</sub>

Fallback fonts for the scripts Inter lacks (Chinese, Japanese, Korean,
Lao), found among the fonts operating systems usually ship and read once.
Nothing is bundled, so a system without them shows missing glyphs as
boxes, as before.

**Items:** [`FontFile`](#cc_views-fonts-fontfile) · [`cjk`](#cc_views-fonts-cjk) · [`lao`](#cc_views-fonts-lao) · [`fallbacks`](#cc_views-fonts-fallbacks)

<a id="cc_views-fonts-fontfile"></a>

### struct `FontFile`

```rust
pub struct FontFile
```

A font file and the face to use in it (collections hold several).

**Fields**

- `data: Vec<u8>`
- `index: u32`

**Methods**

- `pub fn font_ref(&self) -> Option<skrifa::FontRef<'_>>`

<a id="cc_views-fonts-cjk"></a>

### fn `cjk`

```rust
pub fn cjk() -> &'static [FontFile]
```

The system's CJK fonts in fallback order (empty when there is none).

<a id="cc_views-fonts-lao"></a>

### fn `lao`

```rust
pub fn lao() -> &'static [FontFile]
```

The system's Lao font (empty when there is none).

<a id="cc_views-fonts-fallbacks"></a>

### fn `fallbacks`

```rust
pub fn fallbacks() -> impl Iterator<Item = &'static FontFile>
```

Every fallback font, in order: CJK, then Lao.

<a id="mod-cc_views-format"></a>

## `cc_views::format`

<sub>`crates/cc_views/src/format.rs`</sub>

Number formatting for drawings, schedules and the interface.
Quantities follow the project units (`cc_core::Units`), set by the
application with `set_units`; with the default number style the
decimal symbol follows the interface language (`set_decimal_comma`).
Unit symbols use the active notation (`set_russian_unit_symbols`).
The `*_with` functions take the project units and decimal symbol
explicitly (used by tests and previews).

**Items:** [`set_decimal_comma`](#cc_views-format-set_decimal_comma) · [`set_russian_unit_symbols`](#cc_views-format-set_russian_unit_symbols) · [`russian_unit_symbols`](#cc_views-format-russian_unit_symbols) · [`unit_symbol`](#cc_views-format-unit_symbol) · [`set_units`](#cc_views-format-set_units) · [`current`](#cc_views-format-current) · [`number_with`](#cc_views-format-number_with) · [`length_number_with`](#cc_views-format-length_number_with) · [`length_with`](#cc_views-format-length_with) · [`length_unit_with`](#cc_views-format-length_unit_with) · [`inch_size_with`](#cc_views-format-inch_size_with) · [`MetricSize`](#cc_views-format-metricsize) · [`size_with`](#cc_views-format-size_with) · [`POUND`](#cc_views-format-pound) · [`Quantity`](#cc_views-format-quantity) · [`quantity_with`](#cc_views-format-quantity_with) · [`area_with`](#cc_views-format-area_with) · [`volume_with`](#cc_views-format-volume_with) · [`angle_with`](#cc_views-format-angle_with) · [`slope_with`](#cc_views-format-slope_with) · [`parse_slope_with`](#cc_views-format-parse_slope_with) · [`money_with`](#cc_views-format-money_with) · [`scale_with`](#cc_views-format-scale_with) · [`paper_with`](#cc_views-format-paper_with) · [`paper_weight_with`](#cc_views-format-paper_weight_with) · [`parse_paper_with`](#cc_views-format-parse_paper_with) · [`parse_number`](#cc_views-format-parse_number) · [`parse_length_with`](#cc_views-format-parse_length_with) · [`parse_size_with`](#cc_views-format-parse_size_with) · [`number`](#cc_views-format-number) · [`meters`](#cc_views-format-meters) · [`length`](#cc_views-format-length) · [`imperial`](#cc_views-format-imperial) · [`bar_length`](#cc_views-format-bar_length) · [`trimmed`](#cc_views-format-trimmed) · [`bar_diameter`](#cc_views-format-bar_diameter) · [`scale`](#cc_views-format-scale) · [`inch_size`](#cc_views-format-inch_size) · [`size`](#cc_views-format-size) · [`quantity`](#cc_views-format-quantity) · [`system`](#cc_views-format-system) · [`parse_size`](#cc_views-format-parse_size) · [`parse_slope`](#cc_views-format-parse_slope) · [`area`](#cc_views-format-area) · [`volume`](#cc_views-format-volume) · [`angle`](#cc_views-format-angle) · [`slope`](#cc_views-format-slope) · [`money`](#cc_views-format-money) · [`parse_length`](#cc_views-format-parse_length) · [`paper`](#cc_views-format-paper) · [`paper_weight`](#cc_views-format-paper_weight) · [`parse_paper`](#cc_views-format-parse_paper) · [`elevation`](#cc_views-format-elevation)

<a id="cc_views-format-set_decimal_comma"></a>

### fn `set_decimal_comma`

```rust
pub fn set_decimal_comma(comma: bool)
```

Use `,` (true) or `.` (false) as the decimal separator of the language.

<a id="cc_views-format-set_russian_unit_symbols"></a>

### fn `set_russian_unit_symbols`

```rust
pub fn set_russian_unit_symbols(enabled: bool)
```

Use Russian unit symbols in human-readable quantities.

<a id="cc_views-format-russian_unit_symbols"></a>

### fn `russian_unit_symbols`

```rust
pub(crate) fn russian_unit_symbols() -> bool
```

<a id="cc_views-format-unit_symbol"></a>

### fn `unit_symbol`

```rust
pub fn unit_symbol(symbol: &str) -> &str
```

The symbol of a displayed unit in the active language.

<a id="cc_views-format-set_units"></a>

### fn `set_units`

```rust
pub fn set_units(units: &Units)
```

Display quantities with `units` from now on.

<a id="cc_views-format-current"></a>

### fn `current`

```rust
pub fn current() -> (Units, bool)
```

The current units and language decimal symbol.

<a id="cc_views-format-number_with"></a>

### fn `number_with`

```rust
pub fn number_with(units: &Units, comma: bool, value: f64, decimals: usize) -> String
```

`value` with `decimals` places, the style's decimal symbol and grouping.

<a id="cc_views-format-length_number_with"></a>

### fn `length_number_with`

```rust
pub fn length_number_with(units: &Units, comma: bool, meters: f64) -> String
```

A length's number in the length unit, without the unit symbol; feet
and inches and fractional inches carry their own marks.

<a id="cc_views-format-length_with"></a>

### fn `length_with`

```rust
pub fn length_with(units: &Units, comma: bool, meters: f64) -> String
```

Length in meters, in the length unit (symbol when the settings ask for it).

<a id="cc_views-format-length_unit_with"></a>

### fn `length_unit_with`

```rust
pub fn length_unit_with(units: &Units, comma: bool, meters: f64) -> String
```

Length always with its unit symbol (property fields, labels).

<a id="cc_views-format-inch_size_with"></a>

### fn `inch_size_with`

```rust
pub fn inch_size_with(units: &Units, comma: bool, meters: f64) -> String
```

A small size (a section's side, a diameter, a layer) in inches, without
feet, with the inch mark: a fraction at the length precision (`1 1/4"`,
`5/16"`, `16"`; sixteenths when lengths are not in fractional inches),
or the length's decimals when lengths are in decimal feet or inches
(`1.25"`).

<a id="cc_views-format-metricsize"></a>

### enum `MetricSize`

```rust
pub enum MetricSize
```

How `size_with` writes a small size in metric projects: each place
keeps the form it always had there.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

**Variants**

- `Length`

  A length with its unit symbol (see `length_unit_with`): `0,20 m`.

- `Mm(usize)`

  Millimeters with this many decimals and the unit: `12,5 mm`.

- `MmNumber(usize)`

  Millimeters with this many decimals, no unit (tags): `200`.

- `Cm(usize)`

  Centimeters with this many decimals and the unit: `18,0 cm`.

- `CmNumber(usize)`

  Centimeters with this many decimals, no unit: `18,0`.

- `CmTag`

  Centimeters, whole or with one decimal, no unit (section tags): `15`.


<a id="cc_views-format-size_with"></a>

### fn `size_with`

```rust
pub fn size_with(units: &Units, comma: bool, meters: f64, metric: MetricSize) -> String
```

A small size (a section's side, a diameter, a layer, a riser): as
`metric` says in metric projects, in inches with the inch mark in
imperial ones (see `inch_size_with`).

<a id="cc_views-format-pound"></a>

### const `POUND`

```rust
pub const POUND: f64
```

Kilograms in a pound (exact, by definition).

<a id="cc_views-format-quantity"></a>

### enum `Quantity`

```rust
pub enum Quantity
```

A physical quantity stored in SI units: shown in them in metric
projects and in US customary units in imperial ones. Engine commands
and files keep the SI value; only the text changes.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

**Variants**

- `Mass`

  Kilograms; pounds.

- `LinearMass`

  Kilograms per meter; pounds per foot.

- `AreaMass`

  Kilograms per square meter; pounds per square foot.

- `Density`

  Kilograms per cubic meter; pounds per cubic foot.

- `Stress`

  Megapascals; kips per square inch.

- `Flow`

  Liters per second; US gallons per minute.


**Methods**

- `pub fn per_si(self, system: cc_core::MeasurementSystem) -> f64`

  Displayed units per SI unit in `system`.

- `pub fn symbol(self, system: cc_core::MeasurementSystem) -> &'static str`

  The unit's symbol in `system`.


<a id="cc_views-format-quantity_with"></a>

### fn `quantity_with`

```rust
pub fn quantity_with(units: &Units, comma: bool, q: Quantity, si: f64, decimals: usize) -> String
```

`si` (a `Quantity` in SI units) in the units' system, with `decimals`
places and its symbol: `12,50 kg`, `27.56 lb`.

<a id="cc_views-format-area_with"></a>

### fn `area_with`

```rust
pub fn area_with(units: &Units, comma: bool, square_meters: f64) -> String
```

<a id="cc_views-format-volume_with"></a>

### fn `volume_with`

```rust
pub fn volume_with(units: &Units, comma: bool, cubic_meters: f64) -> String
```

<a id="cc_views-format-angle_with"></a>

### fn `angle_with`

```rust
pub fn angle_with(units: &Units, comma: bool, degrees: f64) -> String
```

<a id="cc_views-format-slope_with"></a>

### fn `slope_with`

```rust
pub fn slope_with(units: &Units, comma: bool, slope: f64) -> String
```

Slope given as rise over run: `8,3 %`, `4,76°`, or the rise per 12 of
run (`6:12`, `4.5:12`; trailing zeros dropped).

<a id="cc_views-format-parse_slope_with"></a>

### fn `parse_slope_with`

```rust
pub fn parse_slope_with(units: &Units, text: &str) -> Option<f64>
```

Parse a slope as rise over run: a number in the slope unit (percent,
degrees, or the rise per 12 of run), or with its unit typed: `8.3 %`,
`5°`, a ratio `6:12` or `6/12` (`1 1/2:12` too). `None` for anything
else, and for 90° or more.

<a id="cc_views-format-money_with"></a>

### fn `money_with`

```rust
pub fn money_with(units: &Units, comma: bool, amount: f64) -> String
```

<a id="cc_views-format-scale_with"></a>

### fn `scale_with`

```rust
pub fn scale_with(units: &Units, n: u32) -> String
```

The scale 1:`n` as a project with `units` writes it (`1/4" = 1'-0"` in
imperial projects, else `1:n`; see `cc_core::scale::label`).

<a id="cc_views-format-paper_with"></a>

### fn `paper_with`

```rust
pub fn paper_with(units: &Units, comma: bool, mm: f64) -> String
```

A size on paper (text heights, bubbles, gaps, sheet sides), stored in
millimeters, with its unit: `2,5 mm` in metric projects; in imperial
ones inches as a fraction to 1/64 (`3/32"`, `1 1/2"`, `17"`), or with
three decimals when the size is not on a 1/64 step (`0.098"`).

<a id="cc_views-format-paper_weight_with"></a>

### fn `paper_weight_with`

```rust
pub fn paper_weight_with(units: &Units, comma: bool, mm: f64) -> String
```

A pen weight on paper, stored in millimeters: `0,25 mm` in metric
projects, decimal inches in imperial ones (`0.010"`).

<a id="cc_views-format-parse_paper_with"></a>

### fn `parse_paper_with`

```rust
pub fn parse_paper_with(units: &Units, text: &str) -> Option<f64>
```

Parse a size on paper, in millimeters: a bare number is millimeters in
metric projects and inches in imperial ones. A suffix (`mm`, `cm`, `in`,
their Russian spellings, or `"`) overrides the unit. Fractions and mixed
numbers are read (`3/32`, `1 1/2"`); `None` for anything else.

<a id="cc_views-format-parse_number"></a>

### fn `parse_number`

```rust
pub fn parse_number(text: &str) -> Option<f64>
```

Parse a number typed with either decimal symbol and optional grouping.

<a id="cc_views-format-parse_length_with"></a>

### fn `parse_length_with`

```rust
pub fn parse_length_with(units: &Units, text: &str) -> Option<f64>
```

Parse a length typed in the length unit (a trailing Latin or Russian
unit symbol, or feet and inch marks, override it).

<a id="cc_views-format-parse_size_with"></a>

### fn `parse_size_with`

```rust
pub fn parse_size_with(units: &Units, text: &str) -> Option<f64>
```

Parse a small size (see `size_with`) typed in a project of `units`:
in imperial projects a bare number is inches (`3/4`, `1 1/2`, `6`);
otherwise, and with feet or a unit typed, as `parse_length_with`.

<a id="cc_views-format-number"></a>

### fn `number`

```rust
pub fn number(value: f64, decimals: usize) -> String
```

Plain number with the current decimal symbol and grouping.

<a id="cc_views-format-meters"></a>

### fn `meters`

```rust
pub fn meters(value: f64) -> String
```

Length in the project's length unit (dimensions, tags, schedules).

<a id="cc_views-format-length"></a>

### fn `length`

```rust
pub fn length(value: f64) -> String
```

Length with its unit symbol.

<a id="cc_views-format-imperial"></a>

### fn `imperial`

```rust
pub fn imperial() -> bool
```

Whether lengths are shown in US customary units.

<a id="cc_views-format-bar_length"></a>

### fn `bar_length`

```rust
pub fn bar_length(meters: f64) -> String
```

A bar's length (or a leg's, a radius, a spacing) as bending schedules
and bar tags write it: whole centimeters without the unit in metric
projects, the project's length format in imperial ones.

<a id="cc_views-format-trimmed"></a>

### fn `trimmed`

```rust
pub fn trimmed(value: f64, decimals: usize) -> String
```

A number with up to `decimals` places, trailing zeros dropped (12.5, 10).

<a id="cc_views-format-bar_diameter"></a>

### fn `bar_diameter`

```rust
pub fn bar_diameter(meters: f64) -> String
```

A bar diameter as bar tags write it: `Ø12.5` (millimeters), `Ø1/2"` in imperial projects.

<a id="cc_views-format-scale"></a>

### fn `scale`

```rust
pub fn scale(n: u32) -> String
```

The scale 1:`n` in the current units (see `scale_with`).

<a id="cc_views-format-inch_size"></a>

### fn `inch_size`

```rust
pub fn inch_size(meters: f64) -> String
```

A small size in inches with the inch mark (see `inch_size_with`).

<a id="cc_views-format-size"></a>

### fn `size`

```rust
pub fn size(meters: f64, metric: MetricSize) -> String
```

A small size in the current units (see `size_with`).

<a id="cc_views-format-quantity"></a>

### fn `quantity`

```rust
pub fn quantity(q: Quantity, si: f64, decimals: usize) -> String
```

A physical quantity in the current units (see `quantity_with`).

<a id="cc_views-format-system"></a>

### fn `system`

```rust
pub fn system() -> cc_core::MeasurementSystem
```

The measurement system of the current units.

<a id="cc_views-format-parse_size"></a>

### fn `parse_size`

```rust
pub fn parse_size(text: &str) -> Option<f64>
```

Parse a small size in the current units (see `parse_size_with`).

<a id="cc_views-format-parse_slope"></a>

### fn `parse_slope`

```rust
pub fn parse_slope(text: &str) -> Option<f64>
```

Parse a slope in the current units (see `parse_slope_with`).

<a id="cc_views-format-area"></a>

### fn `area`

```rust
pub fn area(value: f64) -> String
```

<a id="cc_views-format-volume"></a>

### fn `volume`

```rust
pub fn volume(value: f64) -> String
```

<a id="cc_views-format-angle"></a>

### fn `angle`

```rust
pub fn angle(degrees: f64) -> String
```

<a id="cc_views-format-slope"></a>

### fn `slope`

```rust
pub fn slope(value: f64) -> String
```

<a id="cc_views-format-money"></a>

### fn `money`

```rust
pub fn money(value: f64) -> String
```

<a id="cc_views-format-parse_length"></a>

### fn `parse_length`

```rust
pub fn parse_length(text: &str) -> Option<f64>
```

<a id="cc_views-format-paper"></a>

### fn `paper`

```rust
pub fn paper(mm: f64) -> String
```

A size on paper in the current units (see `paper_with`).

<a id="cc_views-format-paper_weight"></a>

### fn `paper_weight`

```rust
pub fn paper_weight(mm: f64) -> String
```

A pen weight in the current units (see `paper_weight_with`).

<a id="cc_views-format-parse_paper"></a>

### fn `parse_paper`

```rust
pub fn parse_paper(text: &str) -> Option<f64>
```

Parse a size on paper in the current units (see `parse_paper_with`).

<a id="cc_views-format-elevation"></a>

### fn `elevation`

```rust
pub fn elevation(value: f64) -> String
```

Level elevation with an explicit sign, e.g. `+3,00`.

<a id="mod-cc_views-graphics"></a>

## `cc_views::graphics`

<sub>`crates/cc_views/src/graphics.rs`</sub>

View graphics applied to drawings: hidden categories, filters and
elements leave the drawing, as do elements the view's phase and phase
filter leave out, and the resolved overrides change the pens of lines and
the colors and patterns of fills (see `cc_core::graphics` and
`cc_core::phase`).

**Items:** [`filter_matches`](#cc_views-graphics-filter_matches) · [`filter_elements`](#cc_views-graphics-filter_elements) · [`PhaseView`](#cc_views-graphics-phaseview) · [`Resolver`](#cc_views-graphics-resolver) · [`apply`](#cc_views-graphics-apply)

<a id="cc_views-graphics-filter_matches"></a>

### fn `filter_matches`

```rust
pub fn filter_matches(project: &Project, geom: &ModelGeometry, filter: &ViewFilter, e: &Element) -> bool
```

Whether `filter` matches the element `e`.

<a id="cc_views-graphics-filter_elements"></a>

### fn `filter_elements`

```rust
pub fn filter_elements(project: &Project, geom: &ModelGeometry, filter: &ViewFilter) -> Vec<ElementId>
```

The elements `filter` matches, in id order.

<a id="cc_views-graphics-phaseview"></a>

### struct `PhaseView`

```rust
pub struct PhaseView<'a>
```

A view's phase (an index into the project's phases) and phase filter.

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `index: usize`
- `filter: Option<&'a PhaseFilter>`

**Methods**

- `pub fn of(project: &'a Project, view: &View) -> Option<Self>`

  The phase of `view`, or `None` for views without one (legends,
  sheets, diagrams) or projects without phases.

- `pub fn style(&self, project: &Project, e: &Element) -> Option<GraphicOverride>`

  The phase overrides of `e` in the view, or `None` when the view
  does not show it (absent from the phase, or a status the filter does
  not display).

- `pub fn shows(&self, project: &Project, e: &Element) -> bool`

  Whether the view shows `e`.


<a id="cc_views-graphics-resolver"></a>

### struct `Resolver`

```rust
pub struct Resolver<'a>
```

Resolves the graphics of elements in one view (cached per element).

*Some fields are private.*

**Methods**

- `pub fn new(project: &'a Project, geom: &'a ModelGeometry, view: &'a View, temporary: Option<&'a TemporaryVisibility>) -> Self`
- `pub fn is_trivial(&self) -> bool`

  Whether the view draws everything as generated.

- `pub fn style(&mut self, id: ElementId) -> Option<GraphicOverride>`

  The overrides of an element, or `None` when it is hidden. Ids that
  are not elements are drawn as they are.


<a id="cc_views-graphics-apply"></a>

### fn `apply`

```rust
pub fn apply(d: &mut Drawing2D, resolver: &mut Resolver, scale: u32)
```

Apply the view's graphics to a generated drawing at `1:scale`.

<a id="mod-cc_views-hatch"></a>

## `cc_views::hatch`

<sub>`crates/cc_views/src/hatch.rs`</sub>

Hatch patterns: the line work of a material `Pattern` clipped to a
polygon. Lines are anchored to the drawing origin so neighbouring
regions line up; `spacing` is the pattern's base spacing in drawing
units (callers derive it from paper millimeters and the view scale).

**Items:** [`hatch`](#cc_views-hatch-hatch) · [`hatch_with_holes`](#cc_views-hatch-hatch_with_holes) · [`MAX_CUSTOM_SEGMENTS`](#cc_views-hatch-max_custom_segments) · [`MIN_PAPER_SPACING`](#cc_views-hatch-min_paper_spacing) · [`paper_unit`](#cc_views-hatch-paper_unit) · [`custom_unit`](#cc_views-hatch-custom_unit) · [`hatch_in`](#cc_views-hatch-hatch_in) · [`hatch_with_holes_in`](#cc_views-hatch-hatch_with_holes_in) · [`hatch_custom`](#cc_views-hatch-hatch_custom)

<a id="cc_views-hatch-hatch"></a>

### fn `hatch`

```rust
pub fn hatch(pattern: Pattern, poly: &[DVec2], spacing: f64) -> Vec<(DVec2, DVec2)>
```

Segments of `pattern` inside `poly`. `Solid` and `None` give no lines
(solid patterns are filled by the caller).

<a id="cc_views-hatch-hatch_with_holes"></a>

### fn `hatch_with_holes`

```rust
pub fn hatch_with_holes(pattern: Pattern, poly: &[DVec2], holes: &[Vec<DVec2>], spacing: f64) -> Vec<(DVec2, DVec2)>
```

Segments of `pattern` inside `poly` and outside each of `holes`.

<a id="cc_views-hatch-max_custom_segments"></a>

### const `MAX_CUSTOM_SEGMENTS`

```rust
pub const MAX_CUSTOM_SEGMENTS: usize
```

Most segments one region may produce with a custom pattern.

<a id="cc_views-hatch-min_paper_spacing"></a>

### const `MIN_PAPER_SPACING`

```rust
pub const MIN_PAPER_SPACING: f64
```

Families whose lines would print closer than this are left out
(they would print as a gray blot), paper millimeters.

<a id="cc_views-hatch-paper_unit"></a>

### fn `paper_unit`

```rust
pub fn paper_unit(spacing: f64) -> f64
```

Drawing units per paper millimeter for a hatch `spacing` (see `crate::cut::spacing`).

<a id="cc_views-hatch-custom_unit"></a>

### fn `custom_unit`

```rust
pub fn custom_unit(fp: &FillPattern, spacing: f64) -> f64
```

Drawing units per millimeter of `fp`'s definition, with its scale: paper
millimeters for drafting patterns, real ones (drawings in meters) for
model patterns.

<a id="cc_views-hatch-hatch_in"></a>

### fn `hatch_in`

```rust
pub fn hatch_in(patterns: &FillPatterns, pattern: Pattern, poly: &[DVec2], spacing: f64) -> Vec<(DVec2, DVec2)>
```

Segments of `pattern` (built-in, or one of `patterns`) inside `poly`;
a custom pattern that is gone gives none.

<a id="cc_views-hatch-hatch_with_holes_in"></a>

### fn `hatch_with_holes_in`

```rust
pub fn hatch_with_holes_in(patterns: &FillPatterns, pattern: Pattern, poly: &[DVec2], holes: &[Vec<DVec2>], spacing: f64) -> Vec<(DVec2, DVec2)>
```

`hatch_in` outside each of `holes`.

<a id="cc_views-hatch-hatch_custom"></a>

### fn `hatch_custom`

```rust
pub fn hatch_custom(fp: &FillPattern, poly: &[DVec2], unit: f64, paper: f64) -> Vec<(DVec2, DVec2)>
```

The line families of `fp` inside `poly`, its lengths at `unit` drawing
units per millimeter (scale included), `paper` drawing units per paper
millimeter (too dense families are left out; dots are drawn as short
strokes). Lines are anchored to the drawing origin, so neighbouring
regions line up.

<a id="mod-cc_views-joins"></a>

## `cc_views::joins`

<sub>`crates/cc_views/src/joins.rs`</sub>

Joined geometry in cut views (plans and sections): the outline of the
element that is cut stops where the cutting element begins, the cutting
element is painted over it, and when both are cut in the same material
the line between them goes, so they read as one piece.

**Items:** [`merge`](#cc_views-joins-merge)

<a id="cc_views-joins-merge"></a>

### fn `merge`

```rust
pub(crate) fn merge(project: &Project, d: &mut Drawing2D)
```

Merge the cut regions of joined elements drawn in `d`.

<a id="mod-cc_views-legend"></a>

## `cc_views::legend`

<sub>`crates/cc_views/src/legend.rs`</sub>

**Items:** [`legend`](#cc_views-legend-legend)

<a id="cc_views-legend-legend"></a>

### fn `legend`

```rust
pub fn legend(project: &Project, types: &[ElementTypeId], scale: u32) -> Drawing2D
```

Legend: one row per type with a 2D symbol and its name.
Rows are spaced in paper units so the legend reads the same at any scale.

<a id="mod-cc_views-level_marks"></a>

## `cc_views::level_marks`

<sub>`crates/cc_views/src/level_marks.rs`</sub>

Level marks: the finished level above a shelf line and the raw level
under it, beside a symbol on the point measured. Plans draw a circle
with two filled quarters crossed by the shelf and a vertical line;
elevations and sections a triangle pointing down to the point on a stem
that the shelf crosses. Sizes are paper millimeters converted with the
view scale, like the other annotations.

**Items:** [`texts`](#cc_views-level_marks-texts) · [`level_marks`](#cc_views-level_marks-level_marks)

<a id="cc_views-level_marks-texts"></a>

### fn `texts`

```rust
pub fn texts(project: &Project, view: &View, mark: &LevelMark, data: &TypeData) -> Option<Vec<String>>
```

The texts a mark writes: the finished level, then the raw one when it
has one, each with its sign, the type's datum added, and its suffix.

<a id="cc_views-level_marks-level_marks"></a>

### fn `level_marks`

```rust
pub fn level_marks(d: &mut Drawing2D, project: &Project, view: &View)
```

The level marks of `view`.

<a id="mod-cc_views-mep"></a>

## `cc_views::mep`

<sub>`crates/cc_views/src/mep.rs`</sub>

Building services in plans: pipes and conduits as bands in their
system's color with diameter, slope and circuit tags, cable trays as
outlines as wide as they are, valves on pipes, and electrical devices
as the usual plan symbols (sizes in paper millimeters).
Symbols follow common Brazilian practice: an outlet is a triangle,
hollow when low, half filled at medium height and filled when high; a
ceiling light is a circle with its circuit above its power; a switch is
a small filled circle; a board is a rectangle with a filled half.

**Items:** [`services`](#cc_views-mep-services) · [`discipline_of`](#cc_views-mep-discipline_of) · [`valve_symbol`](#cc_views-mep-valve_symbol) · [`Frame`](#cc_views-mep-frame) · [`symbol`](#cc_views-mep-symbol)

<a id="cc_views-mep-services"></a>

### fn `services`

```rust
pub(crate) fn services(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, level: LevelId, cut_height: f64, discipline: MepDiscipline, scale: u32)
```

Pipes or conduits of `level` and, in electrical plans, the device symbols.

<a id="cc_views-mep-discipline_of"></a>

### fn `discipline_of`

```rust
pub fn discipline_of(class: SystemClass) -> MepDiscipline
```

The services plan that shows devices of a family of systems.

<a id="cc_views-mep-valve_symbol"></a>

### fn `valve_symbol`

```rust
pub(crate) fn valve_symbol(d: &mut Drawing2D, c: DVec2, dir: DVec2, mm: f64, kind: cc_core::ValveKind, id: Option<cc_core::ElementId>)
```

The plan symbol of a valve at `c` on a pipe running along `dir`: two
triangles meeting at their tips (a gate valve); a ball valve adds a
hollow circle, a globe valve a filled one, and a check valve fills the
triangle the flow leaves through. `mm` is one paper millimeter.

<a id="cc_views-mep-frame"></a>

### struct `Frame`

```rust
pub(crate) struct Frame
```

Where a symbol is drawn: the device's insertion point and rotation.
Wall devices stand out towards their local -Y.

**Fields**

- `origin: DVec2`
- `angle: f64`
- `mm: f64`

  One paper millimeter in model meters.


<a id="cc_views-mep-symbol"></a>

### fn `symbol`

```rust
pub(crate) fn symbol(d: &mut Drawing2D, f: &Frame, device: DeviceKind, height: f64, power: f64, circuit: Option<&str>, id: Option<cc_core::ElementId>)
```

The plan symbol of an electrical device mounted `height` above its level.

<a id="mod-cc_views-notes"></a>

## `cc_views::notes`

<sub>`crates/cc_views/src/notes.rs`</sub>

Text notes and their leaders. Sizes are paper millimeters converted with
the view scale, like the other annotations; text widths are estimated
from the character count (the same estimate wraps multiline text, places
leaders and makes the text clickable).

**Items:** [`wrap`](#cc_views-notes-wrap) · [`TextLayout`](#cc_views-notes-textlayout) · [`text_layout`](#cc_views-notes-text_layout) · [`leader_points`](#cc_views-notes-leader_points) · [`leader_symbol`](#cc_views-notes-leader_symbol) · [`SymbolPens`](#cc_views-notes-symbolpens) · [`symbol_fill`](#cc_views-notes-symbol_fill) · [`symbol_at`](#cc_views-notes-symbol_at) · [`draw_note`](#cc_views-notes-draw_note) · [`note_size`](#cc_views-notes-note_size) · [`notes`](#cc_views-notes-notes)

<a id="cc_views-notes-wrap"></a>

### fn `wrap`

```rust
pub fn wrap(text: &str, size: f64, width: Option<f64>) -> Vec<String>
```

Lines of `text`: explicit line breaks, and words wrapped to `width`
paper millimeters when given (a word longer than the width is split).

<a id="cc_views-notes-textlayout"></a>

### struct `TextLayout`

```rust
pub struct TextLayout
```

Where a note's text goes, in its view's drawing coordinates.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `lines: Vec<(DVec2, String)>`

  Baseline start of each line.

- `angle: f64`

  Text angle, radians.

- `size: f64`

  Text size, paper millimeters.

- `outline: [DVec2; 4]`

  Corners of the text box (top left, top right, bottom right, bottom left).

- `sides: [DVec2; 2]`

  Leader attachment points beside the first line, on the left and right.


**Methods**

- `pub fn attach(&self, toward: DVec2) -> DVec2`

  The attachment point on the side facing `toward`.


<a id="cc_views-notes-text_layout"></a>

### fn `text_layout`

```rust
pub fn text_layout(note: &TextNote, size: f64, scale: u32) -> TextLayout
```

Layout of `note` with text of `size` paper millimeters at `scale`.

<a id="cc_views-notes-leader_points"></a>

### fn `leader_points`

```rust
pub fn leader_points(layout: &TextLayout, leader: &Leader) -> Vec<DVec2>
```

The polyline of a leader: from the text to its elbow (if any) and end.

<a id="cc_views-notes-leader_symbol"></a>

### fn `leader_symbol`

```rust
pub fn leader_symbol(d: &mut Drawing2D, symbol: LeaderSymbol, from: DVec2, end: DVec2, size: f64, scale: u32, id: Option<ElementId>)
```

The symbol at `end` of a leader coming from `from`, sized for text of
`size` paper millimeters at `scale`.

<a id="cc_views-notes-symbolpens"></a>

### struct `SymbolPens`

```rust
pub struct SymbolPens
```

Pens of a symbol's strokes and of a tick, and the color of its fills.

**Traits:** `Debug`, `Clone`, `Copy`, `Default`

**Fields**

- `stroke: (LineStyle, Option<crate::Pen>)`
- `tick: (LineStyle, Option<crate::Pen>)`
- `fill: [u8; 4]`

<a id="cc_views-notes-symbol_fill"></a>

### fn `symbol_fill`

```rust
pub fn symbol_fill(color: Option<[u8; 3]>) -> [u8; 4]
```

The fill of filled symbols drawn in `color` (the default symbol fill for none).

<a id="cc_views-notes-symbol_at"></a>

### fn `symbol_at`

```rust
pub fn symbol_at(d: &mut Drawing2D, symbol: LeaderSymbol, u: DVec2, end: DVec2, len: f64, pens: SymbolPens, id: Option<ElementId>)
```

`symbol` at `end` of a line arriving along `u` (a unit vector), `len`
drawing units long.

<a id="cc_views-notes-draw_note"></a>

### fn `draw_note`

```rust
pub fn draw_note(d: &mut Drawing2D, note: &TextNote, size: f64, scale: u32, id: Option<ElementId>)
```

Draw a note: its text, a clickable area and its leaders.

<a id="cc_views-notes-note_size"></a>

### fn `note_size`

```rust
pub fn note_size(project: &Project, type_id: cc_core::ElementTypeId) -> f64
```

Text size of a note's type (paper millimeters).

<a id="cc_views-notes-notes"></a>

### fn `notes`

```rust
pub fn notes(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32)
```

The text notes placed in `view`.

<a id="mod-cc_views-plan"></a>

## `cc_views::plan`

<sub>`crates/cc_views/src/plan.rs`</sub>

**Re-exports**

- `pub(crate) use crate::cut::POCHE`

**Items:** [`plan`](#cc_views-plan-plan) · [`is_structural`](#cc_views-plan-is_structural) · [`plan_of`](#cc_views-plan-plan_of) · [`section_marks`](#cc_views-plan-section_marks) · [`section_labels`](#cc_views-plan-section_labels) · [`section_pieces`](#cc_views-plan-section_pieces) · [`style_underlay`](#cc_views-plan-style_underlay) · [`strip_fills`](#cc_views-plan-strip_fills)

<a id="cc_views-plan-plan"></a>

### fn `plan`

```rust
pub fn plan(project: &Project, geom: &ModelGeometry, level: LevelId, cut_height: f64, ceiling: bool, scale: u32) -> Drawing2D
```

Floor plan (`ceiling == false`) or reflected ceiling plan of `level`.

<a id="cc_views-plan-is_structural"></a>

### fn `is_structural`

```rust
pub fn is_structural(e: &cc_core::Element) -> bool
```

Whether a structural plan shows the element: slabs, the load-bearing
walls and columns, beams and foundations.

<a id="cc_views-plan-plan_of"></a>

### fn `plan_of`

```rust
pub fn plan_of(project: &Project, geom: &ModelGeometry, level: LevelId, cut_height: f64, (ceiling, structural): (bool, bool), scale: u32, (detail, style): (DetailLevel, VisualStyle)) -> Drawing2D
```

A plan as `plan` draws it, at `detail`; a `structural` plan shows only
the structure, with the beams above the cut drawn as seen from below. A
floor plan in a visual `style` that fills faces draws the pieces of
components (and of swept profiles and connections) as seen from above:
their edges only where no face of them lies over them (see
`PlanCover`); the wireframe sees through them.

<a id="cc_views-plan-section_marks"></a>

### fn `section_marks`

```rust
pub(crate) fn section_marks(d: &mut Drawing2D, project: &Project, scale: u32)
```

The marks of every section in a plan at 1:`scale`, each listed in
`d.sections` to be picked, moved and flipped.

<a id="cc_views-plan-section_labels"></a>

### fn `section_labels`

```rust
pub(crate) fn section_labels(project: &Project, view: &cc_core::View) -> (String, String)
```

What a section head shows: the drawing's number on its sheet and the
sheet's number, or, while the section is on no sheet, the last word of
its name (its letter, as in "Section A") over a dash.

<a id="cc_views-plan-section_pieces"></a>

### fn `section_pieces`

```rust
pub fn section_pieces(start: DVec2, end: DVec2, segments: Option<[f64; 2]>, head: f64) -> Vec<(DVec2, DVec2)>
```

The pieces of a section line drawn between its heads: the whole line, or
with `segments` only that much of it next to each head (the whole line
when they would meet). The line starts at the base of each head's
triangle, `head` from its end.

<a id="cc_views-plan-style_underlay"></a>

### fn `style_underlay`

```rust
pub fn style_underlay(geom: &ModelGeometry, cut_z: f64, ceiling: bool, mut look: crate::Look<'_>, patterns: &cc_core::fill_pattern::FillPatterns) -> Vec<crate::Prim>
```

The faces a plan sees beyond its cut plane at `cut_z`, filled in the
view's visual style, to lie under the plan's linework: what is below the
cut in floor plans, above it in ceiling plans (the shaded, consistent
colors and textures styles). Only fills and surface patterns: the plan
draws its own edges and cuts.
The sun's shadows of a floor plan read the site
(`crate::shadow::Catch::Site`): the whole model casts them, what
stands above the cut plane too, on the faces and on the ground, but for
what is indoors of them (the spaces the look's shadows carry; none
when it carries none). A ceiling plan draws none.

<a id="cc_views-plan-strip_fills"></a>

### fn `strip_fills`

```rust
pub fn strip_fills(d: &mut Drawing2D)
```

The wireframe style of a plan: no fills (but the invisible room areas
that make rooms pickable) and no hatches, only lines.

<a id="mod-cc_views-projected"></a>

## `cc_views::projected`

<sub>`crates/cc_views/src/projected.rs`</sub>

Orthographic projected views: elevations, sections and axonometrics,
with painter's-algorithm hidden surface removal.
Faces are painted back to front. The order comes from comparing the depth
of every pair of faces that overlap on screen, at a point inside their
overlap; sorting by centroid depth alone fails for large faces such as roofs.

**Items:** [`Frame`](#cc_views-projected-frame) · [`Look`](#cc_views-projected-look) · [`CutStyle`](#cc_views-projected-cutstyle) · [`render`](#cc_views-projected-render) · [`clip_convex`](#cc_views-projected-clip_convex) · [`chain_loops`](#cc_views-projected-chain_loops) · [`view_frame`](#cc_views-projected-view_frame) · [`elevation`](#cc_views-projected-elevation) · [`section`](#cc_views-projected-section) · [`axonometric`](#cc_views-projected-axonometric) · [`analytical`](#cc_views-projected-analytical) · [`rebar`](#cc_views-projected-rebar)

<a id="cc_views-projected-frame"></a>

### struct `Frame`

```rust
pub struct Frame
```

Orthographic view frame with optional clip planes.

**Traits:** `Debug`, `Clone`, `Copy`

**Fields**

- `origin: DVec3`
- `right: DVec3`
- `up: DVec3`
- `forward: DVec3`

  View direction (into the scene).

- `near: Option<f64>`

  Keep depth in `near..far` (relative to `origin`).

- `far: Option<f64>`
- `crop_x: Option<(f64, f64)>`

  Keep screen x in this range.


**Methods**

- `pub fn looking(origin: DVec3, forward: DVec3) -> Self`
- `pub fn project(&self, p: DVec3) -> DVec2`
- `pub fn depth(&self, p: DVec3) -> f64`

<a id="cc_views-projected-look"></a>

### struct `Look`

```rust
pub struct Look<'a>
```

How a view draws the model: its detail level, its visual style, the
spacing of patterns (cut and surface hatches) in drawing units, the
light that shades its faces and the shadows it casts.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `detail: DetailLevel`
- `style: VisualStyle`
- `spacing: f64`
- `light: Option<DVec3>`

  The sun lighting the faces (shaded and textures styles): the unit
  vector towards it in project coordinates, or the zero vector when it
  is below the horizon (ambient light only). `None`: the default light,
  fixed to the view.

- `shadows: Option<Shadows<'a>>`

  The shadows the sun casts while it is above the horizon, in the
  styles that fill faces with more than white (see `shadow`); `None`: none.

- `shading: ShadingSettings`

  How light faces turned away from the light are and how dark shadows
  are (`cc_core::View::shading`).


**Methods**

- `pub fn of(project: &Project, geom: &ModelGeometry, view: &cc_core::View, skip: Option<&'a shadow::Skip>, indoors: Option<&'a shadow::Indoors>) -> Self`

  The look of `view` in `project`: lit by the view's sun when it has
  one (`cc_core::View::sun`), which casts shadows when it is set to
  (`cc_core::sun::SunSettings::shadows`): on the model `geom` and on
  the ground plane (`Project::sun_ground_height`), `skip` being the
  elements the view's graphics leave out of them. What is `indoors`
  of a floor plan's or a section's faces catches none (without it,
  every face catches them; see `shadow::Catch`).

- `pub fn casts_shadows(view: &cc_core::View) -> bool`

  Whether `view` is set to draw its sun's shadows: it has a sun that
  casts them, in a visual style that shows them (wherever the sun
  stands). Ceiling plans, which look up at what the sun does not
  reach, draw none.

- `pub fn plain() -> Self`

  Shaded, at the medium detail level, patterns as at 1:100, the default light, no shadows.


<a id="cc_views-projected-cutstyle"></a>

### struct `CutStyle`

```rust
pub struct CutStyle<'a>
```

How section cuts are filled: the project's materials at a hatch spacing.

**Traits:** `Clone`, `Copy`

**Fields**

- `project: &'a Project`
- `spacing: f64`

<a id="cc_views-projected-render"></a>

### fn `render`

```rust
pub fn render(geom: &ModelGeometry, frame: &Frame, cut: Option<CutStyle>, look: Look<'_>, patterns: &cc_core::fill_pattern::FillPatterns) -> Drawing2D
```

Render the solids a view shows through `frame`, at its detail level and
in its visual style; section cuts use `cut` (else, and at the coarse
level, the neutral poche; in the wireframe, only their outline).

<a id="cc_views-projected-clip_convex"></a>

### fn `clip_convex`

```rust
pub(crate) fn clip_convex(subject: &[DVec2], clipper: &[DVec2]) -> Vec<DVec2>
```

Clip a polygon by a counter-clockwise convex polygon (Sutherland-Hodgman;
a polygon that is not convex may come out with sides along the clip's).

<a id="cc_views-projected-chain_loops"></a>

### fn `chain_loops`

```rust
pub(crate) fn chain_loops(segs: &[(DVec2, DVec2)]) -> Vec<Vec<DVec2>>
```

Join segments end-to-end into closed loops.

<a id="cc_views-projected-view_frame"></a>

### fn `view_frame`

```rust
pub fn view_frame(view: &cc_core::View) -> Option<Frame>
```

The frame through which a projected view (elevation, section, the 2D
form of a 3D view) maps the model onto its drawing, as its generator
builds it; `None` for other views (plans draw plan coordinates as they are).

<a id="cc_views-projected-elevation"></a>

### fn `elevation`

```rust
pub fn elevation(project: &Project, geom: &ModelGeometry, direction: Direction, look: Look<'_>) -> Drawing2D
```

<a id="cc_views-projected-section"></a>

### fn `section`

```rust
pub fn section(project: &Project, geom: &ModelGeometry, (start, end): (DVec2, DVec2), depth: f64, scale: u32, look: Look<'_>) -> Drawing2D
```

<a id="cc_views-projected-axonometric"></a>

### fn `axonometric`

```rust
pub fn axonometric(project: &Project, geom: &ModelGeometry, camera: &CameraState, look: Look<'_>) -> Drawing2D
```

<a id="cc_views-projected-analytical"></a>

### fn `analytical`

```rust
pub fn analytical(project: &Project, geom: &ModelGeometry, camera: &CameraState) -> Drawing2D
```

The analytical model seen along the camera: member lines (columns and
beams) and nodes, supports marked with a triangle.

<a id="cc_views-projected-rebar"></a>

### fn `rebar`

```rust
pub fn rebar(geom: &ModelGeometry, camera: &CameraState) -> Drawing2D
```

The reinforcing bars seen along the camera, tagged with their sets.

<a id="mod-cc_views-raster"></a>

## `cc_views::raster`

<sub>`crates/cc_views/src/raster.rs`</sub>

Imported pictures: reading their size and decoding their pixels.

**Items:** [`Rgba`](#cc_views-raster-rgba) · [`probe`](#cc_views-raster-probe) · [`decode`](#cc_views-raster-decode) · [`fade`](#cc_views-raster-fade)

<a id="cc_views-raster-rgba"></a>

### struct `Rgba`

```rust
pub struct Rgba
```

Decoded pixels: width, height and RGBA rows from the top.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `width: u32`
- `height: u32`
- `pixels: Vec<u8>`

<a id="cc_views-raster-probe"></a>

### fn `probe`

```rust
pub fn probe(data: Vec<u8>) -> Option<RasterImage>
```

An image file read as a `RasterImage` (format from its signature, size
from its header); `None` when it is not a picture this reads, or too big.

<a id="cc_views-raster-decode"></a>

### fn `decode`

```rust
pub fn decode(image: &RasterImage, max_side: u32) -> Option<Rgba>
```

The pixels of `image`, reduced (by whole steps) so that neither side
exceeds `max_side`; `None` when it cannot be decoded.

<a id="cc_views-raster-fade"></a>

### fn `fade`

```rust
pub fn fade(pixels: &mut [u8])
```

`c` mixed two thirds with white (as faded lines are).

<a id="mod-cc_views-region"></a>

## `cc_views::region`

<sub>`crates/cc_views/src/region.rs`</sub>

Filled and masking regions in the view they belong to (see
`cc_core::region`), in id order, over the model's linework and under
the view's annotations: a filled region as its type's fill (a solid
color, or a clear background under pattern lines), a masking region
opaque white; then the boundary in its line style, when it has one. A
region with openings is one keyhole ring
(`cc_geom::polygon::keyhole`), or triangles where no ring could be
made.

**Items:** [`MASK`](#cc_views-region-mask) · [`regions`](#cc_views-region-regions) · [`base_color`](#cc_views-region-base_color) · [`draw_filled`](#cc_views-region-draw_filled) · [`draw_masking`](#cc_views-region-draw_masking)

<a id="cc_views-region-mask"></a>

### const `MASK`

```rust
pub const MASK: [u8; 4]
```

Opaque white: masking regions.

<a id="cc_views-region-regions"></a>

### fn `regions`

```rust
pub fn regions(d: &mut Drawing2D, project: &Project, view: ViewId, scale: u32)
```

The regions of `view`, at `1:scale`.

<a id="cc_views-region-base_color"></a>

### fn `base_color`

```rust
pub fn base_color(fill: &PatternFill) -> [u8; 4]
```

The color under a fill's pattern lines: a solid pattern's color, else
clear.

<a id="cc_views-region-draw_filled"></a>

### fn `draw_filled`

```rust
pub fn draw_filled(d: &mut Drawing2D, r: &Region, fill: &PatternFill, styles: &LineStyles, patterns: &cc_core::fill_pattern::FillPatterns, scale: u32, id: Option<ElementId>)
```

Draw a filled region with `fill`.

<a id="cc_views-region-draw_masking"></a>

### fn `draw_masking`

```rust
pub fn draw_masking(d: &mut Drawing2D, r: &Region, styles: &LineStyles, id: Option<ElementId>)
```

Draw a masking region.

<a id="mod-cc_views-schedule"></a>

## `cc_views::schedule`

<sub>`crates/cc_views/src/schedule.rs`</sub>

Schedules: one row per element of a category, one column per field of the
schedule definition, with filtering, sorting and totals.

**Items:** [`Cell`](#cc_views-schedule-cell) · [`Table`](#cc_views-schedule-table) · [`Row`](#cc_views-schedule-row) · [`totaled_by_default`](#cc_views-schedule-totaled_by_default) · [`field_value`](#cc_views-schedule-field_value) · [`schedule`](#cc_views-schedule-schedule) · [`schedule_where`](#cc_views-schedule-schedule_where) · [`to_csv`](#cc_views-schedule-to_csv) · [`MAX_DRAWN_ROWS`](#cc_views-schedule-max_drawn_rows) · [`drawing`](#cc_views-schedule-drawing)

<a id="cc_views-schedule-cell"></a>

### enum `Cell`

```rust
pub enum Cell
```

A cell: plain text, or a translation key for the application to resolve.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Variants**

- `Text(String)`
- `Key(&'static str)`

**Methods**

- `pub fn raw(&self) -> &str`

  The text, or the key itself for untranslated cells.


<a id="cc_views-schedule-table"></a>

### struct `Table`

```rust
pub struct Table
```

A computed schedule. Headers are translation keys.

**Traits:** `Debug`, `Clone`, `PartialEq`, `Default`

**Fields**

- `headers: Vec<&'static str>`
- `rows: Vec<Row>`
- `totals: Vec<String>`

  Totals row aligned with `headers` (the first cell holds the row count);
  empty when totals are hidden.

- `summary: Option<Box<Table>>`

  A table under this one (a bending schedule's summary per diameter).

- `caption: Option<&'static str>`

  The title of a table drawn under another (a translation key).

- `titles: Vec<Option<String>>`

  A column's own heading, where the schedule gives one instead of the
  header's translation (aligned with `headers`; shorter: none).

- `align: Vec<cc_core::ColumnAlign>`

  Where each column's texts stand (aligned with `headers`; shorter: automatic).

- `appearance: cc_core::ScheduleAppearance`

  How the table looks on paper.


**Methods**

- `pub fn header_texts(&self, label: &dyn Fn(&str) -> String) -> Vec<String>`

  The headings of the columns: each its own title, else `label` of its header key.


<a id="cc_views-schedule-row"></a>

### struct `Row`

```rust
pub struct Row
```

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `element: cc_core::ElementId`
- `cells: Vec<Cell>`
- `heading: bool`

  A group's heading (its first cell names the group, across the row)
  rather than a row of values.

- `sketch: Option<(usize, crate::bending::BarSketch)>`

  A bar's shape drawn in cell `.0` instead of its text (bending schedules).

- `footer: bool`

  A group's footer: how many elements it holds in its first cell, and
  its totals in the totaled columns.


**Methods**

- `pub fn new(element: cc_core::ElementId, cells: Vec<Cell>) -> Self`

  A row of values of `element`.

- `pub fn heading(element: cc_core::ElementId, text: Cell, columns: usize) -> Self`

  A group's heading (or, with no text, a blank line) across `columns` cells.


<a id="cc_views-schedule-totaled_by_default"></a>

### fn `totaled_by_default`

```rust
pub fn totaled_by_default(field: ScheduleField) -> bool
```

Whether a schedule adds the column of `field` up unless its format says otherwise.

<a id="cc_views-schedule-field_value"></a>

### fn `field_value`

```rust
pub fn field_value(project: &Project, geom: &ModelGeometry, e: &Element, field: ScheduleField) -> Option<(String, Option<f64>)>
```

A field's value for view filters: its text (keys reduced to their last
word, e.g. `yes`, `exterior`) and, for quantities, its number in SI
units (meters, square and cubic meters; yes/no as 1/0; diameters in
meters). `None` when the element has no value.

<a id="cc_views-schedule-schedule"></a>

### fn `schedule`

```rust
pub fn schedule(project: &Project, geom: &ModelGeometry, category: Category, def: &ScheduleDef) -> Table
```

<a id="cc_views-schedule-schedule_where"></a>

### fn `schedule_where`

```rust
pub fn schedule_where(project: &Project, geom: &ModelGeometry, category: Category, def: &ScheduleDef, keep: &mut dyn FnMut(&Element) -> bool) -> Table
```

A schedule of the elements `keep` accepts (those a schedule view's
phase and phase filter show, see `crate::graphics::PhaseView`).

<a id="cc_views-schedule-to_csv"></a>

### fn `to_csv`

```rust
pub fn to_csv(table: &Table, header: impl Fn(&str) -> String, cell: impl Fn(&Cell) -> String) -> String
```

The table as CSV: `;`-separated when the decimal separator is a comma, so
spreadsheets in those locales open it directly. `header` and `cell`
translate keys.

<a id="cc_views-schedule-max_drawn_rows"></a>

### const `MAX_DRAWN_ROWS`

```rust
pub const MAX_DRAWN_ROWS: usize
```

Most element rows drawn; the rest are counted in a last row.

<a id="cc_views-schedule-drawing"></a>

### fn `drawing`

```rust
pub fn drawing(table: &Table, title: &str, scale: u32, label: &dyn Fn(&str) -> String) -> crate::Drawing2D
```

A schedule as a drawing, for sheets: a title row, the headers, one row
per element and the totals, ruled like a table, then any table under it
(a bending schedule's summary) with its caption. Sizes are paper
millimeters at the view's `scale` (as the panel diagrams), so a sheet
places it at its printed size; numbers are right-aligned, and `label`
translates the headers and the key cells. Group headings run across
their row; rows with a bar's sketch are as tall as it needs.

<a id="mod-cc_views-shadow"></a>

## `cc_views::shadow`

<sub>`crates/cc_views/src/shadow.rs`</sub>

The shadows a view's sun casts in projected views and plans, as exact
polygons (see `crate::projected::render`).
Every opaque face of the model that is not edge-on to the sun casts; a
face turned to the sun catches. The shadow on a face is the union of the
casters standing in front of its plane, slid along the sun's rays onto
that plane and kept within the face (`cc_geom::region::covered`); the
ground catches them the same way on a level plane. A closed convex
solid (a box, a round column) casts as one polygon, the hull of its
points.
Which faces catch them is the view's (`Catch`), as drawings are read:

- Elevations and 3D views: everything casts on everything shown.
- Floor plans read the site. The whole model the view shows casts, what
  stands above the cut plane too (upper storeys, roofs, eaves, the beams
  of a pergola), on every face seen from above and on the ground, as
  the sun does: under an eave, a porch or a pergola too. Only what is
  indoors catches nothing (`Indoors`): the part of a face, or of the
  ground, within a space indoors and under a floor, roof or ceiling, is
  taken out of its shadows (`region::covered_except`), so the rooms
  keep their plain shading. Ceiling plans draw no shadows.
- Sections: what the section cuts away (the model in front of its line)
  casts nothing, the rest casts as it is drawn, closed by its cut; and
  the faces indoors catch nothing, by the same rule, so the rooms cut
  show their plain shading while the fronts, roofs and ground seen
  beyond keep their shadows, an eave's on the wall under it among them.
Bounded: a grid over the casters seen from the sun finds those over a
face, and the faces that cast, the casters looked at, the points of one
union and the points of a whole view are limited; past a limit the view
gets no shadows at all.

**Items:** [`MAX_CASTERS`](#cc_views-shadow-max_casters) · [`MAX_UNION_POINTS`](#cc_views-shadow-max_union_points) · [`MAX_POINTS`](#cc_views-shadow-max_points) · [`MAX_LOOKS`](#cc_views-shadow-max_looks) · [`FACE_PROBE`](#cc_views-shadow-face_probe) · [`TEXTURES_AMBIENT`](#cc_views-shadow-textures_ambient) · [`Skip`](#cc_views-shadow-skip) · [`Shadows`](#cc_views-shadow-shadows) · [`Catch`](#cc_views-shadow-catch) · [`OPEN_AIR`](#cc_views-shadow-open_air) · [`Indoors`](#cc_views-shadow-indoors) · [`Storey`](#cc_views-shadow-storey) · [`point_within`](#cc_views-shadow-point_within) · [`textures_light`](#cc_views-shadow-textures_light) · [`veil`](#cc_views-shadow-veil) · [`ground_veil`](#cc_views-shadow-ground_veil) · [`Scene`](#cc_views-shadow-scene)

<a id="cc_views-shadow-max_casters"></a>

### const `MAX_CASTERS`

```rust
pub const MAX_CASTERS: usize
```

Most faces that cast in one view.

<a id="cc_views-shadow-max_union_points"></a>

### const `MAX_UNION_POINTS`

```rust
pub const MAX_UNION_POINTS: usize
```

Most points of the casters over one face (or the ground).

<a id="cc_views-shadow-max_points"></a>

### const `MAX_POINTS`

```rust
pub const MAX_POINTS: usize
```

Most points of the casters over all the faces of one view.

<a id="cc_views-shadow-max_looks"></a>

### const `MAX_LOOKS`

```rust
pub const MAX_LOOKS: usize
```

Most casters looked at over all the faces of one view.

<a id="cc_views-shadow-face_probe"></a>

### const `FACE_PROBE`

```rust
pub(crate) const FACE_PROBE: f64
```

A face is indoors when the point this far in front of it is, meters.

<a id="cc_views-shadow-textures_ambient"></a>

### const `TEXTURES_AMBIENT`

```rust
pub(crate) const TEXTURES_AMBIENT: f64
```

The light a face of the textures style keeps turned away from the sun
at the default shading (a face square to it is fully lit).

<a id="cc_views-shadow-skip"></a>

### struct `Skip`

```rust
pub struct Skip
```

The elements a view's graphics keep out of its shadows.

**Traits:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

**Fields**

- `hidden: BTreeSet<ElementId>`

  Hidden in the view (by category, filter, element, phase, design
  option or temporarily): they neither cast nor catch shadows.

- `clear: BTreeSet<ElementId>`

  Made see-through by the view's graphics: they cast none.


<a id="cc_views-shadow-shadows"></a>

### struct `Shadows`

```rust
pub struct Shadows<'a>
```

The shadows the sun of a view casts.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `ground: Option<f64>`

  The height of the level plane that catches them around the model
  (see `cc_core::Project::sun_ground_height`); `None`: no ground.

- `skip: Option<&'a Skip>`

  The elements left out of them.

- `catch: Catch<'a>`

  The faces that catch them.


<a id="cc_views-shadow-catch"></a>

### enum `Catch`

```rust
pub enum Catch<'a>
```

Which faces of a view catch its sun's shadows, and what casts them.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Default`

**Variants**

- `All`

  Every face shown; what the view cuts away casts nothing
  (elevations, 3D views).

- `Site(&'a Indoors)`

  A floor plan's reading of the site: the whole model shown casts,
  above the cut plane too, on every face and on the ground; what is
  indoors of them catches nothing.

- `Outdoors(&'a Indoors)`

  A section's: what it cuts away casts nothing, and the faces indoors
  catch nothing.


<a id="cc_views-shadow-open_air"></a>

### static `OPEN_AIR`

```rust
pub static OPEN_AIR: Indoors
```

No space indoors: everything catches the sun's shadows.

<a id="cc_views-shadow-indoors"></a>

### struct `Indoors`

```rust
pub struct Indoors
```

The spaces of a model that are indoors, storey by storey, for the
shadows of floor plans and sections.
A storey runs from a level up to the next one (the last one without
end), and its walls are those standing `STOREY_PROBE` above its level
(halfway up a lower storey). The spaces their outlines close, the walls'
own bodies among them (`RoomArrangement::spaces`: rooms, whether or
not a room was placed in them), are indoors when floors, roofs and
ceilings above cover more than `INDOORS_COVER` of them; a storey keeps
them united, as the outlines of what is indoors on it. So a room is
indoors; a courtyard, a walled garden with the eaves over its edge and a
terrace behind a parapet are not, nor is a porch or a garage that walls
do not close.
A point is indoors when it is in one of the spaces of the storey at its
height (`Indoors::holds`) and a floor, roof or ceiling of another
element lies above it (`Scene::above`, `Scene::covers`): the top of
a roof is not, whatever the storey it falls in. A section takes a face
whole, by the point `FACE_PROBE` in front of it, so the face of an
outer wall under an eave, which looks onto the open, keeps the eave's
shadow; a floor plan takes the part of each face seen from above that
is indoors, in the storey `PLAN_PROBE` above it. Spaces closed by
something other than walls (columns, components) are not found: their
faces catch shadows.

**Traits:** `Debug`, `Clone`, `Default`, `PartialEq`

**Fields**

- `storeys: Vec<Storey>`

  The storeys, from the lowest up.

- `failed: bool`

  A limit was passed finding them: the view draws no shadows.


**Methods**

- `pub fn of(project: &Project, geom: &ModelGeometry, skip: Option<&Skip>, detail: DetailLevel) -> Self`

  The spaces indoors of `geom` (the geometry a view shows, at its
  `detail` level, less what `skip` hides: hidden walls close nothing,
  hidden and see-through slabs cover nothing).

- `pub fn holds(&self, p: DVec3) -> bool`

  Whether `p` is in a space indoors of the storey at its height.

- `pub fn is_empty(&self) -> bool`

  Whether no space is indoors.


<a id="cc_views-shadow-storey"></a>

### struct `Storey`

```rust
pub struct Storey
```

A storey and its spaces indoors.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `z: (f64, f64)`

  From this height up to this one, meters (the second not included).

- `spaces: Vec<Vec<DVec2>>`

  The outlines of what is indoors on it, in plan: its spaces indoors
  united (rings that do not overlap, see `region::covered`).


<a id="cc_views-shadow-point_within"></a>

### fn `point_within`

```rust
pub(crate) fn point_within(verts: &[DVec3], normal: DVec3) -> Option<DVec3>
```

A point of the face `verts` (with unit `normal`): the mean of its
corners, or when that lies outside it (a face that is not convex), the
middle of its largest triangle.

<a id="cc_views-shadow-textures_light"></a>

### fn `textures_light`

```rust
pub(crate) fn textures_light(shading: &ShadingSettings, facing: f64) -> f64
```

The light of a face of the textures style at `facing` (the cosine of
its angle to the light): `TEXTURES_AMBIENT` turned away and full
square to it at the default shading; a view's ambient light moves the
first as it moves the shaded style's grey (up to full light at 1).

<a id="cc_views-shadow-veil"></a>

### fn `veil`

```rust
pub fn veil(style: VisualStyle, facing: f64, shading: &ShadingSettings) -> u8
```

The opacity (0 to 255) of the black veil that a cast shadow lays on a
face of `style` at `facing` (the cosine of its angle to the sun), so
that 2D views match the 3D view: in the shaded style the face shows
the shadow's grey (`ShadingSettings::shadow_grey`) under it, in the
textures style it loses the sun's light and darkens as much as the
shaded style's shadow does from its ambient light, in consistent colors
it darkens as the 3D view's unlit faces do. The shadow fades out where
the face turns edge-on to the sun, as in the 3D view. No shadows in the
other styles.

<a id="cc_views-shadow-ground_veil"></a>

### fn `ground_veil`

```rust
pub fn ground_veil(shading: &ShadingSettings) -> u8
```

The opacity (0 to 255) of the black veil of a shadow on the ground:
`GROUND_SHADOW` at the default shading, none to black with the view's
shadow intensity.

<a id="cc_views-shadow-scene"></a>

### struct `Scene`

```rust
pub(crate) struct Scene<'a>
```

What casts shadows in a view, seen from its sun.

*Some fields are private.*

**Methods**

- `pub fn new(geom: &'a ModelGeometry, sun: DVec3, skip: Option<&Skip>, shown: &dyn Fn(&Solid) -> bool, cut: Option<(DVec3, DVec3)>) -> Option<Self>`

  The casters of `geom` under a sun towards unit `sun`: the faces of
  the solids `shown` that `skip` leaves in, opaque and not edge-on to
  the sun, those of a closed convex solid together as one unit; with
  a `cut` plane (a point and the direction kept), only what lies
  beyond it, closed by the cut's caps. `None` when the sun is not a
  finite direction or more than `MAX_CASTERS` faces cast.

- `pub fn covers(geom: &'a ModelGeometry, skip: Option<&Skip>, shown: &dyn Fn(&Solid) -> bool, cut: Option<(DVec3, DVec3)>) -> Option<Self>`

  What covers the faces of `geom` from the sky: the floors, roofs and
  ceilings among the solids `shown` that `skip` leaves in, opaque, as
  the casters of a sun straight overhead. What they lay on a face is
  the part of it that lies under them, and they stand "towards the
  sun" of the points they lie above (see `Scene::above`).

- `pub fn failed(&self) -> bool`

  Whether a limit was passed: the view then draws no shadows.

- `pub fn above(&self, p: DVec3, apart_from: Option<ElementId>) -> bool`

  Whether a caster stands over the point `p`, towards the sun (for
  `Scene::covers`: whether a floor, roof or ceiling lies above it),
  leaving out everything of the element `apart_from`.

- `pub fn on_face(&mut self, verts: &[DVec3], normal: DVec3, own: (usize, usize), indoors: Option<(&mut Scene<'_>, &Indoors, ElementId)>) -> Vec<Vec<DVec3>>`

  The shadows cast on the face `own` (its solid and face in the
  geometry) with outline `verts` and unit `normal`: rings in its
  plane (see `region::covered`). None on a face turned away from the
  sun or edge-on to it.
  With what is `indoors` (a floor plan's faces, seen from above): the
  scene of what covers from the sky (`Scene::covers`), the spaces
  indoors and the face's element, the part of the face that is
  indoors catches nothing: it is taken out of its shadows. A limit
  passed by the cover is passed by this scene too.

- `pub fn on_ground(&mut self, z: f64, bounds: (DVec3, DVec3), indoors: Option<(&mut Scene<'_>, &Indoors)>) -> Vec<Vec<DVec3>>`

  The shadows cast on the level plane at height `z` around a model
  within `bounds`: rings at that height, no farther from the model
  than the shadows of a sun `MIN_GROUND_ALTITUDE_DEG` up reach.
  With what is `indoors` (the scene of what covers from the sky,
  `Scene::covers`, and the spaces indoors), none where the plane is
  indoors: within a space and under a floor, roof or ceiling.


<a id="mod-cc_views-sheet"></a>

## `cc_views::sheet`

<sub>`crates/cc_views/src/sheet.rs`</sub>

Sheet composition: the title block and the placed viewports, as one
drawing in paper millimeters (origin at the bottom-left corner). The
title block is a component type (`ComponentKind::TitleBlock`) whose 2D
pieces draw the border and the block; its texts' `{fields}` are filled
in from the sheet and the project. The application paints the result
and the exporters write it, so the screen and the PDF/SVG files match.

**Items:** [`MARGIN`](#cc_views-sheet-margin) · [`SheetEnv`](#cc_views-sheet-sheetenv) · [`PlacedViewport`](#cc_views-sheet-placedviewport) · [`Sheet`](#cc_views-sheet-sheet) · [`compose`](#cc_views-sheet-compose) · [`title_block_drawing`](#cc_views-sheet-title_block_drawing)

<a id="cc_views-sheet-margin"></a>

### const `MARGIN`

```rust
pub const MARGIN: f64
```

The margin of the standard title block's border, millimeters.

<a id="cc_views-sheet-sheetenv"></a>

### struct `SheetEnv`

```rust
pub struct SheetEnv
```

What a sheet's fields read besides the model.

**Traits:** `Debug`, `Clone`

**Fields**

- `today: (i32, u32, u32)`

  Today's date (year, month, day), for `{date}` without an issue date.

- `format_date: fn(i32, u32, u32) -> String`

  Writes a date the way the interface language does.

- `as_indicated: String`

  "As indicated", translated (`{sheet_scale}` with several scales).


<a id="cc_views-sheet-placedviewport"></a>

### struct `PlacedViewport`

```rust
pub struct PlacedViewport
```

A viewport resolved on the sheet (for selection and dragging).

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `index: usize`
- `view: ViewId`
- `center: DVec2`

  Center and half extents of the drawing on the sheet, in millimeters.

- `half: DVec2`

<a id="cc_views-sheet-sheet"></a>

### struct `Sheet`

```rust
pub struct Sheet
```

A composed sheet.

**Traits:** `Debug`, `Clone`, `Default`

**Fields**

- `paper: DVec2`
- `drawing: Drawing2D`
- `viewports: Vec<PlacedViewport>`

<a id="cc_views-sheet-compose"></a>

### fn `compose`

```rust
pub fn compose(project: &Project, sheet: &View, env: &SheetEnv, mut drawing_of: impl FnMut(ViewId) -> Option<Rc<Drawing2D>>) -> Option<Sheet>
```

Compose `sheet`; `drawing_of` returns the drawing of a placed view.

<a id="cc_views-sheet-title_block_drawing"></a>

### fn `title_block_drawing`

```rust
pub fn title_block_drawing(pieces: &SheetPieces, expand: &dyn Fn(&str) -> String) -> Drawing2D
```

The drawing of a title block's resolved pieces (paper millimeters),
each text passed through `expand` (the editor fills fields or shows them raw).

<a id="mod-cc_views-sheet_fields"></a>

## `cc_views::sheet_fields`

<sub>`crates/cc_views/src/sheet_fields.rs`</sub>

`{fields}` in title block texts: sheet, project and date values, and
the title block's per-sheet parameters. Unknown keys stay as written.

**Items:** [`KEYS`](#cc_views-sheet_fields-keys) · [`FieldCtx`](#cc_views-sheet_fields-fieldctx) · [`expand`](#cc_views-sheet_fields-expand) · [`expand_with`](#cc_views-sheet_fields-expand_with) · [`value`](#cc_views-sheet_fields-value)

<a id="cc_views-sheet_fields-keys"></a>

### const `KEYS`

```rust
pub const KEYS: [&str; 12]
```

The fixed keys, in the order the editor's menu lists them.

<a id="cc_views-sheet_fields-fieldctx"></a>

### struct `FieldCtx`

```rust
pub struct FieldCtx<'a>
```

What the fields read from.

**Fields**

- `project: &'a Project`
- `sheet: &'a View`
- `def: Option<&'a ComponentDef>`

  The title block definition (for `param:` defaults), if any.

- `today: (i32, u32, u32)`
- `format_date: fn(i32, u32, u32) -> String`
- `as_indicated: &'a str`

  "As indicated", translated.

- `sheets: OnceCell<Vec<ViewId>>`

  The project's sheets in order, worked out on first use (`Default::default()`).


<a id="cc_views-sheet_fields-expand"></a>

### fn `expand`

```rust
pub fn expand(text: &str, ctx: &FieldCtx) -> String
```

Replaces each `{key}` of `text` with its value; `{{` and `}}` are literal
braces, and unknown keys and braces without a match stay as written
(see `field_parts`).

<a id="cc_views-sheet_fields-expand_with"></a>

### fn `expand_with`

```rust
pub fn expand_with(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String
```

`expand` with any lookup (the title block editor's sample values).

<a id="cc_views-sheet_fields-value"></a>

### fn `value`

```rust
pub fn value(key: &str, ctx: &FieldCtx) -> Option<String>
```

The value of one field, or `None` for an unknown key.

<a id="mod-cc_views-sun_path"></a>

## `cc_views::sun_path`

<sub>`crates/cc_views/src/sun_path.rs`</sub>

The sun path diagram of a view with a sun (`cc_core::View::sun` with
`show_path`): a compass ring turned to true north on the ground, the
sky dome's path of the sun over the day shown (and, over several days,
over the first and last), its whole hours, sunrise, sunset and the date,
and the sun where it stands.
`diagram` builds it in world coordinates, view-independent and purely
geometric, so it can be drawn over any projection (2D views here, the
live 3D view by the app); `draw` lays it over a view's drawing as an
overlay (`Drawing2D::overlay`): no element, not picked or snapped to,
not cropped, but fitted.

**Items:** [`MIN_RADIUS`](#cc_views-sun_path-min_radius) · [`MAX_RADIUS`](#cc_views-sun_path-max_radius) · [`MAX_POINTS`](#cc_views-sun_path-max_points) · [`SunLineKind`](#cc_views-sun_path-sunlinekind) · [`SunLine`](#cc_views-sun_path-sunline) · [`SunText`](#cc_views-sun_path-suntext) · [`SunLabelKind`](#cc_views-sun_path-sunlabelkind) · [`SunLabel`](#cc_views-sun_path-sunlabel) · [`SunDiagram`](#cc_views-sun_path-sundiagram) · [`diagram`](#cc_views-sun_path-diagram) · [`GRID_COLOR`](#cc_views-sun_path-grid_color) · [`GRID_WEIGHT`](#cc_views-sun_path-grid_weight) · [`BAND_COLOR`](#cc_views-sun_path-band_color) · [`PATH_COLOR`](#cc_views-sun_path-path_color) · [`PATH_WEIGHT`](#cc_views-sun_path-path_weight) · [`RANGE_WEIGHT`](#cc_views-sun_path-range_weight) · [`DOT_RADIUS_MM`](#cc_views-sun_path-dot_radius_mm) · [`SUN_FILL`](#cc_views-sun_path-sun_fill) · [`SUN_OUTLINE`](#cc_views-sun_path-sun_outline) · [`SUN_OUTLINE_WEIGHT`](#cc_views-sun_path-sun_outline_weight) · [`MIN_SUN_RADIUS_MM`](#cc_views-sun_path-min_sun_radius_mm) · [`CARDINAL_SIZE`](#cc_views-sun_path-cardinal_size) · [`TIME_SIZE`](#cc_views-sun_path-time_size) · [`DATE_SIZE`](#cc_views-sun_path-date_size) · [`LABEL_GAP_MM`](#cc_views-sun_path-label_gap_mm) · [`label_anchor`](#cc_views-sun_path-label_anchor) · [`on_horizon`](#cc_views-sun_path-on_horizon) · [`stacked_on`](#cc_views-sun_path-stacked_on) · [`half_extent`](#cc_views-sun_path-half_extent) · [`text_width`](#cc_views-sun_path-text_width) · [`time_text`](#cc_views-sun_path-time_text) · [`date_text`](#cc_views-sun_path-date_text) · [`label_text`](#cc_views-sun_path-label_text) · [`draw`](#cc_views-sun_path-draw)

<a id="cc_views-sun_path-min_radius"></a>

### const `MIN_RADIUS`

```rust
pub const MIN_RADIUS: f64
```

Smallest and largest radius of the diagram, meters.

<a id="cc_views-sun_path-max_radius"></a>

### const `MAX_RADIUS`

```rust
pub const MAX_RADIUS: f64
```

<a id="cc_views-sun_path-max_points"></a>

### const `MAX_POINTS`

```rust
pub const MAX_POINTS: usize
```

Most points a diagram holds (lines, dots and labels together).

<a id="cc_views-sun_path-sunlinekind"></a>

### enum `SunLineKind`

```rust
pub enum SunLineKind
```

What a line of the diagram is.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

**Variants**

- `Ring`

  A circle of the compass ring (the inner at the radius, then the outer).

- `Tick`

  A tick of the compass every 15 degrees.

- `Axis`

  The north-south and east-west axes.

- `Path`

  The sun's path over the day shown.

- `PathRange`

  The path over the first or last day of a study over several days.


<a id="cc_views-sun_path-sunline"></a>

### struct `SunLine`

```rust
pub struct SunLine
```

A polyline of the diagram, world coordinates.

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `pts: Vec<DVec3>`
- `closed: bool`
- `kind: SunLineKind`

<a id="cc_views-sun_path-suntext"></a>

### enum `SunText`

```rust
pub enum SunText
```

What a label says, to be written in the reader's language.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Variants**

- `Key(&'static str)`

  A translation key (the cardinal points).

- `Time(f64)`

  A time of day, minutes after local midnight.

- `Date(SunDate)`

  A date.


<a id="cc_views-sun_path-sunlabelkind"></a>

### enum `SunLabelKind`

```rust
pub enum SunLabelKind
```

What a label marks.

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

**Variants**

- `Cardinal`
- `Sunrise`
- `Sunset`
- `Date`
- `Now`

  The time of the sun shown.


<a id="cc_views-sun_path-sunlabel"></a>

### struct `SunLabel`

```rust
pub struct SunLabel
```

A label of the diagram at `pos`, pushed away from it along `out` (a
unit direction in world coordinates, or zero).

**Traits:** `Debug`, `Clone`, `Copy`, `PartialEq`

**Fields**

- `pos: DVec3`
- `text: SunText`
- `kind: SunLabelKind`
- `out: DVec3`

<a id="cc_views-sun_path-sundiagram"></a>

### struct `SunDiagram`

```rust
pub struct SunDiagram
```

A sun path diagram in world coordinates (see the module's documentation).

**Traits:** `Debug`, `Clone`, `PartialEq`

**Fields**

- `center: DVec3`

  The center of the compass, on the ground plane.

- `radius: f64`

  The radius of the compass ring and of the sky dome.

- `lines: Vec<SunLine>`
- `dots: Vec<DVec3>`

  Whole hours on the day's path, and the frames of a study on that day.

- `labels: Vec<SunLabel>`
- `sun: Option<DVec3>`

  The sun's place on the dome (`None` below the horizon).

- `sun_radius: f64`

  The radius of the sun's disc, meters.


**Methods**

- `pub fn point_count(&self) -> usize`

  How many points it holds (lines, dots, labels and the sun).


<a id="cc_views-sun_path-diagram"></a>

### fn `diagram`

```rust
pub fn diagram(project: &Project, geom: &ModelGeometry, view: &View) -> Option<SunDiagram>
```

The sun path diagram of `view`: `None` unless it takes a sun, has one and
shows its path. Centered on the plan center of the model's extents in
`geom` (the geometry the view shows; the origin for an empty model), on
the ground level's plane (else the bottom of the extents), with a radius
of three quarters of the extents' plan diagonal within
`MIN_RADIUS`..=`MAX_RADIUS`. Every point is finite, and there are at
most `MAX_POINTS`.

<a id="cc_views-sun_path-grid_color"></a>

### const `GRID_COLOR`

```rust
pub const GRID_COLOR: [u8; 3]
```

Lines of the compass: neutral grey-blue (RGB; weights are pen weights, mm).

<a id="cc_views-sun_path-grid_weight"></a>

### const `GRID_WEIGHT`

```rust
pub const GRID_WEIGHT: f64
```

<a id="cc_views-sun_path-band_color"></a>

### const `BAND_COLOR`

```rust
pub const BAND_COLOR: [u8; 4]
```

The band between the two circles of the ring (RGBA).

<a id="cc_views-sun_path-path_color"></a>

### const `PATH_COLOR`

```rust
pub const PATH_COLOR: [u8; 3]
```

The sun's path: amber.

<a id="cc_views-sun_path-path_weight"></a>

### const `PATH_WEIGHT`

```rust
pub const PATH_WEIGHT: f64
```

<a id="cc_views-sun_path-range_weight"></a>

### const `RANGE_WEIGHT`

```rust
pub const RANGE_WEIGHT: f64
```

<a id="cc_views-sun_path-dot_radius_mm"></a>

### const `DOT_RADIUS_MM`

```rust
pub const DOT_RADIUS_MM: f64
```

Radius of an hour's dot on paper, mm.

<a id="cc_views-sun_path-sun_fill"></a>

### const `SUN_FILL`

```rust
pub const SUN_FILL: [u8; 4]
```

The sun's disc (RGBA) and its outline.

<a id="cc_views-sun_path-sun_outline"></a>

### const `SUN_OUTLINE`

```rust
pub const SUN_OUTLINE: [u8; 3]
```

<a id="cc_views-sun_path-sun_outline_weight"></a>

### const `SUN_OUTLINE_WEIGHT`

```rust
pub const SUN_OUTLINE_WEIGHT: f64
```

<a id="cc_views-sun_path-min_sun_radius_mm"></a>

### const `MIN_SUN_RADIUS_MM`

```rust
pub const MIN_SUN_RADIUS_MM: f64
```

Smallest radius of the sun's disc on paper, mm.

<a id="cc_views-sun_path-cardinal_size"></a>

### const `CARDINAL_SIZE`

```rust
pub const CARDINAL_SIZE: f64
```

Text heights on paper, mm.

<a id="cc_views-sun_path-time_size"></a>

### const `TIME_SIZE`

```rust
pub const TIME_SIZE: f64
```

<a id="cc_views-sun_path-date_size"></a>

### const `DATE_SIZE`

```rust
pub const DATE_SIZE: f64
```

<a id="cc_views-sun_path-label_gap_mm"></a>

### const `LABEL_GAP_MM`

```rust
pub const LABEL_GAP_MM: f64
```

Gap between a label and its point on paper, mm.

<a id="cc_views-sun_path-label_anchor"></a>

### fn `label_anchor`

```rust
pub fn label_anchor(d: &SunDiagram, l: &SunLabel) -> DVec3
```

The point a label is laid out from, world coordinates: a label on the
horizon (see `on_horizon`) stands past the compass's ticks along its
`out`, clear of the ring; the others at their point.

<a id="cc_views-sun_path-on_horizon"></a>

### fn `on_horizon`

```rust
pub fn on_horizon(d: &SunDiagram, l: &SunLabel) -> bool
```

Whether `l` marks a point on the horizon: a sunrise, a sunset, or a
date at the sunrise (a polar day's date stands on the path, above it).

<a id="cc_views-sun_path-stacked_on"></a>

### fn `stacked_on`

```rust
pub fn stacked_on(d: &SunDiagram, index: usize) -> Option<usize>
```

The label that the label at `index` stacks beyond, further from the
ring: a date on the sunrise's time at the same point.

<a id="cc_views-sun_path-half_extent"></a>

### fn `half_extent`

```rust
pub fn half_extent(dir: DVec2, width: f64, height: f64) -> f64
```

Half the extent along the unit direction `dir` of a text box `width`
wide and `height` tall (axis aligned).

<a id="cc_views-sun_path-text_width"></a>

### fn `text_width`

```rust
pub fn text_width(text: &str, size: f64) -> f64
```

The approximate width of `text` written `size` tall (wide characters,
such as CJK ones, a full size; others six tenths of it).

<a id="cc_views-sun_path-time_text"></a>

### fn `time_text`

```rust
pub fn time_text(minutes: f64) -> String
```

A time of day as `HH:MM` (minutes after midnight, any day).

<a id="cc_views-sun_path-date_text"></a>

### fn `date_text`

```rust
pub fn date_text(date: SunDate, label: &dyn Fn(&str) -> String) -> String
```

`date` in the reader's language: the pattern `label` gives for
`DATE_PATTERN_KEY`, else ISO `YYYY-MM-DD`.

<a id="cc_views-sun_path-label_text"></a>

### fn `label_text`

```rust
pub fn label_text(text: SunText, label: &dyn Fn(&str) -> String) -> String
```

The text of a label in the reader's language.

<a id="cc_views-sun_path-draw"></a>

### fn `draw`

```rust
pub fn draw(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, view: &View, label: &dyn Fn(&str) -> String)
```

Lay `view`'s sun path diagram over `d` (nothing when it has none), last
and counted in `Drawing2D::overlay`: plans in plan coordinates, the
other views through their frame (`crate::projected::view_frame`),
with no hidden lines removed and no element.

<a id="mod-cc_views-topography"></a>

## `cc_views::topography`

<sub>`crates/cc_views/src/topography.rs`</sub>

Terrain in floor plans: contour lines (majors heavier) and the boundary
below the cut plane; where the cut plane crosses the surface, the earth
above it hatched in the type's material and outlined as cut.
Building pads: their outline with their elevation, ticks across their
slopes (long and short in turn, the short ones from the top of the
slope), the daylight line, and the existing ground's contours dashed
where the pads changed it.

**Items:** [`draw`](#cc_views-topography-draw)

<a id="cc_views-topography-draw"></a>

### fn `draw`

```rust
pub(crate) fn draw(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, level: LevelId, cut_z: f64, scale: u32)
```

The terrain surfaces of `level` in its floor plan cut at `cut_z`.

{% endraw %}
