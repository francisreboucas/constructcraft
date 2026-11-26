# AGENTS.md

Guidance for AI coding agents working in this repository.

## Commands

```sh
cargo run -p cc_app                                   # app (opens the home screen)
cargo run -p cc_app -- examples/casa_simples.ccad     # open a project
cargo run -p cc_app -- examples/casa_simples.ccad --view "Elevação: Sul" --lang en
cargo run -p cc_core --example casa_simples           # regenerate examples/casa_simples.ccad
cargo run -p cc_cli -- commands                       # the engine's command catalog (no GUI)
cargo run -p cc_cli -- -p examples/casa_simples.ccad run elements_list category=wall
cargo run -p cc_cli -- -p examples/casa_simples.ccad run view_render view="{3D}" path=out.png   # path trace a 3D view
cargo run -p cc_app --features rvt                    # with Revit import (optional, off by default)
cargo run -p cc_cli -- mcp                            # MCP server over stdio
cargo run -p cc_cli -- lua examples/small_house.lua width=10   # a Lua script
cargo run -p cc_cli -- --plugins examples/plugins commands     # the catalog with the example plugin's commands
cargo run -p cc_cli -- reference > docs/commands.md   # regenerate the command reference (a test checks it)
cargo run -p api_doc                                  # regenerate the API reference of the site (docs/api, from the doc comments)

cargo nextest run --workspace                          # all tests (cargo-nextest)
cargo test --workspace --doc                          # doctests (nextest does not run them)
cargo nextest run -p cc_geom rooms                     # tests of one crate matching a name
cargo check --tests                                   # type errors without building test binaries
cargo nextest run -p cc_engine --test main macros     # integration tests matching a name (one binary per crate)
cargo nextest run -p cc_rvt -p cc_engine -p cc_cli -p cc_app --features cc_app/rvt,cc_cli/rvt rvt   # the Revit tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check                            # max_width = 120 (rustfmt.toml)
CC_UI_SNAPSHOTS=1 cargo nextest run -p cc_app snapshots # compare UI screenshots (tests/snapshots)
UPDATE_SNAPSHOTS=1 cargo nextest run -p cc_app snapshots # rewrite them after an intended UI change

# IFC validation (CI only; IfcOpenShell is never a build/runtime dependency)
pip install -r scripts/requirements-ifc.txt
cargo run -p cc_ifc --example export_ifc -- examples/casa_simples.ccad out.ifc
python scripts/validate_ifc.py out.ifc
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy + tests (`cargo nextest run --profile ci`, then the doctests with `cargo test --doc`) on Linux (x86_64 and arm64)/Windows/macOS, clippy and tests of the crates with an `rvt` feature with it on (Linux), the IFC validation job (exports `examples/casa_simples.ccad`, `crates/cc_ifc/tests/fixtures/features.ccad`, `structure.ccad`, `mep.ccad`, `arcs.ccad`, `roofs.ccad`, `roof_joins.ccad`, `phases.ccad`, `topography.ccad`, `curtains.ccad`, `profiles.ccad`, `connections.ccad`, `wall_profiles.ccad`, `spline_walls.ccad`, `attached_walls.ccad` and `pipes.ccad`), and a DCO check on PRs: commits must be signed off (`git commit -s`).

## Architecture

Rust workspace (edition 2024). Dependency direction: `cc_core` ← `cc_geom` ← `cc_views` / `cc_render` / `cc_ifc` ← `cc_engine` ← `cc_plugins` ← `cc_cli` (and `cc_engine` ← `cc_app`), and `cc_core` ← `cc_cad` ← `cc_engine`; `cc_geom` ← `cc_views` ← `cc_raytracing` ← `cc_engine` (and `cc_app`), a CPU path tracer with no GPU dependency over the crates.io `sort-raytracing` library (GPL-3.0-only); `cc_skp` has no workspace dependencies and only `cc_engine`, and `cc_plugins` and `cc_app` (its ZIP reader), use it; `cc_rvt` has no workspace dependencies (it wraps the crates.io `rvt` crate, Apache-2.0) and only `cc_engine` uses it, behind its `rvt` feature (off by default; `cc_app` and `cc_cli` have an `rvt` feature that turns it on); `cc_core` ← `cc_script` ← `cc_geom` and `cc_plugins`, where `cc_script` (the Lua sandbox, JSON ↔ Lua conversion and the runtime of script pieces) builds Lua 5.4 from its C sources (`mlua` with `vendored`), so a C compiler is needed; `cc_engine::draco` (glTF Draco compression) is the crates.io `draco-core` (pure Rust, Apache-2.0). `cc_core` knows nothing of geometry, views or UI; no crate depends on `cc_app`.

**Every user-facing operation must be reachable without the GUI**, as a command of `cc_engine`; the GUI, the `ccad` command line and the MCP server call the same commands. A new feature starts as an engine command (with a test in `crates/cc_engine/tests`), and the GUI calls it; do not put model logic in `cc_app`. The app holds a `Session` (`AppState::session`) and changes the model only through commands: `Session::run(SomeCommand { .. })` (typed, the same path as JSON commands), `actions::run_command` (errors to the status bar), or `engine_edit::{element, element_type, view, material, units, info, level, system}`, which turn a change on a copy of an object into its `*_set` command; dragged values pass a merge key (`Session::run_merged`) so they stay one undo step. The only direct document uses left are cancelling a dialog (`Session::undo_to`) and the untracked 3D camera. User documentation: `docs/automation.md` (and `docs/automacao.md`), `docs/interface.md` (and `docs/interface.pt-BR.md`) for the ribbon, tooltips, palette and UI tests; `docs/commands.md` is generated from the catalog (`reference.rs`), every parameter needs a doc comment, and `examples/small_house.ccmacro` is run by a test. Commands run through `Session::execute` are recorded while a macro is recording (edits only; dragged values once); macros are `cc_engine::script` files, the format `ccad script` runs; `script::parse_lines` reads one line by line (blank, comment, step, error), `script::check` finds what would fail without running it (`macro_check`; `macro_format` rewrites it canonically), and `script::Stepper` runs it step by step, each step one undo step (or the rest squashed into one), stepping back only while the step's undo entry is still the top (`Document::undo_top_id`, a fresh id per recorded or merged step).

The crates in one line each:

- **cc_core** — the BIM model: `Project` is the single source of truth, every edit goes through `Document::edit`/`edit_merged` (a reversible `Transaction`), `EditCtx::finish` runs the `sync_*` passes inside the same transaction, files are versioned RON (`persist.rs`, `FORMAT_VERSION`; new fields need serde defaults) and pass through `Project::repair` on load.
- **cc_geom** — derived geometry, rebuilt by `ModelGeometry::build` whenever the document revision changes.
- **cc_views** — views are queries over the model: `generate(project, geom, view)` returns a `Drawing2D`; schedules, sheets and the SVG/PDF/PNG/DXF exports live here.
- **cc_render** / **cc_raytracing** — the offscreen wgpu renderer of 3D views and the CPU path tracer.
- **cc_ifc**, **cc_cad**, **cc_skp** — IFC export and import, DXF/DWG/SVG reading, SketchUp files; all in-house readers and writers.
- **cc_rvt** — Revit files (.rvt) converted to IFC4 by the crates.io `rvt` crate (not in-house) in a child process with a timeout, which `cc_ifc` then imports; optional (`--features rvt`).
- **cc_engine** — the `Session` and the command catalog (`registry.rs`): a command is a parameter struct implementing `Command { NAME, KIND, run }`, registered in `registry!`; edits go through `commands::edit`; errors are values (`EngineError`); localization (`i18n.rs`, `t!`).
- **cc_script** / **cc_plugins** — the Lua sandbox, scripts and plugins over the engine.
- **cc_cli** — the `ccad` binary (commands, scripts, graphs, MCP server), no GUI.
- **cc_app** — the eframe/egui app; it holds a `Session` and no model logic.

### Architecture notes by topic

The detailed notes (types, functions, invariants, commands and app entry points of each feature) live in `docs/architecture/`, one file per area. **Before working on a feature, read the file that covers it** (or search the folder for the type or command name). **When a change adds or alters a feature, update its bullet there** (a new feature gets a new bullet in the file of its area), and add the topic to this list; do not put feature details in this file, which must stay under 40,000 characters.

- [`docs/architecture/core.md`](docs/architecture/core.md) — cc_core, Components, Component reference planes and dimensions, cc_engine, Measurement systems, Visual programming graphs, cc_plugins, Plugin development, cc_cli, Material libraries.
- [`docs/architecture/geometry.md`](docs/architecture/geometry.md) — cc_geom, cc_render, Joined pieces.
- [`docs/architecture/walls.md`](docs/architecture/walls.md) — Curtain walls, Wall profiles, Spline walls, Niches, Wall flip, Wall draw shapes. 
- [`docs/architecture/elements.md`](docs/architecture/elements.md) — Sketches, Roof coverings and framing, Roof joins, Conceptual masses, Terrain, Building pads, Shafts, Swept profiles.
- [`docs/architecture/structure.md`](docs/architecture/structure.md) — Piles and pile caps, Structural profiles, Structure by material, Steel buildings, Member hosts and trusses, Bracing, Wall framing, Connections.
- [`docs/architecture/reinforcement.md`](docs/architecture/reinforcement.md) — Bar marks and bending schedule, Reinforcement in plans, elevations and sections, Wall reinforcement, Slab reinforcement, Strip footings with a beam and curved rafts, Pile caps and piles.
- [`docs/architecture/mep.md`](docs/architecture/mep.md) — MEP, Run tags and fitting labels, Pipe series, Pipe Catalog (app).
- [`docs/architecture/interop.md`](docs/architecture/interop.md) — cc_ifc, Imported CAD drawings, cc_skp, glTF export, cc_rvt.
- [`docs/architecture/views.md`](docs/architecture/views.md) — cc_views, Schedule definitions, Phases, Design options, Detail levels, Visual styles, View graphics, Drafting views, Callouts and detail views, Browser organization.
- [`docs/architecture/annotation.md`](docs/architecture/annotation.md) — Dimensions, Tags, Level marks, Level heads, Detail lines, Line styles and regions, Fill patterns, Detail components, Title blocks.
- [`docs/architecture/site-sun-render.md`](docs/architecture/site-sun-render.md) — Site location, Sun studies, Rendering.
- [`docs/architecture/app.md`](docs/architecture/app.md) — cc_app, Online component library (3D CraftHouse), Component thumbnails, Plugin command forms, Plugins tab and plugin settings, Plugin console, reload and developer mode, Plugin jobs and external programs.

## UI conventions

- **Familiar by design**: menus, shortcuts, panels and tools go where desktop CAD/BIM users expect them (quick access toolbar top-left, ribbon tabs, properties/browser docked left, standard Ctrl shortcuts and two-letter tool keys). Use general conventions only — never a specific product's icons, colors or wording; prefer neutral terms (Category, Type, Component — not "Family").
- Colors and fonts come from `theme.rs` (`LIGHT` and `DARK` hold the tokens; `PALETTE` reads those of the theme shown, which `theme::sync` follows every frame, and `theme::palette(ctx)` picks by the egui theme; the user picks light, dark or the system's with `ThemeChoice` in the Preferences window, and a text size (`theme::set_font_scale`) that scales the text styles and the property tables; 2D drawings keep white `paper` in both; IBM Plex Sans and IBM Plex Mono in the interface, Inter in drawings and exports). Icons are our own SVG line drawings in `assets/icons/ui` (24 grid, white strokes, `stroke-width` 1.75, round caps/joins, soft fills at 20% opacity), rasterized by resvg per pixel size in two layers and tinted in `icons.rs`: strokes in the ink tone, soft fills in the tint tone (`icons::Tones`), plus an optional badge (new, import, export, remove).
- Ribbon, quick access and shortcut entries are `commands::UiCommand`s: label, shortcut, description, engine command and availability (with the reason) are defined there once; buttons and tooltips read them. Custom widgets report their role, label and state with `Response::widget_info`, so `ui_tests.rs` (egui_kittest) can find them by label; screenshot references live in `crates/cc_app/tests/snapshots` and run on request, since GPUs render slightly differently.
- Every modal uses `dialog::Dialog` (header with close button, footer buttons right-aligned: main action first, Cancel last; auxiliary buttons left; `danger` for destructive). Dialogs that edit data offer OK/Cancel and wrap edits in `dialog::session` so Cancel undoes them. A dialog is moved by its header and widens to fit its footer buttons; its header and footer follow a body wider than `width`, but a body must fit: in tests a dialog whose body overflows panics (`dialog::overflow`), and `every_dialog_keeps_its_body_inside` opens them all, so add a new dialog there. In table cells use `table::checkbox`/`labeled_checkbox`/`boxed` (a plain checkbox has no box in a flat cell) and `table::button_row` for several buttons; a `table::row` value is clipped to its cell.
- Property grids use the shared `table.rs` primitives (collapsible `section`, two-column `row`).
- Settings windows (pages of options, like Preferences) use `settings.rs`: a `Dialog::flush` body laid out by `settings::sidebar_layout`, a `settings::page` per page (title and what it holds), `group` headings and `setting` rows (name and what it does on the left, the control on the right) with `switch`, `segmented`, `combo`, `button` and `path_box` controls; dialogs that ask for a few values use the same `setting` rows or `settings::field`s in `settings::columns`, not bare grids of labels.
- No hard-coded interface text: keys live in `assets/i18n/*.ftl` (`pt-BR` is the reference, then `en`, `es`, `fr`, `de`, `it`, `lo`, `pl`, `uk-UA`, `ru`, `zh-TW`, `zh-CN`, `ja`), read with `t!` / `tr`. A test scans the sources and fails if any key used in code is missing from a language, and another fails unless every language has the same keys and placeholders, so add every new key to all of them.

## Robustness rules

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` — the workspace denies them in clippy (`[workspace.lints]`), and `clippy.toml` allows them only in tests. Return errors and propagate with `?`, or use `let … else`, `if let`, `ok_or(…)?`, `unwrap_or*` where the fallback is truly correct (never one that silently corrupts a document).
- **No `unsafe`** (`unsafe_code = "forbid"`).
- **Files are hostile input.** `.ccad` files pass through `Project::repair` on load (`persist::from_ron_with_repairs`): invalid numbers and dangling references are removed or reset and the app reports how many. Use `get()` for indices that come from files, selections or arithmetic on them; guard divisions by zero and NaN/inf.
- **Bound recursion and loops** over model references with visited sets or limits (hosting can be cyclic in a crafted file; see `EditCtx::remove_element`).
- **Last-resort guard:** opening, saving and exporting run inside `actions::guard` (`catch_unwind`); a panic becomes an error message and the open document is kept. Keep `panic = "unwind"` (the default profile).
- **Tests run with cargo-nextest** (`cargo nextest run`; install it with `cargo install cargo-nextest --locked`), in every session and in CI; doctests still need `cargo test --doc`. Settings live in `.config/nextest.toml` (the CI uses its `ci` profile).
- **Prove it:** every crash fix comes with a small synthetic regression test that failed before the fix.
- **Integration tests are one binary per crate**: `cc_engine`, `cc_ifc`, `cc_plugins` and `cc_skp` set `autotests = false` and build only `tests/main.rs`, which declares each file of `tests/` as a module; a new test file needs its `mod` line there (`every_test_file_is_a_module` fails otherwise).

## Project policies

- Clean-room: behaviour comes from public documentation, general CAD conventions, open standards and user feedback; reference tests use analytic values or open-source tools (see CONTRIBUTING.md and the README "Independence" section).
- Code, comments, identifiers and commit messages are in English; the user communicates in Portuguese.
