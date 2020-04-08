//! Sprites that will always face the player. It uses the perspective projection matrix

use crate::camera::Camera;
use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::net::snapshot::Deltable;
use crate::render::assets::SpriteCache;
use crate::render::shaders::Shaders;
use glam::Mat4;
use hecs::World;
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::NormUnsigned;
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::{Mode, Tess, TessBuilder, TessSliceIndex};
use luminance::texture::Dim2;
use luminance_derive::UniformInterface;
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};
//
//#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
//pub enum VertexSementics {
//    /// useless. Not empty
//}
//
//#[allow(dead_code)]
//#[derive(Vertex, Debug)]
//#[vertex(sem = "VertexSementics")]
//pub struct Vertex {}

#[derive(UniformInterface)]
pub struct ShaderInterface {
    pub projection: Uniform<M44>,
    #[uniform(unbound)]
    pub view: Uniform<M44>,
    pub model: Uniform<M44>,

    pub spritesheet_dimensions: Uniform<[f32; 2]>,
    pub sprite_coord: Uniform<[f32; 4]>,

    pub camera_position: Uniform<[f32; 3]>,
    pub center: Uniform<[f32; 3]>,

    pub tex: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
}

/// like a sprite but not in the 2d space.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Default)]
pub struct Billboard {
    pub texture: String,
    pub enabled: bool,
    pub sprite_nb: usize,
}

impl Deltable for Billboard {
    type Delta = Billboard;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self == old {
            None
        } else {
            Some(self.clone())
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(self.clone())
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.texture = delta.texture.clone();
        self.sprite_nb = delta.sprite_nb;
        self.enabled = delta.enabled;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        delta.clone()
    }
}

pub struct BillboardRenderer {
    tess: Tess,
}
impl BillboardRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let tess = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        Self { tess }
    }

    pub fn render<S>(
        &self,
        projection: &Mat4,
        view: &Mat4,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        world: &World,
        sprite_cache: &SpriteCache,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        let camera_pos = {
            world
                .query::<(&Camera, &Transform)>()
                .iter()
                .filter_map(
                    |(_, (c, t))| {
                        if c.active {
                            Some(t.translation)
                        } else {
                            None
                        }
                    },
                )
                .next()
        };

        if let Some(camera_position) = camera_pos {
            shd_gate.shade(&shaders.billboard_program, |iface, mut rdr_gate| {
                iface.projection.update(projection.to_cols_array_2d());
                iface.view.update(view.to_cols_array_2d());
                iface.camera_position.update(camera_position.into());
                for (e, (transform, billboard)) in world.query::<(&Transform, &Billboard)>().iter()
                {
                    if world.get::<MainPlayer>(e).is_ok() {
                        continue;
                    }

                    if !billboard.enabled {
                        continue;
                    }

                    let assets = sprite_cache.get(&billboard.texture).unwrap();
                    let texture = pipeline.bind_texture(&assets.0);
                    let metadata = &assets.1;

                    let sprite_idx = if billboard.sprite_nb >= metadata.sprites.len() {
                        0
                    } else {
                        billboard.sprite_nb
                    };
                    iface.tex.update(&texture);
                    iface
                        .sprite_coord
                        .update(assets.1.sprites.get(sprite_idx).unwrap().as_array());
                    iface.spritesheet_dimensions.update(assets.1.dim_as_array());

                    let model = transform.to_model();
                    iface.center.update(transform.translation.into());
                    iface.model.update(model.to_cols_array_2d());
                    rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                        tess_gate.render(self.tess.slice(..));
                    });
                }
            });
        }
    }
}
