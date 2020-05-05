//! Material are the properties of the PBR shader. Can have one material for multiple primitives...
//! Material can also contain some textures (color, normal, ...) so the manager needs to load them from
//! file.
use crate::assets::{Asset, AssetError, Loader};
use crate::render::mesh::ShaderFlags;
use log::info;
use luminance::context::GraphicsContext;
use luminance::pixel::NormRGB8UI;
use luminance::texture::{Dim2, GenMipmaps, MagFilter, MinFilter, Wrap};
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Sampler {
    min_filter: Option<gltf::texture::MinFilter>,
    mag_filter: Option<gltf::texture::MagFilter>,
    wrap_s: gltf::texture::WrappingMode,
    wrap_t: gltf::texture::WrappingMode,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Material {
    pub base_color: [f32; 4],
    pub metallic_roughness_values: [f32; 2],
    pub ao: f32,
    pub alpha_cutoff: f32,

    #[serde(skip)]
    pub color_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a color texture.
    // coord set.
    pub color_texture_data: Option<(Sampler, u32)>,

    #[serde(skip)]
    pub normal_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a normal texture.
    // Coord set and normal scale.
    pub normal_texture_data: Option<(Sampler, u32, f32)>,

    #[serde(skip)]
    pub roughness_metallic_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a roughness metallic texture.
    // Coord set
    pub roughness_metallic_texture_data: Option<(Sampler, u32)>,

    #[serde(skip)]
    shader_flags: ShaderFlags,
}

pub struct SyncMaterialLoader;

impl Loader<Material> for SyncMaterialLoader {
    fn load(&self, ctx: &mut GlfwSurface, asset_name: &str) -> Asset<Material> {
        // Just load all the file synchronously.
        info!("Will load {}", asset_name);

        let base_path_str = std::env::var("ASSET_PATH").unwrap_or("./".to_string());
        let base_path = Path::new(&base_path_str);
        let base_path = base_path.join("material");
        let material_path = base_path.join(asset_name.to_owned() + ".ron");
        info!(
            "Loading material parameters at {:?}",
            material_path.display()
        );

        return match fs::read_to_string(material_path)
            .map_err(AssetError::IoError)
            .and_then(|content| {
                ron::de::from_str::<Material>(&content).map_err(AssetError::DeserError)
            }) {
            Ok(mut material) => {
                let mut shader_flags = ShaderFlags::empty();
                //now try to read the texture if it has some.
                if let Some((sampler, _)) = material.color_texture_data.as_ref() {
                    shader_flags |= ShaderFlags::HAS_COLOR_TEXTURE;
                    let color_path = base_path.join(format!("{}{}", asset_name, "_color.png"));
                    match load_with_sampler(ctx, &color_path, sampler) {
                        Ok(texture) => material.color_texture = Some(texture),
                        Err(e) => return e.into(),
                    }
                }
                if let Some((sampler, _, _)) = material.normal_texture_data.as_ref() {
                    shader_flags |= ShaderFlags::HAS_NORMAL_TEXTURE;

                    let normal_path = base_path.join(format!("{}{}", asset_name, "_normal.png"));
                    match load_with_sampler(ctx, &normal_path, &sampler) {
                        Ok(texture) => material.normal_texture = Some(texture),
                        Err(e) => return e.into(),
                    }
                }
                if let Some((sampler, _)) = material.roughness_metallic_texture_data.as_ref() {
                    shader_flags |= ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP;

                    let roughness_metallic_path =
                        base_path.join(format!("{}{}", asset_name, "_roughness_metallic.png"));

                    match load_with_sampler(ctx, &roughness_metallic_path, &sampler) {
                        Ok(texture) => material.roughness_metallic_texture = Some(texture),
                        Err(e) => return e.into(),
                    }
                }

                material.shader_flags = shader_flags;
                Asset::from_asset(material)
            }
            Err(e) => e.into(),
        };
    }
}

fn load_with_sampler(
    ctx: &mut GlfwSurface,
    texture_path: &Path,
    mat_sampler: &Sampler,
) -> Result<luminance::texture::Texture<Dim2, NormRGB8UI>, AssetError> {
    //
    let img = read_image(texture_path)?;
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    let mut sampler = luminance::texture::Sampler::default();
    match mat_sampler.mag_filter {
        Some(gltf::texture::MagFilter::Nearest) => sampler.mag_filter = MagFilter::Nearest,
        Some(gltf::texture::MagFilter::Linear) => sampler.mag_filter = MagFilter::Linear,
        None => (),
    }
    match mat_sampler.min_filter {
        Some(gltf::texture::MinFilter::Nearest) => sampler.min_filter = MinFilter::Nearest,
        Some(gltf::texture::MinFilter::Linear) => sampler.min_filter = MinFilter::Linear,
        Some(gltf::texture::MinFilter::LinearMipmapLinear) => {
            sampler.min_filter = MinFilter::LinearMipmapLinear
        }
        Some(gltf::texture::MinFilter::LinearMipmapNearest) => {
            sampler.min_filter = MinFilter::LinearMipmapNearest
        }
        Some(gltf::texture::MinFilter::NearestMipmapLinear) => {
            sampler.min_filter = MinFilter::NearestMipmapLinear
        }
        Some(gltf::texture::MinFilter::NearestMipmapNearest) => {
            sampler.min_filter = MinFilter::NearestMipmapNearest
        }
        None => (),
    }
    match mat_sampler.wrap_s {
        gltf::texture::WrappingMode::Repeat => sampler.wrap_s = Wrap::Repeat,
        gltf::texture::WrappingMode::MirroredRepeat => sampler.wrap_s = Wrap::MirroredRepeat,
        gltf::texture::WrappingMode::ClampToEdge => sampler.wrap_s = Wrap::ClampToEdge,
    }

    match mat_sampler.wrap_t {
        gltf::texture::WrappingMode::Repeat => sampler.wrap_t = Wrap::Repeat,
        gltf::texture::WrappingMode::MirroredRepeat => sampler.wrap_t = Wrap::MirroredRepeat,
        gltf::texture::WrappingMode::ClampToEdge => sampler.wrap_t = Wrap::ClampToEdge,
    }

    let tex = luminance::texture::Texture::new(ctx, [width, height], 0, sampler).unwrap();

    tex.upload_raw(GenMipmaps::No, &texels).unwrap();
    Ok(tex)
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Result<image::RgbImage, image::ImageError> {
    image::open(path).map(|img| img.flipv().to_rgb())
}
