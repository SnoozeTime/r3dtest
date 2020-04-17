use crate::event::GameEvent;
use crate::net::snapshot::Deltable;
use crate::physics::{BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use glam::{Quat, Vec3};
use hecs::Entity;
use log::error;
use nalgebra::{Isometry3, UnitQuaternion};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
use std::fs;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub mod serialization;
const EPSILON: f32 = 0.00001;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Name(pub String);

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

pub struct WorldLoader {
    entities: Vec<Entity>,
    rx: Receiver<Result<notify::Event, notify::Error>>,
    file_to_watch: String,
    _watcher: RecommendedWatcher,
}

impl WorldLoader {
    pub fn new(file_to_watch: String) -> (Self, hecs::World) {
        let world =
            serialization::deserialize_world(fs::read_to_string(&file_to_watch).unwrap()).unwrap();
        let entities = world.iter().map(|(e, _)| e).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
            std::thread::sleep(Duration::from_millis(400));
            tx.send(res).unwrap()
        })
        .unwrap();

        watcher
            .watch(file_to_watch.clone(), RecursiveMode::Recursive)
            .unwrap();

        (
            Self {
                entities,
                rx,
                file_to_watch,
                _watcher: watcher,
            },
            world,
        )
    }

    pub fn update(
        &mut self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &mut Resources,
    ) {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        let mut should_reload = false;
        for res in &self.rx.try_recv() {
            match res {
                Ok(Event {
                    kind: EventKind::Modify(..),
                    ..
                }) => should_reload = true,
                _ => (),
            }
        }

        if should_reload {
            // remove all entities.
            let mut to_delete: Vec<_> = self
                .entities
                .drain(..)
                .map(|e| GameEvent::Delete(e))
                .collect();
            chan.drain_vec_write(&mut to_delete);

            // add new world and it's entities.
            if let Ok(entity_str) = fs::read_to_string(&self.file_to_watch) {
                if let Ok(new_ser_entities) =
                    ron::de::from_str::<Vec<serialization::SerializedEntity>>(&entity_str)
                {
                    let mut new_entities = serialization::add_to_world(world, new_ser_entities);
                    self.entities.append(&mut new_entities);

                    // Physics :)
                    let mut body_to_entity = resources.fetch_mut::<BodyToEntity>().unwrap();
                    // add the rigid bodies to the simulation.
                    for (e, (t, mut rb)) in world.query::<(&Transform, &mut RigidBody)>().iter() {
                        if rb.handle.is_none() {
                            let id = physics.add_body(&t, &mut rb);
                            body_to_entity.insert(id, e);
                        }
                    }
                } else {
                    error!("Error during world deserialization");
                }
            } else {
                error!("Error while reading file");
            }
        }
    }
}
