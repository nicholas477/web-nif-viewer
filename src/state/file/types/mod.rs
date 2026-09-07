use std::{collections::HashMap, sync::RwLock};

use super::{Filesystem};
use arc_slice::ArcSlice;
use bevy::platform::collections::HashSet;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;


/// A filesystem implementation that contains no data and always returns no files.
pub struct NullFS;

impl Filesystem for NullFS {
    fn read(&self, _path: &str) -> Option<ArcSlice<[u8]>> {
        None
    }

    fn paths(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A filesystem implementation backed by an in-memory hash map.
/// This is used by the archive filesystem to store extracted files in memory.
#[derive(Default)]
pub struct HashmapFS {
    inner: HashMap<String, ArcSlice<[u8]>>
}

impl HashmapFS {
    pub fn new(inner: HashMap<String, ArcSlice<[u8]>>) -> Self {
        Self { inner }
    }

    pub fn new_from_vec(inner: HashMap<String, Vec<u8>>) -> Self {
        let inner = inner
            .into_iter()
            .map(|(k, v)| (k, ArcSlice::from(v.as_slice())))
            .collect();
        Self { inner }
    }
}

impl Filesystem for HashmapFS {
    fn read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        self.inner.get(path).cloned()
    }

    fn paths(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }
}

/// A filesystem implementation that combines multiple filesystems into one, checking each in order for requested files.
#[derive(Default)]
pub struct CombinedFS {
    /// Inner vector storing the filesystems to combine
    inner: RwLock<Vec<Box<dyn Filesystem + Send + Sync>>>,
}

impl CombinedFS {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }

    pub fn add_filesystem(&self, fs: Box<dyn Filesystem + Send + Sync>) {
        self.inner.write().unwrap().push(fs);
    }
}

impl Filesystem for CombinedFS {
    fn read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        for fs in self.inner.read().unwrap().iter() {
            if let Some(data) = fs.read(path) {
                return Some(data);
            }
        }
        None
    }

    fn paths(&self) -> Vec<String> {
        let mut all_paths = HashSet::new();
        for fs in self.inner.read().unwrap().iter() {
            all_paths.extend(fs.paths());
        }
        all_paths.into_iter().collect()
    }
}