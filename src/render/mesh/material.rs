//! Material for the PBR rendering.

use crate::render::mesh::scene::Assets;
use crate::render::mesh::shaders::ShaderFlags;
use crate::render::mesh::ImportData;
use crate::render::mesh::PbrShaderInterface;
use luminance::shader::program::ProgramInterface;
use luminance_glfw::GlfwSurface;

/// For now values, but will be changed to textures later. Cheers.
pub struct Material {
    pub base_color: [f32; 4],
    pub albedo_texture: Option<super::texture::Texture>,
    pub color_texture_coord_set: Option<u32>,

    pub normal_texture: Option<super::texture::Texture>,
    pub normal_texture_coord_set: Option<u32>,
    pub normal_scale: Option<f32>,

    pub roughness_metallic_texture: Option<super::texture::Texture>,
    pub roughness_metallic_texture_coord_set: Option<u32>,

    pub metallic_texture: Option<super::texture::Texture>,
    pub metallic_texture_coord_set: Option<u32>,

    pub metallic_roughness_values: [f32; 2],
    pub ao: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: gltf::material::AlphaMode,

    pub shader_flags: ShaderFlags,
}

impl Material {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        material: &gltf::Material,
        import: &ImportData,
        assets: &mut Assets,
    ) -> Self {
        let pbr_stuff = material.pbr_metallic_roughness();
        let base_color = pbr_stuff.base_color_factor();
        let metallic = pbr_stuff.metallic_factor();

        let roughness = pbr_stuff.roughness_factor();

        let mut shader_flags = ShaderFlags::empty();

        let (albedo_texture, color_texture_coord_set) =
            if let Some(color_texture) = pbr_stuff.base_color_texture() {
                shader_flags = shader_flags | ShaderFlags::HAS_COLOR_TEXTURE;
                (
                    Some(super::texture::Texture::from_gltf(
                        surface,
                        &color_texture.texture(),
                        &import,
                        std::path::Path::new(""),
                    )),
                    Some(color_texture.tex_coord()),
                )
            } else {
                (None, None)
            };

        let (normal_texture, normal_texture_coord_set, normal_scale) =
            if let Some(normal_texture) = material.normal_texture() {
                shader_flags = shader_flags | ShaderFlags::HAS_NORMAL_TEXTURE;

                (
                    Some(super::texture::Texture::from_gltf(
                        surface,
                        &normal_texture.texture(),
                        &import,
                        std::path::Path::new(""),
                    )),
                    Some(normal_texture.tex_coord()),
                    Some(normal_texture.scale()),
                )
            } else {
                (None, None, None)
            };

        let (roughness_metallic_texture, roughness_metallic_texture_coord_set) =
            if let Some(roughness_texture) = pbr_stuff.metallic_roughness_texture() {
                shader_flags = shader_flags | ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP;

                (
                    Some(super::texture::Texture::from_gltf(
                        surface,
                        &roughness_texture.texture(),
                        &import,
                        std::path::Path::new(""),
                    )),
                    Some(roughness_texture.tex_coord()),
                )
            } else {
                (None, None)
            };

        println!("SHADERS FLAGS = {:?}", shader_flags.to_defines());
        let ao = if let Some(occ) = material.occlusion_texture() {
            occ.strength()
        } else {
            0.0
        };

        assets.shaders.add_shader(shader_flags);

        Self {
            base_color,
            metallic_roughness_values: [metallic, roughness],
            albedo_texture,
            color_texture_coord_set,
            normal_texture,
            normal_texture_coord_set,
            normal_scale,
            roughness_metallic_texture,
            roughness_metallic_texture_coord_set,
            metallic_texture: None,
            metallic_texture_coord_set: None,
            ao,
            shader_flags,
            alpha_mode: material.alpha_mode(),
            alpha_cutoff: material.alpha_cutoff(),
        }
    }

    pub fn apply_uniforms(&self, iface: &ProgramInterface<PbrShaderInterface>) {
        println!("Has color map = {:?}", self.albedo_texture.is_some());
        println!("{:?}", self.base_color);
        iface.u_BaseColorFactor.update([
            self.base_color[0],
            self.base_color[1],
            self.base_color[2],
        ]);
        iface.u_AlphaCutoff.update(self.alpha_cutoff);
        iface
            .u_MetallicRoughnessValues
            .update(self.metallic_roughness_values);
    }
}
