//! Debug rendering - for example for AABB colliders...

use crate::physics::{BodyType, PhysicWorld, RigidBody};
use crate::render::shaders::Shaders;
use glam::Mat4;
use hecs::World;
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{Pipeline, ShadingGate};
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::{Mode, Tess, TessBuilder, TessSliceIndex};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::GlfwSurface;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 3]", wrapper = "VertexPosition")]
    Position,
}
#[allow(dead_code)]
#[derive(Vertex, Debug)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    position: VertexPosition,
}

#[derive(UniformInterface)]
pub struct ShaderInterface {
    #[uniform(unbound)]
    pub projection: Uniform<M44>,

    #[uniform(unbound)]
    pub view: Uniform<M44>,

    #[uniform(unbound)]
    pub model: Uniform<M44>,
}

fn get_cube<S>(surface: &mut S) -> Tess
where
    S: GraphicsContext,
{
    //    let model =
    //        super::assets::Obj::load(std::env::var("ASSET_PATH").unwrap() + "models/cube.obj").unwrap();
    //    TessBuilder::new(surface)
    //        .set_mode(Mode::Triangle)
    //        .add_vertices(model.vertices)
    //        .set_indices(model.indices)
    //        .build()
    //        .unwrap()
    //
    let mut vertices = vec![
        //front
        VertexPosition::new([-1.0, 1.0, 1.0]),
        VertexPosition::new([1.0, 1.0, 1.0]),
        VertexPosition::new([1.0, 1.0, 1.0]),
        VertexPosition::new([1.0, -1.0, 1.0]),
        VertexPosition::new([1.0, -1.0, 1.0]),
        VertexPosition::new([-1.0, -1.0, 1.0]),
        VertexPosition::new([-1.0, -1.0, 1.0]),
        VertexPosition::new([-1.0, 1.0, 1.0]),
        //right
        VertexPosition::new([1.0, 1.0, 1.0]),
        VertexPosition::new([1.0, 1.0, -1.0]),
        VertexPosition::new([1.0, 1., -1.0]),
        VertexPosition::new([1.0, -1.0, -1.0]),
        VertexPosition::new([1.0, -1.0, -1.0]),
        VertexPosition::new([1.0, -1.0, 1.0]),
        //back
        VertexPosition::new([1.0, 1.0, -1.0]),
        VertexPosition::new([-1.0, 1.0, -1.0]),
        VertexPosition::new([-1.0, -1.0, -1.0]),
        VertexPosition::new([1.0, -1., -1.0]),
        VertexPosition::new([-1.0, -1.0, -1.0]),
        VertexPosition::new([-1.0, 1.0, -1.0]),
        //left
        VertexPosition::new([-1.0, 1.0, -1.0]),
        VertexPosition::new([-1.0, 1.0, 1.0]),
        VertexPosition::new([-1.0, -1.0, 1.]),
        VertexPosition::new([-1.0, -1.0, -1.0]),
    ];

    TessBuilder::new(surface)
        .add_vertices(
            vertices
                .drain(..)
                .map(|p| Vertex { position: p })
                .collect::<Vec<Vertex>>(),
        )
        .set_mode(Mode::Line)
        .build()
        .unwrap()
}

pub struct DebugRenderer {
    tess: Tess,
}
impl DebugRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let tess = get_cube(surface);
        Self { tess }
    }

    pub fn render<S>(
        &self,
        projection: &Mat4,
        view: &Mat4,
        shd_gate: &mut ShadingGate<S>,
        world: &World,
        shaders: &Shaders,
        physics: &PhysicWorld,
    ) where
        S: GraphicsContext,
    {
        shd_gate.shade(&shaders.debug_program, |iface, mut rdr_gate| {
            iface.projection.update(projection.to_cols_array_2d());
            iface.view.update(view.to_cols_array_2d());
            for (_, rb) in world.query::<&RigidBody>().iter() {
                if let Some(aabb) = physics.get_aabb(rb.handle.unwrap()) {
                    if let Some(ct) = physics.get_collider_type(rb.handle.unwrap()) {
                        if let BodyType::Static = ct {
                            let model = glam::Mat4::from_scale_rotation_translation(
                                aabb.halfwidths,
                                glam::Quat::identity(),
                                aabb.center,
                            );
                            iface.model.update(model.to_cols_array_2d());
                            rdr_gate.render(
                                &RenderState::default().set_depth_test(None),
                                |mut tess_gate| {
                                    tess_gate.render(self.tess.slice(..));
                                },
                            );
                        }
                    }
                }
            }
        });
    }
}
