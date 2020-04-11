use glam::Vec3;
use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
extern crate nalgebra as na;
use self::na::Isometry3;
use na::Point3;
use na::Vector3;
use ncollide3d::pipeline::CollisionGroups;
use ncollide3d::query::Ray;
use ncollide3d::shape::{Cuboid, ShapeHandle};
use nphysics3d::algebra::{Force3, ForceType};
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodyHandle, DefaultBodySet,
    DefaultColliderHandle, DefaultColliderSet, RigidBodyDesc,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};
use std::any::Any;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum Shape {
    // half-width. Center of box is position of rigidbody.
    AABB(glam::Vec3),
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum BodyType {
    Kinematic,
    Dynamic,
    Static,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
struct PhysicConfig {
    grav: f32,
    friction: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RigidBody {
    pub mass: f32,
    pub shape: Shape,
    pub ty: BodyType,
    #[serde(default)]
    pub max_linear_velocity: f32,
    #[serde(default)]
    pub max_angular_velocity: f32,
    #[serde(default)]
    pub linear_damping: f32,

    #[serde(skip)]
    pub handle: Option<BodyIndex>,
}

#[derive(Default)]
pub struct BodyToEntity(HashMap<BodyIndex, Entity>);

impl BodyToEntity {
    pub fn insert(&mut self, body: BodyIndex, entity: Entity) {
        self.0.insert(body, entity);
    }

    pub fn get(&self, body: &BodyIndex) -> Option<&Entity> {
        self.0.get(body)
    }

    pub fn remove(&mut self, body: &BodyIndex) {
        self.0.remove(body);
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub struct BodyIndex(DefaultBodyHandle, DefaultColliderHandle);

pub struct PhysicWorld {
    mechanical_world: DefaultMechanicalWorld<f32>,
    geometrical_world: DefaultGeometricalWorld<f32>,
    bodies: DefaultBodySet<f32>,
    colliders: DefaultColliderSet<f32>,
    joint_constraints: DefaultJointConstraintSet<f32, DefaultBodySet<f32>>,
    force_generators: DefaultForceGeneratorSet<f32, DefaultBodySet<f32>>,
    //ground_handle: BodyIndex,
}

impl Default for PhysicWorld {
    fn default() -> Self {
        Self::new()
    }
}
impl PhysicWorld {
    pub fn new() -> Self {
        let conf_str =
            fs::read_to_string(std::env::var("CONFIG_PATH").unwrap() + "physic.ron").unwrap();
        let conf: PhysicConfig = ron::de::from_str(&conf_str).unwrap();

        let mechanical_world = DefaultMechanicalWorld::new(Vector3::new(0., conf.grav, 0.));
        let geometrical_world = DefaultGeometricalWorld::new();

        let bodies = DefaultBodySet::new();
        let colliders = DefaultColliderSet::new();
        let joint_constraints = DefaultJointConstraintSet::new();
        let force_generators = DefaultForceGeneratorSet::new();

        Self {
            mechanical_world,
            geometrical_world,
            bodies,
            colliders,
            joint_constraints,
            force_generators,
        }
    }

    pub fn step(&mut self, _dt: f32) {
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
        let Shape::AABB(aabb) = body_component.shape;
        let cuboid = ShapeHandle::new(Cuboid::new(Vector3::new(aabb.x(), aabb.y(), aabb.z())));
        let rb = RigidBodyDesc::new()
            .translation(Vector3::new(position.x(), position.y(), position.z()))
            .set_max_angular_velocity(body_component.max_angular_velocity) // NO ROTATION :)
            .set_max_linear_velocity(body_component.max_linear_velocity)
            .set_linear_damping(body_component.linear_damping)
            .set_status(match body_component.ty {
                BodyType::Static => BodyStatus::Static,
                BodyType::Dynamic => BodyStatus::Dynamic,
                BodyType::Kinematic => BodyStatus::Kinematic,
            })
            .build();
        // Insert the rigid body to the body set.
        let rb_handle = self.bodies.insert(rb);

        // Build the collider.
        let co = ColliderDesc::new(cuboid.clone())
            .density(1.0)
            .build(BodyPartHandle(rb_handle, 0));
        // Insert the collider to the body set.
        let collider_handle = self.colliders.insert(co);
        body_component.handle = Some(BodyIndex(rb_handle, collider_handle));
        BodyIndex(rb_handle, collider_handle)
    }

    pub fn remove_body(&mut self, h: BodyIndex) {
        self.bodies.remove(h.0);
        // TODO check if need to remove collider.
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

    pub fn get_shape(&self, h: BodyIndex) -> Option<Shape> {
        if let Some(coll) = self.colliders.get(h.1) {
            let shape = coll.shape().aabb(&Isometry3::new(
                Vector3::new(0., 0., 0.),
                Vector3::new(0., 0., 0.),
            ));

            let half_extents = shape.half_extents();

            return Some(Shape::AABB(glam::vec3(
                half_extents.x,
                half_extents.y,
                half_extents.z,
            )));
        }

        None
    }

    /// Directly add a velocity change :) instead of using an acceleration
    pub fn add_velocity_change(&mut self, h: BodyIndex, force: glam::Vec3) {
        if let Some(body) = self.bodies.get_mut(h.0) {
            body.apply_force(
                0,
                &Force3::new(
                    Vector3::new(force.x(), force.y(), force.z()),
                    Vector3::new(0., 0., 0.),
                ),
                ForceType::VelocityChange,
                true,
            )
        }
    }

    // Set the friction. That's necessary to avoid sliding during movemnet. when the player is walking,
    // friction is high. When jumping it is a bit lower.
    pub fn set_friction(&mut self, h: BodyIndex, friction: f32) {
        if let Some(body) = self.bodies.get_mut(h.0) {
            if let Some(rb) = body.downcast_mut::<nphysics3d::object::RigidBody<f32>>() {
                rb.set_linear_damping(friction);
            }
        }
    }

    pub fn raycast(
        &self,
        h: BodyIndex,
        center_offset: glam::Vec3,
        d: glam::Vec3,
    ) -> Vec<(f32, BodyIndex)> {
        let groups = CollisionGroups::default();

        let ray = Ray::new(
            Point3::new(center_offset.x(), center_offset.y(), center_offset.z()),
            Vector3::new(d.x(), d.y(), d.z()),
        );
        let interference =
            self.geometrical_world
                .interferences_with_ray(&self.colliders, &ray, &groups);
        // (Objects::CollisionObjectHandle, &'a Objects::CollisionObject, RayIntersection<N>)
        let mut results = vec![];
        for (a, b, c) in interference {
            let body_handle = b.body();
            if body_handle != h.0 {
                results.push((c.toi, BodyIndex(body_handle, a)));
            }
        }
        results
    }

    /// Check if the AABBs of the two bodies are overlapping. If yes, return true, else return
    /// false. If body index is not in physics world, return false.
    pub fn check_aabb_collision(&self, a: BodyIndex, b: BodyIndex) -> bool {
        self.geometrical_world
            .contact_pair(&self.colliders, a.1, b.1, true)
            .is_some()
    }
}
