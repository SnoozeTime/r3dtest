use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
extern crate nalgebra as na;
use self::na::Isometry3;
use crate::ecs::Transform;
use glam::Quat;
use na::Point3;
use na::Vector3;
use ncollide3d::bounding_volume::AABB;
use ncollide3d::pipeline::CollisionGroups;
use ncollide3d::query::{Contact, Proximity, Ray};
use ncollide3d::shape::{ConvexHull, Cuboid, ShapeHandle};
use nphysics3d::algebra::{Force3, ForceType, Velocity3};
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::object::{
    BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodyHandle, DefaultBodySet,
    DefaultColliderHandle, DefaultColliderSet, RigidBodyDesc,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum Shape {
    // half-width. Center of box is position of rigidbody.
    AABB(glam::Vec3),
    ConvexHull,
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
    //  joint_constraints: DefaultJointConstraintSet<f32, DefaultBodySet<f32>>,
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
        // let joint_constraints = DefaultJointConstraintSet::new();
        let force_generators = DefaultForceGeneratorSet::new();

        Self {
            mechanical_world,
            geometrical_world,
            bodies,
            colliders,
            //joint_constraints,
            force_generators,
        }
    }

    pub fn step(&mut self, _dt: f32) {
        let mut joint = DefaultJointConstraintSet::new();
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut joint,
            &mut self.force_generators,
        );

        let contact_events = self.geometrical_world.contact_events();
        for event in contact_events.into_iter() {
            debug!("Contact events after step {:?}", event);
        }
        let proximity_events = self.geometrical_world.proximity_events();
        for event in proximity_events.into_iter() {
            debug!("Contact events after step {:?}", event);
        }
        /*
                 /// The contact events pool.
        pub fn contact_events(&self) -> &ContactEvents<CollHandle> {
            self.narrow_phase.contact_events()
        }

        /// The proximity events pool.
        pub fn proximity_events(&self) -> &ProximityEvents<CollHandle> {
            self.narrow_phase.proximity_events()
        }
                */
    }

    pub fn add_body(&mut self, transform: &Transform, body_component: &mut RigidBody) -> BodyIndex {
        // Shape is a cuboid :) for now TODO modify that
        info!("Will add body to physic world = {:?}", body_component);
        let shape_handle = match body_component.shape {
            Shape::AABB(aabb) => {
                ShapeHandle::new(Cuboid::new(Vector3::new(aabb.x(), aabb.y(), aabb.z())))
            }
            Shape::ConvexHull => {
                let points = vec![
                    Point3::new(1.0, -1.0, -1.0),
                    Point3::new(1.0, -1.0, 1.0),
                    Point3::new(0.49584853649139404, -1.0, 1.0),
                    Point3::new(-1.0, -1.0, -0.49584853649139404),
                    Point3::new(-1.0, -1.0, -1.0),
                    // second face
                    Point3::new(1.0, 1.0, 1.0),
                    Point3::new(0.49584853649139404, 1.0, 1.0),
                    Point3::new(0.49584853649139404, -1.0, 1.0),
                    Point3::new(1.0, -1.0, 1.0),
                    //third face
                    Point3::new(-1.0, 1.0, -1.0),
                    Point3::new(1.0, 1.0, -1.0),
                    Point3::new(1.0, -1.0, -1.0),
                    Point3::new(-1.0, -1.0, -1.0),
                    // fourth face
                    Point3::new(1.0, 1.0, -1.0),
                    Point3::new(1.0, 1.0, 1.0),
                    Point3::new(1.0, -1.0, 1.0),
                    Point3::new(1.0, -1.0, -1.0),
                    // 5th
                    Point3::new(-1.0, -1.0, -1.0),
                    Point3::new(-1.0, -1.0, -0.49584853649139404),
                    Point3::new(-1.0, 1.0, -0.49584853649139404),
                    Point3::new(-1.0, 1.0, -1.0),
                    // 6th
                    Point3::new(0.49584853649139404, -1.0, 1.0),
                    Point3::new(0.49584853649139404, 1.0, 1.0),
                    Point3::new(-1.0, 1.0, -0.49584853649139404),
                    Point3::new(-1.0, -1.0, -0.49584853649139404),
                    // 7th
                    Point3::new(-1.0, 1.0, -1.0),
                    Point3::new(-1.0, 1.0, -0.49584853649139404),
                    Point3::new(0.49584853649139404, 1.0, 1.0),
                    Point3::new(1.0, 1.0, 1.0),
                    Point3::new(1.0, 1.0, -1.0),
                ];

                let hull =
                    ConvexHull::try_from_points(&points).expect("Convex hull computation failed.");
                println!("SHAPE HANDLE = {:?}", hull);
                ShapeHandle::new(hull)
            }
        };

        let rb = RigidBodyDesc::new()
            //.translation(Vector3::new(position.x(), position.y(), position.z()))
            .position(transform.to_isometry())
            .set_max_angular_velocity(body_component.max_angular_velocity)
            .set_max_linear_velocity(body_component.max_linear_velocity)
            .set_status(match body_component.ty {
                BodyType::Static => BodyStatus::Static,
                BodyType::Dynamic => BodyStatus::Dynamic,
                BodyType::Kinematic => BodyStatus::Kinematic,
            })
            .build();
        // Insert the rigid body to the body set.
        let rb_handle = self.bodies.insert(rb);

        // Build the collider.
        let co = ColliderDesc::new(shape_handle)
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
    pub fn get_isometry(&self, body_index: BodyIndex) -> Option<Transform> {
        self.bodies.rigid_body(body_index.0).map(|rb| {
            let mut t = Transform::default();
            t.set_isometry(rb.position());
            t
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
            let current_speed = body.part(0).map(|part| part.velocity().linear.magnitude());

            if let Some(speed) = current_speed {
                if speed < 20.0 {
                    body.apply_force(
                        0,
                        &Force3::new(
                            Vector3::new(force.x(), force.y(), force.z()),
                            Vector3::new(0., 0., 0.),
                        ),
                        ForceType::VelocityChange,
                        true,
                    );
                }
            }
        }
    }

    pub fn set_linear_velocity(&mut self, h: BodyIndex, new_velocity: glam::Vec3) {
        if let Some(rb) = self.bodies.rigid_body_mut(h.0) {
            rb.set_linear_velocity(Vector3::new(
                new_velocity.x(),
                new_velocity.y(),
                new_velocity.z(),
            ));
        }
    }

    pub fn get_linear_velocity(&mut self, h: BodyIndex) -> Option<glam::Vec3> {
        if let Some(rb) = self.bodies.rigid_body_mut(h.0) {
            let v = rb.velocity().linear;
            Some(glam::vec3(v.x, v.y, v.z))
        } else {
            None
        }
    }

    pub fn get_position(&mut self, h: BodyIndex) -> Option<glam::Vec3> {
        if let Some(rb) = self.bodies.rigid_body_mut(h.0) {
            let p = rb.position().translation;
            Some(glam::vec3(p.x, p.y, p.z))
        } else {
            None
        }
    }

    pub fn set_position(&mut self, h: BodyIndex, new_position: glam::Vec3) {
        if let Some(rb) = self.bodies.rigid_body_mut(h.0) {
            rb.set_position(Isometry3::new(
                Vector3::new(new_position.x(), new_position.y(), new_position.z()),
                Vector3::new(0., 0., 0.),
            ));
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

    pub fn contact_with(&self, h: BodyIndex) -> Option<Vec<(glam::Vec3, f32)>> {
        if let Some(coll) = self.colliders.get(h.1) {
            let body = self.bodies.rigid_body(coll.body()).unwrap();
            let shape = coll.shape().aabb(&body.position());
            Some(
                self.geometrical_world
                    .interferences_with_aabb(&self.colliders, &shape, &CollisionGroups::default())
                    .filter(|(c, _)| *c != h.1)
                    .filter(|(c, obj)| self.bodies.get(obj.body()).unwrap().is_static())
                    .filter_map(|(c, obj)| {
                        ncollide3d::query::contact(
                            &body.position(),
                            coll.shape(),
                            obj.position(),
                            obj.shape(),
                            1.0,
                        )
                    })
                    .map(|contact| {
                        (
                            glam::vec3(contact.normal.x, contact.normal.y, contact.normal.z),
                            contact.depth,
                        )
                    })
                    .collect(),
            )
        } else {
            None
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
