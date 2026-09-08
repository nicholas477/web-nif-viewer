use arc_slice::ArcSlice;
use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::RwLock,
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions,
};

use super::Filesystem;

/// A filesystem implementation that reads files from the real filesystem and caches them in memory.
/// On wasm, it interacts with the browser's File System Access API to read files.
pub struct RealFS {
    fs_handle: FileSystemDirectoryHandle,
    resolved_paths: RwLock<HashSet<String>>,
}

impl RealFS {
    pub fn new(fs_handle: FileSystemDirectoryHandle) -> Self {
        bevy::log::info!("RealFS: registered browser resource folder");
        Self {
            fs_handle,
            resolved_paths: RwLock::new(HashSet::new()),
        }
    }

    /// Clears files resolved for the previously loaded NIF.
    pub fn clear_cache(&self) {
        self.resolved_paths.write().unwrap().clear();
    }
}

async fn file_handle_for_path(
    root_dir: FileSystemDirectoryHandle,
    path: String,
) -> Result<FileSystemFileHandle, JsValue> {
    let parts = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    
    let (directories, file_name) =
        parts.split_at(parts.len().checked_sub(1).ok_or("empty file path")?);

    let mut directory = root_dir;

    for name in directories {
        bevy::log::info!("RealFS: opening directory {name} for {path}");
        directory = JsFuture::from(
            directory
                .get_directory_handle_with_options(name, &FileSystemGetDirectoryOptions::new()),
        )
        .await?
        .dyn_into()?;
    }

    JsFuture::from(
        directory.get_file_handle_with_options(file_name[0], &FileSystemGetFileOptions::new()),
    )
    .await?
    .dyn_into()
}

impl Filesystem for RealFS {
    fn read<'a>(
        &'a self,
        path: &'a str,
        _absolute_path: bool,
    ) -> Pin<Box<dyn Future<Output = Option<ArcSlice<[u8]>>> + 'a>> {
        Box::pin(async move {
            bevy::log::info!("RealFS: reading requested resource {path}");
            let handle = match file_handle_for_path(self.fs_handle.clone(), path.to_string()).await {
                Ok(handle) => handle,
                Err(error) => {
                    bevy::log::warn!("RealFS: could not locate {path}: {error:?}");
                    return None;
                }
            };

            let file: File = match JsFuture::from(handle.get_file()).await {
                Ok(file) => match file.dyn_into() {
                    Ok(file) => file,
                    Err(error) => {
                        bevy::log::warn!("RealFS: could not cast file handle for {path}: {error:?}");
                        return None;
                    }
                },
                Err(error) => {
                    bevy::log::warn!("RealFS: could not open {path}: {error:?}");
                    return None;
                }
            };

            match JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => {
                    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                    bevy::log::info!("RealFS: read {} bytes from {path}", bytes.len());
                    self.resolved_paths.write().unwrap().insert(path.to_string());
                    Some(ArcSlice::from(bytes.as_slice()))
                }
                Err(error) => {
                    bevy::log::warn!("RealFS: could not read {path}: {error:?}");
                    None
                }
            }
        })
    }

    fn absolute_paths(&self) -> Vec<String> {
        self.resolved_paths.read().unwrap().iter().cloned().collect()
    }
}
