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
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::gameplay::health::Health;
use crate::gameplay::player::{MainPlayer, Player};
use crate::render::Render;
use crate::resources::Resources;
use hecs::{Entity, EntityBuilder, World};
#[allow(unused_imports)]
use log::{debug, error, info, trace};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;
use std::collections::HashMap;
use thiserror::Error;

pub trait Deltable: Debug {
    type Delta: Debug;

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

/// Macro to generate the code that will compute delta for each entity. Just add a component name
/// and a component to the macro to have it included in the state sent to players.
///
/// That works only for component that implements the `Deltable` trait.
macro_rules! snapshot {
    ($(($name:ident, $component:ty)),+) => {

        pub type State = HashMap<
            Entity,
            EntityState
        >;

        #[derive(Debug, Default)]
        pub struct EntityState {
            $(
                pub $name: Option<$component>,
            )+
        }


        fn state_from_current(world: &hecs::World) -> State {

            let mut state = HashMap::new();

            for (e, _) in world.iter() {

                let mut entity_state = EntityState::default();
                $(
                    if let Ok(c) = world.get::<$component>(e) {
                        entity_state.$name = Some((*c).clone());
                    }
                )+
                state.insert(e, entity_state);
            }

            state
        }



        // That is the change for an entity.
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct DeltaEntity {
            pub entity: u64,
            $(
                pub $name: Option<<$component as Deltable>::Delta>,
            )+
        }

        impl DeltaEntity {
            fn is_empty(&self) -> bool {

                $(
                if self.$name.is_some() {
                    return false;
                }
                )+

                true

            }
        }


        impl Applier {
            pub fn apply_latest(&mut self, world: &mut hecs::World, snapshot: DeltaSnapshot, resources: &mut Resources) {
            debug!("LATEST = {:?}", snapshot);
                let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                // remove deleted entities.
                for to_delete in snapshot.entities_to_delete {
                    if let Some(e) = self.server_to_local_entity.get(&to_delete) {
                        debug!("Will delete {}", e.to_bits());
                        if let Err(e) = world.despawn(*e) {
                            error!("Error while despawning entity = {:?}", e);
                        }
                    }
                }

                for deltas in snapshot.deltas {
                    trace!("delta in snapshot = {:?}", deltas);
                    if let Some(e) = self.server_to_local_entity.get(&deltas.entity) {
                        let mut builder = EntityBuilder::new();
                        $(
                            apply_delta::<$component>(world, *e, deltas.$name, &mut builder);
                        )+



                        world
                            .insert(*e, builder.build())
                            .expect("Entity does not exist...");

                        // mmmmh
                        if deltas.delta_health.is_some() {
                            println!("DELTA HEALTH");
                            chan.single_write(GameEvent::HealthUpdate {
                                entity: *e,
                                new_health: world.get::<Health>(*e).unwrap().current,
                            })
                        }
                    } else {
                        // TODO Add new entity.
                        let mut builder = EntityBuilder::new();

                        $(
                            apply_delta_to_new::<$component>(deltas.$name, &mut builder);
                        )+

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
                            builder.add(MainPlayer);
                        }

                        trace!("WILL BUILD NEW ENTITY");

                        let entity = world.spawn(builder.build());
                        trace!("Local entity is {:?}, server entity is {:?}", entity.to_bits(), deltas.entity);
                        self.server_to_local_entity.insert(deltas.entity, entity);

                        if deltas.delta_health.is_some() {
                            chan.single_write(GameEvent::HealthUpdate {
                                entity: entity,
                                new_health: world.get::<Health>(entity).unwrap().current,
                            })
                        }
                    }
                }
            }
        }


        fn compute_delta_entity(entity: Entity, current: &State, old: &State) -> DeltaEntity {
            let mut dentity = DeltaEntity::default();
            dentity.entity = entity.to_bits();

            match (current.get(&entity), old.get(&entity)) {
                (Some(new_components), Some(old_components)) => {
                    $(
                        dentity.$name = compute_delta_for_component(&new_components.$name, &old_components.$name);
                    )+
                }
                (Some(new_components), None) => {
                    $(
                        dentity.$name = compute_complete_for_component(&new_components.$name);
                    )+
                }
                _ => ()
            };
            dentity
        }


    }


}

snapshot! {
    (delta_transform, Transform),
    (delta_render, Render),
    (delta_color, RgbColor),
    (delta_player, Player),
    (delta_health, Health)
}

/// Apply the latest server state to the client state.
#[derive(Default)]
pub struct Applier {
    /// Entity number on the server will not match the client's entity number...
    server_to_local_entity: HashMap<u64, Entity>,
}

use std::fmt::Debug;

fn apply_delta<T>(
    world: &mut World,
    entity: Entity,
    delta: Option<T::Delta>,
    builder: &mut EntityBuilder,
) where
    T: Debug + Deltable + Send + Sync + 'static,
{
    if let Some(d) = delta {
        if let Ok(mut t) = world.get_mut::<T>(entity) {
            trace!("Apply delta {:?} to entity {:?}", d, entity);
            t.apply_delta(&d);
        } else {
            trace!("Add new component to entity {:?}", d);
            // if no component or if entity does not exist.
            builder.add(T::new_component(&d));
        }
    }
}

fn apply_delta_to_new<T>(delta: Option<T::Delta>, builder: &mut EntityBuilder)
where
    T: Debug + Deltable + Send + Sync + 'static,
{
    if let Some(d) = delta {
        // if no component or if entity does not exist.
        trace!("Add new component to entity {:?}", d);

        builder.add(T::new_component(&d));
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
        let state = state_from_current(ecs);
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

fn compute_delta_for_component<T>(new: &Option<T>, old: &Option<T>) -> Option<T::Delta>
where
    T: Deltable + Default,
{
    new.as_ref()
        .and_then(|c| c.compute_delta(old.as_ref().unwrap_or(&T::default())))
}

fn compute_complete_for_component<T>(new: &Option<T>) -> Option<T::Delta>
where
    T: Deltable,
{
    new.as_ref().and_then(|c| c.compute_complete())
}
