//! Skybox will just render a flat color on a quad that cover all screen. It will discard all
//! fragments that have a depth < 1. Depth buffer is from the 3d scene. (Gbuffer)
//!
//! This is quite hacky but will do for now.
//!
use crate::colors::RgbColor;
use crate::render::shaders::Shaders;
use crate::render::OffscreenBuffer;
use luminance::context::GraphicsContext;
use luminance::pipeline::{Pipeline, ShadingGate};
use luminance::pixel::Floating;
use luminance::render_state::RenderState;
use luminance::shader::program::Program;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::{pipeline::BoundTexture, shader::program::Uniform, texture::Dim2};
use luminance_derive::UniformInterface;
use luminance_glfw::GlfwSurface;

pub type SkyboxProgram = Program<(), (), ShaderInterface>;

#[derive(UniformInterface)]
pub struct ShaderInterface {
    pub color: Uniform<[f32; 3]>,
    #[uniform(unbound)]
    pub depth_buffer: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,
}

pub struct SkyboxRenderer {
    quad: Tess,
    color: RgbColor,
}

impl SkyboxRenderer {
    pub fn new(surface: &mut GlfwSurface, color: RgbColor) -> Self {
        let quad = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        Self { quad, color }
    }

    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        offscreen: &OffscreenBuffer,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        let depth_buffer = pipeline.bind_texture(&offscreen.depth_slot());
        shd_gate.shade(&shaders.skybox_program, |iface, mut rdr_gate| {
            iface.color.update(self.color.to_normalized());
            iface.depth_buffer.update(&depth_buffer);
            rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                tess_gate.render(&self.quad);
            });
        });
    }
}
