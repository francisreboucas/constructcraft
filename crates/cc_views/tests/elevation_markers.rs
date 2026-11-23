//! Initial floor plans expose the project's elevation views.

use cc_core::{Project, ViewKind};
use cc_geom::ModelGeometry;

/// Empty plans show all four default elevations within their fitted bounds.
#[test]
fn empty_plan_includes_selectable_elevation_markers() {
    let project = Project::from_template();
    let geometry = ModelGeometry::build(&project);
    let plan = project.views.values().find(|view| matches!(view.kind, ViewKind::FloorPlan { .. })).unwrap();
    let drawing = cc_views::generate(&project, &geometry, plan).unwrap();
    let (low, high) = drawing.fit_bounds().unwrap();
    assert_eq!(drawing.marks.len(), 4);
    for mark in &drawing.marks {
        assert!(drawing.mark_at(mark.center).is_some());
        assert!((mark.center - low).min_element() > 0.0);
        assert!((high - mark.center).min_element() > 0.0);
    }
}
