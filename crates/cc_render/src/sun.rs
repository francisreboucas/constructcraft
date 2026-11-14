//! A sun fixed in the world: the light it casts, the orthographic shadow map
//! fitted to the scene and the ground that catches its shadows. Pure math;
//! the renderer draws them (see `shadow.rs`).

use glam::{Mat4, Vec3, Vec4};

/// A sun fixed in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Unit vector from the model towards the sun (Z up).
    pub direction: Vec3,
    /// Cast shadows.
    pub shadows: bool,
    /// Height of a horizontal ground plane that receives shadows, if any.
    pub ground: Option<f32>,
}

impl Light {
    /// The light as the renderer uses it: its direction made a unit vector,
    /// `None` when it is zero or not a number (the camera's light is used
    /// then); a ground height that is not a number is left out.
    pub(crate) fn usable(self) -> Option<Self> {
        let direction = self.direction.try_normalize()?;
        Some(Self { direction, shadows: self.shadows, ground: self.ground.filter(|z| z.is_finite()) })
    }
}

/// Sets `current` to the usable form of `light`; returns whether it changed
/// (what has been drawn with the old one is then out of date).
pub(crate) fn replace(current: &mut Option<Light>, light: Option<Light>) -> bool {
    let light = light.and_then(Light::usable);
    let changed = *current != light;
    *current = light;
    changed
}

/// Side of the square shadow map, texels.
pub(crate) const SHADOW_MAP_SIZE: u32 = 2048;
/// A sun lower than this casts shadows on the ground no longer than it would
/// from this altitude, degrees.
const MIN_GROUND_ALTITUDE_DEG: f32 = 5.0;
/// Farthest the ground reaches past the scene, meters (the 3D view's far plane).
const MAX_GROUND_REACH: f32 = 5000.0;
/// A sun so low that its shadows run (all but) level catches none on the ground.
const MAX_GROUND_COT: f32 = 1.0e4;
/// The smallest half side of the light's view and depth range, meters, so a
/// flat or empty scene still makes an invertible matrix.
const MIN_HALF_EXTENT: f32 = 0.01;
/// How far a receiver is compared towards the sun, in shadow map texels (for
/// depth precision; the lookups' offset along the normal, in `sun.wgsl`,
/// keeps lit faces from shadowing themselves).
const DEPTH_BIAS_TEXELS: f32 = 0.25;
/// The ground is drawn this far below its height, meters (its depth bias
/// does the rest), so faces lying at that height cover it.
const GROUND_NUDGE: f32 = 1e-3;

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    /// The box around the finite `points`; `None` when there are none.
    pub fn of(points: impl IntoIterator<Item = [f32; 3]>) -> Option<Self> {
        let mut bounds: Option<Self> = None;
        for p in points.into_iter().map(Vec3::from_array).filter(|p| p.is_finite()) {
            bounds = Some(match bounds {
                Some(b) => Self { min: b.min.min(p), max: b.max.max(p) },
                None => Self { min: p, max: p },
            });
        }
        bounds
    }

    fn finite(&self) -> bool {
        self.min.is_finite() && self.max.is_finite() && self.min.cmple(self.max).all()
    }

    pub fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(a.x, b.y, b.z),
            Vec3::new(b.x, b.y, b.z),
        ]
    }
}

/// The frame of a sun towards unit `d`: the level direction its shadows
/// fall along, and the light's right and up axes across its rays (`right`
/// level, `right × up = d`).
fn frame(d: Vec3) -> (Vec3, Vec3, Vec3) {
    let along = Vec3::new(-d.x, -d.y, 0.0).try_normalize().unwrap_or(Vec3::X);
    let right = Vec3::new(-along.y, along.x, 0.0);
    let up = d.cross(right).try_normalize().unwrap_or(Vec3::Y);
    (along, right, up)
}

/// The light's view of a scene: world to its clip space, where x and y run
/// -1 to 1 across the shadow map and depth 0 (nearest the sun) to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShadowFit {
    pub view_proj: Mat4,
    /// The longer side of a map texel, meters.
    pub texel: f32,
    /// The depth range, meters.
    pub depth: f32,
}

/// The orthographic view from a sun towards unit `d` (above the horizon or
/// not) that holds every point of `bounds` (the casters, which set the depth
/// range) and, across the map, the `ground` corners (receivers behind or in
/// front of every caster are compared at the range's ends, where they are
/// lit). It depends on the scene and the sun only, so shadows stay put while
/// the camera moves. `None` for input that is not a number.
pub(crate) fn fit(bounds: &Bounds, d: Vec3, ground: Option<&[Vec3; 4]>, map: u32) -> Option<ShadowFit> {
    if !bounds.finite() || !d.is_finite() || (d.length() - 1.0).abs() > 1e-3 || map == 0 {
        return None;
    }
    let (_, right, up) = frame(d);
    let mut x = (f32::INFINITY, f32::NEG_INFINITY);
    let mut y = x;
    let mut z = x;
    let casters = bounds.corners();
    let receivers = ground.map(|g| g.as_slice()).unwrap_or(&[]);
    for p in casters.iter().chain(receivers) {
        let (px, py) = (p.dot(right), p.dot(up));
        x = (x.0.min(px), x.1.max(px));
        y = (y.0.min(py), y.1.max(py));
    }
    for p in &casters {
        let pz = p.dot(d);
        z = (z.0.min(pz), z.1.max(pz));
    }
    // A margin of two texels each side holds the 3 x 3 lookups of the
    // outermost receivers; the depth range is padded a little too.
    let grow = 1.0 + 4.0 / map as f32;
    let half = |r: (f32, f32)| ((r.1 - r.0) * 0.5).max(MIN_HALF_EXTENT) * grow;
    let (hx, hy) = (half(x), half(y));
    let (cx, cy) = ((x.0 + x.1) * 0.5, (y.0 + y.1) * 0.5);
    let pad = (z.1 - z.0) * 0.01 + MIN_HALF_EXTENT;
    let (near, far) = (z.1 + pad, z.0 - pad);
    let range = near - far;
    let rows = [
        Vec4::new(right.x / hx, right.y / hx, right.z / hx, -cx / hx),
        Vec4::new(up.x / hy, up.y / hy, up.z / hy, -cy / hy),
        Vec4::new(-d.x / range, -d.y / range, -d.z / range, near / range),
        Vec4::W,
    ];
    let view_proj = Mat4::from_cols(rows[0], rows[1], rows[2], rows[3]).transpose();
    let texel = 2.0 * hx.max(hy) / map as f32;
    (view_proj.is_finite() && texel.is_finite() && range.is_finite()).then_some(ShadowFit {
        view_proj,
        texel,
        depth: range,
    })
}

/// The ground at height `z` that catches the shadows of `bounds` from a sun
/// towards unit `d` (above the horizon): a level rectangle along the shadows
/// from the scene to the tip of the longest one, cut at the length a sun
/// `MIN_GROUND_ALTITUDE_DEG` up casts. Its corners, counter-clockwise seen
/// from above, are those of the scene slid along the sun's rays onto the
/// ground, so the light sees them within the scene: they fall inside its
/// shadow map. `None` when nothing stands above the ground, the sun is
/// (almost) level or the input is not a number.
pub(crate) fn ground_quad(bounds: &Bounds, d: Vec3, z: f32) -> Option<[Vec3; 4]> {
    if !bounds.finite() || !d.is_finite() || !z.is_finite() || d.z <= 0.0 {
        return None;
    }
    let height = bounds.max.z - z;
    let cot = Vec3::new(d.x, d.y, 0.0).length() / d.z;
    if height <= 0.0 || !cot.is_finite() || cot > MAX_GROUND_COT {
        return None;
    }
    let (along, right, _) = frame(d);
    let corners = bounds.corners();
    let range = |f: &dyn Fn(Vec3) -> f32| {
        corners.iter().map(|p| f(*p)).fold((f32::INFINITY, f32::NEG_INFINITY), |r, v| (r.0.min(v), r.1.max(v)))
    };
    // Along the shadows, where each corner's ray meets the ground; across
    // them, the rays keep the scene's width.
    let slid = range(&|p| p.dot(along) + (p.z - z) * cot);
    let own = range(&|p| p.dot(along));
    let (b0, b1) = range(&|p| p.dot(right));
    let reach = (height / MIN_GROUND_ALTITUDE_DEG.to_radians().tan()).min(MAX_GROUND_REACH);
    let (a0, a1) = (slid.0.max(own.0 - reach), slid.1.min(own.1 + reach));
    if !(a0 < a1 && b0 < b1) {
        return None;
    }
    let at = |a: f32, b: f32| along * a + right * b + Vec3::Z * z;
    let quad = [at(a0, b0), at(a1, b0), at(a1, b1), at(a0, b1)];
    quad.iter().all(|p| p.is_finite()).then_some(quad)
}

/// The sun's uniforms (`Sun` in `sun.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct SunUniforms {
    pub light_view_proj: [[f32; 4]; 4],
    /// xyz: unit vector towards the sun, zero below the horizon.
    pub dir: [f32; 4],
    /// x: 1 to look shadows up; y: a texel, meters; z: depth bias, in map
    /// depth; w: a texel, in map coordinates.
    pub shadow: [f32; 4],
    /// The ground's corners (xyz), counter-clockwise seen from above.
    pub ground: [[f32; 4]; 4],
}

/// What a frame lit by a sun draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SunSetup {
    pub uniforms: SunUniforms,
    /// The light's view-projection, when the shadow map is drawn.
    pub shadow: Option<Mat4>,
    /// The ground catches shadows.
    pub ground: bool,
}

/// The sun's setup for `light` (usable) over a scene within `bounds`: no
/// diffuse light, shadows or ground below the horizon; shadows only when
/// asked for and the scene has bounds.
pub(crate) fn setup(light: Light, bounds: Option<Bounds>, map: u32) -> SunSetup {
    let d = light.direction;
    let up = d.is_finite() && d.z > 0.0;
    let bounds = bounds.filter(|_| up && light.shadows);
    let quad = bounds.and_then(|b| ground_quad(&b, d, light.ground?));
    let fit = bounds.and_then(|b| fit(&b, d, quad.as_ref(), map));
    let mut uniforms = SunUniforms {
        light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        dir: if up { [d.x, d.y, d.z, 0.0] } else { [0.0; 4] },
        shadow: [0.0; 4],
        ground: [[0.0; 4]; 4],
    };
    if let Some(f) = fit {
        uniforms.light_view_proj = f.view_proj.to_cols_array_2d();
        uniforms.shadow = [1.0, f.texel, f.texel * DEPTH_BIAS_TEXELS / f.depth, 1.0 / map.max(1) as f32];
        if let Some(q) = quad {
            uniforms.ground = q.map(|p| [p.x, p.y, p.z - GROUND_NUDGE, 1.0]);
        }
    }
    SunSetup { uniforms, shadow: fit.map(|f| f.view_proj), ground: fit.is_some() && quad.is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Suns from 5° up to straight overhead, all round.
    fn suns() -> Vec<Vec3> {
        let mut out = Vec::new();
        for alt in [5.0f32, 12.0, 30.0, 45.0, 60.0, 85.0, 89.9, 90.0] {
            for az in (0..360).step_by(15) {
                let (alt, az) = (alt.to_radians(), (az as f32).to_radians());
                out.push(Vec3::new(alt.cos() * az.sin(), alt.cos() * az.cos(), alt.sin()));
            }
        }
        out
    }

    fn scenes() -> Vec<Bounds> {
        vec![
            // A small house, a block and a site.
            Bounds { min: Vec3::new(0.0, 0.0, 0.0), max: Vec3::new(5.0, 4.0, 3.0) },
            Bounds { min: Vec3::new(-12.0, 3.0, -0.5), max: Vec3::new(20.0, 15.0, 9.0) },
            Bounds { min: Vec3::new(-250.0, -250.0, -3.0), max: Vec3::new(250.0, 250.0, 60.0) },
            // Far from the origin.
            Bounds { min: Vec3::new(1000.0, 2000.0, 10.0), max: Vec3::new(1040.0, 2030.0, 25.0) },
        ]
    }

    fn clip(m: Mat4, p: Vec3) -> Vec3 {
        let c = m * p.extend(1.0);
        assert!((c.w - 1.0).abs() < 1e-6, "orthographic");
        c.truncate()
    }

    #[test]
    fn the_scene_and_its_ground_fit_in_the_light_view() {
        let eps = 1e-4;
        for b in scenes() {
            for d in suns() {
                for z in [b.min.z, b.min.z - 1.0, b.min.z + 0.5 * (b.max.z - b.min.z)] {
                    let quad = ground_quad(&b, d, z);
                    let quad = quad.unwrap_or_else(|| panic!("a ground under {b:?} at {z} from {d}"));
                    let f = fit(&b, d, Some(&quad), SHADOW_MAP_SIZE).unwrap();
                    assert!(f.view_proj.is_finite() && f.texel > 0.0 && f.depth > 0.0);
                    for p in b.corners() {
                        let c = clip(f.view_proj, p);
                        assert!(c.x.abs() <= 1.0 && c.y.abs() <= 1.0, "{p} from {d}: {c}");
                        assert!(c.z >= 0.0 && c.z <= 1.0, "{p} from {d}: {c}");
                    }
                    for p in quad {
                        assert!((p.z - z).abs() < 1e-3);
                        let c = clip(f.view_proj, p);
                        assert!(c.x.abs() <= 1.0 + eps && c.y.abs() <= 1.0 + eps, "ground {p} from {d}: {c}");
                    }
                    // The shadow of the scene's top corners (within the reach) lies on the ground.
                    let cot = Vec3::new(d.x, d.y, 0.0).length() / d.z;
                    if cot <= 1.0 / MIN_GROUND_ALTITUDE_DEG.to_radians().tan() {
                        let (along, right, _) = frame(d);
                        let a = |p: Vec3| (p.dot(along), p.dot(right));
                        let (q0, q2) = (a(quad[0]), a(quad[2]));
                        for p in b.corners().into_iter().filter(|p| p.z > z) {
                            let s = p - d * ((p.z - z) / d.z);
                            let (sa, sb) = a(s);
                            let tol = 1e-3 * (1.0 + sa.abs().max(sb.abs()));
                            assert!(sa >= q0.0 - tol && sa <= q2.0 + tol, "{s} beyond {quad:?}");
                            assert!(sb >= q0.1 - tol && sb <= q2.1 + tol, "{s} beside {quad:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn nearer_the_sun_is_shallower() {
        let b = scenes()[0];
        for d in suns() {
            let f = fit(&b, d, None, SHADOW_MAP_SIZE).unwrap();
            let c = Vec3::new(2.5, 2.0, 1.5);
            assert!(clip(f.view_proj, c + d).z < clip(f.view_proj, c).z);
        }
    }

    #[test]
    fn the_ground_is_counter_clockwise_and_shadows_reach_at_most_a_low_sun_s_length() {
        let b = scenes()[0];
        for d in suns() {
            let q = ground_quad(&b, d, 0.0).unwrap();
            let area: f32 = (0..4).map(|i| q[i].x * q[(i + 1) % 4].y - q[(i + 1) % 4].x * q[i].y).sum();
            assert!(area > 0.0, "{d}: {q:?}");
        }
        // A sun 1° up: shadows cut at the length of a 5° sun's.
        let d = Vec3::new(0.0, 1.0f32.to_radians().cos(), 1.0f32.to_radians().sin());
        let q = ground_quad(&b, d, 0.0).unwrap();
        let longest = 3.0 / 5.0f32.to_radians().tan();
        assert!(q.iter().all(|p| p.y >= -longest - 1e-3 && p.y <= 4.0 + 1e-3), "{q:?}");
        // Nothing above the ground, a level sun, a sun below the horizon.
        assert!(ground_quad(&b, Vec3::Z, 3.0).is_none());
        assert!(ground_quad(&b, Vec3::Y, 0.0).is_none());
        assert!(ground_quad(&b, Vec3::new(0.0, 0.6, -0.8), 0.0).is_none());
        assert!(ground_quad(&b, Vec3::new(0.0, 1.0, 1e-7).normalize(), 0.0).is_none());
    }

    #[test]
    fn degenerate_scenes_and_bad_input_make_no_nan() {
        assert_eq!(Bounds::of(std::iter::empty()), None);
        assert_eq!(Bounds::of([[f32::NAN, 0.0, 0.0], [f32::INFINITY, 1.0, 1.0]]), None);
        let one = Bounds::of([[1.0, 2.0, 3.0], [f32::NAN, 0.0, 0.0]]).unwrap();
        assert_eq!(one, Bounds { min: Vec3::new(1.0, 2.0, 3.0), max: Vec3::new(1.0, 2.0, 3.0) });
        // A point, a flat box: finite matrices holding them.
        let flat = Bounds { min: Vec3::ZERO, max: Vec3::new(10.0, 10.0, 0.0) };
        for b in [one, flat] {
            for d in suns() {
                let f = fit(&b, d, None, SHADOW_MAP_SIZE).unwrap();
                assert!(f.view_proj.is_finite() && f.texel.is_finite() && f.texel > 0.0);
                for p in b.corners() {
                    let c = clip(f.view_proj, p);
                    assert!(c.x.abs() <= 1.0 && c.y.abs() <= 1.0 && (0.0..=1.0).contains(&c.z), "{c}");
                }
            }
        }
        // The point stands on nothing: no ground. The flat box at its height neither.
        assert!(ground_quad(&one, Vec3::Z, 3.0).is_none() && ground_quad(&flat, Vec3::Z, 0.0).is_none());
        let nan = Vec3::new(f32::NAN, 0.0, 1.0);
        let bad = Bounds { min: Vec3::new(0.0, f32::NAN, 0.0), max: Vec3::ONE };
        assert!(fit(&flat, nan, None, SHADOW_MAP_SIZE).is_none() && fit(&bad, Vec3::Z, None, 2048).is_none());
        assert!(fit(&flat, Vec3::new(0.0, 0.0, 2.0), None, SHADOW_MAP_SIZE).is_none());
        assert!(ground_quad(&flat, nan, -1.0).is_none() && ground_quad(&flat, Vec3::Z, f32::NAN).is_none());
        assert!(ground_quad(&bad, Vec3::Z, -1.0).is_none());
        let huge = Bounds { min: Vec3::splat(-1e30), max: Vec3::splat(1e30) };
        if let Some(f) = fit(&huge, Vec3::Z, None, SHADOW_MAP_SIZE) {
            assert!(f.view_proj.is_finite());
        }
    }

    #[test]
    fn only_usable_suns_light() {
        let light = |direction, ground| Light { direction, shadows: true, ground };
        assert_eq!(light(Vec3::ZERO, None).usable(), None);
        assert_eq!(light(Vec3::new(f32::NAN, 0.0, 1.0), None).usable(), None);
        assert_eq!(light(Vec3::new(f32::INFINITY, 0.0, 1.0), None).usable(), None);
        let l = light(Vec3::new(0.0, 0.0, 2.0), Some(f32::NAN)).usable().unwrap();
        assert_eq!((l.direction, l.ground), (Vec3::Z, None));
        // Below the horizon: no diffuse light, shadows or ground.
        let s = setup(light(Vec3::new(0.0, 0.6, -0.8), Some(0.0)), Some(scenes()[0]), SHADOW_MAP_SIZE);
        assert_eq!((s.uniforms.dir, s.uniforms.shadow[0], s.shadow, s.ground), ([0.0; 4], 0.0, None, false));
        // Above it, with and without shadows and ground, with and without a scene.
        let d = Vec3::new(0.3, 0.5, 0.8).normalize();
        let s = setup(light(d, Some(0.0)), Some(scenes()[0]), SHADOW_MAP_SIZE);
        assert!(s.shadow.is_some() && s.ground && s.uniforms.shadow[0] == 1.0 && s.uniforms.dir[2] > 0.0);
        assert!(s.uniforms.shadow.iter().all(|v| v.is_finite() && *v > 0.0));
        let s = setup(light(d, None), Some(scenes()[0]), SHADOW_MAP_SIZE);
        assert!(s.shadow.is_some() && !s.ground);
        let s = setup(Light { direction: d, shadows: false, ground: Some(0.0) }, Some(scenes()[0]), 2048);
        assert!(s.shadow.is_none() && !s.ground && s.uniforms.shadow[0] == 0.0 && s.uniforms.dir[2] > 0.0);
        let s = setup(light(d, Some(0.0)), None, SHADOW_MAP_SIZE);
        assert!(s.shadow.is_none() && !s.ground);
    }

    #[test]
    fn only_a_new_light_dirties_the_frame() {
        let sun = Light { direction: Vec3::new(1.0, 1.0, 1.0), shadows: true, ground: Some(0.0) };
        let mut current = None;
        assert!(!replace(&mut current, None));
        assert!(replace(&mut current, Some(sun)));
        // The same sun again (not yet a unit vector: it is compared as used).
        assert!(!replace(&mut current, Some(sun)));
        assert!(!replace(&mut current, Some(Light { direction: sun.direction * 2.0, ..sun })));
        assert!(replace(&mut current, Some(Light { shadows: false, ..sun })));
        assert!(replace(&mut current, Some(Light { ground: None, shadows: false, ..sun })));
        assert!(replace(&mut current, Some(Light { direction: Vec3::Z, shadows: false, ground: None })));
        // An unusable sun is no sun.
        assert!(replace(&mut current, Some(Light { direction: Vec3::ZERO, ..sun })));
        assert_eq!(current, None);
        assert!(!replace(&mut current, Some(Light { direction: Vec3::splat(f32::NAN), ..sun })));
    }
}
