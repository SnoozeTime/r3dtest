use super::Fps;
use crate::camera::Camera;
use crate::event::GameEvent;
use crate::gameplay::player::{MainPlayer, Player, PlayerState};
use crate::input::Input;
use crate::resources::Resources;
use luminance_glfw::{Action, Key, MouseButton};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ClientCommand {
    Move(glam::Vec3),
    LookAt(f32, f32), // pitch and yaw
    Jump,
    Shoot,
}

pub fn process_input(world: &mut hecs::World, resources: &mut Resources) -> Vec<ClientCommand> {
    let mut commands = vec![];

    if let Some((_e, (fps, camera, _, p))) = world
        .query::<(&mut Fps, &mut Camera, &MainPlayer, &Player)>()
        .iter()
        .next()
    {
        let input = resources.fetch::<Input>().unwrap();

        if let PlayerState::Alive = p.state {
            let lateral_dir = {
                if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
                    Some(camera.left)
                } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
                    Some(-camera.left)
                } else {
                    None
                }
            };
            let forward_dir = {
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

            if let Some(direction) = direction {
                commands.push(ClientCommand::Move(direction));
            }

            // orientation of camera.
            if let Some((offset_x, offset_y)) = input.mouse_delta {
                apply_delta_dir(offset_x, offset_y, camera, fps.sensitivity);
                commands.push(ClientCommand::LookAt(camera.pitch, camera.yaw));
            }

            if input.has_key_down(Key::Space) {
                commands.push(ClientCommand::Jump);
            }

            if input.has_mouse_event_happened(MouseButton::Button1, Action::Press) {
                let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                chan.single_write(GameEvent::Shoot);
                commands.push(ClientCommand::Shoot);
            }
        }
    }
    commands
}

fn apply_delta_dir(offset_x: f32, offset_y: f32, camera: &mut Camera, sensitivity: f32) {
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
