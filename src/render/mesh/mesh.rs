use super::{primitive::Primitive, ImportData};
use crate::render::mesh::scene::Assets;
use luminance_glfw::GlfwSurface;

/// Nodes of a scene can have a mesh. A mesh is made of multiple primitives.
#[derive(Default)]
pub struct Mesh {
    pub primitives: Vec<Primitive>,
}

impl Mesh {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        mesh: gltf::Mesh,
        import_data: &ImportData,
        assets: &mut Assets,
    ) -> Self {
        let primitives = mesh
            .primitives()
            .map(|p| Primitive::from_gltf(surface, p, import_data, assets))
            .collect();
        Self { primitives }
    }
}
