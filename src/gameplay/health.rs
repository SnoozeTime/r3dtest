use crate::colors;
use crate::colors::RgbColor;
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::gameplay::player::Player;
use crate::net::snapshot::Deltable;
use crate::render::particle::ParticleEmitter;
use crate::resources::Resources;
use log::info;
use serde_derive::{Deserialize, Serialize};
use shrev::{EventChannel, ReaderId};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Default)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Deltable for Health {
    type Delta = (f32, f32);

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self.current == old.current && self.max == old.max {
            None
        } else {
            Some((self.current - old.current, self.max - old.max))
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some((self.current, self.max))
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.max += delta.1;
        self.current += delta.0;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        Self {
            max: delta.1,
            current: delta.0,
        }
    }
}

pub struct HealthSystem {
    rdr_id: ReaderId<GameEvent>,
}

impl HealthSystem {
    pub fn new(resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        let rdr_id = chan.register_reader();
        Self { rdr_id }
    }

    pub fn update(&mut self, world: &mut hecs::World, resources: &Resources) {
        let mut entities_to_delete = vec![];
        let mut entities_to_spawn = vec![];
        let mut health_updates = vec![];
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();

        for ev in chan.read(&mut self.rdr_id) {
            match ev {
                GameEvent::EntityShot { entity, dir } => {
                    if let Ok(mut health) = world.get_mut::<Health>(*entity) {
                        health.current -= 1.0;
                        info!("Entity was shot. current health = {:?}", health.current);

                        health_updates.push(GameEvent::HealthUpdate {
                            entity: *entity,
                            new_health: health.current,
                        });

                        // SHOW SOME BLOOD.
                        let position = world.get::<Transform>(*entity).unwrap().translation;
                        entities_to_spawn.push(ParticleEmitter::new(
                            position,
                            *dir * 5.0,
                            100,
                            colors::RED,
                            Some(0.5),
                        ));

                        if health.current <= 0.0 {
                            if world.get::<Player>(*entity).is_ok() {
                                entities_to_delete.push(GameEvent::PlayerDead { entity: *entity });
                            } else {
                                entities_to_delete.push(GameEvent::Delete(*entity));
                            }
                        }
                    }
                }
                GameEvent::PickupHealth { entity, health: h } => {
                    info!(
                        "Got pickup health event. for entity {:?} and ammo {}",
                        entity.to_bits(),
                        h
                    );
                    if let Ok(mut health) = world.get_mut::<Health>(*entity) {
                        health.current += *h as f32;
                        health_updates.push(GameEvent::HealthUpdate {
                            entity: *entity,
                            new_health: health.current,
                        });
                    }
                }
                _ => (),
            }
        }

        chan.drain_vec_write(&mut entities_to_delete);
        chan.drain_vec_write(&mut health_updates);
        for e in entities_to_spawn {
            world.spawn((e,));
        }
    }
}
