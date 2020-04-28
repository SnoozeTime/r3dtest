use super::{
    Vertex, VertexColor, VertexNormal, VertexPosition, VertexTangent, VertexTexCoord0,
    VertexTexCoord1,
};
use crate::render::mesh::material::Material;
use crate::render::mesh::scene::{Assets, MaterialId};
use crate::render::mesh::ImportData;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance_glfw::GlfwSurface;

/// Smallest unit in gltf. Contains the vertices,
pub struct Primitive {
    pub tess: Tess,
    pub material: MaterialId,
}

impl Primitive {
    pub fn from_gltf(
        surface: &mut GlfwSurface,
        primitive: gltf::Primitive,
        import_data: &ImportData,
        assets: &mut Assets,
    ) -> Self {
        let buffers = &import_data.1;
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let mut vertices = reader
            .read_positions()
            .unwrap()
            .map(|p| Vertex {
                position: VertexPosition::new(p),
                ..Vertex::default()
            })
            .collect::<Vec<_>>();

        if let Some(normals) = reader.read_normals() {
            for (i, normal) in normals.enumerate() {
                vertices[i].normal = VertexNormal::new(normal);
            }
        }

        if let Some(colors) = reader.read_colors(0) {
            let colors = colors.into_rgba_f32();
            for (i, c) in colors.enumerate() {
                vertices[i].color = VertexColor::new(c);
            }
        }

        if let Some(tangents) = reader.read_tangents() {
            for (i, tangents) in tangents.enumerate() {
                vertices[i].tangent = VertexTangent::new(tangents);
            }
        }

        let mut set = 0;
        while let Some(texture_coords) = reader.read_tex_coords(set) {
            if set > 1 {
                break; //only supports mesh and primitive UV
            }
            for (i, uv) in texture_coords.into_f32().enumerate() {
                match set {
                    0 => vertices[i].tex_coord_0 = VertexTexCoord0::new(uv),
                    1 => vertices[i].tex_coord_1 = VertexTexCoord1::new(uv),
                    _ => (),
                }
            }
            set += 1;
        }

        let indices = reader
            .read_indices()
            .map(|read_indices| read_indices.into_u32().collect::<Vec<_>>());

        let mode = match primitive.mode() {
            gltf::mesh::Mode::TriangleStrip => Mode::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => Mode::TriangleFan,
            gltf::mesh::Mode::Triangles => Mode::Triangle,
            gltf::mesh::Mode::Points => Mode::Point,
            gltf::mesh::Mode::LineLoop => panic!("Not supported"),
            gltf::mesh::Mode::Lines => Mode::Line,
            gltf::mesh::Mode::LineStrip => Mode::LineStrip,
        };

        let material = primitive.material().index();
        // Load material if not yet present.
        if !assets.materials.contains_key(&material) {
            let new_material =
                Material::from_gltf(surface, &primitive.material(), import_data, assets);

            assets
                .materials
                .insert(primitive.material().index(), new_material);
        }

        let mut tess_builder = TessBuilder::new(surface)
            .set_mode(mode)
            .add_vertices(vertices);

        if let Some(indices) = indices {
            tess_builder = tess_builder.set_indices(indices);
        }

        let tess = tess_builder.build().unwrap();

        Self { tess, material }
    }
}
