# API reference

[Overview of the architecture](../api.md) · [Visão geral em português](../api.pt-BR.md) · Generated from the source by `cargo run -p api_doc`.

| Crate | Modules | Items | Methods | Summary |
|---|---|---|---|---|
| [`cc_app`](cc_app.md) | 90 | 807 | 180 | ConstructCraft — open source BIM authoring. |
| [`cc_cad`](cc_cad.md) | 8 | 36 | 30 | Reading DWG and DXF drawings into ConstructCraft's neutral `cc_core::cad::CadDrawing` (an underlay placed in a 2D view). |
| [`cc_cli`](cc_cli.md) | 2 | 3 | 0 | `ccad`: ConstructCraft without a GUI. |
| [`cc_core`](cc_core.md) | 80 | 865 | 1222 | Core BIM data model for ConstructCraft. |
| [`cc_engine`](cc_engine.md) | 92 | 749 | 106 | The ConstructCraft engine: every user-facing operation as a named command that runs on a `Session` without a GUI. |
| [`cc_geom`](cc_geom.md) | 42 | 269 | 199 | Derived geometry for ConstructCraft. |
| [`cc_ifc`](cc_ifc.md) | 9 | 58 | 39 | IFC export and import for ConstructCraft. |
| [`cc_plugins`](cc_plugins.md) | 5 | 29 | 18 | Lua scripts and plugins for ConstructCraft. |
| [`cc_raytracing`](cc_raytracing.md) | 7 | 45 | 29 | Photorealistic rendering of a 3D view: path tracing on the pure Rust `sort-raytracing` library, with no GPU. |
| [`cc_render`](cc_render.md) | 8 | 38 | 35 | Offscreen wgpu renderer for the 3D view. |
| [`cc_rvt`](cc_rvt.md) | 2 | 9 | 1 | Revit project files (.rvt), read by the crates.io `rvt` crate (rvt-rs, Apache-2.0, pure Rust, `forbid(unsafe_code)`) and written as IFC4 text, which `cc_ifc` imports as it imports any IFC file. |
| [`cc_script`](cc_script.md) | 5 | 23 | 9 | ConstructCraft's sandboxed Lua 5.4. |
| [`cc_skp`](cc_skp.md) | 35 | 322 | 155 | SketchUp (.skp) files: reader (current and legacy formats, scenes and per-definition instanced scenes) and writer (`create`, `edit`). |
| [`cc_views`](cc_views.md) | 34 | 277 | 73 | View generation for ConstructCraft. |
