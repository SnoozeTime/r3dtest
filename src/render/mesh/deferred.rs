use crate::camera::Camera;
use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::render::lighting::PointLight;
use crate::render::mesh::scene::Scene;
use crate::render::shaders::Shaders;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::framebuffer::Framebuffer;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, PipelineState, ShadingGate};
use luminance::pixel::{
    Depth32F, Floating, Integral, NormRGBA8UI, NormUnsigned, Unsigned, RGBA16I, RGBA32F, RGBA8UI,
};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, ProgramInterface, Uniform};
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, Sampler};
use luminance_derive::UniformInterface;
use luminance_glfw::GlfwSurface;
use luminance_windowing::Surface;

// G-Buffer
pub type OffscreenBuffer =
    Framebuffer<Dim2, (NormRGBA8UI, NormRGBA8UI, RGBA32F, RGBA32F), Depth32F>;
// light contributions. Values will exceed 1.0 so then we can do HDR, use it for glow and so one. Happy days.
pub type LightBuffer = Framebuffer<Dim2, RGBA32F, ()>;

pub struct DeferredRenderer {
    offscreen_buffer: OffscreenBuffer,
    light_buffer: LightBuffer,
    scene: Scene,
    quad: Tess,
    current_blending_mode: usize,
}

const ALL_BLENDING_MODE: [(Equation, Factor, Factor); 2] = [
    (Equation::Additive, Factor::SrcAlpha, Factor::One),
    (Equation::Additive, Factor::One, Factor::One),
];

/// Shader for the first stage.
#[derive(UniformInterface)]
pub struct PbrShaderInterface {
    // matrix for the position
    #[uniform(unbound)]
    pub projection: Uniform<M44>,
    #[uniform(unbound)]
    pub view: Uniform<M44>,
    #[uniform(unbound)]
    pub model: Uniform<M44>,

    // material.
    #[uniform(unbound)]
    pub albedo: Uniform<[f32; 3]>,

    // optional.
    #[uniform(unbound)]
    pub color_texture: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(unbound)]
    pub color_texture_coord_set: Uniform<u32>,

    // optional.
    #[uniform(unbound)]
    pub normal_texture: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(unbound)]
    pub normal_texture_coord_set: Uniform<u32>,
    #[uniform(unbound)]
    pub normal_scale: Uniform<f32>,

    // optional.
    #[uniform(unbound)]
    pub roughness_metallic_texture: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(unbound)]
    pub roughness_metallic_texture_coord_set: Uniform<u32>,

    #[uniform(unbound)]
    pub metallic: Uniform<f32>,
    #[uniform(unbound)]
    pub roughness: Uniform<f32>,
    #[uniform(unbound)]
    pub ao: Uniform<f32>,
}
pub type PbrOffscreenProgram = Program<super::VertexSemantics, (), PbrShaderInterface>;

/// Shader for the second stage.
#[derive(UniformInterface)]
pub struct PbrLightingOffscreenInterface {
    #[uniform(unbound)]
    pub albedo_map: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(unbound)]
    pub material_map: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(unbound)]
    pub position: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,
    #[uniform(unbound)]
    pub normal: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,

    #[uniform(unbound)]
    light_position: Uniform<[f32; 3]>,
    #[uniform(unbound)]
    light_color: Uniform<[f32; 3]>,
    #[uniform(unbound)]
    camera_position: Uniform<[f32; 3]>,
}
pub type PbrLightingProgram = Program<(), (), PbrLightingOffscreenInterface>;

/// Shader for the third stage.
#[derive(UniformInterface)]
pub struct PbrLightingInterface {
    #[uniform(unbound)]
    pub lighting_map: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,
    #[uniform(unbound)]
    pub albedo_map: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
}
pub type PbrToScreenProgram = Program<(), (), PbrLightingInterface>;

impl DeferredRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let backbuffer = surface.back_buffer().unwrap();
        // offscreen buffer that we will render in the first place
        let (w, h) = (backbuffer.width(), backbuffer.height());
        let offscreen_buffer =
            OffscreenBuffer::new(surface, [w as u32, h as u32], 0, Sampler::default())
                .expect("framebuffer creation");
        let light_buffer = LightBuffer::new(surface, [w as u32, h as u32], 0, Sampler::default())
            .expect("framebuffer creation");

        let asset_path = std::env::var("ASSET_PATH").unwrap() + "material.gltf";
        println!("Will import from {}", asset_path);
        let import = gltf::import(asset_path).unwrap();
        let g_scene = import.0.scenes().next().unwrap();
        let mut scene = Scene::from_gltf(surface, &g_scene, &import);
        scene.add_fake_material(surface);
        let quad = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        Self {
            offscreen_buffer,
            light_buffer,
            scene,
            quad,
            current_blending_mode: 0,
        }
    }

    pub fn next_blending_mode(&mut self) {
        self.current_blending_mode = (self.current_blending_mode + 1) % ALL_BLENDING_MODE.len();
    }

    pub fn render_offscreen(
        &mut self,
        surface: &mut GlfwSurface,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        shaders: &Shaders,
    ) {
        //self.render_scene_to_offscreen(surface, projection, view, shaders);
        //self.render_lighting(surface, world, shaders);
    }
    //    /// First, will render all the scene to a offscreen buffer that stores the important things :)
    //    fn render_scene_to_offscreen(
    //        &mut self,
    //        surface: &mut GlfwSurface,
    //        projection: &glam::Mat4,
    //        view: &glam::Mat4,
    //        shaders: &Shaders,
    //    ) {
    //        surface.pipeline_builder().pipeline(
    //            &self.offscreen_buffer,
    //            &PipelineState::default(),
    //            |pipeline, mut shd_gate| {
    //                // TODO Sort by material.
    //                self.scene
    //                    .render(&pipeline, &mut shd_gate, projection, view);
    //            },
    //        );
    //    }
    //
    //    /// Then, add all light contributions to a light buffer.
    //    fn render_lighting(&self, surface: &mut GlfwSurface, world: &hecs::World, shaders: &Shaders) {
    //        let cam_position = world
    //            .query::<(&Transform, &MainPlayer)>()
    //            .iter()
    //            .next()
    //            .map(|(_, (t, _))| t.translation);
    //
    //        if let Some(cam_position) = cam_position {
    //            surface.pipeline_builder().pipeline(
    //                &self.light_buffer,
    //                &PipelineState::default().set_clear_color([0.0, 0.0, 0.0, 0.0]),
    //                |pipeline, mut shd_gate| {
    //                    for (_, (transform, light)) in world.query::<(&Transform, &PointLight)>().iter()
    //                    {
    //                        //
    //                        let position = pipeline.bind_texture(&self.offscreen_buffer.color_slot().3);
    //                        let normal = pipeline.bind_texture(&self.offscreen_buffer.color_slot().2);
    //                        let albedo_map =
    //                            pipeline.bind_texture(&self.offscreen_buffer.color_slot().0);
    //                        let material_map =
    //                            pipeline.bind_texture(&self.offscreen_buffer.color_slot().1);
    //
    //                        shd_gate.shade(
    //                            &shaders.pbr_light_offscreen_program,
    //                            |iface, mut rdr_gate| {
    //                                iface.albedo_map.update(&albedo_map);
    //                                iface.material_map.update(&material_map);
    //                                iface.position.update(&position);
    //                                iface.normal.update(&normal);
    //                                iface.camera_position.update(cam_position.into());
    //                                iface.light_position.update(transform.translation.into());
    //                                iface.light_color.update(light.color.to_normalized());
    //
    //                                rdr_gate.render(
    //                                    &RenderState::default()
    //                                        .set_blending(ALL_BLENDING_MODE[self.current_blending_mode])
    //                                        .set_depth_test(None),
    //                                    |mut tess_gate| {
    //                                        tess_gate.render(&self.quad);
    //                                    },
    //                                )
    //                            },
    //                        )
    //                    }
    //                },
    //            );
    //        }
    //    }

    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
    ) where
        S: GraphicsContext,
    {
        let cam_position = world
            .query::<(&Transform, &MainPlayer)>()
            .iter()
            .next()
            .map(|(_, (t, _))| t.translation);

        if let Some(cam_position) = cam_position {
            self.scene
                .render(pipeline, shd_gate, projection, view, world, cam_position);
        }
        //        let lighting_map = pipeline.bind_texture(&self.light_buffer.color_slot());
        //        let albedo_map = pipeline.bind_texture(&self.offscreen_buffer.color_slot().0);
        //        shd_gate.shade(&shaders.pbr_light_program, |iface, mut rdr_gate| {
        //            iface.lighting_map.update(&lighting_map);
        //            iface.albedo_map.update(&albedo_map);
        //            rdr_gate.render(&RenderState::default(), |mut tess_gate| {
        //                tess_gate.render(&self.quad);
        //            })
        //        })
    }
}
