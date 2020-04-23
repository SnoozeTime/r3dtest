use crate::render::sprite::Metadata;
use luminance::context::GraphicsContext;
use luminance::pixel::NormRGBA8UI;
use luminance::tess::{Mode, Tess, TessBuilder, TessError};
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use wavefront_obj::obj;
pub type SpriteCache = HashMap<String, (Texture<Dim2, NormRGBA8UI>, Metadata)>;
pub type MeshCache = HashMap<String, Tess>;
#[allow(unused_imports)]
use log::{debug, error, info, trace};
use thiserror::Error;
use try_guard::verify;

pub struct AssetManager {
    pub sprites: SpriteCache,
    pub meshes: MeshCache,
}

impl AssetManager {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let mut sprites = HashMap::new();

        let crosshair = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/crosshair.png",
        );
        let shotgun_tex = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/shotgun.png",
        );
        let pistol_tex = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/pistol.png",
        );
        let soldier_tex = load_texture(
            surface,
            std::env::var("ASSET_PATH").unwrap() + "sprites/soldier.png",
        );

        sprites.insert("crosshair".to_string(), crosshair);
        sprites.insert("shotgun".to_string(), shotgun_tex);
        sprites.insert("soldier".to_string(), soldier_tex);
        sprites.insert("pistol".to_string(), pistol_tex);

        let meshes = load_models(
            surface,
            &[
                std::env::var("ASSET_PATH").unwrap() + "models/monkey.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/axis.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/cube.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/ramp.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/arena.obj",
            ],
        );
        Self { sprites, meshes }
    }
}

type VertexIndex = u32;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Error while loading obj file = {0}")]
    LoadObjError(String),
}
#[derive(Debug)]
pub struct Obj {
    pub vertices: Vec<super::Vertex>,
    pub indices: Vec<VertexIndex>,
}

impl Obj {
    pub fn to_tess<C>(self, ctx: &mut C) -> Result<Tess, TessError>
    where
        C: GraphicsContext,
    {
        TessBuilder::new(ctx)
            .set_mode(Mode::Triangle)
            .add_vertices(self.vertices)
            .set_indices(self.indices)
            .build()
    }

    pub fn load<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let file_content: String = {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            content
        };

        let obj_set =
            obj::parse(file_content).map_err(|e| Error::LoadObjError(format!("{:?}", e)))?;
        let objects = obj_set.objects;

        info!("{} objects", objects.len());
        for obj in &objects {
            info!("name -> {}, geom -> {}", obj.name, obj.geometry.len());
        }
        verify!(objects.len() >= 1).ok_or(Error::LoadObjError(
            "expecting at least one object".to_owned(),
        ))?;

        let object = objects.into_iter().next().unwrap();
        verify!(object.geometry.len() == 1).ok_or(Error::LoadObjError(
            "expecting a single geometry".to_owned(),
        ))?;

        let geometry = object.geometry.into_iter().next().unwrap();
        info!("Loading {}", object.name);
        info!("{} vertices", object.vertices.len());
        info!("{} shapes", geometry.shapes.len());

        // build up vertices; for this to work, we remove duplicated vertices by putting them in a
        // map associating the vertex with its ID
        let mut vertex_cache: HashMap<obj::VTNIndex, VertexIndex> = HashMap::new();
        let mut vertices: Vec<super::Vertex> = Vec::new();
        let mut indices: Vec<VertexIndex> = Vec::new();

        for shape in geometry.shapes {
            if let obj::Primitive::Triangle(a, b, c) = shape.primitive {
                for key in &[a, b, c] {
                    if let Some(vertex_index) = vertex_cache.get(key) {
                        indices.push(*vertex_index);
                    } else {
                        let p = object.vertices[key.0];
                        let position =
                            super::VertexPosition::new([p.x as f32, p.y as f32, p.z as f32]);
                        let n = object.normals[key.2.ok_or(Error::LoadObjError(
                            "Missing normal for a vertex".to_owned(),
                        ))?];
                        let normal = super::VertexNormal::new([n.x as f32, n.y as f32, n.z as f32]);
                        let vertex = super::Vertex { position, normal };
                        let vertex_index = vertices.len() as VertexIndex;

                        vertex_cache.insert(*key, vertex_index);
                        vertices.push(vertex);
                        indices.push(vertex_index);
                    }
                }
            } else {
                return Err(Error::LoadObjError(
                    "unsupported non-triangle shape".to_owned(),
                ));
            }
        }

        Ok(Obj { vertices, indices })
    }
}

fn load_models<P: AsRef<Path>, S: GraphicsContext>(
    surface: &mut S,
    models: &[P],
) -> HashMap<String, Tess> {
    let mut cache = HashMap::new();

    for model in models {
        let model_path: &Path = model.as_ref();
        let filename = model_path.file_stem().unwrap().to_str().unwrap().to_owned();
        info!("Will load mesh {} at {}", filename, model_path.display());

        let mesh = Obj::load(model).unwrap();
        let mesh = mesh.to_tess(surface).unwrap();
        cache.insert(filename, mesh);
    }

    cache
}

// read the texture into memory as a whole bloc (i.e. no streaming)
fn read_image<P: AsRef<Path>>(path: P) -> Option<image::RgbaImage> {
    image::open(path).map(|img| img.flipv().to_rgba()).ok()
}

fn load_texture<P: AsRef<Path>>(
    surface: &mut GlfwSurface,
    path: P,
) -> (Texture<Dim2, NormRGBA8UI>, Metadata) {
    let mut metadata_path = path.as_ref().to_path_buf();

    // first the texture.

    let image = read_image(path).unwrap();
    let tex = load_from_disk(surface, image);

    // then the metadata.
    metadata_path.set_extension("ron");
    println!("Metadata path = {:?}", metadata_path);
    let metadata: Metadata =
        ron::de::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();

    (tex, metadata)
}

fn load_from_disk(surface: &mut GlfwSurface, img: image::RgbaImage) -> Texture<Dim2, NormRGBA8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    // create the luminance texture; the third argument is the number of mipmaps we want (leave it
    // to 0 for now) and the latest is the sampler to use when sampling the texels in the
    // shader (we’ll just use the default one)
    let tex = Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    // the first argument disables mipmap generation (we don’t care so far)
    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
