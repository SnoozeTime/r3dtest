#![allow(warnings)]
//! pack roughness map and metallic map into one texture (g for roughness and b for metallic).
//! Usage: `cargo run --bin pack_material -- roughness.png metallic.png`
use image::Pixel;
use image::{ColorType, DynamicImage, GrayImage, RgbImage, Rgba, RgbaImage};
use log::{error, info};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn process<P: AsRef<Path>>(roughness_path: P, metallic_path: P) -> Result<()> {
    info!("Open {:?}", roughness_path.as_ref().display());
    let roughness_img = image::open(roughness_path.as_ref())?;
    dbg!(roughness_img.color());
    info!("Open {:?}", metallic_path.as_ref().display());

    let metallic_img = image::open(metallic_path)?;
    dbg!(metallic_img.color());

    match roughness_img.color() {
        ColorType::Gray(8) => (),
        ColorType::GrayA(8) => info!("GrayA image, will need to convert to Gray"),
        ColorType::RGB(8) => info!("RGB image, will need to convert to Gray"),
        ColorType::RGBA(8) => info!("RGBA image, will need to convert to Gray"),
        _ => {
            return Err(format!(
                "Image color type not supported = {:?}",
                roughness_img.color()
            )
            .into())
        }
    }
    match metallic_img.color() {
        ColorType::Gray(8) => (),
        ColorType::GrayA(8) => info!("GrayA image, will need to convert to Gray"),
        ColorType::RGB(8) => info!("RGB image, will need to convert to Gray"),
        ColorType::RGBA(8) => info!("RGBA image, will need to convert to Gray"),
        _ => {
            return Err(format!(
                "Image color type not supported = {:?}",
                metallic_img.color()
            )
            .into())
        }
    }

    // Make sure we deal with one channel images.
    let roughness = roughness_img.to_luma();
    let metallic = metallic_img.to_luma();
    let new_image = pack_8b_8b_into_rgb(roughness, metallic)?;

    let mut new_image_path = roughness_path.as_ref().to_path_buf();
    new_image_path.set_file_name("roughness_metallic_map.png");

    info!("Will save to {:?}", new_image_path.display());
    new_image.save(new_image_path)?;
    Ok(())
}
fn describe_rgb(roughness: RgbImage) -> Result<DynamicImage> {
    for (x, y, p) in roughness.enumerate_pixels() {
        println!("{:?}", p.data);
    }
    Ok(DynamicImage::ImageRgb8(roughness))
}

fn pack_8b_8b_into_rgb(roughness: GrayImage, metallic: GrayImage) -> Result<DynamicImage> {
    if roughness.dimensions() != metallic.dimensions() {
        return Err(format!(
            "Roughness map and metallic map do not have the same dimensions. {:?} vs {:?}",
            roughness.dimensions(),
            metallic.dimensions()
        )
        .into());
    }

    let dimensions = roughness.dimensions();

    // Fill a buffer with 0.
    let bytes = 4 * dimensions.0 * dimensions.1;
    // TODO RGBA or RGB?
    let mut new_image = RgbaImage::from_raw(
        dimensions.0,
        dimensions.1,
        (0..bytes).map(|_| 0).collect::<Vec<u8>>(),
    )
    .ok_or(String::from("Cannot create a new image"))?;

    for x in 0..dimensions.0 {
        for y in 0..dimensions.1 {
            let roughness_value = roughness.get_pixel(x, y).data[0];
            let metallic_value = metallic.get_pixel(x, y).data[0];
            let pixel = [0, roughness_value, 0, 255]; // FIXME
            let rgb = Rgba::from_slice(&pixel);
            new_image.put_pixel(x, y, *rgb);
        }
    }

    Ok(DynamicImage::ImageRgba8(new_image))
}

fn main() {
    pretty_env_logger::init();
    let mut args = std::env::args();
    let program_name = args.nth(0).unwrap();
    let roughness_filename = args.nth(0);
    let metallic_filename = args.nth(0);

    let roughness_filename = roughness_filename.unwrap_or_else(|| {
        panic!("Usage: `./pack_material roughness.png metallic.png` Missing roughness.png",)
    });
    let metallic_filename = metallic_filename.unwrap_or_else(|| {
        panic!("Usage: `./pack_material roughness.png metallic.png` Missing metallic.png",)
    });

    if let Err(e) = process(roughness_filename, metallic_filename) {
        error!("Error in main = {:?}", e);
    }
}
