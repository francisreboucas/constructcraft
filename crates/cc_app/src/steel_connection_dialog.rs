//! IDEA StatiCa / RFEM-style Steel Joint Editor dialog:
//! component list + parametric settings on the left, interactive orbitable 3D
//! shaded preview (steel blue members, gold plates, orange-red hex bolts) on
//! the right.

use crate::dialog::{Dialog, two_columns};
use crate::state::AppState;
use crate::t;
use crate::table;
use crate::theme::{self, PALETTE};
use cc_core::steel_connection::{ConnectionKind, ConnectionParams, recognize_for_members};
use cc_core::{Category, ElementId};
use cc_engine::commands::steel_connection::SteelConnectionCreate;
use egui::{Color32, Context, Pos2, RichText, Sense, Shape, Stroke, Ui, vec2};
use glam::Vec3;

const VIEW_W: f32 = 540.0;
const VIEW_H: f32 = 410.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComponentFocus {
    All,
    Plate,
    Bolts,
    Stiffeners,
}

/// State of the open Steel Connection configuration dialog.
pub struct SteelConnectionDialog {
    pub members: Vec<ElementId>,
    pub kind: ConnectionKind,
    pub plate_thickness_mm: f64,
    pub bolt_diameter_mm: f64,
    pub bolt_grade: String,
    pub rows: u32,
    pub cols: u32,
    pub pitch_mm: f64,
    pub edge_mm: f64,
    pub extension_mm: f64,
    pub stiffeners: bool,
    pub base_mode: String,
    pub anchors: u32,
    focus: ComponentFocus,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pub error: Option<String>,
}

fn kind_label(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::BasePlate => "Chapa de base (Base plate)",
        ConnectionKind::EndPlate => "Chapa de topo / Rígida (End plate)",
        ConnectionKind::SinglePlate => "Chapa simples / Cisalhamento (Fin plate)",
        ConnectionKind::DoubleAngle => "Dupla cantoneira (Double angle)",
        ConnectionKind::GussetPlate => "Chapa de nó / Contraventamento (Gusset)",
        ConnectionKind::Splice => "Emenda parafusada (Splice)",
    }
}

fn defaults_for(state: &AppState, members: &[ElementId], forced: Option<ConnectionKind>) -> SteelConnectionDialog {
    let project = state.project();
    let recognized = recognize_for_members(project, members, forced, &ConnectionParams::default());
    if let Some(rc) = recognized {
        SteelConnectionDialog {
            members: members.to_vec(),
            kind: rc.kind,
            plate_thickness_mm: (rc.spec.plate_thickness * 1000.0).max(4.0),
            bolt_diameter_mm: (rc.spec.bolt_diameter * 1000.0).max(10.0),
            bolt_grade: rc.spec.bolt_grade,
            rows: rc.spec.rows.max(1),
            cols: rc.spec.cols.max(1),
            pitch_mm: 65.0,
            edge_mm: 35.0,
            extension_mm: if rc.kind == ConnectionKind::EndPlate { 40.0 } else { 0.0 },
            stiffeners: rc.spec.stiffeners,
            base_mode: "pinned".to_string(),
            anchors: 4,
            focus: ComponentFocus::All,
            yaw: 0.65,
            pitch: 0.42,
            zoom: 1.0,
            error: None,
        }
    } else {
        let single_column =
            members.len() == 1 && project.element(members[0]).is_some_and(|e| e.category() == Category::Column);
        let kind =
            forced.unwrap_or(if single_column { ConnectionKind::BasePlate } else { ConnectionKind::SinglePlate });
        SteelConnectionDialog {
            members: members.to_vec(),
            kind,
            plate_thickness_mm: 12.0,
            bolt_diameter_mm: 16.0,
            bolt_grade: "8.8".to_string(),
            rows: 3,
            cols: 2,
            pitch_mm: 65.0,
            edge_mm: 35.0,
            extension_mm: if kind == ConnectionKind::EndPlate { 40.0 } else { 0.0 },
            stiffeners: false,
            base_mode: "pinned".to_string(),
            anchors: 4,
            focus: ComponentFocus::All,
            yaw: 0.65,
            pitch: 0.42,
            zoom: 1.0,
            error: None,
        }
    }
}

/// Open the steel connection configuration dialog for `members`.
pub fn open(state: &mut AppState, members: Vec<ElementId>) {
    if members.is_empty() {
        return;
    }
    state.dialogs.steel_connection = Some(defaults_for(state, &members, None));
}

fn member_summary(state: &AppState, members: &[ElementId]) -> String {
    let p = state.project();
    let names: Vec<String> = members
        .iter()
        .filter_map(|id| {
            let e = p.element(*id)?;
            let type_name = p.element_type(e.type_id).map(|t| t.name.as_str()).unwrap_or("");
            let cat = match e.category() {
                Category::Column => "Pilar",
                Category::Beam => "Viga",
                _ => "Barra",
            };
            if type_name.is_empty() {
                Some(format!("{cat} #{}", id.0))
            } else {
                Some(format!("{cat} {type_name} (#{})", id.0))
            }
        })
        .collect();
    names.join("  +  ")
}

// ------------------------------------------------------------------ 3D shaded preview

struct Face3D {
    pts: Vec<Vec3>,
    color: Color32,
    edge: Color32,
    depth: f32,
    layer: u8,
}

fn rotate_pt(p: Vec3, yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let x1 = p.x * cy - p.y * sy;
    let y1 = p.x * sy + p.y * cy;
    let z1 = p.z;
    let y2 = y1 * cp - z1 * sp;
    let z2 = y1 * sp + z1 * cp;
    Vec3::new(x1, y2, z2)
}

fn shade(base: Color32, normal: Vec3) -> Color32 {
    let light = Vec3::new(0.45, -0.55, 0.70).normalize();
    let k = (normal.normalize_or_zero().dot(light) * 0.35 + 0.72).clamp(0.42, 1.08);
    Color32::from_rgba_unmultiplied(
        ((base.r() as f32) * k).clamp(0.0, 255.0) as u8,
        ((base.g() as f32) * k).clamp(0.0, 255.0) as u8,
        ((base.b() as f32) * k).clamp(0.0, 255.0) as u8,
        base.a(),
    )
}

fn push_box(faces: &mut Vec<Face3D>, lo: Vec3, hi: Vec3, base: Color32, edge: Color32, yaw: f32, pitch: f32) {
    let corners = [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
    ];
    let quads: [([usize; 4], Vec3); 6] = [
        ([0, 3, 2, 1], -Vec3::Z),
        ([4, 5, 6, 7], Vec3::Z),
        ([0, 1, 5, 4], -Vec3::Y),
        ([2, 3, 7, 6], Vec3::Y),
        ([0, 4, 7, 3], -Vec3::X),
        ([1, 2, 6, 5], Vec3::X),
    ];
    for (idx, n) in quads {
        let pts: Vec<Vec3> = idx.iter().map(|&i| rotate_pt(corners[i], yaw, pitch)).collect();
        let rn = rotate_pt(n, yaw, pitch);
        if rn.y < 0.05 {
            let depth = pts.iter().map(|p| p.y).sum::<f32>() / 4.0;
            faces.push(Face3D { pts, color: shade(base, rn), edge, depth, layer: u8::from(base.a() == 255) });
        }
    }
}

fn push_hex_prism(
    faces: &mut Vec<Face3D>,
    center: Vec3,
    axis: Vec3,
    radius: f32,
    length: f32,
    base: Color32,
    edge: Color32,
    yaw: f32,
    pitch: f32,
) {
    let ax = axis.normalize_or_zero();
    let ref_v = if ax.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let u = ax.cross(ref_v).normalize();
    let v = ax.cross(u).normalize();
    let p0 = center - ax * (length * 0.5);
    let p1 = center + ax * (length * 0.5);
    let mut ring0 = Vec::with_capacity(6);
    let mut ring1 = Vec::with_capacity(6);
    for i in 0..6 {
        let a = std::f32::consts::TAU * (i as f32) / 6.0;
        let off = u * (radius * a.cos()) + v * (radius * a.sin());
        ring0.push(p0 + off);
        ring1.push(p1 + off);
    }
    // End caps
    for (ring, n) in [(&ring0, -ax), (&ring1, ax)] {
        let pts: Vec<Vec3> = ring.iter().map(|&p| rotate_pt(p, yaw, pitch)).collect();
        let rn = rotate_pt(n, yaw, pitch);
        if rn.y < 0.25 {
            let depth = pts.iter().map(|p| p.y).sum::<f32>() / 6.0;
            faces.push(Face3D { pts, color: shade(base, rn), edge, depth, layer: u8::from(base.a() == 255) });
        }
    }
    // Side faces
    for i in 0..6 {
        let j = (i + 1) % 6;
        let pts = vec![
            rotate_pt(ring0[i], yaw, pitch),
            rotate_pt(ring0[j], yaw, pitch),
            rotate_pt(ring1[j], yaw, pitch),
            rotate_pt(ring1[i], yaw, pitch),
        ];
        let n = ((ring0[i] + ring0[j]) * 0.5 - p0).normalize_or_zero();
        let rn = rotate_pt(n, yaw, pitch);
        if rn.y < 0.25 {
            let depth = pts.iter().map(|p| p.y).sum::<f32>() / 4.0;
            faces.push(Face3D { pts, color: shade(base, rn), edge, depth, layer: u8::from(base.a() == 255) });
        }
    }
}

fn draw_dim_3d(
    painter: &egui::Painter,
    project: &dyn Fn(Vec3) -> Pos2,
    yaw: f32,
    pitch: f32,
    p0: Vec3,
    p1: Vec3,
    ext: Vec3,
    label: &str,
    color: Color32,
) {
    let q0 = p0 + ext;
    let q1 = p1 + ext;
    let s_p0 = project(rotate_pt(p0, yaw, pitch));
    let s_p1 = project(rotate_pt(p1, yaw, pitch));
    let s_q0 = project(rotate_pt(q0, yaw, pitch));
    let s_q1 = project(rotate_pt(q1, yaw, pitch));
    let thin = Stroke::new(1.0, color.gamma_multiply(0.75));
    let line = Stroke::new(1.3, color);
    painter.line_segment([s_p0, s_q0], thin);
    painter.line_segment([s_p1, s_q1], thin);
    painter.line_segment([s_q0, s_q1], line);
    for sq in [s_q0, s_q1] {
        painter.circle_filled(sq, 2.2, color);
    }
    let mid = Pos2::new((s_q0.x + s_q1.x) * 0.5, (s_q0.y + s_q1.y) * 0.5);
    let font = egui::FontId::proportional(10.0);
    let galley = painter.layout_no_wrap(label.to_string(), font, color);
    let bg = egui::Rect::from_center_size(mid, galley.size() + vec2(6.0, 2.0));
    painter.rect_filled(bg, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 215));
    painter.galley(bg.min + vec2(3.0, 1.0), galley, color);
}

fn push_i_member_x(
    faces: &mut Vec<Face3D>,
    x0: f32,
    x1: f32,
    h: f32,
    b: f32,
    tf: f32,
    tw: f32,
    base: Color32,
    edge: Color32,
    yaw: f32,
    pitch: f32,
) {
    push_box(faces, Vec3::new(x0, -b * 0.5, h * 0.5 - tf), Vec3::new(x1, b * 0.5, h * 0.5), base, edge, yaw, pitch);
    push_box(faces, Vec3::new(x0, -b * 0.5, -h * 0.5), Vec3::new(x1, b * 0.5, -h * 0.5 + tf), base, edge, yaw, pitch);
    push_box(
        faces,
        Vec3::new(x0, -tw * 0.5, -h * 0.5 + tf),
        Vec3::new(x1, tw * 0.5, h * 0.5 - tf),
        base,
        edge,
        yaw,
        pitch,
    );
}

fn push_i_member_z(
    faces: &mut Vec<Face3D>,
    cx: f32,
    z0: f32,
    z1: f32,
    h: f32,
    b: f32,
    tf: f32,
    tw: f32,
    base: Color32,
    edge: Color32,
    yaw: f32,
    pitch: f32,
) {
    push_box(
        faces,
        Vec3::new(cx + h * 0.5 - tf, -b * 0.5, z0),
        Vec3::new(cx + h * 0.5, b * 0.5, z1),
        base,
        edge,
        yaw,
        pitch,
    );
    push_box(
        faces,
        Vec3::new(cx - h * 0.5, -b * 0.5, z0),
        Vec3::new(cx - h * 0.5 + tf, b * 0.5, z1),
        base,
        edge,
        yaw,
        pitch,
    );
    push_box(
        faces,
        Vec3::new(cx - h * 0.5 + tf, -tw * 0.5, z0),
        Vec3::new(cx + h * 0.5 - tf, tw * 0.5, z1),
        base,
        edge,
        yaw,
        pitch,
    );
}

fn draw_3d_preview(ui: &mut Ui, d: &mut SteelConnectionDialog) {
    let palette = theme::palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(VIEW_W, VIEW_H), Sense::click_and_drag());
    if resp.dragged() {
        let delta = resp.drag_delta();
        d.yaw += delta.x * 0.012;
        d.pitch = (d.pitch + delta.y * 0.012).clamp(-1.45, 1.45);
    }
    let scroll = ui.input(|i| if resp.hovered() { i.smooth_scroll_delta.y } else { 0.0 });
    if scroll.abs() > 0.1 {
        d.zoom = (d.zoom * (1.0 + scroll * 0.002)).clamp(0.45, 2.5);
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, palette.canvas);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);

    let member_col = Color32::from_rgba_unmultiplied(138, 174, 232, 165);
    let member_edge = Color32::from_rgba_unmultiplied(72, 98, 145, 200);
    let plate_col = if d.focus == ComponentFocus::Plate {
        Color32::from_rgb(255, 210, 82)
    } else {
        Color32::from_rgb(232, 188, 78)
    };
    let plate_edge = Color32::from_rgb(135, 98, 24);
    let bolt_col =
        if d.focus == ComponentFocus::Bolts { Color32::from_rgb(255, 90, 35) } else { Color32::from_rgb(228, 92, 42) };
    let bolt_edge = Color32::from_rgb(115, 36, 12);
    let stiff_col = if d.focus == ComponentFocus::Stiffeners {
        Color32::from_rgb(255, 224, 96)
    } else {
        Color32::from_rgb(214, 172, 68)
    };

    let mut faces: Vec<Face3D> = Vec::new();
    let tp = (d.plate_thickness_mm as f32 / 1000.0).clamp(0.005, 0.060);
    let bd = (d.bolt_diameter_mm as f32 / 1000.0).clamp(0.010, 0.036);
    let br = bd * 0.92;
    let bl = bd * 1.4;
    let yaw = d.yaw;
    let pitch = d.pitch;

    match d.kind {
        ConnectionKind::BasePlate => {
            let (pw, ph) = (0.36, 0.42);
            push_box(
                &mut faces,
                Vec3::new(-ph * 0.5, -pw * 0.5, 0.0),
                Vec3::new(ph * 0.5, pw * 0.5, tp),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            push_i_member_z(&mut faces, 0.0, tp, 0.48, 0.25, 0.15, 0.012, 0.008, member_col, member_edge, yaw, pitch);
            if d.stiffeners || d.base_mode == "fixed" {
                push_box(
                    &mut faces,
                    Vec3::new(0.125, -0.006, tp),
                    Vec3::new(ph * 0.48, 0.006, tp + 0.12),
                    stiff_col,
                    plate_edge,
                    yaw,
                    pitch,
                );
                push_box(
                    &mut faces,
                    Vec3::new(-ph * 0.48, -0.006, tp),
                    Vec3::new(-0.125, 0.006, tp + 0.12),
                    stiff_col,
                    plate_edge,
                    yaw,
                    pitch,
                );
            }
            let n = d.anchors.max(2);
            let mut pts = vec![Vec3::new(-0.15, -0.11, tp + bd * 0.5), Vec3::new(0.15, -0.11, tp + bd * 0.5)];
            if n >= 4 {
                pts.push(Vec3::new(-0.15, 0.11, tp + bd * 0.5));
                pts.push(Vec3::new(0.15, 0.11, tp + bd * 0.5));
            }
            if n >= 6 {
                pts.push(Vec3::new(-0.15, 0.0, tp + bd * 0.5));
                pts.push(Vec3::new(0.15, 0.0, tp + bd * 0.5));
            }
            if n >= 8 {
                pts.push(Vec3::new(0.0, -0.11, tp + bd * 0.5));
                pts.push(Vec3::new(0.0, 0.11, tp + bd * 0.5));
            }
            for p in pts.into_iter().take(n as usize) {
                push_hex_prism(&mut faces, p, Vec3::Z, br, bl, bolt_col, bolt_edge, yaw, pitch);
            }
        }
        ConnectionKind::EndPlate => {
            let ext = (d.extension_mm as f32 / 1000.0).clamp(0.0, 0.14);
            // Column on the left
            push_i_member_z(
                &mut faces,
                -0.12,
                -0.34,
                0.34,
                0.24,
                0.15,
                0.012,
                0.008,
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            // Beam on the right
            push_i_member_x(&mut faces, tp, 0.45, 0.25, 0.14, 0.011, 0.007, member_col, member_edge, yaw, pitch);
            // End plate
            push_box(
                &mut faces,
                Vec3::new(0.0, -0.09, -0.14),
                Vec3::new(tp, 0.09, 0.125 + ext),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            if d.stiffeners {
                push_box(
                    &mut faces,
                    Vec3::new(-0.22, -0.07, 0.114),
                    Vec3::new(-0.012, 0.07, 0.125),
                    stiff_col,
                    plate_edge,
                    yaw,
                    pitch,
                );
                push_box(
                    &mut faces,
                    Vec3::new(-0.22, -0.07, -0.125),
                    Vec3::new(-0.012, 0.07, -0.114),
                    stiff_col,
                    plate_edge,
                    yaw,
                    pitch,
                );
            }
            let ys: Vec<f32> = if d.cols >= 4 { vec![-0.068, -0.036, 0.036, 0.068] } else { vec![-0.045, 0.045] };
            let rows = d.rows.clamp(1, 6);
            let mut zs = Vec::new();
            if ext >= 0.035 {
                zs.push(0.125 + ext * 0.5);
            }
            for r in 0..rows {
                let t = if rows == 1 { 0.5 } else { r as f32 / (rows - 1) as f32 };
                zs.push(0.085 - 0.17 * t);
            }
            for z in zs {
                for &y in &ys {
                    push_hex_prism(
                        &mut faces,
                        Vec3::new(tp + bd * 0.35, y, z),
                        Vec3::X,
                        br,
                        bl + tp,
                        bolt_col,
                        bolt_edge,
                        yaw,
                        pitch,
                    );
                }
            }
        }
        ConnectionKind::Splice => {
            push_i_member_x(&mut faces, -0.46, -0.006, 0.25, 0.14, 0.011, 0.007, member_col, member_edge, yaw, pitch);
            push_i_member_x(&mut faces, 0.006, 0.46, 0.25, 0.14, 0.011, 0.007, member_col, member_edge, yaw, pitch);
            let l = 0.28;
            push_box(
                &mut faces,
                Vec3::new(-l * 0.5, -0.07, 0.125),
                Vec3::new(l * 0.5, 0.07, 0.125 + tp),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(-l * 0.5, -0.07, -0.125 - tp),
                Vec3::new(l * 0.5, 0.07, -0.125),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(-l * 0.5, 0.004, -0.095),
                Vec3::new(l * 0.5, 0.004 + tp, 0.095),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(-l * 0.5, -0.004 - tp, -0.095),
                Vec3::new(l * 0.5, -0.004, 0.095),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            for x in [-0.10, -0.04, 0.04, 0.10] {
                for y in [-0.04, 0.04] {
                    push_hex_prism(
                        &mut faces,
                        Vec3::new(x, y, 0.125 + tp),
                        Vec3::Z,
                        br,
                        bl,
                        bolt_col,
                        bolt_edge,
                        yaw,
                        pitch,
                    );
                }
                for z in [-0.045, 0.045] {
                    push_hex_prism(
                        &mut faces,
                        Vec3::new(x, 0.0, z),
                        Vec3::Y,
                        br,
                        2.0 * tp + bl,
                        bolt_col,
                        bolt_edge,
                        yaw,
                        pitch,
                    );
                }
            }
        }
        ConnectionKind::SinglePlate | ConnectionKind::DoubleAngle | ConnectionKind::GussetPlate => {
            // Primary/support beam along Y, secondary beam along X
            push_box(
                &mut faces,
                Vec3::new(-0.14, -0.34, 0.115),
                Vec3::new(0.0, 0.34, 0.127),
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(-0.14, -0.34, -0.127),
                Vec3::new(0.0, 0.34, -0.115),
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(-0.075, -0.34, -0.115),
                Vec3::new(-0.065, 0.34, 0.115),
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            // Secondary beam along X: top flange FLUSH at z = +0.130, coped clear of primary flange (x >= 0.010), depth = 0.220 (z in [-0.090, 0.130])
            let z_top = 0.130_f32;
            let z_bot = -0.090_f32;
            let tf_s = 0.010_f32;
            let tw_s = 0.007_f32;
            let b_s = 0.130_f32;
            let gap = 0.010_f32;
            push_box(
                &mut faces,
                Vec3::new(gap, -b_s * 0.5, z_top - tf_s),
                Vec3::new(0.44, b_s * 0.5, z_top),
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(gap, -b_s * 0.5, z_bot),
                Vec3::new(0.44, b_s * 0.5, z_bot + tf_s),
                member_col,
                member_edge,
                yaw,
                pitch,
            );
            push_box(
                &mut faces,
                Vec3::new(gap, -tw_s * 0.5, z_bot + tf_s),
                Vec3::new(0.44, tw_s * 0.5, z_top - tf_s),
                member_col,
                member_edge,
                yaw,
                pitch,
            );

            let cols = d.cols.clamp(1, 3);
            let rows = d.rows.clamp(1, 6);
            let pitch_m = (d.pitch_mm as f32 / 1000.0).clamp(0.030, 0.120);
            let edge_m = (d.edge_mm as f32 / 1000.0).clamp(0.020, 0.080);
            let pw = 0.010 + 2.0 * edge_m + (cols as f32 - 1.0) * pitch_m;
            let ph = (2.0 * edge_m + (rows as f32 - 1.0) * pitch_m).min(0.185);
            let z_mid = (z_top + z_bot) * 0.5;
            push_box(
                &mut faces,
                Vec3::new(-0.066, 0.004, z_mid - ph * 0.5),
                Vec3::new(pw, 0.004 + tp, z_mid + ph * 0.5),
                plate_col,
                plate_edge,
                yaw,
                pitch,
            );
            if d.kind == ConnectionKind::DoubleAngle {
                push_box(
                    &mut faces,
                    Vec3::new(-0.066, -0.004 - tp, z_mid - ph * 0.5),
                    Vec3::new(pw, -0.004, z_mid + ph * 0.5),
                    plate_col,
                    plate_edge,
                    yaw,
                    pitch,
                );
            }
            for r in 0..rows {
                let z = if rows == 1 { z_mid } else { z_mid + ph * 0.5 - edge_m - (r as f32) * pitch_m };
                for c in 0..cols {
                    let x = gap + edge_m + (c as f32) * pitch_m;
                    push_hex_prism(
                        &mut faces,
                        Vec3::new(x, 0.0, z),
                        Vec3::Y,
                        br,
                        tp + bl,
                        bolt_col,
                        bolt_edge,
                        yaw,
                        pitch,
                    );
                }
            }
        }
    }

    faces.sort_by(|a, b| a.layer.cmp(&b.layer).then_with(|| b.depth.total_cmp(&a.depth)));
    let scale = 420.0 * d.zoom;
    let center = rect.center();
    let project = |p: Vec3| Pos2::new(center.x + p.x * scale, center.y - p.z * scale);
    for f in faces {
        let screen: Vec<Pos2> = f.pts.into_iter().map(project).collect();
        if screen.len() >= 3 {
            painter.add(Shape::convex_polygon(screen, f.color, Stroke::new(0.9, f.edge)));
        }
    }

    // Dynamic 3D dimensions projected in model space
    let dim_col = Color32::from_rgb(185, 38, 38);
    let pitch_m = (d.pitch_mm as f32 / 1000.0).clamp(0.030, 0.120);
    let edge_m = (d.edge_mm as f32 / 1000.0).clamp(0.020, 0.080);
    match d.kind {
        ConnectionKind::BasePlate => {
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(-0.21, -0.18, 0.0),
                Vec3::new(0.21, -0.18, 0.0),
                Vec3::new(0.0, -0.06, 0.0),
                "420 mm",
                dim_col,
            );
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(0.21, -0.18, 0.0),
                Vec3::new(0.21, 0.18, 0.0),
                Vec3::new(0.06, 0.0, 0.0),
                "360 mm",
                dim_col,
            );
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(0.21, 0.18, 0.0),
                Vec3::new(0.21, 0.18, tp),
                Vec3::new(0.05, 0.05, 0.0),
                &format!("t={:.0}mm", d.plate_thickness_mm),
                dim_col,
            );
        }
        ConnectionKind::EndPlate => {
            let ext = (d.extension_mm as f32 / 1000.0).clamp(0.0, 0.14);
            let h_mm = (265.0 + d.extension_mm).round();
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(tp, 0.09, -0.14),
                Vec3::new(tp, 0.09, 0.125 + ext),
                Vec3::new(0.0, 0.07, 0.0),
                &format!("H={h_mm:.0}mm"),
                dim_col,
            );
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(0.0, -0.09, -0.14),
                Vec3::new(tp, -0.09, -0.14),
                Vec3::new(0.0, -0.06, -0.03),
                &format!("t={:.0}mm", d.plate_thickness_mm),
                dim_col,
            );
            if ext >= 10.0 / 1000.0 {
                draw_dim_3d(
                    &painter,
                    &project,
                    yaw,
                    pitch,
                    Vec3::new(tp, -0.09, 0.125),
                    Vec3::new(tp, -0.09, 0.125 + ext),
                    Vec3::new(0.0, -0.06, 0.0),
                    &format!("ext={:.0}mm", d.extension_mm),
                    dim_col,
                );
            }
        }
        ConnectionKind::Splice => {
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(-0.14, -0.07, 0.125 + tp),
                Vec3::new(0.14, -0.07, 0.125 + tp),
                Vec3::new(0.0, -0.05, 0.05),
                "L=280 mm",
                dim_col,
            );
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(0.14, 0.07, 0.125),
                Vec3::new(0.14, 0.07, 0.125 + tp),
                Vec3::new(0.05, 0.04, 0.0),
                &format!("t={:.0}mm", d.plate_thickness_mm),
                dim_col,
            );
        }
        ConnectionKind::SinglePlate | ConnectionKind::DoubleAngle | ConnectionKind::GussetPlate => {
            let cols = d.cols.clamp(1, 3);
            let rows = d.rows.clamp(1, 6);
            let pw = 0.010 + 2.0 * edge_m + (cols as f32 - 1.0) * pitch_m;
            let ph = (2.0 * edge_m + (rows as f32 - 1.0) * pitch_m).min(0.185);
            let z_mid = 0.020_f32;
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(pw, 0.004 + tp, z_mid - ph * 0.5),
                Vec3::new(pw, 0.004 + tp, z_mid + ph * 0.5),
                Vec3::new(0.06, 0.04, 0.0),
                &format!("H={:.0}mm", ph * 1000.0),
                dim_col,
            );
            draw_dim_3d(
                &painter,
                &project,
                yaw,
                pitch,
                Vec3::new(-0.066, 0.004 + tp, z_mid - ph * 0.5),
                Vec3::new(pw, 0.004 + tp, z_mid - ph * 0.5),
                Vec3::new(0.0, 0.04, -0.05),
                &format!("L={:.0}mm", (pw + 0.066) * 1000.0),
                dim_col,
            );
            if rows >= 2 {
                let z0 = z_mid + ph * 0.5 - edge_m;
                let z1 = z0 - pitch_m;
                let x_b = 0.010 + edge_m;
                draw_dim_3d(
                    &painter,
                    &project,
                    yaw,
                    pitch,
                    Vec3::new(x_b, 0.004 + tp, z0),
                    Vec3::new(x_b, 0.004 + tp, z1),
                    Vec3::new(0.05, 0.05, 0.0),
                    &format!("p={:.0}mm", d.pitch_mm),
                    dim_col,
                );
            }
        }
    }

    // XYZ triad in bottom-left corner
    let origin = Pos2::new(rect.left() + 34.0, rect.bottom() - 34.0);
    for (dir, col, lbl) in [
        (Vec3::X, Color32::from_rgb(220, 70, 70), "X"),
        (Vec3::Y, Color32::from_rgb(70, 185, 95), "Y"),
        (Vec3::Z, Color32::from_rgb(75, 135, 235), "Z"),
    ] {
        let r = rotate_pt(dir, yaw, pitch);
        let tip = Pos2::new(origin.x + r.x * 20.0, origin.y - r.z * 20.0);
        painter.line_segment([origin, tip], Stroke::new(1.6, col));
        painter.text(tip, egui::Align2::CENTER_CENTER, lbl, egui::FontId::proportional(9.5), col);
    }

    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        "Prévia 3D Paramétrica (arraste para orbitar • scroll para zoom)",
        egui::FontId::proportional(11.0),
        palette.text_muted,
    );

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Isométrica").clicked() {
            d.yaw = 0.65;
            d.pitch = 0.42;
            d.zoom = 1.0;
        }
        if ui.button("Frontal").clicked() {
            d.yaw = 0.0;
            d.pitch = 0.0;
        }
        if ui.button("Lateral").clicked() {
            d.yaw = 1.57;
            d.pitch = 0.0;
        }
        if ui.button("Topo").clicked() {
            d.yaw = 0.0;
            d.pitch = 1.45;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let summary = if d.kind == ConnectionKind::BasePlate {
                format!("t={:.0}mm | {} chumb. M{:.0}", d.plate_thickness_mm, d.anchors, d.bolt_diameter_mm)
            } else {
                format!(
                    "t={:.0}mm | {}×{} M{:.0} ({})",
                    d.plate_thickness_mm, d.rows, d.cols, d.bolt_diameter_mm, d.bolt_grade
                )
            };
            ui.label(theme::muted(summary));
        });
    });
}

pub fn show(ctx: &Context, state: &mut AppState) {
    let Some(mut d) = state.dialogs.steel_connection.take() else { return };
    let members_text = member_summary(state, &d.members);
    let mut create_btn = 0;
    let response = Dialog::new("steel-connection", t!("action-steel-connection")).width(960.0).show(
        ctx,
        |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::muted("Barras:"));
                ui.label(RichText::new(&members_text).strong());
            });
            ui.add_space(6.0);

            two_columns(ui, [390.0, VIEW_W + 10.0], |ui, col| {
                if col == 0 {
                    // Component tree / list like IDEA StatiCa / RFEM
                    table::section(ui, "sc-components", "Componentes da ligação".into(), |ui| {
                        let items = [
                            (ComponentFocus::All, "Todas as operações (Visão geral)"),
                            (ComponentFocus::Plate, "Chapa / Cantoneira de ligação"),
                            (ComponentFocus::Bolts, "Parafusos hexagonais e arruelas"),
                            (ComponentFocus::Stiffeners, "Enrijecedores / Soldas"),
                        ];
                        for (focus, label) in items {
                            if ui.selectable_label(d.focus == focus, label).clicked() {
                                d.focus = focus;
                            }
                        }
                    });
                    ui.add_space(6.0);

                    table::section(ui, "sc-settings", "Configurações paramétricas".into(), |ui| {
                        let mut chosen_kind = d.kind;
                        table::row(ui, "Tipo", |ui| {
                            egui::ComboBox::from_id_salt("steel-conn-kind")
                                .selected_text(kind_label(d.kind))
                                .width(230.0)
                                .show_ui(ui, |ui| {
                                    for k in ConnectionKind::ALL {
                                        ui.selectable_value(&mut chosen_kind, k, kind_label(k));
                                    }
                                });
                        });
                        if chosen_kind != d.kind {
                            let (yaw, pitch, zoom) = (d.yaw, d.pitch, d.zoom);
                            d = defaults_for(state, &d.members, Some(chosen_kind));
                            d.yaw = yaw;
                            d.pitch = pitch;
                            d.zoom = zoom;
                        }

                        table::row(ui, "Espessura chapa", |ui| {
                            ui.add(
                                egui::DragValue::new(&mut d.plate_thickness_mm)
                                    .speed(0.5)
                                    .range(3.0..=80.0)
                                    .max_decimals(1)
                                    .suffix(" mm"),
                            );
                        });
                        table::row(ui, "Parafuso", |ui| {
                            egui::ComboBox::from_id_salt("steel-conn-bolt-d")
                                .selected_text(format!("{:.1} mm", d.bolt_diameter_mm))
                                .width(105.0)
                                .show_ui(ui, |ui| {
                                    for (dia, lbl) in [
                                        (12.0, "M12 (12 mm)"),
                                        (12.7, "1/2\" (12.7 mm)"),
                                        (15.9, "5/8\" (15.9 mm)"),
                                        (16.0, "M16 (16 mm)"),
                                        (19.0, "3/4\" (19.0 mm)"),
                                        (20.0, "M20 (20 mm)"),
                                        (22.2, "7/8\" (22.2 mm)"),
                                        (24.0, "M24 (24 mm)"),
                                        (25.4, "1\" (25.4 mm)"),
                                        (27.0, "M27 (27 mm)"),
                                        (30.0, "M30 (30 mm)"),
                                    ] {
                                        ui.selectable_value(&mut d.bolt_diameter_mm, dia, lbl);
                                    }
                                });
                            egui::ComboBox::from_id_salt("steel-conn-grade")
                                .selected_text(&d.bolt_grade)
                                .width(85.0)
                                .show_ui(ui, |ui| {
                                    for g in ["A307", "A325", "A490", "8.8", "10.9"] {
                                        ui.selectable_value(&mut d.bolt_grade, g.to_string(), g);
                                    }
                                });
                        });

                        if d.kind == ConnectionKind::BasePlate {
                            table::row(ui, "Base", |ui| {
                                egui::ComboBox::from_id_salt("steel-conn-base-mode")
                                    .selected_text(if d.base_mode == "fixed" { "Engastada" } else { "Rotulada" })
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut d.base_mode, "pinned".to_string(), "Rotulada");
                                        ui.selectable_value(&mut d.base_mode, "fixed".to_string(), "Engastada");
                                    });
                            });
                            table::row(ui, "Chumbadores", |ui| {
                                egui::ComboBox::from_id_salt("steel-conn-anchors")
                                    .selected_text(format!("{} unid.", d.anchors))
                                    .width(100.0)
                                    .show_ui(ui, |ui| {
                                        for n in [2u32, 4, 6, 8] {
                                            ui.selectable_value(&mut d.anchors, n, format!("{n} unid."));
                                        }
                                    });
                            });
                        } else {
                            table::row(ui, "Grade (L × C)", |ui| {
                                ui.add(egui::DragValue::new(&mut d.rows).speed(0.1).range(1..=12).prefix("L: "));
                                ui.add(egui::DragValue::new(&mut d.cols).speed(0.1).range(1..=6).prefix("C: "));
                            });
                            table::row(ui, "Passo / Borda", |ui| {
                                ui.add(
                                    egui::DragValue::new(&mut d.pitch_mm)
                                        .speed(1.0)
                                        .range(30.0..=200.0)
                                        .max_decimals(0)
                                        .suffix(" mm"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut d.edge_mm)
                                        .speed(1.0)
                                        .range(20.0..=120.0)
                                        .max_decimals(0)
                                        .suffix(" mm"),
                                );
                            });
                        }

                        if d.kind == ConnectionKind::EndPlate {
                            table::row(ui, "Extensão sup.", |ui| {
                                ui.add(
                                    egui::DragValue::new(&mut d.extension_mm)
                                        .speed(1.0)
                                        .range(0.0..=250.0)
                                        .max_decimals(0)
                                        .suffix(" mm"),
                                );
                            });
                        }
                        table::row(ui, "Enrijecedores", |ui| {
                            ui.checkbox(&mut d.stiffeners, "Incluir");
                        });
                    });
                } else {
                    draw_3d_preview(ui, &mut d);
                }
            });

            if let Some(err) = &d.error {
                ui.add_space(6.0);
                ui.label(RichText::new(err).color(PALETTE.danger));
            }
        },
        |f| {
            create_btn = f.primary("Criar ligação");
            f.secondary(t!("action-cancel"));
        },
    );

    let close = match response.clicked {
        Some(i) if i == create_btn => {
            let cmd = SteelConnectionCreate {
                elements: d.members.iter().map(|id| id.0).collect(),
                kind: Some(d.kind.code().to_string()),
                plate_thickness: Some(d.plate_thickness_mm / 1000.0),
                bolt_diameter: Some(d.bolt_diameter_mm / 1000.0),
                bolt_grade: Some(d.bolt_grade.clone()),
                rows: (d.kind != ConnectionKind::BasePlate).then_some(d.rows),
                cols: (d.kind != ConnectionKind::BasePlate).then_some(d.cols),
                pitch: (d.kind != ConnectionKind::BasePlate).then_some(d.pitch_mm / 1000.0),
                edge_distance: (d.kind != ConnectionKind::BasePlate).then_some(d.edge_mm / 1000.0),
                extension: (d.kind == ConnectionKind::EndPlate).then_some(d.extension_mm / 1000.0),
                stiffeners: Some(d.stiffeners),
                base_mode: (d.kind == ConnectionKind::BasePlate).then_some(d.base_mode.clone()),
                anchors: (d.kind == ConnectionKind::BasePlate).then_some(d.anchors),
                ..Default::default()
            };
            match state.session.run(cmd) {
                Ok(out) => {
                    if let Some(id) = out["id"].as_u64() {
                        state.selection.clear();
                        state.selection.insert(ElementId(id));
                    }
                    state.message = format!("Ligação {} criada.", kind_label(d.kind));
                    true
                }
                Err(e) => {
                    d.error = Some(e.message);
                    false
                }
            }
        }
        Some(_) => true,
        None => response.dismissed,
    };
    if !close {
        state.dialogs.steel_connection = Some(d);
    }
}
