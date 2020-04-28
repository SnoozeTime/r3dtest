//! Everything related to the players
//! Will keep track of the state of each players:
//! - Alive: the player is shooting as usual
//! - Respawn: The player is dead and will respawn in a few seconds.
//!
//! Also has the `spawn_player` function that will spawn an entity for the player (should be
//! replaced by some configuration file at some point...)
use crate::camera::{Camera, LookAt};
use crate::ecs::serialization;
use crate::ecs::serialization::SerializedEntity;
use crate::ecs::Transform;
use crate::physics::{BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use hecs::{Entity, World};
#[allow(unused_imports)]
use log::{debug, info};
use std::fs;

use crate::animation::AnimationController;
use crate::event::GameEvent;
use crate::gameplay::gun::GunInventory;
use crate::gameplay::health::Health;
use crate::net::snapshot::Deltable;
use crate::render::billboard::Billboard;
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
    //    let transform = Transform {
    //        translation: glam::vec3(0.0, 15.0, -5.0),
    //        scale: glam::vec3(0.53, 1., 1.),
    //        rotation: glam::Quat::identity(),
    //    };
    //
    //    // player is a 2d sprite.
    //    let billboard = Billboard {
    //        sprite_nb: 0,
    //        enabled: false,
    //        texture: "soldier".to_string(),
    //    };
    //    let mut animations = HashMap::new();
    //    animations.insert(
    //        "walk_forward".to_string(),
    //        Animation::new(vec![(4, 10), (5, 10), (6, 10), (7, 10)]),
    //    );
    //    animations.insert(
    //        "walk_backward".to_string(),
    //        Animation::new(vec![(0, 10), (1, 10), (2, 10), (3, 10)]),
    //    );
    //    let animation = AnimationController {
    //        animations,
    //        current_animation: Some("walk_forward".to_string()),
    //    };
    //
    //    let cam = Camera::new(0., 0.);
    //    let look_at = LookAt(cam.front);
    //    let mut rb = RigidBody {
    //        handle: None,
    //        mass: 1.,
    //        shape: AABB(glam::vec3(0.5, 1.1, 0.5)),
    //        ty: BodyType::Dynamic,
    //    };
    //    let idx = physics.add_body(transform.translation, &mut rb);
    //    let fps = Fps {
    //        on_ground: false,
    //        jumping: true,
    //        sensitivity: 0.005,
    //        speed: 1.5,
    //    };
    // physics.set_friction(idx, 0.3);

    let mut body_to_entity = resources.fetch_mut::<BodyToEntity>().unwrap();

    //    let player_health = Health {
    //        max: 10.0,
    //        current: 10.0,
    //    };

    //
    let player_prefab = std::env::var("ASSET_PATH").unwrap() + "prefab/player.ron";

    println!("Prefab file = {:?}", player_prefab);
    let player_prefab = fs::read_to_string(player_prefab).unwrap();
    let ser_entity: SerializedEntity = ron::de::from_str(&player_prefab).unwrap();
    let e = crate::ecs::serialization::spawn_entity(world, &ser_entity);

    let lookat = {
        let cam = world.get::<Camera>(e).unwrap();
        LookAt(cam.front)
    };
    let idx = {
        let mut rb = world.get_mut::<RigidBody>(e).unwrap();
        let transform = world.get::<Transform>(e).unwrap();
        physics.add_body(&*transform, &mut rb)
    };
    let current_gun = {
        let inventory = world
            .get::<GunInventory>(e)
            .expect("Player should have a gun inventory");
        inventory
            .get_first()
            .expect("Inventory should have at least one gun")
    };
    //
    //    let e = world.spawn((
    //        transform,
    //        cam,
    //        rb,
    //        fps,
    //        billboard,
    //        animation,
    //        look_at,
    //        Player {
    //            state: PlayerState::Respawn(2.0),
    //            nb: 0,
    //        },
    //        player_health,
    //    ));

    body_to_entity.insert(idx, e);

    world.insert(e, (lookat, current_gun)).unwrap();

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
                    .get_mut::<Billboard>(*entity)
                    .expect("Player entity should have a billboard component");
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
            //            let mut r = world
            //                .get_mut::<Billboard>(player)
            //                .expect("Player entity should have a billboard component");

            h.current = h.max;
            // r.enabled = true;
            p.state = PlayerState::Alive;

            debug!("Player state now {:?} / {:?}", *h, *p);
        }
    }
}

/// This will change the players animations based on where the main player is. As it uses the main player
/// component, this is *not* server side code.
pub fn update_player_orientations(world: &mut World) {
    if let Some((player_entity, main_player_position)) = world
        .query::<(&Transform, &MainPlayer)>()
        .iter()
        .map(|(e, (t, _))| (e, t.translation))
        .next()
    {
        // update animations :)
        for (e, (_, t, c, a)) in world
            .query::<(&Player, &Transform, &LookAt, &mut AnimationController)>()
            .iter()
        {
            if e == player_entity {
                continue;
            }

            // If the player is not looking in the direction of the main player, display his back.
            let dir = main_player_position - t.translation;
            let dot = c.0.dot(dir);
            if dot < 0.0 {
                a.current_animation = Some("walk_backward".to_owned());
            } else {
                a.current_animation = Some("walk_forward".to_owned());
            }
        }
    }
}
