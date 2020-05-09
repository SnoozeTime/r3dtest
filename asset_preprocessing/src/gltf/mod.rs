//! Scene will be saved as prefab of serialized entities, Primitives will have the index and vertex
//! buffers saved as vec<u8> using bincode.
//! Materials will also be saved and PNG will be extracted (for now).
use fehler::*;
use gltf::image::Source;
use image::DynamicImage;
use image::ImageFormat::{JPEG, PNG};
use log::info;
use r3dtest::ecs::serialization::SerializedEntity;
use r3dtest::ecs::Transform;
use r3dtest::render::Render;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

use r3dtest::assets::{
    material::{Material, Sampler},
    mesh::{RawMesh, RawPrimitive, RawVertex},
};

pub type MaterialId = Option<String>;

type ImportData = (
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
);

type ImgWrapper = (String, DynamicImage);

pub fn sampler_from_gltf(sampler: gltf::texture::Sampler) -> Sampler {
    Sampler {
        min_filter: sampler.min_filter().map(|f| f.as_gl_enum()),
        mag_filter: sampler.mag_filter().map(|f| f.as_gl_enum()),
        wrap_t: sampler.wrap_t().as_gl_enum(),
        wrap_s: sampler.wrap_s().as_gl_enum(),
    }
}

#[throws(GltfError)]
fn material_from_gltf(
    g_material: &gltf::Material,
    material_id: MaterialId,
    import_data: &ImportData,
) -> Material {
    let pbr_stuff = g_material.pbr_metallic_roughness();
    let base_color = pbr_stuff.base_color_factor();
    let metallic = pbr_stuff.metallic_factor();
    let roughness = pbr_stuff.roughness_factor();

    let color_texture_data = if let Some(color_texture) = pbr_stuff.base_color_texture() {
        Some((
            sampler_from_gltf(color_texture.texture().sampler()),
            color_texture.tex_coord(),
        ))
    } else {
        None
    };

    let normal_texture_data = if let Some(normal_texture) = g_material.normal_texture() {
        Some((
            sampler_from_gltf(normal_texture.texture().sampler()),
            normal_texture.tex_coord(),
            normal_texture.scale(),
        ))
    } else {
        None
    };

    let roughness_metallic_texture_data =
        if let Some(roughness_texture) = pbr_stuff.metallic_roughness_texture() {
            Some((
                sampler_from_gltf(roughness_texture.texture().sampler()),
                roughness_texture.tex_coord(),
            ))
        } else {
            None
        };

    let ao = if let Some(occ) = g_material.occlusion_texture() {
        occ.strength()
    } else {
        0.0
    };

    Material {
        base_color,
        metallic_roughness_values: [metallic, roughness],
        color_texture_data,
        normal_texture_data,
        roughness_metallic_texture_data,
        ao,
        emissive_factor: g_material.emissive_factor(),
        alpha_cutoff: g_material.alpha_cutoff(),
        ..Material::default()
    }
}

#[throws(GltfError)]
fn extract_textures(
    g_material: gltf::Material,
    material_id: MaterialId,
    import_data: &ImportData,
) -> Vec<ImgWrapper> {
    let mut images = vec![];
    let pbr_stuff = g_material.pbr_metallic_roughness();

    let image_pref = material_id.clone().unwrap_or("default".to_owned());
    if let Some(color_texture) = pbr_stuff.base_color_texture() {
        let texture = color_texture.texture();
        images.push((
            format!("{}_color.png", image_pref),
            load_texture(texture, import_data)?,
        ));
    }
    if let Some(normal_texture) = g_material.normal_texture() {
        let texture = normal_texture.texture();
        images.push((
            format!("{}_normal.png", image_pref),
            load_texture(texture, import_data)?,
        ));
    }
    if let Some(rm_texture) = pbr_stuff.metallic_roughness_texture() {
        let texture = rm_texture.texture();
        images.push((
            format!("{}_roughness_metallic.png", image_pref),
            load_texture(texture, import_data)?,
        ));
    }
    images
}

#[throws(GltfError)]
fn load_texture(texture: gltf::Texture, import: &ImportData) -> DynamicImage {
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
                _ => throw!(GltfError::UnsupportedImageType(format!(
                    "image: {}, mime_type: {})",
                    g_img.index(),
                    mime_type
                ))),
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
                    _ => throw!(GltfError::UnsupportedImageType(format!(
                        "image: {}, mime_type: {})",
                        g_img.index(),
                        mime_type
                    ))),
                }
            } else if let Some(mime_type) = mime_type {
                let path = Path::new("./").join(uri);

                let file = fs::File::open(path).unwrap();
                let reader = io::BufReader::new(file);
                match mime_type {
                    "image/jpeg" => image::load(reader, JPEG),
                    "image/png" => image::load(reader, PNG),
                    _ => throw!(GltfError::UnsupportedImageType(format!(
                        "image: {}, mime_type: {})",
                        g_img.index(),
                        mime_type
                    ))),
                }
            } else {
                let path = Path::new("./").join(uri);
                image::open(path)
            }
        }
    };

    img?
}

#[throws(GltfError)]
fn extract_prefix(p: &Path) -> String {
    p.file_stem()
        .map(|f| f.to_string_lossy().to_string())
        .ok_or(GltfError::BadFilename(p.display().to_string()))?
}

#[derive(Debug, Error)]
pub enum GltfError {
    #[error(transparent)]
    ImportError(#[from] gltf::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ImageError(#[from] image::ImageError),

    #[error(transparent)]
    RonError(#[from] ron::ser::Error),

    #[error("The image type is not supported: {0}")]
    UnsupportedImageType(String),

    #[error("The GLTF needs at least one scene")]
    NoSceneError,

    #[error(transparent)]
    BincodeError(#[from] bincode::Error),

    #[error("cannot get name of path from {0}")]
    BadFilename(String),
}

#[throws(GltfError)]
fn mesh_from_gltf(
    g_mesh: gltf::Mesh,
    import_data: &ImportData,
    materials: &mut HashMap<MaterialId, Material>,
    images: &mut Vec<ImgWrapper>,
    resource_prefix: &String,
) -> RawMesh {
    let mut primitives = vec![];
    for p in g_mesh.primitives() {
        let p = primitive_from_gltf(p, import_data, materials, images, resource_prefix)?;
        primitives.push(p);
    }
    RawMesh { primitives }
}

#[throws(GltfError)]
fn primitive_from_gltf(
    primitive: gltf::Primitive,
    import_data: &ImportData,
    materials: &mut HashMap<MaterialId, Material>,
    images: &mut Vec<ImgWrapper>,
    resource_prefix: &String,
) -> RawPrimitive {
    let buffers = &import_data.1;
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let mut vertices = reader
        .read_positions()
        .unwrap()
        .map(|p| RawVertex {
            position: p,
            ..RawVertex::default()
        })
        .collect::<Vec<_>>();

    if let Some(normals) = reader.read_normals() {
        for (i, normal) in normals.enumerate() {
            vertices[i].normal = normal
        }
    }

    if let Some(colors) = reader.read_colors(0) {
        let colors = colors.into_rgba_f32();
        for (i, c) in colors.enumerate() {
            vertices[i].color = c;
        }
    }

    if let Some(tangents) = reader.read_tangents() {
        for (i, tangents) in tangents.enumerate() {
            vertices[i].tangent = tangents;
        }
    }

    let mut set = 0;
    while let Some(texture_coords) = reader.read_tex_coords(set) {
        if set > 1 {
            break; //only supports mesh and primitive UV
        }
        for (i, uv) in texture_coords.into_f32().enumerate() {
            match set {
                0 => vertices[i].tex_coord_0 = uv,
                1 => vertices[i].tex_coord_1 = uv,
                _ => (),
            }
        }
        set += 1;
    }

    let indices = reader
        .read_indices()
        .map(|read_indices| read_indices.into_u32().collect::<Vec<_>>());

    let mode = primitive.mode();

    let material_id = primitive
        .material()
        .name()
        .map(|n| format!("{}_{}", resource_prefix, n));
    if !materials.contains_key(&material_id) {
        info!("Will extract material {:?}", material_id);

        let material = material_from_gltf(&primitive.material(), material_id.clone(), import_data)?;
        materials.insert(material_id.clone(), material);
        images.append(&mut extract_textures(
            primitive.material(),
            material_id.clone(),
            import_data,
        )?);
    }

    RawPrimitive {
        vertex_buffer: vertices,
        mode,
        material: material_id,
        index_buffer: indices,
    }
}

#[throws(GltfError)]
fn save_images(asset_dir: PathBuf, images: Vec<ImgWrapper>) {
    info!("Save images to {:?}", asset_dir.display());
    for (image_name, image) in images {
        image.save(asset_dir.join(Path::new(&image_name)))?;
    }
}

#[throws(GltfError)]
fn save_prefab(path: PathBuf, prefab: SerializedEntity) {
    info!("Save prefab to {:?}", path.display());
    let as_str = ron::ser::to_string_pretty(&prefab, ron::ser::PrettyConfig::default())?;
    fs::write(path, as_str)?;
}

#[throws(GltfError)]
fn save_materials(path: PathBuf, materials: HashMap<MaterialId, Material>) {
    info!("Save materials to {:?}", path.display());
    for (id, material) in materials {
        let mut id = id.unwrap_or("default_material".to_owned());
        id.push_str(".ron");
        let material_path = path.join(id);
        let as_str = ron::ser::to_string_pretty(&material, ron::ser::PrettyConfig::default())?;
        fs::write(material_path, as_str)?;
    }
}

#[throws(GltfError)]
fn save_meshes(path: PathBuf, meshes: HashMap<String, RawMesh>) {
    info!("Save meshes to {:?}", path.display());
    for (mut id, mesh) in meshes {
        id.push_str(".bincode");
        let mesh_path = path.join(id);
        let as_str = bincode::serialize(&mesh)?;
        fs::write(mesh_path, as_str)?;
    }
}

#[throws(GltfError)]
pub fn import_gltf<P>(path: P, asset_dir: P)
where
    P: AsRef<Path>,
{
    info!("Will process {:?}", path.as_ref().display());
    let resources_prefix = extract_prefix(path.as_ref())?;
    info!("Import GLTF file");
    let import = gltf::import(path.as_ref())?;
    let g_scene = import.0.scenes().next().ok_or(GltfError::NoSceneError)?;
    info!("Finished importing the file");
    let mut meshes: HashMap<String, RawMesh> = HashMap::new();
    let mut materials: HashMap<MaterialId, Material> = HashMap::new();
    let mut images: Vec<ImgWrapper> = vec![];
    // the parent entity.
    let mut prefab = SerializedEntity {
        transform: Some(Transform::default()),
        ..SerializedEntity::default()
    };

    for node in g_scene.nodes() {
        let mut entity = SerializedEntity::default();
        if let Some(mesh) = node.mesh() {
            let mesh_ref = mesh
                .name()
                .map(|n| n.to_string())
                .unwrap_or(format!("mesh{}", mesh.index()));
            let mesh_id = format!("{}_{}", resources_prefix, mesh_ref);
            if !meshes.contains_key(&mesh_id) {
                info!("Will extract mesh {}", mesh_id);
                let mesh = mesh_from_gltf(
                    mesh,
                    &import,
                    &mut materials,
                    &mut images,
                    &resources_prefix,
                )?;
                meshes.insert(mesh_id.clone(), mesh);
            }

            let render = Render {
                enabled: true,
                mesh: mesh_id.clone(),
            };

            entity.render = Some(render);
        }

        let (translation, rotation, scale) = node.transform().decomposed();
        let rotation: glam::Quat = rotation.into();
        let transform = Transform {
            translation: translation.into(),
            rotation,
            scale: scale.into(),
            dirty: true,
        };

        entity.transform = Some(transform.clone());
        entity.local_transform = Some(transform.into());
        prefab.children.push(entity);
    }

    info!("Saving the assets to {}", asset_dir.as_ref().display());
    let material_path = asset_dir.as_ref().join("material");
    fs::create_dir_all(material_path.clone())?;
    let mesh_path = asset_dir.as_ref().join("mesh");
    fs::create_dir_all(mesh_path.clone())?;
    let prefab_path = asset_dir.as_ref().join("prefab");
    fs::create_dir_all(prefab_path.clone())?;

    save_images(material_path.clone(), images)?;
    save_prefab(
        prefab_path.join(format!("{}_prefab.ron", resources_prefix)),
        prefab,
    )?;
    save_meshes(mesh_path, meshes)?;
    save_materials(material_path, materials)?;

    info!("Success!");
}
