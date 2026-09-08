use std::{collections::HashMap, sync::RwLock};

use super::Filesystem;
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

#[cfg(test)]
mod tests;

/// A filesystem implementation that contains no data and always returns no files.
pub struct NullFS;

impl Filesystem for NullFS {
    fn read(&self, _path: &str, _absolute_path: bool) -> Option<ArcSlice<[u8]>> {
        None
    }

    fn absolute_paths(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Given a nif file, finds the base path that contains it
/// So for path: "mods/example/meshes/tree.nif"
/// it would return: "mods/example"
pub fn get_nif_base_dir(path: &str) -> &str {
    if let Some(idx) = path.to_lowercase().find("meshes") {
        let end = if idx > 0 && (path.as_bytes()[idx - 1] == b'/' || path.as_bytes()[idx - 1] == b'\\') {
            idx - 1
        } else {
            idx
        };
        &path[..end]
    } else {
        path
    }
}

/// A filesystem implementation backed by an in-memory hash map.
/// This is used by the archive filesystem to store extracted files in memory.
#[derive(Default)]
pub struct HashmapFS {
    /// The base path for the filesystem, used as a prefix for all file lookups.
    pub base: RwLock<String>,
    pub inner: HashMap<String, ArcSlice<[u8]>>,
}

impl HashmapFS {
    pub fn new(base: String, inner: HashMap<String, ArcSlice<[u8]>>) -> Self {
        Self {
            base: RwLock::new(base),
            inner,
        }
    }

    pub fn new_from_vec(base: String, inner: HashMap<String, Vec<u8>>) -> Self {
        let inner = inner
            .into_iter()
            .map(|(k, v)| (k, ArcSlice::from(v.as_slice())))
            .collect();
        Self {
            base: RwLock::new(base),
            inner,
        }
    }
}

impl Filesystem for HashmapFS {
    fn has_base(&self) -> bool {
        true
    }

    fn set_base(&self, base: String) {
        *self.base.write().unwrap() = base;
    }

    fn read(&self, path: &str, absolute_path: bool) -> Option<ArcSlice<[u8]>> {
        let base_is_empty = self.base.read().unwrap().is_empty();

        if absolute_path || base_is_empty {
            if !absolute_path {
                bevy::log::info!("HashmapFS: Base is empty, reading absolute file: {path}");
            } else {
                bevy::log::info!("HashmapFS: Reading absolute file: {path}");
            }
            self.inner.get(path).cloned()
        } else {
            let base = self.base.read().unwrap().clone();
            let full_path = format!("{base}/{path}");
            bevy::log::info!("HashmapFS: Reading relative file with base: {full_path}");
            self.inner.get(&full_path).cloned()
        }
    }

    fn absolute_paths(&self) -> Vec<String> {
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

    pub fn clear_filesystems(&self) {
        self.inner.write().unwrap().clear();
    }
}

impl Filesystem for CombinedFS {
    fn read(&self, path: &str, absolute_path: bool) -> Option<ArcSlice<[u8]>> {
        for fs in self.inner.read().unwrap().iter() {
            if let Some(data) = fs.read(path, absolute_path) {
                return Some(data);
            }
        }
        None
    }

    fn absolute_paths(&self) -> Vec<String> {
        let mut all_paths = HashSet::new();
        for fs in self.inner.read().unwrap().iter() {
            all_paths.extend(fs.absolute_paths());
        }
        all_paths.into_iter().collect()
    }
}
