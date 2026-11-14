//! The crate's integration tests as one binary (one link instead of one per
//! file). Each file in this folder is a module here; `every_test_file_is_a_module`
//! fails when one is left out.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod arc_beams;
mod arc_strip_footings;
mod bar_marks;
mod bracing;
mod browser_org;
mod cad;
mod cad_explode;
mod callouts;
mod column_sections;
mod commands;
mod component_env;
mod component_script;
mod connections;
mod curtain;
mod design_options;
mod detail;
mod detail_components;
mod detail_lines;
mod dimension_types;
mod drafting;
mod extension;
mod fill_patterns;
mod french_doors;
mod gltf;
mod graph;
mod graphics;
mod image;
mod imperial_text;
mod level_heights;
mod level_rename;
mod lightning_cabling;
mod macros;
mod mep_trays_valves;
mod multi_edit;
mod obj;
mod pad;
mod phases;
mod piles;
mod profiles;
mod project_templates;
mod regions;
mod ring_footings;
mod schedule_def;
mod section_marks;
mod shaft;
mod skp;
mod slab_rebar;
mod steel_building;
mod structure_materials;
mod structure_stress;
mod sun;
mod textures_paint;
mod title_block_migration;
mod title_blocks;
mod topography;
mod truss;
mod underlay;
mod view_detailing;
mod visual_style;
mod wall_attach;
mod wall_framing;
mod wall_location_joins;
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
