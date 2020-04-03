use crate::event::GameEvent;
use crate::resources::Resources;
use log::info;
use serde_derive::{Deserialize, Serialize};
use shrev::{EventChannel, ReaderId};
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
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
                if let Ok(_health) = world.get_mut::<Health>(*entity) {
                    info!("Entity was shot. Delete it!");
                    entities_to_delete.push(*entity);
                }
            }
        }

        for entity in entities_to_delete {
            info!("Delete entity {:?}", entity);
            chan.single_write(GameEvent::Delete(entity));
        }
    }
}
