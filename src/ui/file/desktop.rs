use std::path::Path;

use bevy::prelude::*;
use rfd::FileDialog;

pub fn initialize_default_mesh(mut state: ResMut<crate::UIState>, fsstate: ResMut<crate::state::FSState>) {
    state.archive.zip_url_input = super::DEFAULT_MESH.0.to_string();
    state.archive.pending_file = Some(super::DEFAULT_MESH.1.to_string());

    start_archive_load(
        state.into_inner(),
        fsstate.into_inner(),
        super::DEFAULT_MESH.0.to_string(),
        Some(super::DEFAULT_MESH.1.to_string()),
    );
}

/// Opens the native ZIP picker and loads the selected archive into the viewer.
pub fn open_archive_picker(state: &mut crate::UIState, fsstate: &mut crate::state::FSState) {
    let Some(path) = FileDialog::new()
        .add_filter("ZIP archive", &["zip"])
        .pick_file()
    else {
        return;
    };

    load_archive(state, fsstate, &path);
}

/// Loads a local archive path selected from a recent-files entry.
pub fn start_archive_load(
    state: &mut crate::UIState,
    fsstate: &mut crate::state::FSState,
    archive_path: String,
    pending_file: Option<String>,
) {
    state.archive.pending_file = pending_file;
    load_archive(state, fsstate, Path::new(&archive_path));
}

fn load_archive(state: &mut crate::UIState, fsstate: &mut crate::state::FSState, path: &Path) {
    let archive_path = path.display().to_string();
    state.archive.zip_url_input = archive_path.clone();
    state.archive.selected_file = None;

    let mut status = state.archive.archive_load_status.write().unwrap();
    status.phase = Some("Opening archive...".to_string());
    status.error = None;
    drop(status);

    match crate::state::file::read_archive(path, &state.archive.archive_load_status) {
        Ok(files) => {
            *fsstate.file_system.write().unwrap() = Some(Box::new(crate::state::file::HashmapFS::new_from_vec("".into(),files)));
            state.archive.archive_load_status.write().unwrap().phase = None;
        }
        Err(error) => {
            let mut status = state.archive.archive_load_status.write().unwrap();
            status.phase = None;
            status.error = Some(format!("Could not open {archive_path}: {error}"));
        }
    }
}
