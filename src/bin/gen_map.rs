#![allow(warnings)]
use glam::Vec3;
use r3dtest::animation::*;
use std::collections::HashMap;

use r3dtest::physics::{BodyType, Shape};
use r3dtest::{
    colors::RgbColor, ecs::serialization, ecs::Transform, physics::RigidBody, render::Render,
};

const CELL_SIZE: f32 = 10.0;
const HALF_CELL: f32 = CELL_SIZE / 2.0;

fn spawn_floor(w: f32, h: f32) -> (serialization::SerializedEntity, f32, f32) {
    let (x_center, z_center) = (0.0f32, 0.0);
    let y_center = -HALF_CELL;
    let t = Transform {
        translation: glam::vec3(x_center, y_center, z_center),
        rotation: glam::Quat::identity(),
        scale: glam::vec3(w * HALF_CELL, HALF_CELL, h * HALF_CELL),
    };

    let render = Render {
        enabled: true,
        mesh: String::from("cube"),
    };

    let color = r3dtest::colors::PASTEL_PURPLE;

    let rb = RigidBody {
        shape: Shape::AABB(glam::vec3(w * HALF_CELL, HALF_CELL, h * HALF_CELL)),
        ty: BodyType::Static,
        mass: 100.0,
        handle: None,
    };
    let (x_offset, z_offset) = (w * CELL_SIZE / 2.0, h * CELL_SIZE / 2.0);

    (
        serialization::SerializedEntity {
            rigid_body: Some(rb),
            transform: Some(t),
            render: Some(render),
            color: Some(color),
            ..serialization::SerializedEntity::default()
        },
        x_offset,
        z_offset,
    )
}
fn spawn_block(x: f32, z: f32, x_offset: f32, z_offset: f32) -> serialization::SerializedEntity {
    let (x_center, z_center) = (
        x * CELL_SIZE + HALF_CELL - x_offset,
        z * CELL_SIZE + HALF_CELL - z_offset,
    );
    let y_center = HALF_CELL;
    let t = Transform {
        translation: glam::vec3(x_center, y_center, z_center),
        rotation: glam::Quat::identity(),
        scale: glam::vec3(HALF_CELL, HALF_CELL, HALF_CELL),
    };

    let render = Render {
        enabled: true,
        mesh: String::from("cube"),
    };

    let color = r3dtest::colors::PASTEL_RED;

    let rb = RigidBody {
        shape: Shape::AABB(glam::vec3(HALF_CELL, HALF_CELL, HALF_CELL)),
        ty: BodyType::Static,
        mass: 100.0,
        handle: None,
    };

    serialization::SerializedEntity {
        rigid_body: Some(rb),
        transform: Some(t),
        render: Some(render),
        color: Some(color),
        ..serialization::SerializedEntity::default()
    }
}
fn main() {
    let cell_size = 2.0f32;
    let level = std::fs::read_to_string("./assets/map/test").unwrap();

    let rows: Vec<_> = level.split("\n").collect();
    let floor_height = rows.len() as f32;
    let floor_width = rows[0].len() as f32;

    let (floor_entity, x_offset, z_offset) = spawn_floor(floor_width, floor_height);
    let mut to_ser = vec![floor_entity];

    // For each # let's create a cube.
    for (z, row) in rows.iter().enumerate() {
        for (x, cell) in row.chars().enumerate() {
            if cell == '#' {
                to_ser.push(spawn_block(x as f32, z as f32, x_offset, z_offset));
            }
        }
    }

    let serialized = ron::ser::to_string_pretty(&to_ser, ron::ser::PrettyConfig::default());
    std::fs::write("bonjour.ron", serialized.unwrap()).unwrap();
}
