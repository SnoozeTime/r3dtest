//! Gun systems and components
//!
//! The main component is the GunInventory. It contains all the gun from the player. When a gun
//! inventory (non-empty) is added, the player will automatically have a Gun component with the first
//! gun in the inventory. This will track the amount of ammo in the gun.
//!
//! When the player switches gun, the current gun's ammo will be saved in the inventory.

use crate::net::snapshot::Deltable;
use hecs::World;
use log::info;
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
}

/// Will update the countdown of guns
pub struct GunSystem;

impl GunSystem {
    pub fn update(&self, world: &mut World, dt: Duration) {
        let as_secs = dt.as_secs_f32();
        for (_, g) in world.query::<&mut Gun>().iter() {
            g.countdown = 0.0f32.max(g.countdown - as_secs);
        }
    }
}
