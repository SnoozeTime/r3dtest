use crate::camera::Camera;
use crate::controller::Fps;
use crate::ecs::Transform;
use crate::gameplay::player::{MainPlayer, Player};
use crate::input::Input;
use crate::resources::Resources;
use luminance_glfw::Key;

pub struct FreeController;

impl FreeController {
    pub fn process_input(&self, world: &mut hecs::World, resources: &mut Resources) {
        if let Some((e, (transform, camera, _, fps))) = world
            .query::<(&mut Transform, &mut Camera, &MainPlayer, &Fps)>()
            .iter()
            .next()
        {
            let input = resources.fetch::<Input>().unwrap();

            // TODO maybe remove that later.
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
                    Some(camera.front)
                } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
                    Some(-camera.front)
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
                transform.translation += direction * 1.0;
            }

            // orientation of camera.
            if let Some((offset_x, offset_y)) = input.mouse_delta {
                apply_delta_dir(offset_x, offset_y, camera, fps.sensitivity);
            }

            if input.has_key_down(Key::Space) {
                transform.translation.set_y(transform.translation.y() + 1.0);
            }
        }
    }
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
