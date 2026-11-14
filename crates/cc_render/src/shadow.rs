//! The sun's GPU side: its pipelines, uniforms and shadow map, made the first
//! time a frame is lit by a sun (see `sun.rs` for the math).

use crate::Vertex;
use crate::renderer::{DEPTH_FORMAT, FACE_BIAS, PipelineSpec, Renderer3D, Uniforms, frame_pipeline};
use crate::sun::{self, Bounds, Light, SunUniforms};
use std::future::Future;
use std::task::{Context, Poll, Waker};

/// The sun's shader module: the renderer's with the sun's entry points.
pub(crate) const SUN_SHADER: &str = concat!(include_str!("shader.wgsl"), "\n", include_str!("sun.wgsl"));
/// Pushes the ground's shadows back behind faces lying in its plane (which
/// are pushed back by [`FACE_BIAS`]), so floors at its height cover it.
const GROUND_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState { constant: 16, slope_scale: 4.0, clamp: 0.0 };
/// Casters are drawn into the shadow map where they are: the receivers'
/// lookups move off their surfaces instead (see `sun.wgsl`), which keeps
/// shadows on their casters.
const CASTER_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState { constant: 0, slope_scale: 0.0, clamp: 0.0 };

/// The opaque triangles that cast shadows.
pub(crate) struct Casters<'a> {
    pub vertices: &'a wgpu::Buffer,
    pub indices: &'a wgpu::Buffer,
    pub count: u32,
}

pub(crate) struct SunPass {
    /// Faces lit by the sun, opaque and blended, and the edges, as the
    /// renderer's own pipelines draw them.
    pub faces: wgpu::RenderPipeline,
    pub blended: wgpu::RenderPipeline,
    pub edges: wgpu::RenderPipeline,
    /// The ground that catches shadows.
    pub ground: wgpu::RenderPipeline,
    /// The shadow pass, depth only.
    casters: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    /// Group 1 of the sun's pipelines: its uniforms and shadow map.
    pub bind_group: wgpu::BindGroup,
    /// The light's view as the shadow pass's camera (group 0).
    light_uniforms: wgpu::Buffer,
    light_group: wgpu::BindGroup,
    map: wgpu::TextureView,
    map_size: u32,
    /// The ground is drawn (the sun is up and casts shadows on it).
    pub draws_ground: bool,
}

impl SunPass {
    /// The sun's pipelines over the renderer's `bind_layout` (group 0) and
    /// its texture `sampler`.
    pub fn new(device: &wgpu::Device, bind_layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cc_render sun shader"),
            source: wgpu::ShaderSource::Wgsl(SUN_SHADER.into()),
        });
        let entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry { binding, visibility, ty, count: None };
        let sun_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cc_render sun"),
            entries: &[
                entry(
                    0,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                ),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cc_render sun layout"),
            bind_group_layouts: &[Some(bind_layout), Some(&sun_layout)],
            immediate_size: 0,
        });
        let tri = wgpu::PrimitiveTopology::TriangleList;
        let spec = |label, topology, (vs, fs), bias, blend| PipelineSpec {
            label,
            topology,
            vs,
            fs,
            vertices: true,
            cull: None,
            bias,
            blend,
        };
        let pipeline = |spec| frame_pipeline(device, &layout, &shader, spec);
        let faces = pipeline(spec("cc_render sun faces", tri, ("vs_sun", "fs_sun_face"), FACE_BIAS, None));
        let alpha = Some(wgpu::BlendState::ALPHA_BLENDING);
        let blended = pipeline(spec("cc_render sun blended", tri, ("vs_sun", "fs_sun_face"), FACE_BIAS, alpha));
        let lines = wgpu::PrimitiveTopology::LineList;
        let edges = pipeline(spec("cc_render sun edges", lines, ("vs_main", "fs_edge"), Default::default(), None));
        // Seen from above only; drawn over what is in front of it, not hiding anything.
        let ground = pipeline(PipelineSpec {
            vertices: false,
            cull: Some(wgpu::Face::Back),
            ..spec("cc_render ground", tri, ("vs_ground", "fs_ground"), GROUND_BIAS, alpha)
        });

        let casters_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cc_render shadow layout"),
            bind_group_layouts: &[Some(bind_layout)],
            immediate_size: 0,
        });
        let position = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }];
        let casters = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cc_render shadow"),
            layout: Some(&casters_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &position,
                })],
            },
            primitive: wgpu::PrimitiveState { topology: tri, cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: CASTER_BIAS,
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cc_render sun"),
            size: std::mem::size_of::<SunUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let map_size = sun::SHADOW_MAP_SIZE.min(device.limits().max_texture_dimension_2d).max(1);
        let map = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("cc_render shadow map"),
                size: wgpu::Extent3d { width: map_size, height: map_size, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let compare = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cc_render shadow map"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cc_render sun"),
            layout: &sun_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&map) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&compare) },
            ],
        });
        let light_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cc_render light view"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layers = Renderer3D::texture_array(device, 1, 2, 1).create_view(&Renderer3D::array_view());
        let light_group = Renderer3D::bind(device, bind_layout, &light_uniforms, &layers, sampler);
        Self {
            faces,
            blended,
            edges,
            ground,
            casters,
            uniforms,
            bind_group,
            light_uniforms,
            light_group,
            map,
            map_size,
            draws_ground: false,
        }
    }

    /// Set the sun's uniforms for `light` (usable) over a mesh within
    /// `bounds`, and draw the shadow map of its opaque triangles when it
    /// casts shadows.
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        light: Light,
        bounds: Option<Bounds>,
        casters: Option<Casters>,
    ) {
        let setup = sun::setup(light, bounds, self.map_size);
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&setup.uniforms));
        self.draws_ground = setup.ground;
        let Some(view_proj) = setup.shadow else { return };
        let camera = Uniforms { view_proj: view_proj.to_cols_array_2d(), light_dir: [0.0; 4], style: [0.0; 4] };
        queue.write_buffer(&self.light_uniforms, 0, bytemuck::bytes_of(&camera));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cc_render shadow pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.map,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(c) = casters.filter(|c| c.count > 0) {
            pass.set_pipeline(&self.casters);
            pass.set_bind_group(0, &self.light_group, &[]);
            pass.set_vertex_buffer(0, c.vertices.slice(..));
            pass.set_index_buffer(c.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..c.count, 0, 0..1);
        }
    }
}

/// Run `make` catching the device errors it causes (wgpu aborts on those
/// left uncaught): `None` if there were any. Native devices report them at
/// once; a report still pending counts as none.
pub(crate) fn caught<T>(device: &wgpu::Device, make: impl FnOnce() -> T) -> Option<T> {
    let filters = [wgpu::ErrorFilter::OutOfMemory, wgpu::ErrorFilter::Internal, wgpu::ErrorFilter::Validation];
    let scopes = filters.map(|f| device.push_error_scope(f));
    let made = make();
    let mut failed = false;
    // Innermost first.
    for scope in scopes.into_iter().rev() {
        failed |= matches!(poll_once(scope.pop()), Some(Some(_)));
    }
    (!failed).then_some(made)
}

/// The output of a future that is ready at once.
fn poll_once<F: Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(v) => Some(v),
        Poll::Pending => None,
    }
}
