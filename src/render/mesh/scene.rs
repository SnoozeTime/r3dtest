//! A scene is made of nodes, which are made of meshes..
//! Nodes have their own transform but they can also have children nodes.

use super::shaders::ShaderFlags;
use crate::ecs::Transform;
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::render::mesh::deferred::PbrShaderInterface;
use crate::render::mesh::material::Material;
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::shaders::PbrShaders;
use crate::render::mesh::texture::Texture;
use crate::render::mesh::{ImportData, ShaderInterface};
use hecs::World;
use luminance::context::GraphicsContext;
use luminance::pipeline::{Pipeline, ShadingGate, TessGate};
use luminance::pixel::{NormRGB8UI, NormRGBA8UI};
use luminance::render_state::RenderState;
use luminance::shader::program::ProgramInterface;
use luminance::texture::{Dim2, GenMipmaps, MagFilter, MinFilter, Sampler, Wrap};
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;
use std::path::Path;

pub type MeshId = usize;
pub type MaterialId = Option<usize>; // None is the default material.

pub struct Scene {
    nodes: Vec<Node>,
    assets: Assets,
}

/// All the assets for the scene :)
/// meshes, shaders, materials.
#[derive(Default)]
pub struct Assets {
    pub shaders: PbrShaders,
    pub materials: HashMap<MaterialId, Material>,
    pub meshes: HashMap<MeshId, Mesh>,
}

impl Scene {
    pub fn add_fake_material(&mut self, surface: &mut GlfwSurface) {
        let texture_img = read_image(
            std::env::var("ASSET_PATH").unwrap() + "rustediron1-alt2-bl/rustediron2_normal.png",
        )
        .unwrap();
        let texture = load_from_disk(surface, texture_img);
        let normal_map = Texture { texture };

        let texture_img = read_image(
            std::env::var("ASSET_PATH").unwrap() + "rustediron1-alt2-bl/rustediron2_roughness.png",
        )
        .unwrap();
        let roughness_texture = load_from_disk(surface, texture_img);
        let roughness_map = Texture {
            texture: roughness_texture,
        };
        let flags = ShaderFlags::HAS_NORMAL_TEXTURE
            | ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP
            | ShaderFlags::HAS_COLOR_TEXTURE;

        let texture_img = read_image(
            std::env::var("ASSET_PATH").unwrap() + "rustediron1-alt2-bl/rustediron2_basecolor.png",
        )
        .unwrap();
        let basecolor_texture = load_from_disk(surface, texture_img);
        let base_color_map = Texture {
            texture: basecolor_texture,
        };

        let texture_img = read_image(
            std::env::var("ASSET_PATH").unwrap() + "rustediron1-alt2-bl/rustediron2_basecolor.png",
        )
        .unwrap();
        let metallic_texture = load_from_disk(surface, texture_img);
        let metallic_map = Texture {
            texture: metallic_texture,
        };

        let material = Material {
            base_color: [1.0, 1.0, 1.0, 1.0],
            albedo_texture: Some(base_color_map),
            color_texture_coord_set: Some(0),
            normal_scale: Some(1.0),
            normal_texture_coord_set: Some(0),
            normal_texture: Some(normal_map),
            roughness_metallic_texture_coord_set: Some(0),
            roughness_metallic_texture: Some(roughness_map),
            metallic_roughness_values: [0.0, 1.0],
            metallic_texture: Some(metallic_map),
            metallic_texture_coord_set: Some(0),
            ao: 1.0,
            alpha_cutoff: 0.0,
            alpha_mode: gltf::material::AlphaMode::Opaque,
            shader_flags: flags,
        };

        self.assets.shaders.add_shader(flags);
        self.assets.materials.insert(None, material);
    }

    pub fn from_gltf(surface: &mut GlfwSurface, scene: &gltf::Scene, data: &ImportData) -> Self {
        let mut assets = Assets::default();
        let nodes = scene
            .nodes()
            .map(|node| Node::from_gltf(surface, &node, data, &mut assets))
            .collect();
        let mut scene = Self { nodes, assets };
        scene.add_fake_material(surface);
        scene
    }

    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        camera_position: glam::Vec3,
        // iface: &ProgramInterface<PbrShaderInterface>,
        // tess_gate: &mut TessGate<S>,
    ) where
        S: GraphicsContext,
    {
        // FIXME that's really inefficient. Keep order by material instead. Material -> Mesh
        for node in &self.nodes {
            if let Some(mesh_id) = node.mesh_id {
                if let Some(mesh) = self.assets.meshes.get(&mesh_id) {
                    for primitive in mesh.primitives.iter() {
                        let material = self.assets.materials.get(&None).unwrap();
                        //let material = self.assets.materials.get(&primitive.material).unwrap();
                        let shader = self
                            .assets
                            .shaders
                            .shaders
                            .get(&material.shader_flags)
                            .unwrap();

                        shd_gate.shade(&shader, |iface, mut rdr_gate| {
                            iface.view.update(view.to_cols_array_2d());
                            iface.projection.update(projection.to_cols_array_2d());
                            iface
                                .model
                                .update(node.transform.to_model().to_cols_array_2d());
                            iface.u_Camera.update(camera_position.into());
                            // The texture if needed.
                            if let (
                                Some(color_texture),
                                Some(color_coord),
                                Some(normal_texture),
                                Some(normal_coord),
                                Some(scale),
                                Some(roughness_map),
                                Some(roughness_coord),
                                Some(metallic_map),
                                Some(metallic_coord),
                            ) = (
                                material.albedo_texture.as_ref(),
                                material.color_texture_coord_set,
                                material.normal_texture.as_ref(),
                                material.normal_texture_coord_set,
                                material.normal_scale,
                                material.roughness_metallic_texture.as_ref(),
                                material.roughness_metallic_texture_coord_set,
                                material.metallic_texture.as_ref(),
                                material.metallic_texture_coord_set,
                            ) {
                                println!("HERE");
                                let color_texture = pipeline.bind_texture(&color_texture.texture);
                                iface.u_BaseColorSampler.update(&color_texture);
                                iface.u_BaseColorTexCoord.update(color_coord);
                                let normal_texture = pipeline.bind_texture(&normal_texture.texture);
                                iface.u_NormalSampler.update(&normal_texture);
                                iface.u_NormalTexCoord.update(normal_coord);
                                iface.u_NormalScale.update(scale);
                                let roughness_texture =
                                    pipeline.bind_texture(&roughness_map.texture);
                                iface.u_MetallicRoughnessSampler.update(&roughness_texture);
                                iface.u_MetallicRoughnessTexCoord.update(roughness_coord);

                                let metallic_texture = pipeline.bind_texture(&metallic_map.texture);
                                iface.u_MetallicSampler.update(&metallic_texture);
                                iface.u_MetallicTexCoord.update(metallic_coord);
                            }
                            if let (
                                None,
                                None,
                                Some(normal_texture),
                                Some(normal_coord),
                                Some(scale),
                                Some(roughness_map),
                                Some(roughness_coord),
                            ) = (
                                material.albedo_texture.as_ref(),
                                material.color_texture_coord_set,
                                material.normal_texture.as_ref(),
                                material.normal_texture_coord_set,
                                material.normal_scale,
                                material.roughness_metallic_texture.as_ref(),
                                material.roughness_metallic_texture_coord_set,
                            ) {
                                let normal_texture = pipeline.bind_texture(&normal_texture.texture);
                                iface.u_NormalSampler.update(&normal_texture);
                                iface.u_NormalTexCoord.update(normal_coord);
                                iface.u_NormalScale.update(scale);
                                let roughness_texture =
                                    pipeline.bind_texture(&roughness_map.texture);
                                iface.u_MetallicRoughnessSampler.update(&roughness_texture);
                                iface.u_MetallicRoughnessTexCoord.update(roughness_coord);
                            }

                            //                            if let (Some(normal_texture), Some(normal_coord), Some(scale)) = (
                            //                                material.normal_texture.as_ref(),
                            //                                material.normal_texture_coord_set,
                            //                                material.normal_scale,
                            //                            ) {
                            //                                let normal_texture = pipeline.bind_texture(&normal_texture.texture);
                            //                                iface.normal_texture.update(&normal_texture);
                            //                                iface.normal_texture_coord_set.update(normal_coord);
                            //                                iface.normal_scale.update(scale);
                            //                            }

                            if let Some((_, light)) =
                                world.query::<&DirectionalLight>().iter().next()
                            {
                                iface.u_LightColor.update(light.color.to_normalized());
                                iface.u_LightDirection.update(light.direction.into());
                            } else {
                                iface.u_LightColor.update([1.0, 1.0, 1.0]);
                                iface.u_LightDirection.update([0.0, -1.0, 1.0]);
                            }

                            if let Some((_, light)) = world.query::<&AmbientLight>().iter().next() {
                                iface
                                    .u_AmbientLightColor
                                    .update(light.color.to_normalized());
                                iface.u_AmbientLightIntensity.update(light.intensity);
                            } else {
                                iface.u_AmbientLightColor.update([1.0, 1.0, 1.0]);
                                iface.u_AmbientLightIntensity.update(0.3);
                            }
                            material.apply_uniforms(&iface);

                            rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                                tess_gate.render(&primitive.tess);
                            });
                        });
                    }
                }
            }
        }
    }
}

pub struct Node {
    transform: Transform,
    mesh_id: Option<MeshId>,
}

impl Node {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        node: &gltf::Node,
        data: &ImportData,
        assets: &mut Assets,
    ) -> Self {
        let mesh_id = node.mesh().map(|mesh| {
            let mesh_index = mesh.index();

            if !assets.meshes.contains_key(&mesh_index) {
                let mesh = Mesh::from_gltf(surface, mesh, data, assets);
                assets.meshes.insert(mesh_index, mesh);
            }

            mesh_index
        });

        let (translation, rotation, scale) = node.transform().decomposed();
        let rotation: glam::Quat = rotation.into();

        // TODO maybe create components in the ECS instead...
        let transform = Transform {
            translation: translation.into(),
            rotation,
            scale: scale.into(),
        };

        Self { transform, mesh_id }
    }

    //    pub fn render<S>(
    //        &self,
    //        iface: &ProgramInterface<PbrShaderInterface>,
    //        tess_gate: &mut TessGate<S>,
    //        assets: &Assets,
    //    ) where
    //        S: GraphicsContext,
    //    {
    //        if let Some(mesh_id) = self.mesh_id {
    //            if let Some(mesh) = assets.meshes.get(&mesh_id) {
    //                iface
    //                    .model
    //                    .update(self.transform.to_model().to_cols_array_2d());
    //                mesh.render(iface, tess_gate, assets);
    //            }
    //        }
    //    }
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Option<image::RgbImage> {
    image::open(path).map(|img| img.flipv().to_rgb()).ok()
}

fn load_from_disk(
    surface: &mut GlfwSurface,
    img: image::RgbImage,
) -> luminance::texture::Texture<Dim2, NormRGB8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    /**
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    **/
    // create the luminance texture; the third argument is the number of mipmaps we want (leave it
    // to 0 for now) and the latest is the sampler to use when sampling the texels in the
    // shader (we’ll just use the default one)
    let mut sampler = Sampler::default();
    sampler.mag_filter = MagFilter::Linear;
    sampler.min_filter = MinFilter::LinearMipmapLinear;
    sampler.wrap_t = Wrap::Repeat;
    sampler.wrap_s = Wrap::Repeat;

    let tex = luminance::texture::Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    // the first argument disables mipmap generation (we don’t care so far)
    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
