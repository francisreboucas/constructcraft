use crate::shadow::{Casters, SunPass, caught};
use crate::sun::{self, Bounds, Light};
use crate::textures::{LAYER_SIZE, TextureSet, mip_count};
use crate::{Highlight, SceneMesh, Shown, Vertex, view_proj};
use cc_core::CameraState;
use std::ops::Range;
use wgpu::util::DeviceExt;

// egui samples native textures as gamma-encoded values, so the shader encodes sRGB itself.
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;
const BACKGROUND: wgpu::Color = wgpu::Color { r: 0.84, g: 0.88, b: 0.93, a: 1.0 };
/// Pushes faces slightly back so coplanar edges win the depth test.
pub(crate) const FACE_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState { constant: 2, slope_scale: 1.5, clamp: 0.0 };
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x2];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
    /// x: 1 for unlit faces.
    pub style: [f32; 4],
}

/// What sets a pipeline drawing into the frame's targets apart.
pub(crate) struct PipelineSpec<'a> {
    pub label: &'a str,
    pub topology: wgpu::PrimitiveTopology,
    pub vs: &'a str,
    pub fs: &'a str,
    /// It reads the mesh's vertices (else it makes its own).
    pub vertices: bool,
    pub cull: Option<wgpu::Face>,
    pub bias: wgpu::DepthBiasState,
    /// Blended without writing depth; opaque with `None`.
    pub blend: Option<wgpu::BlendState>,
}

/// A pipeline drawing into the frame's multisampled color and depth targets.
pub(crate) fn frame_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    spec: PipelineSpec,
) -> wgpu::RenderPipeline {
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let buffers = [Some(vertex_layout)];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(spec.vs),
            compilation_options: Default::default(),
            buffers: if spec.vertices { &buffers } else { &[] },
        },
        primitive: wgpu::PrimitiveState { topology: spec.topology, cull_mode: spec.cull, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(spec.blend.is_none()),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: spec.bias,
        }),
        multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(spec.fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: spec.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

struct Targets {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve_texture: wgpu::Texture,
    resolve: wgpu::TextureView,
}

struct Buffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    transparent: wgpu::Buffer,
    transparent_count: u32,
    edges: wgpu::Buffer,
    edge_count: u32,
}

/// A [`Highlight`] on the GPU: its opaque faces and edges, and the parts of
/// the base mesh to draw.
struct HighlightBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    edges: wgpu::Buffer,
    edge_count: u32,
    shown: Option<Shown>,
}

/// The ranges to draw of a buffer of `count` items: all of it, or the
/// ranges `shown` keeps, cut to the buffer.
fn ranges(shown: Option<&[Range<u32>]>, count: u32) -> Vec<Range<u32>> {
    let all = std::slice::from_ref(&(0..u32::MAX));
    shown.unwrap_or(all).iter().map(|r| r.start.min(count)..r.end.min(count)).filter(|r| !r.is_empty()).collect()
}

/// Draws a [`SceneMesh`] into an offscreen, multisampled texture, with an
/// optional [`Highlight`] (the selection) over it, lit from the camera or by
/// a sun fixed in the world (see [`Renderer3D::set_light`]).
pub struct Renderer3D {
    faces: wgpu::RenderPipeline,
    /// Blended faces that do not write depth (transparent materials).
    blended: wgpu::RenderPipeline,
    edges: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    /// The generation of the textures uploaded (see [`TextureSet::generation`]).
    textures: Option<u64>,
    buffers: Option<Buffers>,
    highlight: Option<HighlightBuffers>,
    targets: Option<Targets>,
    /// Camera and size of the frame in the targets, while it is current:
    /// rendering it again is skipped.
    drawn: Option<(CameraState, [u32; 2])>,
    /// The mesh's faces are drawn without light.
    unlit: bool,
    /// The sun, usable (see [`Light::usable`]); `None` lights from the camera.
    light: Option<Light>,
    /// The box around the uploaded mesh's vertices.
    bounds: Option<Bounds>,
    /// The sun's pipelines and shadow map, made for the first frame it lights.
    sun: Option<SunPass>,
    /// Making them failed on this device: frames are lit from the camera.
    sun_failed: bool,
    /// The sun's uniforms and shadow map are those of `light` and the
    /// uploaded mesh (they do not depend on the camera or the size).
    shadows_current: bool,
}

impl Renderer3D {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cc_render shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cc_render uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cc_render layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cc_render uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cc_render textures"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        // No textures yet: an empty (black) array of two 1-pixel layers.
        let layers = Self::texture_array(device, 1, 2, 1).create_view(&Self::array_view());
        let bind_group = Self::bind(device, &bind_layout, &uniforms, &layers, &sampler);

        let pipeline = |label, topology, fs, bias, blend| {
            let spec = PipelineSpec { label, topology, vs: "vs_main", fs, vertices: true, cull: None, bias, blend };
            frame_pipeline(device, &layout, &shader, spec)
        };
        let tri = wgpu::PrimitiveTopology::TriangleList;
        let faces = pipeline("cc_render faces", tri, "fs_face", FACE_BIAS, None);
        let blended = pipeline("cc_render blended", tri, "fs_face", FACE_BIAS, Some(wgpu::BlendState::ALPHA_BLENDING));
        let edges = pipeline("cc_render edges", wgpu::PrimitiveTopology::LineList, "fs_edge", Default::default(), None);
        Self {
            faces,
            blended,
            edges,
            uniforms,
            bind_layout,
            sampler,
            bind_group,
            textures: None,
            buffers: None,
            highlight: None,
            targets: None,
            drawn: None,
            unlit: false,
            light: None,
            bounds: None,
            sun: None,
            sun_failed: false,
            shadows_current: false,
        }
    }

    /// Light the frames by `light`, a sun fixed in the world, or from the
    /// camera with `None` (the default). A direction that is zero or not a
    /// number counts as `None`. The frame is drawn again, and the shadow map
    /// made again, only when the light changes.
    pub fn set_light(&mut self, light: Option<Light>) {
        if sun::replace(&mut self.light, light) {
            self.drawn = None;
            self.shadows_current = false;
        }
    }

    /// A texture array of `layers` square layers of `size` pixels with `mips` levels.
    pub(crate) fn texture_array(device: &wgpu::Device, size: u32, layers: u32, mips: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cc_render textures"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Views of the texture array are arrays even with one layer.
    pub(crate) fn array_view() -> wgpu::TextureViewDescriptor<'static> {
        wgpu::TextureViewDescriptor {
            label: Some("cc_render textures"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        }
    }

    pub(crate) fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &wgpu::Buffer,
        layers: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cc_render uniforms"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(layers) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        })
    }

    /// Upload the materials' textures (when they changed since the last
    /// call); meshes built with [`SceneMesh::build_textured`] and the same
    /// set refer to their layers.
    pub fn set_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, set: &TextureSet) {
        if self.textures == Some(set.generation()) {
            return;
        }
        self.textures = Some(set.generation());
        self.drawn = None;
        let layers = set.len().max(2) as u32;
        let (size, mips) = if set.is_empty() { (1, 1) } else { (LAYER_SIZE, mip_count()) };
        let texture = Self::texture_array(device, size, layers, mips);
        for level in 0..mips {
            let side = (size >> level).max(1);
            for (layer, pixels) in set.level(level as usize).enumerate() {
                if pixels.len() != (side * side * 4) as usize {
                    continue;
                }
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: level,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(side * 4),
                        rows_per_image: Some(side),
                    },
                    wgpu::Extent3d { width: side, height: side, depth_or_array_layers: 1 },
                );
            }
        }
        let view = texture.create_view(&Self::array_view());
        self.bind_group = Self::bind(device, &self.bind_layout, &self.uniforms, &view, &self.sampler);
    }

    /// Upload a new base mesh; the highlight over the old one is dropped
    /// (its ranges refer to that mesh).
    pub fn upload(&mut self, device: &wgpu::Device, mesh: &SceneMesh) {
        self.drawn = None;
        self.unlit = mesh.unlit;
        self.highlight = None;
        self.bounds = Bounds::of(mesh.vertices.iter().map(|v| v.pos));
        self.shadows_current = false;
        if mesh.indices.is_empty() && mesh.transparent_indices.is_empty() && mesh.edges.is_empty() {
            self.buffers = None;
            return;
        }
        // A buffer beyond the device's limit is a validation error, which
        // wgpu treats as fatal: draw nothing rather than abort the app.
        let largest = [
            std::mem::size_of_val(mesh.vertices.as_slice()),
            std::mem::size_of_val(mesh.edges.as_slice()),
            std::mem::size_of_val(mesh.indices.as_slice()),
            std::mem::size_of_val(mesh.transparent_indices.as_slice()),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        if !fits(largest, device.limits().max_buffer_size) {
            self.buffers = None;
            return;
        }
        let init = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };
        // Zero-length buffers are invalid; pad with a dummy vertex.
        let pad = [Vertex::plain([0.0; 3], [0.0; 3], [0.0; 4])];
        let verts: &[Vertex] = if mesh.vertices.is_empty() { &pad } else { &mesh.vertices };
        let edges: &[Vertex] = if mesh.edges.is_empty() { &pad } else { &mesh.edges };
        let indices: &[u32] = if mesh.indices.is_empty() { &[0, 0, 0] } else { &mesh.indices };
        let transparent: &[u32] =
            if mesh.transparent_indices.is_empty() { &[0, 0, 0] } else { &mesh.transparent_indices };
        self.buffers = Some(Buffers {
            vertices: init("cc_render vertices", bytemuck::cast_slice(verts), wgpu::BufferUsages::VERTEX),
            indices: init("cc_render indices", bytemuck::cast_slice(indices), wgpu::BufferUsages::INDEX),
            index_count: mesh.indices.len() as u32,
            transparent: init("cc_render transparent", bytemuck::cast_slice(transparent), wgpu::BufferUsages::INDEX),
            transparent_count: mesh.transparent_indices.len() as u32,
            edges: init("cc_render edges", bytemuck::cast_slice(edges), wgpu::BufferUsages::VERTEX),
            edge_count: mesh.edges.len() as u32,
        });
    }

    /// Draw `highlight` (made for the mesh last uploaded) over that mesh, or
    /// nothing with `None`; the base mesh's buffers stay as they are.
    pub fn set_highlight(&mut self, device: &wgpu::Device, highlight: Option<&Highlight>) {
        self.drawn = None;
        self.highlight = None;
        let Some(h) = highlight else { return };
        let mesh = &h.mesh;
        let largest = [
            std::mem::size_of_val(mesh.vertices.as_slice()),
            std::mem::size_of_val(mesh.edges.as_slice()),
            std::mem::size_of_val(mesh.indices.as_slice()),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        if !fits(largest, device.limits().max_buffer_size) {
            return;
        }
        let init = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };
        let pad = [Vertex::plain([0.0; 3], [0.0; 3], [0.0; 4])];
        let verts: &[Vertex] = if mesh.vertices.is_empty() { &pad } else { &mesh.vertices };
        let edges: &[Vertex] = if mesh.edges.is_empty() { &pad } else { &mesh.edges };
        let indices: &[u32] = if mesh.indices.is_empty() { &[0, 0, 0] } else { &mesh.indices };
        self.highlight = Some(HighlightBuffers {
            vertices: init("cc_render highlight vertices", bytemuck::cast_slice(verts), wgpu::BufferUsages::VERTEX),
            indices: init("cc_render highlight indices", bytemuck::cast_slice(indices), wgpu::BufferUsages::INDEX),
            index_count: mesh.indices.len() as u32,
            edges: init("cc_render highlight edges", bytemuck::cast_slice(edges), wgpu::BufferUsages::VERTEX),
            edge_count: mesh.edges.len() as u32,
            shown: h.shown.clone(),
        });
    }

    /// (Re)create the render targets when the size changes; returns whether they were recreated.
    fn ensure_targets(&mut self, device: &wgpu::Device, size: [u32; 2]) -> bool {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return false;
        }
        self.targets = Some(Self::create_targets(device, size));
        true
    }

    fn create_targets(device: &wgpu::Device, size: [u32; 2]) -> Targets {
        let texture = |label, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let view = |t: wgpu::Texture| t.create_view(&Default::default());
        let resolve_texture = texture(
            "cc_render resolve",
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        Targets {
            size,
            msaa: view(texture("cc_render msaa", COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT)),
            depth: view(texture("cc_render depth", DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT)),
            resolve: resolve_texture.create_view(&Default::default()),
            resolve_texture,
        }
    }

    /// Render a frame (unless the last one, with the same mesh, light, camera
    /// and size, is still in the targets). Returns the output view and whether it
    /// was recreated (callers must then re-register it with egui); `None` if
    /// there are no targets.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        camera: &CameraState,
    ) -> Option<(&wgpu::TextureView, bool)> {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        let recreated = self.ensure_targets(device, size);
        if !recreated && self.drawn == Some((*camera, size)) {
            return self.targets.as_ref().map(|t| (&t.resolve, false));
        }
        self.drawn = Some((*camera, size));
        let aspect = size[0] as f32 / size[1] as f32;
        let fwd = camera.forward().as_vec3();
        // Sun from behind the viewer's left shoulder, high up.
        let light = (glam::Vec3::new(-fwd.x + 0.4, -fwd.y + 0.3, 1.6)).normalize();
        let u = Uniforms {
            view_proj: view_proj(camera, aspect).to_cols_array_2d(),
            light_dir: [light.x, light.y, light.z, 0.0],
            style: [if self.unlit { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));
        if self.light.is_some() && self.sun.is_none() && !self.sun_failed {
            // A device that cannot make them keeps the camera's light.
            self.sun = caught(device, || SunPass::new(device, &self.bind_layout, &self.sampler));
            self.sun_failed = self.sun.is_none();
        }

        let targets = self.targets.as_ref()?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("cc_render") });
        // The sun's shadow map is drawn again only for a new light or mesh.
        let sun = match (self.light, self.sun.as_mut()) {
            (Some(light), Some(sun)) => {
                if !self.shadows_current {
                    let casters = self.buffers.as_ref().map(|b| Casters {
                        vertices: &b.vertices,
                        indices: &b.indices,
                        count: b.index_count,
                    });
                    sun.update(device, queue, &mut encoder, light, self.bounds, casters);
                    self.shadows_current = true;
                }
                Some(&*sun)
            }
            _ => None,
        };
        let (faces, blended, edges) = match sun {
            Some(s) => (&s.faces, &s.blended, &s.edges),
            None => (&self.faces, &self.blended, &self.edges),
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cc_render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.msaa,
                    depth_slice: None,
                    resolve_target: Some(&targets.resolve),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BACKGROUND), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            if let Some(s) = sun {
                pass.set_bind_group(1, &s.bind_group, &[]);
            }
            // The base mesh leaves out what the highlight draws in its place.
            let h = self.highlight.as_ref();
            let shown = h.and_then(|h| h.shown.as_ref());
            if let Some(b) = &self.buffers {
                let opaque = ranges(shown.map(|s| s.indices.as_slice()), b.index_count);
                if !opaque.is_empty() {
                    pass.set_pipeline(faces);
                    pass.set_vertex_buffer(0, b.vertices.slice(..));
                    pass.set_index_buffer(b.indices.slice(..), wgpu::IndexFormat::Uint32);
                    for r in opaque {
                        pass.draw_indexed(r, 0, 0..1);
                    }
                }
            }
            if let Some(h) = h
                && h.index_count > 0
            {
                pass.set_pipeline(faces);
                pass.set_vertex_buffer(0, h.vertices.slice(..));
                pass.set_index_buffer(h.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..h.index_count, 0, 0..1);
            }
            if let Some(b) = &self.buffers {
                let base_edges = ranges(shown.map(|s| s.edges.as_slice()), b.edge_count);
                if !base_edges.is_empty() {
                    pass.set_pipeline(edges);
                    pass.set_vertex_buffer(0, b.edges.slice(..));
                    for r in base_edges {
                        pass.draw(r, 0..1);
                    }
                }
            }
            // After the base edges: lines the highlight shares with the base win the depth test.
            if let Some(h) = h
                && h.edge_count > 0
            {
                pass.set_pipeline(edges);
                pass.set_vertex_buffer(0, h.edges.slice(..));
                pass.draw(0..h.edge_count, 0..1);
            }
            // The ground's shadows over the opaque model, under what shows through glass.
            if let Some(s) = sun.filter(|s| s.draws_ground) {
                pass.set_pipeline(&s.ground);
                pass.draw(0..6, 0..1);
            }
            // Transparent faces last, over everything opaque.
            if let Some(b) = &self.buffers {
                let transparent = ranges(shown.map(|s| s.transparent.as_slice()), b.transparent_count);
                if !transparent.is_empty() {
                    pass.set_pipeline(blended);
                    pass.set_vertex_buffer(0, b.vertices.slice(..));
                    pass.set_index_buffer(b.transparent.slice(..), wgpu::IndexFormat::Uint32);
                    for r in transparent {
                        pass.draw_indexed(r, 0, 0..1);
                    }
                }
            }
        }
        queue.submit([encoder.finish()]);
        Some((&targets.resolve, recreated))
    }

    /// Render one frame of `size` pixels and read it back as tightly packed
    /// RGBA bytes (already sRGB encoded), e.g. for thumbnails.
    pub fn render_to_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        camera: &CameraState,
    ) -> Option<Vec<u8>> {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        self.render(device, queue, size, camera)?;
        let targets = self.targets.as_ref()?;
        let row = size[0] * 4;
        let padded = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cc_render readback"),
            size: u64::from(padded) * u64::from(size[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("cc_render readback") });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &targets.resolve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(size[1]),
                },
            },
            wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        let data = slice.get_mapped_range().ok()?;
        let mut rgba = Vec::with_capacity((row * size[1]) as usize);
        for y in 0..size[1] as usize {
            let start = y * padded as usize;
            rgba.extend_from_slice(&data[start..start + row as usize]);
        }
        drop(data);
        buffer.unmap();
        Some(rgba)
    }
}

/// Whether a buffer of `bytes` is within the device's `limit`, and its
/// element counts fit the `u32` draw ranges.
fn fits(bytes: usize, limit: u64) -> bool {
    u64::try_from(bytes).is_ok_and(|b| b <= limit) && bytes / std::mem::size_of::<u32>() <= u32::MAX as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meshes_beyond_the_buffer_limit_are_not_uploaded() {
        let limit = wgpu::Limits::default().max_buffer_size;
        assert!(fits(0, limit) && fits(1 << 20, limit));
        assert!(!fits(usize::try_from(limit).unwrap_or(usize::MAX).saturating_add(1), limit));
    }

    #[test]
    fn shown_ranges_are_cut_to_the_buffer() {
        assert_eq!(ranges(None, 9), vec![0..9]);
        assert!(ranges(None, 0).is_empty());
        let shown = [0..3, 5..8, 12..u32::MAX];
        assert_eq!(ranges(Some(&shown), 10), vec![0..3, 5..8]);
        assert_eq!(ranges(Some(&shown), 20), vec![0..3, 5..8, 12..20]);
    }
}

#[cfg(test)]
mod shader_tests {
    /// The shader parses and validates (as wgpu does when the pipelines are
    /// made): derivatives in uniform control flow, attributes and bindings.
    #[test]
    fn the_shader_validates() {
        let validate = |source: &str, entries: &[&str]| {
            let module = naga::front::wgsl::parse_str(source).unwrap();
            let mut validator =
                naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
            validator.validate(&module).unwrap();
            for entry in entries {
                assert!(module.entry_points.iter().any(|e| e.name == *entry), "{entry}");
            }
        };
        validate(include_str!("shader.wgsl"), &["vs_main", "fs_face", "fs_edge"]);
        let sun_entries = ["vs_main", "fs_edge", "vs_sun", "fs_sun_face", "vs_shadow", "vs_ground", "fs_ground"];
        validate(crate::shadow::SUN_SHADER, &sun_entries);
    }
}

/// Frames drawn on a GPU. They need an adapter, which CI machines may lack
/// or have only in software, so they are ignored by default: run them with
/// `cargo nextest run -p cc_render --run-ignored all` (`WGPU_BACKEND` picks
/// the backend). Without an adapter they pass without checking anything.
#[cfg(test)]
mod gpu_tests {
    use super::*;
    use crate::sun::Light;
    use glam::{DVec3, Vec3};
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
                return v;
            }
            std::thread::yield_now();
        }
    }

    fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = block_on(instance.request_adapter(&Default::default())).ok()?;
        block_on(adapter.request_device(&Default::default())).ok()
    }

    /// The faces of the box from `min` to `max`.
    fn add_box(mesh: &mut SceneMesh, min: Vec3, max: Vec3, color: [f32; 4]) {
        let corner = |i: usize| {
            let pick = |bit: usize, a: f32, b: f32| if i & bit == 0 { a } else { b };
            [pick(1, min.x, max.x), pick(2, min.y, max.y), pick(4, min.z, max.z)]
        };
        let faces = [
            (Vec3::NEG_Z, [0, 2, 3, 1]),
            (Vec3::Z, [4, 5, 7, 6]),
            (Vec3::NEG_Y, [0, 1, 5, 4]),
            (Vec3::Y, [2, 6, 7, 3]),
            (Vec3::NEG_X, [0, 4, 6, 2]),
            (Vec3::X, [1, 3, 7, 5]),
        ];
        for (normal, quad) in faces {
            let start = mesh.vertices.len() as u32;
            mesh.vertices.extend(quad.map(|i| Vertex::plain(corner(i), normal.to_array(), color)));
            mesh.indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        }
    }

    const SIZE: [u32; 2] = [320, 240];

    /// The pixel at world point `p`.
    fn pixel(rgba: &[u8], camera: &CameraState, p: Vec3) -> [u8; 4] {
        let ndc = view_proj(camera, SIZE[0] as f32 / SIZE[1] as f32).project_point3(p);
        let x = ((ndc.x + 1.0) * 0.5 * SIZE[0] as f32) as usize;
        let y = ((1.0 - ndc.y) * 0.5 * SIZE[1] as f32) as usize;
        assert!(x < SIZE[0] as usize && y < SIZE[1] as usize, "{p} is off the frame");
        let at = (y * SIZE[0] as usize + x) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
    }

    fn brightness(c: [u8; 4]) -> i32 {
        i32::from(c[0]) + i32::from(c[1]) + i32::from(c[2])
    }

    /// A tower 2 x 2 x 4 m standing at the origin, on a slab or not.
    fn scene(slab: bool, unlit: bool) -> SceneMesh {
        let mut mesh = SceneMesh { unlit, ..Default::default() };
        if slab {
            add_box(&mut mesh, Vec3::new(-10.0, -10.0, -0.2), Vec3::new(10.0, 10.0, 0.0), [0.5, 0.5, 0.45, 1.0]);
        }
        add_box(&mut mesh, Vec3::ZERO, Vec3::new(2.0, 2.0, 4.0), [0.4, 0.5, 0.7, 1.0]);
        mesh
    }

    #[test]
    #[ignore = "needs a GPU adapter"]
    fn the_sun_casts_shadows_on_the_model_and_the_ground() {
        let Some((device, queue)) = gpu() else {
            eprintln!("no GPU adapter: nothing checked");
            return;
        };
        // From the south, looking down; the sun in the south-west, 40° up:
        // the tower's shadow falls north-east of it.
        let camera = CameraState { target: DVec3::new(1.0, 1.0, 0.0), yaw_deg: 0.0, pitch_deg: 60.0, distance: 25.0 };
        let sun = Light { direction: Vec3::new(-1.0, -1.0, 1.2), shadows: true, ground: Some(0.0) };
        let (shade, lit) = (Vec3::new(3.5, 3.5, 0.0), Vec3::new(-3.0, 3.0, 0.0));
        let draw = |mesh: &SceneMesh, light: Option<Light>, camera: &CameraState| {
            let mut r = Renderer3D::new(&device);
            r.upload(&device, mesh);
            r.set_light(light);
            r.render_to_rgba(&device, &queue, SIZE, camera).unwrap()
        };
        for unlit in [false, true] {
            let slab = scene(true, unlit);
            let frame = draw(&slab, Some(sun), &camera);
            let (s, l) = (pixel(&frame, &camera, shade), pixel(&frame, &camera, lit));
            assert!(brightness(s) + 60 < brightness(l), "unlit {unlit}: {s:?} in shadow, {l:?} lit");
            // The ground at the slab's top stays under it: the slab's shadow is not darkened twice.
            let bare = draw(&slab, Some(Light { ground: None, ..sun }), &camera);
            let b = pixel(&bare, &camera, shade);
            assert!((brightness(b) - brightness(s)).abs() <= 3, "{b:?} without the ground, {s:?} with it");
            // No sun: as the renderer always drew it, whatever light it had before.
            let before = draw(&slab, None, &camera);
            let mut r = Renderer3D::new(&device);
            r.upload(&device, &slab);
            r.set_light(Some(sun));
            assert_ne!(r.render_to_rgba(&device, &queue, SIZE, &camera).unwrap(), before);
            r.set_light(None);
            assert_eq!(r.render_to_rgba(&device, &queue, SIZE, &camera).unwrap(), before);
            // Below the horizon: no shadows, darker than in the sun.
            let night = draw(&slab, Some(Light { direction: Vec3::new(-1.0, -1.0, -0.5), ..sun }), &camera);
            let (ns, nl) = (pixel(&night, &camera, shade), pixel(&night, &camera, lit));
            assert!((brightness(ns) - brightness(nl)).abs() <= 3, "{ns:?} {nl:?}");
            assert!(unlit || brightness(nl) + 30 < brightness(l), "{nl:?} at night, {l:?} by day");
        }
        // The ground alone catches the shadow; where lit it shows the background.
        let tower = scene(false, false);
        let frame = draw(&tower, Some(sun), &camera);
        let background = [frame[0], frame[1], frame[2], frame[3]];
        assert_eq!(pixel(&frame, &camera, lit), background);
        let s = pixel(&frame, &camera, shade);
        assert!(brightness(s) + 100 < brightness(background), "{s:?}");
        // Seen from below, it is not drawn.
        let below = CameraState { pitch_deg: -30.0, ..camera };
        assert_eq!(draw(&tower, Some(sun), &below), draw(&tower, Some(Light { ground: None, ..sun }), &below));
    }

    #[test]
    #[ignore = "needs a GPU adapter"]
    fn only_a_new_light_redraws_the_frame() {
        let Some((device, queue)) = gpu() else {
            eprintln!("no GPU adapter: nothing checked");
            return;
        };
        let camera = CameraState::default();
        let sun = Light { direction: Vec3::new(0.3, -0.5, 0.8), shadows: true, ground: Some(0.0) };
        let mut r = Renderer3D::new(&device);
        r.upload(&device, &scene(true, false));
        r.render(&device, &queue, SIZE, &camera);
        assert!(r.drawn.is_some() && !r.shadows_current);
        r.set_light(None);
        assert!(r.drawn.is_some());
        r.set_light(Some(sun));
        assert!(r.drawn.is_none());
        r.render(&device, &queue, SIZE, &camera);
        assert!(r.drawn.is_some() && r.shadows_current);
        // The same sun (its direction as long or twice as long) keeps the frame and the shadow map.
        r.set_light(Some(sun));
        r.set_light(Some(Light { direction: sun.direction * 2.0, ..sun }));
        assert!(r.drawn.is_some() && r.shadows_current);
        // The camera moving keeps the shadow map.
        let turned = CameraState { yaw_deg: camera.yaw_deg + 10.0, ..camera };
        r.render(&device, &queue, SIZE, &turned);
        assert!(r.drawn == Some((turned, SIZE)) && r.shadows_current);
        r.set_light(Some(Light { ground: None, ..sun }));
        assert!(r.drawn.is_none() && !r.shadows_current);
        // A new mesh makes the shadow map again.
        r.render(&device, &queue, SIZE, &camera);
        r.upload(&device, &scene(false, false));
        assert!(r.drawn.is_none() && !r.shadows_current);
    }

    /// A sun that casts no shadows makes no shadow map of its size; the
    /// first one that does makes it, and it lights the same as a fresh one.
    #[test]
    #[ignore = "needs a GPU adapter"]
    fn the_shadow_map_is_made_only_for_shadows() {
        let Some((device, queue)) = gpu() else {
            eprintln!("no GPU adapter: nothing checked");
            return;
        };
        let camera = CameraState { target: DVec3::new(1.0, 1.0, 0.0), yaw_deg: 0.0, pitch_deg: 60.0, distance: 25.0 };
        let sun = Light { direction: Vec3::new(-1.0, -1.0, 1.2), shadows: true, ground: Some(0.0) };
        let mut r = Renderer3D::new(&device);
        r.upload(&device, &scene(true, false));
        r.set_light(Some(Light { shadows: false, ..sun }));
        r.render(&device, &queue, SIZE, &camera);
        assert_eq!(r.sun.as_ref().map(|s| s.map_size()), Some(1));
        // Below the horizon a shadow-casting sun casts none either.
        r.set_light(Some(Light { direction: Vec3::new(-1.0, -1.0, -0.5), ..sun }));
        r.render(&device, &queue, SIZE, &camera);
        assert_eq!(r.sun.as_ref().map(|s| s.map_size()), Some(1));
        r.set_light(Some(sun));
        let grown = r.render_to_rgba(&device, &queue, SIZE, &camera).unwrap();
        assert!(r.sun.as_ref().is_some_and(|s| s.map_size() > 1));
        let mut fresh = Renderer3D::new(&device);
        fresh.upload(&device, &scene(true, false));
        fresh.set_light(Some(sun));
        assert_eq!(fresh.render_to_rgba(&device, &queue, SIZE, &camera).unwrap(), grown);
    }
}
