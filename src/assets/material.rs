//! Material are the properties of the PBR shader. Can have one material for multiple primitives...
//! Material can also contain some textures (color, normal, ...) so the manager needs to load them from
//! file.
use crate::assets::{AbstractGraphicContext, Asset, AssetError, Loader};
use crate::render::mesh::ShaderFlags;
use bitflags::_core::cell::RefCell;
use crossbeam_channel::unbounded;
use image::RgbImage;
use log::error;
use log::info;
use luminance::context::GraphicsContext;
use luminance::pixel::NormRGB8UI;
use luminance::state::GraphicsState;
use luminance::texture::{Dim2, GenMipmaps, MagFilter, MinFilter, Wrap};
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;

#[derive(Debug, Serialize, Deserialize)]
pub struct Sampler {
    min_filter: Option<u32>,
    mag_filter: Option<u32>,
    wrap_s: u32,
    wrap_t: u32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Material {
    pub base_color: [f32; 4],
    pub metallic_roughness_values: [f32; 2],
    pub ao: f32,
    pub alpha_cutoff: f32,

    #[serde(skip)]
    pub color_image: Option<image::RgbImage>,
    #[serde(skip)]
    pub color_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a color texture.
    // coord set.
    pub color_texture_data: Option<(Sampler, u32)>,

    #[serde(skip)]
    pub normal_image: Option<image::RgbImage>,
    #[serde(skip)]
    pub normal_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a normal texture.
    // Coord set and normal scale.
    pub normal_texture_data: Option<(Sampler, u32, f32)>,

    #[serde(skip)]
    pub roughness_metallic_image: Option<image::RgbImage>,
    #[serde(skip)]
    pub roughness_metallic_texture: Option<luminance::texture::Texture<Dim2, NormRGB8UI>>,
    // if that is not None, the materials has a roughness metallic texture.
    // Coord set
    pub roughness_metallic_texture_data: Option<(Sampler, u32)>,

    // ----------------------------------------------------------
    // Emissive color, emissive map. Not affected by the light and so on.
    // Added to the total light at the end
    // ----------------------------------------------------------
    #[serde(default)]
    pub emissive_factor: [f32; 3],

    #[serde(skip)]
    pub shader_flags: ShaderFlags,
}

impl Material {
    /// Just keep the physical properties (no image nor texture). this is meant to be edited via an
    /// editor.
    pub fn clone_strip(&self) -> Material {
        Material {
            base_color: self.base_color,
            metallic_roughness_values: self.metallic_roughness_values,
            ao: self.ao,
            alpha_cutoff: self.alpha_cutoff,
            emissive_factor: self.emissive_factor,
            ..Material::default()
        }
    }
}

/// WARNING !!! It's not safe to share opengl state between states and textures should be uploaded
/// in the main thread!!! It is Send here because I assume I WILL NOT use opengl in another thread.
/// Only the images are loaded from file, which actually takes a bunch of time.
unsafe impl Send for Material {}

pub struct SyncMaterialLoader {
    base_path: PathBuf,
}

impl SyncMaterialLoader {
    pub fn new() -> Self {
        let base_path_str = std::env::var("ASSET_PATH").unwrap_or("./".to_string());
        let base_path = Path::new(&base_path_str);

        Self {
            base_path: base_path.join("material"),
        }
    }
}
impl Loader<Material> for SyncMaterialLoader {
    fn load(&mut self, asset_name: &str) -> Asset<Material> {
        let asset = Asset::new();
        load_material(&self.base_path, asset_name, Asset::clone(&asset));
        asset
    }

    fn upload_to_gpu(&self, ctx: &mut GlfwSurface, inner: &mut Material) {
        upload_to_gpu(ctx, inner);
    }
}

fn upload_to_gpu(ctx: &mut GlfwSurface, inner: &mut Material) {
    if let Some(img) = inner.color_image.take() {
        if let Some((sampler, _)) = inner.color_texture_data.as_ref() {
            let tex = load_with_sampler(ctx, img, sampler).unwrap(); // FIXME unwrap.
            inner.color_texture = Some(tex);
        }
    }
    if let Some(img) = inner.normal_image.take() {
        if let Some((sampler, _, _)) = inner.normal_texture_data.as_ref() {
            let tex = load_with_sampler(ctx, img, sampler).unwrap(); // FIXME unwrap.
            inner.normal_texture = Some(tex);
        }
    }

    if let Some(img) = inner.roughness_metallic_image.take() {
        if let Some((sampler, _)) = inner.roughness_metallic_texture_data.as_ref() {
            let tex = load_with_sampler(ctx, img, sampler).unwrap(); // FIXME unwrap.
            inner.roughness_metallic_texture = Some(tex);
        }
    }
}

fn load_material(base_path: &PathBuf, asset_name: &str, mut asset: Asset<Material>) {
    // Just load all the file synchronously.
    info!("Will load {}", asset_name);
    let material_path = base_path.join(asset_name.to_owned() + ".ron");
    info!(
        "Loading material parameters at {:?}",
        material_path.display()
    );

    match fs::read_to_string(material_path)
        .map_err(AssetError::IoError)
        .and_then(|content| ron::de::from_str::<Material>(&content).map_err(AssetError::DeserError))
    {
        Ok(mut material) => {
            let mut shader_flags = ShaderFlags::empty();
            //now try to read the texture if it has some.
            if let Some((sampler, _)) = material.color_texture_data.as_ref() {
                shader_flags |= ShaderFlags::HAS_COLOR_TEXTURE;
                let color_path = base_path.join(format!("{}{}", asset_name, "_color.png"));

                match read_image(color_path) {
                    Ok(img) => material.color_image = Some(img),
                    Err(e) => {
                        asset.set_error(e.into());
                        return;
                    }
                }
            }
            if let Some((sampler, _, _)) = material.normal_texture_data.as_ref() {
                shader_flags |= ShaderFlags::HAS_NORMAL_TEXTURE;

                let normal_path = base_path.join(format!("{}{}", asset_name, "_normal.png"));
                match read_image(normal_path) {
                    Ok(img) => material.normal_image = Some(img),
                    Err(e) => {
                        asset.set_error(e.into());
                        return;
                    }
                }
            }
            if let Some((sampler, _)) = material.roughness_metallic_texture_data.as_ref() {
                shader_flags |= ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP;

                let roughness_metallic_path =
                    base_path.join(format!("{}{}", asset_name, "_roughness_metallic.png"));
                match read_image(roughness_metallic_path) {
                    Ok(img) => material.roughness_metallic_image = Some(img),
                    Err(e) => {
                        asset.set_error(e.into());
                        return;
                    }
                }
            }

            material.shader_flags = shader_flags;
            asset.set_loaded(material);
            info!("Finished loading {:?}", asset_name);
        }
        Err(e) => {
            error!("Error loading asset = {:?}", e);
            asset.set_error(e);
        }
    };
}

fn load_with_sampler(
    ctx: &mut GlfwSurface,
    img: image::RgbImage,
    mat_sampler: &Sampler,
) -> Result<luminance::texture::Texture<Dim2, NormRGB8UI>, AssetError> {
    //
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    let mut sampler = luminance::texture::Sampler::default();
    /**

        /// Corresponds to `GL_NEAREST`.
        pub const NEAREST: u32 = 9728;

        /// Corresponds to `GL_LINEAR`.
        pub const LINEAR: u32 = 9729;

        /// Corresponds to `GL_NEAREST_MIPMAP_NEAREST`.
        pub const NEAREST_MIPMAP_NEAREST: u32 = 9984;

        /// Corresponds to `GL_LINEAR_MIPMAP_NEAREST`.
        pub const LINEAR_MIPMAP_NEAREST: u32 = 9985;

        /// Corresponds to `GL_NEAREST_MIPMAP_LINEAR`.
        pub const NEAREST_MIPMAP_LINEAR: u32 = 9986;

        /// Corresponds to `GL_LINEAR_MIPMAP_LINEAR`.
        pub const LINEAR_MIPMAP_LINEAR: u32 = 9987;

        /// Corresponds to `GL_CLAMP_TO_EDGE`.
        pub const CLAMP_TO_EDGE: u32 = 33_071;

        /// Corresponds to `GL_MIRRORED_REPEAT`.
        pub const MIRRORED_REPEAT: u32 = 33_648;

        /// Corresponds to `GL_REPEAT`.
        pub const REPEAT: u32 = 10_497;
    **/
    match mat_sampler.mag_filter {
        Some(9728) => sampler.mag_filter = MagFilter::Nearest,
        Some(9729) => sampler.mag_filter = MagFilter::Linear,
        _ => (),
    }
    match mat_sampler.min_filter {
        Some(9728) => sampler.min_filter = MinFilter::Nearest,
        Some(9729) => sampler.min_filter = MinFilter::Linear,
        Some(9987) => sampler.min_filter = MinFilter::LinearMipmapLinear,
        Some(9985) => sampler.min_filter = MinFilter::LinearMipmapNearest,
        Some(9986) => sampler.min_filter = MinFilter::NearestMipmapLinear,
        Some(9984) => sampler.min_filter = MinFilter::NearestMipmapNearest,
        _ => (),
    }
    match mat_sampler.wrap_s {
        10_497 => sampler.wrap_s = Wrap::Repeat,
        33_648 => sampler.wrap_s = Wrap::MirroredRepeat,
        33_071 => sampler.wrap_s = Wrap::ClampToEdge,
        _ => (),
    }

    match mat_sampler.wrap_t {
        10_497 => sampler.wrap_t = Wrap::Repeat,
        33_648 => sampler.wrap_t = Wrap::MirroredRepeat,
        33_071 => sampler.wrap_t = Wrap::ClampToEdge,
        _ => (),
    }

    let tex = luminance::texture::Texture::new(ctx, [width, height], 0, sampler).unwrap();

    tex.upload_raw(GenMipmaps::No, &texels).unwrap();
    Ok(tex)
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Result<image::RgbImage, image::ImageError> {
    image::open(path).map(|img| img.flipv().to_rgb())
}

pub struct AsyncMaterialLoader {
    child_thread: thread::JoinHandle<()>,
    tx: crossbeam_channel::Sender<(Asset<Material>, String)>,
}

impl AsyncMaterialLoader {
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<(Asset<Material>, String)>();
        let child_thread = thread::spawn(move || {
            let base_path_str = std::env::var("ASSET_PATH").unwrap_or("./".to_string());
            let base_path = Path::new(&base_path_str);
            let base_path = base_path.join("material");

            //            let mut ctx = AbstractGraphicContext::new();
            while let Ok((asset, asset_name)) = rx.recv() {
                load_material(&base_path, asset_name.as_str(), asset);
            }
        });

        Self { child_thread, tx }
    }
}

impl Loader<Material> for AsyncMaterialLoader {
    fn load(&mut self, asset_name: &str) -> Asset<Material> {
        let asset = Asset::default();
        self.tx
            .send((Asset::clone(&asset), asset_name.to_owned()))
            .unwrap();
        asset
    }

    fn upload_to_gpu(&self, ctx: &mut GlfwSurface, inner: &mut Material) {
        upload_to_gpu(ctx, inner);
    }
}
