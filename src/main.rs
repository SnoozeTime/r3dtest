use luminance_glfw::{Action, GlfwSurface, Key, Surface, WindowDim, WindowOpt};
use std::process::exit;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use log::{debug, error, info};
use serde_derive::{Deserialize, Serialize};

use luminance_windowing::CursorMode;
use std::fs::{self};

use r3dtest::animation::AnimationSystem;
use r3dtest::controller::fps::FpsController;
use r3dtest::controller::free::FreeController;
use r3dtest::controller::{client, Controller};
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
use r3dtest::render::assets::AssetManager;
use r3dtest::render::debug::update_debug_components;
use r3dtest::render::Renderer;
use r3dtest::{
    ecs, ecs::Transform, event::GameEvent, input::Input, physics::RigidBody, resources::Resources,
};
use shrev::EventChannel;
use std::thread;

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

fn setup_resources() -> Resources {
    let mut resources = Resources::default();
    let event_channel: EventChannel<GameEvent> = EventChannel::new();
    resources.insert(event_channel);
    let input = Input::new();
    resources.insert(input);
    resources
}

enum ControllerMode {
    Player,
    Free,
}

fn main_loop(mut surface: GlfwSurface, map_name: String) {
    let mut physics = PhysicWorld::default();

    // SETUP WORLD.
    let (mut loader, mut world) = WorldLoader::new(format!(
        "{}world/{}",
        std::env::var("ASSET_PATH").unwrap(),
        map_name
    ));
    //let mut world = ecs::serialization::deserialize_world(world_str).unwrap();

    let mut body_to_entity = BodyToEntity::default();
    // add the rigid bodies to the simulation.
    for (e, (t, mut rb)) in world.query::<(&Transform, &mut RigidBody)>().iter() {
        let id = physics.add_body(&t, &mut rb);
        body_to_entity.insert(id, e);
    }

    let mut resources = setup_resources();
    resources.insert(body_to_entity);
    let asset_manager = AssetManager::new(&mut surface);
    resources.insert(asset_manager);

    let player_entity = spawn_player(&mut world, &mut physics, &resources);
    world.insert_one(player_entity, MainPlayer).unwrap();

    let mut garbage_collector = GarbageCollector::new(&mut resources);
    let mut health_system = HealthSystem::new(&mut resources);
    let controller = Controller;
    let mut renderer = Renderer::new(&mut surface, &mut resources);
    let mut ui_system = UiSystem::new(&mut world, &mut resources);
    let mut player_system = PlayerSystem::new(&mut resources);
    let mut animation_system = AnimationSystem;
    let pickup_system = PickUpSystem;
    let mut gun_system = GunSystem::new(&mut resources);

    let dt = Duration::from_millis(16);

    let mut current_time = Instant::now();
    let client_controller = client::ClientController::get_offline_controller();
    //let mut fps_controller = FpsController::default();

    let mut controller_mode = ControllerMode::Player;
    let free_controller = FreeController;
    let main_player_entity = world
        .query::<&MainPlayer>()
        .iter()
        .map(|(e, _)| e)
        .next()
        .unwrap();
    'app: loop {
        {
            let mut input = resources.fetch_mut::<Input>().unwrap();
            input.process_events(&mut surface);
            if input.should_exit {
                break 'app;
            }
            if input.has_key_event_happened(Key::F1, Action::Press) {
                renderer.toggle_debug();
            }

            if input.has_key_event_happened(Key::F2, Action::Press) {
                // toggle controller.
                toggle_controller(&mut controller_mode, player_entity, &world, &mut physics);
            }
        }

        match controller_mode {
            ControllerMode::Player => {
                let cmds = client_controller
                    .process_input(&mut world, &mut resources)
                    .drain(..)
                    .map(|ev| (player_entity, Event::Client(ev)))
                    .collect();
                //fps_controller.apply_commands(&cmds);
                controller.apply_inputs(cmds, &mut world, &mut physics, &resources);
                controller.update(&mut world, &mut physics, &resources);
            }
            ControllerMode::Free => free_controller.process_input(&mut world, &mut resources),
        }

        renderer.update_view_matrix(&world);

        // ----------------------------------------------------
        // PHYSIC SIMULATION
        // ----------------------------------------------------

        let new_time = Instant::now();
        let frame_time = new_time - current_time;
        current_time = new_time;
        physics.step(frame_time.as_secs_f32());

        // Update the positions.
        for (_, (mut t, rb)) in world.query::<(&mut Transform, &RigidBody)>().iter() {
            if let Some(h) = rb.handle {
                if let Some(new_iso) = physics.get_isometry(h) {
                    t.translation = new_iso.translation;
                    t.rotation = new_iso.rotation;
                }
            }
        }
        renderer.update(&mut world, dt, &mut resources);

        // Update health if somebody has been SHOT.
        health_system.update(&mut world, &resources);
        ui_system.update(&mut world, &mut resources);
        player_system.update(dt, &mut world, &resources);
        animation_system.animate(&mut world);
        update_player_orientations(&mut world);
        update_debug_components(&mut world, &physics);
        gun_system.update(&mut world, dt, &mut resources);
        pickup_system.update(&world, &physics, &mut resources);
        //fps_controller.update(&mut world, &mut physics, dt);
        // ----------------------------------------------------
        // RENDERING
        // ----------------------------------------------------
        renderer.render(&mut surface, &world, &resources);

        // potential reload the world.
        loader.update(&mut world, &mut physics, &mut resources);
        // remove all old entities.
        garbage_collector.collect(&mut world, &mut physics, &resources);

        renderer.check_updates(&mut surface, &world, &resources);

        // FIXME
        let now = Instant::now();
        let frame_duration = now - current_time;
        if frame_duration < dt {
            thread::sleep(dt - frame_duration);
        }
        current_time = now;
    }
}

fn toggle_controller(
    current_controller_mode: &mut ControllerMode,
    player_entity: hecs::Entity,
    world: &hecs::World,
    physics: &mut PhysicWorld,
) {
    let new_mode = match current_controller_mode {
        ControllerMode::Player => {
            let rb = world.get::<RigidBody>(player_entity).unwrap();
            physics.remove_body(rb.handle.unwrap());
            ControllerMode::Free
        }
        ControllerMode::Free => {
            let mut rb = world.get_mut::<RigidBody>(player_entity).unwrap();
            let t = world.get::<Transform>(player_entity).unwrap();
            physics.add_body(&t, &mut rb);
            ControllerMode::Player
        }
    };
    *current_controller_mode = new_mode;
}
