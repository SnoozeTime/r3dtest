//! Clean entities the right way. Done at the end of a frame.

use crate::event::GameEvent;
use crate::physics::{BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use shrev::{EventChannel, ReaderId};

/// ahahaha what a confusing name.
pub struct GarbageCollector {
    rdr_id: ReaderId<GameEvent>,
}

impl GarbageCollector {
    pub fn new(resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        let rdr_id = chan.register_reader();
        Self { rdr_id }
    }

    pub fn collect(
        &mut self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) {
        let chan = resources.fetch::<EventChannel<GameEvent>>().unwrap();
        for ev in chan.read(&mut self.rdr_id) {
            if let GameEvent::Delete(e) = ev {
                // remove the physic body.
                if let Ok(rb) = world.get::<RigidBody>(*e) {
                    if let Some(h) = rb.handle {
                        // remove from physics.
                        physics.remove_body(h);

                        // remove from body to entity cache.
                        let mut cache = resources.fetch_mut::<BodyToEntity>().unwrap();
                        cache.remove(&h);
                    }
                }

                // remove from world
                world.despawn(*e).unwrap();
            }
        }
    }

    pub fn collect_without_physics(&mut self, world: &mut hecs::World, resources: &Resources) {
        let chan = resources.fetch::<EventChannel<GameEvent>>().unwrap();
        for ev in chan.read(&mut self.rdr_id) {
            if let GameEvent::Delete(e) = ev {
                // remove from world
                world.despawn(*e).unwrap();
            }
        }
    }
}
