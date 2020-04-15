//! Debug rendering - for example for AABB colliders...

use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::net::snapshot::Deltable;
use crate::physics::{PhysicWorld, RigidBody, Shape};
use crate::render::shaders::Shaders;
use glam::{Mat4, Vec3};
use hecs::World;
#[allow(unused_imports)]
use log::{debug, info};
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::ShadingGate;
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::{Mode, Tess, TessBuilder, TessSliceIndex};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DebugRender {
    None,
    Aabb(Vec3),
}

impl Default for DebugRender {
    fn default() -> Self {
        Self::None
    }
}

impl Deltable for DebugRender {
    type Delta = DebugRender;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self == old {
            None
        } else {
            Some(*self)
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(*self)
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        *self = *delta;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        *delta
    }
}

pub fn update_debug_components(world: &mut hecs::World, physics: &PhysicWorld) {
    debug!("Will update debug components");
    let mut to_add = vec![];

    for (e, rb) in world.query::<&RigidBody>().iter() {
        if world.get::<DebugRender>(e).is_err() {
            // add component to entity.
            if let Some(shape) = physics.get_shape(rb.handle.unwrap()) {
                if let Shape::AABB(extends) = shape {
                    to_add.push((e, extends));
                }
            }
        }
    }

    debug!("TO ADD {:?}", to_add);
    for (e, debug_render) in to_add {
        world
            .insert_one(e, DebugRender::Aabb(debug_render))
            .unwrap();
    }
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
    ) where
        S: GraphicsContext,
    {
        shd_gate.shade(&shaders.debug_program, |iface, mut rdr_gate| {
            iface.projection.update(projection.to_cols_array_2d());
            iface.view.update(view.to_cols_array_2d());
            for (e, (t, debug_render)) in world.query::<(&Transform, &DebugRender)>().iter() {
                if world.get::<MainPlayer>(e).is_ok() {
                    continue;
                }
                if let DebugRender::Aabb(aabb) = debug_render {
                    let model = glam::Mat4::from_scale_rotation_translation(
                        *aabb,
                        glam::Quat::identity(),
                        t.translation,
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
        });
    }
}
