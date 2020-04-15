use crate::camera::Camera;
use crate::controller::client::ClientCommand;
use crate::controller::Fps;
use crate::ecs::Transform;
use crate::event::Event;
use crate::gameplay::player::Player;
use crate::physics::{PhysicWorld, RigidBody};
use hecs::{Entity, World};
use log::debug;
use std::collections::HashMap;
use std::time::Duration;

/// Controller for the FPS players. (and maybe enemies). The Rigid body dynamic physics
/// is too floaty and hard to control
#[derive(Default)]
pub struct FpsController {
    commands: HashMap<Entity, Vec<ClientCommand>>,
}

impl FpsController {
    pub fn apply_commands(&mut self, inputs: &Vec<(Entity, Event)>) {
        self.commands.clear();
        for (e, c) in inputs {
            if !self.commands.contains_key(e) {
                self.commands.insert(*e, vec![]);
            }
            if let Event::Client(cmd) = c {
                self.commands.get_mut(e).unwrap().push(cmd.clone());
            }
        }
    }

    pub fn update(&self, world: &mut World, physics: &mut PhysicWorld, dt: Duration) {
        for (e, (t, rb, fps, _player, cam)) in world
            .query::<(&Transform, &RigidBody, &Fps, &Player, &Camera)>()
            .iter()
        {
            let h = rb.handle.unwrap();
            let empty = vec![];
            let entity_commands = self.commands.get(&e).unwrap_or(&empty);
            let (jump, forward, lateral) = FpsController::process_commands(entity_commands);

            let mut vel = physics.get_linear_velocity(h).unwrap();

            let contacts = physics.contact_with(h);
            let mut on_ground = false;
            let mut direction_normal = glam::Vec3::unit_y();
            if let Some(contacts) = contacts {
                for contact in contacts.iter() {
                    vel += -vel.dot(contact.0) * contact.0.normalize();
                    on_ground = true;
                    //direction_normal = contact.0;
                }
            }

            // Raycast in the direction we are going.
            let forward_dir = direction_normal.cross(cam.left);
            let lateral_dir = cam.left;

            let mut can_move = true;
            if lateral != 0.0 || forward != 0.0 {
                // first ray should be at player position + ray * offset.
                // second ray should be at first pos + ray.left
                let ray_dir = (lateral * lateral_dir + forward * forward_dir);
                let vel_along_direction = ray_dir.dot(vel);
                debug!("Velocity along direction = {:?}", vel_along_direction);
                let ray_left = ray_dir.cross(glam::Vec3::unit_y());

                let center_position = t.translation + 1.0 * ray_dir;
                let left_position = center_position + 1.0 * ray_left;
                let right_position = center_position - 1.0 * ray_left;
                let raycast_result = physics.raycast(h, center_position, ray_dir);
                debug!("First ray = {:?}", raycast_result);
                for result in raycast_result {
                    if result.0 <= 1.0 {
                        can_move = false;
                        break;
                    }
                }

                let raycast_result = physics.raycast(h, left_position, ray_dir);
                debug!("Second ray = {:?}", raycast_result);

                for result in raycast_result {
                    if result.0 <= 1.0 {
                        can_move = false;
                        break;
                    }
                }
                let raycast_result = physics.raycast(h, right_position, ray_dir);
                debug!("Third ray = {:?}", raycast_result);
                for result in raycast_result {
                    if result.0 <= 1.0 {
                        can_move = false;
                        break;
                    }
                }
            }

            // Gravity.
            if !on_ground {
                vel += -3.0 * glam::Vec3::unit_y() * dt.as_secs_f32();
            }

            if can_move {
                vel += forward_dir * forward * fps.speed + lateral_dir * lateral * fps.speed;
            } else {
                vel -= (forward_dir + lateral_dir).dot(vel) * (forward_dir + lateral_dir);
            }

            if jump && on_ground {
                vel += 1.0 * glam::Vec3::unit_y();
            }

            // damping when not moving
            if forward == 0.0 {
                vel -= forward_dir.dot(vel) * forward_dir * rb.linear_damping;
            }
            if lateral == 0.0 {
                vel -= lateral_dir.dot(vel) * lateral_dir * rb.linear_damping;
            }

            // max speed
            if vel.length() >= rb.max_linear_velocity * 0.95 {
                vel = vel.normalize() * rb.max_linear_velocity * 0.95;
            }

            physics.set_linear_velocity(h, vel);
        }
    }

    fn process_commands(entity_commands: &Vec<ClientCommand>) -> (bool, f32, f32) {
        let mut jump = false;
        let mut forward = 0.0;
        let mut lateral = 0.0;

        for cmd in entity_commands {
            match cmd {
                ClientCommand::Jump => jump = true,
                ClientCommand::Forward(axis_forward) => forward = *axis_forward,
                ClientCommand::Lateral(axis_lateral) => lateral = *axis_lateral,
                _ => (),
            }
        }

        (jump, forward, lateral)
    }
}
