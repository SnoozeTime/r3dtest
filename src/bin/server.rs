#![allow(warnings)]
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use log::{debug, error, info};
use serde_derive::{Deserialize, Serialize};

use std::fs;

use r3dtest::controller::Controller;
use r3dtest::gameplay::delete::GarbageCollector;
use r3dtest::gameplay::gun::GunSystem;
use r3dtest::gameplay::health::HealthSystem;
use r3dtest::gameplay::pickup::PickUpSystem;
use r3dtest::gameplay::player::PlayerSystem;
use r3dtest::net::server::NetworkSystem;
use r3dtest::physics::{BodyToEntity, PhysicWorld};
use r3dtest::render::debug::update_debug_components;
use r3dtest::{ecs, ecs::Transform, event::GameEvent, physics::RigidBody, resources::Resources};
use shrev::EventChannel;
use std::net::SocketAddr;
use std::thread;

fn setup_resources() -> Resources {
    let mut resources = Resources::default();
    let event_channel: EventChannel<GameEvent> = EventChannel::new();
    resources.insert(event_channel);
    resources
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerConfig {
    frame_step: u64, // milliseconds
    host: String,
    world: String,
}

fn main() {
    dotenv::dotenv().ok().unwrap();
    pretty_env_logger::init();

    let server_config: ServerConfig = {
        let conf_str =
            fs::read_to_string(std::env::var("CONFIG_PATH").unwrap() + "server.ron").unwrap();
        ron::de::from_str(&conf_str).unwrap()
    };

    // TODO CREATE SERVER
    //let server_addr: SocketAddr = "127.0.0.1:13466".parse().unwrap();
    let server_addr: SocketAddr = server_config.host.parse().unwrap();
    let mut backend = NetworkSystem::new(server_addr);

    let dt = Duration::from_millis(server_config.frame_step);

    let mut current_time = Instant::now();
    let mut physics = PhysicWorld::default();

    let world_str = fs::read_to_string(format!(
        "{}{}",
        std::env::var("ASSET_PATH").unwrap(),
        server_config.world
    ))
    .unwrap();
    let mut world = ecs::serialization::deserialize_world(world_str).unwrap();

    let mut body_to_entity = BodyToEntity::default();

    // add the rigid bodies to the simulation.
    for (e, (t, mut rb)) in world.query::<(&Transform, &mut RigidBody)>().iter() {
        let id = physics.add_body(&t, &mut rb);
        body_to_entity.insert(id, e);
    }
    let mut resources = setup_resources();
    resources.insert(body_to_entity);

    let mut garbage_collector = GarbageCollector::new(&mut resources);
    let mut health_system = HealthSystem::new(&mut resources);
    let controller = Controller;

    fs::write(
        "online_server.ron",
        ecs::serialization::serialize_world(&world).unwrap(),
    )
    .unwrap();

    let mut player_system = PlayerSystem::new(&mut resources);
    let mut gun_system = GunSystem::new(&mut resources);
    let pickup_system = PickUpSystem;
    'app: loop {
        let client_events = backend.poll_events(&mut world, &mut physics, &resources);
        controller.apply_inputs(client_events, &mut world, &mut physics, &resources);

        // GAMEPLAY UPDATE
        // ----------------------------------------------------
        controller.update(&mut world, &mut physics, &resources);

        // ----------------------------------------------------
        // PHYSIC SIMULATION
        // ----------------------------------------------------

        let new_time = Instant::now();
        let frame_time = new_time - current_time;
        current_time = new_time;
        physics.step();

        // Update the positions.
        for (_, (mut t, rb)) in world.query::<(&mut Transform, &RigidBody)>().iter() {
            if let Some(h) = rb.handle {
                let new_position = physics.get_pos(h).unwrap();
                t.translation = new_position;
            }
        }

        // Update health if somebody has been SHOT.
        health_system.update(&mut world, &resources);
        player_system.update(dt, &mut world, &resources);
        update_debug_components(&mut world, &physics);
        pickup_system.update(&world, &physics, &mut resources);
        gun_system.update(&mut world, dt, &mut resources);
        // remove all old entities.
        garbage_collector.collect(&mut world, &mut physics, &resources);

        backend.send_state(&mut world, &resources);

        // FIXME
        let now = Instant::now();
        let frame_duration = now - current_time;
        if frame_duration < dt {
            thread::sleep(dt - frame_duration);
        }
        current_time = now;
    }
}
