use crate::controller::client::ClientCommand;
use hecs::Entity;

#[derive(Debug)]
pub enum Event {
    Client(ClientCommand),
    Game(GameEvent),
}

#[derive(Debug)]
pub enum GameEvent {
    EntityShot {
        entity: Entity,
    },
    Delete(Entity),

    /// text has been changed, or new text is added. The renderer needs to update its font
    /// cache.
    UpdateText,

    HealthUpdate {
        entity: Entity,
        new_health: f32,
    },

    /// One of the player is dead. Change its state to spawning ;)
    PlayerDead {
        entity: Entity,
    },
}
