//! Individual UI editor for components
//!
use crate::ecs::{Name, Transform};
use glam::Quat;
use imgui::{im_str, Ui};
use nalgebra::UnitQuaternion;

/// Edit the transform component of an entity
#[derive(Default)]
pub struct TransformEditor;

impl TransformEditor {
    pub fn edit(&self, ui: &Ui, transform: &mut Transform) {
        //
        let mut translation = transform.translation.into();
        if ui
            .input_float3(&im_str!("translation"), &mut translation)
            .build()
        {
            transform.translation = translation.into();
        }

        let mut scale = transform.scale.into();
        if ui.input_float3(&im_str!("scale"), &mut scale).build() {
            transform.scale = scale.into();
        }

        // need to convert back and forth to euler angles for the rotation.
        let mut angles = quat_to_euler(transform.rotation).into();
        if ui.input_float3(&im_str!("rotation"), &mut angles).build() {
            transform.rotation = glam::Quat::from_rotation_ypr(angles[0], angles[1], angles[2]);
        }
    }
}

#[allow(dead_code)]
fn quat_to_euler(q: Quat) -> glam::Vec3 {
    let (axis, angle) = q.to_axis_angle();
    let rot = UnitQuaternion::from_scaled_axis(
        nalgebra::Vector3::new(axis.x(), axis.y(), axis.z()) * angle,
    );
    let (roll, pitch, yaw) = rot.euler_angles();
    glam::Vec3::new(yaw, pitch, roll)
}

#[derive(Default)]
pub struct NameEditor;

impl NameEditor {
    pub fn edit(&self, ui: &Ui, name: &mut Name) {
        let mut imstring = imgui::ImString::from(name.0.clone());
        if ui.input_text(&im_str!("Name"), &mut imstring).build() {
            name.0 = imstring.to_string();
        }
    }
}
