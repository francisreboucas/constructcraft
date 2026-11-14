//! Navigation cube for 3D views: a cube aligned with the world axes and a
//! compass ring, drawn in the view's corner.
//!
//! Each cube face is split into a 3 x 3 grid: the center cell looks at the
//! face, edge cells look along the edge and corner cells give the
//! isometric views (26 directions). Dragging the cube orbits, dragging the
//! ring turns around the vertical axis, and the home button returns to the
//! default view.

use crate::icons::{self, Icon};
use crate::t;
use crate::theme::{self, PALETTE};
use cc_core::CameraState;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2, vec2};
use glam::DVec3;

/// Translation keys used at runtime (checked by the i18n tests).
#[cfg(test)]
pub const KEYS: [&str; 10] = [
    "cube-top",
    "cube-bottom",
    "cube-front",
    "cube-back",
    "cube-right",
    "cube-left",
    "axis-north",
    "axis-south",
    "axis-east",
    "axis-west",
];

/// Half size of the cube on screen, points.
const HALF: f32 = 22.0;
/// Radius of the compass ring in cube units.
const RING: f64 = 1.75;
/// Fraction of a face's half width taken by its center cell.
const CENTER: f64 = 0.6;

/// What the user asked of the cube this frame.
#[derive(Debug, Default)]
pub struct CubeAction {
    /// View from this direction (unit vector from the target to the eye).
    pub look_from: Option<DVec3>,
    /// Orbit by a drag (as on the canvas).
    pub orbit: Option<Vec2>,
    /// Turn around Z by this many degrees.
    pub turn: f64,
    pub home: bool,
}

struct Face {
    normal: DVec3,
    u: DVec3,
    v: DVec3,
    key: &'static str,
}

const FACES: [Face; 6] = [
    Face { normal: DVec3::Z, u: DVec3::X, v: DVec3::Y, key: "cube-top" },
    Face { normal: DVec3::NEG_Z, u: DVec3::X, v: DVec3::NEG_Y, key: "cube-bottom" },
    Face { normal: DVec3::NEG_Y, u: DVec3::X, v: DVec3::Z, key: "cube-front" },
    Face { normal: DVec3::Y, u: DVec3::NEG_X, v: DVec3::Z, key: "cube-back" },
    Face { normal: DVec3::X, u: DVec3::Y, v: DVec3::Z, key: "cube-right" },
    Face { normal: DVec3::NEG_X, u: DVec3::NEG_Y, v: DVec3::Z, key: "cube-left" },
];

/// Camera basis: right, up and forward.
fn basis(cam: &CameraState) -> (DVec3, DVec3, DVec3) {
    let fwd = cam.forward();
    let right = fwd.cross(DVec3::Z).try_normalize().unwrap_or(DVec3::X);
    (right, right.cross(fwd), fwd)
}

/// Yaw and pitch that look at the target from `dir` (keeping `yaw` when looking straight down or up).
pub fn angles_from(dir: DVec3, yaw: f64) -> (f64, f64) {
    let d = dir.try_normalize().unwrap_or(DVec3::Z);
    let pitch = d.z.asin().to_degrees().clamp(-89.0, 89.0);
    let yaw = if d.truncate().length() < 1e-6 {
        // Straight down or up: keep the nearest quarter turn so plans stay square.
        (yaw / 90.0).round() * 90.0
    } else {
        (-d.x).atan2(-d.y).to_degrees()
    };
    (yaw, pitch)
}

fn inside(poly: &[Pos2], p: Pos2) -> bool {
    let n = poly.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() > 1e-6 {
            if sign != 0.0 && c.signum() != sign {
                return false;
            }
            sign = c.signum();
        }
    }
    true
}

/// Show the cube in the top-right corner of `view`, its compass turned to
/// `north` (true north in plan, a unit vector; project +Y without a site).
pub fn show(ui: &mut Ui, view: Rect, cam: &CameraState, north: glam::DVec2) -> CubeAction {
    let mut action = CubeAction::default();
    let center = view.right_top() + vec2(-62.0, 66.0);
    let (right, up, fwd) = basis(cam);
    let to = |p: DVec3| center + vec2(p.dot(right) as f32, -p.dot(up) as f32) * HALF;

    let area = Rect::from_center_size(center, Vec2::splat(HALF * 2.0 * RING as f32 + 18.0));
    let resp = ui.interact(area, ui.id().with("view-cube"), Sense::click_and_drag());
    let hover = resp.hover_pos();
    let painter = ui.painter_at(area.expand(24.0));

    // Compass ring around the base of the cube.
    let ring: Vec<Pos2> = (0..48)
        .map(|i| to(DVec3::from((glam::DVec2::from_angle(i as f64 / 48.0 * std::f64::consts::TAU) * RING, -1.0))))
        .collect();
    let on_ring = |p: Pos2| {
        (0..ring.len()).any(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            let t = ((p - a).dot(b - a) / (b - a).length_sq().max(1e-6)).clamp(0.0, 1.0);
            (a + (b - a) * t).distance(p) < 7.0
        })
    };

    // Visible faces, far to near, and the cell under the pointer (nearest face first).
    let mut faces: Vec<&Face> = FACES.iter().filter(|f| f.normal.dot(fwd) < -1e-3).collect();
    faces.sort_by(|a, b| b.normal.dot(fwd).total_cmp(&a.normal.dot(fwd)));
    let cells = |f: &Face| -> Vec<(DVec3, Vec<Pos2>)> {
        let edges = [-1.0, -CENTER, CENTER, 1.0];
        let mut out = Vec::new();
        for (i, step_u) in [-1.0, 0.0, 1.0].iter().enumerate() {
            for (j, step_v) in [-1.0, 0.0, 1.0].iter().enumerate() {
                let (u0, u1, v0, v1) = (edges[i], edges[i + 1], edges[j], edges[j + 1]);
                let poly = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
                    .iter()
                    .map(|(a, b)| to(f.normal + f.u * *a + f.v * *b))
                    .collect();
                out.push((f.normal + f.u * *step_u + f.v * *step_v, poly));
            }
        }
        out
    };
    let hovered_cell = hover.filter(|_| !resp.dragged()).and_then(|p| {
        faces.iter().rev().find_map(|f| cells(f).into_iter().find(|(_, poly)| inside(poly, p)).map(|(d, _)| d))
    });
    let ring_hovered = hovered_cell.is_none() && hover.is_some_and(on_ring);

    let ring_color =
        if ring_hovered || (resp.dragged() && ring_hovered) { PALETTE.accent } else { PALETTE.border_strong };
    painter.add(egui::Shape::closed_line(ring.clone(), Stroke::new(5.0, PALETTE.panel.gamma_multiply(0.9))));
    painter.add(egui::Shape::closed_line(ring.clone(), Stroke::new(1.2, ring_color)));
    let north = north.try_normalize().unwrap_or(glam::DVec2::Y);
    let east = glam::DVec2::new(north.y, -north.x);
    for (dir, key) in [(north, "axis-north"), (-north, "axis-south"), (east, "axis-east"), (-east, "axis-west")] {
        let p = to(DVec3::from((dir * RING, -1.0)));
        let bold = key == "axis-north";
        painter.circle_filled(p, 7.0, PALETTE.panel);
        let color = if bold { PALETTE.accent } else { PALETTE.text_muted };
        painter.text(
            p,
            egui::Align2::CENTER_CENTER,
            crate::i18n::tr(key),
            egui::FontId::new(9.5, theme::semibold()),
            color,
        );
    }

    // Shadow, then faces with their cells.
    painter.circle_filled(center + vec2(0.0, 2.0), HALF * 1.45, Color32::from_black_alpha(14));
    for f in &faces {
        let facing = (-f.normal.dot(fwd)) as f32;
        for (dir, poly) in cells(f) {
            let hot = hovered_cell == Some(dir);
            let fill = if hot { PALETTE.accent_soft } else { PALETTE.surface.gamma_multiply(0.75 + 0.25 * facing) };
            painter.add(egui::Shape::convex_polygon(poly.clone(), fill, Stroke::NONE));
            if hot {
                painter.add(egui::Shape::closed_line(poly, Stroke::new(1.2, PALETTE.accent)));
            }
        }
        let outline: Vec<Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|(a, b)| to(f.normal + f.u * *a + f.v * *b))
            .collect();
        painter.add(egui::Shape::closed_line(outline, Stroke::new(1.0, PALETTE.border_strong)));
        if facing > 0.3 {
            let color = PALETTE.text.gamma_multiply(facing.clamp(0.4, 1.0));
            let size = 7.5 + 1.5 * facing;
            painter.text(
                to(f.normal),
                egui::Align2::CENTER_CENTER,
                crate::i18n::tr(f.key),
                egui::FontId::new(size, theme::semibold()),
                color,
            );
        }
    }

    // Home button above the cube.
    let home_rect = Rect::from_center_size(center + vec2(-HALF * 2.2, -HALF * 2.1), vec2(20.0, 20.0));
    let home = ui.interact(home_rect, ui.id().with("view-cube-home"), Sense::click());
    if home.hovered() {
        ui.painter().rect_filled(home_rect, 4.0, PALETTE.hover);
    }
    icons::paint(ui.painter(), home_rect.shrink(3.0), Icon::Home, PALETTE.text_muted);
    home.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, t!("cube-home")));
    if home.on_hover_text(t!("cube-home")).clicked() {
        action.home = true;
    }

    // Interaction.
    if resp.clicked()
        && let Some(dir) = hovered_cell
    {
        action.look_from = Some(dir);
    }
    if resp.dragged() {
        let start_on_ring = ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|p| on_ring(p) && !faces.iter().any(|f| cells(f).iter().any(|(_, poly)| inside(poly, p))));
        if start_on_ring {
            action.turn = resp.drag_delta().x as f64 * 0.5;
        } else {
            action.orbit = Some(resp.drag_delta());
        }
    }
    if resp.hovered() && !resp.dragged() {
        resp.on_hover_text(t!("cube-hint"));
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_give_expected_angles() {
        // From the south (front face) the camera looks north.
        assert_eq!(angles_from(DVec3::NEG_Y, 10.0), (0.0, 0.0));
        // From the east it looks west.
        let (yaw, pitch) = angles_from(DVec3::X, 0.0);
        assert!((yaw + 90.0).abs() < 1e-9 && pitch.abs() < 1e-9);
        // Straight down keeps the nearest quarter turn.
        assert_eq!(angles_from(DVec3::Z, 100.0), (90.0, 89.0));
        // Corner: from south-east, above.
        let (yaw, pitch) = angles_from(DVec3::new(1.0, -1.0, 1.0), 0.0);
        assert!((yaw + 45.0).abs() < 1e-9);
        assert!((pitch - 35.264389682754654).abs() < 1e-9);
        // The camera then really looks from that direction.
        let cam = CameraState { yaw_deg: yaw, pitch_deg: pitch, ..Default::default() };
        assert!((cam.forward() + DVec3::new(1.0, -1.0, 1.0).normalize()).length() < 1e-9);
    }

    #[test]
    fn faces_are_right_handed_and_outward() {
        for f in &FACES {
            assert!((f.u.cross(f.v) - f.normal).length() < 1e-12, "{}", f.key);
        }
    }
}
