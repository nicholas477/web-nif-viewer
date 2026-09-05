use std::{fs, path::Path};

use bevy::prelude::*;
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

/// Loads a local archive path selected from a recent-files entry.
pub fn start_archive_load(
    state: &mut crate::UIState,
    archive_path: String,
    pending_file: Option<String>,
) {
    state.archive.pending_file = pending_file;
    load_archive(state, Path::new(&archive_path));
}

/// Desktop recent-file persistence is not implemented yet.
pub fn record_recent_file(_archive_path: &str, _file_name: &str) {}

/// Returns no entries until desktop recent-file persistence is implemented.
pub fn recent_files() -> Vec<crate::RecentFile> {
    Vec::new()
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

    match fs::read(path).map_err(crate::file::FileError::IoError).and_then(|bytes| {
        crate::file::unzip(bytes, &state.archive.archive_load_status)
    }) {
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