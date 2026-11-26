# Automation: engine, command line and MCP

[Português](automacao.md)

Everything ConstructCraft does to a model can be done without the interface. The **engine** (`cc_engine`) holds the operations as a catalog of named **commands**; three clients use that one catalog:

```
                 ┌──────────────────────────────┐
  desktop app ──▶│                              │
  ccad (CLI)  ──▶│  cc_engine: Session +        │──▶ the model (.ccad)
  MCP agents  ──▶│  catalog of commands         │
  Lua scripts ──▶│  (+ plugin commands)         │
                 └──────────────────────────────┘
```

A wall drawn with the mouse, a `ccad run wall_create …`, a Lua script calling `ccad.run("wall_create", …)` and an AI agent calling the `wall_create` tool run the same code, with the same defaults, checks and undo. The full list of commands with their parameters is in the [command reference](commands.md); [plugins](#plugins) add their own commands to the catalog.

## Concepts

**Session.** One open project: the document (with its undo history) plus geometry and drawings derived from it. The CLI opens one per invocation (or script), the MCP server one per connection, the app one per window.

**Commands.** Each command has a `snake_case` name, a JSON object of parameters described by a JSON Schema, and a JSON result. Kinds:

| Kind | Does | Examples |
|---|---|---|
| query | reads the model | `elements_list`, `element_get`, `schedule_table` |
| edit | changes the model: one undo step | `wall_create`, `elements_move`, `element_set` |
| file | works with files or replaces the project | `project_open`, `ifc_import`, `sheet_export` |

**Units and coordinates.** Lengths are meters, angles degrees, points `[x, y]` in plan. Dimensions use the coordinates of their view's drawing (plan X/Y in plans; horizontal position and height in elevations and sections). Areas and volumes in results are m² and m³.

**References.** Levels, types, views and materials are given by **id or name** (`"level": 3` or `"level": "Ground"`; names are matched without regard to case; an ambiguous name is an error asking for the id). Sheets can also be named by their number (`"A101"`). Elements are given by id; `elements_list` and the results of creating commands give them. Omitted levels default to the lowest level and omitted types to the category's default type.

**Reading and changing parameters.** `element_get`, `type_get` and `view_get` show an object's `params`; `element_set`, `type_set` and `view_set` take the fields to change in the same shape and merge them in:

```sh
ccad -p house.ccad run element_get id=42
ccad -p house.ccad --save run element_set id=42 'params={"height": 2.7}'
ccad -p house.ccad --save run element_set id=57 'params={"shape": {"Gable": {"slope": 0.5, "ridge_deg": 0}}}'
```

Objects merge field by field; arrays and plain values replace; `null` clears an optional value. `material_set`, `units_set` (shaped like `material_get`, `units_get`) work alike.

**View graphics.** What a view shows and how is part of the view: `view_graphics_get` lists every category's visibility and overrides, the applied filters and the element overrides; `view_graphics_set` shows or hides a category and overrides its projection and cut lines (color, weight in paper mm, pattern), surface and cut fills (color, pattern), halftone and transparency; `element_override_set` and `element_hide_in_view` do the same for single elements in one view. Overrides are objects like `{"cut_lines": {"color": [255, 0, 0], "weight": 0.5, "pattern": "Dash"}, "cut_fill": {"pattern": "Solid", "color": [200, 200, 200]}, "halftone": true, "transparency": 0.4}`; they merge like `params` (`null` resets an attribute, `reset: true` starts over). They apply to plans, ceiling plans, elevations, sections, 3D views, sheets and PDF/SVG exports.

```sh
ccad -p house.ccad --save run view_graphics_set view=Térreo category=door visible=false
ccad -p house.ccad --save run filter_create name=Long 'categories=["wall"]' 'rules=[{"param": "length", "op": "greater", "value": 5}]'
ccad -p house.ccad --save run view_filter_add view=Térreo filter=Long 'overrides={"cut_lines": {"color": [255, 0, 0]}}'
ccad -p house.ccad run filter_elements filter=Long
```

*Filters* (`filter_create`, `filter_set`, `filter_delete`, `filters_list`, `filter_elements`) select elements by category and rules on parameters — any schedule field (`type`, `mark`, `comments`, `level`, `material`, `system`, `structural`, `width`, `height`, `thickness`, `length`, `area`, `volume`, `diameter`…) or `elevation` (of the element's level) — with `equals`, `not_equals`, `contains`, `not_contains`, `begins_with`, `not_begins_with`, `greater`, `greater_or_equal`, `less`, `less_or_equal`, `has_value` and `has_no_value`, all of them (default) or any (`any: true`). A rule on a field that none of the filter's categories has (`width` on walls) is refused, since it could never pass. Quantities are compared in SI units (meters), text without case. A view applies filters in order (`view_filter_add`, `view_filter_set`, `view_filter_move`, `view_filter_remove`): the first matching filter wins for each attribute, an element's own overrides beat the filters, and filters beat the category; an element is hidden when it is hidden in the view, its category is off, or any matching filter hides it.

*View templates* (`template_create` from a view, `template_set`, `template_delete`, `templates_list`) hold a scale, category graphics, filters, the plan cut height, the building services display (services over a faint architecture) and the structural display, each with an `include` flag. `view_template_apply` copies them into views once; `view_template_assign` makes views follow a template: the included properties are copied now and on every change of the template (in the same undo step), and editing them in the view is refused. Deleting a template frees its views, which keep their properties.

*Temporary hide/isolate* (`view_temporary` with `action` `hide`, `isolate`, `hide_category`, `isolate_category` or `reset`) is display state of the session, not a model change: it is not saved, not undone and not printed (sheets and exports show the view whole). `view_temporary_apply` makes it permanent (hidden in the view, categories turned off) as one undo step.

**Construction phases.** A renovation is modeled in phases (`phases_list`; the template has *Existing* and *New Construction*): every model element is built in a phase (`phase_created`) and may be demolished in the same or a later one (`phase_demolished`), as `element_get` shows. `phase_create` (at the end or `before` another), `phase_set` (rename, or move with `index`; a move that would demolish elements before building them is refused) and `phase_delete` (a phase in use needs `merge_into`) manage them. New elements are built in the session's creation phase (`phase_creation_set`; the active view's in the app, else the last phase), copies too. `elements_phase_set` sets `created` and `demolished` (`null`: standing) and `elements_demolish` demolishes in a `phase` or the phase of a `view`. Doors, windows and rebar follow their hosts: they are never older than their host and are demolished with it unless demolished earlier. Dimensions, text and grids have no phase.

```sh
ccad -p house.ccad --save run elements_phase_set 'ids=[12, 13]' created=Existing
ccad -p house.ccad --save run elements_demolish 'ids=[13]' view=Térreo
ccad -p house.ccad --save run view_set view=Térreo 'phase_filter=Show Previous + Demo'
ccad -p house.ccad run phases_list
```

Each plan, elevation, section, 3D view and schedule shows the model in its `phase` with a `phase_filter` (`view_set`; view templates control the filter, not the phase). Relative to the view's phase an element is *existing* (built earlier, standing), *demolished* (in this phase), *new* (built in this phase) or *temporary* (built and demolished in it); elements built later or demolished earlier are not shown. Phase filters (`phase_filters_list`, `phase_filter_create`, `phase_filter_set`, `phase_filter_delete`; the template has *Show All*, *Show Complete*, *Show New*, *Show Previous + Demo*, *Show Previous + New*, *Show Previous Phase* and *Show Demo + New*) map each status to `not_displayed`, `by_category` or `overridden`, which uses the project's graphics of the status (`phase_graphics_get`, `phase_graphics_set`: existing faint, demolished dashed without cut pattern, temporary faint and dashed), below element overrides and view filters and above the category. A view whose phase leaves elements out gets geometry of its own: doors of later phases do not open existing walls and walls join only with walls of the phase. A room is bounded by the walls standing at the end of the phase it is created in, so walls demolished in that phase or earlier do not bound it. Schedules list what their phase and filter show and offer the fields `PhaseCreated` and `PhaseDemolished` (also usable in filter rules as `phase_created`, `phase_demolished`).

**Project browser organization.** The project browser groups the views by type alone unless a scheme is in use. `browser_scheme_create` makes a named scheme (`group_by`: the criteria from the outermost group in, among `view_type`, `level`, `discipline` and `phase`; default `["view_type", "level"]`) and puts it in use unless `use_it` is false; `browser_scheme_set` renames it or changes its criteria, `browser_scheme_delete` removes it, and `browser_scheme_use` picks the scheme the browser follows (no `scheme`: the default). `browser_schemes_list` lists them, and `browser_tree` returns the tree the browser shows — group rows with `group`, `name`, `value`, `views` and `children`, view rows with `view` and `name` — for the scheme in use, another `scheme`, or any `group_by`. A view without a value for a criterion (a section has no level) stays beside that criterion's groups.

**Design options.** Alternatives for part of the model live in option sets (`options_list`): `option_set_create` makes a set with named `options` (the first is primary), `option_create`, `option_set` (rename, `primary: true`, move with `index`), `option_delete` (not the primary; it deletes the option's elements), `option_set_rename` and `option_set_delete` (with every element of its options) manage them. An element belongs to the main model or to one option (`element_get` `design_option`); `elements_option_set` moves elements into an option or back with `null` (doors, windows and other hosted elements follow their host; annotations have no option). Each plan, elevation, section, 3D view and schedule shows the main model plus one option per set: `view_options_set` chooses one, or `null` for automatic (`view_get` `design_options` shows the choice and what is shown); view templates can control it (`template_set` `design_options`, `include.design_options`). Automatic means the option being edited when it is in that set (`option_active_set`, session state, not saved: new model elements go into it), else the primary. Elements of options a view does not show are left out of its drawing, schedule and geometry: walls join and rooms are bounded only among what is shown together. `option_accept_primary` moves the primary option's elements into the main model and deletes the set with its other options and their elements. IFC export writes the main model and the primary options only, each option element with `ConstructCAD_DesignOption` (`OptionSet`, `Option`, `Primary`), which import reads back as sets and options.

```sh
ccad -p house.ccad --save run option_set_create name=Entrance 'options=["Porch", "Canopy"]'
ccad -p house.ccad --save run elements_option_set 'ids=[31]' option="Entrance: Canopy"
ccad -p house.ccad --save run view_options_set view=Térreo set=Entrance option=Canopy
```

**Scales.** A view's `scale` is the denominator `N` of 1:`N`. `view_create`, `view_set`, `template_set` and `callout_create` also take it as text: `"1:50"`, or in imperial projects a label such as `"1/4\" = 1'-0\""` (48) or `"1\" = 20'"` (240). `view_get` adds `scale_text`, the scale as the project writes it (`1:50`, or `1/4" = 1'-0"` in imperial projects), and an imperial callout's default detail scale is an imperial one.

**Measurement systems.** Command parameters and results are SI whatever the project's units (meters, square and cubic meters, degrees, kilograms; slopes in percent where a command says so), and so are files, IFC and DXF; `project_info` reports the `system`. What a command writes as text follows the project: in an imperial project the types it makes on demand are named and sized in round inches (`Ramp 4'-0"`, `Baseboard 4"`, `Cable tray 8"×4"`, `Conductor Ø3/8"`, door types `2'-8" x 6'-8"`, profiles `I 8"x8"x1/4"x3/8"`; defaults such as wall and story heights, pipe and device elevations, drain falls, rebar cover and terrain contours are imperial too, as each parameter's description says), check messages (`stair_check`, `ramp_check`) write inches and slopes per 12, and schedules (`schedule_table`, `schedule_export_csv`) write `lb` and `lb/ft` for weights, inches for thicknesses, diameters and risers, and headers without metric units. Conductor cross-sections stay in mm².

**Drafting views.** `view_create` with `kind: "drafting"` makes a 2D view independent of the model (category `drafting`, 1:10 unless `scale` says otherwise): it draws no model geometry, only the elements placed in it — `text_create`, `dimension_create` (and the chain and ordinate forms), `cad_import` and `image_import` take it as their `view`. `view_get` shows its `scale` and `detail`, `view_set` changes them (it has no phase or visual style), and `view_graphics_set` hides or overrides its categories. It goes on sheets with `sheet_place_view` and exports with `view_export` (PNG, SVG, PDF, DXF) like other views; `view_delete` deletes what was drawn in it.

**Callouts and detail views.** `callout_create` draws a callout in a floor plan, ceiling plan, section or elevation (`parent`): `rect` is two opposite corners in the parent's drawing coordinates (meters; `y` is the height in sections and elevations). It makes a detail view of that region (named `Detail N` unless `name` says otherwise, at `scale`, by default a standard scale about five times larger than the parent's: 1:10 under 1:50) of the parent's kind, so it stays live with the model: it follows the parent's level, section line or direction, shows the model as the parent does (phase, detail level, visual style, graphics) and is cropped to the rectangle, and its own detail lines, regions, detail components, texts and dimensions are drawn whole. With `drafting`, the callout points to that existing drafting view instead (no crop; one callout per drafting view). The parent draws the callout as a rectangle with rounded corners and a tag: the detail's number on its sheet over the sheet's number once it is placed, else the last word of its name over a dash. `view_get` shows `crop` (`[[min x, min y], [max x, max y]]`) and `callout` (`parent`, `parent_name`, `rect`, `drafting`); `view_set` `crop` moves or resizes a detail view's callout (they are one rectangle), crops any plan, section or elevation, or with `null` shows everything again (a callout needs its rectangle); for a drafting view a callout points to, it moves that callout. `callouts_list` lists the callouts drawn in a view, `callout_remove` removes a callout and keeps its view, `view_delete` of a detail view removes its callout, and deleting the parent leaves its details as ordinary cropped views. Plans do not draw a section's or elevation's detail view as another section or elevation mark. Files whose callouts point to missing views, or whose crops are not valid rectangles, are repaired on load.

```sh
ccad -p house.ccad --save run callout_create parent="Ground Floor" rect=[[2,3],[5,6]] scale=20
```

**Sun.** Plans, ceiling plans, 3D views, elevations and sections may have a sun: `view_set` `sun` takes the fields to change (`null` removes it) and `view_get` shows them with `derived`. `mode` is `still` (`date`, `time`), `single_day` (`date`, `from`, `to`, `sunrise_to_sunset`, `interval` of 15, 30 or 45 minutes or an hour), `multi_day` (`from_date`, `to_date`, `from`, `to`, `sunrise_to_sunset`, `interval` also `day`, `week` or `month`) or `lighting` (`azimuth`, `altitude`, `relative_to_view`), and `conventional` sets the façade drawing's sun, a `lighting` relative to the view at azimuth 225 and altitude atan(1/√2) ≈ 35.26, whose rays run along a cube's diagonal from over the viewer's left shoulder so a shadow falls down and to the right as far as its caster stands out (`view_get` shows it as `lighting` with `derived.conventional`); every mode has `frame` (the study frame shown), `show_path`, `shadows` and `ground` (the level receiving the shadows). Shadows are drawn in 3D views and, in the `shaded`, `consistent_colors` and `textures` visual styles, in plans, elevations and sections, so `view_export` and sheets carry them (a floor plan casts them from the whole model, a section from what lies beyond its line, neither onto what is indoors; ceiling plans draw none). Dates are `YYYY-MM-DD` and times `HH:MM`, local to the site; another mode starts from its defaults and keeps the date, a field the mode does not have is refused. `derived` gives the azimuth (clockwise from true north) and altitude of the current frame, its date and time, sunrise and sunset, the number of frames and whether the project has a site location (else the equator at Greenwich). `sun_position` answers for a view or for any `date` and `time` (optionally at another `latitude`, `longitude` and `utc_offset`), `sun_frames` lists a study's frames. `view_set` `shading` patches how those views shade faces and cast shadows in the shaded, consistent colors and textures styles (2D and 3D, not `view_render`): `ambient`, the light of faces turned away from the light (0.3 to 1, default 0.6), and `shadow`, how dark cast shadows are (0 none to 1 black, default 0.55; a shaded shadow's grey is `1 - shadow`); `view_get` shows it (`null` for views that take no sun). Named presets keep a mode: `sun_preset_save` (from a `view` or a `sun`; `replace` to overwrite), `sun_presets_list`, `sun_preset_rename`, `sun_preset_delete`, and a view takes one with `"sun": {"preset": "Summer"}`. `site_set` `utc_offset` sets local time's offset from UTC (hours, -12 to 14; `null` for the longitude's nominal zone, shown as `utc_offset_used`).

```sh
ccad -p house.ccad --save run view_set view="3D 1" 'sun={"mode":"single_day","date":"2026-12-21","interval":"minutes_30","shadows":true}'
```

**Detail lines.** `detail_line_create` draws view-specific 2D lines in a floor plan, ceiling plan, elevation, section, legend or drafting view (`view`), in the view's drawing coordinates in meters (as for text notes and dimensions): `curves` take the same forms as sketches (`line`, `arc` by `bulge` or `through`, `circle`, `ellipse` or part of one, `rectangle`, `polygon`), each curve one element (category `detailline`; a polyline is its segments), all in one undo step, and `style` is one of the built-in line styles `thin` (default), `medium`, `wide`, `hidden`, `dashed`, `center` and `dotted`. Only their view draws them, on top of the model; `view_delete` deletes them with it, and 3D views and IFC export leave them out. `element_get` shows `params.curve` (the curve as `Line`, `Arc`, `Circle` or `Ellipse`) and `params.style` (`{"BuiltIn": "Wide"}`) with the derived `length` and `style_name`; `element_set` changes them (`"style": "hidden"` also works). The generic commands edit them: `elements_move`, `elements_copy`, `elements_rotate`, `elements_mirror`, `elements_delete`, `trim_corner` (two detail lines, meeting where their lines, circles or ellipses cross), `trim_extend` (to another detail line or the line of a wall, beam or grid), `element_split` and `element_offset` (arcs and circles concentric; ellipses have no offset). DXF export puts them on a layer named after their style (`THIN`, `CENTER`…).

```sh
ccad -p house.ccad --save run detail_line_create view=Térreo style=dashed 'curves=[{"kind":"circle","center":[2,3],"radius":0.5}]'
```

**Line styles.** Besides the built-in styles, a project has its own line styles (name, weight on paper in mm, color, dash pattern), kept as project settings: `line_styles_list` lists both (built-in by code, the project's by name, with `weight`, `color`, `pattern` and `used_by`), `line_style_create` adds one (`name` unique and not a built-in code, `weight` 0.05 to 5 mm, default 0.25, `color` `[r, g, b]`, `pattern` `solid`, `dash`, `short_dash`, `dot` or `dash_dot`), `line_style_set` changes it (the lines drawn with it follow) and `line_style_delete` deletes it, moving the detail lines and region boundaries that use it to `replace_with` (default `thin`) in the same undo step. Wherever a command takes a line style (`detail_line_create`'s `style`, a region's `boundary`, `element_set`) a project style is named like a built-in one; `params.style` then reads `{"Project": <id>}`. A file whose lines use a style that is gone gets the thin style when it is opened.

**Filled and masking regions.** `filled_region_create` draws a 2D area in a floor plan, ceiling plan, elevation, section, legend or drafting view (`view`), in its drawing coordinates in meters: `curves` (as for sketches) close into loops, the largest the outline and those inside it openings, all in one undo step. It is filled as its type says and edged in the `boundary` line style (default `thin`; `none` for no boundary); `type` picks a filled region type, else the project's first, made with the first region (diagonal lines over white). `filled_region_type_create` makes another (`foreground` and `background` patterns — `none`, `solid`, `diagonal`, `diagonal_back`, `crosshatch`, `horizontal`, `vertical`, `grid`, `brick`, `concrete`, `sand`, `insulation`, `earth` — each with its color; a `solid` pattern fills with its color and a `none` background leaves the model visible between the pattern lines); `type_set` changes its `fill`. `masking_region_create` draws an opaque white area that hides the model's linework under it (no boundary unless `boundary` names a style). Regions are drawn in their own view only, in id order, over the model and under the view's annotations (detail lines, dimensions, texts and, in plans, tags, grids and section marks); 3D views and IFC export leave them out. `element_get` shows `params` (`outline`, `openings`, `sketch`, `boundary`) with the derived `area`, `perimeter` and `boundary_name`; `element_set` changes the type or `"boundary": "wide"` / `"none"`; `sketch_get` and `sketch_set` read and redraw the outline; the move, copy, rotate, mirror and delete commands work on them, and `view_delete` deletes them with their view. Exports write the fills: SVG, PDF and PNG as polygons (a region with openings as one ring), DXF as `SOLID` triangles and pattern lines on a layer named after the boundary's style (a region without a boundary on its category's, `FILLEDREGION` or `MASKINGREGION`).

```sh
ccad -p house.ccad --save run filled_region_create view=Térreo boundary=wide 'curves=[{"kind":"rectangle","corner":[0,0],"opposite":[4,3]}]'
```

**Detail components.** A detail component is a reusable 2D component for construction details (category `detailcomponent`, component kind `Detail`): its pieces are the 2D pieces title blocks use — `stroke` (lines and arcs through points, open or `closed`, with a pen `weight` in paper millimeters, `color`, dash `pattern` and, closed, a solid `fill` or a pattern fill), `text` and `image` — plus `nested` pieces that place another detail component inside it (`component`, `x`, `y`, `rotation`, `scale`, `mirrored`), all in meters in its own frame (text heights too) and as numbers or expressions over its parameters. `detail_component_create` makes one as a new type named `name` (unique among types and components): from `pieces`, or from `elements` of a `view` — detail lines keep their pen, filled and masking regions become closed strokes with their fill (openings as outlines), text notes texts at their printed height (leaders as lines) and detail components nested pieces — whose point `origin` (default: the lower left corner of their extents) becomes the component's origin; with `replace` (default true) those elements are deleted and the component is placed where they were (`placed`), all in one undo step. `detail_component_place` places a type in a floor plan, ceiling plan, elevation, section, legend or drafting view (`view`): its origin at `position`, turned `rotation` degrees, sized by `scale` (0.0001 to 10000) and flipped across its own X axis first when `mirrored`. Only that view draws it, over the model and its regions and under detail lines and annotations; `view_delete` deletes it with the view, and 3D views and IFC leave it out. `element_get` shows `params` (`view`, `position`, `rotation_deg`, `scale`, `mirrored`, `overrides`) with the derived `component`, `pieces` and `nested`; `element_set` changes them (not the view); the move, copy, rotate, mirror and delete commands work on it. `component_sheet_piece_add`, `component_sheet_piece_set` and `component_sheet_piece_delete` edit a detail component's pieces (every placed one follows), `component_param_*` its parameters. A nested piece names a component (it reads the parameter values of that component's first type); naming one that holds this component (a cycle) is refused, and drawing stops at 8 levels of nesting and a budget of pieces in any case. `component_save` writes the components it nests into the `.ccomp` file too, and `component_load` adds those the project lacks; `component_library_list` lists detail components with `kind` `Detail` and `category` `detail_component`. `cad_import` with `blocks: true` brings a DWG or DXF file's blocks in as detail components instead of flattening them: every block inserted by a move, turn, uniform scale or mirror becomes a type named after it (its own inserts of blocks as nested pieces, references that would close a cycle dropped), each such insert of the drawing a placed detail component, and what is left (other inserts, dimensions, loose entities) stays in the imported drawing; the result lists the new `components.types` and `components.elements`. DXF export (`view_export`) writes a view's detail components as blocks (one per component, a placed one with its own parameter values its own) and `INSERT`s on the `DETAILCOMPONENT` layer, the nested ones as inserts inside their blocks.

```sh
ccad -p house.ccad --save run detail_component_create name=Brick 'pieces=[{"kind":"stroke","closed":true,"fill":[200,80,60],"points":[{"u":0,"v":0},{"u":0.2,"v":0},{"u":0.2,"v":0.1},{"u":0,"v":0.1}]}]'
ccad -p house.ccad --save run detail_component_place type=Brick view="Drafting View 1" position=[1,2] rotation=90
ccad -p house.ccad --save run cad_import path=details.dwg view="Drafting View 1" blocks=true
```

**Imported CAD drawings.** `cad_import` reads a DWG (R12 to 2018 and later, through the acadrust library) or DXF file (ASCII or binary, read by ConstructCraft itself) into a floor plan, ceiling plan, elevation, section, legend or drafting view as one undo step, and reports what it read: `format`, `version`, `units` (and `units_assumed`), `layers`, `entities`, `blocks`, `primitives`, `skipped` (entities not drawn, by type), `truncated` and the `extents` in the view. Blocks are inserted (nested, scaled, rotated, arrayed), arcs and circles stay arcs, ellipses, splines and hatch boundaries become polylines, texts and MTEXT keep their position, height and rotation; 3D solids, images, infinite lines and similar entities are skipped. `position` with `align` places the drawing's origin (`origin`, default), the center of its extents (`center`) or the origin unless it is more than 1 km from the view's content (`auto`); `units` overrides the file's (`mm`, `cm`, `dm`, `m`, `km`, `in`, `ft`, `yd`, `mi`), and unitless files are read as meters when small, else millimeters. The element starts pinned and only its view draws it, under the model (category `cadimport`; 3D views and IFC export leave it out). `element_get` shows its placement in `params` (`units`, `scale`, `position`, `rotation_deg`, `mirrored`, `hidden_layers`, `color_mode` `Grey`/`Layers`/`Black`, `halftone`) but not the drawing; `element_set` changes them; the move, rotate, mirror, pin and delete commands work on it. `cad_layers` lists its layers and `cad_layers_set` shows or hides them. `cad_explode` turns it into native elements of its view in one undo step: lines, polylines (bulged segments as arcs), arcs and circles become detail lines, one element per segment (ellipses, splines and hatch boundaries, stored as polylines, come out as their segments); fills become filled regions with no boundary, of a solid filled region type of the fill's color (named *Solid #RRGGBB*, reused when one fills with that color); single-line texts become text notes at the same place, height, rotation and alignment, of a text type of their size on paper at the view's scale (reused when one has that size). Each layer becomes a project line style of the layer's name and color (light colors darkened for paper), reused when a style of that name exists; layers hidden in the element are left out (it starts with those off or frozen in the file), and the placement is applied. Pictures stay in the element (its other primitives removed); without them the element is deleted. The color mode and halftone are not carried over. The result lists the `created` elements, the counts `detail_lines`, `filled_regions` and `texts`, the new `line_styles` and `types`, `kept` (the element when pictures stay) and `skipped` primitives (too small or out of bounds); a drawing that would make more than 100,000 elements, or add more than 1,000 types, is refused. With `cad_import` `blocks: true` the blocks are already detail components, so exploding leaves them as they are.

```sh
ccad -p house.ccad --save run cad_import path=survey.dwg view=Térreo align=auto
ccad -p house.ccad --save run element_set id=120 'params={"units": "Centimeters", "halftone": true}'
ccad -p house.ccad --save run cad_layers_set id=120 'layers=["DIM", "TEXT"]' visible=false
ccad -p house.ccad --save run cad_explode id=120
```

**Validation and undo.** Every edit is one undo step (`undo`, `redo`). An edit that would leave the model invalid — a door pointing at a missing wall, a negative size, a reference to nothing — is refused and leaves no trace. Pinned elements are not moved, trimmed or deleted.

**Errors.** A failed command returns `{"error": {"code": …, "message": …}}`. Messages are in the session's language (`--lang`, `language_set`).

| Code | Meaning |
|---|---|
| `unknown_command` | no command has that name |
| `invalid_arguments` | the parameters do not match the schema or make no sense (zero length, unknown kind…) |
| `not_found` | a referenced element, type, level, view or material does not exist |
| `rejected` | the edit would leave the model invalid, or is not allowed (pinned, in use…) |
| `io` | reading or writing a file failed |
| `internal` | a bug; the project is kept as it was |
| `script` | a Lua script or plugin failed: a syntax or run-time error, or a limit reached |
| `pending` | a plugin command goes on in the background, waiting for an external program (only in a session that does not block, see [External programs](#external-programs)) |
| `cancelled` | a plugin command waiting for an external program was cancelled; what it did was undone |

## Command line: `ccad`

`ccad` is a separate program without graphics dependencies (it runs on servers and in CI). Release packages include it; from the sources: `cargo build --release -p cc_cli` (the binary is `target/release/ccad`).

```text
ccad [options] commands             list the commands
ccad [options] schema <command>     the JSON Schema of a command's parameters
ccad reference                      the command reference in Markdown
ccad [options] run <command> [args] run one command
ccad [options] script <file|-> [level=<id|name>]
                                    run a script of commands
ccad [options] lua <file|-> [args]  run a Lua script
ccad [options] plugins              list the plugins loaded with --plugins
ccad plugins install <zip> <folder> install a plugin archive into a plugins folder
ccad plugins remove <id> <folder>   remove an installed plugin
ccad plugins new <folder> <id> [name=<name>]   a new plugin from the template
ccad plugins test <plugin folder> [--filter <text>]   run its tests/*_test.lua
ccad [options] mcp                  serve the commands over MCP (stdio)

-p, --project <file>   open a project (.ccad, or .ifc to import) first
--save                 save the project afterwards (to its file)
--save-as <file>       save the project afterwards to <file>
--plugins <folder>     load the plugin in <folder>, or each plugin in its
                       subfolders (repeatable)
--lang <code>          language of default names and messages (also CCAD_LANG):
                       en, pt-BR, es, fr, de, it, lo, pl, uk-UA, ru, zh-TW, zh-CN or ja
```

Arguments of `run` can be a JSON object, `@file.json`, `-` (standard input) or `key=value` pairs, whose values are read as JSON when they parse and as text otherwise:

```sh
ccad -p house.ccad --save run wall_create start=[0,0] end=[5,0] height=2.8
ccad -p house.ccad --save run wall_create start=[5,0] end=[5,4] mid=[6,2]   # an arc wall through mid
ccad -p house.ccad run elements_list category=door
ccad -p house.ccad --save run view_set '{"view": "Section 1", "scale": 100}'
ccad -p model.ifc --save-as model.ccad run project_info              # import an IFC file
ccad -p house.ccad run sheet_export sheet=A101 path=a101.pdf
ccad -p house.ccad run view_export view="Section 1" path=section.png dpi=300   # any view as PNG, SVG or PDF
ccad -p house.ccad run view_export view="Level 1" path=level1.dxf              # or as DXF, at full size in mm
ccad -p house.ccad run view_export view="South" path=south.dxf hidden_lines=false   # an elevation as displayed
```

A DXF of an elevation, section or 3D view holds its visible linework: the lines hidden behind opaque surfaces are removed, the surface fills (shading) left out and the lines joined into polylines, while cut fills, regions and texts stay; the result says `"hidden_lines": true`. `hidden_lines=false` writes the drawing's primitives as displayed, and `hidden_lines=true` cleans a plan or other view too. A view too large to clean is written as displayed, with `"hidden_lines": false` and a `warning`.

Results are printed as JSON on standard output. Errors go to standard error as JSON with exit code 1; usage errors exit with 2. Without `--save` or `--save-as` the project file is not changed.

Results combine with other tools; for example, with `jq`:

```sh
wall=$(ccad -p house.ccad --save run wall_create start=[0,0] end=[6,0] | jq .id)
ccad -p house.ccad --save run door_create wall=$wall offset=1.5
```

### Scripts

A script is a list of steps, either a JSON array of `{"command": …, "args": {…}}` objects or one such object per line (lines starting with `//` are comments). `ccad script` runs the steps in order in one session and stops at the first error; each step is one undo step.

[`examples/small_house.ccmacro`](../examples/small_house.ccmacro) builds a small house:

```text
// A small house built with engine commands only.
{"command": "wall_create", "args": {"start": [0, 0], "end": [8, 0], "height": 2.8}}
…
{"command": "door_create", "args": {"point": [2, -0.5]}}
{"command": "floor_create", "args": {"from_walls": true}}
{"command": "room_create", "args": {"point": [2, 3], "name": "Living room"}}
```

```sh
ccad --save-as house.ccad script examples/small_house.ccmacro
```

### Outlines drawn with curves

Floors, ceilings, roofs, foundation slabs and masses take their outline as `curves` instead of points: lines, arcs (by `bulge` or a point they pass `through`), circles, ellipses (whole or a part), splines (`through` the points given, `closed` back to the first, or by the `control` points of their cubic Bézier segments, `p0, c1, c2, p1…`, as `sketch_get` gives them back), rectangles and regular polygons (these two become lines). Component pieces take splines too: a profile point's `bezier` (`u1`, `v1`, `u2`, `v2`) makes the segment to the next point a cubic Bézier curve with those two handles. Loops inside the outline are openings of floors and roofs. The element keeps its curves: `sketch_get` gives them back (or lines along the outline of an element drawn with points) and `sketch_set` draws the outline again; moving, rotating and mirroring the element moves them too, and editing the outline's points directly drops them.

```text
{"command": "mass_create", "args": {"curves": [{"kind": "circle", "center": [0, 0], "radius": 5}], "height": 9}}
{"command": "floor_create", "args": {"curves": [{"kind": "rectangle", "corner": [0, 0], "opposite": [10, 8]}, {"kind": "circle", "center": [5, 4], "radius": 1}]}}
```

### Terrain

`topography_create` makes a terrain surface from measured points `[x, y, z]` (elevations above the level) inside a `boundary` or `curves`; with neither, the convex hull of the points bounds it. The boundary's vertices take the elevations the points give them, and the earth closes down to a flat base `base_depth` (default 2 m) below the lowest point, so sections cut it as a solid. `element_get` shows its lowest and highest elevations, base, area, volume and any warnings (`few-points`, `collinear`, `boundary-crossing`) in `derived.topography`. `topography_points_set` replaces the points or changes, removes and adds them by index; `sketch_set` redraws the boundary, and with `points` replaces the points in the same step. A terrain type sets the earth material and the contour lines' interval and majors (`topography_type_create`, `type_set`).

```text
{"command": "topography_create", "args": {"points": [[0, 0, 0], [20, 0, 2], [20, 10, 2.5], [0, 10, 0.5], [10, 5, 1.2]]}}
```

`pad_create` grades a building pad into a terrain: a flat area at `elevation` (meters above the terrain's level) inside an `outline` or `curves`, with a cut slope where the ground stands above it and a fill slope where it lies below, each running out to the existing ground. Slopes are the horizontal run per unit of rise (`cut_slope` 1 is 1:1, `fill_slope` 1.5 is 1:1.5); without them the pad uses its type's (`pad_type_create`, `type_set`). The `host` terrain defaults to the one whose boundary holds the outline's center. A pad lives on its terrain's level, moves with it and is deleted with it; pads of one terrain apply in creation order. `element_get` shows a pad's `cut`, `fill`, `net` (fill − cut), `area` and warnings (`off-terrain`, `coarse`, `failed`) in `derived.pad`, and a terrain's `derived.topography` sums its pads' `cut` and `fill`. Pad schedules have `Elevation`, `CutVolume` and `FillVolume`, which total like volumes.

```text
{"command": "pad_create", "args": {"outline": [[6, 3], [14, 3], [14, 8], [6, 8]], "elevation": 1.2}}
```

### Curtain walls

A curtain wall is a wall whose type has a curtain grid instead of layers: vertical lines along the wall and horizontal lines up from its base (a fixed distance, a number of divisions or a maximum spacing, justified at the start, the center or the end), a panel in each cell (glass, solid, empty or a component, such as a door) and mullions on the lines and around the border. `curtain_type_create` makes the type and `wall_create` places walls of it, straight or arc (flat panels along the arc). Each wall can adjust its grid: `curtain_grid_set` adds, moves and removes its lines and removes pieces of them (the cells beside merge into one panel), `curtain_panel_set` swaps panels, and `curtain_get` lists lines and panels. A curtain wall drawn inside another wall opens it where it stands. Wall schedules have `PanelCount`, `GlazedArea` and `MullionLength`. IFC export writes them as `IfcCurtainWall`s made of `IfcPlate`s and `IfcMember`s and import brings them back with their grid; a curtain wall from another program comes back as one when its panels make a plain grid, else as separate objects.

```text
{"command": "curtain_type_create", "args": {"name": "Storefront", "curtain": {"vertical": {"spacing": 1.2}, "horizontal": {"layout": "FixedNumber", "count": 2}}}}
{"command": "wall_create", "args": {"start": [2, 0], "end": [6, 0], "type": "Storefront", "height": 2.8}}
```

### Pipe series

A pipe series is one product line: its sizes (outside diameter, wall and inside diameter) and the fittings sold for it with their dimensions. `pipe_catalog` lists the 22 built-in series (Brazil, Europe, US) or, with `series`, one series' rows in millimeters. `pipe_series_import` copies a built-in series (`code`) or a series file (`path` or `text`, JSON in millimeters as `pipe_series_export` writes it) into the project; `on_clash` (`skip`, `replace`, `rename`) settles a code the project has, and `sizes` (`["all"]` or labels) also makes the pipe types, named after the series and size. `pipe_type_create` makes one type of a row, importing a built-in series when needed. A type of a series keeps the row's outside diameter (`type_set` takes `series` and `size`; a null `series` makes it free), and sizing uses the row's inside diameter: `mep_sizing` gives each run of a series the `size` it needs, and `mep_size_apply` keeps it in its series, making the missing type. `pipe_series_delete` refuses a series in use unless `detach`. The templates' pipe types are sizes of series, and older projects' template pipes are linked to them when opened.

Where pipes of one series meet, the fitting is matched to the fittings sold for that series: an elbow takes the nearest catalog angle (within 7.5°), a tee or a wye (a branch meeting the run at 45°) its run and branch sizes, a coupling or a reducer its sizes. A reducing tee the series does not sell becomes a tee plus reducers on the branch, and a reducer that skips sizes a chain of up to three reducers (`status: composite`, with its `pieces`); a fitting with no row (a 60° elbow, a size the series lacks) is `missing`, with its `reason`. `mep_fittings` lists each with its `name` ("Elbow 90° PVC soldável 25 mm"), `series`, `size` and `size2`, and counts the missing ones. Matched fittings are drawn in 3D with the catalog's dimensions (rules on the outside diameter where a value is unknown), their laying lengths come off the pipes' `cut_length` (`element_get`, and the `CutLength` field of pipe schedules, beside `Series`, `Size` and `InsideDiameter`), and IFC export types each by an `IfcPipeFittingType` per catalog row; pipe segment types carry the `ConstructCAD_PipeSeries` property set.

```text
{"command": "pipe_series_import", "args": {"code": "EU-PVCU-1452-PN10", "sizes": ["d32", "d63"]}}
{"command": "pipe_type_create", "args": {"series": "US-CU-L", "size": "3/4\""}}
{"command": "type_set", "args": {"type": "PVC-U pressure pipe PN10 d32", "params": {"size": "d50"}}}
```

### Wall profiles

A straight wall may have an elevation profile: the outline of its face drawn in its own elevation plane, `[u, z]` meters along its location line from its start and above its base, so its base and top need not be level (a top following sloping ground, a stepped base, a gable end, an arched top). `wall_profile_set` takes the outline as `curves` (as for sketches) or `points`; it must be one closed loop that does not cross itself and that a vertical line crosses at most twice (a bottom and a top line along the wall, vertical ends and steps allowed), with no holes, within the wall's length (a shorter profile makes the wall only that long) and at most 100 m below or 1000 m above its base. `wall_profile_slope` sets a straight base and top from one end to the other (`top_start`, `top_end`, `base_start`, `base_end`, heights above the base; by default the base at 0 and the top at the wall's height), and `wall_profile_remove` takes the profile off. `wall_create` takes `profile` curves too, and `sketch_get` / `sketch_set` read and draw a wall's profile (`sketch_get` gives the rectangle of its length and height when it has none). The wall's height and top constraint stay its nominal box; its body, volume, plans (cut only where the plane passes through it), elevations, sections, 3D views, area in schedules and IFC follow the profile. Doors, windows and niches that do not lie inside it are left out of the wall. When the wall's length changes the profile stays where it is along the wall, is cut at the wall's ends and follows an end it reached when the wall grows there; a mirror walks it the other way and a split splits it. Arc walls, curtain walls, walls attached to a roof and framed walls take no profile. `element_get` shows it in `params.profile` and its `area`, `min_z`, `max_z`, `start` and `end` in `derived.profile`.

```text
{"command": "wall_profile_slope", "args": {"wall": 12, "top_start": 3.0, "top_end": 1.2}}
{"command": "wall_profile_set", "args": {"wall": 13, "points": [[0, 0], [6, 0], [6, 3], [3, 4.5], [0, 3]]}}
```

### Layer extents

Each layer of a wall type may have a vertical extent of its own: `base_offset`, meters from the wall's bottom at each point along it (positive starts higher, an insulation above the ground; negative runs below, a core going deeper into the foundation; within ±100 m), and `height`, meters from the layer's own bottom (0.001 to 1000; null or left out: up to the wall's top). A layer never rises above the wall's top, and where it has no height left it is absent. They ride on `type_set` with the whole `layers` list as `type_get` shows it (default extents are not shown); floors and roofs take none. The wall's body is stepped across it, plans cut each layer at its own height, openings cut every layer they cross, and the wall's volume is the sum of its layers'; `element_get` lists each layer's `material`, `thickness`, `volume`, and its lowest `base` and highest `top` above the wall's base in `derived.layers`. Joins, rooms and the doors a wall hosts still see the wall whole.

```text
{"command": "type_set", "args": {"type": "Insulated", "params": {"layers": [
  {"material": "EPS", "thickness": 0.05, "function": "Insulation", "base_offset": 1.0},
  {"material": "Concrete", "thickness": 0.2, "function": "Structure", "base_offset": -0.5},
  {"material": "Plaster", "thickness": 0.02, "function": "Finish", "height": 2.4}]}}}
```

### Spline walls

A wall may follow a spline, a smooth free-form curve, instead of a line or an arc (first stage of the feature). `wall_create` draws one from `start` to `end` `through` the points given between them (at most 63: the curve passes through each, its tangents taken from the neighbours), or by the `control` points of its cubic Bézier segments (`p0, c1, c2, p1, c1, c2, p2…`, 3n + 1 points for n segments, at most 64, the first being `start` and the last `end`), as `element_get` shows them in `params.spline.control`. The curve is the wall's location line, so `location_line` moves the body across it as for other walls. It must be open and at least 5 cm long, keep its direction where segments join (a corner takes two walls), leave the face inside every bend a radius of 5 cm at least (a curve cannot bend tighter than half the wall, measured from its location line, plus that) and not cross itself; a curve that does not is refused with the reason and the point.

`element_get` gives `derived.length` along the body centerline, and in `derived.spline` the number of `segments`, the least radius of curvature of the location line (`min_radius`), the least radius left to a face inside a bend (`face_radius`) and the `knots` the segments join at. `element_set` patches `params.spline.control` (`"spline": null` makes the wall straight between its ends, and a straight wall takes a spline); moving one end alone (`params.end`) takes that end's knot and handle along. Move, rotate, mirror, copy and delete work as for any wall; `element_split` splits the curve where picked and `element_offset` makes a spline wall alongside (as near as a spline follows another).

The body follows the curve as chords no more than 0.5 mm off it: plans draw the cut with its layers along it at every detail level, elevations, sections and 3D views show the curved faces, rooms are bounded by it, dimensions hold on to its faces, and schedules give its length, area and volume. Its ends join other walls as a straight wall with its direction there would (corners and T joints, exact on the curved faces), and other walls may end on its curved side.

Not there yet, and refused by the commands: doors, windows and niches in a spline wall, an attached top (`wall_attach`), an elevation profile, a curtain wall type, reinforcement, framing, and trim, extend and align (to it or of it); `strip_footings_under_walls` leaves it out. IFC export writes a spline wall as an `IfcWall` whose body is a closed shell of facets, with its type's material layer set and no axis; importing that file brings it back as a component of the same shape.

```text
{"command": "wall_create", "args": {"start": [0, 0], "end": [9, 0.5], "through": [[3, 1.5], [6, -1]], "height": 2.8}}
{"command": "wall_create", "args": {"start": [0, 4], "end": [6, 4], "control": [[0, 4], [1, 4], [2, 5], [3, 4], [4, 3], [5, 4], [6, 4]]}}
{"command": "element_set", "args": {"id": 14, "params": {"spline": {"control": [[0, 0], [2, 2], [5, 2], [7, 0]]}}}}
```

### Roof joins

A cross wing or bay is joined to the roof it meets with `roof_join` (`roof`, the wing; `to`, the main roof): the wing's end runs on into the main roof, or is cut, until it meets the main roof's surface, forming valleys, and the main roof is cut away under it. The wing meets the main roof along its edges that lie on it, or across a gap its nearest edge looks out over (no farther than the wing is wide or long); when the edges beside that end are sloped eaves (a gable wing with its ridge running into the main roof, or a hip wing, which gets a gable there), the end runs on, else it is only cut. A wing lower than the main roof meets it higher up the slope; one higher runs on level until the main roof rises to its eaves. Chains (a bay on a wing on a main roof) and several wings on one roof work. The join stays when either roof changes, and is refused when the roofs do not meet, for a roof joined to itself or when it would close a loop. Walls attached to either roof follow both; areas, tiles, framing, schedules and IFC count each part once. `element_get` gives `derived.join` (`to`, `active`, `issue` when the join does nothing, the `valleys` and their `valley_length`, and `cut_by`, the wings that cut a main roof), and `roof_check` reports a join that does nothing. `roof_unjoin` builds both as drawn again; deleting the main roof frees its wings.

```text
{"command": "roof_join", "args": {"roof": 21, "to": 12}}
{"command": "roof_unjoin", "args": {"roof": 21}}
```

### Cable trays and valves

`cable_tray_create` draws a cable tray run like `conduit_create` (with `circuits`, `offset`, `end_offset`); a project without a cable tray type gets one (200 × 100 mm, ladder), and `cable_tray_type_create` makes others (`width`, `height`, `style`: `ladder`, `solid_bottom` or `perforated`). Trays join only other trays, and their elbows and tees are listed by `mep_fittings`. `valve_place` puts a valve on a pipe at a `point` near it or a `distance` along it, of a `type` or a `kind` (`gate`, `ball`, `check`, `globe`; a type of that kind is made when there is none); `valve_type_create` makes types of several `kinds` at once. A valve takes its pipe's diameter and system, is a node of its network and follows the pipe; `element_get` shows its `derived.point`, `kind` and `diameter`, and `element_set` moves it along the pipe (`params.at`).

```text
{"command": "cable_tray_create", "args": {"start": [0, 0], "end": [6, 0], "circuits": ["1"]}}
{"command": "valve_place", "args": {"pipe": 42, "distance": 0.5, "kind": "ball"}}
```

### Lightning protection and structured cabling

`lps_conductor_create` draws a lightning protection conductor from `start` to `end` (on the roof surface under its ends unless `offset`/`end_offset` say otherwise), or a down conductor from the roof over `start` down to 0.5 m below the level when there is no `end`; `role` (`air`, `down`, `earth`) defaults by where it runs. `earth_ring_create` lays buried conductors around the level's walls (or `points`), `air_terminal_place` puts a rod of `height` on the roof under `position`, and `earth_electrode_place` drives a rod of `length` below the level. They join the first lightning protection system (`system_create` with `kind: "lightning"`), made with the first of them. `lps_check` (`class` `I` to `IV`) samples the roofs and returns how many points the air termination protects, by the rolling sphere and mesh methods, with the unprotected points; `systems_list` gives each lightning protection system's terminals, electrodes and conductor lengths by role.

`rack_place` places a rack and creates its data system (`kind: "data"`, its `panel` the rack, coded after the rack's mark); `data_outlet_place` places outlets cabled to a `rack` (or a data `system`) with `ports` jacks; `cabling_list` gives per rack the outlets, cables, the estimated length of each and the cables over 90 m. `device_type_create` gives the project a built-in device type (`data_outlet`, `rack`, `air_terminal`, `earth_electrode` or any electrical device). Conduits and cable trays carry data systems like circuits.

```text
{"command": "earth_ring_create", "args": {}}
{"command": "lps_conductor_create", "args": {"start": [0, 4], "end": [10, 4]}}
{"command": "air_terminal_place", "args": {"position": [5, 4], "height": 2}}
{"command": "lps_check", "args": {"class": "II"}}
{"command": "rack_place", "args": {"position": [0, 0]}}
{"command": "data_outlet_place", "args": {"position": [8, 3], "rack": 120, "ports": 2}}
```

### Structural profiles

Column and beam types are drawn and built with a structural profile: a standard parametric shape (`i`, `channel`, `rect_tube`, `cold_lipped_u`, `rect`…) of a material (`steel`, `cold_formed`, `timber`, `concrete`), or a sketch. `profile_catalog` lists the built-in series (W, HP, IPE, HEA, HEB, NBR 6355 cold-formed, sawn timber, glulam, SHS, RHS, CHS) and, with `series` (and `search`), their rows; `profile_import` copies rows into the project as profiles that remember their catalog entry (`on_clash`: `keep` the existing profile, the default, or `rename`). `profile_type_create` makes a column or beam type of a profile, with a material of its class; `type_set` changes a type's `profile` (a name, or `null` for its own section), `profile_rotation_deg` and `profile_mirror`. `profile_create`, `profile_set`, `profile_duplicate` and `profile_delete` (`replace_with`) edit profiles, whose types follow; `profile_get` gives the outline and section properties, `element_get` a column's or beam's `derived.section` (area, mass per meter, moments of inertia and moduli), and `analytical_model` each member's section. The members of an arc beam follow its arc: each has an `arc` (`bulge`, `center`, `radius`, `sweep` in degrees) and the arc's `length` (format 3); `arc_segments` (on `analytical_model` and `analytical_export`) writes each as that many straight members between nodes on the arc, for programs that take only straight members.

```text
{"command": "profile_import", "args": {"series": "IPE", "designations": ["IPE 200"]}}
{"command": "profile_type_create", "args": {"profile": "IPE 200", "category": "beam"}}
{"command": "beam_create", "args": {"type": "IPE 200", "start": [0, 0], "end": [6, 0]}}
{"command": "profile_set", "args": {"profile": "IPE 200", "name": "IPE 200 (roof)"}}
{"command": "profile_get", "args": {"profile": "IPE 200 (roof)"}}
{"command": "profiles_list", "args": {"material": "steel"}}
```

### Structure by material and grid frames

`structural_type_for` (`category` column or beam, `material` concrete, steel or timber) returns the first type of that material, or creates a default one from the catalog. `grid_frame_create` places columns on every storey of a range of levels and beams on the upper levels over the grids (`grids` default all, `base_level`/`top_level` default the lowest two levels, `column_type`/`beam_type` or a `material`), with optional secondary beams in each bay, in one undo step (at most 2000 members); it returns the ids and the number skipped.

```text
{"command": "grid_frame_create", "args": {"base_level": "Térreo", "top_level": "Cobertura", "material": "steel", "secondary": {"parallel_to": 12, "spacing": 2.5}}}
```

`steel_building_create` makes a parametric steel building: portal frames of columns and rafters over `bays` bays, with purlins and girts (`null` turns one off), as ordinary columns and beams in one undo step; types default to the catalog's `W310x38,7`. Its members are regenerated, with their ids kept, when `element_set` changes the building's parameters; a member edited or deleted directly is detached and left alone. Members follow the building's phases and design option, and a copy or paste makes its own. `element_get` lists `derived.members`, `detached` and `at_limit` (the layout reached the member limit and left the last purlins or girts out).

```text
{"command": "steel_building_create", "args": {"span": 24, "eave_height": 7, "bays": 8, "roof": "mono", "girts": null}}
{"command": "element_set", "args": {"id": 42, "params": {"span": 30}}}
```

`truss_create` makes a parametric truss: a host element whose chords, diagonals and posts are ordinary beams and columns, made in one undo step and returned as `{id, members}`. `shape` is `flat`, `mono` or `gable`; `pattern` is `pratt`, `howe`, `warren`, or for gables `kingpost`, `queenpost` and `fink`; `copies` trusses stand `spacing` apart to the left of the span line; `material` (`steel` or `timber`) picks the catalog rows of the default types (concrete is refused). Members are laid out as they are built: each chord is one continuous beam per straight run (two top chords on a gable); the posts stand between the chords, on the faces of solid chords or inside channels that open toward the web (the steel defaults: channels on their sides, the bottom chord open upward and the top one downward); and where the truss ends on a post the chords run on over it and the far post is turned around, so both ends are closed. Like a steel building, its members are regenerated with their ids kept when `element_set` changes its parameters, and a member edited or deleted directly is detached.

```text
{"command": "truss_create", "args": {"start": [0, 0], "end": [12, 0], "height": 2, "shape": "gable", "pattern": "fink", "panels": 6, "copies": 5, "spacing": 1.2, "material": "timber"}}
{"command": "element_set", "args": {"id": 42, "params": {"panels": 8}}}
```

`connections_create` places steel and timber connections: `kind` is `base_plate` (on columns), `end_plate`, `fin_plate`, `timber_bolts` or `hanger` (on beams); `members` lists them, else `level` (default the active level) takes all of its columns or beams. Only the ends that suit the kind get one, ends that already have one are skipped, and the result is `{created, skipped}` in one undo step. `params` changes the kind's default sizes (meters; `thickness`, `width` and `length`, 0 meaning from the section, `bolt_diameter`, `rows`, `columns`, `spacing`, `edge`, `anchor_length`). Later `element_set` edits the sizes, not the member, end or kind. Schedules have the fields `ConnectionKind`, `PlateMass` (kg), `BoltCount` and `BoltDiameter`.

```text
{"command": "connections_create", "args": {"kind": "end_plate", "members": [12, 13], "params": {"rows": 3, "bolt_diameter": 0.02}}}
{"command": "element_set", "args": {"id": 42, "params": {"params": {"thickness": 0.02}}}}
```

`wall_framing_create` frames straight walls in light steel or wood: one framing element per wall, owning ordinary columns (studs, king and jack studs, cripples) and beams (tracks or plates, headers, sills, blocking), made in one undo step and returned as `{framings: [{id, wall, members}]}`. `system` is `light_steel` (default) or `wood`; `spacing` is the stud spacing in meters (0.2 to 1.2; 0.4 and 0.6 by default); `stud_type`, `track_type` and `header_type` take an id or a name (default: the system's catalog types); `double_top` (wood) and `blocking` are booleans. Curved walls and walls already framed are refused before anything is made. The members are regenerated, with their ids kept, when `element_set` changes the framing's parameters or when the wall or its doors and windows change; a member edited or deleted directly is detached. The default tracks and plates are the catalog rows turned 90 degrees, lying flat (the section's width is their height); a given type is used as it draws. The framing and its members follow the wall's phases, design option and level. Studs are not merged where an opening reaches a wall end, where two openings are closer than about four stud widths, or at the corner of two framed walls; splitting a framed wall reframes only the part that keeps the framing.

```text
{"command": "wall_framing_create", "args": {"walls": [12, 13], "system": "wood", "blocking": true}}
{"command": "element_set", "args": {"id": 42, "params": {"spacing": 0.4}}}
```

`beam_create` draws an arc beam with `mid` (a point on the arc), `bulge` or `radius` (positive counter-clockwise; `major_arc` takes the longer arc), as `wall_create` draws arc walls; `element_get` then gives its `center`, `radius`, `sweep` and `true_length` along the arc. `rebar_create` lays bars in a beam or column: `top`, `bottom` (beams) or `perimeter` (columns) with a `count`, `sides` (side bars on both side faces of a beam: a `count` per face, or a `spacing` over the height) and `stirrups` every `spacing`, each closed by two hooks at one corner (`hook`: 135 degrees by default, 90 to 180, 0 for none; `tail`: the straight end, 10 bar diameters by default, at least 5 cm) that count in its length and show in the bending schedule (`stirrup 14x44 hook 135° 5 hook 135° 5`). In an arc beam the bars follow the arc, each as long as the arc at its own radius, and the stirrups are radial. Bars that do not fit side by side go in a second layer, and bars along a beam longer than `max_length` (default 12 m; 0 for whole bars) are lap spliced by `lap` (default 40 diameters), neighbouring splices staggered.

```text
{"command": "beam_create", "args": {"start": [2.5, 0], "end": [0, 2.5], "radius": 2.5}}
{"command": "rebar_create", "args": {"host": 42, "layout": "bottom", "count": 6}}
{"command": "rebar_create", "args": {"host": 42, "layout": "sides", "spacing": 0.15}}
{"command": "rebar_create", "args": {"host": 42, "layout": "stirrups", "spacing": 0.12}}
```

With `anchor` a beam's top, bottom and side bars run on into the columns and pile caps its ends frame into, along their own line or circle: up to the support's far face less the cover, at most the anchorage length past its near face (`true` for 40 bar diameters, or a number of diameters from 10 to 100). Where the support is too shallow for that, top and bottom bars end in a 90° hook whose leg makes up the rest, at least 8 diameters and inside the support's height less the cover: `anchor_hook` `auto` (top bars down, bottom bars up), `down`, `up` or `none`; side bars stay straight. Where another beam carries on in line beyond the support (a continuous beam, or the arcs of a ring beam), the bars of both run half a lap past its middle, so they lap by `lap` inside it, or reach its far face less the cover when it is shallower. The anchorage counts in the bars' length (and so in bar marks and schedules) before they are spliced; `lap_at_joins` still acts where a beam joins another without a column. `rebar_typical` takes `anchor` for the beams' top and bottom bars, and `element_set` changes a set's `anchorage` (`{"length": 30, "hook": "none"}`, or `null`):

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "bottom", "count": 3, "anchor": true}}
{"command": "rebar_create", "args": {"host": 42, "layout": "top", "anchor": 50, "anchor_hook": "down"}}
```

A column that continues a cylindrical wall has an `annular_sector` profile; `rebar_create` then takes its bars face by face (the faces are listed, in order, in the column's `derived.faces`), ties that follow the outline, cross-ties and lifts:

```text
{"command": "profile_create", "args": {"name": "P1", "material": "concrete", "shape": "annular_sector", "params": {"r_in": 2.0, "r_out": 2.2, "sweep": 30}}}
{"command": "profile_type_create", "args": {"profile": "P1", "category": "column"}}
{"command": "column_create", "args": {"type": "P1", "position": [0, 2.1]}}
{"command": "rebar_create", "args": {"host": 42, "layout": "perimeter", "faces": [7, 3, 7, 3], "same_type": true}}
{"command": "rebar_create", "args": {"host": 42, "layout": "stirrups", "spacing": 0.15}}
{"command": "rebar_create", "args": {"host": 42, "layout": "ties", "spacing": 0.30}}
```

Cross-ties run from every bar between the corners to the bar opposite. `tie_faces` limits them to the bars of some faces, by their name or index in `derived.faces` (an annular sector's `outer_arc`, `end_side`, `inner_arc` and `start_side`): in an annular sector `["outer_arc", "inner_arc"]` ties only the bars of the outer and inner arcs to each other, leaving out the long tie between the middle bars of the two radial faces, which the stirrup already holds:

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "ties", "spacing": 0.30, "tie_faces": ["outer_arc", "inner_arc"]}}
```

A column's vertical bars cut in lifts (`max_length`, or `lifts` at given heights) are all spliced at the same heights; `stagger` splices every other bar around the section higher: `half_lap` (half the lap length) or an offset in meters, at most `(max_length - lap) / 2`. Each bar still runs from the base to the top, so a staggered bar's first piece is longer by the offset and its last one shorter (a splice moved to less than a lap below the top is left out); with `max_length` the lifts are shortened where the longer first piece would not fit the stock. The staggered pieces have their own bar marks. `element_set` changes it (`{"splice": {"stagger": "half_lap"}}`, `{"offset": 0.3}` or `"none"`):

```text
{"command": "rebar_create", "args": {"host": 42, "layout": "perimeter", "count": 8, "max_length": 4, "lap": 0.5, "stagger": "half_lap"}}
```

`wall_rebar_create` reinforces walls, straight or arc: horizontal (ring) bars with a spacing or `zones` by height, vertical bars with bent ends and additional sets, and ties, near both faces with a cover each. A cylindrical wall gets its four bar families in one command:

```text
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "cover_exterior": 0.03, "cover_interior": 0.04, "horizontal": {"type": "Barra Ø 10,0 mm", "zones": [{"up_to": 2, "spacing": 0.10}, {"up_to": 4, "spacing": 0.12}, {"up_to": 6, "spacing": 0.15}]}, "vertical": {"spacing": 0.20, "bottom": {"across": 0.12, "along": 0.60}}, "max_length": 5}}
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "vertical": {"spacing": 0.20, "offset": 0.10, "to": 1.5}}}
{"command": "wall_rebar_create", "args": {"walls": [12, 13], "ties": {"every_horizontal": 1, "every_vertical": 2}}}
```

`slab_rebar_create` reinforces floors and foundation slabs of any boundary: a mesh, or ring and radial bars, clipped to the true boundary and around openings, with trimming bars. A circular slab with a manhole, reinforced top and bottom:

```text
{"command": "floor_create", "args": {"structural": true, "curves": [{"kind": "circle", "center": [0, 0], "radius": 2.0}, {"kind": "rectangle", "corner": [-0.3, -0.3], "opposite": [0.3, 0.3]}]}}
{"command": "slab_rebar_create", "args": {"slabs": [42], "type": "Barra Ø 8,0 mm", "spacing": 0.15, "cover": 0.03}}
{"command": "slab_rebar_create", "args": {"slabs": [42], "layout": "radial", "layers": "bottom", "spacing": 0.20, "radial_count": 24, "intermediate": 1.0}}
{"command": "element_get", "args": {"id": 43}}
```

A ring footing under a cylindrical wall: an inverted-T strip type, an arc strip, its reinforcement, and a circular raft that keeps its true curve:

```text
{"command": "type_set", "args": {"type": "Viga baldrame 20 x 40 cm", "params": {"width": 1.6, "thickness": 0.4, "haunch": {"edge_thickness": 0.2, "beam_width": 0.4, "beam_height": 0.5}}}}
{"command": "strip_footing_create", "args": {"start": [3, 0], "end": [-3, 0], "bulge": 1}}
{"command": "strip_rebar_create", "args": {"strips": [42], "transverse_spacing": 0.20, "longitudinal_spacing": 0.20, "beam": {"top": 3, "sides": 1, "stirrups": 0.15}}}
{"command": "foundation_slab_create", "args": {"curves": [{"kind": "circle", "center": [0, 0], "radius": 2.5}]}}
```

`pile_cap_rebar_create` reinforces pile caps (the given `caps`, or the caps of a `level` or of the project that have no reinforcement yet), one set per family in one undo step, laid out from each cap's layout so that they follow its type: `bottom` bars over the piles (a band of `count` bars between each two piles of a three- or four-pile cap, along the sides of the triangle or the square, and with `diagonals` across a square's diagonals; `count` bars along the line of a two-pile or line cap; a mesh `spacing` apart in a single-pile, round or ring cap), lying on the piles' heads, their ends bent up the side faces by `bend`; a `top` mesh; `sides` loops around the outline; and `stirrups` across a two-pile or line cap. Each family takes its values or `false`; caps too small for the cover come back in `skipped`. `pile_rebar_create` reinforces piles (`piles`, the piles of `caps`, or those of a level or of the project without reinforcement): `count` bars (or `spacing`) around the section from `length` below the head (0: the whole pile) up to `projection` above it, at most to the cap's top less the cover, and a spiral of `pitch` (its exact coil length in the bending schedule) or ties (`transverse`). A three-pile cap and its piles:

```text
{"command": "pile_cap_type_create", "args": {"layout": "triangle", "spacing": 1.2, "edge_distance": 0.4, "thickness": 0.8}}
{"command": "pile_cap_create", "args": {"position": [0, 0], "type": 42}}
{"command": "pile_cap_rebar_create", "args": {"caps": [43], "type": "Barra Ø 12,5 mm", "cover": 0.05, "bottom": {"count": 4, "bend": 0.3}, "sides": {"spacing": 0.2}}}
{"command": "pile_rebar_create", "args": {"caps": [43], "count": 6, "length": 6.0, "transverse": "spiral", "pitch": 0.1, "cover": 0.05}}
{"command": "bending_schedule_create", "args": {}}
```

`bracing_create` braces one bay of a frame: a host element whose braces are ordinary beams, made in one undo step and returned as `{id, members}`. The bay is `start` to `end` (the two column axes) or the two `columns`, whose base and height it then takes; `pattern` is `x`, `diagonal` (with `flip` and `alternate`), `v`, `chevron` or `k`; `stories` storeys of `height` stand one above the other from `base_offset`; `material` (`steel` or `timber`) picks the catalog row of the default brace type. `element_set` changes its parameters and regenerates the braces with their ids kept.

```text
{"command": "bracing_create", "args": {"start": [0, 0], "end": [6, 0], "pattern": "chevron", "height": 3, "stories": 4}}
{"command": "bracing_create", "args": {"columns": [12, 13], "pattern": "k", "stories": 2}}
{"command": "element_set", "args": {"id": 42, "params": {"pattern": "X"}}}
```

### Sheets and title blocks

`sheet_set` changes a sheet: `number`, `size` (`"A0"`…`"A4"`, `"ANSI A"`…`"ANSI E"`, `"ARCH A"`…`"ARCH E1"`, `"ARCH E"`, or `{"width": …, "height": …}` in millimeters), `portrait`, `title_block` (a title block type, or `null` for none), `fields` (the sheet's values of the title block's per-sheet fields; `null` clears one) and `issue_date` (`YYYY-MM-DD`, or `""`). `sheet_fields` shows what a sheet's title block will read: every field with its value, the per-sheet fields with this sheet's value and the default, and the pieces that do not resolve. `project_info_set` takes `fields` (project fields by name, read as `{project:Name}`).

A title block is a component whose 2D pieces draw the whole sheet, in millimeters from the paper's bottom-left corner. `title_block_create` copies the standard one (or starts `empty`); `component_sheet_piece_add`, `component_sheet_piece_set` (a merge patch over the piece as `component_get` shows it under `def.sheet`) and `component_sheet_piece_delete` edit its strokes, texts and pictures; several pieces are added (`pieces`), deleted (`indices`) or patched and added (`add`) in one step. A picture is not patched: `component_sheet_piece_set` replaces it with `image_path` (a file) or `image` (base64), and `component_get` and `type_get` show it only as a summary (`format`, `width`, `height`, `bytes`), never as base64. `title_block_create` refuses a name already used by a type or a component. The category is `titleblock` (`types_list`, `categories_list`). `type_delete` may delete the last title block type: its sheets then have no title block, and a sheet without one is drawn as blank paper. Expressions read the parameters and `SheetWidth`/`SheetHeight`, so one title block serves every paper; texts hold `{fields}` (`sheet_number`, `sheet_name`, `sheet_count`, `sheet_index`, `sheet_scale`, `paper`, `project_name`, `project_number`, `client`, `address`, `author`, `date`, `issue_date`, `project:<Name>`, `param:<Name>`). A per-sheet field is a text parameter with `instance`.

```text
{"command": "title_block_create", "args": {"name": "Office"}}
{"command": "component_param_add", "args": {"type": "Office", "name": "Drawn", "kind": "text", "instance": true, "text": "Team"}}
{"command": "component_sheet_piece_add", "args": {"type": "Office", "piece": {"kind": "text", "x": "SheetWidth - 60", "y": 8, "text": "{param:Drawn} · {sheet_index}/{sheet_count}", "size": 2.5}}}
{"command": "component_sheet_piece_add", "args": {"type": "Office", "piece": {"kind": "image", "x": 15, "y": 15, "width": 30, "height": 12, "image_path": "logo.png"}}}
{"command": "view_create", "args": {"kind": "sheet", "name": "Plans"}}
{"command": "sheet_set", "args": {"sheet": "Plans", "title_block": "Office", "size": "A1", "issue_date": "2026-11-06", "fields": {"Drawn": "Ana"}}}
{"command": "sheet_fields", "args": {"sheet": "Plans"}}
```

**References between steps.** Scripts have no variables, but an argument can take a value from the result of an earlier step: `{"$step": n}` is the `id` that step `n` (counted from 1) returned, and `{"$step": n, "path": "/created/0"}` any value of its result, named by a [JSON pointer](https://www.rfc-editor.org/rfc/rfc6901). `{"$level": "active"}` is the active level: in the app, the level of the active plan; with `ccad script`, the level given as `level=<id|name>`; without either, the lowest level. A reference to a step that has not run yet, or to a value its result does not have, stops the script with an `invalid_arguments` error.

```text
{"command": "wall_create", "args": {"level": {"$level": "active"}, "start": [0, 0], "end": [6, 0]}}
{"command": "window_create", "args": {"wall": {"$step": 1}, "offset": 2, "sill": 1}}
```

```sh
ccad -p house.ccad --save script walls.ccmacro level="Level 1"
```

Commands also help by accepting positions where it matters (`door_create` and `window_create` find the wall nearest `point`; floors, ceilings and roofs follow the walls with `from_walls`). For logic between steps, write a [Lua script](#lua-scripts), call `ccad run` from a shell script or use an agent over MCP.

### Bar marks and bending schedules

Every reinforcing bar has a mark naming its shape (the same diameter, grade, legs and bends within the length tolerance, 5 mm by default, and 1 degree): `bar_marks_list` lists them with their shapes (`code`, e.g. `L 30-470`), counts, lengths, weights and sets. Shapes are numbered by partition, then diameter and length from the largest; `element_set` gives a set its own `bar_mark`; `bar_marks_update` freezes the marks shown so they stay when the model changes, and `bar_marks_renumber` starts again (`prefix`, `start`, `keep_overrides`). `bar_marking_set` changes the tolerance and the `stock_length` bars are bought in, which `bar_length_check` uses to list the bars that need splices. `bending_schedule_create` makes a schedule with a row per mark (a sketch of the shape on sheets, its code in CSV) and a summary per diameter (`partitions`, `group_by_partition`, `waste_percent`); `bar_tag_create` writes a set's count, mark, diameter, spacing and length in a plan, elevation or section.

```text
{"command": "bar_marks_update", "args": {}}
{"command": "bending_schedule_create", "args": {"partitions": ["WALLS"], "waste_percent": 10}}
{"command": "bar_tag_create", "args": {"view": 45, "set": 120, "position": [1, 1.5], "multiplier": 2}}
{"command": "bar_length_check", "args": {"max_length": 12}}
```

A floor plan, elevation or section draws the reinforcement over the model with `view_set` `rebar`: the bars its cut plane crosses as dots as wide as the bar, and the rest of the bars within its range as thin lines (a plan down to the level below, a section up to its depth and within its line). Bar tags draw the set they tag within the same range:

```text
{"command": "view_set", "args": {"view": 46, "rebar": true}}
```

### Tags

Plans tag doors, windows, rooms, columns, beams, pad footings and pile caps by themselves. A tag type (`tag_type_create`, `target` and a partial `style`) says what a tag writes and how it looks: up to 8 `lines` of text with `{Field}` placeholders (a schedule field by name, such as `{Width}`, `{Height}` or `{Comments}`, or `{Mark}`, `{TypeName}`, `{Name}`, `{Number}`, `{Area}`, `{Section}`, `{Level}`) and a size in paper millimeters, a `symbol` (`Circle`, `Hexagon`, `Rectangle`, `RoundedRectangle`, `Ellipse`, `Diamond`, `Triangle`, `Underline`, `None`, or `{"Component": id}` of a detail component drawn at paper size, its texts' fields filled in), its `width` and `height` (0 fits the text), pens, `leader` and `orientation`; `type_set` changes it. The template has one tag type per target, of the default style (the tags plans always drew); a project with no tag type of a target (an older file) draws its tags in that style too. Each plan picks a tag type per target (`view_set` `tag_types`) and adjusts the tag of each element with `tag_set` (`offset` or `delta`, `rotation`, `type`, `hidden`, `leader`, `reset`; `null` clears a field); `tag_get` shows the adjustments, the type and the texts.

```text
{"command": "tag_type_create", "args": {"target": "door", "name": "Door size", "style": {"lines": [{"text": "{Mark}", "size": 2.5}, {"text": "{Width} x {Height}", "size": 2}], "symbol": "Rectangle"}}}
{"command": "view_set", "args": {"view": "Térreo", "tag_types": {"door": "Door size"}}}
{"command": "tag_set", "args": {"view": "Térreo", "ids": [12, 14], "delta": [0, 0.5], "rotation": 90}}
{"command": "tag_get", "args": {"view": "Térreo", "id": 12}}
```

Grids and the marks of sections and elevations are styled the same way. A grid type's `style` (`type_set` `params.style`) sets the `ends` with a bubble (`End`, `Start`, `Both`, `None`; a grid's own `ends`, through `element_set` `params`, comes first), the bubble's `symbol` (a shape, or a detail component whose texts write `{Name}`), `text_size`, `pen`, `color` and `line_pattern`. Each plan, elevation or section moves a grid's bubbles with `tag_set` (`offset` or `delta` for the end bubble, `start_offset` for the start one, each drawn with an elbow back to the line) or hides them (`hidden`; the line stays). A view mark type (`view_mark_type_create`, `kind` `section` or `elevation` and a partial `style`) draws the heads of sections and the markers of elevations: the `head` size in paper millimeters (0: 0.5 m in the model, as elevation markers always were), its `symbol`, a `split` line, the `top`, `bottom` and `label` texts with `{Number}`, `{Sheet}`, `{Name}` and `{Letter}`, the `arrow`, the section's `heads`, its `line_weight` and the colors. A section or an elevation is marked with its own type (`view_set` `mark_type`), else the project's first of its kind (the template has one of each, of the default style), else as plans always marked it. `mark_move` moves an elevation's marker in one floor plan (`delta`, `offset` or `reset`).

Elevations and sections draw every level as a line with a head at its right end. `level_head_set` adjusts them per view: `hidden` hides levels in that view only, `heads` puts their heads at the `end`, the `start`, `both` ends or `none`, `elbow` and `start_elbow` move a head up (negative: down) from its line, meters in the view, with an elbow leader, and `reset` puts them back as by default; `view_get` lists them under `level_heads`. The category `level` of `view_graphics_set` hides all of them or overrides their lines.

```text
{"command": "type_set", "args": {"type": "Grid 8 mm", "params": {"style": {"ends": "Both", "symbol": "Hexagon", "text_size": 4}}}}
{"command": "tag_set", "args": {"view": "Térreo", "ids": [30], "offset": [1.5, 0], "start_offset": [-1.5, 0]}}
{"command": "view_mark_type_create", "args": {"kind": "section", "name": "Big head", "style": {"head": 16, "top": {"text": "{Letter}", "size": 4}}}}
{"command": "view_set", "args": {"view": "Section A", "mark_type": "Big head"}}
{"command": "mark_move", "args": {"plan": "Térreo", "view": "Elevação: Sul", "delta": [0, -2]}}
```

### Photorealistic rendering

A 3D view keeps its render settings in `render` (`view_get`), changed by `view_set` with a merge patch: `samples` per pixel, light bounces (`max_depth`), the image's `width` and `height` in pixels, `fov_deg`, `exposure` in stops, `tone_map` (`Filmic` or `Standard`), the firefly `clamp`, `clay` (every opaque surface one white matte, glass kept, under the same sky and sun) and the `environment` (`sky_strength`, `sun_strength`, `ground_color`, `ground_plane`); `null` removes them. The sun is the view's (`sun`). `view_render` path traces the view on the processor and writes the image (`.png`, `.jpg`, or linear `.exr`/`.hdr`) with the view's camera, as the 3D view shows it; `settings` patches the view's settings for that render only, and `region` (`[x, y, width, height]` in pixels) renders only part of the image. It returns the size, the samples, the seconds and the triangles, and changes nothing in the model.

```text
{"command": "view_set", "args": {"view": "{3D}", "render": {"samples": 256, "width": 1920, "height": 1080}}}
{"command": "view_render", "args": {"view": "{3D}", "path": "house.png"}}
{"command": "view_render", "args": {"view": "{3D}", "path": "clay.jpg", "settings": {"clay": true, "samples": 64}}}
```

```sh
cargo run -p cc_cli -- -p examples/casa_simples.ccad run view_render view="{3D}" path=out.png 'settings={"samples":16,"width":320,"height":200}'
```

## Lua scripts

`ccad lua <file>` runs a [Lua 5.4](https://www.lua.org/manual/5.4/) script over the session. The script calls commands with `ccad.run`, keeps their results in variables and loops, so an id created by one step can be used by the next. Its arguments (the same forms as `run`: `key=value` pairs, a JSON object, `@file.json`) are `ccad.args`; what it prints goes to standard output and what it returns is printed as JSON.

[`examples/small_house.lua`](../examples/small_house.lua) builds the small house above, sized by its arguments:

```lua
local w = ccad.args.width or 8
local d = ccad.args.depth or 6
local corners = { { 0, 0 }, { w, 0 }, { w, d }, { 0, d } }
local outer = {}
for i = 1, 4 do
  outer[i] = ccad.run("wall_create", { start = corners[i], ["end"] = corners[i % 4 + 1], height = 2.8 }).id
end
ccad.run("door_create", { wall = outer[1], offset = w / 4 })
…
```

```sh
ccad --save-as house.ccad lua examples/small_house.lua width=10 depth=7
```

The `ccad` table:

| | |
|---|---|
| `ccad.run(name, args)` | runs a command and returns its result as a Lua table; raises an error if it fails |
| `ccad.try(name, args)` | the same, but returns `nil, err` on failure (`err.code`, `err.message`; see [Errors](#concepts)) |
| `ccad.commands()` | the commands: `name`, `kind`, `group`, `description` each |
| `ccad.describe(name)` | one command, with the JSON Schema of its parameters as `schema` |
| `ccad.args` | the script's arguments (also `...`) |
| `ccad.null` | JSON `null` (Lua's `nil` leaves a field out) |
| `ccad.array(t)` | marks `t` as a list, so that an empty table is sent as `[]` rather than `{}` |
| `ccad.json.encode(v)`, `ccad.json.decode(s)` | JSON text to and from Lua values |
| `ccad.version`, `ccad.api` | the ConstructCraft version and the plugin API version (2) |

Arguments and results are JSON: objects are tables with string keys, arrays are sequences (`{ 0, 0 }` is a point), `end` is a Lua keyword and is written `["end"]`. A whole number computed with `/` (which always gives a float in Lua) is sent as an integer, so `10 / 2` works where an id or count is expected.

**The sandbox.** Scripts run in a restricted Lua: the `table`, `string`, `math`, `utf8` and `coroutine` libraries and the base functions, without `io`, `os`, `debug`, `package`, `require`, `load`, `dofile` or `loadfile`. A script reaches the project only through commands, and files only through the commands that read or write them. A run has a budget of 500 million Lua instructions, 60 seconds and 256 MB of memory (string functions count by the size of their text, pattern matching by its steps, and `string.rep` and `string.gsub` make at most 16 MB); past them it stops with a `script` error (catching that error with `pcall` does not keep the script going). Each command a script runs is its own undo step.

## Visual programming graphs

A graph (`.ccgraph`, JSON) is a set of nodes joined by wires and run as a
dataflow. **Every engine command is a node**: its parameters are the
node's inputs and its result the `result` output (plus `element`, the id,
for `*_create` commands). Built-in nodes give input values (`number`,
`integer`, `slider`, `boolean`, `text`, `select_elements`,
`elements_of_category`, `levels`), math, logic, lists, geometry as the values
commands take (points `[x, y]`, `line`, `arc`, `circle`, `rectangle`,
`rectangle_points`, `polygon_points`, `polyline_segments`, `divide_line`…),
element parameters (`parameter_get`, `parameter_set`) and outputs (`watch`,
`output`). A Lua node runs a function of its inputs in the sandbox (no
access to the model); it returns its outputs in order, or one table with
their names. `graph_nodes` lists every node with its ports and types.

- **Lists map over nodes.** A node given a list where it takes one value runs
  once per item and its outputs become lists (`lacing`: `shortest`,
  `longest` or `cross`). A list of 4 start points and 4 end points into
  `wall_create` makes 4 walls.
- **Running again updates.** Each element a node creates is bound to the
  node and the item that made it. The next run changes it in place (same id,
  persistent id, mark and comments), makes the new ones and deletes the ones
  no longer made; with nothing changed, nothing happens. A run is one undo
  step. `graph_elements` lists a graph's elements, `graph_release` frees
  them (the next run makes new ones).
- **Order.** A node runs after the nodes wired into it; nodes that do not
  depend on each other run in the order they were added (lowest id first).
- **Errors are per node.** A failing node shows its message, the nodes after
  it are skipped, the others run, and its elements stay as they were.
- A graph never runs `project_new`, `project_open`, `undo`, `redo`,
  `language_set` or graphs, and runs file commands only when whoever runs it allows them: `graph_run`'s
  `allow_files`, `ccad --allow-files graph`, or the editor's *Allow file
  commands* checkbox, which lasts until another graph is opened. A graph
  file cannot allow them itself (an `allow_files` field in it is ignored),
  and a graph a plugin runs never may.

```sh
ccad --save-as house.ccad graph examples/graphs/house.ccgraph width=12 depth=8
ccad run graph_check path=examples/graphs/house.ccgraph
ccad run graph_nodes search=wall
```

Input nodes with a `name` are the graph's inputs (`key=value` above,
`inputs` of `graph_run`); `output` nodes with a `name` are its results. A
minimal graph:

```json
{
  "uid": "6f1c2a0e9b3d4c8a8e2f5a7b1c3d9e02",
  "nodes": [
    { "id": 1, "kind": { "builtin": "range" }, "values": { "end": 15, "step": 5 } },
    { "id": 2, "kind": { "builtin": "point" } },
    { "id": 3, "kind": { "builtin": "point" }, "values": { "y": 4 } },
    { "id": 4, "kind": { "command": "wall_create" }, "values": { "height": 2.8 } }
  ],
  "wires": [
    { "from": [1, "list"], "to": [2, "x"] },
    { "from": [1, "list"], "to": [3, "x"] },
    { "from": [2, "point"], "to": [4, "start"] },
    { "from": [3, "point"], "to": [4, "end"] }
  ]
}
```

## Plugins

A plugin is a folder with a `plugin.toml` manifest and Lua code that adds commands to the catalog. Its commands are listed and run like the built-in ones, by `ccad run`, scripts, Lua scripts and MCP agents, and are named `<plugin id>__<command>` (two underscores, which built-in names never contain).

```sh
ccad --plugins examples/plugins commands                # the catalog, with the plugins' commands
ccad --plugins examples/plugins plugins                 # the plugins: permissions, commands, buttons, settings
ccad --plugins examples/plugins run quick_rooms__rectangle_room width=4 depth=3 name=Office door=south
claude mcp add constructcad -- ccad mcp --plugins /path/to/plugins
```

`--plugins` takes a plugin's folder or a folder of plugin folders, and can be repeated. In a folder of plugins, each plugin's folder is named after its id (as installs name them); hidden folders (starting with `.`) are skipped. A plugin that fails to load is reported on standard error and left out; what plugins print also goes to standard error.

### Writing a plugin

[`examples/plugins/quick_rooms`](../examples/plugins/quick_rooms) is a complete example. `plugin.toml`:

```toml
id = "quick_rooms"          # lowercase letters, digits and single underscores, up to 32
name = "Quick rooms"
version = "0.2.0"
api = 2                     # the plugin API it is written for (1, 2 or 3, see below)
description = "Rectangular rooms in one step, and wall totals by type."
authors = ["…"]
license = "GPL-3.0-or-later"
homepage = "https://…"
main = "main.lua"           # the default
permissions = []            # or ["files"], ["tools"] (api 2), ["network"] (api 3), see below
```

`main.lua` runs once when the plugin loads and registers commands with `ccad.command`:

```lua
local rect = require("lib.rect")   -- lib/rect.lua in the plugin's folder

ccad.command {
  name = "rectangle_room",         -- the command is quick_rooms__rectangle_room
  title = "Rectangular room",
  description = "Four walls around a rectangle, its floor and its room.",
  kind = "edit",                   -- "query", "edit" (the default) or "file"
  params = {                       -- a JSON Schema of the arguments
    properties = { width = { type = "number" }, depth = { type = "number" } },
    required = { "width", "depth" },
  },
  run = function(args)
    local p = rect.corners(0, 0, args.width, args.depth)
    for i = 1, 4 do ccad.run("wall_create", { start = p[i], ["end"] = p[i % 4 + 1] }) end
    return { room = ccad.run("room_create", { point = { args.width / 2, args.depth / 2 } }).id }
  end,
}
```

- `run` gets the arguments as a table and returns the command's result (a table, a value or nothing). Missing `required` arguments are refused before it runs; `error("message", 0)` fails the command with that message.
- **One undo step:** however many commands a plugin command runs, it is undone at once, and when it fails (an error, a failed `ccad.run`, a limit reached) everything it did is undone.
- **What a command may run:** a `query` command only queries; an `edit` command queries and edits; commands that read or write files (import, export, save) need the `files` permission (without it, edit commands that read a file a parameter names, such as `cad_import` or `fill_pattern_import` with a `path`, refuse the path; their inline forms, such as `fill_pattern_import` with `text`, still work), and a command of kind `file` can only be declared with it. Without `files`, a command still works with the files its arguments name in `save_path` and `open_path` parameters ([API 2](#files-without-the-files-permission)). Opening or creating a project, `undo`, `redo` and `language_set` cannot run inside a plugin command, nor can another plugin's command.
- **These rules hold for everything a command runs:** the engine checks every command run while a plugin command runs (its scope), also the nodes of a graph it runs with `graph_run`; a graph's `plugin_data_set` node for another plugin's id is refused like a direct `ccad.run`. Lua scripts the user runs (`ccad lua`) are not restricted.
- **The sandbox** is that of [Lua scripts](#lua-scripts), with the budget counted per command; each plugin has its own Lua state, which keeps its variables between commands. `require("a.b")` loads `a/b.lua` from the plugin's folder (once; files outside the folder are refused), and `ccad.plugin` holds its `id`, `name` and `version`. `ccad.run` works only while a command runs, not while the plugin loads.
- **Permissions** are granted by whoever loads the plugin: `ccad --plugins` grants what the manifest asks for, since you named the folder. `files` lets its commands work with any file; `tools` (API 2) lets them run the external programs the user chooses for its tools ([External programs](#external-programs)).

### Plugin API 2: buttons, settings, translations and data

A plugin written for `api = 2` (`ccad.api` is 3 in this version, see API 3 below) can also declare ribbon buttons and settings in its manifest, translate its text and give its parameters hints for the interface. Manifests with `api = 1` load as before; buttons and settings are refused under `api = 1` (`buttons need api = 2`), so the index never offers such a plugin to a build that only knows API 1.

```toml
id = "quick_rooms"
name = "%name%"                  # looked up in locales/<language>.toml
version = "0.2.0"
api = 2

[[buttons]]                      # at most 64
command = "rectangle_room"       # a command main.lua registers (its short name)
label = "%button.room%"          # at most 64 characters
icon = "icons/room.svg"          # optional: an SVG in the plugin's folder, at most 64 KiB
panel = "%panel%"                # optional: the ribbon panel (default: the plugin's name)
size = "large"                   # "large" (the default) or "small"
when = "view:floor_plan"         # optional: when the button is available

[settings.default_height]        # at most 64; keys like command names
type = "number"                  # boolean, number, integer or string
default = 2.8                    # required, and must fit the setting
title = "%settings.height%"      # optional (default: the key made readable, "Default height")
description = "Height of the walls it draws"
ui = "length"                    # optional, numbers only: shown in the project's units
minimum = 2.0                    # numbers and integers; also maximum

[settings.default_door]
type = "string"
default = "none"
enum = ["none", "south", "east", "north", "west"]   # strings only: the allowed values
enum_labels = ["%side.none%", "%side.south%", "%side.east%", "%side.north%", "%side.west%"]   # optional, one per value
```

A button whose command `main.lua` does not register is a load error naming the command. **`when`** is a comma-separated list of terms that must all hold:

| Term | Holds when |
|---|---|
| `selection` | at least one element is selected |
| `selection:one` | exactly one element is selected |
| `selection:<category>` | at least one element is selected and all are of the category (`wall`, `door`…; the names of `categories_list`) |
| `view:<kind>` | the active view is a `floor_plan`, `ceiling_plan`, `three_d`, `elevation`, `section`, `legend`, `drafting`, `schedule`, `diagram` or `sheet` |

An unknown term, category or view kind is a load error.

**Settings.** `ccad.settings` holds the current value of each setting (the stored value, or the default): it is read-only (assigning to it raises an error), is there while `main.lua` runs, and is refreshed before each command runs. Settings are listed in the manifest's order. The values are kept per plugin in `<plugins folder>/.settings/<id>.json`; stored values that no longer fit the manifest (an unknown key, a wrong type, a value out of `enum`, `minimum` or `maximum`) are dropped when the plugin loads, and a plugin loaded from its own folder (not a folder of plugins) starts with the defaults. Removing a plugin removes its settings. From the command line:

```sh
ccad plugins settings ~/plugins quick_rooms                         # the settings and their values, as JSON
ccad plugins settings ~/plugins quick_rooms default_height=3 default_door=north
```

Values are written like `run`'s `key=value` arguments (JSON or plain text; a string setting takes the text as it is), checked against the manifest and stored; a value that does not fit is refused and nothing is written. This reads the manifest and the translations only: the plugin's code does not run.

**Translations.** `locales/<code>.toml` files in the plugin's folder, `<code>` one of ConstructCraft's languages (`en`, `pt-BR`, `es`, `fr`, `de`, `it`, `lo`, `pl`, `uk-UA`, `ru`, `zh-TW`, `zh-CN`, `ja`; other files are ignored), hold `key = "text"` lines or tables whose keys join with `.` (each file at most 256 KiB; a file that is not TOML of texts is a load error):

```toml
# locales/pt-BR.toml
name = "Salas rápidas"
[button]
room = "Sala"                 # the key button.room
[settings]
height = "Altura padrão"
```

A text that is exactly `%key%` is looked up when it is shown, in the current language: that language's file, then one of the same base language (`pt` for `pt-BR`), then `en`; a key found nowhere shows as the key. This applies to the manifest's `name` and `description`, buttons' `label` and `panel`, settings' `title`, `description` and `enum_labels`, commands' `title` and `description`, and the `title`, `description` and `enum_labels` of each parameter. Changing the language changes the text at once, without reloading plugins. Under `api = 1` texts are shown as written.

**Parameter hints.** A parameter of `params` may say how the interface asks for it; with `api = 2` the hints are checked when the command registers (a wrong one is a load error), and they are passed on to MCP clients unchanged:

| `ui` | Type of the parameter | Asks for |
|---|---|---|
| `length` | `number` | a length, in the project's units |
| `element` | `integer` | an element id (`category` narrows it) |
| `elements` | `array` | element ids (`category` narrows them) |
| `point` | `array` | a point `[x, y]` |
| `level` | `integer` | a level id |
| `view` | `integer` | a view id |
| `type` | `integer` | a type id (`category` narrows it) |
| `material` | `string` | a material name |
| `save_path` | `string` | a file to write (the app's save dialog); the command may write it |
| `open_path` | `string` | a file to read (the app's open dialog); the command may read it |

`category` (a category name) goes only with `element`, `elements` and `type`; `extensions` (1 to 16 lowercase extensions of 1 to 16 letters or digits, without the dot, such as `{ "idf", "epw" }`) only with `save_path` and `open_path`; `enum_labels` gives a label to each value of `enum` (as many strings); `title` is a short name for the parameter.

```lua
params = { properties = {
  width = { type = "number", ui = "length", title = "%param.width%" },
  wall = { type = "integer", ui = "element", category = "wall" },
  side = { type = "string", enum = { "north", "south" }, enum_labels = { "%north%", "%south%" } },
} }
```

**Plugin data.** A plugin keeps its own data in the project, on elements or on the project itself, saved in the `.ccad` file (the commands `plugin_data_get`, `plugin_data_set` and `plugin_data_list` of the [command reference](commands.md)):

| | |
|---|---|
| `ccad.data.get(element)` | what the plugin keeps on the element (`nil`: on the project), or `ccad.null` |
| `ccad.data.set(element, value)` | keep `value` (any JSON, up to 256 KiB) there, replacing what was there; `nil` removes it |
| `ccad.data.list()` | `{ project = true/false, elements = { ids… } }` |

`ccad.data.set` is an edit, so a `query` command cannot call it, and it is part of the command's one undo step. A plugin command reaches only its own data: running a `plugin_data_*` command through `ccad.run` or `ccad.try` with another plugin's id is refused (`a plugin reads and writes only its own data`). Lua scripts the user runs (`ccad lua`) reach any plugin's data.

#### Files without the `files` permission

A command that writes or reads a file the user chooses does not need the `files` permission: it takes the path in a `save_path` (to write) or `open_path` (to read) parameter, and while it runs, that path, and only it, is granted:

- **Before it runs**, each top-level `save_path`/`open_path` argument is checked: a string, not empty, with one of the parameter's `extensions` when it lists them (in any case: `MODEL.IDF` is an `idf`), a file to read that exists, a file to write in a folder that exists. A path that fails is refused (`invalid_arguments`) and the command does not run. Paths are compared as files, not as text: `a/../model.idf` and the absolute path of `model.idf` are the same file.
- **`ccad.file.read(path)`** returns the bytes of a granted `open_path` file as a Lua string (at most 64 MiB). **`ccad.file.write(path, data)`** writes a granted `save_path` file; `data` is a string or a list of strings, joined (for large outputs, build a list rather than one long string). A command writes at most 256 MiB in all. The direction is checked: a `save_path` file cannot be read, nor an `open_path` one written, and any other path is refused (`rejected`). `ccad.file` exists only in plugins, and works only while a command runs.
- **Writes are kept until the command succeeds:** what `ccad.file.write` gives is held in memory and written only when the command ends without error, each file to a temporary file in its folder, then renamed over the target. A command that fails leaves no file and no partial file, and an existing file unchanged.
- **The engine's own exports** work with the granted path too: a command may run `ifc_export`, `gltf_export`, `view_export`, `sheet_export`, `schedule_export_csv`, `analytical_export`, `view_render`, `component_save`, `fill_pattern_export` or `pipe_series_export` with its granted `save_path`, and imports such as `cad_import` with its `open_path`; with another path they are refused, and other file commands (`project_save`, `ifc_import`…) still need `files`. A file an engine command wrote stays even if the plugin command fails afterwards (only its model changes are undone).
- A plugin with the `files` permission works with any file, as before; `ccad.file` still reads and writes only the granted paths.

From the command line and MCP the path is an ordinary argument: `ccad --plugins ~/plugins run energy__export_idf path=model.idf`.

#### Exporters and importers

A plugin can add entries to the app's Export and Import lists:

```toml
[[exporters]]                    # at most 16
command = "export_idf"           # a command with exactly one ui = "save_path" parameter
label = "%export.idf%"           # e.g. "EnergyPlus model (.idf)"; at most 64 characters

[[importers]]                    # at most 16
command = "import_weather"       # a command with exactly one ui = "open_path" parameter
label = "%import.epw%"
```

The file types come from the parameter's `extensions`. An entry whose command `main.lua` does not register, or whose command has no such parameter or more than one, is a load error; both need `api = 2`. In the app, clicking the entry opens the save (exporter) or open (importer) dialog for those extensions, then the command's form with the chosen path (or runs the command at once when the path is its only parameter). `ccad plugins` lists them (`exporters`, `importers`: command, label, parameter and extensions).

Plugins are shared through the community index, [constructcraft-plugins](https://github.com/francisreboucas/constructcraft-plugins): one entry per plugin, with where to download each version and its checksum, submitted by pull request. In the app, *Manage → Plugins* installs, updates, turns off and removes plugins ([Plugins](interface.md#plugins)); without it, `ccad plugins install <file.zip> <folder>` installs a downloaded archive (checked the same way: safe paths, sizes, and a plugin that loads) and `ccad plugins remove <id> <folder>` removes one.

#### External programs

A plugin can run a program installed on the computer, such as the EnergyPlus simulator, when the user allows it and chooses which program file it is. The program runs outside the sandbox, with the user's permissions, so nothing runs that the user did not approve, and the app keeps working while it runs.

```toml
permissions = ["tools"]                          # needs api = 2

[[tools]]                                        # at most 8
id = "energyplus"                                # like a plugin id; ccad.process.run names it
name = "EnergyPlus"                              # may be "%key%"
executables = ["energyplus.exe", "energyplus"]   # the program's accepted file names
homepage = "https://energyplus.net"              # optional: where to get it
version_args = ["--version"]                     # optional: what "Test" runs
```

`[[tools]]` without the `tools` permission, or under `api = 1`, is a load error; so are batch files (`.bat`, `.cmd`) and names with folders in `executables`.

**Approval.** Two things are the user's choice, never the plugin's:

- **The permission:** in the app, *Allow it to run external programs* in the Plugins window (kept per plugin, like the `files` permission); `ccad --plugins` grants what the manifest asks for, since you named the folder.
- **The program:** the path of each tool, chosen in the Plugins window (*External programs*: *Browse…*, or *Use …* for a program of that name found on `PATH`, never taken by itself) and kept in `<plugins folder>/.tools/<plugin id>.json` (`{ "<tool id>": "<absolute path>" }`; removed with the plugin). A path is valid only when it is an existing file whose name is one of `executables` (in any case on Windows). From the command line:

```sh
ccad plugins tools ~/plugins energy                         # the tools: path chosen, why it cannot run, a suggestion from PATH
ccad plugins tools ~/plugins energy energyplus=/opt/EnergyPlus/energyplus --test   # choose it, and run version_args
ccad plugins tools ~/plugins energy energyplus=             # forget it
ccad --plugins ~/plugins/energy --tool energy.energyplus=/opt/EnergyPlus/energyplus run energy__simulate
```

`--tool <plugin>.<tool>=<path>` (repeatable) gives a program for one run, also to a plugin loaded from its own folder, which reads no stored paths; a path that does not fit is reported on standard error and left out. `ccad plugins` lists each plugin's `tools` (`id`, `name`, `executables`, `homepage`, `path`, `problem`).

**Running it.** In a command's `run` function:

```lua
local r = ccad.process.run("energyplus", {
  args = { "-w", "in.epw", "-r", "in.idf" },     -- passed as they are, without a shell
  files = { ["in.idf"] = idf_text },              -- written into the run folder first
  copy = { ["in.epw"] = args.weather },           -- a file given to this command to read (open_path), copied there
  outputs = { "eplusout.csv", "eplustbl.htm", "*.err" },  -- read back; * matches any characters within a name
  timeout = 600,                                  -- seconds: default 300, at most 7200
})
-- r = { code = 0, stdout = "…", stderr = "…", truncated = false, timed_out = false,
--       files = { ["eplusout.csv"] = "…", ["eplusout.err"] = "…" } }
```

- **The run folder:** each run gets a new folder of its own in the system's temporary folder (`constructcraft-<plugin>-<random>`), which is the program's working folder; the `files` are written and the `copy` files copied into it, the `outputs` found there are read back (outputs not found are simply missing from `files`; links are not followed), and the folder is deleted afterwards, also when the run fails, times out or is cancelled.
- **Names** in `files`, `copy` and `outputs` are relative to the run folder, plain names joined by `/` (`sub/in.idf`): `..`, absolute paths, drives, `\`, `:` and Windows device names (`CON`, `NUL`, `COM1`…) are refused.
- **No shell:** the program is started directly, and each argument reaches it as written: `&`, `;`, `|`, quotes and `$(…)` mean nothing.
- **Environment:** the program gets only `PATH`, `SystemRoot`, `TEMP`, `TMP`, `HOME`, `USERPROFILE`, `LANG` and `LC_*` from the app's environment; nothing else is passed on. Standard input is empty.
- **Limits:** the inputs (`files` and `copy`) at most 256 MiB in all; each output at most 64 MiB and all outputs at most 256 MiB (beyond, the call raises an error); standard output and error kept up to 1 MiB each (`truncated = true` when either was cut). On timeout the program is killed and `timed_out = true` (no `code`). A non-zero exit code is returned, not raised: the plugin decides what it means.
- **Stopping:** on timeout or cancel the program is killed. Programs it started itself may keep running: no job objects or process groups are used.
- **What approval means:** the plugin decides the arguments and input files, so choosing a general-purpose program for a tool (a shell, a script interpreter) gives the plugin everything that program can do; choose the program the tool is meant for.
- **Errors** raise at the call, like `ccad.run`, and name what to do: no `tools` permission, a tool the manifest does not declare, no program chosen (it names the Plugins window, `ccad plugins tools` and `--tool`), a program that is missing or of another name, a bad name, a limit, a program that could not start. `pcall` catches them.
- **Where it works:** only in a command's own `run` function (also inside `pcall`), not while `main.lua` loads, not in event handlers, not in a coroutine the plugin starts, a module being required or a function a library function calls (such as `table.sort`'s comparator). Lua scripts the user runs (`ccad lua`) have no `ccad.process`.

**The command waits without holding the app.** A command's `run` function runs in a coroutine: `ccad.process.run` hands the program to a background thread and the command waits there. With `ccad`, scripts, macros and MCP the command simply takes as long as the program; in the app it is left pending: a dialog shows the plugin, the program, the time elapsed and the last line it wrote, with *Cancel* ([Plugins](interface.md#plugins)). While it waits:

- it is still one command: its edits before and after the wait are one undo step, a failure (or *Cancel*) undoes all of them, the files it writes with `ccad.file.write` are written only when it ends without error, and `model_changed` fires once, when it ends;
- the session runs only queries: any command that edits, saves, opens or exports, `undo` and `redo`, and other plugin commands are refused (`a plugin command is running`), and no event fires;
- the time spent waiting does not count against the command's 60 seconds: that limit is for Lua; its instruction budget goes on across the wait.

A command may run several programs one after the other; *Cancel* stops the one running and undoes the command (`cancelled`). In the engine, `Session::set_blocking(false)` leaves such commands pending (`execute` returns the code `pending`), `Session::poll_job` goes on with them and returns their result when they end, `Session::job` describes the one waiting, and `Session::cancel_job` stops it; sessions block by default, and commands run by other commands (graphs, scripts) and macro steps are always waited for.

#### Events

A plugin can react to what happens to the project by registering handlers while `main.lua` loads:

```lua
ccad.on("model_changed", function(e)
  local index = ccad.data.get(nil)
  if index == ccad.null then index = { walls = 0 } end
  index.walls = index.walls + #e.added - #e.removed   -- (a real index would look at the categories)
  ccad.data.set(nil, index)                            -- its own data: allowed
end)
ccad.on("project_saved", function(e) print("saved to " .. e.path) end)
```

| Event | When | `e` |
|---|---|---|
| `project_opened` | after `project_open`, `project_new` or an import that replaces the project (also the app's Open and New, but not its Close), and once when the plugins are loaded on a session that already holds a project, so a plugin can start | `path` (`nil` when the project has no file yet), `new` (`true` when it has none) |
| `project_saving` | before `project_save` writes the file (the app's Save and Save As, `ccad --save`) | `path` |
| `project_saved` | after the file was written | `path` |
| `model_changed` | after each command that changed the project: `undo` and `redo` too, and a plugin's command once, when it ends | `command` (the command's name: `"wall_create"`, `"undo"`, `"redo"`, `"quick_rooms__rectangle_room"`…), `added`, `modified`, `removed` (element ids, ascending, at most 10 000 each, with `truncated = true` when a list was cut), and `other = true` when something other than elements changed (types, views, levels, settings, the project's plugin data); then the lists may all be empty |

- **Registration:** `ccad.on(event, handler)` works only while `main.lua` loads (later it raises an error, like `ccad.command`); an unknown event name is a load error; a plugin registers at most 32 handlers. Handlers run plugin by plugin in load order (in a folder of plugins, the folders' name order), then in the order they were registered.
- **Which commands fire them:** the ones the user, a script or an agent runs, from the app, `ccad`, Lua scripts or MCP. What a plugin command runs is part of it (the command fires once, when it ends), and what a handler runs never fires anything, so handlers cannot start a loop. Each step of a macro or script (`ccad script`, `ccad lua`, the macro editor) is a command of its own and fires its own events, as each is its own undo step. Failed commands, queries and edits that change nothing fire nothing.
- **What a handler may do:** query anything, and keep its own data up to date with `ccad.data.set` (`plugin_data_set` for its own id). Anything else that edits (`element_set`…) is refused, and so are files and other plugins' commands. After `undo` and `redo` handlers only query: plugin data is part of the project's history, so undo and redo already bring it back in step with the model (and an edit there would clear what can be redone). What handlers store for `model_changed` joins the undo step of the command that changed the model, so one undo takes back both, and a value being dragged stays one undo step. What they store for `project_opened`, `project_saving` and `project_saved`, which have no such step, is one undo step of its own per plugin, named after the plugin. Neither is recorded in a macro being recorded. Plugin data is saved with the project like the rest: what is stored in `project_saving` is in the file, and storing something in `project_opened` or `project_saved` leaves unsaved changes.
- **Budget:** each handler call may take at most 50 million Lua instructions and 5 seconds (and the plugin's memory limit).
- **Failures never fail the command:** an error or a budget overrun in a handler becomes a message (`[id] model_changed handler 1: …`, on standard error with `ccad`, in the status bar and the log in the app), and what that call stored is dropped. A handler that fails 3 times in a row is turned off until the plugins load again.
- **Cost:** the lists of `model_changed` come from the project's undo machinery: while some plugin listens to it, each command notes the elements it touches as they were before it (once each), and compares them with what they are after; no geometry is rebuilt, and nothing is noted while no plugin listens.

### Plugin API 3: network access and component catalogs

A plugin written for `api = 3` (`ccad.api` is 3 in this version) can also download data over HTTPS and offer a catalog of components in the Component Library window. Manifests with `api = 1` or `2` load as before; `network`, `allowed_hosts` and `[[catalogs]]` are refused below `api = 3`.

```toml
api = 3
permissions = ["network"]                              # needs api = 3
allowed_hosts = ["example.org", "*.cdn.example.org"]   # at most 16

[[catalogs]]                                           # at most 8
id = "furniture"                                       # like a plugin id
label = "Furniture"                                    # the tab's name; may be "%key%"
description = "Models from example.org"                # optional, shown in the sidebar
```

**The `network` permission.** It needs at least one host in `allowed_hosts`: lowercase host names, or `*.` and a domain for its subdomains (not the domain itself); no scheme, port, path, `localhost` or IP address. In the app it is the user's choice, *Allow it to access the network* in the Plugins window (kept per plugin, off by default, showing the hosts); `ccad --plugins` grants what the manifest asks for, since you named the folder.

**`ccad.http`.** Only `https://` URLs whose host is in `allowed_hosts` are fetched, checked again on every redirect (at most 5, to absolute `https://` URLs or paths on the same host); URLs with credentials or a port are refused, responses are at most 64 MiB, and requests time out (30 s by default, at most 120 s). Without the permission each function is an error. A request waits for its answer: in a command, keep downloads few and small, and leave large ones to a catalog (below), which runs on a thread of its own.

```lua
local body, status = ccad.http.get(url, { max_bytes = 1e6, timeout = 10 })   -- at most 16 MiB by default
local path = ccad.http.download(url, "models/chair.glb", { sha256 = "…" })  -- into the plugin's cache
local path = ccad.http.cached("models/chair.glb", { sha256 = "…" })         -- nil when not there; no network
local text = ccad.http.read_cached("data/index.json", { max_bytes = 1e6 })  -- its bytes, or nil; no network
```

Each plugin has its own cache folder (in the app, `plugin-cache/<plugin id>` in its data folder; for `ccad`, the user's cache folder). Keys are relative paths of letters, digits, `.`, `_` and `-`, at most 5 levels and 160 characters, so a key never leaves the folder. With `sha256` (64 hexadecimal digits), a download is kept only when its SHA-256 matches, and a cached file that no longer matches is deleted and reads as missing; `download` reuses a cached file that matches instead of fetching it again. A plugin command may read files in its own cache folder (to pass them to `gltf_component_load`, for example) without the `files` permission.

**Catalogs.** Each `[[catalogs]]` entry becomes a tab of the Component Library window (beside *My Folders* and the online library), registered by `main.lua` while it loads:

```lua
ccad.catalog {
  id = "furniture",
  load = function()            -- once, when the tab opens
    return { total = 1200, notice = "…", categories = { { id = "chairs", name = "Chairs", count = 140 } },
             colors = { "white", "black" } }
  end,
  search = function(q)         -- q: text, category?, color?, max_width?, max_depth?, max_height? (m), offset, limit
    return { total = 3, items = { { id = "c1", name = "Chair", subtitle = "Oak", kind = "furniture",
             article = "000.123.45", bytes = 250000, dimensions = { width = 0.45, depth = 0.5, height = 0.8 },
             colors = { "white" }, url = "https://example.org/c1", has_thumbnail = true } } }
  end,
  thumbnail = function(id)     -- optional: the path of a picture in the plugin's cache
    return ccad.http.download(base .. id .. ".png", "thumbs/" .. id .. ".png")
  end,
  prepare = function(id)       -- when the user loads the item: download it and say how to load it
    local path = ccad.http.download(base .. id .. ".glb", "models/" .. id .. ".glb", { sha256 = sums[id] })
    return { command = "gltf_component_load", args = { path = path, name = "Chair", kind = "furniture" } }
  end,
}
```

The catalog's functions run on a thread of their own, so the window keeps working while they download. A thumbnail must be a file inside the plugin's cache, at most 4 MiB and 2048 pixels a side; `prepare` must name `gltf_component_load`, `component_load` or `skp_component_load` with a `path` inside the plugin's cache, else the app refuses it. A catalog declared in `plugin.toml` that `main.lua` does not register is a load error, and `ccad.catalog` called later is an error. `ccad plugins` lists each plugin's `catalogs` and `allowed_hosts`.

### Developing a plugin

**Start from the template.** `ccad plugins new <folder> <id> [name=<name>]` makes `<folder>/<id>/` (refused when it exists or the id is invalid) with a plugin that loads and whose test passes: `plugin.toml` (API 2, one button, one setting), `main.lua` (one command with UI hints that reads `ccad.settings` and keeps `ccad.data`), `locales/en.toml` and `locales/pt-BR.toml`, `icons/<id>.svg`, `tests/basic_test.lua`, a `README.md`, and `types/ccad.d.lua` with a `.luarc.json` that points the Lua language server at it.

```sh
ccad plugins new ~/dev room_tools name="Room tools"
ccad plugins test ~/dev/room_tools
```

**Types.** [`docs/lua/ccad.d.lua`](lua/ccad.d.lua) declares the whole `ccad` table for [LuaLS](https://luals.github.io/) (VS Code's Lua extension and other editors): `run`, `try`, `commands`, `describe`, `json`, `array`, `null`, `version`, `api`, `plugin`, the `ccad.command` spec with its parameters and UI hints, `settings`, `data`, `file`, `process` (request and result), `on` with the event names and payloads, and `ccad.test`. Put its folder in `workspace.library` of `.luarc.json` (the template does) to get completion and checks. A test fails when the runtime table has a name the file does not declare, so it stays complete.

**Tests.** `ccad plugins test <plugin folder> [--filter <text>] [--tool <plugin>.<tool>=<path>]` runs each `tests/*_test.lua` of the plugin (in name order) as a Lua script with full access, like `ccad lua`, over a new project from the default metric template with the plugin loaded and granted what its manifest asks for (programs from `--tool`, as for `--plugins`). A file is one test, its top level; a file that defines global functions named `test_*` is one test per function, each in a new project of its own after the file's top level ran there (its setup). Names are `<file>` or `<file>::<function>`; `--filter` runs those whose name holds the text. `ccad.test` has the assertions, which raise an error at the line that called them:

| | |
|---|---|
| `ccad.test.eq(actual, expected, msg)` | equal values; tables compared deeply (`expected [1,3], got [1,2]`) |
| `ccad.test.near(actual, expected, tol, msg)` | numbers at most `tol` apart (default 1e-9) |
| `ccad.test.fails(fn, pattern)` | `fn` raises an error whose text holds `pattern` (plain text, or a Lua pattern); returns the text |
| `ccad.test.ok(value, msg)` | a true value; returns it |

```lua
-- tests/rooms_test.lua
local t = ccad.test
function test_a_room_has_four_walls()
  ccad.run("quick_rooms__rectangle_room", { width = 4, depth = 3 })
  t.eq(#ccad.run("elements_list", { category = "wall" }), 4)
end
```

The output is one line per test, `PASS <name>` or `FAIL <name>: <message>` followed by what the test and the plugin printed, then `<n> passed, <m> failed`. The exit code is 0 when all passed, 1 when one failed (or none matched the filter), 2 for usage errors (no `plugin.toml`, no tests, a `--tool` that does not fit). [`examples/plugins/quick_rooms/tests`](../examples/plugins/quick_rooms/tests) is an example.

**In the app.** The Plugins window's *Reload Plugins* loads every plugin again from its files (permissions stay; not while a plugin command runs). In *Preferences → File locations*, **Plugin developer mode** checks the plugins folder and the **Development folder** every second (the size and time of their `.lua`, `.toml` and `.svg` files) and reloads the plugins when one changed, with a message in the status bar; the reload waits while a plugin command runs. The development folder (a plugin's folder or a folder of plugins) loads in developer mode like the plugins folder, with the same permissions by id; a plugin in both loads from the development folder, and the console says so. The **Plugin console** (*Manage → Plugins*, or *Plugin Console* in the Plugins window) lists, with their time since the app started and their plugin, what plugins print, their load errors, their event handlers' errors and handlers turned off, and each load; it shows one plugin or all, *Clear* empties it and *Copy* copies what it shows; it keeps the last 2000 entries ([Plugins](interface.md#plugins)).

## Scripted components

A component piece can be a **script**: Lua source that declares the component's parameters and hotspots and builds its pieces from their values. The script is pure: it sees the values and nothing else (no commands, files or model), so it runs once for each set of values; moving or rotating a placed component does not run it again. [`examples/components/cabinet.lua`](../examples/components/cabinet.lua) is a complete example.

```lua
param.length   { "depth", label = "Depth", default = 0.55 }          -- shared by every element of a type
param.choice   { "handle", { { "bar", "Bar" }, { "none", "None" } } }
param.length   { "width", label = "Width", default = 0.8, instance = true }  -- each element's own
param.boolean  { "worktop", default = true, instance = true }
param.material { "body", label = "Body", instance = true }
hotspot        { "plinth", 0, 0.05, 0, label = "Plinth", shape = "diamond", color = "#ff8800" }

function geometry(p)
  local pieces = { box { 0, 0, 0, p.width, p.depth, 0.85, material = p.body } }
  if p.worktop then pieces[#pieces + 1] = box { 0, 0, 0.85, p.width, p.depth + 0.02, 0.03 } end
  return pieces
end
```

| Declaration | Value in `p` |
|---|---|
| `param.length`, `param.angle`, `param.number { id, label=, default=0, instance= }` | a number (meters, degrees) |
| `param.boolean { id, default=false }` | `true` or `false` |
| `param.text { id, default="" }` | a string |
| `param.choice { id, { {key, label}, … }, default=first key }` | the chosen key |
| `param.material { id, default="" }` | a project material's name (empty: the piece's material) |
| `hotspot { id, x, y, z, label=, shape="circle", color="#00ffff" }` | `{ x, y, z }`, where the element has it |

Declarations go at the script's top level, never inside `geometry`. Ids are names (letters, digits, `_`) and must be distinct; changing a label does not rename a parameter. A parameter without `instance = true` belongs to the type, and changing it changes every element of the type; with it, each element keeps its own value. Hotspot shapes are `circle`, `square`, `triangle`, `diamond` and `triangle_down`; colors are `#RRGGBB` or `#RRGGBBAA`. `p` also holds the component's other parameters, its reference planes and, in doors and windows, `WallThickness`.

`geometry(p)` returns a list of pieces (lists of pieces are flattened), in the component's frame (meters from the insertion point, x to the right, y to the back, z up). Every builder takes its values in order or by name, plus `name`, `material`, `void`, `in_plan`, `plan_only` and `detail` (a list of `coarse`, `medium`, `fine`):

| Builder | Values |
|---|---|
| `box` | `x, y, z, width, depth, height` |
| `cylinder` | `x, y, z, radius, height` |
| `extrusion` | `points` (`{x, y}` in plan), `z, height` |
| `profile` | `points` (`{u, v}` or `{u, v, radius = r}` for an arc to the next point), `offset, depth`, `plane = "plan"`, `"front"` or `"side"` |
| `revolution` | `points, offset, axis, angle` (default 360), `plane` |
| `sweep` | `path, profile, offset`, `plane` |
| `blend` | `bottom, top, offset, depth`, `plane` |
| `mesh` | `vertices` (`{x, y, z}`), `faces` (lists of vertex numbers from 1, counter-clockwise seen from outside) |

Scripts run in the [sandbox](#lua-scripts) with smaller limits (20 million instructions, 5 seconds, 64 MB, 10,000 pieces, a million points), and `math.random` repeats the same numbers. When a script fails, the element keeps the last pieces that worked and the error, with its line, shows in `element_get` (`derived.script_errors`), in the properties palette and in `component_params_get`; fixing the script or the value brings it back.

Commands:

```sh
ccad run component_create name=Cabinet 'def={"items": [{"name": "Script", "shape": {"Script": {"source": ""}}}]}'
ccad run component_script_set type=Cabinet piece=Script source=@examples/components/cabinet.lua
ccad run component_params_get type=Cabinet
ccad run component_params_set type=Cabinet name=handle value='"none"'
ccad run component_params_set 'elements=[100, 101]' name=width value=1.2
ccad run component_hotspot_move element=100 name=plinth 'point=[0, 0.08, 0]'
```

- `component_script_set` writes a script piece (or adds one). A script that does not load (a syntax error, a bad declaration) is refused; the declarations become the component's parameters and hotspots, those still declared keep their values when their kind still fits, and the result lists the errors of the first run. A component holds one script piece; its parameters are changed in the script, not with `component_param_set` or `component_param_delete`.
- `component_params_get` shows a type's parameters and hotspots, or those of placed components of one component, with `mixed` where they differ.
- `component_params_set` sets a type's value, or the instance value of several elements at once: numbers, `true`/`false`, texts, a choice's key or a material's name. It is one undo step, and nothing changes when any element refuses the value.
- `component_hotspot_move` moves a hotspot in the component's frame, or in project coordinates with `world=true`.

A material a parameter names counts as used: it cannot be deleted, and renaming it renames the values. IFC export writes the pieces the script made, as for any component.

## Macros in the app

*Manage → Macros → Record Macro* records the changes you make until you press *Stop Recording*; the macro is saved as a script (`.ccmacro`, the format above). *Run Macro* plays a macro or any script on the open project; `ccad script` runs the same files. With a macros folder set in *Preferences → File locations*, the macro file dialogs start there and *Run Macro* is a menu of its `.ccmacro` files (plus *Browse…* for any other file).

- Only edits are recorded: opening, saving and exporting are left out, so replaying never overwrites files.
- A value dragged in the properties is recorded once, with its final value; changes made in a dialog that you cancel are not recorded.
- Elements, types, views and levels created while recording are written as [references](#scripts) to the step that created them (`{"$step": n}`), so the macro uses what it creates each time it runs. The level of the plan you record in is written as `{"$level": "active"}`: the macro plays on the level of the plan active when you run it (other levels stay as recorded).
- Other steps refer to elements by id. Played on the project it was recorded on (or on a copy of it as it was when recording started) a macro reproduces the same changes; on another project, steps that name existing elements by id may fail or reach other elements.

### Checking and formatting macros

`macro_check` (with the macro's `text` or a file's `path`) reads a macro without running it and returns its number of steps and the problems found, each with its line, step and severity: lines that are not steps (bad JSON, no `"command"`), commands the session does not have (plugin commands count), argument names a command does not take, required arguments left out (a placeholder counts as given), `$step` references to a step that is not an earlier one and `$level` values other than `"active"` are errors; keys of a step other than `command` and `args`, which running it ignores, are warnings. `macro_format` rewrites a macro with one step per line in the compact form recorded macros are saved in, keeping comments and blank lines where they are and lines that are not steps as they are.

```sh
ccad run macro_check path=examples/small_house.ccmacro
```

For the app's step-by-step running, `cc_engine::script::Stepper` runs a macro one step at a time, each step one undo step, and steps back over the last one as long as nothing else changed the model since (a query steps back without undoing anything); the steps left can also run as a single undo step.

## Command palette in the app

**Ctrl+Shift+P** (or the magnifier at the end of the ribbon tabs) opens the command palette. It finds interface commands, views to open or create and the engine's commands by name, description or shortcut, without regard to accents or case. Engine commands open a form with their JSON arguments, prefilled with the required fields; **Run** (Ctrl+Enter) executes them in the open project, as one undo step, and shows the result. File commands (`project_open`, `ifc_export`…) are left to the *File* tab.

Every ribbon button's tooltip names the engine command it runs, so what you do by hand can be written as a script.

## MCP server

`ccad mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) server on standard input/output. Every command is a tool with the same name, description and input schema; queries are marked read-only. The server keeps one session, so an agent opens or creates a project, works on it over many calls and saves it.

**Claude Code:**

```sh
claude mcp add constructcad -- ccad mcp
# or with a project open from the start
claude mcp add constructcad -- ccad mcp -p /path/to/house.ccad
```

**Claude Desktop and other clients** (`claude_desktop_config.json` or the client's MCP settings; use the full path of `ccad`):

```json
{
  "mcpServers": {
    "constructcad": { "command": "/path/to/ccad", "args": ["mcp", "--lang", "en"] }
  }
}
```

On Windows, `ccad.exe` is installed next to the app, by default in `C:\Program Files\ConstructCraft` (JSON doubles the backslashes):

```json
{
  "mcpServers": {
    "constructcad": { "command": "C:\\Program Files\\ConstructCraft\\ccad.exe", "args": ["mcp"] }
  }
}
```

The installer's *Add ccad to the PATH* option (on by default) lets clients find plain `ccad`, but only clients started after the installation see the new PATH; the full path always works. Pointed at the app itself (`constructcraft mcp` or `constructcraft --mcp`), the client gets the same server: the app runs the `ccad` next to it on the same standard input and output.

Requests an agent can handle with these tools, for example: *"open house.ccad, list the doors without a mark and number them"*, *"import model.ifc and tell me the volume of the concrete walls"*, *"add a second floor 3 m above the ground floor with the same exterior walls"*, *"export sheet A101 as PDF"*.

Notes:

- Paths are on the machine where `ccad` runs.
- Errors come back as tool results with `isError` and the error object, so the model can correct its call.
- The server speaks protocol revisions 2025-06-18, 2025-03-26 and 2024-11-05 and is implemented in `crates/cc_cli/src/mcp.rs` (JSON-RPC 2.0, one message per line).

## For developers: adding a command

Every user-facing feature starts as an engine command; the app calls it rather than editing the document itself.

1. Write the parameter struct in `crates/cc_engine/src/commands/<area>.rs`: `#[derive(Debug, Deserialize, Serialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`, a doc comment (the command's description) and a doc comment on every field (its parameter description; a test fails without them).
2. Implement `Command` with `NAME` (`snake_case`), `KIND` and `run`. Resolve references with `refs`, change the model inside `commands::edit` (one undo step, refused if the result needs repair), and return JSON (`created(s, id)` for elements). Use `t!` messages for errors people will read.
3. Register it in its group in `registry.rs`.
4. Test it in `crates/cc_engine/tests/commands.rs` through `Session::execute`.
5. Regenerate the reference: `cargo run -p cc_cli -- reference > docs/commands.md` (a test fails while it is stale).
6. In the app, call it with `Session::run(Command { … })`, `actions::run_command` or the `engine_edit` helpers (use `Session::run_merged` for dragged values). That also makes it recordable in macros.

Commands added at run time implement `cc_engine::Extension` (`crates/cc_engine/src/extension.rs`) and are set on the session with `Session::set_extension`; `cc_plugins::PluginHost` is the one for Lua plugins. `Session::commands`, `describe` and `kind_of` cover both, and `Session::execute` runs an extension command as one undo step (`Document::squash_since`).

Architecture details are in [AGENTS.md](../AGENTS.md).
