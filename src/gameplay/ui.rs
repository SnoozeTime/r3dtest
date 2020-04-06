//! Stuff displayed on the screen (2D)
//! Health, armor, gun, ammos and so on.

use crate::colors::RgbColor;
use crate::event::GameEvent;
use crate::gameplay::health::Health;
use crate::gameplay::player::MainPlayer;
use crate::render::sprite::ScreenPosition;
use crate::render::text::Text;
use crate::resources::Resources;
use shrev::{EventChannel, ReaderId};

pub struct UiSystem {
    health_entity: hecs::Entity,
    armor_entity: hecs::Entity,
    rdr_id: ReaderId<GameEvent>,
}

impl UiSystem {
    /// Create all the UI entities :)
    pub fn new(world: &mut hecs::World, resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();

        let rdr_id = chan.register_reader();
        chan.single_write(GameEvent::UpdateText);

        let health_entity = spawn_health_counter(world);
        let armor_entity = spawn_armor_counter(world);
        Self {
            health_entity,
            armor_entity,
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
                _ => (),
            }
        }

        if should_update {
            chan.single_write(GameEvent::UpdateText);
        }
    }
}

fn spawn_health_counter(world: &mut hecs::World) -> hecs::Entity {
    let h = if let Some((e, (h, _))) = world.query::<(&Health, &MainPlayer)>().iter().next() {
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
