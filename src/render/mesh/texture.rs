//! Textures are loaded from either image or directly from the gltf...

use crate::render::mesh::ImportData;
use gltf::image::Source;
use std::path::Path;
use std::{fs, io};

use iced::Application;
use image;
use image::DynamicImage::*;
use image::GenericImageView;
use image::ImageFormat::{JPEG, PNG};
use image::{DynamicImage, FilterType};
use luminance::pixel::{Pixel, R8UI, RG8UI, RGB8UI, RGBA8UI};
use luminance::texture::{Dim2, GenMipmaps, Sampler};
use luminance_glfw::GlfwSurface;

// TODO use enum instead
pub struct Texture {
    texture: luminance::texture::Texture<Dim2, RGB8UI>,
}

impl Texture {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        texture: &gltf::Texture,
        import: &ImportData,
        base_path: &Path,
    ) -> Self {
        let buffers = &import.1;
        let g_img = texture.source();
        let img = match g_img.source() {
            Source::View { view, mime_type } => {
                let parent_buffer = &buffers[view.buffer().index()].0;
                let begin = view.offset();
                let end = begin + view.length();
                let data = &parent_buffer[begin..end];
                match mime_type {
                    "image/jpeg" => image::load_from_memory_with_format(data, JPEG),
                    "image/png" => image::load_from_memory_with_format(data, PNG),
                    _ => panic!(format!(
                        "unsupported image type (image: {}, mime_type: {})",
                        g_img.index(),
                        mime_type
                    )),
                }
            }
            Source::Uri { uri, mime_type } => {
                if uri.starts_with("data:") {
                    let encoded = uri.split(',').nth(1).unwrap();
                    let data = base64::decode(&encoded).unwrap();
                    let mime_type = if let Some(ty) = mime_type {
                        ty
                    } else {
                        uri.split(',')
                            .nth(0)
                            .unwrap()
                            .split(':')
                            .nth(1)
                            .unwrap()
                            .split(';')
                            .nth(0)
                            .unwrap()
                    };

                    match mime_type {
                        "image/jpeg" => image::load_from_memory_with_format(&data, JPEG),
                        "image/png" => image::load_from_memory_with_format(&data, PNG),
                        _ => panic!(format!(
                            "unsupported image type (image: {}, mime_type: {})",
                            g_img.index(),
                            mime_type
                        )),
                    }
                } else if let Some(mime_type) = mime_type {
                    let path = base_path
                        .parent()
                        .unwrap_or_else(|| Path::new("./"))
                        .join(uri);
                    let file = fs::File::open(path).unwrap();
                    let reader = io::BufReader::new(file);
                    match mime_type {
                        "image/jpeg" => image::load(reader, JPEG),
                        "image/png" => image::load(reader, PNG),
                        _ => panic!(format!(
                            "unsupported image type (image: {}, mime_type: {})",
                            g_img.index(),
                            mime_type
                        )),
                    }
                } else {
                    let path = base_path
                        .parent()
                        .unwrap_or_else(|| Path::new("./"))
                        .join(uri);
                    image::open(path)
                }
            }
        };

        // TODO: handle I/O problems
        let dyn_img = img.expect("Image loading failed.");
        match dyn_img {
            /// Each pixel in this image is 8-bit Rgb
            DynamicImage::ImageRgb8(_) => (),
            _ => panic!("Image type not supported"),
        }
        let (width, height) = dyn_img.dimensions();
        let needs_power_of_two = false;
        let (data, width, height) =
            if needs_power_of_two && (!width.is_power_of_two() || !height.is_power_of_two()) {
                let nwidth = width.next_power_of_two();
                let nheight = height.next_power_of_two();
                let resized = dyn_img.resize(nwidth, nheight, FilterType::Lanczos3);
                (resized.raw_pixels(), resized.width(), resized.height())
            } else {
                (dyn_img.raw_pixels(), dyn_img.width(), dyn_img.height())
            };

        // Now load the texture
        let mut tex: luminance::texture::Texture<Dim2, RGB8UI> =
            luminance::texture::Texture::new(surface, [width, height], 0, Sampler::default())
                .expect("luminance texture creation");

        // the first argument disables mipmap generation (we don’t care so far)
        tex.upload_raw(GenMipmaps::No, &data).unwrap();

        Self { texture: tex }
    }
}
