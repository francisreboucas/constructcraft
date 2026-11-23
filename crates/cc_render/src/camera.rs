use cc_core::CameraState;
use cc_geom::Ray;
use glam::{DVec3, Mat4, Vec3};

const FOV_Y: f32 = 45.0;

fn eye_target(cam: &CameraState) -> (Vec3, Vec3) {
    (cam.eye().as_vec3(), cam.target.as_vec3())
}

/// Perspective view-projection matrix (Z up, wgpu depth range 0..1).
pub fn view_proj(cam: &CameraState, aspect: f32) -> Mat4 {
    let (eye, target) = eye_target(cam);
    let near = (cam.distance as f32 * 0.01).clamp(0.01, 1.0);
    let proj = glam::camera::rh::proj::directx::perspective(FOV_Y.to_radians(), aspect.max(1e-3), near, 5000.0);
    proj * glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z)
}

/// World-space ray through normalized device coordinates (`x`, `y` in -1..1, y up).
pub fn pick_ray(cam: &CameraState, aspect: f32, ndc_x: f32, ndc_y: f32) -> Ray {
    // Inverting a single-precision world matrix loses precision for models
    // placed far from the origin, especially at the far clipping plane.
    let forward = cam.forward();
    let right = forward.cross(DVec3::Z).normalize_or_zero();
    let up = right.cross(forward);
    let half_height = (f64::from(FOV_Y).to_radians() * 0.5).tan();
    let direction = forward
        + right * (f64::from(ndc_x) * f64::from(aspect.max(1e-3)) * half_height)
        + up * (f64::from(ndc_y) * half_height);
    let near = (cam.distance * 0.01).clamp(0.01, 1.0);
    Ray { origin: cam.eye() + direction * near, dir: direction.normalize_or_zero() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_ray_hits_target() {
        let cam = CameraState::default();
        let ray = pick_ray(&cam, 1.5, 0.0, 0.0);
        let to_target = (cam.target - ray.origin).normalize();
        assert!(ray.dir.dot(to_target) > 0.9999);
    }

    /// Large georeferenced coordinates must not change a pointer ray's direction.
    #[test]
    fn pointer_rays_are_stable_at_survey_coordinates() {
        let camera = CameraState::default();
        let shifted = CameraState { target: camera.target + DVec3::new(417_000.0, 78_000.0, 0.0), ..camera };
        for (x, y) in [(0.0, 0.0), (0.7, -0.5), (-0.8, 0.8)] {
            let local = pick_ray(&camera, 1.5, x, y);
            let survey = pick_ray(&shifted, 1.5, x, y);
            assert!(local.dir.distance(survey.dir) < 1e-12);
            assert!((survey.origin - local.origin - (shifted.target - camera.target)).length() < 1e-8);
        }
    }
}
