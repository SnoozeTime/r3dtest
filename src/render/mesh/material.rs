//! Material for the PBR rendering.

use crate::render::mesh::deferred::PbrShaderInterface;
use crate::render::mesh::ImportData;
use luminance::shader::program::ProgramInterface;
use luminance_glfw::GlfwSurface;

/// For now values, but will be changed to textures later. Cheers.
pub struct Material {
    albedo: [f32; 3],
    albedo_texture: Option<super::texture::Texture>,
    roughness: f32,
    ao: f32,
    metallic: f32,
}

impl Material {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        material: &gltf::Material,
        import: &ImportData,
    ) -> Self {
        let pbr_stuff = material.pbr_metallic_roughness();
        let albedo = pbr_stuff.base_color_factor();
        let metallic = pbr_stuff.metallic_factor();
        let roughness = pbr_stuff.roughness_factor();

        let albedo_texture = pbr_stuff.base_color_texture().map(|color_texture| {
            super::texture::Texture::from_gltf(
                surface,
                &color_texture.texture(),
                &import,
                std::path::Path::new(""),
            )
        });

        let ao = if let Some(occ) = material.occlusion_texture() {
            occ.strength()
        } else {
            0.0
        };

        Self {
            albedo: [albedo[0], albedo[1], albedo[2]],
            albedo_texture,
            metallic,
            roughness,
            ao,
        }
    }

    pub fn apply_uniforms(&self, iface: &ProgramInterface<PbrShaderInterface>) {
        iface.albedo.update(self.albedo);
        iface.roughness.update(self.roughness);
        iface.ao.update(self.ao);
        iface.metallic.update(self.metallic);
    }
}
