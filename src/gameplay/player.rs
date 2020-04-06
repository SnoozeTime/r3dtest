use crate::camera::Camera;
use crate::colors;
use crate::controller::Fps;
use crate::ecs::serialization;
use crate::ecs::Transform;
use crate::physics::Shape::AABB;
use crate::physics::{BodyToEntity, BodyType, PhysicWorld, RigidBody};
use crate::render::sprite::{ScreenPosition, SpriteRender};
use crate::render::Render;
use crate::resources::Resources;
use hecs::{Entity, World};
#[allow(unused_imports)]
use log::{debug, info};
use std::fs;

use crate::event::GameEvent;
use crate::gameplay::health::Health;
use crate::net::snapshot::Deltable;
use serde_derive::{Deserialize, Serialize};
use shrev::{EventChannel, ReaderId};
use std::time::Duration;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PlayerState {
    Alive,
    Dead,
    // time to respawn.
    Respawn(f32),
}

impl PartialEq<PlayerState> for PlayerState {
    fn eq(&self, other: &PlayerState) -> bool {
        match (*self, *other) {
            (PlayerState::Alive, PlayerState::Alive) => true,
            (PlayerState::Dead, PlayerState::Dead) => true,
            (PlayerState::Respawn(respawn), PlayerState::Respawn(other_respawn)) => {
                respawn == other_respawn
            }
            _ => false,
        }
    }
}
impl Eq for PlayerState {}

/// Player is any human players. Up to 8 players in a multiplayer game.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Player {
    pub state: PlayerState,
    nb: usize,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            state: PlayerState::Alive,
            nb: 0,
        }
    }
}
/// YOU!
pub struct MainPlayer;

impl Deltable for Player {
    type Delta = Player;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if *self == *old {
            None
        } else {
            Some(*self)
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(*self)
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.state = delta.state;
        self.nb = delta.nb;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        *delta
    }
}

pub fn spawn_player(
    world: &mut hecs::World,
    physics: &mut PhysicWorld,
    resources: &Resources,
) -> Entity {
    let transform = Transform {
        translation: glam::vec3(0.0, 15.0, -5.0),
        scale: glam::Vec3::one(),
        rotation: glam::Quat::identity(),
    };
    let mesh = Render {
        mesh: "cube".to_string(),
        enabled: false,
    };
    let color = colors::RED;
    let cam = Camera::new(0., 0.);
    let mut rb = RigidBody {
        handle: None,
        mass: 1.,
        shape: AABB(glam::vec3(2.0, 2.0, 2.0)),
        ty: BodyType::Dynamic,
    };
    let idx = physics.add_body(transform.translation, &mut rb);
    let fps = Fps {
        on_ground: false,
        jumping: true,
        sensitivity: 0.005,
        speed: 1.5,
    };
    // physics.set_friction(idx, 0.3);

    let mut body_to_entity = resources.fetch_mut::<BodyToEntity>().unwrap();

    let player_health = Health {
        max: 10.0,
        current: 10.0,
    };

    let e = world.spawn((
        transform,
        cam,
        rb,
        fps,
        mesh,
        color,
        Player {
            state: PlayerState::Respawn(2.0),
            nb: 0,
        },
        player_health,
    ));

    body_to_entity.insert(idx, e);
    e
}

/// Spawn the entities that has the sprites (crosshair, gun...)
pub fn spawn_player_ui(world: &mut World) {
    let ui_str = fs::read_to_string(&format!(
        "{}{}",
        std::env::var("CONFIG_PATH").unwrap(),
        "ui.ron"
    ))
    .unwrap();
    let ui_entities: Vec<serialization::SerializedEntity> = ron::de::from_str(&ui_str).unwrap();
    serialization::add_to_world(world, ui_entities);
}

/// Monitor/Change state of players.
pub struct PlayerSystem {
    rdr_id: ReaderId<GameEvent>,
}

impl PlayerSystem {
    pub fn new(resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        Self {
            rdr_id: chan.register_reader(),
        }
    }

    /// dt in seconds
    pub fn update(&mut self, dt: Duration, world: &mut World, resources: &Resources) {
        let chan = resources.fetch::<EventChannel<GameEvent>>().unwrap();

        for ev in chan.read(&mut self.rdr_id) {
            if let GameEvent::PlayerDead { entity } = ev {
                let mut p = world
                    .get_mut::<Player>(*entity)
                    .expect("Player entity should have a player component");
                let mut r = world
                    .get_mut::<Render>(*entity)
                    .expect("Player entity should have a render component");
                info!(
                    "Player system will change the player to Spawning: {:?} / {:?}",
                    *p, *r
                );
                p.state = PlayerState::Respawn(5.0); // 5 seconds to respawn.
                r.enabled = false;
            }
        }

        // now, process player states.
        let mut player_to_respawn = vec![];
        for (e, p) in world.query::<&mut Player>().iter() {
            if let PlayerState::Respawn(ref mut time_to_respawn) = p.state {
                debug!("Player time to respawn = {:?}", time_to_respawn);
                *time_to_respawn -= dt.as_secs_f32();

                if *time_to_respawn <= 0.0 {
                    debug!("Will respawn player");
                    player_to_respawn.push(e);
                }
            }
        }

        self.respawn_players(world, player_to_respawn);
    }

    fn respawn_players(&self, world: &mut World, players: Vec<Entity>) {
        for player in players {
            let mut h = world
                .get_mut::<Health>(player)
                .expect("Player should have a health component");
            let mut p = world
                .get_mut::<Player>(player)
                .expect("Player entity should have a player component");
            let mut r = world
                .get_mut::<Render>(player)
                .expect("Player entity should have a render component");

            h.current = h.max;
            r.enabled = true;
            p.state = PlayerState::Alive;

            debug!("Player state now {:?} / {:?} / {:?}", *h, *r, *p);
        }
    }
}
