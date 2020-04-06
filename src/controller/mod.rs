use crate::camera::Camera;
use crate::controller::client::ClientCommand;
use crate::event::{Event, GameEvent};
use crate::gameplay::player::{Player, PlayerState};
use crate::physics::{BodyIndex, BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;

pub mod client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fps {
    pub speed: f32,
    pub sensitivity: f32,

    #[serde(skip)]
    pub jumping: bool,

    #[serde(skip)]
    pub on_ground: bool,
}

pub fn apply_inputs(
    inputs: Vec<(Entity, Event)>,
    world: &mut hecs::World,
    physics: &mut PhysicWorld,
    resources: &Resources,
) {
    for (e, ev) in inputs {
        // don't do anything if player is not alive.
        let can_process = {
            let p = world
                .get::<Player>(e)
                .expect("Player entity should have a player component");
            if let PlayerState::Alive = p.state {
                true
            } else {
                false
            }
        };
        if can_process {
            match ev {
                Event::Client(cmd) => {
                    apply_cmd(e, cmd, world, physics, resources);
                }
                _ => (),
            }
        }
    }
}

fn apply_cmd(
    e: Entity,
    cmd: ClientCommand,
    world: &mut hecs::World,
    physics: &mut PhysicWorld,
    resources: &Resources,
) {
    match cmd {
        ClientCommand::LookAt(pitch, yaw) => {
            let mut camera = world.get_mut::<Camera>(e).unwrap();
            camera.pitch = pitch;
            camera.yaw = yaw;
            camera.compute_vectors();
        }
        ClientCommand::Move(dir) => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let fps = world.get::<Fps>(e).unwrap();
            let h = rb.handle.unwrap();
            physics.add_velocity_change(h, dir.normalize() * fps.speed);
        }
        ClientCommand::Jump => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let mut fps = world.get_mut::<Fps>(e).unwrap();

            if fps.on_ground {
                info!("JUMP");
                physics.add_velocity_change(rb.handle.unwrap(), 20.0 * glam::Vec3::unit_y());
                fps.jumping = true;
                physics.set_friction(rb.handle.unwrap(), 0.0);
            }
        }
        ClientCommand::Shoot => {
            let camera = world.get::<Camera>(e).unwrap();
            let rb = world.get::<RigidBody>(e).unwrap();
            let h = rb.handle.unwrap();
            debug!("CAMERA IS {:?}", *camera);
            debug!(
                "Will raycast from {:?} direction {:?}",
                physics.get_pos(h),
                camera.front
            );

            let mut d = physics.raycast(h, glam::vec3(0.0, 0.0, 0.0), camera.front);
            debug!("{:?}", d);
            d.sort_by(|(toi, _), (toi_o, _)| toi.partial_cmp(toi_o).unwrap());
            if let Some(ev) = create_shot_event(d, resources) {
                let mut event_channel = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                event_channel.single_write(ev);
            }
        }
    }
}

pub struct Controller;

impl Controller {
    pub fn apply_inputs(
        &self,
        inputs: Vec<(Entity, Event)>,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) {
        for (e, ev) in inputs {
            match ev {
                Event::Client(cmd) => {
                    apply_cmd(e, cmd, world, physics, resources);
                }
                _ => (),
            }
        }
    }

    /// Check at each frames if the body is on ground.
    pub fn update(
        &self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        _resources: &Resources,
    ) {
        for (_, (fps, rb)) in world.query::<(&mut Fps, &RigidBody)>().iter() {
            let h = rb.handle.unwrap();
            let on_ground = {
                let mut d = physics.raycast(h, glam::vec3(0.0, 0.0, 0.0), -glam::Vec3::unit_y());
                d.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

                debug!("Raycast on_ground = {:?}", d);
                if let Some((minimum_distance, _)) = d.pop() {
                    if minimum_distance < 2.5 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if on_ground {
                trace!(" NOW ON GROUND!");
            }
            fps.on_ground = on_ground;

            if fps.jumping && on_ground {
                physics.set_friction(h, 0.1);
                fps.jumping = false;
            }
        }
    }
}

fn create_shot_event(
    raycast_result: Vec<(f32, BodyIndex)>,
    resources: &Resources,
) -> Option<GameEvent> {
    raycast_result
        .iter()
        .map(|(_, h)| {
            info!("Body to entity");
            let body_to_entity = resources.fetch::<BodyToEntity>().unwrap();
            info!("Get entity");
            let entity = body_to_entity.get(&h).unwrap();
            GameEvent::EntityShot { entity: *entity }
        })
        .next()
}
