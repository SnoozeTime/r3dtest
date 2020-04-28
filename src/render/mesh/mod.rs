//! Mesh are read from a GLTF file. A mesh can be made of multiple primitives.
//! Each primitive will have a Tess.
use luminance::linear::M44;
use luminance::pipeline::BoundTexture;
use luminance::pixel::NormUnsigned;
use luminance::shader::program::{Program, Uniform};
use luminance::texture::Dim2;
use luminance_derive::{Semantics, UniformInterface, Vertex};

pub mod deferred;
pub mod material;
pub mod mesh;
pub mod primitive;
pub mod scene;
mod shaders;
pub mod texture;

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

    #[sem(name = "Color", repr = "[f32; 4]", wrapper = "VertexColor")]
    Color,

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
    pub color: VertexColor,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: VertexPosition::new([0.0, 0.0, 0.0]),
            normal: VertexNormal::new([0.0, 0.0, 0.0]),
            tangent: VertexTangent::new([0.0, 0.0, 0.0, 0.0]),
            tex_coord_0: VertexTexCoord0::new([0.0, 0.0]),
            tex_coord_1: VertexTexCoord1::new([0.0, 0.0]),
            color: VertexColor::new([1.0, 1.0, 1.0, 1.0]),
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

#[derive(UniformInterface)]
pub struct PbrShaderInterface {
    // matrix for the position
    #[uniform(unbound)]
    pub projection: Uniform<M44>,
    #[uniform(unbound)]
    pub view: Uniform<M44>,
    #[uniform(unbound)]
    pub model: Uniform<M44>,

    #[uniform(name = "u_Camera", unbound)]
    pub u_camera: Uniform<[f32; 3]>,

    // material.
    #[uniform(name = "u_BaseColorFactor", unbound)]
    pub u_base_color_factor: Uniform<[f32; 3]>,
    #[uniform(name = "u_MetallicRoughnessValues", unbound)]
    pub u_metallic_roughness_values: Uniform<[f32; 2]>,
    #[uniform(name = "u_AlphaBlend", unbound)]
    pub u_alpha_blend: Uniform<f32>,
    #[uniform(name = "u_AlphaCutoff", unbound)]
    pub u_alpha_cutoff: Uniform<f32>,
    // optional.
    #[uniform(name = "u_BaseColorSampler", unbound)]
    pub u_base_color_sampler: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(name = "u_BaseColorTexCoord", unbound)]
    pub u_base_color_tex_coord: Uniform<u32>,

    // optional.
    #[uniform(name = "u_NormalSampler", unbound)]
    pub u_normal_sampler: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(name = "u_NormalTexCoord", unbound)]
    pub u_normal_tex_coord: Uniform<u32>,
    #[uniform(name = "u_NormalScale", unbound)]
    pub u_normal_scale: Uniform<f32>,

    // optional.
    #[uniform(name = "u_MetallicRoughnessSampler", unbound)]
    pub u_metallic_roughness_sampler: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    #[uniform(name = "u_MetallicRoughnessTexCoord", unbound)]
    pub u_metallic_roughness_tex_coord: Uniform<u32>,

    // light sources.
    #[uniform(name = "u_LightDirection", unbound)]
    pub u_light_direction: Uniform<[f32; 3]>,
    #[uniform(name = "u_LightColor", unbound)]
    pub u_light_color: Uniform<[f32; 3]>,
    #[uniform(name = "u_AmbientLightColor", unbound)]
    pub u_ambient_light_color: Uniform<[f32; 3]>,
    #[uniform(name = "u_AmbientLightIntensity", unbound)]
    pub u_ambient_light_intensity: Uniform<f32>,
}
