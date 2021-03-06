//! Individual UI editor for components
//!
use crate::assets::AssetManager;
use crate::ecs::{Name, Transform};
use crate::geom;
use crate::physics::{RigidBody, Shape};
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::render::mesh::mesh::Mesh;
use crate::render::Render;
use crate::resources::Resources;
use crate::transform::LocalTransform;
use glam::Quat;
use imgui::{im_str, CollapsingHeader, ColorEdit, ComboBox, Ui};
use nalgebra::UnitQuaternion;

/// Edit the transform component of an entity
#[derive(Default)]
pub struct TransformEditor;

impl TransformEditor {
    pub fn edit(&self, ui: &Ui, transform: &mut Transform) {
        if CollapsingHeader::new(im_str!("Transform"))
            .default_open(true)
            .build(ui)
        {
            let mut translation = transform.translation.into();
            if ui
                .input_float3(&im_str!("translation"), &mut translation)
                .build()
            {
                transform.translation = translation.into();
                transform.dirty = true;
            }

            let mut scale = transform.scale.into();
            if ui.input_float3(&im_str!("scale"), &mut scale).build() {
                transform.scale = scale.into();
                transform.dirty = true;
            }

            // need to convert back and forth to euler angles for the rotation.
            let euler: (f32, f32, f32) = geom::euler_from_quat(transform.rotation);
            let mut angles = [euler.0, euler.1, euler.2];
            if ui.input_float3(&im_str!("rotation"), &mut angles).build() {
                transform.dirty = true;
                transform.rotation = geom::quat_from_euler(angles[0], angles[1], angles[2]);
                // transform.rotation = glam::Quat::from_rotation_ypr(angles[0], angles[1], angles[2]);
            }
        }
    }
}

#[derive(Default)]
pub struct LocalTransformEditor;

impl LocalTransformEditor {
    pub fn edit(&self, ui: &Ui, transform: &mut LocalTransform) {
        if CollapsingHeader::new(im_str!("Local Transform"))
            .default_open(true)
            .build(ui)
        {
            let mut translation = transform.translation.into();
            if ui
                .input_float3(&im_str!("local translation"), &mut translation)
                .build()
            {
                transform.translation = translation.into();
                transform.dirty = true;
            }

            let mut scale = transform.scale.into();
            if ui.input_float3(&im_str!("local scale"), &mut scale).build() {
                transform.scale = scale.into();
                transform.dirty = true;
            }

            // need to convert back and forth to euler angles for the rotation.
            let euler: (f32, f32, f32) = geom::euler_from_quat(transform.rotation);
            let mut angles = [euler.0, euler.1, euler.2];
            if ui
                .input_float3(&im_str!("local rotation"), &mut angles)
                .build()
            {
                transform.dirty = true;
                transform.rotation = geom::quat_from_euler(angles[0], angles[1], angles[2]);
            }
        }
    }
}

/// Edit the name of an entity.
#[derive(Default)]
pub struct NameEditor;

impl NameEditor {
    pub fn edit(&self, ui: &Ui, name: &mut Name) {
        if CollapsingHeader::new(im_str!("Name"))
            .default_open(true)
            .build(ui)
        {
            let mut imstring = imgui::ImString::from(name.0.clone());
            if ui.input_text(&im_str!("Name"), &mut imstring).build() {
                name.0 = imstring.to_string();
            }
        }
    }
}

/// Edit the rigid body of an entity. For now, just the bounds of the AABB collider should be OK.
#[derive(Default)]
pub struct RigidBodyEditor;

impl RigidBodyEditor {
    pub fn edit(&self, ui: &Ui, rb: &mut RigidBody) -> bool {
        let mut edited = false;
        if CollapsingHeader::new(im_str!("Rigid Body"))
            .default_open(true)
            .build(ui)
        {
            let Shape::AABB(bounds) = rb.shape;
            let mut bounds = bounds.into();
            if ui
                .input_float3(&im_str!("Rigidbody bounds"), &mut bounds)
                .build()
            {
                edited = true;
                rb.shape = Shape::AABB(bounds.into());
            }
        }
        edited
    }
}

/// Edit the ambient light component :)
#[derive(Default)]
pub struct AmbientLightEditor;

impl AmbientLightEditor {
    pub fn edit(&self, ui: &Ui, ambient: &mut AmbientLight) {
        if CollapsingHeader::new(im_str!("Ambient Light"))
            .default_open(true)
            .build(ui)
        {
            let mut color = ambient.color.to_rgba_normalized();
            if ColorEdit::new(im_str!("Color"), &mut color).build(ui) {
                ambient.color = color.into();
            }
            ui.input_float(im_str!("Intensity"), &mut ambient.intensity)
                .build();
        }
    }
}

// Edit the directional light.
#[derive(Default)]
pub struct DirectionalLightEditor;

impl DirectionalLightEditor {
    pub fn edit(&self, ui: &Ui, light: &mut DirectionalLight) {
        if CollapsingHeader::new(im_str!("Directional Light"))
            .default_open(true)
            .build(ui)
        {
            let mut color = light.color.to_rgba_normalized();
            if ColorEdit::new(im_str!("Color"), &mut color).build(ui) {
                light.color = color.into();
            }

            let mut direction = light.direction.into();
            if ui
                .input_float3(&im_str!("Direction"), &mut direction)
                .build()
            {
                light.direction = direction.into();
            }
        }
    }
}

// Edit render component
#[derive(Default)]
pub struct RenderEditor;

impl RenderEditor {
    pub fn edit(&self, ui: &Ui, render: &mut Render, resources: &Resources) {
        if CollapsingHeader::new(im_str!("Render"))
            .default_open(true)
            .build(ui)
        {
            let mut imstring = imgui::ImString::from(render.mesh.clone());
            let mut mesh_manager = resources.fetch_mut::<AssetManager<Mesh>>().unwrap();

            let mut items = Vec::new();
            let mut selected = 0;
            for (i, k) in mesh_manager.keys().enumerate() {
                if render.mesh == k.0 {
                    selected = i;
                }

                items.push(im_str!("{}", k.0));
            }

            //let assets: Vec<String> = mesh_manager.keys().map(|k| im_str!("{}", k.0)).collect();

            if ComboBox::new(im_str!("hi")).build_simple_string(
                ui,
                &mut selected,
                &items.iter().collect::<Vec<_>>(),
            ) {
                render.mesh = items[selected].to_string();
            }

            //            if ui.input_text(im_str!("Mesh"), &mut imstring).build() {
            //                render.mesh = imstring.to_string();
            //            }
        }
    }
}
