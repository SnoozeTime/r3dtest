//! Mesh are read from a GLTF file. A mesh can be made of multiple primitives.
//! Each primitive will have a Tess.
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::NormUnsigned;
use luminance::shader::program::{Program, ProgramInterface, Uniform};
use luminance::texture::Dim2;
use luminance_derive::{Semantics, UniformInterface, Vertex};

pub mod deferred;
pub mod import;
pub mod material;
pub mod mesh;
pub mod primitive;
pub mod scene;
mod shaders;
pub mod texture;
use crate::assets::material::Material;
use crate::assets::mesh::MaterialId;
use crate::assets::{AssetManager, Handle};
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::primitive::Primitive;
use crate::render::mesh::shaders::PbrShaders;
use crate::render::Render;
use crate::resources::Resources;
use luminance::context::GraphicsContext;
use luminance::render_state::RenderState;
use luminance::tess::{Tess, TessSlice};
use luminance_glfw::GlfwSurface;
pub use shaders::ShaderFlags;
use shrev::EventChannel;
use std::collections::HashMap;
use std::rc::Rc;

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
    #[uniform(name = "u_EmissiveFactor", unbound)]
    pub u_emissive_factor: Uniform<[f32; 3]>,
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

pub struct PbrRenderer {
    /// Shader for physically based rendering.
    shaders: PbrShaders,

    default_material_handle: Handle,
}

impl PbrRenderer {
    pub fn new() -> Self {
        Self {
            shaders: PbrShaders::new(),
            default_material_handle: Handle("default_material".to_owned()),
        }
    }
    pub fn render<S>(
        &mut self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        resources: &Resources,
    ) where
        S: GraphicsContext,
    {
        let camera_entity =
            crate::camera::find_main_camera(world).expect("World should have a main camera");
        let camera_position = world.get::<Transform>(camera_entity).unwrap().translation;
        // Do I need to rebuild that everyframe?
        let mut sorted_primitives: HashMap<MaterialId, Vec<(Rc<Tess>, Transform)>> =
            HashMap::with_capacity(10);

        let mut mesh_manager = resources.fetch_mut::<AssetManager<Mesh>>().unwrap();
        for (_, (t, render)) in world.query::<(&Transform, &Render)>().iter() {
            match mesh_manager.get(&Handle(render.mesh.clone())) {
                Some(asset) => asset.execute(|m| {
                    for p in m.primitives.iter() {
                        if sorted_primitives.contains_key(&p.material) {
                            sorted_primitives
                                .get_mut(&p.material)
                                .unwrap()
                                .push((Rc::clone(&p.tess), *t))
                        } else {
                            // TODO maybe don't do that. Keep keys populated and just reset the vec at the end of the frame?
                            sorted_primitives
                                .insert(p.material.clone(), vec![(Rc::clone(&p.tess), *t)]);
                        }
                    }
                }),
                None => {
                    mesh_manager.load(render.mesh.as_str());
                }
            }
        }

        let mut material_manager = resources.fetch_mut::<AssetManager<Material>>().unwrap();
        for (material_id, primitives) in sorted_primitives {
            let material_handle_str = material_id
                .map(|m| m.clone())
                .unwrap_or(self.default_material_handle.0.clone());
            let material_handle = Handle(material_handle_str);
            let material_asset = {
                match material_manager.get(&material_handle) {
                    Some(asset) => {
                        if asset.is_loaded() {
                            asset
                        } else {
                            material_manager.get(&self.default_material_handle).unwrap()
                        }
                    }
                    None => {
                        material_manager.load(material_handle.0.clone().as_str());
                        material_manager.get(&self.default_material_handle).unwrap()
                    }
                }
            };

            material_asset.execute(|material| {
                self.shaders.add_shader(material.shader_flags);
                let shader = self.shaders.shaders.get(&material.shader_flags).unwrap();

                shd_gate.shade(&shader, |iface, mut rdr_gate| {
                    // Now bind all uniforms.
                    iface.view.update(view.to_cols_array_2d());
                    iface.projection.update(projection.to_cols_array_2d());
                    iface.u_camera.update(camera_position.into());
                    if let Some((_, light)) = world.query::<&DirectionalLight>().iter().next() {
                        iface.u_light_color.update(light.color.to_normalized());
                        iface.u_light_direction.update(light.direction.into());
                    } else {
                        iface.u_light_color.update([1.0, 1.0, 1.0]);
                        iface.u_light_direction.update([0.0, -1.0, 1.0]);
                    }
                    iface.u_base_color_factor.update([
                        material.base_color[0],
                        material.base_color[1],
                        material.base_color[2],
                    ]);
                    iface.u_emissive_factor.update(material.emissive_factor);
                    iface.u_alpha_cutoff.update(material.alpha_cutoff);
                    iface
                        .u_metallic_roughness_values
                        .update(material.metallic_roughness_values);
                    self.bind_textures(pipeline, &iface, material);
                    if let Some((_, light)) = world.query::<&AmbientLight>().iter().next() {
                        iface
                            .u_ambient_light_color
                            .update(light.color.to_normalized());
                        iface.u_ambient_light_intensity.update(light.intensity);
                    } else {
                        iface.u_ambient_light_color.update([1.0, 1.0, 1.0]);
                        iface.u_ambient_light_intensity.update(0.3);
                    }
                    for (tess, t) in &primitives {
                        iface.model.update(t.to_model().to_cols_array_2d());
                        rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                            tess_gate.render(&**tess);
                        });
                    }
                });
            });
        }
    }

    /// Need to do a big exhaustive match instead of using if lets here. If using if let, the binding
    /// is overriden in the next if let.
    fn bind_textures(
        &self,
        pipeline: &Pipeline,
        iface: &ProgramInterface<PbrShaderInterface>,
        material: &Material,
    ) {
        match (
            &material.color_texture,
            material.color_texture_data.as_ref(),
            &material.normal_texture,
            material.normal_texture_data.as_ref(),
            &material.roughness_metallic_texture,
            material.roughness_metallic_texture_data.as_ref(),
        ) {
            (Some(color_tex), Some((_, color_coord)), None, None, None, None) => {
                let color_tex = pipeline.bind_texture(&color_tex);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(*color_coord);
            }
            (
                Some(color_tex),
                Some((_, color_coord)),
                Some(normal_tex),
                Some((_, normal_coord, normal_scale)),
                None,
                None,
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex);
                let normal_tex = pipeline.bind_texture(&normal_tex);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(*color_coord);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(*normal_coord);
                iface.u_normal_scale.update(*normal_scale);
            }
            (
                Some(color_tex),
                Some((_, color_coord)),
                Some(normal_tex),
                Some((_, normal_coord, normal_scale)),
                Some(rm_tex),
                Some((_, rm_coord)),
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex);
                let normal_tex = pipeline.bind_texture(&normal_tex);
                let rm_tex = pipeline.bind_texture(&rm_tex);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(*color_coord);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(*normal_coord);
                iface.u_normal_scale.update(*normal_scale);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(*rm_coord);
            }
            (
                Some(color_tex),
                Some((_, color_coord)),
                None,
                None,
                Some(rm_tex),
                Some((_, rm_coord)),
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex);
                let rm_tex = pipeline.bind_texture(&rm_tex);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(*color_coord);

                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(*rm_coord);
            }
            (None, None, Some(normal_tex), Some((_, normal_coord, normal_scale)), None, None) => {
                let normal_tex = pipeline.bind_texture(&normal_tex);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(*normal_coord);
                iface.u_normal_scale.update(*normal_scale);
            }
            (
                None,
                None,
                Some(normal_tex),
                Some((_, normal_coord, normal_scale)),
                Some(rm_tex),
                Some((_, rm_coord)),
            ) => {
                let normal_tex = pipeline.bind_texture(&normal_tex);
                let rm_tex = pipeline.bind_texture(&rm_tex);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(*normal_coord);
                iface.u_normal_scale.update(*normal_scale);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(*rm_coord);
            }
            (None, None, None, None, Some(rm_tex), Some((_, rm_coord))) => {
                let rm_tex = pipeline.bind_texture(&rm_tex);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(*rm_coord);
            }
            _ => (),
        }
    }
}
