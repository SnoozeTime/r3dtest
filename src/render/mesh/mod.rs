//! Mesh are read from a GLTF file. A mesh can be made of multiple primitives.
//! Each primitive will have a Tess.
use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::primitive::Primitive;
use crate::render::mesh::scene::Scene;
use crate::render::shaders::Shaders;
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::ShadingGate;
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::Tess;
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::GlfwSurface;

pub mod deferred;
mod material;
mod mesh;
mod primitive;
mod scene;
mod shaders;
mod texture;

type ImportData = (
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 3]", wrapper = "VertexPosition")]
    Position,

    #[sem(name = "normal", repr = "[f32; 3]", wrapper = "VertexNormal")]
    Normal,

    #[sem(name = "tangent", repr = "[f32; 4]", wrapper = "VertexTangent")]
    Tangent,

    #[sem(name = "tex_coord_0", repr = "[f32; 2]", wrapper = "VertexTexCoord0")]
    TextCoord0,

    #[sem(name = "tex_coord_1", repr = "[f32; 2]", wrapper = "VertexTexCoord1")]
    TextCoord1,
}
#[allow(dead_code)]
#[derive(Vertex, Debug)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    pub position: VertexPosition,
    pub normal: VertexNormal,
    pub tangent: VertexTangent,
    pub tex_coord_0: VertexTexCoord0,
    pub tex_coord_1: VertexTexCoord1,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: VertexPosition::new([0.0, 0.0, 0.0]),
            normal: VertexNormal::new([0.0, 0.0, 0.0]),
            tangent: VertexTangent::new([0.0, 0.0, 0.0, 0.0]),
            tex_coord_0: VertexTexCoord0::new([0.0, 0.0]),
            tex_coord_1: VertexTexCoord1::new([0.0, 0.0]),
        }
    }
}

pub type DeferredSceneProgram = Program<VertexSemantics, (), PbrShaderInterface>;

#[derive(Debug, UniformInterface)]
pub struct ShaderInterface {
    #[uniform(unbound)]
    pub projection: Uniform<M44>,

    #[uniform(unbound)]
    pub view: Uniform<M44>,

    #[uniform(unbound)]
    pub model: Uniform<M44>,

    #[uniform(unbound)]
    pub color: Uniform<[f32; 3]>,

    #[uniform(unbound)]
    pub emissive: Uniform<[f32; 3]>,
}

/**
// material
uniform vec3  albedo;
uniform float metallic;
uniform float roughness;
uniform float ao;
// direct lights
uniform vec3 lightPositions[4];
uniform vec3 lightColors[4];
**/
#[derive(Debug, UniformInterface)]
pub struct PbrShaderInterface {
    // matrix for the position
    #[uniform(unbound)]
    pub projection: Uniform<M44>,
    #[uniform(unbound)]
    pub view: Uniform<M44>,
    #[uniform(unbound)]
    pub model: Uniform<M44>,

    #[uniform(unbound)]
    pub camera_position: Uniform<[f32; 3]>,

    // material.
    #[uniform(unbound)]
    pub albedo: Uniform<[f32; 3]>,
    #[uniform(unbound)]
    pub metallic: Uniform<f32>,
    #[uniform(unbound)]
    pub roughness: Uniform<f32>,
    #[uniform(unbound)]
    pub ao: Uniform<f32>,

    // light sources.
    #[uniform(unbound)]
    pub light_positions: Uniform<[f32; 3]>,
    #[uniform(unbound)]
    pub light_colors: Uniform<[f32; 3]>,
}

pub struct GltfSceneRenderer {
    scene: Scene,
}

impl GltfSceneRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let import = gltf::import("material.gltf").unwrap();
        let g_scene = import.0.scenes().next().unwrap();
        let scene = Scene::from_gltf(surface, &g_scene, &import);

        Self { scene }
    }

    pub fn render<S>(
        &self,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        shd_gate: &mut ShadingGate<S>,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        if let Some((_, (t, _))) = world.query::<(&Transform, &MainPlayer)>().iter().next() {
            shd_gate.shade(&shaders.scene_program, |iface, mut rdr_gate| {
                iface.view.update(view.to_cols_array_2d());
                iface.projection.update(projection.to_cols_array_2d());
                iface.camera_position.update(t.translation.into());
                let model = glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::one(),
                    glam::Quat::identity(),
                    glam::Vec3::new(0.0, 4.0, 0.0),
                );

                rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                    // self.scene.render(&iface, &mut tess_gate);
                });
            });
        }
    }
}
