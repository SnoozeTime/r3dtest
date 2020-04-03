use crate::physics::Shape;
use glam::{Mat3, Vec3};
use slotmap::SlotMap;
use thiserror::Error;

#[derive(Debug, Error)]
enum PhysicError {
    #[error("Body does not exist for index")]
    NoSuchBody,
}

/// ID of body and id of collider in body.
type ColliderId = (slotmap::DefaultKey, usize);

#[derive(Default)]
pub struct PhysicWorld {
    current_state: SlotMap<slotmap::DefaultKey, RigidBody>,
}

impl PhysicWorld {
    /// Will add a body with a given position and orientation. No collider in it yet.
    pub fn add_body(&mut self, position: Vec3, orientation: Mat3) -> slotmap::DefaultKey {
        let mut rb = RigidBody::default();
        rb.position = position;
        rb.m = 0.0;
        rb.inv_m = 0.0;
        rb.orientation = orientation;
        rb.update_orientation();
        self.current_state.insert(rb)
    }

    pub fn add_collider_to_body(
        &mut self,
        body_index: slotmap::DefaultKey,
        collider: Collider,
    ) -> Result<(), PhysicError> {
        if self.current_state.contains_key(body_index) {
            self.current_state[body_index].add_collider(collider);
            Ok(())
        } else {
            Err(PhysicError::NoSuchBody)
        }
    }
}

#[derive(Debug, Default)]
pub struct RigidBody {
    /// mass
    m: f32,
    /// inverse of mass
    inv_m: f32,
    local_inverse_inertia_tensor: Mat3,
    global_inverse_inertia_tensor: Mat3,

    global_centroid: Vec3,
    local_centroid: Vec3,

    position: Vec3,
    orientation: Mat3,
    inverse_orientation: Mat3,
    linear_velocity: Vec3,
    angular_velocity: Vec3,

    force_accumulator: Vec3,
    torque_accumulator: Vec3,

    colliders: Vec<Collider>,
}

impl RigidBody {
    /// Add a collider to this list of colliders for this rigid body.
    ///
    /// Will update mass, centroid and so on.
    pub fn add_collider(&mut self, collider: Collider) {
        self.colliders.push(collider);

        self.local_centroid = Vec3::zero();
        self.m = 0.0;

        for collider in &self.colliders {
            self.m += collider.m;
            self.local_centroid += collider.m * collider.local_centroid;
        }

        self.inv_m = 1.0 / self.m;
        self.local_centroid *= self.inv_m;

        // parallel axis theorem
        let mut local_inertia_tensor = Mat3::zero();
        for collider in &self.colliders {
            let r = self.local_centroid - collider.local_centroid;
            let r_dot_r = r.dot(r);

            // outer product
            let r_out_r = Mat3::from_cols(
                Vec3::new(r.x() * r.x(), r.y() * r.x(), r.z() * r.x()),
                Vec3::new(r.x() * r.y(), r.y() * r.y(), r.z() * r.y()),
                Vec3::new(r.x() * r.z(), r.y() * r.z(), r.z() * r.z()),
            );

            local_inertia_tensor +=
                collider.local_inertia_tensor + collider.m * (r_dot_r * Mat3::identity() - r_out_r);
        }
        self.local_inverse_inertia_tensor = local_inertia_tensor.inverse();
    }

    fn update_global_centroid_from_pos(&mut self) {
        self.global_centroid = self.orientation * self.local_centroid + self.position;
    }

    fn update_position_from_global_centroid(&mut self) {
        self.position = self.global_centroid - self.orientation * self.local_centroid;
    }

    pub fn local_to_global(&self, p: Vec3) -> Vec3 {
        self.orientation * p + self.position
    }

    pub fn global_to_local(&self, p: Vec3) -> Vec3 {
        self.inverse_orientation * (p - self.position)
    }

    pub fn local_to_global_vec(&self, v: Vec3) -> Vec3 {
        self.orientation * v
    }

    pub fn global_to_local_vec(&self, v: Vec3) -> Vec3 {
        self.inverse_orientation * v
    }

    fn update_orientation(&mut self) {
        let normalized_quat = glam::Quat::from_rotation_mat3(&self.orientation).normalize();
        self.orientation = glam::Mat3::from_quat(normalized_quat);
        self.inverse_orientation = self.orientation.transpose();
    }

    fn update_inertia_tensor(&mut self) {
        self.global_inverse_inertia_tensor =
            self.orientation * self.local_inverse_inertia_tensor * self.inverse_orientation;
    }
    /// Apply a force to this rigid body.
    ///
    /// # params:
    /// `at` is the point of application (global space)
    /// `force` is the force to apply.
    ///
    pub fn apply_force(&mut self, force: Vec3, at: Vec3) {
        self.force_accumulator += force;
        self.torque_accumulator += (at - self.global_centroid).cross(force);
    }
}

#[derive(Debug)]
pub struct Collider {
    m: f32,
    local_centroid: Vec3,
    local_inertia_tensor: Mat3,

    shape: Shape,
}

impl Collider {
    pub fn aabb(m: f32, local_centroid: Vec3, halfwidth: Vec3) -> Self {
        let shape = Shape::AABB(halfwidth);
        Self {
            m,
            local_centroid,
            local_inertia_tensor: Mat3::identity(),
            shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    macro_rules! assert_delta {
        ($x:expr, $y:expr, $d:expr) => {
            if !($x - $y < $d || $y - $x < $d) {
                panic!();
            }
        };
    }

    macro_rules! assert_vec_eq {
        ($x:expr, $y:expr) => {{
            assert_delta!($x.x(), $y.x(), 0.001);
            assert_delta!($x.y(), $y.y(), 0.001);
            assert_delta!($x.z(), $y.z(), 0.001);
        }};
    }

    #[test]
    fn global_to_local() {
        let mut rb = RigidBody::default();
        rb.position = glam::vec3(2.0, 2.0, 0.0);
        rb.orientation = Mat3::identity();

        assert_vec_eq!(
            rb.global_to_local(glam::vec3(1.5, 1.5, 0.0)),
            glam::vec3(-0.5, -0.5, 0.0)
        );

        rb.orientation = Mat3::from_rotation_z(PI / 2.0);
        rb.update_orientation();
        assert_vec_eq!(
            rb.global_to_local(glam::vec3(1.5, 1.5, 0.0)),
            glam::vec3(-0.5, 0.5, 0.0)
        );
    }

    #[test]
    fn local_to_global() {
        let mut rb = RigidBody::default();
        rb.position = glam::vec3(2.0, 2.0, 0.0);
        rb.orientation = Mat3::from_rotation_z(PI / 4.0);
        rb.update_orientation();

        assert_vec_eq!(
            rb.local_to_global(glam::vec3(-0.5, -0.5, 0.0)),
            glam::vec3(2.0, 2.0 - 0.5_f32.sqrt(), 0.0)
        );
    }
}
