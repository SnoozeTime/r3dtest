use crate::controller::client::ClientCommand;
use hecs::Entity;

#[derive(Debug)]
pub enum Event {
    Client(ClientCommand),
    Game(GameEvent),
}

#[derive(Debug)]
pub enum GameEvent {
    EntityShot { entity: Entity },
    Delete(Entity),
}
