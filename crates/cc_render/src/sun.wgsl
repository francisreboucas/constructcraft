// The sun's entry points (see `sun.rs` and `shadow.rs`): compiled after
// `shader.wgsl`, whose declarations they use, into a module of their own.

struct Sun {
    // World to the shadow map's clip space: x and y -1 to 1 across the map,
    // depth 0 (nearest the sun) to 1.
    light_view_proj: mat4x4<f32>,
    // xyz: unit vector towards the sun, zero below the horizon.
    dir: vec4<f32>,
    // x: 1 to look shadows up; y: a texel, meters; z: depth bias, in map
    // depth; w: a texel, in map coordinates.
    shadow: vec4<f32>,
    // The ground's corners (xyz), counter-clockwise seen from above.
    ground: array<vec4<f32>, 4>,
};

@group(1) @binding(0) var<uniform> sun: Sun;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;

// How far a lookup moves off its surface along the normal, in shadow map
// texels, plus as many again per unit of the sine of its angle to the sun:
// the depth this puts between a lit plane and its own place in the map
// grows with the angle as fast as the plane's depth across the 3 x 3
// lookups does, so lit faces do not shadow themselves at any angle, while
// a point lifted off the ground next to a wall stays behind the wall (no
// gap between a caster and its shadow).
const OFFSET_TEXELS: f32 = 0.5;
const OFFSET_SLOPE_TEXELS: f32 = 2.0;
// What the unlit styles keep of a color where the sun does not reach.
const UNLIT_SHADOW: f32 = 0.7;
// How dark the ground's shadows are (black at this opacity).
const GROUND_SHADOW: f32 = 0.35;

struct SunOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) plane: vec2<f32>,
    @location(3) fg: vec4<f32>,
    @location(4) bg: vec4<f32>,
    @location(5) tex: vec4<f32>,
    // Where the fragment is, meters: at the centroid, so the pixels on the
    // edges of a face look shadows up on the face.
    @location(6) @interpolate(perspective, centroid) world: vec3<f32>,
};

@vertex
fn vs_sun(v: VsIn) -> SunOut {
    var o: SunOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.normal = v.normal;
    o.color = v.color;
    o.plane = v.plane;
    o.fg = v.fg;
    o.bg = v.bg;
    o.tex = v.tex;
    o.world = v.pos;
    return o;
}

// The shadow pass: depth only, seen from the sun (`u.view_proj` holds the light's view).
@vertex
fn vs_shadow(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.view_proj * vec4<f32>(pos, 1.0);
}

// How much of the sun reaches `world` on a surface of unit normal `n` at
// `facing` (the cosine of its angle) to it: 0 in shadow to 1, from the
// shadow map compared at 3 x 3 points around it (bilinear each); lit
// outside the map.
fn sunlit(world: vec3<f32>, n: vec3<f32>, facing: f32) -> f32 {
    let sine = sqrt(max(1.0 - facing * facing, 0.0));
    let offset = sun.shadow.y * (OFFSET_TEXELS + OFFSET_SLOPE_TEXELS * sine);
    let p = sun.light_view_proj * vec4<f32>(world + n * offset, 1.0);
    let uv = p.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return 1.0;
    }
    // A receiver in front of every caster or behind them all is compared at
    // the end of the range: lit in front, behind lit only where nothing casts.
    let depth = clamp(p.z - sun.shadow.z, 0.0, 1.0);
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = uv + vec2<f32>(f32(x), f32(y)) * sun.shadow.w;
            lit += textureSampleCompareLevel(shadow_map, shadow_sampler, at, depth);
        }
    }
    return lit / 9.0;
}

@fragment
fn fs_sun_face(i: SunOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let base = surface(i.color.rgb, i.plane, i.fg, i.bg, i.tex);
    let facing = dot(n, sun.dir.xyz);
    // The share of the sun that reaches the face: none where it is turned
    // away (no lookup), all when shadows are off.
    var reach = 1.0;
    if sun.shadow.x > 0.5 {
        reach = 0.0;
        if facing > 0.0 {
            reach = sunlit(i.world, n, facing);
        }
    }
    // The camera light's terms (`fs_face`), the sun in its place.
    let diffuse = max(facing, 0.0) * reach;
    let sky = 0.5 + 0.5 * n.z;
    let lit = 0.30 + 0.60 * diffuse + 0.25 * sky;
    // The unlit styles darken what the sun does not reach, when it casts shadows.
    var shade = 1.0;
    if sun.shadow.x > 0.5 {
        shade = mix(UNLIT_SHADOW, 1.0, reach * smoothstep(0.0, 0.05, facing));
    }
    let k = select(lit, shade, u.style.x > 0.5);
    return vec4<f32>(gamma_from_linear(base * k), i.color.a);
}

struct GroundOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
};

// The ground that catches shadows: two triangles over its four corners.
@vertex
fn vs_ground(@builtin(vertex_index) index: u32) -> GroundOut {
    var corner = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);
    let p = sun.ground[corner[min(index, 5u)]].xyz;
    var o: GroundOut;
    o.clip = u.view_proj * vec4<f32>(p, 1.0);
    o.world = p;
    return o;
}

// Clear where the sun reaches the ground, a dark veil where it does not.
@fragment
fn fs_ground(i: GroundOut) -> @location(0) vec4<f32> {
    let shadow = 1.0 - sunlit(i.world, vec3<f32>(0.0, 0.0, 1.0), sun.dir.z);
    return vec4<f32>(0.0, 0.0, 0.0, GROUND_SHADOW * shadow);
}
