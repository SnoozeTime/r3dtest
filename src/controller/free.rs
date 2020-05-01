use crate::camera::Camera;
use crate::controller::Fps;
use crate::ecs::Transform;
use crate::gameplay::player::MainPlayer;
use crate::input::Input;
use crate::resources::Resources;
use luminance_glfw::Key;

pub struct FreeController;

impl FreeController {
    pub fn process_input(
        &self,
        world: &mut hecs::World,
        resources: &mut Resources,
        e: hecs::Entity,
    ) {
        let mut transform = world.get_mut::<Transform>(e).unwrap();
        let fps = world.get::<Fps>(e).unwrap();
        let input = resources.fetch::<Input>().unwrap();
        let (front, up, left) = crate::geom::quat_to_direction(transform.rotation);

        // TODO maybe remove that later.
        let lateral_dir = {
            if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
                Some(left)
            } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
                Some(-left)
            } else {
                None
            }
        };
        let forward_dir = {
            if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
                Some(front)
            } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
                Some(-front)
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
            transform.translation += direction * 0.5;
            transform.dirty = true;
        }

        // orientation of camera.
        if let Some((offset_x, offset_y)) = input.mouse_delta {
            apply_delta_dir(offset_x, offset_y, &mut transform, fps.sensitivity, left);
        }

        if input.has_key_down(Key::Space) {
            let translation = transform.translation.y();
            transform.translation.set_y(translation + 0.5);
            transform.dirty = true;
        }
        //}
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
