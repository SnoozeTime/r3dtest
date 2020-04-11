//! Pick up items on the floor. Can be health, ammo, weapons and so on :)

use crate::event::GameEvent;
use crate::gameplay::gun::{GunInventory, GunType};
use crate::gameplay::player::Player;
use crate::net::snapshot::Deltable;
use crate::physics::{BodyIndex, PhysicWorld, RigidBody};
use crate::resources::Resources;
use log::debug;
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum PickUp {
    Ammo(GunType),
    Health(i32),
    Gun(GunType),
}

impl Default for PickUp {
    fn default() -> Self {
        PickUp::Health(10)
    }
}

impl Deltable for PickUp {
    type Delta = PickUp;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self != old {
            Some(*self)
        } else {
            None
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(*self)
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        *self = *delta;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        *delta
    }
}

pub struct PickUpSystem;

impl PickUpSystem {
    pub fn update(&self, world: &hecs::World, physics: &PhysicWorld, resources: &mut Resources) {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        let player_handles: Vec<(hecs::Entity, BodyIndex)> = world
            .query::<(&Player, &RigidBody)>()
            .iter()
            .map(|(e, (_, rb))| (e, rb.handle.unwrap()))
            .collect();

        let mut events = vec![];
        for (pickup_entity, (pick_up, rb)) in world.query::<(&PickUp, &RigidBody)>().iter() {
            debug!("Will process Player handles {:?}", player_handles);
            let pickup_handle = rb.handle.unwrap();

            let mut collide = None;
            // for each pickup, look if there is collision with a player.
            for (e, player_handle) in player_handles.iter() {
                debug!(
                    "Should check collisions between {:?} and {:?}",
                    player_handle, pickup_handle
                );
                if physics.check_aabb_collision(*player_handle, pickup_handle) {
                    collide = Some(*e);
                    break;
                }
            }

            if let Some(player_entity) = collide {
                // Send events and shit.
                let maybe_ev = match pick_up {
                    PickUp::Gun(gt) => {
                        let inv = world
                            .get::<GunInventory>(player_entity)
                            .expect("Player should have gun inventory");
                        Some(GameEvent::PickupGun {
                            entity: player_entity,
                            gun: *gt,
                        })
                    }
                    PickUp::Health(h) => Some(GameEvent::PickupHealth {
                        entity: player_entity,
                        health: *h,
                    }),
                    PickUp::Ammo(gun) => {
                        let inv = world
                            .get::<GunInventory>(player_entity)
                            .expect("Player should have gun inventory");
                        if inv.has_gun(*gun) {
                            Some(GameEvent::PickupAmmo {
                                entity: player_entity,
                                gun: *gun,
                            })
                        } else {
                            None
                        }
                    }
                };

                if let Some(ev) = maybe_ev {
                    events.push(ev);
                    // event to delete the pick up entity.
                    events.push(GameEvent::Delete(pickup_entity));
                }
            }
        }

        chan.drain_vec_write(&mut events);
    }
}
