use std::{fs, path::Path};

use bevy::prelude::*;
use directories::ProjectDirs;
use rfd::FileDialog;

/// Opens the native ZIP picker and loads the selected archive into the viewer.
pub fn open_archive_picker(state: &mut crate::UIState) {
    let Some(path) = FileDialog::new()
        .add_filter("ZIP archive", &["zip"])
        .pick_file()
    else {
        return;
    };

    load_archive(state, &path);
}

fn recent_files_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "nicholas477", "nif-viewer")
        .map(|proj_dirs| proj_dirs.data_local_dir().join("recent_files.toml"))
        .or_else(|| {
            bevy::log::warn!("Could not determine project directories for recent files.");
            None
        })
}

/// Loads a local archive path selected from a recent-files entry.
pub fn start_archive_load(
    state: &mut crate::UIState,
    archive_path: String,
    pending_file: Option<String>,
) {
    state.archive.pending_file = pending_file;
    load_archive(state, Path::new(&archive_path));
}

/// Records a recent archive/file pair in the local configuration.
pub fn record_recent_file(archive_path: &str, file_name: &str) {
    let mut recent_files = recent_files();
    recent_files.files.retain(|recent| recent.zip_url != archive_path);
    recent_files.files.insert(
        0,
        crate::RecentFile {
            zip_url: archive_path.to_string(),
            file_name: file_name.to_string(),
        },
    );
    recent_files.files.truncate(super::MAX_RECENT_FILES);

    if let Some(recent_files_path) = recent_files_path()
        && let Ok(contents) = toml::to_string(&recent_files).inspect_err(|e| {
            bevy::log::warn!("Failed to serialize recent files to TOML: {e}");
        })
    {
        let _ = fs::create_dir_all(recent_files_path.parent().unwrap());
        let _ = fs::write(&recent_files_path, contents);
    }
}

/// Returns a list of recent files from the recent files toml configuration.
/// Win: C:\Users\Alice\AppData\Roaming\nicholas477\nif-viewer\data
pub fn recent_files() -> crate::RecentFiles {
    if let Some(recent_files_path) = recent_files_path()
        && recent_files_path.exists()
        && let Ok(contents) = fs::read_to_string(&recent_files_path)
        && let Ok(recent_files) =
            toml::from_str::<crate::RecentFiles>(&contents).inspect_err(|e| {
                bevy::log::warn!("Failed to deserialize recent files to TOML: {e}");
            })
    {
        return recent_files;
    }

    crate::RecentFiles { files: Vec::new() }
}

fn load_archive(state: &mut crate::UIState, path: &Path) {
    let archive_path = path.display().to_string();
    state.archive.zip_url_input = archive_path.clone();
    state.archive.selected_file = None;
    *state.archive.file_system.write().unwrap() = Default::default();

    let mut status = state.archive.archive_load_status.write().unwrap();
    status.phase = Some("Opening archive...".to_string());
    status.error = None;
    drop(status);

    match fs::read(path)
        .map_err(crate::file::FileError::IoError)
        .and_then(|bytes| crate::file::unzip(bytes, &state.archive.archive_load_status))
    {
        Ok(files) => {
            *state.archive.file_system.write().unwrap() = files;
            state.archive.archive_load_status.write().unwrap().phase = None;
        }
        Err(error) => {
            let mut status = state.archive.archive_load_status.write().unwrap();
            status.phase = None;
            status.error = Some(format!("Could not open {archive_path}: {error}"));
        }
    }
}
