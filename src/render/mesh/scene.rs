//! A scene is made of nodes, which are made of meshes..
//! Nodes have their own transform but they can also have children nodes.

use crate::ecs::Transform;
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::render::mesh::material::Material;
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::shaders::PbrShaders;
use crate::render::mesh::ImportData;
use crate::render::mesh::PbrShaderInterface;
use crate::render::Render;
use luminance::context::GraphicsContext;
use luminance::pipeline::{Pipeline, ShadingGate};
use luminance::render_state::RenderState;
use luminance::shader::program::ProgramInterface;
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;

pub type MeshId = String;
pub type MaterialId = Option<usize>; // None is the default material.

pub struct Scene {
    pub nodes: Vec<Node>,
    pub assets: Assets,
}

/// All the assets for the scene :)
/// meshes, shaders, materials.
pub struct Assets {
    pub shaders: PbrShaders,
    pub materials: HashMap<MaterialId, Material>,
    pub meshes: HashMap<MeshId, Mesh>,
}

impl Default for Assets {
    fn default() -> Self {
        let mut materials = HashMap::new();
        materials.insert(None, Material::default());
        Self {
            shaders: PbrShaders::new(),
            materials,
            meshes: HashMap::new(),
        }
    }
}

impl Scene {
    pub fn check_updates(&mut self) {
        self.assets.shaders.update();
    }
    //    pub fn add_fake_material(&mut self, surface: &mut GlfwSurface) {
    //        //        let material = Material::from_textures(
    //        //            surface,
    //        //            [1.0, 1.0, 1.0, 1.0],
    //        //            Some((
    //        //                std::env::var("ASSET_PATH").unwrap()
    //        //                    + "industrial-tile1-bl/industrial-tile1-albedo.png",
    //        //                0,
    //        //            )),
    //        //            Some((
    //        //                std::env::var("ASSET_PATH").unwrap()
    //        //                    + "industrial-tile1-bl/industrial-tile1-normal-ogl.png",
    //        //                0,
    //        //                1.0,
    //        //            )),
    //        //            Some((
    //        //                std::env::var("ASSET_PATH").unwrap()
    //        //                    + "industrial-tile1-bl/roughness_metallic_map.png",
    //        //                0,
    //        //            )),
    //        //            [0.0, 1.0],
    //        //        )
    //        //        .unwrap();
    //        //
    //        //        self.assets.shaders.add_shader(material.shader_flags);
    //        //        self.assets.materials.insert(None, material);
    //    }

    pub fn from_gltf(surface: &mut GlfwSurface, scene: &gltf::Scene, data: &ImportData) -> Self {
        let mut assets = Assets::default();
        let nodes = scene
            .nodes()
            .map(|node| Node::from_gltf(surface, &node, data, &mut assets))
            .collect();
        Self { nodes, assets }
    }

    pub fn render<S>(
        &self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate<S>,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        camera_position: glam::Vec3,
    ) where
        S: GraphicsContext,
    {
        // TODO Need to update the internal graph of the renderer. Need to make an internal graph
        // where things are sorted by material. Then, every frame tag the transforms that are changed
        // as dirty to update the graph.
        for (_, (t, r)) in world.query::<(&Transform, &Render)>().iter() {
            if let Some(mesh) = self.assets.meshes.get(&r.mesh) {
                for primitive in mesh.primitives.iter() {
                    let material = self
                        .assets
                        .materials
                        .get(&primitive.material)
                        .unwrap_or(self.assets.materials.get(&None).unwrap());
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
                        iface.model.update(t.to_model().to_cols_array_2d());
                        iface.u_camera.update(camera_position.into());

                        self.bind_textures(pipeline, &iface, &material);
                        if let Some((_, light)) = world.query::<&DirectionalLight>().iter().next() {
                            iface.u_light_color.update(light.color.to_normalized());
                            iface.u_light_direction.update(light.direction.into());
                        } else {
                            iface.u_light_color.update([1.0, 1.0, 1.0]);
                            iface.u_light_direction.update([0.0, -1.0, 1.0]);
                        }

                        if let Some((_, light)) = world.query::<&AmbientLight>().iter().next() {
                            iface
                                .u_ambient_light_color
                                .update(light.color.to_normalized());
                            iface.u_ambient_light_intensity.update(light.intensity);
                        } else {
                            iface.u_ambient_light_color.update([1.0, 1.0, 1.0]);
                            iface.u_ambient_light_intensity.update(0.3);
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

    /// Need to do a big exhaustive match instead of using if lets here. If using if let, the binding
    /// is overriden in the next if let.
    fn bind_textures(
        &self,
        pipeline: &Pipeline,
        iface: &ProgramInterface<PbrShaderInterface>,
        material: &Material,
    ) {
        match (
            &material.albedo_texture,
            material.color_texture_coord_set,
            &material.normal_texture,
            material.normal_texture_coord_set,
            material.normal_scale,
            &material.roughness_metallic_texture,
            material.roughness_metallic_texture_coord_set,
        ) {
            (Some(color_tex), Some(color_coord), None, None, None, None, None) => {
                let color_tex = pipeline.bind_texture(&color_tex.texture);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(color_coord);
            }
            (
                Some(color_tex),
                Some(color_coord),
                Some(normal_tex),
                Some(normal_coord),
                Some(normal_scale),
                None,
                None,
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex.texture);
                let normal_tex = pipeline.bind_texture(&normal_tex.texture);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(color_coord);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(normal_coord);
                iface.u_normal_scale.update(normal_scale);
            }
            (
                Some(color_tex),
                Some(color_coord),
                Some(normal_tex),
                Some(normal_coord),
                Some(normal_scale),
                Some(rm_tex),
                Some(rm_coord),
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex.texture);
                let normal_tex = pipeline.bind_texture(&normal_tex.texture);
                let rm_tex = pipeline.bind_texture(&rm_tex.texture);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(color_coord);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(normal_coord);
                iface.u_normal_scale.update(normal_scale);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(rm_coord);
            }
            (
                Some(color_tex),
                Some(color_coord),
                None,
                None,
                None,
                Some(rm_tex),
                Some(rm_coord),
            ) => {
                let color_tex = pipeline.bind_texture(&color_tex.texture);
                let rm_tex = pipeline.bind_texture(&rm_tex.texture);
                iface.u_base_color_sampler.update(&color_tex);
                iface.u_base_color_tex_coord.update(color_coord);

                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(rm_coord);
            }
            (None, None, Some(normal_tex), Some(normal_coord), Some(normal_scale), None, None) => {
                let normal_tex = pipeline.bind_texture(&normal_tex.texture);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(normal_coord);
                iface.u_normal_scale.update(normal_scale);
            }
            (
                None,
                None,
                Some(normal_tex),
                Some(normal_coord),
                Some(normal_scale),
                Some(rm_tex),
                Some(rm_coord),
            ) => {
                let normal_tex = pipeline.bind_texture(&normal_tex.texture);
                let rm_tex = pipeline.bind_texture(&rm_tex.texture);
                iface.u_normal_sampler.update(&normal_tex);
                iface.u_normal_tex_coord.update(normal_coord);
                iface.u_normal_scale.update(normal_scale);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(rm_coord);
            }
            (None, None, None, None, None, Some(rm_tex), Some(rm_coord)) => {
                let rm_tex = pipeline.bind_texture(&rm_tex.texture);
                iface.u_metallic_roughness_sampler.update(&rm_tex);
                iface.u_metallic_roughness_tex_coord.update(rm_coord);
            }
            _ => (),
        }
    }
}

pub struct Node {
    pub transform: Transform,
    pub mesh_id: Option<MeshId>,
}

impl Node {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        node: &gltf::Node,
        data: &ImportData,
        assets: &mut Assets,
    ) -> Self {
        let mesh_id = node.mesh().map(|mesh| {
            let mesh_id = mesh
                .name()
                .map(|n| n.to_string())
                .unwrap_or(format!("mesh{}", mesh.index()));

            if !assets.meshes.contains_key(&mesh_id) {
                let mesh = Mesh::from_gltf(surface, mesh, data, assets);
                assets.meshes.insert(mesh_id.clone(), mesh);
            }
            mesh_id
        });
        //
        let (translation, rotation, scale) = node.transform().decomposed();
        let rotation: glam::Quat = rotation.into();
        //
        //        // TODO maybe create components in the ECS instead...
        let transform = Transform {
            translation: translation.into(),
            rotation,
            scale: scale.into(),
            dirty: true,
        };

        Self { transform, mesh_id }
    }
}
