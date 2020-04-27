use r3dtest::render::mesh::scene::Scene;

use luminance_glfw::{GlfwSurface, WindowDim, WindowOpt};
use luminance_windowing::Surface;
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
}
