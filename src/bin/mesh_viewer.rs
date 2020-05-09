use luminance_glfw::{Action, GlfwSurface, Key, Surface, WindowDim, WindowOpt};
use std::process::exit;
use std::time::{Duration, Instant};

use imgui::{Context, FontConfig, FontGlyphRanges, FontSource};
#[allow(unused_imports)]
use log::{debug, error, info};
use luminance_windowing::CursorMode;
use r3dtest::animation::AnimationSystem;
use r3dtest::assets::material::{AsyncMaterialLoader, Material, SyncMaterialLoader};
use r3dtest::assets::{mesh::SyncMeshLoader, AssetManager};
use r3dtest::camera::Camera;
use r3dtest::colors::RgbColor;
use r3dtest::controller::free::FreeController;
use r3dtest::controller::{client, Controller, Fps};
use r3dtest::ecs::WorldLoader;
use r3dtest::event::Event;
use r3dtest::gameplay::delete::GarbageCollector;
use r3dtest::gameplay::gun::GunSystem;
use r3dtest::gameplay::health::HealthSystem;
use r3dtest::gameplay::pickup::PickUpSystem;
use r3dtest::gameplay::player::{
    spawn_player, update_player_orientations, MainPlayer, PlayerSystem,
};
use r3dtest::gameplay::ui::UiSystem;
use r3dtest::physics::{BodyToEntity, PhysicWorld};
use r3dtest::render::debug::update_debug_components;
use r3dtest::render::lighting::{AmbientLight, DirectionalLight};
use r3dtest::render::mesh::mesh::Mesh;
use r3dtest::render::{Render, RenderConfig, Renderer};
use r3dtest::transform::HasChildren;
use r3dtest::{
    ecs::Transform, event::GameEvent, input::Input, physics::RigidBody, resources::Resources,
};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
use std::fs::{self};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    width: u32,
    height: u32,
}

fn main() {
    dotenv::dotenv().ok().unwrap();
    pretty_env_logger::init();

    let map_name: String = std::env::args().nth(1).unwrap_or("lol.ron".to_string());
    let window_config =
        fs::read_to_string(std::env::var("CONFIG_PATH").unwrap() + "config.ron").unwrap();
    let conf: WindowConfig = ron::de::from_str(&window_config).unwrap();
    let surface = GlfwSurface::new(
        WindowDim::Windowed(conf.width, conf.height),
        "Hello, World",
        WindowOpt::default().set_cursor_mode(CursorMode::Disabled),
    );

    match surface {
        Ok(surface) => {
            debug!("Will enter main loop");
            main_loop(surface, map_name);
        }
        Err(e) => {
            error!("Cannot create graphic surface: {}", e);
            exit(1)
        }
    }
    info!("Hello, world!");
}

fn load_optional_config<T: DeserializeOwned + 'static>(path: &str, resources: &mut Resources) {
    if let Ok(conf_str) = fs::read_to_string(std::env::var("CONFIG_PATH").unwrap() + path) {
        let conf: Result<T, _> = ron::de::from_str(&conf_str);
        if let Ok(conf) = conf {
            resources.insert(conf);
        } else {
            error!("Found render config but could not deserialize it.");
        }
    } else {
        info!("No config for Renderer. Will use default instead");
        resources.insert(RenderConfig::default());
    }
}

fn setup_resources() -> Resources {
    let mut resources = Resources::default();
    let event_channel: EventChannel<GameEvent> = EventChannel::new();
    resources.insert(event_channel);
    let input = Input::new();
    resources.insert(input);

    // optional renderer config.
    load_optional_config::<RenderConfig>("render.ron", &mut resources);

    resources
}

#[derive(Clone, Copy, Debug)]
enum ControllerMode {
    Player,
    Free,
    Editor,
}

fn main_loop(mut surface: GlfwSurface, map_name: String) {
    let mut world = hecs::World::new();

    let mut resources = setup_resources();

    let mut garbage_collector = GarbageCollector::new(&mut resources);

    let controller = Controller;
    let mut renderer = Renderer::new(&mut surface, &mut resources);

    let dt = Duration::from_millis(16);
    let mut controller_mode = ControllerMode::Free;
    let mut previous_controller_mode = ControllerMode::Free;
    let free_controller = FreeController;
    let mut imgui = Context::create();
    let font_size = 13.0;

    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../../assets/fonts/DejaVuSans.ttf"),
        size_pixels: font_size,
        config: Some(FontConfig {
            rasterizer_multiply: 1.75,
            glyph_ranges: FontGlyphRanges::default(),
            ..FontConfig::default()
        }),
    }]);

    let mut imgui_renderer = imgui_luminance::Renderer::new(&mut surface, &mut imgui);
    imgui.set_ini_filename(None);

    let size = surface.size();
    let mut editor = r3dtest::editor::Editor::new(size[0], size[1]);
    //
    //    let mut mesh_manager: AssetManager<Mesh> =
    //        AssetManager::from_loader(Box::new(SyncMeshLoader::new(&mut surface)));
    //    let mut material_manager: AssetManager<Material> =
    //        AssetManager::from_loader(Box::new(AsyncMaterialLoader::new()));
    //    mesh_manager.load("_simple_sphere_Sphere");
    //    material_manager.load("default_material");
    //    material_manager.load("material_Floor");
    //    resources.insert(mesh_manager);
    //    resources.insert(material_manager);
    r3dtest::assets::create_asset_managers(&mut surface, &mut resources);

    let free_camera = world.spawn((
        Transform::new(
            glam::vec3(0.0, 0.0, -1.0),
            glam::Quat::identity(),
            glam::Vec3::one(),
        ),
        Camera {
            active: true,
            pitch: 0.0,
            yaw: 0.0,
            front: glam::Vec3::zero(),
            left: glam::Vec3::zero(),
        },
        Fps {
            sensitivity: 0.004,
            ..Fps::default()
        },
    ));

    // a sphere
    world.spawn((
        Transform::default(),
        Render {
            mesh: "_simple_sphere_Sphere".to_string(),
            enabled: true,
        },
    ));

    // some lights
    world.spawn((AmbientLight {
        color: r3dtest::colors::PASTEL_RED,
        intensity: 0.2,
    },));
    world.spawn((DirectionalLight {
        direction: glam::vec3(1.0, 11.0, 1.0),
        color: r3dtest::colors::PASTEL_RED,
        intensity: 0.2,
    },));

    'app: loop {
        {
            let mut input = resources.fetch_mut::<Input>().unwrap();
            if let ControllerMode::Editor = controller_mode {
                input.process_events_with_editor(&mut surface, imgui.io_mut(), &imgui_renderer);
            } else {
                input.process_events(&mut surface);
            }
            if input.should_exit {
                break 'app;
            }
            if input.has_key_event_happened(Key::Enter, Action::Press) {
                editor_mode(
                    &mut surface,
                    &mut controller_mode,
                    &mut previous_controller_mode,
                );
            }
        }

        if let ControllerMode::Free = controller_mode {
            free_controller.process_input(&mut world, &mut resources, free_camera);
        }
        renderer.update_view_matrix(&world);
        r3dtest::transform::update_transforms(&mut world);
        renderer.update(&mut world, dt, &mut resources);

        // ----------------------------------------------------
        // RENDERING
        // ----------------------------------------------------
        // render the editor.
        let ui = imgui.frame();
        editor.show_components(&ui, &world, &mut resources);
        let draw_data = ui.render();
        imgui_renderer.prepare(&mut surface, draw_data);
        renderer.render(
            &mut surface,
            &world,
            &resources,
            if let ControllerMode::Editor = controller_mode {
                Some((&imgui_renderer, &draw_data))
            } else {
                None
            },
        );

        // remove all old entities.
        garbage_collector.collect_without_physics(&mut world, &resources);
        renderer.check_updates(&mut surface, &mut world, &resources, None);
        surface.swap_buffers();

        // Upload the materials to GPU when available.
        {
            let mut mat_mgr = resources.fetch_mut::<AssetManager<Material>>().unwrap();
            mat_mgr.upload_all(&mut surface);
        }
    }
}

fn editor_mode(
    surface: &mut GlfwSurface,
    current_controller_mode: &mut ControllerMode,
    previous_controller_mode: &mut ControllerMode,
) {
    match current_controller_mode {
        ControllerMode::Player | ControllerMode::Free => {
            surface.set_cursor_mode(CursorMode::Visible);
            *previous_controller_mode = *current_controller_mode;
            *current_controller_mode = ControllerMode::Editor;
        }
        _ => {
            surface.set_cursor_mode(CursorMode::Disabled);
            *current_controller_mode = *previous_controller_mode;
            *previous_controller_mode = ControllerMode::Editor;
        }
    };
}
