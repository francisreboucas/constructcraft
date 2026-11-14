//! The crate's integration tests as one binary (one link instead of one per
//! file). Each file in this folder is a module here; `every_test_file_is_a_module`
//! fails when one is left out.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod classes;
mod connections;
mod rebar;
mod round_trip;
mod tank;

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
