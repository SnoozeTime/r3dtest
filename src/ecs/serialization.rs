#![allow(unused)]
use super::Transform;
use crate::animation::AnimationController;
use crate::camera::{Camera, LookAt};
use crate::colors::RgbColor;
use crate::controller::Fps;
use crate::gameplay::{
    gun::Gun, gun::GunInventory, health::Health, pickup::PickUp, player::Player,
};
use crate::physics::RigidBody;
use crate::render::{
    billboard::Billboard,
    debug::DebugRender,
    lighting::{AmbientLight, DirectionalLight},
    particle::ParticleEmitter,
    sprite::{ScreenPosition, SpriteRender},
    Render,
};
use hecs::World;
use serde_derive::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Error deserializing World = {0}")]
    DeserializeError(ron::de::Error),

    #[error("Error serializing World = {0}")]
    SerializeError(ron::ser::Error),
}

fn get_component<T>(world: &World, e: hecs::Entity) -> Option<T>
where
    T: Clone + Send + Sync + 'static,
{
    world.get::<T>(e).ok().map(|c| (*c).clone())
}

macro_rules! serialize {
    ($(($name:ident, $component:ty)),+) => {


        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct SerializedEntity {
            $(
                #[serde(skip_serializing_if = "Option::is_none")]
                #[serde(default)]
                pub $name: Option<$component>
            ),+

        }

        pub fn deserialize_world(world_str: String) -> Result<hecs::World, SerializationError> {
            let mut world = World::new();
            let serialized_entities: Vec<SerializedEntity> =
                ron::de::from_str(&world_str).map_err(SerializationError::DeserializeError)?;

            add_to_world(&mut world, serialized_entities);
            Ok(world)
        }

        pub fn add_to_world(world: &mut World, serialized_entities: Vec<SerializedEntity>) {

            let mut builder = hecs::EntityBuilder::new();
            for e in serialized_entities {

                $(
                    if let Some(c) = e.$name {
                        builder.add(c);
                    }
                )+

                world.spawn(builder.build());
            }
        }

        pub fn spawn_entity(world: &mut World, serialized: &SerializedEntity) -> hecs::Entity {
            let mut builder = hecs::EntityBuilder::new();
            $(
                if let Some(ref c) = serialized.$name {
                  builder.add(c.clone());
                    }
            )+

            world.spawn(builder.build())
        }

        pub fn serialize_entities(world: &hecs::World) -> Vec<SerializedEntity> {
            let entities: Vec<_> = world
                .iter()
                .filter_map(|(e, _)| {
                    // Now, add the components to serialize...
                    let mut one_not_none = false;
                    $(
                        let $name = get_component::<$component>(world, e);
                        if $name.is_some() {
                            one_not_none = true;
                        }
                    )+

                    if one_not_none {
                        Some(SerializedEntity {
                            $(
                                $name
                            ),+
                        })
                    } else {
                        None
                    }
                })
                .collect();

            entities
        }

        pub fn serialize_world(world: &hecs::World) -> Result<String, SerializationError> {
            let entities = serialize_entities(world);

            ron::ser::to_string_pretty(&entities, ron::ser::PrettyConfig::default()).map_err(SerializationError::SerializeError)
        }

    };
}

serialize! {
    (transform, Transform),
    (render, Render),
    (rigid_body, RigidBody),
    (color, RgbColor),
    (camera, Camera),
    (fps, Fps),
    (health, Health),
    (sprite, SpriteRender),
    (screen_position, ScreenPosition),
    (animation, AnimationController),
    (billboard, Billboard),
    (look_at, LookAt),
    (debug_render, DebugRender),
    (player, Player),
    (gun, Gun),
    (gun_inventory, GunInventory),
    (pickup, PickUp),
    (particle, ParticleEmitter),
    (ambient_light, AmbientLight),
    (directional_light, DirectionalLight)
}
