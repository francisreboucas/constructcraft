//! Interactive 3D view backed by [`cc_render::Renderer3D`].

use crate::canvas::{ZoomCommand, ZoomNav};
use crate::state::AppState;
use cc_core::{CameraState, ViewId, ViewKind};
use cc_render::{Renderer3D, SceneMesh, pick_ray};
use egui::{PointerButton, Sense, Ui};
use glam::DVec3;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

pub struct View3DState {
    pub camera: CameraState,
    renderer: Option<Renderer3D>,
    pub texture: Option<egui::TextureId>,
    /// The meshes uploaded to the renderer.
    scene: Scene,
    /// Camera changed since it was last written back to the model.
    camera_dirty: bool,
    fit_requested: bool,
    /// The camera of the last fit and the aspect it fitted: while the camera
    /// is left as fitted, a resized view fits it again.
    auto_fit: Option<(CameraState, f32)>,
    /// Camera transition started by the navigation cube.
    anim: Option<Transition>,
    /// Earlier and undone cameras for "previous" / "next" pan/zoom.
    back: Vec<CameraState>,
    forward: Vec<CameraState>,
    /// Time of the last wheel zoom: a pause starts a new history step.
    last_wheel: f64,
    /// "Zoom in region" waits for a window; the drag's start once it began.
    zoom_region: bool,
    region_start: Option<egui::Pos2>,
    /// Where a window or crossing selection drag started.
    select_start: Option<egui::Pos2>,
    /// Snap points and previews of the modify tools.
    pub(crate) picks: crate::modify3d::PickCache,
    /// The view's sun: the light and the sun path drawn over the image.
    sun: crate::sun_overlay::SunView,
}

/// Cameras kept for "previous" and "next".
const HISTORY: usize = 50;
/// Half the vertical field of view.
const HALF_FOV_DEG: f64 = 22.5;

/// Smooth turn of the camera to new angles.
#[derive(Debug, Clone, Copy)]
struct Transition {
    from_yaw: f64,
    from_pitch: f64,
    /// Yaw change along the shortest way.
    delta_yaw: f64,
    to_pitch: f64,
    start: f64,
}

const TRANSITION_SECONDS: f64 = 0.35;

impl View3DState {
    fn new(camera: CameraState) -> Self {
        Self {
            camera,
            renderer: None,
            texture: None,
            scene: Scene::default(),
            camera_dirty: false,
            fit_requested: false,
            auto_fit: None,
            anim: None,
            back: Vec::new(),
            forward: Vec::new(),
            last_wheel: f64::NEG_INFINITY,
            zoom_region: false,
            region_start: None,
            select_start: None,
            picks: Default::default(),
            sun: Default::default(),
        }
    }

    /// Remember the camera before changing it.
    fn remember(&mut self) {
        if self.back.last() != Some(&self.camera) {
            self.back.push(self.camera);
            if self.back.len() > HISTORY {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    /// Frame the model on the next frame, remembering the camera.
    pub fn request_fit(&mut self) {
        self.remember();
        self.fit_requested = true;
    }

    pub fn nav(&self) -> ZoomNav {
        ZoomNav { back: !self.back.is_empty(), forward: !self.forward.is_empty(), sheet_size: false }
    }

    /// Run a zoom command of the view toolbar ("fit all" is handled by the caller).
    pub fn apply(&mut self, cmd: ZoomCommand) {
        self.anim = None;
        match cmd {
            ZoomCommand::Region => {
                self.zoom_region = true;
                self.region_start = None;
            }
            ZoomCommand::OutTwo => {
                self.remember();
                self.camera.distance = (self.camera.distance * 2.0).clamp(0.5, 5000.0);
            }
            ZoomCommand::Fit | ZoomCommand::FitAll => self.request_fit(),
            // No paper scale in 3D.
            ZoomCommand::SheetSize => return,
            ZoomCommand::Previous | ZoomCommand::Next => {
                let (from, to) = if cmd == ZoomCommand::Previous {
                    (&mut self.back, &mut self.forward)
                } else {
                    (&mut self.forward, &mut self.back)
                };
                let Some(camera) = from.pop() else { return };
                to.push(self.camera);
                self.camera = camera;
            }
        }
        self.camera_dirty = true;
    }

    /// Zoom so that `window` (screen) fills the view: its center becomes the
    /// target (moved in the plane through the target facing the camera) and
    /// the distance shrinks with the window.
    fn zoom_to_window(&mut self, window: egui::Rect, view: egui::Rect) {
        let cam = &mut self.camera;
        let fwd = cam.forward();
        let right = fwd.cross(DVec3::Z).normalize_or_zero();
        let up = right.cross(fwd);
        let k = cam.distance * 2.0 * HALF_FOV_DEG.to_radians().tan() / view.height().max(1.0) as f64;
        let d = window.center() - view.center();
        cam.target += (right * d.x as f64 - up * d.y as f64) * k;
        let fraction = (window.width() / view.width().max(1.0)).max(window.height() / view.height().max(1.0));
        cam.distance = (cam.distance * fraction as f64).clamp(0.5, 5000.0);
        self.camera_dirty = true;
    }

    /// "Zoom in region": drag a window; a click or Esc cancels. Returns true
    /// while it takes the pointer (orbit and picking wait meanwhile).
    fn region(&mut self, ui: &Ui, resp: &egui::Response, painter: &egui::Painter, rect: egui::Rect) -> bool {
        if !self.zoom_region {
            return false;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || resp.clicked() {
            self.zoom_region = false;
            self.region_start = None;
            return true;
        }
        if resp.drag_started_by(PointerButton::Primary) {
            self.region_start = ui.input(|i| i.pointer.press_origin());
        }
        let (Some(a), Some(b)) = (self.region_start, resp.interact_pointer_pos().or(resp.hover_pos())) else {
            return true;
        };
        let window = egui::Rect::from_two_pos(a, b);
        let accent = crate::theme::PALETTE.accent;
        painter.rect_filled(window, 0.0, accent.gamma_multiply(0.08));
        painter.rect_stroke(window, 0.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Middle);
        if resp.drag_stopped() {
            self.zoom_region = false;
            self.region_start = None;
            if window.width() > 4.0 && window.height() > 4.0 {
                self.remember();
                self.zoom_to_window(window, rect);
            }
        }
        true
    }

    fn turn_to(&mut self, yaw: f64, pitch: f64, now: f64) {
        let delta_yaw = (yaw - self.camera.yaw_deg + 540.0).rem_euclid(360.0) - 180.0;
        self.anim = Some(Transition {
            from_yaw: self.camera.yaw_deg,
            from_pitch: self.camera.pitch_deg,
            delta_yaw,
            to_pitch: pitch,
            start: now,
        });
    }

    /// Advance a running transition; returns whether one is running.
    fn animate(&mut self, now: f64) -> bool {
        let Some(a) = self.anim else { return false };
        let t = ((now - a.start) / TRANSITION_SECONDS).clamp(0.0, 1.0);
        let e = t * t * (3.0 - 2.0 * t);
        self.camera.yaw_deg = a.from_yaw + a.delta_yaw * e;
        self.camera.pitch_deg = a.from_pitch + (a.to_pitch - a.from_pitch) * e;
        self.camera_dirty = true;
        if t >= 1.0 {
            self.camera.yaw_deg = self.camera.yaw_deg.rem_euclid(360.0);
            self.anim = None;
        }
        true
    }
}

/// How view graphics show in 3D: the surface color (a solid surface fill or
/// its color), transparency, halftone and the projection line color for edges.
fn solid_style(g: &cc_core::GraphicOverride) -> cc_render::SolidStyle {
    let fill = g.surface_fill;
    let color = match fill.pattern {
        Some(cc_core::Pattern::None) => Some([255, 255, 255]),
        _ => fill.color,
    };
    cc_render::SolidStyle {
        color,
        transparency: g.transparency,
        halftone: g.halftone == Some(true),
        edges: g.projection_lines.color,
    }
}

fn selection_hash(sel: &BTreeSet<cc_core::ElementId>) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    sel.hash(&mut h);
    h.finish()
}

/// What a 3D view shows of the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The solids in the view's graphics and visual style.
    Model,
    /// The analytical model over faint solids.
    Analytical,
    /// The reinforcing bars over faint solids.
    Rebar,
}

/// What a base mesh was built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BaseKey {
    revision: u64,
    temporary: u64,
    mode: Mode,
    /// The hash of the selection built into the mesh: only a selection too
    /// scattered for a highlight (see [`cc_render::MAX_HIDDEN_RUNS`]).
    baked: Option<u64>,
}

/// The renderer as [`Scene::update`] drives it.
trait SceneSink {
    fn set_textures(&mut self, set: &cc_render::TextureSet);
    fn upload(&mut self, mesh: &SceneMesh);
    fn set_highlight(&mut self, highlight: Option<&cc_render::Highlight>);
}

struct Gpu<'a> {
    renderer: &'a mut Renderer3D,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl SceneSink for Gpu<'_> {
    fn set_textures(&mut self, set: &cc_render::TextureSet) {
        self.renderer.set_textures(self.device, self.queue, set);
    }
    fn upload(&mut self, mesh: &SceneMesh) {
        self.renderer.upload(self.device, mesh);
    }
    fn set_highlight(&mut self, highlight: Option<&cc_render::Highlight>) {
        self.renderer.set_highlight(self.device, highlight);
    }
}

/// The meshes of a 3D view: a base mesh that follows the model and the
/// temporary visibility, and the selection drawn over it as a small
/// highlight, so selecting does not rebuild or upload the whole scene.
#[derive(Default)]
struct Scene {
    base: Option<BaseKey>,
    /// Where each solid part sits in the base mesh.
    spans: Vec<cc_render::Span>,
    /// The base mesh and the selection hash the highlight was made for.
    highlight: Option<(BaseKey, u64)>,
    /// Triangles and edges of the geometry shown, with the revision they were made at.
    tessellation: Option<(u64, cc_render::Tessellation)>,
    /// The materials' textures, decoded, with the revision they follow.
    textures: (Option<u64>, cc_render::TextureSet),
    /// The analytical model, with the revision it was made at.
    analytical: Option<(u64, cc_geom::analytical::AnalyticalModel)>,
    /// How many base meshes were built.
    builds: usize,
}

impl Scene {
    /// Bring the meshes in `sink` up to date with the session and `selection`.
    fn update(
        &mut self,
        session: &mut cc_engine::Session,
        view_id: ViewId,
        mode: Mode,
        selection: &BTreeSet<cc_core::ElementId>,
        sink: &mut dyn SceneSink,
    ) {
        session.view_geometry(view_id);
        let (revision, temporary) = (session.doc.revision(), session.temporary_revision());
        let key = |baked| BaseKey { revision, temporary, mode, baked };
        let sel = selection_hash(selection);
        if self.base.is_none_or(|b| b != key(b.baked)) {
            self.build_base(session, view_id, mode, &BTreeSet::new(), sink);
            self.base = Some(key(None));
        }
        let Some(base) = self.base else { return };
        if self.highlight == Some((base, sel)) {
            return;
        }
        // A selection scattered over too many runs of the base mesh is built into it instead.
        let scattered =
            mode == Mode::Model && cc_render::hidden_runs(&self.spans, selection) > cc_render::MAX_HIDDEN_RUNS;
        let baked = scattered.then_some(sel);
        if base.baked != baked {
            let built_with = if scattered { selection.clone() } else { BTreeSet::new() };
            self.build_base(session, view_id, mode, &built_with, sink);
            self.base = Some(key(baked));
        }
        let Some(base) = self.base else { return };
        let highlight = match mode {
            _ if scattered || selection.is_empty() => None,
            Mode::Model => {
                self.tessellation.as_ref().map(|(_, tess)| SceneMesh::highlight(tess, &self.spans, selection))
            }
            Mode::Analytical => {
                self.analytical.as_ref().map(|(_, model)| SceneMesh::analytical_highlight(model, selection))
            }
            Mode::Rebar => Some(SceneMesh::rebar_highlight(session.geom_for(view_id), selection)),
        };
        sink.set_highlight(highlight.as_ref());
        self.highlight = Some((base, sel));
    }

    /// Build and upload the base mesh with `selected` built in (none but a scattered selection).
    fn build_base(
        &mut self,
        session: &cc_engine::Session,
        view_id: ViewId,
        mode: Mode,
        selected: &BTreeSet<cc_core::ElementId>,
        sink: &mut dyn SceneSink,
    ) {
        self.builds += 1;
        self.highlight = None;
        let revision = session.doc.revision();
        let project = &session.doc.project;
        let geom = session.geom_for(view_id);
        if self.tessellation.as_ref().is_none_or(|(r, _)| *r != revision) {
            self.tessellation = Some((revision, cc_render::Tessellation::new(geom)));
        }
        let Some((_, tess)) = &self.tessellation else { return };
        let mut mesh = match mode {
            Mode::Analytical => {
                if self.analytical.as_ref().is_none_or(|(r, _)| *r != revision) {
                    self.analytical = Some((revision, cc_geom::analytical::analytical_model(project, geom)));
                }
                let Some((_, model)) = &self.analytical else { return };
                SceneMesh::build_analytical(tess, model, selected)
            }
            Mode::Rebar => SceneMesh::build_rebar(geom, tess, selected),
            Mode::Model => match project.view(view_id) {
                Some(view) => {
                    let temporary = session.temporary(view_id);
                    let mut resolver = cc_views::graphics::Resolver::new(project, geom, view, temporary);
                    let style = &mut |e| resolver.style(e).map(|g| solid_style(&g));
                    if self.textures.0 != Some(revision) {
                        self.textures.1.update(project);
                        self.textures.0 = Some(revision);
                    }
                    sink.set_textures(&self.textures.1);
                    let textures = &self.textures.1;
                    SceneMesh::build_textured(tess, selected, style, view.detail, view.style(), textures)
                }
                None => SceneMesh::build_plain(tess, selected),
            },
        };
        sink.upload(&mesh);
        self.spans = std::mem::take(&mut mesh.spans);
    }
}

/// Zoom the camera to frame the whole model.
pub fn fit_camera(camera: &mut CameraState, geom: &cc_geom::ModelGeometry, aspect: f32) {
    let Some((lo, hi)) = geom.bounds() else { return };
    camera.target = (lo + hi) * 0.5;
    let radius = ((hi - lo).length() * 0.5).max(1.0);
    // The narrower of the vertical (45°) and horizontal fields of view limits the fit.
    let half_v = HALF_FOV_DEG.to_radians();
    let half_h = (half_v.tan() * aspect as f64).atan();
    camera.distance = radius / half_v.min(half_h).sin() * 1.05;
}

pub fn show(ui: &mut Ui, state: &mut AppState, view_id: ViewId, render_state: Option<&egui_wgpu::RenderState>) {
    let Some(ViewKind::ThreeD { camera, analytical, rebar }) = state.project().view(view_id).map(|v| v.kind.clone())
    else {
        return;
    };
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    let aspect = rect.width() / rect.height().max(1.0);
    let Some(rs) = render_state else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            crate::t!("msg-no-gpu"),
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );
        return;
    };

    // The geometry of the view's phase.
    state.session.view_geometry(view_id);
    let geom = state.session.geom_for(view_id);
    let v3 = state.view3d.entry(view_id).or_insert_with(|| {
        let mut v = View3DState::new(camera);
        if geom.bounds().is_some() && camera == CameraState::default() {
            fit_camera(&mut v.camera, geom, aspect);
            v.auto_fit = Some((v.camera, aspect));
        }
        v
    });
    if v3.fit_requested {
        fit_camera(&mut v3.camera, geom, aspect);
        v3.auto_fit = Some((v3.camera, aspect));
        v3.fit_requested = false;
        v3.camera_dirty = true;
    } else if let Some((camera, fitted)) = v3.auto_fit
        && camera == v3.camera
        && (fitted - aspect).abs() > 1e-3
    {
        fit_camera(&mut v3.camera, geom, aspect);
        v3.auto_fit = Some((v3.camera, aspect));
    }

    // Navigation: left-drag or Shift+middle orbits, middle/right-drag pans, wheel zooms.
    let now = ui.input(|i| i.time);
    if v3.animate(now) {
        ui.ctx().request_repaint();
    }
    let zooming = v3.region(ui, &resp, &painter, rect);
    if resp.dragged() {
        v3.anim = None;
    }
    // A selection box: a left drag with the Select tool, or Shift+drag, as the preference says.
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.drag_started_by(PointerButton::Primary)
        && !zooming
        && state.tool == crate::tools::Tool::Select
        && state.prefs.select_drag_3d != shift
    {
        v3.select_start = ui.input(|i| i.pointer.press_origin());
    }
    if resp.drag_started() && !zooming && v3.select_start.is_none() {
        v3.remember();
    }
    // Wheel or pinch zoom, two-finger (trackpad) pan (see `gestures`).
    let gesture = if resp.hovered() { Some(crate::gestures::read(ui)) } else { None };
    let zoom = gesture.map_or(1.0, |g| g.zoom(0.0025));
    let finger_pan = gesture.map_or(egui::Vec2::ZERO, |g| g.pan);
    if (zoom - 1.0).abs() > 1e-6 || finger_pan != egui::Vec2::ZERO {
        if now - v3.last_wheel > 0.6 {
            v3.remember();
        }
        v3.last_wheel = now;
    }
    if resp.double_clicked_by(PointerButton::Middle) {
        v3.remember();
    }
    let cam = &mut v3.camera;
    let d = resp.drag_delta();
    if zooming || v3.select_start.is_some() {
        // The zoom window and the selection box are not an orbit.
    } else if resp.dragged_by(PointerButton::Primary) || (shift && resp.dragged_by(PointerButton::Middle)) {
        cam.yaw_deg += d.x as f64 * 0.4;
        cam.pitch_deg = (cam.pitch_deg + d.y as f64 * 0.4).clamp(-89.0, 89.0);
        v3.camera_dirty = true;
    } else if resp.dragged_by(PointerButton::Middle)
        || resp.dragged_by(PointerButton::Secondary)
        || finger_pan != egui::Vec2::ZERO
    {
        let d = if finger_pan != egui::Vec2::ZERO { finger_pan } else { d };
        let fwd = cam.forward();
        let right = fwd.cross(DVec3::Z).normalize_or_zero();
        let up = right.cross(fwd);
        let k = cam.distance * 2.0 * HALF_FOV_DEG.to_radians().tan() / rect.height().max(1.0) as f64;
        cam.target += (-right * d.x as f64 + up * d.y as f64) * k;
        v3.camera_dirty = true;
    }
    if (zoom - 1.0).abs() > 1e-6 {
        cam.distance = (cam.distance / zoom).clamp(0.5, 5000.0);
        v3.camera_dirty = true;
    }
    if resp.double_clicked_by(PointerButton::Middle) {
        fit_camera(cam, geom, aspect);
        v3.camera_dirty = true;
    }

    // Picking (the Demolish tool takes the clicked element instead, once the view state is released).
    let mut demolish_pick = None;
    let mut opening_pick = None;
    let mut attach_pick = None;
    let mut paint_pick = None;
    // The modify tools that work in 3D take the clicks themselves (see `modify3d`).
    let modifying = crate::modify3d::works_in_3d(&state.tool);
    if resp.clicked()
        && !zooming
        && !modifying
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let nx = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
        let ny = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
        let ray = pick_ray(&v3.camera, aspect, nx, ny);
        let ctrl = ui.input(|i| i.modifiers.command);
        let (project, temporary) = (&state.session.doc.project, state.session.temporary(view_id));
        let picked = match project.view(view_id) {
            Some(view) => {
                let mut resolver = cc_views::graphics::Resolver::new(project, geom, view, temporary);
                geom.pick_where(ray, &mut |e| resolver.style(e).is_some())
            }
            None => geom.pick(ray, &[]),
        };
        // The graded earth inside a building pad picks the pad.
        let picked = picked.map(|(id, t)| {
            let hit = ray.origin + ray.dir * t;
            (cc_geom::pad::pad_at(project, id, hit.truncate()).unwrap_or(id), t)
        });
        let demolishing = state.tool == crate::tools::Tool::Demolish;
        let placing = matches!(state.tool, crate::tools::Tool::Hosted { .. });
        let attaching = state.tool == crate::tools::Tool::AttachTop;
        let painting = match state.tool {
            crate::tools::Tool::Paint { remove } => Some(remove),
            _ => None,
        };
        match picked {
            _ if painting.is_some() => paint_pick = painting.map(|remove| (remove, picked.map(|(id, _)| id), ray)),
            _ if demolishing => demolish_pick = Some(picked.map(|(id, _)| id)),
            _ if attaching => attach_pick = Some(picked.map(|(id, _)| id)),
            Some((id, t)) if placing && project.element(id).is_some_and(|e| e.as_wall().is_some()) => {
                let hit = ray.origin + ray.dir * t;
                opening_pick = Some((id, glam::DVec2::new(hit.x, hit.y)));
            }
            _ if placing => state.message = crate::t!("hint-hosted"),
            Some((id, _)) if ctrl => {
                if !state.selection.remove(&id) {
                    state.selection.insert(id);
                }
            }
            Some((id, _)) => state.selection = BTreeSet::from([id]),
            None if !ctrl => state.selection.clear(),
            None => {}
        }
    }

    // Rebuild the base mesh when the model or the temporary visibility
    // changed; a new selection only replaces the highlight drawn over it.
    let renderer = v3.renderer.get_or_insert_with(|| Renderer3D::new(&rs.device));
    let mode = match (analytical, rebar) {
        (true, _) => Mode::Analytical,
        (false, true) => Mode::Rebar,
        (false, false) => Mode::Model,
    };
    let mut gpu = Gpu { renderer: &mut *renderer, device: &rs.device, queue: &rs.queue };
    v3.scene.update(&mut state.session, view_id, mode, &state.selection, &mut gpu);
    // The view's sun (else the camera's light), as the live camera sees it.
    let light = match state.session.doc.project.view(view_id) {
        Some(view) => {
            let (project, geom, revision) =
                (&state.session.doc.project, state.session.geom_for(view_id), state.session.doc.revision());
            v3.sun.update(project, geom, view, revision, &v3.camera)
        }
        None => None,
    };
    renderer.set_light(light);
    let ppp = ui.ctx().pixels_per_point();
    let size = [(rect.width() * ppp).round() as u32, (rect.height() * ppp).round() as u32];
    let Some((texture_view, recreated)) = renderer.render(&rs.device, &rs.queue, size, &v3.camera) else { return };
    {
        let mut egui_renderer = rs.renderer.write();
        match v3.texture {
            None => {
                v3.texture =
                    Some(egui_renderer.register_native_texture(&rs.device, texture_view, wgpu::FilterMode::Linear))
            }
            Some(id) if recreated => egui_renderer.update_egui_texture_from_wgpu_texture(
                &rs.device,
                texture_view,
                wgpu::FilterMode::Linear,
                id,
            ),
            Some(_) => {}
        }
    }
    if let Some(tex) = v3.texture {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(tex, rect, uv, egui::Color32::WHITE);
    }
    // The sun path over the image, under the selection box, the cube and the tools.
    v3.sun.paint(&painter, &crate::modify3d::Frame::new(rect, v3.camera));
    if let Some(start) = v3.select_start {
        let cur = resp.interact_pointer_pos().or(resp.hover_pos()).unwrap_or(start);
        crate::tools::select_box(&painter, start, cur);
        if resp.drag_stopped() {
            v3.select_start = None;
            let ndc = |p: egui::Pos2| {
                let x = (p.x - rect.left()) / rect.width().max(1.0) * 2.0 - 1.0;
                let y = 1.0 - (p.y - rect.top()) / rect.height().max(1.0) * 2.0;
                glam::DVec2::new(x as f64, y as f64)
            };
            let (a, b, window) = (ndc(start), ndc(cur), cur.x >= start.x);
            let (project, temporary) = (&state.session.doc.project, state.session.temporary(view_id));
            let geom = state.session.geom_for(view_id);
            let ids = match project.view(view_id) {
                Some(view) => {
                    let mut resolver = cc_views::graphics::Resolver::new(project, geom, view, temporary);
                    let mut shown = |e| resolver.style(e).is_some();
                    cc_render::elements_in_rect(&geom.solids, &v3.camera, aspect, a, b, window, &mut shown)
                }
                None => cc_render::elements_in_rect(&geom.solids, &v3.camera, aspect, a, b, window, &mut |_| true),
            };
            if !ui.input(|i| i.modifiers.command) {
                state.selection.clear();
            }
            state.selection.extend(ids);
        }
    }
    let north = state.session.doc.project.info.site.as_ref().map_or(glam::DVec2::Y, |s| s.north());
    let cube = crate::view_cube::show(ui, rect, &v3.camera, north);
    if cube.look_from.is_some() || cube.home {
        v3.remember();
    }
    if let Some(dir) = cube.look_from {
        let (yaw, pitch) = crate::view_cube::angles_from(dir, v3.camera.yaw_deg);
        v3.turn_to(yaw, pitch, now);
        ui.ctx().request_repaint();
    }
    if cube.home {
        let home = CameraState::default();
        fit_camera(&mut v3.camera, state.session.geom_for(view_id), aspect);
        v3.turn_to(home.yaw_deg, home.pitch_deg, now);
        ui.ctx().request_repaint();
    }
    if let Some(d) = cube.orbit {
        v3.anim = None;
        v3.camera.yaw_deg += d.x as f64 * 0.4;
        v3.camera.pitch_deg = (v3.camera.pitch_deg + d.y as f64 * 0.4).clamp(-89.0, 89.0);
        v3.camera_dirty = true;
    }
    if cube.turn != 0.0 {
        v3.anim = None;
        v3.camera.yaw_deg += cube.turn;
        v3.camera_dirty = true;
    }

    let nav = v3.nav();
    let camera_now = v3.camera;
    if modifying && !zooming {
        crate::modify3d::handle(ui, &resp, &painter, state, view_id, crate::modify3d::Frame::new(rect, camera_now));
    }
    if let Some((wall, point)) = opening_pick {
        let tool = state.tool.clone();
        crate::tools::place_opening_3d(state, &tool, wall, point);
    }
    if let Some(picked) = attach_pick {
        crate::modify::attach_pick(state, picked);
    }
    if let Some((remove, picked, ray)) = paint_pick {
        crate::paint_ui::pick(state, view_id, remove, picked, ray);
    }
    if let Some(picked) = demolish_pick {
        let ids = crate::phases_dialog::demolishable(state, view_id, picked);
        crate::phases_dialog::demolish(state, view_id, &ids);
    }
    crate::canvas::temporary_frame(ui, state, rect, view_id);
    if let Some(cmd) = crate::canvas::view_toolbar(ui, state.zoom_default, rect, view_id, None, nav) {
        crate::canvas::zoom(state, view_id, cmd, 1.0);
        ui.ctx().request_repaint();
    }
    let Some(v3) = state.view3d.get_mut(&view_id) else { return };

    // Persist the camera into the view when the interaction ends (not undoable).
    let interacting = resp.dragged()
        || v3.anim.is_some()
        || ui.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO || i.zoom_delta() != 1.0)
        || cube.orbit.is_some()
        || cube.turn != 0.0;
    if v3.camera_dirty && !interacting {
        v3.camera_dirty = false;
        let camera = v3.camera;
        state.session.set_view_camera(view_id, camera);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts what the scene sends to the renderer.
    #[derive(Default)]
    struct Recorder {
        uploads: usize,
        /// Vertices and edge vertices of each highlight set (`None`: cleared).
        highlights: Vec<Option<(usize, usize)>>,
    }

    impl SceneSink for Recorder {
        fn set_textures(&mut self, _: &cc_render::TextureSet) {}
        fn upload(&mut self, _: &SceneMesh) {
            self.uploads += 1;
        }
        fn set_highlight(&mut self, h: Option<&cc_render::Highlight>) {
            self.highlights.push(h.map(|h| (h.mesh.vertices.len(), h.mesh.edges.len())));
        }
    }

    #[test]
    fn a_new_selection_does_not_rebuild_the_scene() {
        let project = cc_core::from_ron(include_str!("../../../examples/casa_simples.ccad")).unwrap();
        let mut session = cc_engine::Session::new(cc_core::Document::new(project));
        let view = session.doc.project.views_of(cc_core::ViewCategory::ThreeD).next().unwrap().id;
        let walls: Vec<cc_core::ElementId> =
            session.doc.project.elements.values().filter(|e| e.as_wall().is_some()).map(|e| e.id).collect();
        let (mut scene, mut gpu) = (Scene::default(), Recorder::default());
        let mut update = |scene: &mut Scene, session: &mut cc_engine::Session, sel: &[cc_core::ElementId]| {
            scene.update(session, view, Mode::Model, &sel.iter().copied().collect(), &mut gpu);
        };
        update(&mut scene, &mut session, &[]);
        assert_eq!(scene.builds, 1);
        // Selecting, adding to the selection and clearing it only change the highlight.
        update(&mut scene, &mut session, &walls[..1]);
        update(&mut scene, &mut session, &walls[..1]);
        update(&mut scene, &mut session, &walls[..2]);
        update(&mut scene, &mut session, &[]);
        assert_eq!(scene.builds, 1);
        // An edit rebuilds it, and the highlight over it.
        session.execute("elements_delete", serde_json::json!({"ids": [walls[3].0]})).unwrap();
        update(&mut scene, &mut session, &walls[..1]);
        assert_eq!(scene.builds, 2);
        drop(update);
        assert_eq!(gpu.uploads, 2);
        let h = &gpu.highlights;
        assert_eq!(h.len(), 5, "{h:?}");
        // None at first, one wall, two walls, cleared, the wall again over the new base.
        let one = h[1].unwrap();
        assert!(h[0].is_none() && one.0 > 0 && one.1 > 0);
        assert!(h[2].is_some_and(|(v, _)| v > one.0));
        assert!(h[3].is_none() && h[4].is_some());
    }

    #[test]
    fn zoom_commands_move_the_camera_and_walk_back() {
        let mut v = View3DState::new(CameraState::default());
        let start = v.camera;
        v.apply(ZoomCommand::OutTwo);
        assert!((v.camera.distance - start.distance * 2.0).abs() < 1e-9);
        // A window around the view center halves the distance, keeping the target.
        let view = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        v.remember();
        v.zoom_to_window(egui::Rect::from_center_size(view.center(), egui::vec2(400.0, 300.0)), view);
        assert!((v.camera.distance - start.distance).abs() < 1e-9);
        assert!(v.camera.target.distance(start.target) < 1e-9);
        v.apply(ZoomCommand::Previous);
        v.apply(ZoomCommand::Previous);
        assert_eq!(v.camera, start);
        assert!(!v.nav().back && v.nav().forward);
        v.apply(ZoomCommand::Next);
        assert!((v.camera.distance - start.distance * 2.0).abs() < 1e-9);
        // No paper scale in 3D.
        v.apply(ZoomCommand::SheetSize);
        assert!((v.camera.distance - start.distance * 2.0).abs() < 1e-9);
        assert!(!v.nav().sheet_size);
    }
}
