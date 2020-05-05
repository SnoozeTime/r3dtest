#![allow(warnings)]
use r3dtest::ecs::Transform;
use r3dtest::render::mesh::scene::Scene;
use r3dtest::render::Render;

use luminance_glfw::{GlfwSurface, WindowDim, WindowOpt};
use luminance_windowing::Surface;
use r3dtest::ecs::serialization::SerializedEntity;
use std::fs;
use std::fs::File;
fn main() {
    dotenv::dotenv().ok().unwrap();
    pretty_env_logger::init();

    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(100, 100),
        "Hello, World",
        WindowOpt::default(),
    )
    .unwrap();

    let asset_path = std::env::args().nth(1).unwrap(); //;std::env::var("ASSET_PATH").unwrap() + "material.gltf";
    println!("Will import from {}", asset_path);
    let import = gltf::import(asset_path).unwrap();
    let g_scene = import.0.scenes().next().unwrap();
    let scene = Scene::from_gltf(&mut surface, &g_scene, &import);

    println!("scene materials = {:?}", scene.assets.materials);
    println!("meshes = {:?}", scene.assets.meshes.keys());

    let serialized: Vec<_> = scene
        .nodes
        .iter()
        .map(|node| {
            let t = node.transform;
            let mut ser = SerializedEntity::default();
            ser.transform = Some(t);

            if let Some(ref m) = node.mesh_id {
                ser.render = Some(Render {
                    mesh: m.clone(),
                    enabled: true,
                });
            }

            ser
        })
        .collect();

    std::fs::write(
        "benoit.ron",
        ron::ser::to_string_pretty(&serialized, ron::ser::PrettyConfig::default()).unwrap(),
    );
}
