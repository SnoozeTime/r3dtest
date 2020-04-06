#[allow(unused)]
pub const PASTEL_RED: RgbColor = RgbColor::new(212, 80, 121);
#[allow(unused)]
pub const PASTEL_PURPLE: RgbColor = RgbColor::new(110, 87, 115);
#[allow(unused)]
pub const PASTEL_ORANGE: RgbColor = RgbColor::new(234, 144, 133);
#[allow(unused)]
pub const PASTEL_BEIGE: RgbColor = RgbColor::new(233, 225, 204);
#[allow(unused)]
pub const RED: RgbColor = RgbColor::new(255, 0, 0);
#[allow(unused)]
pub const BLUE: RgbColor = RgbColor::new(0, 0, 255);
#[allow(unused)]
pub const GREEN: RgbColor = RgbColor::new(0, 255, 0);
use crate::net::snapshot::Deltable;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_normalized(self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }

    pub fn to_rgba_normalized(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            1.0,
        ]
    }
}

impl Deltable for RgbColor {
    type Delta = RgbColor;

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
        self.r = delta.r;
        self.g = delta.g;
        self.b = delta.b;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        *delta
    }
}
