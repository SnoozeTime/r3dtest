#[allow(unused_imports)]
use log::{debug, error, info};
use luminance_glfw::{GlfwSurface, Surface, WindowDim, WindowOpt};
use luminance_windowing::CursorMode;
use r3dtest::controller::client::ClientCommand;
use r3dtest::gameplay::delete::GarbageCollector;
use r3dtest::gameplay::player::spawn_player_ui;
use r3dtest::net::client::ClientSystem;
use r3dtest::render::Renderer;
use r3dtest::{
    camera::Camera, controller::client, ecs, event::GameEvent, input::Input, resources::Resources,
};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
use std::fs;
use std::net::SocketAddr;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    width: u32,
    height: u32,
}

fn main() {
    dotenv::dotenv().ok().unwrap();
    pretty_env_logger::init();

    let window_config = fs::read_to_string("config.ron").unwrap();
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
}

fn connection_loop(server_addr: SocketAddr) -> (hecs::World, hecs::Entity, ClientSystem) {
    // Tries to connect.
    let mut backend = ClientSystem::new(server_addr);

    // at this point, correctly connected to the server. Will loop until we get the latest state.
    let mut current_time = Instant::now();
    let dt = Duration::from_millis(30);

    let mut world = hecs::World::new();
    let entity;

    'connection_loop: loop {
        // send ping and latest state.
        backend.send_commands(&vec![]);

        // receive state from server.
        backend.poll_events(&mut world);

        // If there is a camera, we can stop.
        if let Some((e, _)) = world.query::<&Camera>().iter().next() {
            println!("Found a camera -> {:?}", e.to_bits());
            entity = e;
            break 'connection_loop;
        }

        let now = Instant::now();
        let frame_duration = now - current_time;
        if frame_duration < dt {
            thread::sleep(dt - frame_duration);
        }
        current_time = now;
    }

    (world, entity, backend)
}

fn setup_resources() -> Resources {
    let mut resources = Resources::default();
    let event_channel: EventChannel<GameEvent> = EventChannel::new();
    resources.insert(event_channel);
    let input = Input::default();
    resources.insert(input);
    resources
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientConfig {
    host: String,
}

fn main_loop(mut surface: GlfwSurface) {
    // 1. Create the surface and renderer.
    let mut renderer = Renderer::new(&mut surface);

    // 2. CONNECT TO THE SERVER!
    let conf_str = fs::read_to_string("client.ron").unwrap();
    let conf: ClientConfig = ron::de::from_str(&conf_str).unwrap();
    let (mut world, camera_entity, mut backend) = connection_loop(conf.host.parse().unwrap());

    let mut resources = setup_resources();

    let mut garbage_collector = GarbageCollector::new(&mut resources);

    let mut current_time = Instant::now();
    let dt = Duration::from_millis(16);

    fs::write(
        "online_client_before.ron",
        ecs::serialization::serialize_world(&world).unwrap(),
    )
    .unwrap();
    spawn_player_ui(&mut world);

    'app: loop {
        {
            let mut input = resources.fetch_mut::<Input>().unwrap();
            input.process_events(&mut surface);
            if input.should_exit {
                break 'app;
            }
        }
        let cmds = client::process_input(&mut world, &resources);
        backend.send_commands(&cmds);

        // State from the server - will update all positions and so on...
        backend.poll_events(&mut world);
        renderer.update_view_matrix(&world);

        // ----------------------------------------------------
        // RENDERING
        // ----------------------------------------------------
        renderer.render(&mut surface, &world);

        // remove all old entities.
        garbage_collector.collect_without_physics(&mut world, &resources);

        // FIXME
        let now = Instant::now();
        let frame_duration = now - current_time;
        if frame_duration < dt {
            thread::sleep(dt - frame_duration);
        }
        current_time = now;
    }

    fs::write(
        "online_client.ron",
        ecs::serialization::serialize_world(&world).unwrap(),
    )
    .unwrap();
}
