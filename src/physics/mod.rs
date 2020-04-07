use crate::physics::bounding_box::{Aabb, Ray};
use crate::physics::collision::generate_contacts;
use glam::Vec3;
use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

pub mod bounding_box;
pub mod collision;

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

    #[serde(skip)]
    pub handle: Option<BodyIndex>,
}

pub type BodyIndex = usize;

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

pub struct PhysicWorld {
    current_state: Vec<Option<RigidBodyInternal>>,
    previous_state: Vec<Option<RigidBodyInternal>>,

    vel_change: HashMap<BodyIndex, glam::Vec3>,

    accumulator: f32,

    // step in seconds of the simulation.
    step_dt: f32,
    // time elapsed since the beginning.
    t: f32,

    conf: PhysicConfig,
}

impl Default for PhysicWorld {
    fn default() -> Self {
        let conf_str =
            fs::read_to_string(std::env::var("CONFIG_PATH").unwrap() + "physic.ron").unwrap();
        let conf = ron::de::from_str(&conf_str).unwrap();
        Self {
            current_state: vec![],
            previous_state: vec![],
            accumulator: 0.0,
            step_dt: 0.01, // 10ms step.
            t: 0.0,
            vel_change: HashMap::new(),
            conf,
        }
    }
}

impl PhysicWorld {
    pub fn add_body(&mut self, position: glam::Vec3, body_component: &mut RigidBody) -> BodyIndex {
        let body = RigidBodyInternal::new(position, body_component.clone());
        self.current_state.push(Some(body));
        self.previous_state.push(Some(body));
        let handle = self.current_state.len() - 1;
        body_component.handle = Some(handle);
        handle
    }

    pub fn remove_body(&mut self, h: BodyIndex) {
        if let Some(current) = self.current_state.get_mut(h) {
            *current = None;
            *self.previous_state.get_mut(h).unwrap() = None;
        }
    }

    pub fn step(&mut self, frame_time: f32) {
        self.accumulator += frame_time;

        while self.accumulator >= self.step_dt {
            // integrate physics.
            debug!("Integrate at {}", self.t);
            let collisions = self.find_collisions();
            self.resolve_collisions(collisions);
            self.integrate();
            self.accumulator -= self.step_dt;
            self.t += self.step_dt;
        }

        self.vel_change.clear();
    }

    /// simple integration
    //  F = ma
    //  a = F/m
    //  dv/dt = a
    //  v = a*dt + v0
    //  p = v*dt + p0
    fn integrate(&mut self) {
        for (i, body) in self.current_state.iter_mut().enumerate() {
            self.previous_state[i] = *body;

            if let Some(body) = body.as_mut() {
                if !body.enabled {
                    continue;
                }

                if let BodyType::Dynamic = body.ty {
                    // now F is just the gravity. Fgrav = m•g = (70 kg)•(9.8 m/s2) = 686 N
                    // a is just g
                    let next_pos = body.position + self.step_dt * body.velocity;
                    debug!("vel = {:?}", body.velocity);

                    let next_vel = body.velocity
                        + self.step_dt * glam::vec3(0.0, -self.conf.grav, 0.0)
                        - body.friction * body.velocity;
                    if body.velocity_change != Vec3::zero() {
                        info!("Velocity before change = {:?}", next_vel);
                        info!("CHANGE IS {:?}", body.velocity_change);
                    }

                    let next_vel = next_vel + body.velocity_change;
                    if body.velocity_change != Vec3::zero() {
                        info!("Velocity after change = {:?}", next_vel);
                    }
                    debug!("Next vel = {:?}", next_vel);
                    body.velocity = next_vel;
                    body.velocity_change = Vec3::zero();

                    // MAX SPEED.
                    if body.velocity.length() > 10.0 {
                        body.velocity = body.velocity.normalize() * 10.0;
                    }
                    trace!("Next vel = {:?}", next_vel);

                    body.position = next_pos;
                    trace!("New body = {:?}", body);
                }
            }
        }
    }

    fn find_collisions(&mut self) -> Vec<(BodyIndex, BodyIndex, Vec3)> {
        let mut collisions = vec![];
        for i in 0..(self.current_state.len() - 1) {
            for j in i + 1..self.current_state.len() {
                match (
                    self.current_state.get(i).unwrap(),
                    self.current_state.get(j).unwrap(),
                ) {
                    (Some(body), Some(other)) => {
                        if !body.enabled || !other.enabled {
                            continue;
                        }

                        if body.ty == BodyType::Static && other.ty == BodyType::Static {
                            continue;
                        }

                        if body.ty == BodyType::Dynamic && other.ty == BodyType::Dynamic {
                            continue;
                        }

                        if let Some(normal) = body.intersect(other, self.step_dt) {
                            debug!("Collision between {} and {}", i, j);
                            collisions.push((i, j, normal));
                        }
                    }
                    _ => (),
                }
            }
        }

        collisions
    }

    fn resolve_collisions(&mut self, mut collisions: Vec<(BodyIndex, BodyIndex, Vec3)>) {
        if collisions.is_empty() {
            return;
        }
        // sort collisions. First, process those with fastest velocities.
        debug!("Start resolving collisions");
        collisions.sort_by_key(|coll| {
            let body = self.current_state.get(coll.0).unwrap().as_ref().unwrap();
            let other = self.current_state.get(coll.1).unwrap().as_ref().unwrap();

            let rel_vel = body.velocity - other.velocity;
            debug!("RelVel: {:?} - {}", rel_vel, rel_vel.length().ceil() as i32);
            rel_vel.length().ceil() as i32 // boooo
        });

        debug!("{:?}", collisions);

        for &collision in collisions.iter().rev() {
            // AABB so let's do an impulse along the axis.
            debug!("RESOLUTION {} - {}", collision.0, collision.1);

            // collision.0 always < collision.1 because of how we built the collisions vec.
            let (head, tail) = self.current_state.split_at_mut(collision.0 + 1);

            let body = head.get_mut(collision.0).unwrap().as_mut().unwrap();
            let other = tail
                .get_mut(collision.1 - collision.0 - 1)
                .unwrap()
                .as_mut()
                .unwrap();

            let normal = collision.2;

            let rel_vel = body.velocity - other.velocity;
            //Impulse = MomentumAfter - MomentumBefore
            //Impulse = Mass * VelocityAfter - Mass * VelocityBefore
            let contact_vel = rel_vel.dot(normal);

            debug!(
                "Normal between {} and {} is {:?}. Contact velocity = {}",
                collision.0, collision.1, normal, contact_vel,
            );

            // FIXME - other body does not
            match (body.ty, other.ty) {
                (BodyType::Dynamic, BodyType::Static) => {
                    // simplification that mass of static is infinite. so inv_b = 0
                    // vafter_a = vbefore_a + inv_a * (-vrel . normal)/ (inv_a+inv_b)
                    body.velocity -= rel_vel * normal;
                }
                (BodyType::Static, BodyType::Dynamic) => {
                    // simplification that mass of static is infinite. so inv_b = 0
                    // vafter_a = vbefore_a + inv_a * (-vrel . normal)/ (inv_a+inv_b)
                    trace!("Rel vel = {:?}", rel_vel);
                    trace!("Normal = {:?}", normal);
                    trace!("Vel before impulsion = {:?}", other.velocity);
                    other.velocity += rel_vel * normal;
                    trace!("Vel after impulsion = {:?}?", other.velocity);
                }
                (BodyType::Dynamic, BodyType::Dynamic) => {
                    trace!("Dynamic-dynamic resolution");
                    let impulse = rel_vel * normal / (body.inverse_mass + other.inverse_mass);
                    trace!("impulse is {:?}", impulse);

                    trace!("Body vel before resolution = {:?}", body.velocity);
                    body.velocity -= body.inverse_mass * impulse;
                    trace!("Body vel after resolution = {:?}", body.velocity);

                    debug!("Other vel before resolution = {:?}", other.velocity);
                    other.velocity += other.inverse_mass * impulse;
                    debug!("Other vel after resolution = {:?}", other.velocity);
                }
                _ => (),
            }
        }
    }

    pub fn get_pos(&self, body_index: BodyIndex) -> Option<glam::Vec3> {
        if let Some(Some(current_state)) = self.current_state.get(body_index) {
            let previous_state = self
                .previous_state
                .get(body_index)
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap();
            let alpha = self.accumulator / self.step_dt;
            // interpolate the position;
            let pos = alpha * previous_state.position + (1.0 - alpha) * current_state.position;
            Some(pos)
        } else {
            None
        }
    }

    pub fn enable(&mut self, h: BodyIndex, enabled: bool) {
        if let Some(Some(current_state)) = self.current_state.get_mut(h) {
            current_state.enabled = enabled;
        }
    }

    pub fn add_force(&mut self, h: BodyIndex, force: glam::Vec3) {
        if let Some(Some(_)) = self.current_state.get_mut(h) {
            self.vel_change.insert(h, force);
        }
    }

    /// Directly add a velocity change :) instead of using an acceleration
    pub fn add_velocity_change(&mut self, h: BodyIndex, force: glam::Vec3) {
        if let Some(Some(current_state)) = self.current_state.get_mut(h) {
            info!("Velocity before change = {:?}", current_state.velocity);
            current_state.velocity_change += force;
        }
    }

    // Set the friction. That's necessary to avoid sliding during movemnet. when the player is walking,
    // friction is high. When jumping it is a bit lower.
    pub fn set_friction(&mut self, h: BodyIndex, friction: f32) {
        if let Some(Some(current_state)) = self.current_state.get_mut(h) {
            current_state.friction = friction;
        }
    }

    /// Return bounding box (good for debug rendering)
    pub fn get_aabb(&self, h: BodyIndex) -> Option<Aabb> {
        if let Some(Some(current_state)) = self.current_state.get(h) {
            let Shape::AABB(halfwidth) = current_state.shape;
            let aabb = Aabb::new(current_state.position, halfwidth);
            Some(aabb)
        } else {
            None
        }
    }

    pub fn get_collider_type(&self, h: BodyIndex) -> Option<BodyType> {
        if let Some(Some(current_state)) = self.current_state.get(h) {
            Some(current_state.ty)
        } else {
            None
        }
    }

    /// Return true if pos + delta is within an AABB
    pub fn check_collide(&self, h: BodyIndex, delta: glam::Vec3) -> bool {
        let my_shape = self.current_state.get(h).unwrap().as_ref().unwrap().shape;

        if let Some(Some(current_state)) = self.current_state.get(h) {
            let next_pos = {
                match my_shape {
                    Shape::AABB(hw) => {
                        current_state.position
                            + delta
                            + hw.dot(delta.normalize()) * delta.normalize()
                    }
                }
            };
            for (i, body) in self.current_state.iter().enumerate() {
                if i == h {
                    continue;
                }

                if let Some(ref body) = body {
                    if body.contains(next_pos) {
                        return true;
                    }
                }
            }
            false
        } else {
            false
        }
    }

    pub fn raycast(&self, h: BodyIndex, center_offset: Vec3, d: Vec3) -> Vec<(f32, BodyIndex)> {
        let position = {
            let body = self.current_state.get(h).unwrap().as_ref().unwrap();
            body.position
        };

        let mut toi = vec![];

        for (idx, body) in self.current_state.iter().enumerate() {
            if idx == h {
                continue;
            }

            if let Some(body) = body {
                let ray = Ray::new(position + center_offset, d.normalize());
                let Shape::AABB(hw) = body.shape;
                let aabb = Aabb::new(body.position, hw);
                if let Some((t, _)) = aabb.interset_ray(ray) {
                    toi.push((t, idx));
                }
            }
        }

        toi
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub(crate) struct RigidBodyInternal {
    // primary
    position: glam::Vec3,

    // Secondary
    velocity: glam::Vec3,

    // Constant
    mass: f32,
    inverse_mass: f32,
    shape: Shape,
    ty: BodyType,

    /// if true, the body will be used in the simulation
    enabled: bool,

    friction: f32,

    velocity_change: glam::Vec3,
}

impl RigidBodyInternal {
    pub(crate) fn new(position: glam::Vec3, body: RigidBody) -> Self {
        Self {
            position,
            velocity: glam::vec3(0.0, 0.0, 0.0),
            mass: body.mass,
            inverse_mass: 1.0 / body.mass,
            shape: body.shape,
            ty: body.ty,
            enabled: true,
            friction: 0.0,
            velocity_change: Vec3::zero(),
        }
    }

    pub(crate) fn intersect(&self, other: &RigidBodyInternal, dt: f32) -> Option<Vec3> {
        match self.shape {
            Shape::AABB(halfwidth) => match other.shape {
                Shape::AABB(other_halfwidth) => {
                    let aabb = Aabb::new(self.position, halfwidth);
                    let other_aabb = Aabb::new(other.position, other_halfwidth);
                    if let Some((t, norm)) =
                        generate_contacts(&aabb, &self.velocity, &other_aabb, &other.velocity)
                    {
                        trace!("T => {:?}, norm = {:?}", t, norm);
                        if t < dt {
                            trace!("T IS LESS THAN DT");
                            Some(norm)
                        } else {
                            None
                        }
                    } else {
                        None
                    }

                    //intersect_aabb_aabb(self.position, halfwidth, other.position, other_halfwidth)
                }
            },
        }
    }

    pub fn contains(&self, point: glam::Vec3) -> bool {
        match self.shape {
            Shape::AABB(halfwidth) => {
                let min = self.position - halfwidth;
                let max = self.position + halfwidth;

                trace!(
                    "min x {} <= point x {} and max x {} >= point x {}",
                    min.x(),
                    point.x(),
                    max.x(),
                    point.x()
                );
                trace!(
                    "min y {} <= point y {} and max y {} >= point y {}",
                    min.y(),
                    point.y(),
                    max.y(),
                    point.y()
                );
                trace!(
                    "min z {} <= point z {} and max z {} >= point z {}",
                    min.z(),
                    point.z(),
                    max.z(),
                    point.z()
                );
                (min.x() <= point.x() && max.x() >= point.x())
                    && (min.y() <= point.y() && max.y() >= point.y())
                    && (min.z() <= point.z() && max.z() >= point.z())
            }
        }
    }
}
