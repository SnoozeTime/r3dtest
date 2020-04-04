use crate::camera::Camera;
use crate::controller::client::ClientCommand;
use crate::event::{Event, GameEvent};
use crate::physics::{BodyIndex, BodyToEntity, PhysicWorld, RigidBody};
use crate::resources::Resources;
use hecs::Entity;
#[allow(unused_imports)]
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use shrev::EventChannel;

pub mod client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fps {
    pub speed: f32,
    pub sensitivity: f32,

    #[serde(skip)]
    pub jumping: bool,

    #[serde(skip)]
    pub on_ground: bool,
}

pub fn apply_inputs(
    inputs: Vec<(Entity, Event)>,
    world: &mut hecs::World,
    physics: &mut PhysicWorld,
    resources: &Resources,
) {
    for (e, ev) in inputs {
        match ev {
            Event::Client(cmd) => {
                apply_cmd(e, cmd, world, physics, resources);
            }
            _ => (),
        }
    }
}

fn apply_cmd(
    e: Entity,
    cmd: ClientCommand,
    world: &mut hecs::World,
    physics: &mut PhysicWorld,
    resources: &Resources,
) {
    match cmd {
        ClientCommand::LookAt(pitch, yaw) => {
            let mut camera = world.get_mut::<Camera>(e).unwrap();
            camera.pitch = pitch;
            camera.yaw = yaw;
            camera.compute_vectors();
        }
        ClientCommand::Move(dir) => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let fps = world.get::<Fps>(e).unwrap();
            let h = rb.handle.unwrap();
            physics.add_velocity_change(h, dir.normalize() * fps.speed);
        }
        ClientCommand::Jump => {
            let rb = world.get::<RigidBody>(e).unwrap();
            let mut fps = world.get_mut::<Fps>(e).unwrap();

            if fps.on_ground {
                info!("JUMP");
                physics.add_velocity_change(rb.handle.unwrap(), 20.0 * glam::Vec3::unit_y());
                fps.jumping = true;
                physics.set_friction(rb.handle.unwrap(), 0.0);
            }
        }
        ClientCommand::Shoot => {
            let camera = world.get::<Camera>(e).unwrap();
            let rb = world.get::<RigidBody>(e).unwrap();
            let h = rb.handle.unwrap();
            println!("CAMERA IS {:?}", *camera);
            println!(
                "Will raycast from {:?} direction {:?}",
                physics.get_pos(h),
                camera.front
            );

            let mut d = physics.raycast(h, glam::vec3(0.0, 0.0, 0.0), camera.front);
            println!("{:?}", d);
            if let Some(ev) = create_shot_event(d, resources) {
                let mut event_channel = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                event_channel.single_write(ev);
            }
        }
    }
}

pub struct Controller;

impl Controller {
    pub fn apply_inputs(
        &self,
        inputs: Vec<(Entity, Event)>,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) {
        for (e, ev) in inputs {
            match ev {
                Event::Client(cmd) => {
                    apply_cmd(e, cmd, world, physics, resources);
                }
                _ => (),
            }
        }
    }

    /// Check at each frames if the body is on ground.
    pub fn update(
        &self,
        world: &mut hecs::World,
        physics: &mut PhysicWorld,
        _resources: &Resources,
    ) {
        for (_, (fps, rb)) in world.query::<(&mut Fps, &RigidBody)>().iter() {
            let h = rb.handle.unwrap();
            let on_ground = {
                let mut d = physics.raycast(h, glam::vec3(0.0, 0.0, 0.0), -glam::Vec3::unit_y());
                d.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

                debug!("Raycast on_ground = {:?}", d);
                if let Some((minimum_distance, _)) = d.pop() {
                    if minimum_distance < 2.5 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if on_ground {
                trace!(" NOW ON GROUND!");
            }
            fps.on_ground = on_ground;

            if fps.jumping && on_ground {
                physics.set_friction(h, 0.1);
                fps.jumping = false;
            }
        }
    }
}

fn create_shot_event(
    raycast_result: Vec<(f32, BodyIndex)>,
    resources: &Resources,
) -> Option<GameEvent> {
    raycast_result
        .iter()
        .map(|(_, h)| {
            info!("Body to entity");
            let body_to_entity = resources.fetch::<BodyToEntity>().unwrap();
            info!("Get entity");
            let entity = body_to_entity.get(&h).unwrap();
            GameEvent::EntityShot { entity: *entity }
        })
        .next()
}

//
//pub enum Controller {
//    Fps(FpsController),
//    Free(FreeController),
//}
//
//impl Controller {
//    pub fn update(
//        &self,
//        input: &Input,
//        transform: &mut Transform,
//        fps_camera: &mut Camera,
//    ) -> bool {
//        match self {
//            Controller::Free(ref free) => free.update_pos(input, transform, fps_camera),
//            Controller::Fps(ref fps) => fps.update_pos(input, transform, fps_camera),
//        }
//    }
//}
//
//pub struct FpsController {
//    speed: f32,
//    sensitivity: f32,
//}
//
//impl Default for FpsController {
//    fn default() -> Self {
//        Self {
//            sensitivity: 0.005,
//            speed: 0.1,
//        }
//    }
//}
//
//impl FpsController {
//    pub fn update_pos(
//        &self,
//        input: &Input,
//        transform: &mut Transform,
//        fps_camera: &mut Camera,
//    ) -> bool {
//        let mut has_moved = false;
//
//        let mut lateral_dir = {
//            if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
//                Some(Direction::Left)
//            } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
//                Some(Direction::Right)
//            } else {
//                None
//            }
//        };
//        let mut forward_dir = {
//            if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
//                Some(Direction::Forward)
//            } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
//                Some(Direction::Backward)
//            } else {
//                None
//            }
//        };
//
//        match (lateral_dir, forward_dir) {
//            (Some(ld), Some(fd)) => {
//                self.move_along(ld, self.speed / 2.0, transform, fps_camera);
//                self.move_along(fd, self.speed / 2.0, transform, fps_camera);
//                has_moved = true;
//            }
//            (Some(ld), None) => {
//                self.move_along(ld, self.speed, transform, fps_camera);
//                has_moved = true;
//            }
//            (None, Some(fd)) => {
//                self.move_along(fd, self.speed, transform, fps_camera);
//                has_moved = true;
//            }
//            _ => (),
//        }
//
//        if let Some((offset_x, offset_y)) = input.mouse_delta {
//            self.apply_delta_dir(offset_x, offset_y, fps_camera);
//            has_moved = true;
//        }
//        has_moved
//    }
//
//    /// Move the camera position according to a direction and with a speed.
//    fn move_along(
//        &self,
//        direction: Direction,
//        speed: f32,
//        transform: &mut Transform,
//        camera: &Camera,
//    ) {
//        let dir = match direction {
//            Direction::Left => camera.left,
//            Direction::Right => -camera.left,
//            Direction::Forward => camera.left.cross(glam::Vec3::unit_y()),
//            Direction::Backward => -camera.left.cross(glam::Vec3::unit_y()),
//        };
//
//        transform.translation += dir * speed;
//    }
//
//    fn apply_delta_dir(&self, offset_x: f32, offset_y: f32, camera: &mut Camera) {
//        camera.yaw += offset_x * self.sensitivity;
//        camera.pitch += offset_y * self.sensitivity;
//        if camera.pitch >= 89.0 {
//            camera.pitch = 89.0;
//        }
//        if camera.pitch <= -89.0 {
//            camera.pitch = -89.0;
//        }
//
//        camera.compute_vectors();
//    }
//}
//
//pub struct FreeController {
//    speed: f32,
//    sensitivity: f32,
//}
//
//impl Default for FreeController {
//    fn default() -> Self {
//        Self {
//            sensitivity: 0.005,
//            speed: 0.1,
//        }
//    }
//}
//
//impl FreeController {
//    pub fn update_pos(
//        &self,
//        input: &Input,
//        transform: &mut Transform,
//        fps_camera: &mut Camera,
//    ) -> bool {
//        let mut has_moved = false;
//
//        let mut lateral_dir = {
//            if input.key_down.contains(&Key::Left) || input.key_down.contains(&Key::A) {
//                Some(Direction::Left)
//            } else if input.key_down.contains(&Key::Right) || input.key_down.contains(&Key::D) {
//                Some(Direction::Right)
//            } else {
//                None
//            }
//        };
//        let mut forward_dir = {
//            if input.key_down.contains(&Key::Up) || input.key_down.contains(&Key::W) {
//                Some(Direction::Forward)
//            } else if input.key_down.contains(&Key::Down) || input.key_down.contains(&Key::S) {
//                Some(Direction::Backward)
//            } else {
//                None
//            }
//        };
//
//        match (lateral_dir, forward_dir) {
//            (Some(ld), Some(fd)) => {
//                self.move_along(ld, self.speed / 2.0, transform, fps_camera);
//                self.move_along(fd, self.speed / 2.0, transform, fps_camera);
//                has_moved = true;
//            }
//            (Some(ld), None) => {
//                self.move_along(ld, self.speed, transform, fps_camera);
//                has_moved = true;
//            }
//            (None, Some(fd)) => {
//                self.move_along(fd, self.speed, transform, fps_camera);
//                has_moved = true;
//            }
//            _ => (),
//        }
//
//        if let Some((offset_x, offset_y)) = input.mouse_delta {
//            self.apply_delta_dir(offset_x, offset_y, fps_camera);
//            has_moved = true;
//        }
//        has_moved
//    }
//
//    /// Move the camera position according to a direction and with a speed.
//    fn move_along(
//        &self,
//        direction: Direction,
//        speed: f32,
//        transform: &mut Transform,
//        camera: &Camera,
//    ) {
//        let dir = match direction {
//            Direction::Left => camera.left,
//            Direction::Right => -camera.left,
//            Direction::Forward => camera.front,
//            Direction::Backward => -camera.front,
//        };
//
//        transform.translation += dir * speed;
//    }
//
//    fn apply_delta_dir(&self, offset_x: f32, offset_y: f32, camera: &mut Camera) {
//        camera.yaw += offset_x * self.sensitivity;
//        camera.pitch += offset_y * self.sensitivity;
//        if camera.pitch >= 89.0 {
//            camera.pitch = 89.0;
//        }
//        if camera.pitch <= -89.0 {
//            camera.pitch = -89.0;
//        }
//
//        camera.compute_vectors();
//    }
//}
