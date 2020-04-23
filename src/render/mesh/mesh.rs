use super::{primitive::Primitive, ImportData};
use crate::render::mesh::deferred::PbrShaderInterface;
use crate::render::mesh::ShaderInterface;

use luminance::context::GraphicsContext;
use luminance::pipeline::TessGate;
use luminance::shader::program::ProgramInterface;
use luminance_glfw::GlfwSurface;

/// Nodes of a scene can have a mesh. A mesh is made of multiple primitives.
pub struct Mesh {
    primitives: Vec<Primitive>,
}

impl Mesh {
    pub fn render<S>(
        &self,
        iface: &ProgramInterface<PbrShaderInterface>,
        tess_gate: &mut TessGate<S>,
    ) where
        S: GraphicsContext,
    {
        //        iface.light_positions.update([0.0, 7.0, 0.0]);
        //        iface.light_colors.update([1.0, 1.0, 1.0]);
        for p in &self.primitives {
            //iface.color.update(p.color);
            p.material.apply_uniforms(&iface);
            tess_gate.render(&p.tess);
        }
    }

    pub fn from_gltf(
        surface: &mut GlfwSurface,
        mesh: gltf::Mesh,
        import_data: &ImportData,
    ) -> Self {
        let primitives = mesh
            .primitives()
            .map(|p| Primitive::from_gltf(surface, p, import_data))
            .collect();
        Self { primitives }
    }
}
