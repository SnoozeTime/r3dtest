use crate::render::shaders::Shaders;
use hecs::World;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::TessSliceIndex;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{GlfwSurface, Surface};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
//impl Deltable for SpriteRender {
//    type Delta = SpriteRender;
//
//    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
//        if self == old {
//            None
//        } else {
//            Some(self.clone())
//        }
//    }
//
//    fn compute_complete(&self) -> Option<Self::Delta> {
//        Some(self.clone())
//    }
//
//    fn apply_delta(&mut self, delta: &Self::Delta) {
//        self.sprite_nb = delta.sprite_nb;
//        self.texture = delta.texture.clone();
//    }
//
//    fn new_component(delta: &Self::Delta) -> Self {
//        delta.clone()
//    }
//}

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
    textures: HashMap<String, (Texture<Dim2, NormRGBA8UI>, Metadata)>,
    w: f32,
    h: f32,
    tess: Tess,
    render_state: RenderState,
}

impl SpriteRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        //        let image = read_image(std::env::var("ASSET_PATH").unwrap() + "crosshair.png").unwrap();
        //        let tex = load_from_disk(surface, image);
        //        let shotgun_image =
        //            read_image(std::env::var("ASSET_PATH").unwrap() + "shotgun.png").unwrap();
        //        let shotgun_tex = load_from_disk(surface, shotgun_image);
        let mut textures = HashMap::new();

        let crosshair = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/crosshair.png",
        );
        let shotgun_tex = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/shotgun.png",
        );

        textures.insert("crosshair".to_string(), crosshair);
        textures.insert("shotgun".to_string(), shotgun_tex);

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
            textures,
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
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        let projection = glam::Mat4::orthographic_rh_gl(0.0, self.w, 0.0, self.h, -1.0, 10.0);

        shd_gate.shade(&shaders.sprite_program, |iface, mut rdr_gate| {
            iface.projection.update(projection.to_cols_array_2d());

            for (_, (pos, sprite)) in world.query::<(&ScreenPosition, &SpriteRender)>().iter() {
                let assets = self.textures.get(&sprite.texture).unwrap();
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

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Option<image::RgbaImage> {
    image::open(path).map(|img| img.flipv().to_rgba()).ok()
}

fn load_texture<P: AsRef<Path>>(
    surface: &mut GlfwSurface,
    path: P,
) -> (Texture<Dim2, NormRGBA8UI>, Metadata) {
    let mut metadata_path = path.as_ref().to_path_buf();

    // first the texture.

    let image = read_image(path).unwrap();
    let tex = load_from_disk(surface, image);

    // then the metadata.
    metadata_path.set_extension("ron");
    println!("Metadata path = {:?}", metadata_path);
    let metadata: Metadata =
        ron::de::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();

    (tex, metadata)
}

fn load_from_disk(surface: &mut GlfwSurface, img: image::RgbaImage) -> Texture<Dim2, NormRGBA8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    // create the luminance texture; the third argument is the number of mipmaps we want (leave it
    // to 0 for now) and the latest is the sampler to use when sampling the texels in the
    // shader (we’ll just use the default one)
    let tex = Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    // the first argument disables mipmap generation (we don’t care so far)
    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
