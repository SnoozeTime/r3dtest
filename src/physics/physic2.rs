extern crate nalgebra as na;

use crate::physics::bounding_box::Aabb;
use crate::physics::{RigidBody, Shape};
use na::Vector3;
use ncollide3d::shape::{Cuboid, ShapeHandle};
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle,
    DefaultColliderSet, RigidBodyDesc,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};

pub struct PhysicWorld {
    mechanical_world: DefaultMechanicalWorld<f32>,
    geometrical_world: DefaultGeometricalWorld<f32>,
    bodies: DefaultBodySet<f32>,
    colliders: DefaultColliderSet<f32>,
    joint_constraints: DefaultJointConstraintSet<f32, DefaultBodySet<f32>>,
    force_generators: DefaultForceGeneratorSet<f32, DefaultBodySet<f32>>,
}
pub struct BodyIndex(DefaultBodyHandle);

impl PhysicWorld {
    pub fn new() -> Self {
        let mut mechanical_world = DefaultMechanicalWorld::new(Vector3::new(0., -9.81, 0.));
        let mut geometrical_world = DefaultGeometricalWorld::new();

        let mut bodies = DefaultBodySet::new();
        let mut colliders = DefaultColliderSet::new();
        let mut joint_constraints = DefaultJointConstraintSet::new();
        let mut force_generators = DefaultForceGeneratorSet::new();

        Self {
            mechanical_world,
            geometrical_world,
            bodies,
            colliders,
            joint_constraints,
            force_generators,
        }
    }

    pub fn step(&mut self) {
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        )
    }

    pub fn add_body(&mut self, position: glam::Vec3, body_component: &mut RigidBody) -> BodyIndex {
        // Shape is a cuboid :) for now TODO modify that
        let rad = 0.5;
        let Shape::AABB(aabb) = body_component.shape;
        let cuboid = ShapeHandle::new(Cuboid::new(Vector3::new(aabb.x(), aabb.y(), aabb.z())));
        let rb = RigidBodyDesc::new()
            .translation(Vector3::new(position.x(), position.y(), position.z()))
            .set_max_angular_velocity(0.0) // NO ROTATION :)
            .build();
        // Insert the rigid body to the body set.
        let rb_handle = self.bodies.insert(rb);

        // Build the collider.
        let co = ColliderDesc::new(cuboid.clone())
            .density(1.0)
            .build(BodyPartHandle(rb_handle, 0));
        // Insert the collider to the body set.
        self.colliders.insert(co);
        BodyIndex(rb_handle)
    }

    pub fn remove_body(&mut self, h: BodyIndex) {
        self.bodies.remove(h.0);
        // TODO check if need to remove collider.
        //        if let Some(current) = self.current_state.get_mut(h) {
        //            *current = None;
        //            *self.previous_state.get_mut(h).unwrap() = None;
        //        }
    }

    pub fn get_pos(&self, body_index: BodyIndex) -> Option<glam::Vec3> {
        self.bodies
            .get(body_index.0)
            .and_then(|body| body.part(0))
            .map(|part| {
                let arr: [f32; 3] = part.position().translation.vector.into();
                glam::vec3(arr[0], arr[1], arr[2])
            })
    }

    pub fn add_force(&mut self, h: BodyIndex, force: glam::Vec3) {}

    /// Directly add a velocity change :) instead of using an acceleration
    pub fn add_velocity_change(&mut self, h: BodyIndex, force: glam::Vec3) {}

    // Set the friction. That's necessary to avoid sliding during movemnet. when the player is walking,
    // friction is high. When jumping it is a bit lower.
    pub fn set_friction(&mut self, h: BodyIndex, friction: f32) {}

    /// Return bounding box (good for debug rendering)
    pub fn get_aabb(&self, h: BodyIndex) -> Option<Aabb> {
        None
    }

    pub fn raycast(
        &self,
        h: BodyIndex,
        center_offset: glam::Vec3,
        d: glam::Vec3,
    ) -> Vec<(f32, BodyIndex)> {
        vec![]
    }

    /// Check if the AABBs of the two bodies are overlapping. If yes, return true, else return
    /// false. If body index is not in physics world, return false.
    pub fn check_aabb_collision(&self, a: BodyIndex, b: BodyIndex) -> bool {
        false
    }
}
