//! The crate's integration tests as one binary (one link instead of one per
//! file). Each file in this folder is a module here; `every_test_file_is_a_module`
//! fails when one is left out.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod analytical_arcs;
mod arc_beams;
mod arc_strip_footings;
mod bar_marks;
mod beam_anchorage;
mod bracing;
mod browser_org;
mod cad;
mod cad_explode;
mod callouts;
mod column_sections;
mod commands;
mod component_env;
mod component_script;
mod concrete_walls;
mod connections;
mod curtain;
mod design_options;
mod detail;
mod detail_components;
mod detail_lines;
mod dimension_lock;
mod dimension_types;
mod drafting;
mod dxf_hidden_lines;
mod events;
mod extension;
mod fill_patterns;
mod french_doors;
mod gltf;
mod gltf_export;
mod graph;
mod graphics;
mod image;
mod imperial_text;
mod jobs;
mod level_heads;
mod level_heights;
mod level_rename;
mod lightning_cabling;
mod macro_editor;
mod macros;
mod mep_trays_valves;
mod multi_edit;
mod niche;
mod obj;
mod pad;
mod phases;
mod pile_rebar;
mod piles;
mod pipe_fittings;
mod pipes;
mod plugin_data;
mod plugin_scope;
mod profiles;
mod project_templates;
mod rebar_3d_view;
mod rebar_copy;
mod rebar_in_views;
mod regions;
mod render;
mod render_settings;
mod ring_footings;
mod roof_edge_overhangs;
mod roof_join;
mod roof_notch;
mod schedule_def;
mod section_marks;
mod shaft;
mod skp;
mod slab_rebar;
mod spline_walls;
mod splines;
mod staggered_splices;
mod steel_building;
mod stirrup_hooks;
mod structure_materials;
mod structure_stress;
mod sun;
mod sweep_curves;
mod tags;
mod textures_paint;
mod tie_hooks;
mod title_block_migration;
mod title_blocks;
mod topography;
mod truss;
mod underlay;
mod view_detailing;
mod view_marks;
mod visual_style;
mod wall_attach;
mod wall_flip;
mod wall_framing;
mod wall_layer_extents;
mod wall_location_joins;
mod wall_profiles;
mod wall_rebar;
mod window_operation;

/// Every `.rs` file of `tests/` is declared above, so none is skipped.
#[test]
fn every_test_file_is_a_module() {
    let main = include_str!("main.rs");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(stem) =
            path.file_stem().and_then(|s| s.to_str()).filter(|_| path.extension().is_some_and(|e| e == "rs"))
        else {
            continue;
        };
        assert!(
            stem == "main" || main.contains(&format!("\nmod {stem};")),
            "tests/{stem}.rs is not a module of tests/main.rs"
        );
    }
}
