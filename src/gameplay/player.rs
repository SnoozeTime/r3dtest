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
use std::fs;

use crate::gameplay::health::Health;
use crate::net::snapshot::Deltable;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum PlayerState {
    Alive,
    Dead,
    Respawn,
}

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
            state: PlayerState::Alive,
            nb: 0,
        },
        player_health,
    ));

    body_to_entity.insert(idx, e);
    e
}

/// Spawn the entities that has the sprites (crosshair, gun...)
pub fn spawn_player_ui(world: &mut World) {
    let ui_str = fs::read_to_string("config/ui.ron").unwrap();
    let ui_entities: Vec<serialization::SerializedEntity> = ron::de::from_str(&ui_str).unwrap();
    serialization::add_to_world(world, ui_entities);

    //    // crosshair.
    //    let screen_pos = ScreenPosition {
    //        x: 0.5,
    //        y: 0.5,
    //        w: 0.02,
    //        h: 0.02,
    //    };
    //    let sprite = SpriteRender {
    //        texture: "crosshair".to_string(),
    //        sprite_nb: 0,
    //    };
    //
    //    world.spawn((screen_pos, sprite));
    //
    //    // gun
    //    {
    //        let screen_pos = ScreenPosition {
    //            x: 0.75,
    //            y: 0.15,
    //            w: 0.20,
    //            h: 0.20,
    //        };
    //        let sprite = SpriteRender {
    //            texture: "shotgun".to_string(),
    //            sprite_nb: 0,
    //        };
    //        world.spawn((screen_pos, sprite));
    //    }
}
