//! A 3D view's sun: the light the renderer draws it with
//! ([`cc_render::Light`]) and its sun path diagram painted over the image
//! ([`cc_views::sun_path::diagram`], projected through the live camera).
//!
//! Both are resolved once per document revision and kept: a sun placed
//! relative to the view (lighting `relative_to_view`) is resolved again
//! when the live camera turns, so it follows the viewer while orbiting;
//! every other sun stays put. The diagram's ring and paths do not depend on
//! the camera; only the sun's disc of a view-relative sun moves with it.
//! The diagram is drawn over the model (never hidden by it) and is not
//! picked.

use crate::modify3d::Frame;
use cc_core::sun::SunSettings;
use cc_core::{CameraState, Project, View};
use cc_geom::ModelGeometry;
use cc_views::sun_path::{self as sp, SunDiagram, SunLabelKind, SunLineKind};
use egui::{Color32, FontId, Painter, Pos2, Shape, Stroke};
use glam::{DVec2, DVec3};

/// Screen points per millimeter of the 2D pens' weights.
const PX_PER_MM: f32 = 8.0;
/// Radius of an hour's dot, points.
const DOT_PX: f32 = 3.0;
/// Smallest radius of the sun's disc, points.
const MIN_SUN_PX: f32 = 8.0;
/// Gap between a label and its point, points.
const LABEL_GAP_PX: f32 = 4.0;
/// Text sizes, points.
const CARDINAL_PX: f32 = 15.0;
const DATE_PX: f32 = 12.0;
const TIME_PX: f32 = 11.0;
/// Dashes of the range paths, points.
const DASH_PX: f32 = 6.0;
const GAP_PX: f32 = 4.0;
/// The colours of the live view's texts (the 2D drawing's are black).
const SUNRISE_TEXT: [u8; 3] = [230, 140, 0];
const SUNSET_TEXT: [u8; 3] = [210, 60, 50];
const DATE_TEXT: [u8; 3] = [40, 80, 190];

/// A 3D view's resolved sun.
#[derive(Default)]
pub struct SunView {
    /// The revision and, for a sun relative to the view, the live camera's
    /// yaw the light was resolved for.
    key: Option<(u64, Option<u64>)>,
    light: Option<cc_render::Light>,
    /// The unit vector towards the sun, when the view has one.
    direction: Option<DVec3>,
    /// The sun is above the horizon.
    up: bool,
    /// The revision the diagram was built for.
    diagram_key: Option<u64>,
    diagram: Option<SunDiagram>,
    /// The sun is placed relative to the view (the disc follows the camera).
    follows_view: bool,
}

/// The light of `sun` at `project`'s site seen through `camera`: shadows
/// only while the sun is up, onto the ground level's plane when one is set.
fn light(project: &Project, sun: &SunSettings, camera: &CameraState) -> (cc_render::Light, DVec3, bool) {
    let state = sun.state(&project.sun_site(), cc_core::sun::camera_forward(camera));
    let shadows = sun.shadows && state.above_horizon;
    let ground = sun
        .ground
        .filter(|_| shadows)
        .and_then(|l| project.level(l))
        .map(|l| l.elevation as f32)
        .filter(|z| z.is_finite());
    let light = cc_render::Light { direction: state.direction.as_vec3(), shadows, ground };
    (light, state.direction, state.above_horizon)
}

impl SunView {
    /// Bring the light and the diagram up to date for `view` at document
    /// `revision`, the viewer looking through `camera`; returns the light
    /// (`None`: the view has no sun, and the camera's light is used).
    pub fn update(
        &mut self,
        project: &Project,
        geom: &ModelGeometry,
        view: &View,
        revision: u64,
        camera: &CameraState,
    ) -> Option<cc_render::Light> {
        let follows = view.sun.as_ref().is_some_and(SunSettings::follows_view);
        let key = (revision, follows.then_some(camera.yaw_deg.to_bits()));
        if self.key != Some(key) {
            self.key = Some(key);
            self.follows_view = follows;
            match &view.sun {
                Some(sun) => {
                    let (light, direction, up) = light(project, sun, camera);
                    (self.light, self.direction, self.up) = (Some(light), Some(direction), up);
                }
                None => (self.light, self.direction, self.up) = (None, None, false),
            }
        }
        if self.diagram_key != Some(revision) {
            self.diagram_key = Some(revision);
            self.diagram = sp::diagram(project, geom, view);
        }
        self.light
    }

    /// Paint the sun path diagram (when the view shows it) through `frame`.
    pub fn paint(&self, painter: &Painter, frame: &Frame) {
        let Some(d) = &self.diagram else { return };
        let painter = painter.with_clip_rect(frame.rect);
        // A view-relative sun stands where the live camera puts it.
        let sun = match (self.follows_view, self.direction) {
            (true, Some(dir)) => self.up.then(|| d.center + dir * d.radius),
            _ => d.sun,
        };
        paint_diagram(&painter, frame, d, sun);
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn width(weight_mm: f64) -> f32 {
    (weight_mm as f32 * PX_PER_MM).max(1.0)
}

/// The visible runs of a polyline on screen: segments clipped at the near
/// plane, consecutive ones joined while they meet.
fn runs(frame: &Frame, pts: &[DVec3], closed: bool) -> Vec<Vec<Pos2>> {
    let n = pts.len();
    let count = if closed && n > 2 { n } else { n.saturating_sub(1) };
    let mut out: Vec<Vec<Pos2>> = Vec::new();
    for i in 0..count {
        let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else { continue };
        let Some((pa, pb)) = frame.segment(*a, *b) else { continue };
        match out.last_mut() {
            Some(run) if run.last().is_some_and(|last| last.distance(pa) < 1e-3) => run.push(pb),
            _ => out.push(vec![pa, pb]),
        }
    }
    out
}

/// The band between the ring's two circles, when both show whole.
fn band(frame: &Frame, d: &SunDiagram) -> Option<Shape> {
    let mut rings = d.lines.iter().filter(|l| l.kind == SunLineKind::Ring);
    let (inner, outer) = (rings.next()?, rings.next()?);
    let project = |pts: &[DVec3]| pts.iter().map(|p| frame.screen(*p)).collect::<Option<Vec<Pos2>>>();
    let (inner, outer) = (project(&inner.pts)?, project(&outer.pts)?);
    let n = inner.len().min(outer.len());
    if n < 3 {
        return None;
    }
    let color = rgba(sp::BAND_COLOR);
    let mut mesh = egui::Mesh::default();
    for (a, b) in inner.iter().zip(&outer).take(n) {
        mesh.colored_vertex(*a, color);
        mesh.colored_vertex(*b, color);
    }
    let n = n as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        let (a0, b0, a1, b1) = (2 * i, 2 * i + 1, 2 * j, 2 * j + 1);
        mesh.add_triangle(a0, b0, b1);
        mesh.add_triangle(a0, b1, a1);
    }
    Some(Shape::mesh(mesh))
}

fn paint_diagram(painter: &Painter, frame: &Frame, d: &SunDiagram, sun: Option<DVec3>) {
    if let Some(band) = band(frame, d) {
        painter.add(band);
    }
    let order = [SunLineKind::Ring, SunLineKind::Tick, SunLineKind::Axis, SunLineKind::PathRange, SunLineKind::Path];
    for kind in order {
        let (color, weight) = match kind {
            SunLineKind::Ring | SunLineKind::Tick | SunLineKind::Axis => (sp::GRID_COLOR, sp::GRID_WEIGHT),
            SunLineKind::PathRange => (sp::PATH_COLOR, sp::RANGE_WEIGHT),
            SunLineKind::Path => (sp::PATH_COLOR, sp::PATH_WEIGHT),
        };
        let stroke = Stroke::new(width(weight), rgb(color));
        for l in d.lines.iter().filter(|l| l.kind == kind) {
            for run in runs(frame, &l.pts, l.closed) {
                if kind == SunLineKind::PathRange {
                    painter.extend(Shape::dashed_line(&run, stroke, DASH_PX, GAP_PX));
                } else {
                    painter.add(Shape::line(run, stroke));
                }
            }
        }
    }
    let dot = rgb(sp::PATH_COLOR);
    for p in d.dots.iter().filter_map(|p| frame.screen(*p)) {
        painter.circle_filled(p, DOT_PX, dot);
    }
    // The sun's disc, as large as it stands on the dome (and no smaller than a mark).
    let mut sun_px = 0.0;
    if let Some(s) = sun
        && let Some(at) = frame.screen(s)
    {
        let right = frame.camera.forward().cross(DVec3::Z).try_normalize().unwrap_or(DVec3::X);
        let edge = frame.screen(s + right * d.sun_radius).map_or(0.0, |e| e.distance(at));
        sun_px = if edge.is_finite() { edge.clamp(MIN_SUN_PX, 200.0) } else { MIN_SUN_PX };
        let outline = Stroke::new(width(sp::SUN_OUTLINE_WEIGHT), rgb(sp::SUN_OUTLINE));
        painter.circle(at, sun_px, rgba(sp::SUN_FILL), outline);
    }
    labels(painter, frame, d, sun, sun_px);
}

/// The text colour and font of a label.
fn style(kind: SunLabelKind) -> (Color32, FontId) {
    match kind {
        SunLabelKind::Cardinal => (rgb(sp::GRID_COLOR), FontId::new(CARDINAL_PX, crate::theme::semibold())),
        SunLabelKind::Sunrise => (rgb(SUNRISE_TEXT), FontId::proportional(TIME_PX)),
        SunLabelKind::Sunset => (rgb(SUNSET_TEXT), FontId::proportional(TIME_PX)),
        SunLabelKind::Date => (rgb(DATE_TEXT), FontId::new(DATE_PX, crate::theme::semibold())),
        SunLabelKind::Now => (rgb(DATE_TEXT), FontId::new(TIME_PX, crate::theme::semibold())),
    }
}

/// The labels, laid out as the 2D drawing lays them out (see
/// [`sp::label_anchor`], [`sp::stacked_on`]) but measured on screen.
fn labels(painter: &Painter, frame: &Frame, d: &SunDiagram, sun: Option<DVec3>, sun_px: f32) {
    let galleys: Vec<_> = d
        .labels
        .iter()
        .map(|l| {
            let (color, font) = style(l.kind);
            let text = sp::label_text(l.text, &|k: &str| crate::i18n::tr(k));
            painter.layout_no_wrap(text, font, color)
        })
        .collect();
    let size = |i: usize| galleys.get(i).map_or(DVec2::ZERO, |g| DVec2::new(g.size().x as f64, g.size().y as f64));
    for (i, l) in d.labels.iter().enumerate() {
        let Some(galley) = galleys.get(i) else { continue };
        if galley.is_empty() {
            continue;
        }
        // The current time follows the disc of a sun that moved with the camera.
        let point = match (l.kind, sun) {
            (SunLabelKind::Now, Some(s)) => s,
            _ => sp::label_anchor(d, l),
        };
        let Some(at) = frame.screen(point) else { continue };
        // The label's direction on screen, from a short step along `out`;
        // the current time beside the disc, on the side away from the
        // compass's center (the sky above the disc may be off the view).
        let step = frame.screen(point + l.out * (0.05 * d.radius)).map(|p| p - at);
        let center = frame.screen(d.center).unwrap_or(frame.rect.center());
        let dir = match step {
            _ if l.kind == SunLabelKind::Now => DVec2::new(if at.x < center.x { -1.0 } else { 1.0 }, 0.0),
            Some(v) if v.length() > 2.0 => DVec2::new(v.x as f64, v.y as f64).normalize(),
            _ if sp::on_horizon(d, l) => DVec2::NEG_Y,
            _ => DVec2::ZERO,
        };
        let mut clear = match l.kind {
            SunLabelKind::Cardinal => 0.0,
            SunLabelKind::Now => f64::from(sun_px + LABEL_GAP_PX),
            _ => f64::from(LABEL_GAP_PX),
        };
        if let Some(j) = sp::stacked_on(d, i) {
            let under = size(j);
            clear += 2.0 * sp::half_extent(dir, under.x, under.y) + f64::from(LABEL_GAP_PX);
        }
        let own = size(i);
        let offset = dir * (clear + sp::half_extent(dir, own.x, own.y));
        let center = at + egui::vec2(offset.x as f32, offset.y as f32);
        if !(center.x.is_finite() && center.y.is_finite()) {
            continue;
        }
        let top_left = center - galley.size() * 0.5;
        // A soft light halo keeps the text legible over the model.
        let halo = Color32::from_white_alpha(170);
        for o in [egui::vec2(1.0, 0.0), egui::vec2(-1.0, 0.0), egui::vec2(0.0, 1.0), egui::vec2(0.0, -1.0)] {
            painter.galley_with_override_text_color(top_left + o, galley.clone(), halo);
        }
        painter.galley(top_left, galley.clone(), Color32::PLACEHOLDER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::sun::{SunDate, SunMode};

    fn sample() -> (Project, ModelGeometry) {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let geom = ModelGeometry::build(&project);
        (project, geom)
    }

    fn three_d(project: &Project) -> View {
        project.views.values().find(|v| matches!(v.kind, cc_core::ViewKind::ThreeD { .. })).unwrap().clone()
    }

    /// No sun: no light (the camera's) and no diagram. A dated sun keeps its
    /// light while the camera turns; a view-relative light turns with it.
    #[test]
    fn the_light_follows_the_camera_only_for_a_view_relative_sun() {
        let (project, geom) = sample();
        let mut view = three_d(&project);
        let camera = CameraState::default();
        let turned = CameraState { yaw_deg: camera.yaw_deg + 90.0, ..camera };
        let mut s = SunView::default();
        assert_eq!(s.update(&project, &geom, &view, 1, &camera), None);
        assert!(s.diagram.is_none());

        let date = SunDate::new(2026, 6, 1).unwrap();
        view.sun = Some(SunSettings {
            mode: SunMode::Still { date, time: 600 },
            shadows: true,
            ground: project.levels.first().map(|l| l.id),
            ..SunSettings::default()
        });
        let mut s = SunView::default();
        let a = s.update(&project, &geom, &view, 1, &camera).unwrap();
        assert!(a.shadows && a.ground.is_some() && a.direction.z > 0.0);
        assert_eq!(s.update(&project, &geom, &view, 1, &turned), Some(a), "orbiting keeps a dated sun");
        assert!(s.diagram.is_some());

        // Shadows off: no ground either.
        view.sun.as_mut().unwrap().shadows = false;
        let off = s.update(&project, &geom, &view, 2, &camera).unwrap();
        assert!(!off.shadows && off.ground.is_none());

        view.sun = Some(SunSettings { mode: SunMode::from_name("lighting").unwrap(), ..SunSettings::default() });
        let mut s = SunView::default();
        let a = s.update(&project, &geom, &view, 3, &camera).unwrap();
        let b = s.update(&project, &geom, &view, 3, &turned).unwrap();
        assert!(a.direction.distance(b.direction) > 0.5, "{a:?} {b:?}");
        // Turned a quarter: the light's plan direction turns a quarter too.
        let (pa, pb) = (a.direction.truncate().normalize(), b.direction.truncate().normalize());
        assert!(pa.dot(pb).abs() < 1e-4, "{pa} {pb}");
        assert!((a.direction.z - b.direction.z).abs() < 1e-6);
    }

    /// A segment through the eye is cut at the near plane, not flung across
    /// the screen; one wholly behind the eye is not drawn.
    #[test]
    fn segments_behind_the_eye_are_clipped() {
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let camera = CameraState { target: DVec3::ZERO, yaw_deg: 0.0, pitch_deg: 0.0, distance: 10.0 };
        let frame = Frame::new(rect, camera);
        let eye = camera.eye();
        let fwd = camera.forward();
        let ahead = eye + fwd * 5.0 + DVec3::Z;
        let behind = eye - fwd * 5.0 + DVec3::Z;
        let (a, b) = frame.segment(ahead, behind).unwrap();
        assert!(a.x.is_finite() && b.y.is_finite());
        // The clipped end lies above the center, on the same side as the segment.
        assert!(b.y < rect.center().y && a.y < rect.center().y, "{a:?} {b:?}");
        assert!(frame.segment(behind, behind - fwd).is_none());
        // A ring around the eye leaves out only what is behind.
        let ring: Vec<DVec3> = (0..32)
            .map(|i| eye + DVec3::from((DVec2::from_angle(i as f64 / 32.0 * std::f64::consts::TAU) * 4.0, -1.0)))
            .collect();
        let parts = runs(&frame, &ring, true);
        assert!(!parts.is_empty());
        for run in &parts {
            assert!(run.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        }
    }
}
