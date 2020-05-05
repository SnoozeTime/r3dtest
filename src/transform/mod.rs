//! Transform is the position of a game object in the world space.
//! Some entities can have children. For example, a robot will have its arms and legs that will
//! move relative the the robot body center, in local space.
use glam::{Mat4, Quat, Vec3};
use serde_derive::{Deserialize, Serialize};
// TODO move Transform here.
use crate::ecs::Transform;
use log::error;
use std::collections::VecDeque;

/// Transform relative the the parent component.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalTransform {
    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,

    #[serde(default = "default_dirty")]
    pub dirty: bool,
}

impl From<Transform> for LocalTransform {
    fn from(t: Transform) -> Self {
        Self {
            translation: t.translation,
            scale: t.scale,
            rotation: t.rotation,
            dirty: true,
        }
    }
}

fn default_dirty() -> bool {
    true
}
impl LocalTransform {
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            scale,
            rotation,
            dirty: true,
        }
    }

    pub fn to_model(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

pub struct HasParent {
    pub entity: hecs::Entity,
}

pub struct HasChildren {
    pub children: Vec<hecs::Entity>,
}

pub fn update_transforms(world: &mut hecs::World) {
    let mut to_process = VecDeque::new();
    /// first gather the entities to update.
    for (e, (transform, has_children)) in world.query::<(&mut Transform, &HasChildren)>().iter() {
        // Root entities.
        if let Ok(_) = world.get::<HasParent>(e) {
            continue;
        }

        // Process all parents even if their transform is not dirty. The reason is that children
        // can be moved independently, so we would need to update their children.
        for child in &has_children.children {
            to_process.push_back((transform.clone(), *child));
        }
        transform.dirty = false;
    }

    // process in order of insertion.
    while let Some((t, child)) = to_process.pop_front() {
        let parent_matrix = t.to_model();
        // First, calculate the new transform.
        let mut global_transform = world
            .get_mut::<Transform>(child)
            .expect("Child component should have a global transform");
        let mut local_transform = world
            .get_mut::<LocalTransform>(child)
            .expect("Child component should have a local transform");

        if local_transform.dirty || t.dirty {
            // Need to recalculate the global transform.
            let local_matrix = local_transform.to_model();
            let new_global_matrix = parent_matrix * local_matrix;
            let (scale, rot, translation) = new_global_matrix.to_scale_rotation_translation();
            global_transform.scale = scale;
            global_transform.rotation = rot.normalize();
            global_transform.translation = translation;
            global_transform.dirty = true;
        }

        if let Ok(children) = world.get::<HasChildren>(child) {
            for child_of_child in &children.children {
                to_process.push_back((*global_transform, *child_of_child));
            }
        }

        global_transform.dirty = false;
        local_transform.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec_eq(vec1: glam::Vec3, vec2: glam::Vec3) {
        println!("Compare vec1 {:?} with vec2 {:?}", vec1, vec2);
        assert!((vec1.x() - vec2.x()).abs() < 0.00001);
        assert!((vec1.y() - vec2.y()).abs() < 0.00001);
        assert!((vec1.z() - vec2.z()).abs() < 0.00001);
    }

    fn assert_quat_eq(q1: Quat, q2: Quat) {
        let q1: [f32; 4] = q1.into();
        let q2: [f32; 4] = q2.into();
        assert!((q1[0] - q2[0]).abs() < 0.00001);
        assert!((q1[1] - q2[1]).abs() < 0.00001);
        assert!((q1[2] - q2[2]).abs() < 0.00001);
        assert!((q1[3] - q2[3]).abs() < 0.00001);
    }

    #[test]
    fn one_parent_one_child() {
        let mut world = hecs::World::new();

        // add the parent.
        let parent_entity =
            world.spawn((Transform::new(Vec3::zero(), Quat::identity(), Vec3::one()),));

        let child_entity = world.spawn((
            Transform::default(),
            LocalTransform::new(glam::vec3(1.0, 0.0, 0.0), Quat::identity(), Vec3::one()),
            HasParent {
                entity: parent_entity,
            },
        ));

        world.insert_one(
            parent_entity,
            HasChildren {
                children: vec![child_entity],
            },
        );

        println!("Initialize");
        update_transforms(&mut world);

        // Now check the child global transform is correct.
        {
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(1.0, 0.0, 0.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }

        // Now, move the parent a bit.
        {
            let mut parent_transform = world.get_mut::<Transform>(parent_entity).unwrap();
            assert_eq!(parent_transform.dirty, false); // should have been set to false after the update_transforms function.
            parent_transform.translation = Vec3::one();
            parent_transform.dirty = true;
        }

        update_transforms(&mut world);
        // Now check the child global transform is correct.
        {
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(2.0, 1.0, 1.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }

        // A bit of rotation in the mix.
        {
            let mut parent_transform = world.get_mut::<Transform>(parent_entity).unwrap();
            assert_eq!(parent_transform.dirty, false); // should have been set to false after the update_transforms function.
            parent_transform.translation = Vec3::zero();
            parent_transform.rotation = Quat::from_rotation_y(std::f32::consts::PI);
            parent_transform.dirty = true;
        }
        update_transforms(&mut world);
        {
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(-1.0, 0.0, 0.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(
                global_transform.rotation,
                Quat::from_rotation_y(std::f32::consts::PI),
            );
        }

        // Update the child local.
        {
            let mut local_transform = world.get_mut::<LocalTransform>(child_entity).unwrap();
            assert_eq!(local_transform.dirty, false); // should have been set to false after the update_transforms function.
            local_transform.rotation = Quat::from_rotation_y(std::f32::consts::PI);
            local_transform.dirty = true;
        }

        update_transforms(&mut world);
        {
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(-1.0, 0.0, 0.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }
    }

    #[test]
    fn one_parent_one_child_with_one_child() {
        let mut world = hecs::World::new();

        // add the parent.
        let parent_entity =
            world.spawn((Transform::new(Vec3::zero(), Quat::identity(), Vec3::one()),));

        let child_entity = world.spawn((
            Transform::default(),
            LocalTransform::new(glam::vec3(1.0, 0.0, 0.0), Quat::identity(), Vec3::one()),
            HasParent {
                entity: parent_entity,
            },
        ));

        let grand_child_entity = world.spawn((
            Transform::default(),
            LocalTransform::new(glam::vec3(1.0, 0.0, 0.0), Quat::identity(), Vec3::one()),
            HasParent {
                entity: child_entity,
            },
        ));

        world.insert_one(
            parent_entity,
            HasChildren {
                children: vec![child_entity],
            },
        );

        world.insert_one(
            child_entity,
            HasChildren {
                children: vec![grand_child_entity],
            },
        );

        update_transforms(&mut world);
        // Now check the child global transform is correct.
        {
            println!("Check child global transform");

            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(1.0, 0.0, 0.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
            println!("Check grandchild global transform");

            let global_transform = world.get::<Transform>(grand_child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(2.0, 0.0, 0.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }

        println!("Move parent's global");
        // Now, move the parent a bit.
        {
            let mut parent_transform = world.get_mut::<Transform>(parent_entity).unwrap();
            assert_eq!(parent_transform.dirty, false); // should have been set to false after the update_transforms function.
            parent_transform.translation = Vec3::one();
            parent_transform.dirty = true;
        }
        update_transforms(&mut world);

        // Now check the child global transform is correct.
        {
            println!("Check child global transform");
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(2.0, 1.0, 1.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
            println!("Check grandchild global transform");
            let global_transform = world.get::<Transform>(grand_child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(3.0, 1.0, 1.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }

        println!("Move child's local");

        // Now, move the child's local transform a bit.
        {
            let mut local_transform = world.get_mut::<LocalTransform>(child_entity).unwrap();
            assert_eq!(local_transform.dirty, false); // should have been set to false after the update_transforms function.
            local_transform.translation = Vec3::one();
            local_transform.dirty = true;
        }
        update_transforms(&mut world);
        {
            let global_transform = world.get::<Transform>(child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(2.0, 2.0, 2.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());

            let global_transform = world.get::<Transform>(grand_child_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(3.0, 2.0, 2.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());

            let global_transform = world.get::<Transform>(parent_entity).unwrap();
            assert_vec_eq(global_transform.translation, Vec3::new(1.0, 1.0, 1.0));
            assert_vec_eq(global_transform.scale, Vec3::one());
            assert_quat_eq(global_transform.rotation, Quat::identity());
        }
    }
}
