use crate::camera::{Camera, Direction};
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::input::Input;
use crate::physics::{BodyIndex, BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use log::info;
use luminance_glfw::{Key, MouseButton};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
use std::collections::HashMap;

pub struct FpsController;

pub enum ControllerEvent {
    KeyDown(Key),
    KeyUp(Key),
    Jump,
    Shoot,
}

impl FpsController {
    pub fn init(&self, world: &mut hecs::World, physics: &mut PhysicWorld) {
        if let Some((e, (fps, rb))) = world.query::<(&Fps, &RigidBody)>().iter().next() {
            // we landed.
            physics.set_friction(rb.handle.unwrap(), 0.1);
        }
    }

    pub fn update(
        &self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) {
        if let Some((e, (fps, camera, transform, rb))) = world
            .query::<(&mut Fps, &mut Camera, &Transform, &mut RigidBody)>()
            .iter()
            .next()
        {
            let h = rb.handle.unwrap();
            let input = resources.fetch::<Input>().unwrap();

            // force to apply to the body this frame.
            let mut lateral_dir = {
                if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
                    Some(camera.left)
                } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
                    Some(-camera.left)
                } else {
                    None
                }
            };
            let mut forward_dir = {
                if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
                    Some(camera.left.cross(glam::Vec3::unit_y()))
                } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
                    Some(-camera.left.cross(glam::Vec3::unit_y()))
                } else {
                    None
                }
            };

            let direction = match (forward_dir, lateral_dir) {
                (Some(fd), Some(ld)) => Some((fd + ld).normalize()),
                (Some(fd), None) => Some(fd),
                (None, Some(ld)) => Some(ld),
                _ => None,
            };
            if let Some(d) = direction {
                physics.add_velocity_change(h, d * fps.speed);
            }

            // orientation of camera.
            if let Some((offset_x, offset_y)) = input.mouse_delta {
                self.apply_delta_dir(offset_x, offset_y, camera, fps.sensitivity);
            }

            let on_ground = {
                let mut d = physics.raycast(h, glam::vec3(0.0, -1.0, 0.0), -glam::Vec3::unit_y());
                d.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

                if let Some((minimum_distance, _)) = d.pop() {
                    if minimum_distance < 0.1 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if fps.jumping && on_ground {
                // we landed.
                physics.set_friction(rb.handle.unwrap(), 0.1);
                fps.jumping = false;
            }

            // Jump
            // ------------------------------------------------------------------------------------
            if input.has_key_down(Key::Space) && on_ground {
                physics.add_velocity_change(rb.handle.unwrap(), 20.0 * glam::Vec3::unit_y());
                fps.jumping = true;
                physics.set_friction(rb.handle.unwrap(), 0.0);
            }

            // SHOOT
            // ------------------------------------------------------------------------------------
            if input.is_mouse_down(MouseButton::Button1) {
                let mut d = physics.raycast(h, glam::vec3(0.0, -1.0, 0.0), camera.front);
                println!("{:?}", d);
                if let Some(ev) = create_shot_event(d, resources) {
                    let mut event_channel =
                        resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                    event_channel.single_write(ev);
                }
            }
        }
    }

    pub fn update_controller(
        &self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        events: Vec<ControllerEvent>,
        resources: &Resources,
    ) {
        if let Some((e, (fps, camera, transform, rb))) = world
            .query::<(&mut Fps, &mut Camera, &Transform, &mut RigidBody)>()
            .iter()
            .next()
        {
            let h = rb.handle.unwrap();
            let input = resources.fetch::<Input>().unwrap();

            // force to apply to the body this frame.
            let mut lateral_dir = {
                if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
                    Some(camera.left)
                } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
                    Some(-camera.left)
                } else {
                    None
                }
            };
            let mut forward_dir = {
                if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
                    Some(camera.left.cross(glam::Vec3::unit_y()))
                } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
                    Some(-camera.left.cross(glam::Vec3::unit_y()))
                } else {
                    None
                }
            };

            let direction = match (forward_dir, lateral_dir) {
                (Some(fd), Some(ld)) => Some((fd + ld).normalize()),
                (Some(fd), None) => Some(fd),
                (None, Some(ld)) => Some(ld),
                _ => None,
            };
            if let Some(d) = direction {
                physics.add_velocity_change(h, d * fps.speed);
            }

            // orientation of camera.
            if let Some((offset_x, offset_y)) = input.mouse_delta {
                self.apply_delta_dir(offset_x, offset_y, camera, fps.sensitivity);
            }

            let on_ground = {
                let mut d = physics.raycast(h, glam::vec3(0.0, -1.0, 0.0), -glam::Vec3::unit_y());
                d.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

                if let Some((minimum_distance, _)) = d.pop() {
                    if minimum_distance < 0.1 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if fps.jumping && on_ground {
                // we landed.
                physics.set_friction(rb.handle.unwrap(), 0.1);
                fps.jumping = false;
            }

            // Jump
            // ------------------------------------------------------------------------------------
            if input.has_key_down(Key::Space) && on_ground {
                physics.add_velocity_change(rb.handle.unwrap(), 20.0 * glam::Vec3::unit_y());
                fps.jumping = true;
                physics.set_friction(rb.handle.unwrap(), 0.0);
            }

            // SHOOT
            // ------------------------------------------------------------------------------------
            if input.is_mouse_down(MouseButton::Button1) {
                let mut d = physics.raycast(h, glam::vec3(0.0, -1.0, 0.0), camera.front);
                println!("{:?}", d);
                if let Some(ev) = create_shot_event(d, resources) {
                    let mut event_channel =
                        resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                    event_channel.single_write(ev);
                }
            }
        }
    }

    fn apply_delta_dir(&self, offset_x: f32, offset_y: f32, camera: &mut Camera, sensitivity: f32) {
        camera.yaw += offset_x * sensitivity;
        camera.pitch += offset_y * sensitivity;
        if camera.pitch >= 89.0 {
            camera.pitch = 89.0;
        }
        if camera.pitch <= -89.0 {
            camera.pitch = -89.0;
        }

        camera.compute_vectors();
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
