//! Gun systems and components
//!
//! The main component is the GunInventory. It contains all the gun from the player. When a gun
//! inventory (non-empty) is added, the player will automatically have a Gun component with the first
//! gun in the inventory. This will track the amount of ammo in the gun.
//!
//! When the player switches gun, the current gun's ammo will be saved in the inventory.

use crate::event::GameEvent;
use crate::gameplay::player::MainPlayer;
use crate::net::snapshot::Deltable;
use crate::resources::Resources;
use hecs::World;
use log::info;
use serde_derive::{Deserialize, Serialize};
use shrev::{EventChannel, ReaderId};
use std::collections::HashMap;
use std::time::Duration;

/// Current gun. Will have the ammos, the weapon type and the amount of time to wait before the
/// player can shoot again.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct Gun {
    /// What gun is it?
    pub gun_type: GunType,

    /// Current amount of ammo in the gun
    pub ammo: i32,

    /// When 0, the player can shoot
    countdown: f32,
}

impl Gun {
    pub fn can_shoot(&self) -> bool {
        info!("countdown and ammo {} {}", self.countdown, self.ammo);
        self.countdown <= 0.0 && self.ammo > 0
    }

    /// Decrease the amount of ammo and reset countdown
    pub fn shoot(&mut self) {
        self.ammo = 0i32.max(self.ammo - 1);
        self.countdown = self.gun_type.get_time_to_wait();
    }
}

impl Deltable for Gun {
    type Delta = (Option<GunType>, Option<i32>, Option<f32>);

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        let delta_type = {
            if self.gun_type != old.gun_type {
                Some(self.gun_type)
            } else {
                None
            }
        };

        let delta_ammo = if self.ammo != old.ammo {
            Some(self.ammo)
        } else {
            None
        };

        let delta_t = if self.countdown != old.countdown {
            Some(self.countdown)
        } else {
            None
        };
        match (delta_type, delta_ammo, delta_t) {
            (None, None, None) => None,
            (a, b, c) => Some((a, b, c)),
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some((Some(self.gun_type), Some(self.ammo), Some(self.countdown)))
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        if let Some(gt) = delta.0 {
            self.gun_type = gt;
        }

        if let Some(ammo) = delta.1 {
            self.ammo = ammo;
        }

        if let Some(t) = delta.2 {
            self.countdown = t;
        }
    }

    fn new_component(delta: &Self::Delta) -> Self {
        let mut def = Gun::default();
        def.apply_delta(delta);
        def
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum GunType {
    Shotgun,
    Pistol,
}

impl Default for GunType {
    fn default() -> Self {
        GunType::Pistol
    }
}

impl GunType {
    /// Return the maximum amount of ammo this gun can have
    pub fn get_max_ammo(self) -> i32 {
        match self {
            GunType::Pistol => 30,
            GunType::Shotgun => 15,
        }
    }

    /// Return the amount of time to wait between two shots.
    pub fn get_time_to_wait(self) -> f32 {
        match self {
            GunType::Pistol => 0.2,
            GunType::Shotgun => 0.75,
        }
    }

    pub fn get_ammo_pickup(self) -> i32 {
        match self {
            GunType::Pistol => 10,
            GunType::Shotgun => 4,
        }
    }

    pub fn get_gun_slot(self) -> GunSlot {
        match self {
            GunType::Pistol => 1,
            GunType::Shotgun => 0,
        }
    }

    pub fn get_prefab_path(self) -> String {
        let filename = match self {
            GunType::Pistol => "pistol",
            GunType::Shotgun => "shotgun",
        };

        format!(
            "{}prefab/{}.ron",
            std::env::var("ASSET_PATH").unwrap(),
            filename
        )
    }
}

pub type GunSlot = usize;
/// Contains the weapons of a player.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
pub struct GunInventory {
    guns: HashMap<GunSlot, Gun>,
}

// TODO really rough
impl Deltable for GunInventory {
    type Delta = GunInventory;
    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self != old {
            Some(self.clone())
        } else {
            None
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(self.clone())
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        *self = delta.clone();
    }

    fn new_component(delta: &Self::Delta) -> Self {
        delta.clone()
    }
}

impl GunInventory {
    pub fn get_gun(&self, slot: GunSlot) -> Option<&Gun> {
        self.guns.get(&slot)
    }

    pub fn get_gun_mut(&mut self, slot: GunSlot) -> Option<&mut Gun> {
        self.guns.get_mut(&slot)
    }

    pub fn get_first(&self) -> Option<Gun> {
        self.guns.iter().map(|(_, v)| *v).next()
    }

    /// Will save the current gun and return the new gun.
    pub fn switch_gun(&mut self, current_gun: Gun, next_gun: GunSlot) -> Option<Gun> {
        if current_gun.gun_type.get_gun_slot() == next_gun {
            return None;
        }
        if let Some(&next_gun) = self.get_gun(next_gun) {
            // save current gun.
            self.guns
                .insert(current_gun.gun_type.get_gun_slot(), current_gun);
            Some(next_gun)
        } else {
            None
        }
    }

    pub fn has_gun(&self, gun: GunType) -> bool {
        self.guns.contains_key(&gun.get_gun_slot())
    }
}

/// Will update the countdown of guns
pub struct GunSystem {
    rdr_id: ReaderId<GameEvent>,
}

impl GunSystem {
    pub fn new(resources: &mut Resources) -> Self {
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        Self {
            rdr_id: chan.register_reader(),
        }
    }
    pub fn update(&mut self, world: &mut World, dt: Duration, resources: &mut Resources) {
        let as_secs = dt.as_secs_f32();
        for (_, g) in world.query::<&mut Gun>().iter() {
            g.countdown = 0.0f32.max(g.countdown - as_secs);
        }

        // If there is any pick up event :)
        let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
        let mut to_send = vec![];
        for ev in chan.read(&mut self.rdr_id) {
            match ev {
                GameEvent::PickupAmmo { entity, gun } => {
                    info!("Got pickup ammo event");
                    let mut inventory = world
                        .get_mut::<GunInventory>(*entity)
                        .expect("Entity should have inventory");
                    let mut current_gun = world
                        .get_mut::<Gun>(*entity)
                        .expect("Player should have gun.");
                    if let Some(g) = inventory.get_gun_mut(gun.get_gun_slot()) {
                        g.ammo += gun.get_ammo_pickup();
                        if current_gun.gun_type == *gun {
                            current_gun.ammo += gun.get_ammo_pickup();
                        }
                        info!("New gun ammo is {}", g.ammo);
                        if world.get::<MainPlayer>(*entity).is_ok() {
                            to_send.push(GameEvent::AmmoChanged);
                        }
                    }
                }
                GameEvent::PickupGun { entity, gun } => {
                    let mut inventory = world
                        .get_mut::<GunInventory>(*entity)
                        .expect("Entity should have inventory");
                    if let Some(g) = inventory.get_gun_mut(gun.get_gun_slot()) {
                        g.ammo += gun.get_ammo_pickup();
                    } else {
                        inventory.guns.insert(
                            gun.get_gun_slot(),
                            Gun {
                                ammo: gun.get_max_ammo(),
                                countdown: 0.0,
                                gun_type: *gun,
                            },
                        );
                    }
                    if world.get::<MainPlayer>(*entity).is_ok() {
                        to_send.push(GameEvent::GunChanged);
                    }
                }
                _ => (),
            }
        }
        chan.drain_vec_write(&mut to_send);
    }
}
