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

use std::{
    collections::HashMap,
    fmt,
    io::{self, Cursor, Read},
    sync::{Arc, RwLock},
};
use zip::ZipArchive;

pub type FS = Arc<RwLock<HashMap<String, Vec<u8>>>>;
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

/// Finds a NIF asset reference relative to its source file, searching each ancestor directory.
pub fn find_file(
    file_system: &FS,
    source_path: &str,
    requested_path: &str,
) -> Option<Vec<u8>> {
    let requested_path = requested_path.replace('/', "\\");
    let file_system = file_system.read().ok()?;

    for directory in ancestor_directories(source_path) {
        let candidate = if directory.is_empty() {
            normalize_path(&requested_path)
        } else {
            normalize_path(&format!("{directory}\\{requested_path}"))
        };
        if let Some(bytes) = file_system.get(&candidate) {
            return Some(bytes.clone());
        }
    }

    None
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

/// Returns the source file's directory followed by every parent through the archive root.
fn ancestor_directories(source_path: &str) -> impl Iterator<Item = String> {
    let mut directories = Vec::new();
    let mut directory = normalize_path(source_path)
        .rsplit_once('\\')
        .map(|(directory, _)| directory.to_string());

    loop {
        match directory.take() {
            Some(current) => {
                directory = current
                    .rsplit_once('\\')
                    .map(|(parent, _)| parent.to_string());
                directories.push(current);
            }
            None => {
                directories.push(String::new());
                break;
            }
        }
    }

    directories.into_iter()
}
