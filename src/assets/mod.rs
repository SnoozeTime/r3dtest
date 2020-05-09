//! Aaaah Asset management. :)
//! For now, let's load everything in the main thread ahead of time (Loading screen).
use crate::assets::material::{AsyncMaterialLoader, Material};
use crate::assets::mesh::SyncMeshLoader;
use crate::assets::LoadingStatus::Loaded;
use crate::render::mesh::mesh::Mesh;
use crate::resources::Resources;
use luminance::context::GraphicsContext;
use luminance::state::GraphicsState;
use luminance_glfw::GlfwSurface;
use std::cell::RefCell;
use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;

pub mod material;
pub mod mesh;
pub mod prefab;

pub fn create_asset_managers(surface: &mut GlfwSurface, resources: &mut Resources) {
    let mut mesh_manager: AssetManager<Mesh> =
        AssetManager::from_loader(Box::new(SyncMeshLoader::new(surface)));
    let mut material_manager: AssetManager<Material> =
        AssetManager::from_loader(Box::new(AsyncMaterialLoader::new()));
    material_manager.load("default_material");
    material_manager.load("material_Floor");
    mesh_manager.load("_simple_sphere_Sphere");
    mesh_manager.load("material_Cube");

    resources.insert(mesh_manager);
    resources.insert(material_manager);
}
pub struct AbstractGraphicContext(Rc<RefCell<GraphicsState>>);

impl AbstractGraphicContext {
    /// Should be called once per thread. Otherwise, just build it from the existing graphic context.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(GraphicsState::new().unwrap())))
    }

    pub fn from_glfw(surface: &GlfwSurface) -> Self {
        let gfx_state = unsafe { surface.state().clone() };
        Self(gfx_state)
    }
}

unsafe impl GraphicsContext for AbstractGraphicContext {
    fn state(&self) -> &Rc<RefCell<GraphicsState>> {
        &self.0
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Handle(pub String);

#[derive(Debug, Error)]
pub enum AssetError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    DeserError(#[from] ron::de::Error),

    #[error(transparent)]
    ImageError(#[from] image::ImageError),

    #[error(transparent)]
    BincodeError(#[from] bincode::Error),
}

pub struct Asset<T> {
    asset: Arc<Mutex<LoadingStatus<T, AssetError>>>,
}

impl<T> Clone for Asset<T> {
    fn clone(&self) -> Self {
        Asset {
            asset: Arc::clone(&self.asset),
        }
    }
}

impl<T> Default for Asset<T> {
    fn default() -> Self {
        Asset::new()
    }
}

impl<T> From<AssetError> for Asset<T> {
    fn from(e: AssetError) -> Self {
        Self {
            asset: Arc::new(Mutex::new(LoadingStatus::Error(e))),
        }
    }
}

impl<T> Asset<T> {
    pub fn new() -> Self {
        Self {
            asset: Arc::new(Mutex::new(LoadingStatus::Loading)),
        }
    }

    pub fn from_asset(asset: T) -> Self {
        Self {
            asset: Arc::new(Mutex::new(LoadingStatus::Ready(asset))),
        }
    }

    pub fn set_ready(&mut self, v: T) {
        *self.asset.lock().unwrap() = LoadingStatus::Loaded(v);
    }

    pub fn set_loaded(&mut self, v: T) {
        *self.asset.lock().unwrap() = LoadingStatus::Loaded(v);
    }

    pub fn set_error(&mut self, e: AssetError) {
        *self.asset.lock().unwrap() = LoadingStatus::Error(e);
    }

    /// Returns true if the asset has finished loading.
    pub fn is_loaded(&self) -> bool {
        let asset = &*self.asset.lock().unwrap();
        if let LoadingStatus::Ready(_) = asset {
            true
        } else {
            false
        }
    }

    /// Returns true if the asset has failed loading.
    pub fn is_error(&self) -> bool {
        let asset = &*self.asset.lock().unwrap();
        if let LoadingStatus::Error(_) = asset {
            true
        } else {
            false
        }
    }

    /// Execute a function only if the asset is loaded.
    pub fn execute<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let asset = &*self.asset.lock().unwrap();
        if let LoadingStatus::Ready(ref inner) = asset {
            f(inner);
        }
    }

    /// Execute a function only if the asset is loaded.
    pub fn execute_mut<F>(&self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        let asset = &mut *self.asset.lock().unwrap();
        if let LoadingStatus::Ready(ref mut inner) = asset {
            f(inner);
        }
    }
}
impl<T: Clone> Asset<T> {
    /// Some assets should not be modified so it's better to get a copy of them
    /// (Dialog for example)
    pub fn clone_inner(&self) -> Option<T> {
        let asset = &*self.asset.lock().unwrap();
        if let LoadingStatus::Loaded(ref inner) = asset {
            Some((*inner).clone())
        } else {
            None
        }
    }
}

pub enum LoadingStatus<T, E> {
    Ready(T),
    Loaded(T),
    Loading,
    Error(E),
}

impl<T: Default, E> LoadingStatus<T, E> {
    pub fn move_to_read(&mut self) {
        match self {
            LoadingStatus::Loaded(asset) => *self = LoadingStatus::Ready(std::mem::take(asset)),
            _ => (),
        }
    }
}

pub struct AssetManager<T: Default> {
    // might want to use a LRU instead...
    store: HashMap<Handle, Asset<T>>,
    loader: Box<dyn Loader<T>>,
}

impl<T: Default> AssetManager<T> {
    pub fn from_loader(loader: Box<dyn Loader<T>>) -> Self {
        Self {
            store: HashMap::new(),
            loader,
        }
    }

    pub fn load(&mut self, asset_name: &str) -> Handle {
        let handle = Handle(asset_name.to_owned());
        if self.store.contains_key(&handle) {
            return handle;
        }
        let asset = self.loader.load(asset_name);
        self.store.insert(handle.clone(), asset);
        handle
    }

    pub fn upload_all(&mut self, ctx: &mut GlfwSurface) {
        // once every now and then, check the resources ready to be uploaded by the current thread.
        for asset in self.store.values() {
            let mut asset = &mut *asset.asset.lock().unwrap();
            if let LoadingStatus::Loaded(ref mut t) = asset {
                // UPLOAD
                self.loader.upload_to_gpu(ctx, t);
            }
            asset.move_to_read();
        }
    }

    pub fn get(&self, handle: &Handle) -> Option<&Asset<T>> {
        self.store.get(handle)
    }

    pub fn get_mut(&mut self, handle: &Handle) -> Option<&mut Asset<T>> {
        self.store.get_mut(handle)
    }

    pub fn is_loaded(&self, handle: &Handle) -> bool {
        self.store
            .get(handle)
            .map(|asset| asset.is_loaded())
            .unwrap_or(false)
    }

    pub fn is_error(&self, handle: &Handle) -> bool {
        self.store
            .get(handle)
            .map(|asset| asset.is_error())
            .unwrap_or(false)
    }

    /// Return the assets that are currently managed
    pub fn keys(&self) -> Keys<Handle, Asset<T>> {
        self.store.keys()
    }
}

pub trait Loader<T> {
    /// Get an asset from an handle
    fn load(&mut self, asset_name: &str) -> Asset<T>;

    fn upload_to_gpu(&self, ctx: &mut GlfwSurface, inner: &mut T) {}
}
