//! Aaaah Asset management. :)
//! For now, let's load everything in the main thread ahead of time (Loading screen).
use luminance::context::GraphicsContext;
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;

pub mod material;
pub mod mesh;

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

#[derive(Clone)]
pub struct Asset<T> {
    asset: Arc<Mutex<LoadingStatus<T, AssetError>>>,
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
            asset: Arc::new(Mutex::new(LoadingStatus::Loaded(asset))),
        }
    }

    pub fn set(&mut self, v: T) {
        *self.asset.lock().unwrap() = LoadingStatus::Loaded(v);
    }

    pub fn set_error(&mut self, e: AssetError) {
        *self.asset.lock().unwrap() = LoadingStatus::Error(e);
    }

    /// Returns true if the asset has finished loading.
    pub fn is_loaded(&self) -> bool {
        let asset = &*self.asset.lock().unwrap();
        if let LoadingStatus::Loaded(_) = asset {
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
        if let LoadingStatus::Loaded(ref inner) = asset {
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
    Loaded(T),
    Loading,
    Error(E),
}

pub struct AssetManager<T> {
    // might want to use a LRU instead...
    store: HashMap<Handle, Asset<T>>,
    loader: Box<dyn Loader<T>>,
}

impl<T: Clone> AssetManager<T> {
    pub fn from_loader(loader: Box<dyn Loader<T>>) -> Self {
        Self {
            store: HashMap::new(),
            loader,
        }
    }

    pub fn load(&mut self, ctx: &mut GlfwSurface, asset_name: &str) -> Handle {
        let handle = Handle(asset_name.to_owned());
        if self.store.contains_key(&handle) {
            return handle;
        }
        let asset = self.loader.load(ctx, asset_name);
        self.store.insert(handle.clone(), asset);
        handle
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
}

pub trait Loader<T> {
    /// Get an asset from an handle
    fn load(&self, ctx: &mut GlfwSurface, asset_name: &str) -> Asset<T>;
}
