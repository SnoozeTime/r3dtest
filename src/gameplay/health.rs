use crate::event::GameEvent;
use crate::net::snapshot::Deltable;
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

    pub fn update(&mut self, world: &hecs::World, resources: &Resources) {
        let mut entities_to_delete = vec![];

        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();

        for ev in chan.read(&mut self.rdr_id) {
            if let GameEvent::EntityShot { entity } = ev {
                if let Ok(mut health) = world.get_mut::<Health>(*entity) {
                    health.current -= 1.0;
                    info!("Entity was shot. current health = {:?}", health.current);
                    if health.current <= 0.0 {
                        entities_to_delete.push(*entity);
                    }
                }
            }
        }

        for entity in entities_to_delete {
            info!("Delete entity {:?}", entity);
            chan.single_write(GameEvent::Delete(entity));
        }
    }
}
