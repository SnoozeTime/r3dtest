//! A scene is made of nodes, which are made of meshes..
//! Nodes have their own transform but they can also have children nodes.

use crate::ecs::Transform;
use crate::render::mesh::deferred::PbrShaderInterface;
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::{ImportData, ShaderInterface};
use luminance::context::GraphicsContext;
use luminance::pipeline::TessGate;
use luminance::shader::program::ProgramInterface;
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;

type MeshId = usize;

pub struct Scene {
    nodes: Vec<Node>,
    meshes: HashMap<MeshId, Mesh>,
}

impl Scene {
    pub fn from_gltf(surface: &mut GlfwSurface, scene: &gltf::Scene, data: &ImportData) -> Self {
        let mut meshes = HashMap::new();

        let nodes = scene
            .nodes()
            .map(|node| Node::from_gltf(surface, &node, data, &mut meshes))
            .collect();

        Self { nodes, meshes }
    }

    pub fn render<S>(
        &self,
        iface: &ProgramInterface<PbrShaderInterface>,
        tess_gate: &mut TessGate<S>,
    ) where
        S: GraphicsContext,
    {
        for node in &self.nodes {
            node.render(iface, tess_gate, &self.meshes);
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
        meshes: &mut HashMap<MeshId, Mesh>,
    ) -> Self {
        let mesh_id = node.mesh().map(|mesh| {
            let mesh_index = mesh.index();

            if !meshes.contains_key(&mesh_index) {
                meshes.insert(mesh_index, Mesh::from_gltf(surface, mesh, data));
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

    pub fn render<S>(
        &self,
        iface: &ProgramInterface<PbrShaderInterface>,
        tess_gate: &mut TessGate<S>,
        meshes: &HashMap<MeshId, Mesh>,
    ) where
        S: GraphicsContext,
    {
        if let Some(mesh_id) = self.mesh_id {
            if let Some(mesh) = meshes.get(&mesh_id) {
                iface
                    .model
                    .update(self.transform.to_model().to_cols_array_2d());
                mesh.render(iface, tess_gate);
            }
        }
    }
}
