//! Material for the PBR rendering.

use crate::render::mesh::scene::Assets;
use crate::render::mesh::shaders::ShaderFlags;
use crate::render::mesh::texture::Texture;
use crate::render::mesh::ImportData;
use crate::render::mesh::PbrShaderInterface;
use bitflags::_core::fmt::Formatter;
use luminance::pixel::NormRGB8UI;
use luminance::shader::program::ProgramInterface;
use luminance::texture::{Dim2, GenMipmaps, MagFilter, MinFilter, Sampler, Wrap};
use luminance_glfw::GlfwSurface;
use std::fmt;
use std::path::Path;
use thiserror::Error;

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

    pub metallic_roughness_values: [f32; 2],
    pub ao: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: gltf::material::AlphaMode,

    pub shader_flags: ShaderFlags,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            albedo_texture: None,
            color_texture_coord_set: None,
            normal_scale: None,
            normal_texture_coord_set: None,
            normal_texture: None,
            roughness_metallic_texture: None,
            roughness_metallic_texture_coord_set: None,
            metallic_roughness_values: [0.0, 0.5],
            ao: 1.0,
            alpha_cutoff: 0.0,
            alpha_mode: gltf::material::AlphaMode::Opaque,
            shader_flags: ShaderFlags::empty(),
        }
    }
}

#[derive(Debug, Error)]
pub enum TextureError {
    #[error(transparent)]
    NoSuchFile(#[from] std::io::Error),

    #[error(transparent)]
    ImageError(#[from] image::ImageError),
}

impl fmt::Debug for Material {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(
            f,
            "[MATERIAL:\nbase_color = {:?}\nhas_color_texture = {}\ncolor_texture_set={:?}",
            self.base_color,
            self.albedo_texture.is_some(),
            self.color_texture_coord_set
        )?;
        writeln!(f, "has normal_texture={:?}\nnormal_texture_coord_set={:?}\nnormal-scale={:?}\nhas roughness_metallic_texture={}", self.normal_texture.is_some(), self.normal_texture_coord_set, self.normal_scale, self.roughness_metallic_texture.is_some())?;
        writeln!(f, "roughness_metallic_texture_coord_set={:?}\nmetallic_roughness_values={:?}\nao={:?}\nalpha_cutoff={:?}\nalpha_mode={:?}", self.roughness_metallic_texture_coord_set, self.metallic_roughness_values, self.ao, self.alpha_cutoff, self.alpha_mode)?;
        writeln!(f, "shader_flags={:?}", self.shader_flags)
    }
}

impl Material {
    /// Create a material directly from textures.
    pub fn from_textures<P>(
        surface: &mut GlfwSurface,
        base_color: [f32; 4],
        color_texture: Option<(P, u32)>,
        normal_texture: Option<(P, u32, f32)>,
        metallic_roughness_texture: Option<(P, u32)>,
        metallic_roughness_values: [f32; 2],
    ) -> Result<Self, TextureError>
    where
        P: AsRef<Path>,
    {
        let mut shader_flags = ShaderFlags::empty();

        // Load color texture.
        let (albedo_texture, color_texture_coord_set) = if let Some((path, coord)) = color_texture {
            let img = read_image(path)?;
            let texture = load_from_disk(surface, img);
            shader_flags |= ShaderFlags::HAS_COLOR_TEXTURE;
            (Some(Texture { texture }), Some(coord))
        } else {
            (None, None)
        };

        // Load normal texture.
        let (normal_texture, normal_texture_coord_set, normal_scale) =
            if let Some((path, coord, scale)) = normal_texture {
                let img = read_image(path)?;
                let texture = load_from_disk(surface, img);
                shader_flags |= ShaderFlags::HAS_NORMAL_TEXTURE;
                (Some(Texture { texture }), Some(coord), Some(scale))
            } else {
                (None, None, None)
            };

        // load Metallic and roughness
        let (roughness_metallic_texture, roughness_metallic_texture_coord_set) =
            if let Some((path, coord)) = metallic_roughness_texture {
                let img = read_image(path)?;
                let texture = load_from_disk(surface, img);
                shader_flags |= ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP;
                (Some(Texture { texture }), Some(coord))
            } else {
                (None, None)
            };

        let mat = Self {
            base_color,
            albedo_texture,
            color_texture_coord_set,
            normal_texture,
            normal_texture_coord_set,
            normal_scale,
            metallic_roughness_values,
            roughness_metallic_texture,
            roughness_metallic_texture_coord_set,
            alpha_mode: gltf::material::AlphaMode::Opaque,
            alpha_cutoff: 0.0,
            ao: 1.0,
            shader_flags,
        };

        println!("MATERIAL -> {:?}", mat);
        Ok(mat)
    }

    /// Create a material from a GLTF document.
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
            ao,
            shader_flags,
            alpha_mode: material.alpha_mode(),
            alpha_cutoff: material.alpha_cutoff(),
        }
    }

    pub fn apply_uniforms(&self, iface: &ProgramInterface<PbrShaderInterface>) {
        iface.u_base_color_factor.update([
            self.base_color[0],
            self.base_color[1],
            self.base_color[2],
        ]);
        iface.u_alpha_cutoff.update(self.alpha_cutoff);
        iface
            .u_metallic_roughness_values
            .update(self.metallic_roughness_values);
    }
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Result<image::RgbImage, image::ImageError> {
    image::open(path).map(|img| img.flipv().to_rgb())
}

fn load_from_disk(
    surface: &mut GlfwSurface,
    img: image::RgbImage,
) -> luminance::texture::Texture<Dim2, NormRGB8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    let mut sampler = Sampler::default();
    sampler.mag_filter = MagFilter::Linear;
    sampler.min_filter = MinFilter::LinearMipmapLinear;
    sampler.wrap_t = Wrap::Repeat;
    sampler.wrap_s = Wrap::Repeat;

    let tex = luminance::texture::Texture::new(surface, [width, height], 0, sampler)
        .expect("luminance texture creation");

    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
