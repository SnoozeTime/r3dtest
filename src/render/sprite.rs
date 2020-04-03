use crate::render::shaders::Shaders;
use hecs::World;
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::shader::program::Uniform;
use luminance::tess::TessSliceIndex;
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};

use luminance::blending::{Equation, Factor};
use luminance::render_state::RenderState;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance_glfw::{GlfwSurface, Surface};
use std::collections::HashMap;
use std::path::Path;

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

//

#[derive(UniformInterface)]
pub struct ShaderInterface {
    #[uniform(unbound)]
    pub projection: Uniform<M44>,

    #[uniform(unbound)]
    pub model: Uniform<M44>,

    pub tex: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
}

pub struct SpriteRenderer {
    textures: HashMap<String, Texture<Dim2, NormRGBA8UI>>,
    w: f32,
    h: f32,
    tess: Tess,
    render_state: RenderState,
}

impl SpriteRenderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let image = read_image("./assets/crosshair.png").unwrap();
        let tex = load_from_disk(surface, image);
        let mut textures = HashMap::new();
        textures.insert("hi".to_string(), tex);
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
        _world: &World,
        shaders: &Shaders,
    ) where
        S: GraphicsContext,
    {
        let texture = pipeline.bind_texture(self.textures.get("hi").as_ref().unwrap());
        let projection = glam::Mat4::orthographic_rh_gl(0.0, self.w, 0.0, self.h, -1.0, 10.0);
        let model = glam::Mat4::from_scale_rotation_translation(
            glam::vec3(20., 20., 1.0),
            glam::Quat::identity(),
            glam::vec3(self.w / 2.0, self.h / 2.0, -1.),
        );

        shd_gate.shade(&shaders.sprite_program, |iface, mut rdr_gate| {
            iface.projection.update(projection.to_cols_array_2d());
            iface.tex.update(&texture);
            rdr_gate.render(&self.render_state, |mut tess_gate| {
                iface.model.update(model.to_cols_array_2d());
                tess_gate.render(self.tess.slice(..));
            });
        });
    }
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Option<image::RgbaImage> {
    image::open(path).map(|img| img.flipv().to_rgba()).ok()
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
