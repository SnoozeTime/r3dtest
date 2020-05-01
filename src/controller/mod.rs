use crate::camera::{Camera, LookAt};
use crate::controller::client::ClientCommand;
use crate::ecs::Transform;
use crate::event::{Event, GameEvent};
use crate::gameplay::gun::{Gun, GunInventory};
use crate::gameplay::player::{Player, PlayerState};
use crate::physics::{BodyIndex, BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, error, info, trace};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
pub mod client;
pub mod fps;
pub mod free;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Fps {
    pub speed: f32,
    pub air_speed: f32,
    pub sensitivity: f32,

    #[serde(skip)]
    pub jumping: bool,

    #[serde(skip)]
    pub on_ground: bool,

    #[serde(skip)]
    pub moving: bool,
}

impl Fps {
    pub fn get_speed(&self) -> f32 {
        if self.on_ground {
            self.speed
        } else {
            self.air_speed
        }
    }
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
        //        ClientCommand::LookAt(pitch, yaw) => {
        //            let mut camera = world.get_mut::<Camera>(e).unwrap();
        //            let mut lookat = world.get_mut::<LookAt>(e).unwrap();
        //            camera.pitch = pitch;
        //            camera.yaw = yaw;
        //            camera.compute_vectors();
        //            lookat.0 = camera.front;
        //        }
        //        ClientCommand::CameraMoved => {
        //            let rb = world.get::<RigidBody>(e).unwrap();
        //            let t = world.get::<Transform>(e).unwrap();
        //            physics.set_rotation(rb.handle.unwrap(), *t);
        //        }
        ClientCommand::Move(dir) => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let mut fps = world.get_mut::<Fps>(e).unwrap();
            let h = rb.handle.unwrap();
            fps.moving = true;
            let speed = fps.get_speed();
            physics.add_velocity_change(h, dir.normalize() * speed);
        }
        ClientCommand::Jump => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let mut fps = world.get_mut::<Fps>(e).unwrap();

            if fps.on_ground {
                trace!("JUMP");
                // 10.0 for hiiiiiigh jump
                physics.add_velocity_change(rb.handle.unwrap(), 1.5 * glam::Vec3::unit_y());
                fps.jumping = true;
            }
        }
        ClientCommand::Shoot => {
            // let camera = world.get::<Camera>(e).unwrap();
            let rb = world.get::<RigidBody>(e).unwrap();
            let t = world.get::<Transform>(e).unwrap();
            let directions = crate::geom::quat_to_direction(t.rotation);
            if let Ok(mut gun) = world.get_mut::<Gun>(e) {
                if gun.can_shoot() {
                    gun.shoot();
                    let h = rb.handle.unwrap();

                    let mut d = physics.raycast(h, t.translation, directions.0);
                    trace!("{:?}", d);
                    d.sort_by(|(toi, _), (toi_o, _)| toi.partial_cmp(toi_o).unwrap());
                    if let Some(ev) = create_shot_event(d, resources, directions.0) {
                        let mut event_channel =
                            resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                        event_channel.single_write(ev);
                    }
                }
            } else {
                error!("Cannot shoot without a gun");
            }
        }
        ClientCommand::ChangeGun(gun_slot) => {
            match (world.get_mut::<GunInventory>(e), world.get_mut::<Gun>(e)) {
                (Ok(mut inventory), Ok(mut gun)) => {
                    if let Some(new_gun) = inventory.switch_gun(*gun, gun_slot) {
                        trace!("Will change to gun slot {}", gun_slot);

                        *gun = new_gun;
                        let mut event_channel =
                            resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                        event_channel.single_write(GameEvent::GunChanged);
                    }
                }
                _ => (),
            }
        }
        _ => (),
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
        for (_, (fps, rb, t)) in world.query::<(&mut Fps, &RigidBody, &Transform)>().iter() {
            let h = rb.handle.unwrap();
            let on_ground = {
                let mut d = physics.raycast(h, t.translation, -glam::Vec3::unit_y());
                d.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

                trace!("Raycast on_ground = {:?}", d);
                if let Some((minimum_distance, _)) = d.first() {
                    trace!("Minimum distance = {}", minimum_distance);
                    if *minimum_distance < 1.5 {
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
                //physics.set_friction(h, 10.0);
                fps.jumping = false;
            }
            if !fps.moving && on_ground {
                let mut vel = physics.get_linear_velocity(h).unwrap();
                vel.set_y(0.0);

                physics.add_velocity_change(h, -rb.linear_damping * vel);
            }

            fps.moving = false;
        }
    }
}

fn create_shot_event(
    raycast_result: Vec<(f32, BodyIndex)>,
    resources: &Resources,
    direction: glam::Vec3,
) -> Option<GameEvent> {
    raycast_result
        .iter()
        .map(|(_, h)| {
            info!("Body to entity");
            let body_to_entity = resources.fetch::<BodyToEntity>().unwrap();
            info!("Get entity");
            let entity = body_to_entity.get(&h).unwrap();
            GameEvent::EntityShot {
                entity: *entity,
                dir: direction,
            }
        })
        .next()
}
