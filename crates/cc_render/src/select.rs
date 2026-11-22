//! Window and crossing selection in a 3D view: which elements a screen
//! rectangle takes, as the plans do with their linework. A window takes the
//! elements whose solids lie wholly inside it (and in front of the camera);
//! a crossing also takes those that any face of reaches into it. Hidden
//! faces count too, as in plans.

use crate::camera::view_proj;
use cc_core::{CameraState, ElementId};
use cc_geom::Solid;
use glam::{DVec2, DVec4};
use std::collections::{BTreeMap, BTreeSet};

/// Points closer to the eye plane than this (in clip `w`) are behind the camera.
const MIN_W: f64 = 1e-6;

/// Elements of `solids` (those `shown` accepts) taken by the rectangle from
/// `lo` to `hi` in normalized device coordinates (-1..1, y up): wholly
/// inside it when `window`, or reaching into it otherwise.
pub fn elements_in_rect(
    solids: &[Solid],
    cam: &CameraState,
    aspect: f32,
    lo: DVec2,
    hi: DVec2,
    window: bool,
    shown: &mut dyn FnMut(ElementId) -> bool,
) -> Vec<ElementId> {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let local_camera = CameraState { target: glam::DVec3::ZERO, ..*cam };
    let m = view_proj(&local_camera, aspect).as_dmat4();
    // Per element: whether every solid so far lies inside (window) or any reaches in (crossing).
    let mut inside: BTreeMap<ElementId, bool> = BTreeMap::new();
    let mut shown_cache: BTreeMap<ElementId, bool> = BTreeMap::new();
    for solid in solids {
        if solid.faces.is_empty() || !*shown_cache.entry(solid.element).or_insert_with(|| shown(solid.element)) {
            continue;
        }
        let entry = inside.entry(solid.element).or_insert(window);
        if window {
            *entry = *entry && solid_inside(solid, &m, cam.target, lo, hi);
        } else if !*entry {
            *entry = solid_reaches(solid, &m, cam.target, lo, hi);
        }
    }
    let taken: BTreeSet<ElementId> = inside.into_iter().filter_map(|(id, ok)| ok.then_some(id)).collect();
    taken.into_iter().collect()
}

fn clip(m: &glam::DMat4, p: glam::DVec3) -> DVec4 {
    *m * p.extend(1.0)
}

fn in_rect(p: DVec2, lo: DVec2, hi: DVec2) -> bool {
    p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y
}

/// Every vertex in front of the camera and inside the rectangle.
fn solid_inside(solid: &Solid, m: &glam::DMat4, origin: glam::DVec3, lo: DVec2, hi: DVec2) -> bool {
    solid.faces.iter().flat_map(|f| f.verts.iter()).all(|&p| {
        let c = clip(m, p - origin);
        c.w > MIN_W && in_rect(DVec2::new(c.x / c.w, c.y / c.w), lo, hi)
    })
}

/// Some face, clipped to the part in front of the camera, overlaps the rectangle.
fn solid_reaches(solid: &Solid, m: &glam::DMat4, origin: glam::DVec3, lo: DVec2, hi: DVec2) -> bool {
    solid.faces.iter().any(|f| {
        let polygon = front_part(f.verts.iter().map(|&p| clip(m, p - origin)));
        polygon_meets_rect(&polygon, lo, hi)
    })
}

/// The part of a polygon (in clip space) in front of the camera, in NDC
/// (Sutherland–Hodgman against the plane `w = MIN_W`).
fn front_part(points: impl Iterator<Item = DVec4>) -> Vec<DVec2> {
    let points: Vec<DVec4> = points.collect();
    let mut out = Vec::with_capacity(points.len() + 2);
    let n = points.len();
    for i in 0..n {
        let (Some(&a), Some(&b)) = (points.get(i), points.get((i + 1) % n)) else { continue };
        let (a_in, b_in) = (a.w > MIN_W, b.w > MIN_W);
        if a_in {
            out.push(a);
        }
        if a_in != b_in {
            let t = (MIN_W - a.w) / (b.w - a.w);
            out.push(a + (b - a) * t);
        }
    }
    out.into_iter().filter(|c| c.w > 0.0).map(|c| DVec2::new(c.x / c.w, c.y / c.w)).collect()
}

fn polygon_meets_rect(polygon: &[DVec2], lo: DVec2, hi: DVec2) -> bool {
    let n = polygon.len();
    if n == 0 {
        return false;
    }
    if polygon.iter().any(|&p| in_rect(p, lo, hi)) {
        return true;
    }
    let edges = || (0..n).filter_map(|i| Some((*polygon.get(i)?, *polygon.get((i + 1) % n)?)));
    if edges().any(|(a, b)| segment_meets_rect(a, b, lo, hi)) {
        return true;
    }
    // The rectangle wholly inside the polygon (a large face around it).
    let c = (lo + hi) * 0.5;
    n >= 3
        && edges()
            .filter(|(a, b)| (a.y > c.y) != (b.y > c.y) && c.x < a.x + (c.y - a.y) / (b.y - a.y) * (b.x - a.x))
            .count()
            % 2
            == 1
}

/// Liang–Barsky: whether segment `a`–`b` passes through the rectangle.
fn segment_meets_rect(a: DVec2, b: DVec2, lo: DVec2, hi: DVec2) -> bool {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-d.x, a.x - lo.x), (d.x, hi.x - a.x), (-d.y, a.y - lo.y), (d.y, hi.y - a.y)] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::Category;
    use glam::DVec3;

    /// A 1 m cube with its base centered at (`x`, 0, 0).
    fn cube(id: u64, x: f64) -> Solid {
        let mut s = Solid::new(ElementId(id), Category::Wall, [200; 3]);
        s.add_box(DVec2::new(x, 0.0), DVec2::X, DVec2::Y, -0.5, 0.5, -0.5, 0.5, 0.0, 1.0);
        s
    }

    /// Looking straight down at the origin from 20 m.
    fn top_camera() -> CameraState {
        CameraState { target: DVec3::new(0.0, 0.0, 0.5), distance: 20.0, yaw_deg: 0.0, pitch_deg: 89.0 }
    }

    fn ndc_of(cam: &CameraState, p: DVec3) -> DVec2 {
        let c = view_proj(cam, 1.0).as_dmat4() * p.extend(1.0);
        DVec2::new(c.x / c.w, c.y / c.w)
    }

    fn take(solids: &[Solid], cam: &CameraState, lo: DVec2, hi: DVec2, window: bool) -> Vec<u64> {
        elements_in_rect(solids, cam, 1.0, lo, hi, window, &mut |_| true).into_iter().map(|e| e.0).collect()
    }

    #[test]
    fn window_takes_whole_solids_and_crossing_those_it_touches() {
        let cam = top_camera();
        let solids = [cube(1, -3.0), cube(2, 0.0), cube(3, 3.0)];
        // A rectangle around the middle cube that also reaches halfway into the right one.
        let lo = ndc_of(&cam, DVec3::new(-1.0, -1.0, 0.5)).min(ndc_of(&cam, DVec3::new(3.0, 1.0, 0.5)));
        let hi = ndc_of(&cam, DVec3::new(-1.0, -1.0, 0.5)).max(ndc_of(&cam, DVec3::new(3.0, 1.0, 0.5)));
        assert_eq!(take(&solids, &cam, lo, hi, true), vec![2]);
        assert_eq!(take(&solids, &cam, lo, hi, false), vec![2, 3]);
        // A small rectangle inside the middle cube's top face: no vertex or
        // edge inside, but the face covers it.
        let c = ndc_of(&cam, DVec3::new(0.0, 0.0, 1.0));
        let d = DVec2::splat(1e-3);
        assert_eq!(take(&solids, &cam, c - d, c + d, false), vec![2]);
        assert!(take(&solids, &cam, c - d, c + d, true).is_empty());
    }

    /// Window selection stays the same after translating a model far from the origin.
    #[test]
    fn survey_coordinates_preserve_window_selection() {
        let camera = top_camera();
        let solids = [cube(1, -3.0), cube(2, 0.0), cube(3, 3.0)];
        let low = DVec2::splat(-0.2);
        let high = DVec2::splat(0.2);
        let expected = take(&solids, &camera, low, high, true);
        let offset = DVec3::new(417_626.123, 78_712.456, 0.0);
        let mut shifted = solids;
        for solid in &mut shifted {
            for face in &mut solid.faces {
                for vertex in &mut face.verts {
                    *vertex += offset;
                }
            }
        }
        let shifted_camera = CameraState { target: camera.target + offset, ..camera };
        assert_eq!(take(&shifted, &shifted_camera, low, high, true), expected);
        assert_eq!(expected, vec![2]);
    }

    #[test]
    fn hidden_elements_and_solids_behind_the_camera_are_left_out() {
        let cam = top_camera();
        let solids = [cube(1, 0.0), cube(2, 0.0)];
        let all = (DVec2::splat(-1.0), DVec2::splat(1.0));
        let ids: Vec<u64> = elements_in_rect(&solids, &cam, 1.0, all.0, all.1, true, &mut |e| e.0 != 2)
            .into_iter()
            .map(|e| e.0)
            .collect();
        assert_eq!(ids, vec![1]);
        // A cube above the eye: in no window, and crossing only reaches it
        // through the part in front, which is none.
        let mut above = Solid::new(ElementId(9), Category::Wall, [0; 3]);
        above.add_box(DVec2::ZERO, DVec2::X, DVec2::Y, -0.5, 0.5, -0.5, 0.5, 30.0, 31.0);
        assert!(take(&[above.clone()], &cam, all.0, all.1, true).is_empty());
        assert!(take(&[above], &cam, all.0, all.1, false).is_empty());
    }

    #[test]
    fn a_wall_crossing_the_eye_plane_is_taken_by_crossing_not_window() {
        // Camera inside a long wall's span: part of it behind the eye.
        let cam = CameraState { target: DVec3::new(0.0, 0.0, 1.0), distance: 5.0, yaw_deg: 0.0, pitch_deg: 0.0 };
        let mut wall = Solid::new(ElementId(4), Category::Wall, [0; 3]);
        let (eye, u) = (cam.eye(), cam.forward().truncate().normalize());
        // 2 m to the side of the eye, running 50 m behind and ahead of it.
        wall.add_box(eye.truncate() + u.perp() * 2.0, u, u.perp(), -50.0, 50.0, -0.1, 0.1, 0.0, 3.0);
        let all = (DVec2::splat(-1.0), DVec2::splat(1.0));
        assert!(take(&[wall.clone()], &cam, all.0, all.1, true).is_empty());
        assert_eq!(take(&[wall], &cam, all.0, all.1, false), vec![4]);
    }
}
