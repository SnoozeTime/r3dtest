use crate::ecs::Transform;
use crate::net::snapshot::Deltable;
use serde_derive::{Deserialize, Serialize};

pub fn get_view(world: &hecs::World) -> Option<glam::Mat4> {
    for (_, (cam, transform)) in world.query::<(&Camera, &Transform)>().iter() {
        if cam.active {
            return Some(cam.get_view(transform.translation));
        }
    }

    None
}

/// Get the camera currently active. This is used for the view matrix and the light
/// calculation.
pub fn find_main_camera(world: &hecs::World) -> Option<hecs::Entity> {
    for (e, cam) in world.query::<&Camera>().iter() {
        if cam.active {
            return Some(e);
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LookAt(pub glam::Vec3);

impl Default for LookAt {
    fn default() -> Self {
        Self(glam::Vec3::zero())
    }
}

impl Deltable for LookAt {
    type Delta = LookAt;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self != old {
            Some(*self)
        } else {
            None
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(*self)
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.0 = delta.0;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        Self(delta.0)
    }
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
