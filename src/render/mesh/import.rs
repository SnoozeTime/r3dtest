//! Import a GLTF in the current scene and add the game objects.

use crate::ecs::{Name, Transform};
use crate::render::mesh::scene::Scene;
use crate::render::Render;
use crate::transform::{HasChildren, HasParent, LocalTransform};
use hecs::EntityBuilder;
use luminance_glfw::GlfwSurface;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GltfError {
    #[error(transparent)]
    ImportError(#[from] gltf::Error),

    #[error("GLTF should at least have one scene")]
    NoSceneInGltf,
}

/// Will create a new entity and import the gltf as a children. It helps in case the mesh is super complicated.
pub fn import_gltf<P: AsRef<Path>>(
    surface: &mut GlfwSurface,
    global_scene: &mut Scene,
    world: &mut hecs::World,
    path: P,
) -> Result<(), GltfError> {
    let import = gltf::import(path.as_ref())?;

    // let's assume that everything is in the first scene.
    let g_scene = import.0.scenes().next().ok_or(GltfError::NoSceneInGltf)?;

    // TODO import material and meshes asynchronously.
    let scene = Scene::from_gltf(surface, &g_scene, &import);

    // TODO find a way to merge the assets nicely. This can override existing materials...
    for (id, material) in scene.assets.materials {
        global_scene.assets.materials.insert(id, material);
    }
    for (id, mesh) in scene.assets.meshes {
        global_scene.assets.meshes.insert(id, mesh);
    }
    for (flag, shader) in scene.assets.shaders.shaders {
        global_scene.assets.shaders.shaders.insert(flag, shader);
    }

    // now that everything is imported, let's create the entities.
    // first the parent entity.
    let parent_entity = world.spawn((
        Transform::new(
            glam::Vec3::zero(),
            glam::Quat::identity(),
            glam::Vec3::one(),
        ),
        Name(path.as_ref().display().to_string()),
    ));

    let mut children = vec![];
    for node in scene.nodes {
        let mut builder = EntityBuilder::new();
        let local_transform: LocalTransform = node.transform.into();
        builder.add(local_transform);
        builder.add(Transform::default()); // local transform will adjust that.
        if let Some(mesh) = node.mesh_id {
            builder.add(Render {
                mesh,
                enabled: true,
            });
        }
        builder.add(HasParent {
            entity: parent_entity,
        });
        children.push(world.spawn(builder.build()));
    }

    world
        .insert_one(parent_entity, HasChildren { children })
        .expect("Parent entity should exist.");
    Ok(())
}
