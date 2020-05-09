//! Prefabs are entities (or list of entities) that can be imported from a file and instantiated.
use crate::ecs::serialization::SerializedEntity;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct Prefab {
    entities: SerializedEntity,
}

pub struct PrefabLoader {
    base_path: PathBuf,
}
