use crate::net::snapshot::Deltable;
use glam::{Quat, Vec3};
use nalgebra::{Isometry3, UnitQuaternion};
use serde_derive::{Deserialize, Serialize};

pub mod serialization;
const EPSILON: f32 = 0.00001;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameObjectReference(pub String);

/// Simple transform component. Where is the game object.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
}

impl Transform {
    pub fn to_model(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn to_isometry(&self) -> Isometry3<f32> {
        /**
                /// let tra = Translation3::new(0.0, 0.0, 3.0);
        /// let rot = UnitQuaternion::from_scaled_axis(Vector3::y() * f32::consts::PI);
        /// let iso = Isometry3::from_parts(tra, rot);

            **/
        let tra = nalgebra::geometry::Translation3::new(
            self.translation.x(),
            self.translation.y(),
            self.translation.z(),
        );
        let (axis, angle) = self.rotation.to_axis_angle();
        let rot = UnitQuaternion::from_scaled_axis(
            nalgebra::Vector3::new(axis.x(), axis.y(), axis.z()) * angle,
        );

        Isometry3::from_parts(tra, rot)
    }

    pub fn set_isometry(&mut self, isometry: &Isometry3<f32>) {
        self.translation = glam::vec3(
            isometry.translation.x,
            isometry.translation.y,
            isometry.translation.z,
        );
        self.rotation = Quat::from_xyzw(
            isometry.rotation.i,
            isometry.rotation.j,
            isometry.rotation.k,
            isometry.rotation.w,
        );
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::zero(),
            scale: Vec3::zero(),
            rotation: Quat::identity(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformDelta {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

impl Deltable for Transform {
    type Delta = TransformDelta;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        let delta_pos = self.translation - old.translation;
        let delta_rot: Quat = old.rotation.conjugate() * self.rotation;
        let delta_scale = self.scale - old.scale;

        let delta_pos = if delta_pos.length_squared() > EPSILON {
            Some(delta_pos.into())
        } else {
            None
        };

        let delta_rot = if delta_rot.length_squared() > EPSILON {
            // Some(delta_rot.into())
            None // FIXME
        } else {
            None
        };
        let delta_scale = if delta_scale.length_squared() > EPSILON {
            Some(delta_scale.into())
        } else {
            None
        };

        match (delta_pos, delta_rot, delta_scale) {
            (None, None, None) => None,
            (p, r, s) => Some(TransformDelta {
                translation: p,
                rotation: r,
                scale: s,
            }),
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(TransformDelta {
            translation: Some(self.translation),
            rotation: Some(self.rotation),
            scale: Some(self.scale),
        })
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        if let Some(t) = delta.translation {
            self.translation += t;
        }

        if let Some(r) = delta.rotation {
            self.rotation = self.rotation * r;
        }

        if let Some(s) = delta.scale {
            self.scale += s;
        }
    }

    fn new_component(delta: &Self::Delta) -> Self {
        let mut t = Transform::default();
        t.apply_delta(delta);
        t
    }
}
