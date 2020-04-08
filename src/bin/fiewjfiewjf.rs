#![allow(warnings)]
use glam::Vec3;
use r3dtest::animation::*;
use std::collections::HashMap;

fn main() {
    let lookat = Vec3::new(1.0, 0.0, 1.0).normalize();

    let dir = Vec3::new(-1.1, 1.0, 1.0);
    println!("{:?}", lookat.dot(dir));
}
