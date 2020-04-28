use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::render::mesh::scene::Scene;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{Pipeline, ShadingGate};
use luminance_glfw::GlfwSurface;

pub struct DeferredRenderer {
    scene: Scene,
    current_blending_mode: usize,
}

const ALL_BLENDING_MODE: [(Equation, Factor, Factor); 2] = [
    (Equation::Additive, Factor::SrcAlpha, Factor::One),
    (Equation::Additive, Factor::One, Factor::One),
];

impl DeferredRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let asset_path = std::env::var("ASSET_PATH").unwrap() + "material.gltf";
        let import = gltf::import(asset_path).unwrap();
        let g_scene = import.0.scenes().next().unwrap();
        let scene = Scene::from_gltf(surface, &g_scene, &import);
        Self {
            scene,
            current_blending_mode: 0,
        }
    }

    pub fn next_blending_mode(&mut self) {
        self.current_blending_mode = (self.current_blending_mode + 1) % ALL_BLENDING_MODE.len();
    }

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
    }
}
