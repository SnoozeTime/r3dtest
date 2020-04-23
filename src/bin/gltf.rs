use r3dtest::render::mesh::{
    Vertex, VertexNormal, VertexPosition, VertexTangent, VertexTexCoord0, VertexTexCoord1,
};

use std::fs::File;
fn print_tree(node: &gltf::Node, depth: i32) {
    for _ in 0..(depth - 1) {
        print!("  ");
    }
    print!(" -");
    print!(" Node {}", node.index());
    #[cfg(feature = "names")]
    print!(" ({})", node.name().unwrap_or("<Unnamed>"));
    println!();
    for child in node.children() {
        print_tree(&child, depth + 1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("cube.gltf")?;
    let reader = std::io::BufReader::new(file);
    let gltf = gltf::Gltf::from_reader(reader)?;
    for skin in gltf.skins() {
        println!("{:?}", skin);
    }
    for m in gltf.meshes() {
        println!("primitive = {:?}", m.primitives());
    }
    for scene in gltf.scenes() {
        println!("Scene {}", scene.index());
        for node in scene.nodes() {
            println!("{:?}", node);
            print_tree(&node, 0);
        }
    }
    let import = gltf::import("cube.gltf")?;
    println!("import {:?}", import.1);
    Ok(())
}

fn read_buffer() -> Result<(), Box<dyn std::error::Error>> {
    //type Import = (Document, Vec<buffer::Data>, Vec<image::Data>);
    let import = gltf::import("cube.gltf")?;
    let buff = import.1;

    for node in import.0.nodes() {
        if let Some(name) = node.name() {
            if name == "Cube" {
                let (transform, rot, scale) = dbg!(node.transform().decomposed());
                if let Some(mesh) = node.mesh() {
                    println!("{:?}. {:?}", mesh.index(), mesh.name());
                    for (i, primitive) in mesh.primitives().enumerate() {
                        let buffers = &buff;
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                        let mut vertices = reader
                            .read_positions()
                            .unwrap_or_else(|| panic!("BIM"))
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

                        println!("{:?}", vertices);
                        println!("{:?}", indices);
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() {
    read_buffer().unwrap_or_default();
}
