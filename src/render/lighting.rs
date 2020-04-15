use crate::colors::RgbColor;
use crate::render::shaders::Shaders;
use crate::render::OffscreenBuffer;
use glam::Vec3;
use hecs::World;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::Floating;
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::Dim2;
use luminance_derive::UniformInterface;
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};

pub type AmbientLightProgram = Program<(), (), AmbientShaderInterface>;
pub type DirectionalLightProgram = Program<(), (), DirectionalShaderInterface>;

/// Component to add ambient lighting to a scene. Ambient lighting
/// is applying some light to all objects indiscriminately.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct AmbientLight {
    /// Color of the ambient lighting
    color: RgbColor,
    /// intensity of the light. Between 0 and 1.
    intensity: f32,
}

#[derive(UniformInterface)]
pub struct AmbientShaderInterface {
    // the diffuse texture.
    pub diffuse: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,

    // color of ambient lighting
    #[uniform(unbound)]
    pub color: Uniform<[f32; 3]>,

    // intensity (between 0 and 1)
    #[uniform(unbound)]
    pub intensity: Uniform<f32>,
}

/// Directional light will apply lighting on surfaces facing the direction of the light.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct DirectionalLight {
    // direction of the light.
    direction: Vec3,
    /// Color of the directional light
    color: RgbColor,
    /// intensity of the light. Between 0 and 1.
    intensity: f32,
}

#[derive(UniformInterface)]
pub struct DirectionalShaderInterface {
    /// the diffuse texture.
    pub diffuse: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,

    /// the normal texture
    pub normal: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,

    /// depth texture
    #[uniform(unbound)]
    pub depth: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,

    /// direction of the light.
    #[uniform(unbound)]
    pub direction: Uniform<[f32; 3]>,

    /// color of the light
    #[uniform(unbound)]
    pub color: Uniform<[f32; 3]>,

    /// intensity (between 0 and 1)
    #[uniform(unbound)]
    pub intensity: Uniform<f32>,
}

/// The lighting system will render the offscreen buffer to the screen framebuffer in multiple
/// lighting passes, each adding some color to the final result.
pub struct LightingSystem {
    quad: Tess,
    render_state: RenderState,
}

impl LightingSystem {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let quad = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        let render_state = RenderState::default()
            .set_blending((Equation::Additive, Factor::SrcAlpha, Factor::One))
            .set_depth_test(None);
        Self { quad, render_state }
    }

    /// Render the lights. In this function, we are already in the shading part of the renderering process.
    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        world: &World,
        offscreen: &OffscreenBuffer,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        // first extract the diffuse texture from the offscreen shader.
        let diffuse_texture = pipeline.bind_texture(&offscreen.color_slot().0);
        shd_gate.shade(&shaders.ambient_program, |iface, mut rdr_gate| {
            for (_, light) in world.query::<&AmbientLight>().iter() {
                iface.color.update(light.color.to_normalized());
                iface.intensity.update(light.intensity);
                iface.diffuse.update(&diffuse_texture);
                rdr_gate.render(&self.render_state, |mut tess_gate| {
                    tess_gate.render(&self.quad);
                })
            }
        });

        let normal_texture = pipeline.bind_texture(&offscreen.color_slot().1);
        let depth_texture = pipeline.bind_texture(&offscreen.depth_slot());
        shd_gate.shade(&shaders.directional_program, |iface, mut rdr_gate| {
            for (_, light) in world.query::<&DirectionalLight>().iter() {
                iface.color.update(light.color.to_normalized());
                iface.intensity.update(light.intensity);
                iface.diffuse.update(&diffuse_texture);
                iface.depth.update(&depth_texture);
                iface.normal.update(&normal_texture);
                iface.direction.update(light.direction.into());
                rdr_gate.render(&self.render_state, |mut tess_gate| {
                    tess_gate.render(&self.quad);
                })
            }
        });
    }
}
