use crate::ecs::Transform;
use serde_derive::{Deserialize, Serialize};

pub fn get_view(world: &hecs::World) -> Option<glam::Mat4> {
    for (_, (cam, transform)) in world.query::<(&Camera, &Transform)>().iter() {
        if cam.active {
            return Some(cam.get_view(transform.translation));
        }
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub active: bool,
    pub pitch: f32,
    pub yaw: f32,
    pub front: glam::Vec3,
    pub left: glam::Vec3,
}

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Left,
    Right,
    Forward,
    Backward,
}

impl Camera {
    pub fn new(pitch: f32, yaw: f32) -> Self {
        let front = glam::vec3(
            pitch.cos() * yaw.cos(),
            pitch.sin(),
            pitch.cos() * yaw.sin(),
        );
        let world_up = glam::Vec3::unit_y();
        let left = world_up.cross(front);
        Self {
            active: true,
            front,
            pitch,
            yaw,
            left,
        }
    }

    /// Compute the look at matrix to send to the shader.
    pub fn get_view(&self, position: glam::Vec3) -> glam::Mat4 {
        glam::Mat4::look_at_rh(position, position + self.front, glam::Vec3::unit_y())
    }

    pub fn compute_vectors(&mut self) {
        // Now we need to recompute the vectors.
        self.front = glam::vec3(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        );

        let world_up = glam::Vec3::unit_y();
        self.left = world_up.cross(self.front);
    }
}
