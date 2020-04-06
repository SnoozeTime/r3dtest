use luminance_glfw::{GlfwSurface, Key, Surface, WindowDim, WindowOpt};
use std::process::exit;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use log::{debug, error, info};
use serde_derive::{Deserialize, Serialize};

use luminance_windowing::CursorMode;
use std::fs::{self};

use r3dtest::animation::AnimationSystem;
use r3dtest::colors::RgbColor;
use r3dtest::controller::{client, Controller};
use r3dtest::event::Event;
use r3dtest::gameplay::delete::GarbageCollector;
use r3dtest::gameplay::health::HealthSystem;
use r3dtest::gameplay::player::{spawn_player, spawn_player_ui, MainPlayer, PlayerSystem};
use r3dtest::gameplay::ui::UiSystem;
use r3dtest::physics::{BodyToEntity, PhysicWorld};
use r3dtest::render::sprite::ScreenPosition;
use r3dtest::render::text::Text;
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
            main_loop(surface);
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
    let input = Input::default();
    resources.insert(input);
    resources
}

fn main_loop(mut surface: GlfwSurface) {
    let mut current_time = Instant::now();

    let mut physics = PhysicWorld::default();

    // SETUP WORLD.
    let world_str = fs::read_to_string(&format!(
        "{}{}",
        std::env::var("ASSET_PATH").unwrap(),
        "world/lol.ron"
    ))
    .unwrap();
    let mut world = ecs::serialization::deserialize_world(world_str).unwrap();

    let mut body_to_entity = BodyToEntity::default();
    // add the rigid bodies to the simulation.
    for (e, (t, mut rb)) in world.query::<(&Transform, &mut RigidBody)>().iter() {
        let id = physics.add_body(t.translation, &mut rb);
        body_to_entity.insert(id, e);
    }

    let mut resources = setup_resources();
    resources.insert(body_to_entity);

    let player_entity = spawn_player(&mut world, &mut physics, &resources);
    world.insert(player_entity, (MainPlayer,)).unwrap();

    let mut garbage_collector = GarbageCollector::new(&mut resources);
    let mut health_system = HealthSystem::new(&mut resources);
    let controller = Controller;
    let mut renderer = Renderer::new(&mut surface, &mut resources);
    let mut ui_system = UiSystem::new(&mut world, &mut resources);
    let mut player_system = PlayerSystem::new(&mut resources);
    let mut animation_system = AnimationSystem;

    let dt = Duration::from_millis(16);

    let mut current_time = Instant::now();

    'app: loop {
        {
            let mut input = resources.fetch_mut::<Input>().unwrap();
            input.process_events(&mut surface);
            if input.should_exit {
                break 'app;
            }
        }

        let cmds = client::process_input(&mut world, &mut resources)
            .drain(..)
            .map(|ev| (player_entity, Event::Client(ev)))
            .collect();
        controller.apply_inputs(cmds, &mut world, &mut physics, &resources);
        controller.update(&mut world, &mut physics, &resources);
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
                let new_position = physics.get_pos(h).unwrap();
                t.translation = new_position;
            }
        }
        renderer.update_view_matrix(&world);

        // Update health if somebody has been SHOT.
        health_system.update(&world, &resources);
        ui_system.update(&mut world, &mut resources);
        player_system.update(dt, &mut world, &resources);
        animation_system.animate(&mut world);

        // ----------------------------------------------------
        // RENDERING
        // ----------------------------------------------------
        renderer.render(&mut surface, &world);

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
