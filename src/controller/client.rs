use super::Fps;
use crate::camera::Camera;
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::gameplay::gun::{Gun, GunSlot};
use crate::gameplay::player::{MainPlayer, Player, PlayerState};
use crate::input::Input;
use crate::resources::Resources;
use crate::transform::HasChildren;
use log::info;
use luminance_glfw::{Action, Key, MouseButton};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ClientCommand {
    Move(glam::Vec3),
    CameraMoved,
    LookAt(f32, f32), // pitch and yaw
    Jump,
    Shoot,
    ChangeGun(GunSlot),
    Forward(f32),
    Lateral(f32),
}

pub struct ClientController {
    net: bool,
}

impl ClientController {
    pub fn get_net_controller() -> Self {
        Self { net: true }
    }

    pub fn get_offline_controller() -> Self {
        Self { net: false }
    }

    pub fn process_input(
        &self,
        world: &mut hecs::World,
        resources: &mut Resources,
    ) -> Vec<ClientCommand> {
        let mut commands = vec![];

        if let Some((e, (t, fps, _, p))) = world
            .query::<(&mut Transform, &mut Fps, &MainPlayer, &Player)>()
            .iter()
            .next()
        {
            // player should have the camera as children if there is any camera.
            let input = resources.fetch::<Input>().unwrap();

            if let PlayerState::Alive = p.state {
                let (front, up, left) = crate::geom::quat_to_direction(t.rotation);
                // TODO maybe remove that later.
                let lateral_dir = {
                    if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
                        Some(left)
                    } else if input.key_down.contains(&Key::Right)
                        || input.key_down.contains(&Key::D)
                    {
                        Some(-left)
                    } else {
                        None
                    }
                };
                let forward_dir = {
                    if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
                        Some(left.cross(glam::Vec3::unit_y()))
                    } else if input.key_down.contains(&Key::Down)
                        || input.key_down.contains(&Key::S)
                    {
                        Some(-left.cross(glam::Vec3::unit_y()))
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
                    info!("Apply mouse delta {} {}", offset_x, offset_y);
                    apply_delta_dir(offset_x, offset_y, t, fps.sensitivity, left);
                    commands.push(ClientCommand::CameraMoved);
                }

                if input.has_key_down(Key::Space) {
                    commands.push(ClientCommand::Jump);
                }

                if input.has_mouse_event_happened(MouseButton::Button1, Action::Press) {
                    if let Ok(gun) = world.get_mut::<Gun>(e) {
                        if gun.can_shoot() {
                            if self.net {
                                // in the case of remote client, we will decrease the ammo right now before validation from server
                                // to make sure the UI is reactive. If there is discrepancy, the server will anyway send the correct state
                                // and remove the discrepancy
                                //gun.shoot();
                            }
                            let mut chan =
                                resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                            chan.single_write(GameEvent::Shoot);
                            commands.push(ClientCommand::Shoot);
                        }
                    }
                }

                if input.has_key_event_happened(Key::Num1, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(0))
                } else if input.has_key_event_happened(Key::Num2, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(1))
                } else if input.has_key_event_happened(Key::Num3, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(2))
                } else if input.has_key_event_happened(Key::Num4, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(3))
                } else if input.has_key_event_happened(Key::Num5, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(4))
                } else if input.has_key_event_happened(Key::Num6, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(5))
                } else if input.has_key_event_happened(Key::Num7, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(6))
                } else if input.has_key_event_happened(Key::Num8, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(7))
                } else if input.has_key_event_happened(Key::Num9, Action::Press) {
                    commands.push(ClientCommand::ChangeGun(8))
                }
            }
        }
        commands
    }
}

fn apply_delta_dir(
    offset_x: f32,
    offset_y: f32,
    t: &mut Transform,
    sensitivity: f32,
    left: glam::Vec3,
) {
    let rot_up = glam::Quat::from_rotation_y(-offset_x * sensitivity);
    let rot_left = glam::Quat::from_axis_angle(left, -offset_y * sensitivity);
    t.rotation = rot_up * rot_left * t.rotation;
    t.dirty = true;
}
