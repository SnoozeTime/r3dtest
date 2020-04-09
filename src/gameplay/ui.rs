//! Stuff displayed on the screen (2D)
//! Health, armor, gun, ammos and so on.

use crate::animation::{Animation, AnimationController};
use crate::colors::RgbColor;
use crate::ecs::serialization::SerializedEntity;
use crate::event::GameEvent;
use crate::gameplay::gun::Gun;
use crate::gameplay::health::Health;
use crate::gameplay::player::MainPlayer;
use crate::render::sprite::{ScreenPosition, SpriteRender};
use crate::render::text::Text;
use crate::resources::Resources;
use log::info;
use shrev::{EventChannel, ReaderId};
use std::collections::HashMap;
use std::fs;

pub struct UiSystem {
    health_entity: hecs::Entity,
    ammo_entity: hecs::Entity,
    _armor_entity: hecs::Entity,
    _crosshair_entity: hecs::Entity,
    weapon_entity: Option<hecs::Entity>,
    rdr_id: ReaderId<GameEvent>,
}

impl UiSystem {
    /// Create all the UI entities :)
    pub fn new(world: &mut hecs::World, resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();

        let rdr_id = chan.register_reader();
        chan.single_write(GameEvent::UpdateText);

        let health_entity = spawn_health_counter(world);
        let ammo_entity = spawn_ammo_counter(world);
        let armor_entity = spawn_armor_counter(world);

        let weapon_entity = spawn_weapon(world);
        let crosshair_entity = spawn_crosshair(world);
        Self {
            health_entity,
            ammo_entity,
            _armor_entity: armor_entity,
            weapon_entity,
            _crosshair_entity: crosshair_entity,
            rdr_id,
        }
    }

    pub fn update(&mut self, world: &mut hecs::World, resources: &mut Resources) {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();

        let mut should_update = false;
        for ev in chan.read(&mut self.rdr_id) {
            match ev {
                GameEvent::HealthUpdate { entity, new_health } => {
                    println!("HEALTH UPDATE EVENT {:?}", ev);
                    if world.get::<MainPlayer>(*entity).is_ok() {
                        // we can update the health counter.
                        let mut text = world.get_mut::<Text>(self.health_entity).unwrap();
                        text.content = format!("{}", new_health);
                        should_update = true;
                    }
                }
                GameEvent::Shoot | GameEvent::AmmoChanged => {
                    if let Some(weapon_entity) = self.weapon_entity {
                        let mut animation =
                            world.get_mut::<AnimationController>(weapon_entity).unwrap();
                        animation.current_animation = Some("shoot".to_string());
                    }

                    if self.update_ammo(world) {
                        should_update = true;
                    }
                }
                GameEvent::GunChanged => {
                    info!("Gun changed event in UI");
                    if let Some(e) = self.weapon_entity {
                        world.despawn(e).unwrap();
                    }
                    self.weapon_entity = spawn_weapon(world);
                    if self.update_ammo(world) {
                        should_update = true;
                    }
                }
                _ => (),
            }
        }

        if should_update {
            chan.single_write(GameEvent::UpdateText);
        }
    }

    fn update_ammo(&self, world: &hecs::World) -> bool {
        let mut should_update = false;
        if let Some(weapon_entity) = self.weapon_entity {
            if let Some((_, (g, _))) = world.query::<(&Gun, &MainPlayer)>().iter().next() {
                let mut text = world.get_mut::<Text>(self.ammo_entity).unwrap();
                text.content = format!("{}", g.ammo);
                should_update = true;
            }
        }
        should_update
    }
}

fn spawn_health_counter(world: &mut hecs::World) -> hecs::Entity {
    let h = if let Some((_, (h, _))) = world.query::<(&Health, &MainPlayer)>().iter().next() {
        format!("{}", h.current)
    } else {
        "100".to_string()
    };
    let e = world.spawn((
        Text {
            content: h,
            font_size: 50.0,
        },
        ScreenPosition {
            x: 0.02,
            y: 0.01,
            ..ScreenPosition::default()
        },
        RgbColor { r: 255, g: 0, b: 0 },
    ));

    e
}

fn spawn_ammo_counter(world: &mut hecs::World) -> hecs::Entity {
    let h = if let Some((_, (g, _))) = world.query::<(&Gun, &MainPlayer)>().iter().next() {
        format!("{}", g.ammo)
    } else {
        "0".to_string()
    };

    let e = world.spawn((
        Text {
            content: h,
            font_size: 25.0,
        },
        ScreenPosition {
            x: 0.7,
            y: 0.02,
            ..ScreenPosition::default()
        },
        RgbColor {
            r: 177,
            g: 177,
            b: 177,
        },
    ));

    e
}

fn spawn_armor_counter(world: &mut hecs::World) -> hecs::Entity {
    let e = world.spawn((
        Text {
            content: "0".to_string(),
            font_size: 25.0,
        },
        ScreenPosition {
            x: 0.1,
            y: 0.02,
            ..ScreenPosition::default()
        },
        RgbColor { r: 0, g: 0, b: 255 },
    ));

    e
}

fn spawn_weapon(world: &mut hecs::World) -> Option<hecs::Entity> {
    let prefab = if let Some((e, (_, g))) = world.query::<(&MainPlayer, &Gun)>().iter().next() {
        let prefab_path = g.gun_type.get_prefab_path();
        let ser: SerializedEntity =
            ron::de::from_str(&fs::read_to_string(&prefab_path).unwrap()).unwrap();
        Some(ser)
    } else {
        None
    };

    prefab.and_then(|ser| {
        let e = crate::ecs::serialization::spawn_entity(world, &ser);
        Some(e)
    })
    //    let mut animations = HashMap::new();
    //    animations.insert(
    //        "shoot".to_string(),
    //        Animation {
    //            keyframes: vec![(1, 4), (0, 4)],
    //            single: true,
    //            elapsed_frame: 0,
    //            current_index: 0,
    //        },
    //    );
    //
    //    let e = world.spawn((
    //        ScreenPosition {
    //            x: 0.75,
    //            y: 0.15,
    //            w: 0.2,
    //            h: 0.2,
    //        },
    //        SpriteRender {
    //            sprite_nb: 0,
    //            texture: String::from("pistol"),
    //        },
    //        AnimationController {
    //            animations,
    //            current_animation: None,
    //        },
    //    ));
    //
    //    e
}

fn spawn_crosshair(world: &mut hecs::World) -> hecs::Entity {
    let e = world.spawn((
        ScreenPosition {
            x: 0.5,
            y: 0.5,
            w: 0.01,
            h: 0.01,
        },
        SpriteRender {
            sprite_nb: 0,
            texture: String::from("crosshair"),
        },
    ));

    e
}
