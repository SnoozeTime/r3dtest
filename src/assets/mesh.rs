use crate::assets::{AbstractGraphicContext, Asset, AssetError, Loader};
use crate::render::mesh::mesh::Mesh;
use crate::render::mesh::primitive::Primitive;
use crate::render::mesh::{
    Vertex, VertexColor, VertexNormal, VertexPosition, VertexTangent, VertexTexCoord0,
    VertexTexCoord1,
};
use log::{error, info};
use luminance::tess::{Mode, TessBuilder};
use luminance_glfw::GlfwSurface;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
pub type MaterialId = Option<String>;
use std::rc::Rc;
// TODO figure out how to not duplicate.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RawMesh {
    pub primitives: Vec<RawPrimitive>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RawPrimitive {
    pub vertex_buffer: Vec<RawVertex>,
    pub index_buffer: Option<Vec<u32>>,
    pub mode: gltf::mesh::Mode,
    pub material: MaterialId,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RawVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub color: [f32; 4],
    pub tex_coord_0: [f32; 2],
    pub tex_coord_1: [f32; 2],
}

pub struct SyncMeshLoader {
    ctx: AbstractGraphicContext,
    base_path: PathBuf,
}

impl SyncMeshLoader {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let base_path_str = std::env::var("ASSET_PATH").unwrap_or("./".to_string());
        let base_path = Path::new(&base_path_str);
        Self {
            base_path: base_path.join("mesh"),
            ctx: AbstractGraphicContext::from_glfw(surface),
        }
    }
}

impl Loader<Mesh> for SyncMeshLoader {
    fn load(&mut self, asset_name: &str) -> Asset<Mesh> {
        info!("Loading mesh {}", asset_name);
        let mesh_from_path = self.base_path.join(asset_name.to_owned() + ".bincode");

        return match std::fs::read(mesh_from_path)
            .map_err(AssetError::IoError)
            .and_then(|buf| bincode::deserialize::<RawMesh>(&buf).map_err(AssetError::BincodeError))
        {
            Ok(meshLoaded) => {
                info!("Successfully deserialized asset file");
                let mut primitives = vec![];
                for p in meshLoaded.primitives {
                    let vertices = p
                        .vertex_buffer
                        .iter()
                        .map(|raw| Vertex {
                            position: VertexPosition::new(raw.position),
                            normal: VertexNormal::new(raw.normal),
                            tangent: VertexTangent::new(raw.tangent),
                            tex_coord_0: VertexTexCoord0::new(raw.tex_coord_0),
                            tex_coord_1: VertexTexCoord1::new(raw.tex_coord_1),
                            color: VertexColor::new(raw.color),
                        })
                        .collect::<Vec<_>>();

                    let mut tess_builder = TessBuilder::new(&mut self.ctx).add_vertices(vertices);
                    if let Some(indices) = p.index_buffer {
                        tess_builder = tess_builder.set_indices(indices);
                    }
                    // FIXME
                    tess_builder = tess_builder.set_mode(Mode::Triangle);
                    primitives.push(Primitive {
                        tess: Rc::new(tess_builder.build().unwrap()),
                        material: p.material, // FIXME
                    })
                }

                info!("Finished Loading {}", asset_name);

                Asset::from_asset(Mesh { primitives })
            }
            Err(e) => {
                error!("Error loading the asset = {:?}", e);
                e.into()
            }
        };
    }
}
