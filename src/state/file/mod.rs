#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;

mod types;

pub use types::*;

use arc_slice::ArcSlice;
use std::{
    collections::HashMap,
    fmt,
    io::{self, Cursor, Read},
    sync::{Arc, RwLock},
};
use zip::ZipArchive;

pub trait Filesystem: Sync + Send {
    fn read(&self, path: &str, absolute_path: bool) -> Option<ArcSlice<[u8]>>;

    fn absolute_paths(&self) -> Vec<String>;

    /// If this filesystem has a base path where it contains meshes/textures folder
    fn has_base(&self) -> bool {
        false
    }

    /// Sets the base path for this filesystem, if applicable.
    fn set_base(&self, base: String) {
        // Default implementation does nothing.
    }

    fn rebase_to_file(&self, file_path: &str) {
        if self.has_base() {
            bevy::log::info!("Rebasing to file: {}", file_path);
            let base = get_nif_base_dir(file_path).to_string();
            self.set_base(base);
        }
    }
}

pub type ArchiveLoadStatus = Arc<RwLock<crate::ArchiveLoadStatus>>;

#[derive(Debug)]
pub enum FileError {
    #[cfg(target_arch = "wasm32")]
    FetchError(String),
    UnzipError(String),
    IoError(io::Error),
}

impl fmt::Display for FileError {
    /// Formats an archive or network error for display in the viewer.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_arch = "wasm32")]
            FileError::FetchError(message) => write!(f, "Fetch Error: {message}"),
            FileError::UnzipError(msg) => write!(f, "Unzip Error: {}", msg),
            FileError::IoError(err) => write!(f, "IO Error: {}", err),
        }
    }
}

impl std::error::Error for FileError {
    /// Exposes the underlying I/O error when one is available.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

/// Extracts and normalizes every file in a ZIP archive.
pub fn unzip(
    zip_bytes: Vec<u8>,
    status: &ArchiveLoadStatus,
) -> Result<HashMap<String, Vec<u8>>, FileError> {
    let mut file_system: HashMap<String, Vec<u8>> = HashMap::new();

    status.write().unwrap().phase = Some("Opening archive...".to_string());
    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| FileError::UnzipError(format!("{:?}", e)))?;

    // Iterate through each file inside the ZIP
    for i in 0..archive.len() {
        status.write().unwrap().phase =
            Some(format!("Extracting files... {}/{}", i + 1, archive.len()));
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileError::UnzipError(format!("{:?}", e)))?;
        let name = file.name().to_string();

        if file.is_file() {
            // Read the file contents into a buffer or handle it as needed
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(FileError::IoError)?;

            file_system.insert(normalize_path(&name), contents);
        }
    }

    status.write().unwrap().phase = None;
    Ok(file_system)
}

/// Converts an archive path to the viewer's canonical backslash lowercase form.
pub fn normalize_path(path: &str) -> String {
    path.replace('/', "\\")
        .split('\\')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .fold(Vec::new(), |mut segments, segment| {
            if segment == ".." {
                segments.pop();
            } else {
                segments.push(segment);
            }
            segments
        })
        .join("\\")
        .to_ascii_lowercase()
}
