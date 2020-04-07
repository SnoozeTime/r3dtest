use crate::render::assets::SpriteCache;
use crate::render::shaders::Shaders;
use hecs::World;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::NormUnsigned;
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::TessSliceIndex;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::Dim2;
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{GlfwSurface, Surface};
use serde_derive::{Deserialize, Serialize};

/// Component to display a sprite on the screen.
///
/// This component and the Transform component are necessary to display a sprite on screen.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq)]
pub struct SpriteRender {
    /// Texture spritesheet to use for the sprite
    pub texture: String,
    /// index of sprite on the sheet.
    pub sprite_nb: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub height: f32,
    pub width: f32,
    pub sprites: Vec<SpriteMetadata>,
}

impl Metadata {
    pub fn dim_as_array(&self) -> [f32; 2] {
        [self.width, self.height]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpriteMetadata {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl SpriteMetadata {
    pub fn as_array(&self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
}

/// Screen position. x and y are between 0 and 1.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct ScreenPosition {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSementics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,

    #[sem(name = "uv", repr = "[f32; 2]", wrapper = "VertexUv")]
    TextureCoord,
}

#[allow(dead_code)]
#[derive(Vertex, Debug)]
#[vertex(sem = "VertexSementics")]
pub struct Vertex {
    position: VertexPosition,
    uv: VertexUv,
}

#[derive(UniformInterface)]
pub struct ShaderInterface {
    #[uniform(unbound)]
    pub projection: Uniform<M44>,

    #[uniform(unbound)]
    pub model: Uniform<M44>,

    pub spritesheet_dimensions: Uniform<[f32; 2]>,
    pub sprite_coord: Uniform<[f32; 4]>,

    pub tex: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
}

pub struct SpriteRenderer {
    w: f32,
    h: f32,
    tess: Tess,
    render_state: RenderState,
}

impl SpriteRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let render_state = RenderState::default().set_blending((
            Equation::Additive,
            Factor::SrcAlpha,
            Factor::Zero,
        ));
        let tess = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        Self {
            tess,
            w: surface.width() as f32,
            h: surface.height() as f32,
            render_state,
        }
    }

    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        world: &World,
        sprite_cache: &SpriteCache,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        let projection = glam::Mat4::orthographic_rh_gl(0.0, self.w, 0.0, self.h, -1.0, 10.0);

        shd_gate.shade(&shaders.sprite_program, |iface, mut rdr_gate| {
            iface.projection.update(projection.to_cols_array_2d());

            for (_, (pos, sprite)) in world.query::<(&ScreenPosition, &SpriteRender)>().iter() {
                let assets = sprite_cache.get(&sprite.texture).unwrap();
                let texture = pipeline.bind_texture(&assets.0);
                let metadata = &assets.1;

                let sprite_idx = if sprite.sprite_nb >= metadata.sprites.len() {
                    0
                } else {
                    sprite.sprite_nb
                };
                iface.tex.update(&texture);
                iface
                    .sprite_coord
                    .update(assets.1.sprites.get(sprite_idx).unwrap().as_array());
                iface.spritesheet_dimensions.update(assets.1.dim_as_array());
                let model = glam::Mat4::from_scale_rotation_translation(
                    glam::vec3(self.w * pos.w, self.h * pos.h, 1.0),
                    glam::Quat::identity(),
                    glam::vec3(self.w * pos.x, self.h * pos.y, -1.),
                );
                iface.model.update(model.to_cols_array_2d());

                rdr_gate.render(&self.render_state, |mut tess_gate| {
                    tess_gate.render(self.tess.slice(..));
                });
            }
        });
    }
}
