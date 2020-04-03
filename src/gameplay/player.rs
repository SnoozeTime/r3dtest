use crate::camera::Camera;
use crate::colors;
use crate::controller::Fps;
use crate::ecs::Transform;
use crate::physics::Shape::AABB;
use crate::physics::{BodyToEntity, BodyType, PhysicWorld, RigidBody};
use crate::render::Render;
use crate::resources::Resources;
use hecs::Entity;

pub struct Player;

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

    let e = world.spawn((transform, cam, rb, fps, mesh, color));

    body_to_entity.insert(idx, e);
    e
}
