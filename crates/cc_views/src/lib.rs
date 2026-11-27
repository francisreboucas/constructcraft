//! View generation for ConstructCraft.
//!
//! Each [`cc_core::View`] is a query over the model. [`generate`] produces a
//! resolution-independent [`Drawing2D`] for graphical views; [`schedule`]
//! computes tables.

mod annotate;
pub mod bar_tags;
pub mod bending;
pub mod cad;
pub mod callout;
pub mod component_view;
mod cut;
pub mod detail;
pub mod detail_component;
mod diagram;
mod drawing;
mod drawing_index;
pub mod dxf;
pub mod export;
pub mod fonts;
pub mod format;
pub mod graphics;
pub mod hatch;
pub mod interior;
mod joins;
mod legend;
pub mod level_heads;
pub mod level_marks;
pub mod linework;
mod mep;
pub use mep::{discipline_of, fitting_lines, plan_fittings};
pub mod notes;
mod plan;
mod projected;
pub mod raster;
pub mod rebar_view;
pub mod region;
mod schedule;
mod shadow;
mod sheet;
pub mod sheet_fields;
pub mod sun_path;
pub mod tag;
mod topography;

pub use annotate::{
    DimensionLayout, HIGHLIGHT_ONLY, dimension_layout, dimension_type, draw_dimension, draw_marks, grid_style,
};
pub use component_view::{ComponentView, ComponentViewKind, ViewConnector, ViewDimension, component_view};
pub use drawing::{
    Anchor, BubbleHandle, Drawing2D, FittingTagHandle, InteriorArrow, LevelHandle, LevelHeadHandle, LineStyle,
    MAX_HITS, MarkHandle, Pen, Prim, ResolvedPen, SectionHandle, TagHandle, halftone,
};
pub use drawing_index::DrawingIndex;
pub use legend::legend;
pub use plan::{is_structural, plan, plan_of, section_pieces};
pub use projected::{Frame, Look, axonometric, elevation, section, view_frame};
pub use schedule::{Cell, Row, Table, field_value, schedule, schedule_where, to_csv, totaled_by_default};
pub use shadow::{Catch as ShadowCatch, Indoors, Shadows, Skip as ShadowSkip, Storey};
pub use sheet::{MARGIN, PlacedViewport, Sheet, SheetEnv, compose as compose_sheet, title_block_drawing};
pub use sun_path::{SunDiagram, SunLabel, SunLabelKind, SunLine, SunLineKind, SunText, diagram as sun_diagram};

use cc_core::{Project, View, ViewKind};
use cc_geom::ModelGeometry;

/// Drawing for any graphical view (`None` for schedules and sheets), with
/// the headings of diagrams left as their translation keys.
pub fn generate(project: &Project, geom: &ModelGeometry, view: &View) -> Option<Drawing2D> {
    generate_labeled(project, geom, view, &|key| key.to_string())
}

/// Drawing for any graphical view, with the view's graphics applied;
/// `label` translates the headings drawn by the view itself (diagrams).
pub fn generate_labeled(
    project: &Project,
    geom: &ModelGeometry,
    view: &View,
    label: &dyn Fn(&str) -> String,
) -> Option<Drawing2D> {
    generate_temporary(project, geom, view, label, None)
}

/// Drawing for any graphical view with its graphics and, when given, the
/// elements temporarily hidden or isolated in it.
pub fn generate_temporary(
    project: &Project,
    geom: &ModelGeometry,
    view: &View,
    label: &dyn Fn(&str) -> String,
    temporary: Option<&cc_core::TemporaryVisibility>,
) -> Option<Drawing2D> {
    let mut resolver = graphics::Resolver::new(project, geom, view, temporary);
    // What the view's graphics keep out of its sun's shadows.
    let skip = Look::casts_shadows(view).then(|| shadow_skip(geom, &mut resolver));
    // What is indoors of a floor plan's or a section's faces catches none of them.
    let indoors = skip
        .as_ref()
        .filter(|_| {
            matches!(
                view.kind,
                ViewKind::FloorPlan { .. } | ViewKind::Section { .. } | ViewKind::InteriorElevation { .. }
            )
        })
        .map(|skip| Indoors::of(project, geom, Some(skip), view.detail));
    let look = Look::of(project, geom, view, skip.as_ref(), indoors.as_ref());
    // The crop of the drawing: the view's own, or an interior elevation's space.
    let mut crop = view.crop;
    // The space an interior elevation is fitted to, whose floor and ceiling it draws.
    let mut space = None;
    let mut d = match &view.kind {
        ViewKind::FloorPlan { level, cut_height, .. } | ViewKind::CeilingPlan { level, cut_height, .. } => {
            let ceiling = matches!(view.kind, ViewKind::CeilingPlan { .. });
            let (structural, tags, services) = match view.kind {
                ViewKind::FloorPlan { structural, structure_tags, mep, .. } => {
                    (structural, structural || structure_tags, mep)
                }
                _ => (false, false, None),
            };
            let drawn = (view.detail, view.style());
            let marks = (view.scale, &view.mark_offsets);
            let mut d = plan::plan_drawing(project, geom, *level, *cut_height, (ceiling, structural), marks, drawn);
            joins::merge(project, &mut d);
            let style = view.style();
            if !style.fills() {
                plan::strip_fills(&mut d);
            } else if style.colored() {
                let cut_z = project.level_elevation(*level) + cut_height;
                let under = plan::style_underlay(geom, cut_z, ceiling, look, &project.fill_patterns);
                d.prims.splice(0..0, under);
            }
            // The bars over the cut concrete, under the plan's marks and tags.
            rebar_view::draw_all(&mut d, project, geom, view);
            if let Some(discipline) = services {
                // The architecture stays in the background of a services plan.
                fade(&mut d);
                mep::services(&mut d, project, geom, *level, *cut_height, discipline, view, label);
            }
            // Regions over the model, under the plan's marks, tags and grids.
            region::regions(&mut d, project, view.id, view.scale);
            detail_component::components(&mut d, project, view.id, view.scale);
            if !ceiling {
                plan::section_marks(&mut d, project, view.scale);
            }
            annotate::plan_annotations(&mut d, project, geom, *level, ceiling, structural, view, label);
            if tags {
                let ctx = tag::TagCtx {
                    project,
                    geom,
                    scale: view.scale,
                    placements: &view.tags,
                    types: &view.tag_types,
                    label,
                };
                annotate::structure_tags_in(&mut d, &ctx, *level);
            }
            d
        }
        ViewKind::Elevation { direction } => {
            let mut d = elevation(project, geom, *direction, look);
            rebar_view::draw_all(&mut d, project, geom, view);
            if let Some(frame) = projected::view_frame(view) {
                level_heads::draw(&mut d, project, view, &frame);
                annotate::projected_grids(&mut d, project, &frame, view.scale, &view.tags);
            }
            d
        }
        // While no walls close a space around the marker, or its wall is not
        // one of them, the view says so instead.
        ViewKind::InteriorElevation { .. } => match interior::try_resolve(project, geom, view) {
            Ok(r) => {
                crop = r.crop;
                if matches!(view.kind, ViewKind::InteriorElevation { auto_extents: true, .. }) {
                    space = r.crop;
                }
                let mut d = projected::section_in(project, geom, &r.frame, view.scale, look);
                joins::merge(project, &mut d);
                rebar_view::draw_all(&mut d, project, geom, view);
                level_heads::draw(&mut d, project, view, &r.frame);
                annotate::projected_grids(&mut d, project, &r.frame, view.scale, &view.tags);
                d
            }
            Err(why) => {
                let mut d = Drawing2D::default();
                d.text_at(glam::DVec2::ZERO, label(why.key()), 3.5, Anchor::LeftBottom, 0.0);
                d
            }
        },
        ViewKind::Section { start, end, depth, .. } => {
            let mut d = section(project, geom, (*start, *end), *depth, view.scale, look);
            joins::merge(project, &mut d);
            rebar_view::draw_all(&mut d, project, geom, view);
            if let Some(frame) = projected::view_frame(view) {
                level_heads::draw(&mut d, project, view, &frame);
                annotate::projected_grids(&mut d, project, &frame, view.scale, &view.tags);
            }
            d
        }
        ViewKind::ThreeD { camera, analytical: true, .. } => projected::analytical(project, geom, camera),
        // The reinforcement over the model drawn faint, as the 3D view shows it.
        ViewKind::ThreeD { camera, rebar: true, .. } => {
            over_faint_model(axonometric(project, geom, camera, look), projected::rebar(geom, camera, view.scale))
        }
        ViewKind::ThreeD { camera, .. } => axonometric(project, geom, camera, look),
        ViewKind::Legend { types } => legend(project, types, view.scale),
        // No model geometry: only what the view owns, added below.
        ViewKind::Drafting { .. } => Drawing2D::default(),
        ViewKind::Diagram { panel } => diagram::panel_diagram(project, geom, *panel, view.scale, label),
        // A schedule draws as a table, for sheets: the elements its phase shows.
        ViewKind::Schedule { category, def } => {
            let phase = graphics::PhaseView::of(project, view);
            let options = project.shown_options(Some(view));
            let table = schedule::schedule_where(project, geom, *category, def, &mut |e| {
                project.option_shows(e, &options) && phase.as_ref().is_none_or(|ph| ph.shows(project, e))
            });
            schedule::drawing(&table, &view.name, view.scale, label)
        }
        ViewKind::Sheet { .. } => return None,
    };
    // Imported CAD drawings, under everything (not in 3D views or diagrams).
    if hosts_cad(&view.kind) {
        cad::underlays(&mut d, project, view.id, view.scale);
        // Regions over the model (plans drew theirs under their annotations).
        if view.level().is_none() {
            region::regions(&mut d, project, view.id, view.scale);
            detail_component::components(&mut d, project, view.id, view.scale);
        }
        // Detail lines, on top of the model and the regions.
        detail::lines(&mut d, project, view.id);
    }
    annotate::dimensions(&mut d, project, view.id, view.scale);
    notes::notes(&mut d, project, view.id, view.scale);
    level_marks::level_marks(&mut d, project, view);
    bar_tags::bar_tags(&mut d, project, geom, view);
    callout::marks(&mut d, project, view);
    graphics::apply(&mut d, &mut resolver, view.scale);
    underlay(&mut d, project, geom, view);
    if let Some(rect) = crop.filter(|r| r.is_valid() && view.kind.croppable()) {
        callout::crop(&mut d, project, view.id, rect);
    }
    // The floor and the ceiling of an interior elevation's space, as cut
    // lines over what the crop left (they are the crop's own bottom and top).
    if let Some(rect) = space.filter(|r| r.is_valid()) {
        let [a, b, c, e] = rect.corners();
        d.line(a, b, LineStyle::Cut, None);
        d.line(e, c, LineStyle::Cut, None);
    }
    // The sun path last, over everything: neither cropped nor overridden.
    sun_path::draw(&mut d, project, geom, view, label);
    pick_ranks(&mut d, project);
    Some(d)
}

/// Mark the drawing's datum elements and the ones whose outline yields to
/// another's, for picking (see [`Drawing2D::hits_at`]).
fn pick_ranks(d: &mut Drawing2D, project: &Project) {
    use cc_core::Category as C;
    let ids: std::collections::BTreeSet<_> = d.prims.iter().filter_map(Prim::element).collect();
    for id in ids {
        match project.element(id).map(|e| e.category()) {
            Some(C::Grid | C::Level) => {
                d.datum.insert(id);
            }
            Some(C::Floor | C::Ceiling | C::Roof | C::Foundation | C::Topography | C::Pad) => {
                d.yielding.insert(id);
            }
            _ => {}
        }
    }
}

/// The elements of `geom` that the view's graphics keep out of its sun's
/// shadows: those it hides cast and catch none, those it makes see-through
/// cast none (as in the 3D view, where hidden solids leave the scene and
/// see-through ones the shadow map).
fn shadow_skip(geom: &ModelGeometry, resolver: &mut graphics::Resolver) -> shadow::Skip {
    let mut skip = shadow::Skip::default();
    if resolver.is_trivial() {
        return skip;
    }
    let mut last = None;
    for id in geom.solids.iter().map(|s| s.element) {
        // An element's solids follow one another.
        if last.replace(id) == Some(id) {
            continue;
        }
        match resolver.style(id) {
            None => {
                skip.hidden.insert(id);
            }
            Some(g) if g.transparency.is_some_and(|t| t > 0.001) => {
                skip.clear.insert(id);
            }
            Some(_) => {}
        }
    }
    skip
}

/// The plans of the levels a floor or ceiling plan shows under itself
/// ([`cc_core::Underlay`]), first in `d` and counted in `d.underlay`: the
/// model only (no annotations or texts), with the view's category
/// visibility, every line in the faint pen and fills faded, belonging to no
/// element.
fn underlay(d: &mut Drawing2D, project: &Project, geom: &ModelGeometry, view: &View) {
    let (Some(level), Some(u)) = (view.level(), view.kind.underlay()) else { return };
    let cut_height = match view.kind {
        ViewKind::FloorPlan { cut_height, .. } | ViewKind::CeilingPlan { cut_height, .. } => cut_height,
        _ => return,
    };
    let mut under = Vec::new();
    for (other, looking) in u.plans(&project.levels, level) {
        let ceiling = looking == cc_core::Looking::Up;
        let drawn = (view.detail, view.style());
        let mut p = plan_of(project, geom, other, cut_height, (ceiling, false), view.scale, drawn);
        joins::merge(project, &mut p);
        graphics::apply(&mut p, &mut graphics::Resolver::new(project, geom, view, None), view.scale);
        under.extend(p.prims.into_iter().filter_map(underlay_prim));
    }
    d.underlay = under.len();
    d.prims.splice(0..0, under);
}

/// A primitive of an underlay: the model's lines in the faint pen, its
/// fills faded to white (invisible ones, texts, pictures and the markers of
/// sections and elevations left out), none of them belonging to an element.
fn underlay_prim(prim: Prim) -> Option<Prim> {
    prim.element()?;
    match prim {
        Prim::Line { a, b, .. } => Some(Prim::Line { a, b, style: LineStyle::Faint, element: None, pen: None }),
        Prim::Fill { pts, color, .. } if color[3] >= 8 => {
            let mut color = color;
            for c in color.iter_mut().take(3) {
                *c = (f64::from(*c) + (255.0 - f64::from(*c)) * 0.75).round() as u8;
            }
            Some(Prim::Fill { pts, color, element: None, cut: false })
        }
        _ => None,
    }
}

/// Whether a view can hold imported CAD drawings: plans, ceiling plans,
/// elevations, sections, legends and drafting views (2D views with a drawing plane).
pub fn hosts_cad(kind: &ViewKind) -> bool {
    matches!(
        kind,
        ViewKind::FloorPlan { .. }
            | ViewKind::CeilingPlan { .. }
            | ViewKind::Elevation { .. }
            | ViewKind::InteriorElevation { .. }
            | ViewKind::Section { .. }
            | ViewKind::Legend { .. }
            | ViewKind::Drafting { .. }
    )
}

/// Draw everything so far faint: lines in the faint pen (dashes kept),
/// fills mixed two thirds with white.
fn fade(d: &mut Drawing2D) {
    for prim in &mut d.prims {
        match prim {
            Prim::Line { style, .. }
                if matches!(style, LineStyle::Cut | LineStyle::Projection | LineStyle::Thin | LineStyle::Hatch) =>
            {
                *style = LineStyle::Faint;
            }
            Prim::Image { faded, .. } => *faded = true,
            Prim::Fill { color, .. } => {
                for c in color.iter_mut().take(3) {
                    *c = (f64::from(*c) + (255.0 - f64::from(*c)) * 0.66).round() as u8;
                }
            }
            _ => {}
        }
    }
}

/// `over` drawn on top of `model` faded ([`fade`]).
fn over_faint_model(mut model: Drawing2D, over: Drawing2D) -> Drawing2D {
    fade(&mut model);
    model.prims.extend(over.prims);
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::{Category, ViewCategory};
    use glam::DVec2;

    fn sample() -> (Project, ModelGeometry) {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        (project, geom)
    }

    /// The sun's shadows of a drawing: black veils that belong to no element, by opacity.
    fn shadow_veils(d: &Drawing2D) -> Vec<u8> {
        d.prims
            .iter()
            .filter_map(|p| match p {
                Prim::Fill { color: [0, 0, 0, a], element: None, .. } => Some(*a),
                _ => None,
            })
            .collect()
    }

    /// A view whose sun casts shadows draws them in the shaded style: on
    /// the faces of elevations and 3D views, and in plans on what is not
    /// indoors and on the ground around the model. No shadows
    /// with them off, in hidden line or with the sun below the horizon;
    /// hidden elements cast none.
    #[test]
    fn the_sun_casts_shadows_in_2d_views() {
        use cc_core::sun::{SunMode, SunSettings};
        let (project, geom) = sample();
        let sun = |altitude: f64, shadows: bool| SunSettings {
            mode: SunMode::Lighting { azimuth: 235.0, altitude, relative_to_view: false },
            shadows,
            show_path: false,
            ..Default::default()
        };
        let ground = shadow::ground_veil(&cc_core::ShadingSettings::default());
        for category in [ViewCategory::Elevation, ViewCategory::FloorPlan, ViewCategory::ThreeD] {
            let mut v = project.views_of(category).next().unwrap().clone();
            if category == ViewCategory::Elevation {
                v = project.views.values().find(|v| v.name == "Sul").unwrap().clone();
            }
            v.visual_style = Some(cc_core::VisualStyle::Shaded);
            let draw = |v: &View| generate(&project, &geom, v).unwrap();
            v.sun = Some(sun(35.0, false));
            let plain = draw(&v);
            assert!(shadow_veils(&plain).is_empty(), "{category:?}: shadows off");
            v.sun = Some(sun(35.0, true));
            let cast = draw(&v);
            let veils = shadow_veils(&cast);
            assert!(veils.iter().any(|a| *a != ground), "{category:?}: shadows on faces");
            let on_ground = veils.iter().filter(|a| **a == ground).count();
            assert_eq!(on_ground > 0, category != ViewCategory::Elevation, "{category:?}: {on_ground} on the ground");
            // Everything else is drawn as without them, in the same order.
            let rest: Vec<&Prim> = cast
                .prims
                .iter()
                .filter(|p| !matches!(p, Prim::Fill { color: [0, 0, 0, _], element: None, .. }))
                .collect();
            assert_eq!(rest.len(), plain.prims.len(), "{category:?}");
            for (i, (a, b)) in rest.iter().zip(&plain.prims).enumerate() {
                assert_eq!(*a, b, "{category:?}: primitive {i}");
            }
            // Hidden line shows none; neither does a sun at the horizon.
            let hidden = View { visual_style: Some(cc_core::VisualStyle::HiddenLine), ..v.clone() };
            assert!(shadow_veils(&draw(&hidden)).is_empty(), "{category:?}: hidden line");
            let night = View { sun: Some(sun(0.0, true)), ..v.clone() };
            assert!(shadow_veils(&draw(&night)).is_empty(), "{category:?}: no sun");
            // With every category that stands hidden, nothing is left to cast or to catch.
            let mut bare = v.clone();
            for e in project.elements.values().filter(|e| geom.solids.iter().any(|s| s.element == e.id)) {
                bare.graphics.hidden.insert(e.id);
            }
            assert!(shadow_veils(&draw(&bare)).is_empty(), "{category:?}: everything hidden");
            // The roof alone hidden: fewer shadows in the elevation (its eaves cast none).
            if category == ViewCategory::Elevation {
                let mut open = v.clone();
                open.graphics.hidden.extend(project.elements_of(Category::Roof).map(|e| e.id));
                assert!(shadow_veils(&draw(&open)).len() < veils.len());
            }
        }
    }

    /// A section through the sample house sees only its rooms beyond its
    /// line: no shadows, where catching them as an elevation does it would
    /// draw the roof's and the walls' on the far wall. In the floor plan
    /// no shadow lies indoors, though the whole house casts on the ground.
    /// A ceiling plan draws none.
    #[test]
    fn rooms_and_ceilings_get_no_shadows() {
        use cc_core::sun::{SunMode, SunSettings};
        let (project, geom) = sample();
        let lit = |category: ViewCategory| {
            let mut v = project.views_of(category).next().unwrap().clone();
            v.visual_style = Some(cc_core::VisualStyle::Shaded);
            v.sun = Some(SunSettings {
                mode: SunMode::Lighting { azimuth: 235.0, altitude: 35.0, relative_to_view: false },
                shadows: true,
                show_path: false,
                ..Default::default()
            });
            v
        };
        let v = lit(ViewCategory::Section);
        assert!(Look::casts_shadows(&v));
        assert!(shadow_veils(&generate(&project, &geom, &v).unwrap()).is_empty());
        // The rooms are indoors, whether or not a room was placed in them; the garden is not.
        let indoors = Indoors::of(&project, &geom, None, v.detail);
        assert!(!indoors.failed);
        for room in &project.elements_of(Category::Room).collect::<Vec<_>>() {
            let cc_core::ElementKind::Room(r) = &room.kind else { panic!() };
            let z = project.level_elevation(room.level) + 1.5;
            assert!(indoors.holds(r.position.extend(z)), "{:?}", r.position);
        }
        assert!(project.elements_of(Category::Room).count() > 0);
        let (lo, hi) = geom.bounds().unwrap();
        assert!(!indoors.holds(DVec2::new(lo.x - 1.0, lo.y - 1.0).extend(1.5)));
        assert!(!indoors.holds(DVec2::new(hi.x + 1.0, (lo.y + hi.y) / 2.0).extend(1.5)));
        // The same section catching shadows on every face.
        let ViewKind::Section { start, end, depth, .. } = v.kind else { panic!() };
        let all = section(&project, &geom, (start, end), depth, v.scale, Look::of(&project, &geom, &v, None, None));
        assert!(!shadow_veils(&all).is_empty());
        // The floor plan: no shadow holds a room's point.
        let plan = generate(&project, &geom, &lit(ViewCategory::FloorPlan)).unwrap();
        assert!(!shadow_veils(&plan).is_empty());
        for room in project.elements_of(Category::Room) {
            let cc_core::ElementKind::Room(r) = &room.kind else { panic!() };
            let over = plan.prims.iter().any(|p| {
                matches!(p, Prim::Fill { pts, color: [0, 0, 0, a], element: None, .. }
                    if *a > 0 && cc_geom::polygon::contains(pts, r.position))
            });
            assert!(!over, "a shadow in the room at {:?}", r.position);
        }
        let c = lit(ViewCategory::CeilingPlan);
        assert!(!Look::casts_shadows(&c));
        assert!(shadow_veils(&generate(&project, &geom, &c).unwrap()).is_empty());
    }

    /// The names of the grids `d` draws, with where each line stands (x) when vertical.
    fn grids_drawn(project: &Project, d: &Drawing2D) -> Vec<(String, f64)> {
        let mut found: Vec<(String, f64)> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Line { a, b, style: LineStyle::Center, element: Some(id), .. } => {
                    let cc_core::ElementKind::Grid(g) = &project.element(*id)?.kind else { return None };
                    assert!((a.x - b.x).abs() < 1e-9 && b.y > a.y, "a vertical line, bubble on top");
                    Some((g.name.clone(), a.x))
                }
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }

    /// Issue #332: elevations and sections draw the grids square to them,
    /// where they stand; the view's graphics hide them.
    #[test]
    fn elevations_and_sections_draw_the_grids_square_to_them() {
        let (mut project, geom) = sample();
        let names = |found: &[(String, f64)]| found.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
        let elevation = |project: &Project, direction| {
            let view = project
                .views
                .values()
                .find(|v| matches!(v.kind, ViewKind::Elevation { direction: d } if d == direction))
                .unwrap();
            grids_drawn(project, &generate(project, &geom, view).unwrap())
        };
        // Looking north (from the south): the grids along Y, at their x.
        let south = elevation(&project, cc_core::Direction::South);
        assert_eq!(names(&south), ["1", "2", "3"]);
        for ((_, x), expected) in south.iter().zip([0.0, 5.0, 9.0]) {
            assert!((x - expected).abs() < 1e-9, "{x} is not {expected}");
        }
        assert_eq!(names(&elevation(&project, cc_core::Direction::East)), ["A", "B"]);

        // A section along X from -1 to 6 looking south: grid 3 stands past its end.
        let id = project.views.values().find(|v| matches!(v.kind, ViewKind::Section { .. })).unwrap().id;
        let section = |project: &mut Project, depth| {
            let view = project.views.get_mut(&id).unwrap();
            view.kind =
                ViewKind::Section { start: DVec2::new(-1.0, 3.0), end: DVec2::new(6.0, 3.0), depth, segments: None };
            let view = view.clone();
            let d = generate(project, &geom, &view).unwrap();
            // The bubbles stand over everything else the view draws.
            let top = d.prims.iter().filter(|p| p.element().is_none()).filter_map(|p| match p {
                Prim::Line { a, b, .. } => Some(a.y.max(b.y)),
                _ => None,
            });
            let top = top.fold(f64::MIN, f64::max);
            for p in &d.prims {
                if let Prim::Line { b, style: LineStyle::Center, .. } = p {
                    assert!(b.y > top, "the grid ends above the level lines");
                }
            }
            grids_drawn(project, &d)
        };
        let cut = section(&mut project, 2.0);
        assert_eq!(names(&cut), ["1", "2"]);
        assert!((cut[0].1 + 1.0).abs() < 1e-9 && (cut[1].1 + 6.0).abs() < 1e-9, "{cut:?}");
        // Grids that end before the cut and lie behind it are not seen.
        for e in project.elements.values_mut() {
            if let cc_core::ElementKind::Grid(g) = &mut e.kind
                && g.name == "2"
            {
                (g.start.y, g.end.y) = (4.0, 7.5);
            }
        }
        assert_eq!(names(&section(&mut project, 2.0)), ["1"]);

        // Hidden with their category, in that view only.
        let view = project.views.get_mut(&id).unwrap();
        let mut hidden = view.graphics.category(Category::Grid);
        hidden.visible = false;
        view.graphics.set_category(Category::Grid, hidden);
        assert!(section(&mut project, 2.0).is_empty());
        assert_eq!(names(&elevation(&project, cc_core::Direction::South)), ["1", "2", "3"]);
    }

    /// Issue #154: the plan of the level below drawn halftone under a plan,
    /// belonging to no element; none under the lowest level.
    #[test]
    fn plans_show_the_level_below_halftone() {
        let (mut project, _) = sample();
        let (ground, first) = (project.levels[0].id, project.levels[1].id);
        let below = cc_core::Underlay {
            below: Some(cc_core::UnderlayPlan { level: cc_core::UnderlayLevel::Next, looking: cc_core::Looking::Down }),
            ..Default::default()
        };
        for v in project.views.values_mut() {
            if let Some(u) = v.kind.underlay_mut() {
                *u = below;
            }
        }
        let geom = ModelGeometry::build(&project);
        let plan = |level| project.views_of(ViewCategory::FloorPlan).find(|v| v.level() == Some(level)).unwrap();
        let upper = generate(&project, &geom, plan(first)).unwrap();
        assert!(upper.underlay > 20, "the ground floor's walls: {}", upper.underlay);
        for prim in &upper.prims[..upper.underlay] {
            assert_eq!(prim.element(), None);
            if let Prim::Line { style, .. } = prim {
                assert_eq!(*style, LineStyle::Faint);
            }
        }
        assert!(upper.own_prims().iter().any(|p| p.element().is_some()), "its own model over it");
        assert_eq!(upper.without_underlay().prims.len(), upper.prims.len() - upper.underlay);
        // Nothing is below the ground floor.
        assert_eq!(generate(&project, &geom, plan(ground)).unwrap().underlay, 0);
        // A level of its own: looking up, as a ceiling plan.
        let up = cc_core::Underlay {
            above: Some(cc_core::UnderlayPlan {
                level: cc_core::UnderlayLevel::Level(first),
                looking: cc_core::Looking::Up,
            }),
            ..Default::default()
        };
        assert_eq!(up.plans(&project.levels, ground), vec![(first, cc_core::Looking::Up)]);
        assert!(up.plans(&project.levels, first).is_empty(), "not itself");
    }

    #[test]
    fn ground_plan_shows_rooms_grids_and_dimensions() {
        let (project, geom) = sample();
        let level = project.levels[0].id;
        let view = project.views_of(ViewCategory::FloorPlan).find(|v| v.level() == Some(level)).unwrap();
        let d = generate(&project, &geom, view).unwrap();
        let texts: Vec<&str> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for expected in ["Sala", "Quarto", "1", "A", "B"] {
            assert!(texts.contains(&expected), "missing {expected:?} in {texts:?}");
        }
        // The overall dimension of the south wall.
        assert!(texts.contains(&format::number(9.0, 2).as_str()), "{texts:?}");
        assert!(d.prims.iter().any(|p| matches!(p, Prim::Line { style: LineStyle::Center, .. })));
        // Door and window marks are tagged.
        assert!(texts.iter().filter(|t| **t == "2").count() >= 2);

        // The upper plan has the grids but neither the rooms nor the dimensions.
        let upper =
            project.views_of(ViewCategory::FloorPlan).find(|v| v.level() == Some(project.levels[1].id)).unwrap();
        let d = generate(&project, &geom, upper).unwrap();
        let has = |s: &str| d.prims.iter().any(|p| matches!(p, Prim::Text { text, .. } if text == s));
        assert!(has("A") && !has("Sala") && !has(&format::number(9.0, 2)));
    }

    #[test]
    fn room_schedule_lists_enclosed_rooms() {
        let (project, geom) = sample();
        let table = schedule(&project, &geom, Category::Room, &Default::default());
        assert_eq!(table.rows.len(), 2);
        // Interior of the left room: 5 m minus half walls, by 6 m minus half walls.
        let area = (5.0 - 0.075 - 0.075) * (6.0 - 0.15);
        assert_eq!(table.rows[0].cells[3], Cell::Text(format::area(area)));
    }

    /// Zoom to fit a plan frames the model, not the elevation markers around it
    /// nor a section's far clip line.
    #[test]
    fn plans_fit_to_the_model() {
        let (project, geom) = sample();
        let plan = project.views.values().find(|v| matches!(v.kind, ViewKind::FloorPlan { .. })).unwrap();
        let d = generate(&project, &geom, plan).unwrap();
        let (lo, hi) = d.bounds().unwrap();
        let (flo, fhi) = d.fit_bounds().unwrap();
        let (mlo, mhi) = geom.bounds().unwrap();
        // The markers stand outside on every side; the fitted box holds the model.
        assert!((flo - lo).min_element() > 0.5 && (hi - fhi).min_element() > 0.5, "{lo} {hi} / {flo} {fhi}");
        assert!(flo.x <= mlo.x && flo.y <= mlo.y && fhi.x >= mhi.x && fhi.y >= mhi.y);
        // Other views fit to everything drawn.
        let elevation = project.views.values().find(|v| matches!(v.kind, ViewKind::Elevation { .. })).unwrap();
        let d = generate(&project, &geom, elevation).unwrap();
        assert_eq!(d.fit_bounds(), d.bounds());
    }

    #[test]
    fn sheet_places_views_at_scale_and_exports() {
        let (project, geom) = sample();
        let sheet = project.views_of(ViewCategory::Sheet).next().unwrap().clone();
        let ViewKind::Sheet { viewports, .. } = &sheet.kind else { panic!("not a sheet") };
        assert!(!viewports.is_empty());
        let env = SheetEnv {
            today: (2026, 1, 1),
            format_date: |y, m, d| format!("{y}-{m}-{d}"),
            as_indicated: String::new(),
        };
        let composed = compose_sheet(&project, &sheet, &env, |id| {
            project.view(id).and_then(|v| generate(&project, &geom, v)).map(std::rc::Rc::new)
        })
        .unwrap();
        assert_eq!(composed.paper, DVec2::new(420.0, 297.0));
        assert_eq!(composed.viewports.len(), viewports.len());
        // Each viewport keeps its view's extent at the view scale.
        for placed in &composed.viewports {
            let view = project.view(placed.view).unwrap();
            let (lo, hi) = generate(&project, &geom, view).unwrap().bounds().unwrap();
            let k = 1000.0 / view.scale as f64;
            assert!(((hi - lo) * 0.5 * k - placed.half).abs().max_element() < 1e-9);
        }
        let texts: Vec<&str> = composed
            .drawing
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"PROJETO"), "the standard title block's caption: {texts:?}");
        assert!(texts.contains(&project.info.name.as_str()));
        // Everything lies on the paper.
        let (lo, hi) = composed.drawing.bounds().unwrap();
        assert!(lo.min_element() >= 0.0 && hi.x <= 420.0 && hi.y <= 297.0, "{lo} {hi}");
        let svg = export::to_svg(&composed.drawing, composed.paper);
        assert!(svg.matches("<line ").count() > 100);
        let pdf = export::to_pdf(&composed.drawing, composed.paper);
        assert!(pdf.starts_with(b"%PDF") && pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn placed_component_appears_in_3d_plan_schedule_and_legend() {
        let mut project = Project::from_template();
        let level = project.levels[0].id;
        let table = project
            .types_of(Category::Component)
            .find(|t| matches!(&t.data, cc_core::TypeData::Component { def, .. } if def.items.len() == 5))
            .unwrap()
            .id;
        let id = cc_core::ElementId(project.alloc_id());
        let component = cc_core::Component {
            position: DVec2::new(2.0, 3.0),
            rotation_deg: 90.0,
            base_offset: 0.0,
            overrides: Default::default(),
            properties: Vec::new(),
            system: None,
            ..Default::default()
        };
        project.elements.insert(
            id,
            cc_core::Element {
                id,
                level,
                type_id: table,
                mark: "M1".into(),
                comments: String::new(),
                kind: cc_core::ElementKind::Component(component),
                uid: Default::default(),
                phase_created: Default::default(),
                phase_demolished: None,
                design_option: None,
                paints: Vec::new(),
                plugin_data: Default::default(),
            },
        );
        assert_eq!(project.repair(), 0);
        let geom = ModelGeometry::build(&project);
        assert_eq!(geom.solids_of(id).count(), 5);
        // Rotated 90°: the 1.20 m width runs along Y.
        let (lo, hi) =
            geom.solids_of(id).filter_map(|s| s.bounds()).fold(
                (glam::DVec3::splat(f64::INFINITY), glam::DVec3::splat(f64::NEG_INFINITY)),
                |(lo, hi), (a, b)| (lo.min(a), hi.max(b)),
            );
        assert!((hi - lo - glam::DVec3::new(0.8, 1.2, 0.75)).abs().max_element() < 1e-9, "{lo} {hi}");
        // In plan only the top (marked for plans) is drawn: one closed outline of 4 lines.
        let d = plan::plan(&project, &geom, level, 1.2, false, 50);
        let lines = d.prims.iter().filter(|p| matches!(p, Prim::Line { element: Some(e), .. } if *e == id)).count();
        assert_eq!(lines, 4);
        assert_eq!(d.hit_test(DVec2::new(2.4, 3.0), 0.05), Some(id));
        // Schedule extents come from the pieces, in the component's own frame.
        let table = schedule(&project, &geom, Category::Component, &Default::default());
        let row = table.rows.iter().find(|r| r.element == id).unwrap();
        let cells: Vec<String> = row.cells.iter().map(|c| c.raw().to_string()).collect();
        assert!(cells.contains(&format::number(1.2, 2)), "{cells:?}");
        assert!(cells.contains(&format::number(0.8, 2)), "{cells:?}");
        // The legend draws every library component.
        let types: Vec<_> = project.types_of(Category::Component).map(|t| t.id).collect();
        let legend = legend(&project, &types, 50);
        assert!(legend.prims.iter().filter(|p| matches!(p, Prim::Line { .. })).count() > 20);
    }

    #[test]
    fn analytical_view_draws_members_and_nodes() {
        let project = cc_core::from_ron(include_str!("../../cc_ifc/tests/fixtures/structure.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        let model = cc_geom::analytical::analytical_model(&project, &geom);
        let mut view = project.views_of(ViewCategory::ThreeD).next().unwrap().clone();
        let ViewKind::ThreeD { analytical, .. } = &mut view.kind else { panic!("3D view") };
        *analytical = true;
        let d = generate(&project, &geom, &view).unwrap();
        // One line per member (tagged with its element), one marker per node.
        let members = d.prims.iter().filter(|p| matches!(p, crate::Prim::Line { element: Some(_), .. })).count();
        assert_eq!(members, model.members.len());
        assert!(model.nodes.iter().any(|n| n.support.is_some()));
        // The physical model is not drawn.
        assert!(!d.prims.iter().any(|p| matches!(p, crate::Prim::Fill { .. })));
    }

    #[test]
    fn structural_plans_show_only_the_structure_tagged() {
        let project = cc_core::from_ron(include_str!("../../cc_ifc/tests/fixtures/structure.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        let mut view = project.views_of(ViewCategory::FloorPlan).next().unwrap().clone();
        let elements = |d: &Drawing2D| -> std::collections::BTreeSet<cc_core::ElementId> {
            d.prims
                .iter()
                .filter_map(|p| match p {
                    crate::Prim::Line { element, .. } | crate::Prim::Fill { element, .. } => *element,
                    _ => None,
                })
                .collect()
        };
        let ordinary = generate(&project, &geom, &view).unwrap();
        let ViewKind::FloorPlan { structural, .. } = &mut view.kind else { panic!("floor plan") };
        *structural = true;
        let d = generate(&project, &geom, &view).unwrap();
        // The (non-structural) walls go; beams and columns stay.
        let walls: Vec<_> = project.walls().map(|(e, _)| e.id).collect();
        assert!(walls.iter().all(|w| elements(&ordinary).contains(w)));
        assert!(walls.iter().all(|w| !elements(&d).contains(w)));
        let beams = project.elements_of(Category::Beam).filter(|e| e.level == view.level().unwrap()).count();
        let texts: Vec<&str> = d
            .prims
            .iter()
            .filter_map(|p| match p {
                crate::Prim::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // Beams and columns are tagged with their (numbered) mark and section in centimeters.
        let tagged = |prefix: char, section: &str| {
            texts.iter().filter(|t| t.starts_with(prefix) && t.ends_with(&format!(" {section}"))).count()
        };
        assert!(tagged('V', "15×40") >= beams.min(1), "{texts:?}");
        assert!(tagged('P', "20×20") >= 1, "{texts:?}");
    }
}
