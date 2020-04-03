// Utilities to extract state changes from two
// ECS.
//
// Fortunately, we do not send everything over the network
// At the moment, only position and render state will be
// target for the delta.
//
// For example, if the object has moved a bit, send the delta. If the mesh has morphed, send it as
// well.
use crate::camera::Camera;
use crate::collections::ring_buffer::RingBuffer;
use crate::colors::RgbColor;
use crate::controller::Fps;
use crate::ecs::{Transform, TransformDelta};
use crate::gameplay::player::Player;
use crate::render::Render;
use hecs::{Entity, EntityBuilder};
#[allow(unused_imports)]
use log::{debug, error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub trait Deltable {
    type Delta;

    /// Create the component delta between two instances
    fn compute_delta(&self, old: &Self) -> Option<Self::Delta>;

    /// Delta from one component. Use when the component was added this frame.
    fn compute_complete(&self) -> Option<Self::Delta>;

    /// Apply delta to current component
    fn apply_delta(&mut self, delta: &Self::Delta);

    /// Create new component from the delta.
    fn new_component(delta: &Self::Delta) -> Self;
}

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("Ringbuffer is currently empty")]
    RingBufferEmpty,

    #[error("The client's known state is too old")]
    ClientCaughtUp,

    #[error("Provided state index is out of bound")]
    InvalidStateIndex,
}

pub type State = HashMap<Entity, (Option<Transform>, Option<Render>, Option<RgbColor>)>;

/// Apply the latest server state to the client state.
#[derive(Default)]
pub struct Applier {
    /// Entity number on the server will not match the client's entity number...
    server_to_local_entity: HashMap<u64, Entity>,
}

impl Applier {
    pub fn apply_latest(&mut self, world: &mut hecs::World, snapshot: DeltaSnapshot) {
        // remove deleted entities.
        for to_delete in snapshot.entities_to_delete {
            if let Some(e) = self.server_to_local_entity.get(&to_delete) {
                info!("Will delete {}", e.to_bits());
                if let Err(e) = world.despawn(*e) {
                    error!("Error while despawning entity = {:?}", e);
                }
            }
        }

        for deltas in snapshot.deltas {
            if let Some(e) = self.server_to_local_entity.get(&deltas.entity) {
                let mut builder = EntityBuilder::new();
                if let Some(delta) = deltas.delta_transform {
                    if let Ok(mut t) = world.get_mut::<Transform>(*e) {
                        t.apply_delta(&delta);
                    } else {
                        builder.add(Transform::new_component(&delta));
                    }
                }

                if let Some(delta) = deltas.delta_color {
                    if let Ok(mut t) = world.get_mut::<RgbColor>(*e) {
                        t.apply_delta(&delta);
                    } else {
                        builder.add(RgbColor::new_component(&delta));
                    }
                }

                if let Some(delta) = deltas.delta_render {
                    if let Ok(mut t) = world.get_mut::<Render>(*e) {
                        t.apply_delta(&delta);
                    } else {
                        builder.add(Render::new_component(&delta));
                    }
                }

                world
                    .insert(*e, builder.build())
                    .expect("Entity does not exist...");
            } else {
                // TODO Add new entity.
                let mut builder = EntityBuilder::new();

                if let Some(delta) = deltas.delta_transform {
                    builder.add(Transform::new_component(&delta));
                }

                if let Some(delta) = deltas.delta_color {
                    builder.add(RgbColor::new_component(&delta));
                }

                if let Some(delta) = deltas.delta_render {
                    builder.add(Render::new_component(&delta));
                }

                // SPECIAL CASE IF PLAYER.
                if snapshot.player_entity == deltas.entity {
                    let cam = Camera::new(0., 0.);
                    builder.add(cam);
                    let fps = Fps {
                        on_ground: false,
                        jumping: false,
                        sensitivity: 0.005,
                        speed: 1.5,
                    };
                    builder.add(fps);
                    builder.add(Player);
                }

                let entity = world.spawn(builder.build());
                self.server_to_local_entity.insert(deltas.entity, entity);
            }
        }
    }
}

/// Give a delta between current snapshot and the previous state of the game.
///
/// Internally, it keeps a circular buffer with a bunch of ECS. Each clients
/// will have a last known state. The delta is computed between current and last
/// known, then sent to the client.
///
/// When a client hasn't updated its state fast enough and the circular buffer makes
/// a full round, the client will be considered disconnected. Timeout to disconnection
/// can be calculated from buffer size and frame duration. (60 fps -> 1 sec timeout =
/// buffer of size 60).
pub struct Snapshotter {
    state_buf: RingBuffer<State>,
    empty_ecs: State,
}

impl Snapshotter {
    pub fn new(ring_size: usize) -> Self {
        let state_buf = RingBuffer::new(ring_size);
        let empty_ecs = HashMap::new();

        Snapshotter {
            state_buf,
            empty_ecs,
        }
    }

    /// Update ring buffer with current state.
    pub fn set_current(&mut self, ecs: &hecs::World) {
        // it's making a copy.
        let mut state = HashMap::new();
        for (e, (t, r, c)) in ecs
            .query::<(Option<&Transform>, Option<&Render>, Option<&RgbColor>)>()
            .iter()
        {
            state.insert(e, (t.map(|t| *t), r.map(|r| r.clone()), c.map(|c| *c)));
        }

        self.state_buf.push(state);
    }

    pub fn get_current_index(&self) -> usize {
        self.state_buf.head_index()
    }

    /// Compute snapshot between current and last known state.
    /// If return value is None. it means, we cannot compute because the
    /// last known state has been replaced by now. -> disconnect client.
    pub fn get_delta(
        &self,
        known_state: usize,
        current_world: &hecs::World,
        player_entity: Entity,
    ) -> Result<DeltaSnapshot, SnapshotError> {
        if known_state == self.state_buf.head_index() {
            return Err(SnapshotError::ClientCaughtUp);
        }

        if let Some(old_ecs) = self.state_buf.get(known_state) {
            if let Some(new_ecs) = self.state_buf.head() {
                Ok(compute_delta(
                    old_ecs,
                    new_ecs,
                    current_world,
                    player_entity.to_bits(),
                ))
            } else {
                Err(SnapshotError::RingBufferEmpty)
            }
        } else {
            Err(SnapshotError::InvalidStateIndex)
        }
    }

    /// From client that havn't received anything yet.
    pub fn get_full_snapshot(
        &self,
        current_world: &hecs::World,
        player_entity: Entity,
    ) -> Result<DeltaSnapshot, SnapshotError> {
        if let Some(new_ecs) = self.state_buf.head() {
            Ok(compute_delta(
                &self.empty_ecs,
                new_ecs,
                current_world,
                player_entity.to_bits(),
            ))
        } else {
            debug!("RingBuffer is empty? {}", self.state_buf.head_index());
            Err(SnapshotError::RingBufferEmpty)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaSnapshot {
    pub player_entity: u64,
    pub deltas: Vec<DeltaEntity>,
    pub entities_to_delete: Vec<u64>,
}

// That is the change for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaEntity {
    pub entity: u64,
    pub delta_transform: Option<TransformDelta>,
    pub delta_render: Option<String>,
    pub delta_color: Option<RgbColor>,
}

impl DeltaEntity {
    fn is_empty(&self) -> bool {
        match (
            self.delta_render.as_ref(),
            self.delta_color,
            self.delta_transform.as_ref(),
        ) {
            (None, None, None) => true,
            _ => false,
        }
    }
}

// Compute change between two ECS
//
// What kind of action:
// - UPDATE entity (if update non-existing, should create it)
// - DEALLOCATE entity
pub fn compute_delta(
    old: &State,
    current: &State,
    current_world: &hecs::World,
    player_entity: u64,
) -> DeltaSnapshot {
    // Deallocating should be done first on client side to remove
    // outdated entities.
    // Find entities to delete, i.e. alive before but dead now.
    let mut to_delete = vec![];
    for k in old.keys() {
        if !current.contains_key(k) {
            to_delete.push(k.to_bits());
        }
    }

    // Get all live entities in current
    let mut deltas = Vec::new();

    for (entity, _) in current_world.iter() {
        let delta_entity = compute_delta_entity(entity, &current, &old);

        if !delta_entity.is_empty() {
            deltas.push(delta_entity);
        }
    }

    DeltaSnapshot {
        player_entity,
        deltas,
        entities_to_delete: to_delete,
    }
}

fn compute_delta_entity(entity: Entity, current: &State, old: &State) -> DeltaEntity {
    let (delta_transform, delta_render, delta_color) =
        match (current.get(&entity), old.get(&entity)) {
            (Some(new_components), Some(old_components)) => (
                new_components.0.and_then(|t| {
                    t.compute_delta(&old_components.0.unwrap_or(Transform::default()))
                }),
                new_components.1.as_ref().and_then(|t| {
                    t.compute_delta(
                        &old_components
                            .1
                            .as_ref()
                            .map(|r| r.clone())
                            .unwrap_or(Render::default()),
                    )
                }),
                new_components.2.and_then(|t| {
                    t.compute_delta(&old_components.2.unwrap_or(RgbColor::default()))
                }),
            ),
            (Some(new_components), None) => (
                new_components.0.and_then(|t| t.compute_complete()),
                new_components.1.as_ref().and_then(|t| t.compute_complete()),
                new_components.2.and_then(|t| t.compute_complete()),
            ),
            _ => (None, None, None),
        };

    DeltaEntity {
        entity: entity.to_bits(),
        delta_render,
        delta_color,
        delta_transform,
    }
}
// TODO To apply client-side, need to maintain a map from entity to entity. Entity server side
// and entity client side might not match.

//pub fn apply_delta(ecs: &mut hecs::World, delta_snapshot: DeltaSnapshot) {
//    // First delete the entities that have to be deleted.
//    for entity in &delta_snapshot.entities_to_delete {
//        ecs.despawn(*entity);
//    }
//
//    // Then apply the deltas.
//    for delta in &delta_snapshot.deltas {
//        // hum I wonder. Allocator should be only relevant on server side so let's just
//        // override here and see if any bug :D
//        if !ecs.is_entity_alive(&delta.entity) {
//            ecs.overwrite(&delta.entity);
//
//            // Maybe need to create some components.
//            match &delta.delta_transform {
//                (None, None, None) => (),
//                _ => {
//                    ecs.components
//                        .transforms
//                        .set(&delta.entity, TransformComponent::default());
//                }
//            }
//
//            match &delta.delta_model {
//                (None, None) => (),
//                _ => {
//                    ecs.components
//                        .models
//                        .set(&delta.entity, ModelComponent::default());
//                }
//            }
//
//            match &delta.delta_light {
//                (None, None, None) => (),
//                _ => {
//                    ecs.components
//                        .lights
//                        .set(&delta.entity, LightComponent::default());
//                }
//            }
//        }
//
//        if let Some(transform) = ecs.components.transforms.get_mut(&delta.entity) {
//            apply_transform_delta(transform, &delta.delta_transform);
//        }
//
//        if let Some(model) = ecs.components.models.get_mut(&delta.entity) {
//            apply_model_delta(model, &delta.delta_model);
//        }
//
//        if let Some(light) = ecs.components.lights.get_mut(&delta.entity) {
//            apply_light_delta(light, &delta.delta_light);
//        }
//    }
//}
