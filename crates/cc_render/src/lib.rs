//! Offscreen wgpu renderer for the 3D view.
//!
//! The app builds a [`SceneMesh`] from the model geometry, uploads it with
//! [`Renderer3D::upload`] and calls [`Renderer3D::render`] each frame; the
//! resulting texture is shown as an egui image. [`Renderer3D::set_light`]
//! lights it by a sun fixed in the world, with cast shadows.

mod camera;
mod mesh;
mod renderer;
mod select;
mod shadow;
mod sun;
pub mod textures;

pub use camera::{pick_ray, view_proj};
pub use mesh::{
    Highlight, MAX_HIDDEN_RUNS, SceneMesh, Shown, SolidStyle, Span, Tessellation, Vertex, hidden_runs, plane_axes,
};
pub use renderer::Renderer3D;
pub use select::elements_in_rect;
pub use sun::Light;
pub use textures::TextureSet;
