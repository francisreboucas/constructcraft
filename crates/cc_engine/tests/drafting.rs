//! Drafting views (issue #148): 2D views independent of the model, with
//! their own scale, showing only the text, dimensions and imported drawings
//! placed in them; placed on sheets and exported like other views.

// Test helpers outside #[test] functions may fail loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cc_engine::{ErrorCode, Session};
use serde_json::{Value, json};

fn run(s: &mut Session, name: &str, args: Value) -> Value {
    s.execute(name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn fails(s: &mut Session, name: &str, args: Value) -> ErrorCode {
    s.execute(name, args).map(|v| panic!("{name} should fail, gave {v}")).unwrap_err().code
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ccad-drafting-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// A millimeter DXF with one 2 m line.
fn line_dxf() -> String {
    let groups: &[(i32, &str)] = &[
        (0, "SECTION"),
        (2, "HEADER"),
        (9, "$INSUNITS"),
        (70, "4"),
        (0, "ENDSEC"),
        (0, "SECTION"),
        (2, "ENTITIES"),
        (0, "LINE"),
        (8, "0"),
        (10, "0"),
        (20, "0"),
        (30, "0"),
        (11, "2000"),
        (21, "0"),
        (31, "0"),
        (0, "ENDSEC"),
        (0, "EOF"),
    ];
    groups.iter().map(|(c, v)| format!("{c:>3}\n{v}\n")).collect()
}

/// The elements a view's drawing shows.
fn drawn(s: &mut Session, view: u64) -> std::collections::BTreeSet<u64> {
    let d = s.drawing(cc_core::ViewId(view)).unwrap();
    d.prims.iter().filter_map(|p| p.element()).map(|e| e.0).collect()
}

#[test]
fn drafting_views_show_only_what_is_drawn_in_them() {
    let mut s = Session::default();
    // Some model, which the drafting view must not show.
    run(&mut s, "wall_create", json!({"start": [0, 0], "end": [5, 0]}));

    let view = run(&mut s, "view_create", json!({"kind": "drafting"}));
    let id = view["id"].as_u64().unwrap();
    assert_eq!(view["category"], "drafting");
    assert_eq!(view["scale"], 10, "details are drawn large by default");
    let listed = run(&mut s, "views_list", json!({"category": "drafting"}));
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert!(drawn(&mut s, id).is_empty(), "no model geometry");
    assert!(s.drawing(cc_core::ViewId(id)).unwrap().prims.is_empty());

    // Its scale and detail level are its own.
    run(&mut s, "view_set", json!({"view": id, "scale": 5, "detail": "fine", "params": {}}));
    let got = run(&mut s, "view_get", json!({"view": id}));
    assert_eq!(got["scale"], 5);
    assert_eq!(got["detail"], "fine");
    // It does not show the model, so it has no visual style or phase.
    assert_eq!(
        fails(&mut s, "view_set", json!({"view": id, "visual_style": "wireframe"})),
        ErrorCode::InvalidArguments
    );
    let phase = run(&mut s, "phases_list", json!({}))[0]["id"].clone();
    assert_eq!(fails(&mut s, "view_set", json!({"view": id, "phase": phase})), ErrorCode::InvalidArguments);
    // But its categories can be hidden like in any other 2D view.
    run(&mut s, "view_graphics_set", json!({"view": id, "category": "text", "visible": false}));

    // Text, dimensions and imported drawings go in it, and only they show.
    run(&mut s, "view_graphics_set", json!({"view": id, "category": "text", "visible": true}));
    let text =
        run(&mut s, "text_create", json!({"view": id, "position": [0, 1], "text": "Detail"}))["id"].as_u64().unwrap();
    let dim = run(&mut s, "dimension_create", json!({"view": id, "a": [0, 0], "b": [2, 0], "offset": 0.3}))["id"]
        .as_u64()
        .unwrap();
    let path = scratch("line.dxf");
    std::fs::write(&path, line_dxf()).unwrap();
    let cad = run(&mut s, "cad_import", json!({"path": path, "view": id, "position": [0, 0]}))["id"].as_u64().unwrap();
    assert_eq!(drawn(&mut s, id), [text, dim, cad].into_iter().collect());
    // A dimension in a drafting view ties to nothing in the model.
    match &s.doc.project.element(cc_core::ElementId(dim)).unwrap().kind {
        cc_core::ElementKind::Dimension(d) => {
            assert_eq!(d.refs, [None, None]);
            assert!((d.value() - 2.0).abs() < 1e-9);
        }
        other => panic!("{other:?}"),
    }
    // Nothing drawn in it shows in the plans.
    let plan = run(&mut s, "views_list", json!({"category": "floor_plan"}))[0]["id"].as_u64().unwrap();
    let in_plan = drawn(&mut s, plan);
    assert!(![text, dim, cad].iter().any(|e| in_plan.contains(e)));

    // On a sheet, and exported in every format.
    let sheet = run(&mut s, "view_create", json!({"kind": "sheet"}))["id"].as_u64().unwrap();
    run(&mut s, "sheet_place_view", json!({"sheet": sheet, "view": id}));
    for file in ["detail.svg", "detail.pdf", "detail.png", "detail.dxf"] {
        let out = scratch(file);
        run(&mut s, "view_export", json!({"view": id, "path": out, "dpi": 60}));
        assert!(std::fs::metadata(&out).unwrap().len() > 100, "{file}");
    }
    let sheet_svg = scratch("sheet.svg");
    run(&mut s, "sheet_export", json!({"sheet": sheet, "path": sheet_svg}));
    assert!(std::fs::read_to_string(&sheet_svg).unwrap().contains("Detail"));

    // It saves and loads back unchanged.
    let ron = cc_core::to_ron(&s.doc.project).unwrap();
    let back = cc_core::from_ron(&ron).unwrap();
    assert_eq!(back.view(cc_core::ViewId(id)), s.doc.project.view(cc_core::ViewId(id)));

    // Deleting it takes what was drawn in it.
    run(&mut s, "view_delete", json!({"view": id}));
    for e in [text, dim, cad] {
        assert!(s.doc.project.element(cc_core::ElementId(e)).is_none(), "{e}");
    }
}

#[test]
fn a_wide_cyrillic_note_fits_its_pdf_page() {
    let mut s = Session::default();
    let view = run(&mut s, "view_create", json!({"kind": "drafting"}))["id"].clone();
    run(&mut s, "text_create", json!({"view": view, "position": [0, 0], "text": "Ш".repeat(50)}));
    let svg = run(&mut s, "view_export", json!({"view": view, "path": scratch("wide-note.svg")}));
    let pdf = run(&mut s, "view_export", json!({"view": view, "path": scratch("wide-note.pdf")}));
    let svg_width = svg["paper_mm"][0].as_f64().unwrap();
    let pdf_width = pdf["paper_mm"][0].as_f64().unwrap();
    assert!((svg_width - 160.0).abs() < 1e-6, "{svg}");
    // The default 2.5 mm note has 50 advances of 1961/2048 em in Inter.
    let note_width = 50.0 * 2.5 * 1961.0 / 2048.0;
    assert!(pdf_width >= 85.0 + note_width, "{pdf}");
}

/// An empty drafting view still exports and sits on a sheet.
#[test]
fn empty_drafting_views_export() {
    let mut s = Session::default();
    let id = run(&mut s, "view_create", json!({"kind": "drafting", "name": "Eave", "scale": 20}))["id"].clone();
    assert_eq!(run(&mut s, "view_get", json!({"view": "Eave"}))["scale"], 20);
    let sheet = run(&mut s, "view_create", json!({"kind": "sheet"}))["id"].clone();
    run(&mut s, "sheet_place_view", json!({"sheet": sheet, "view": id}));
    for file in ["empty.svg", "empty.pdf", "empty.png", "empty.dxf"] {
        let r = s.execute("view_export", json!({"view": id, "path": scratch(file), "dpi": 30}));
        assert!(r.is_ok() || r.as_ref().is_err_and(|e| e.code == ErrorCode::InvalidArguments), "{file}: {r:?}");
    }
    run(&mut s, "sheet_export", json!({"sheet": sheet, "path": scratch("empty-sheet.pdf")}));
}
