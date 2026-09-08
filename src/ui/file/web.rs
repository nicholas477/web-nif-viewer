use std::sync::{Arc, RwLock};

use crate::state::query;
use bevy::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;

/// Initializes archive and selected-file state from the URL or default mesh.
pub fn initialize_default_mesh(
    mut state: ResMut<crate::UIState>,
    fsstate: ResMut<crate::state::FSState>,
) {
    let query_state =
        crate::state::query::query_state().unwrap_or_else(|| crate::state::query::QueryState {
            path: super::DEFAULT_MESH.0.to_string(),
            selected_file: super::DEFAULT_MESH.1.to_string(),
            source: crate::RecentFileSource::Url,
            view_state: state.view.clone(),
        });

    state.archive.zip_url_input = query_state.path.clone();
    state.archive.recent_source = query_state.source;
    state.view = query_state.view_state.clone();
    start_archive_load(
        state.into_inner(),
        fsstate.into_inner(),
        query_state.path,
        Some(query_state.selected_file),
        query_state.source,
    );
}

/// Opens the browser archive URL dialog.
pub fn open_url_dialog(state: &mut crate::UIState, _fsstate: &mut crate::state::FSState) {
    state.archive.show_zip_popup = true;
}

/// Draws the browser archive URL dialog and starts loading the submitted URL.
pub fn draw_url_dialog(
    ctx: &bevy_egui::egui::Context,
    state: &mut crate::UIState,
    fsstate: &mut crate::state::FSState,
) {
    if !state.archive.show_zip_popup {
        return;
    }

    let mut open = state.archive.show_zip_popup;
    let mut submitted_url = None;
    let mut cancelled = false;
    bevy_egui::egui::Window::new("Load URL")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(480.0)
        .show(ctx, |ui| {
            ui.text_edit_singleline(&mut state.archive.zip_url_input);
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    submitted_url = Some(state.archive.zip_url_input.trim().to_string());
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
        });

    if let Some(url) = submitted_url.filter(|url| !url.is_empty()) {
        open = false;
        start_archive_load(
            state,
            fsstate,
            url,
            None,
            crate::RecentFileSource::Url,
        );
    }
    if cancelled {
        open = false;
    }
    state.archive.show_zip_popup = open;
}

/// Opens the browser's ZIP file picker and starts the upload after selection.
pub fn open_upload_picker(state: &mut crate::UIState) {
    let status = state.archive.upload_status.clone();
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(element) = document.create_element("input") else {
        return;
    };
    let Ok(input) = element.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
    };
    input.set_type("file");
    input.set_accept(".zip,application/zip");

    let on_change = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(input) = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };

        let status = status.clone();
        {
            let mut status = status.write().unwrap();
            status.phase = Some("Preparing upload...".to_string());
            status.error = None;
            status.success = None;
        }
        spawn_local(async move {
            match crate::state::file::upload_file(file, &status).await {
                Ok(download_url) => {
                    let mut status = status.write().unwrap();
                    status.success = Some("Upload complete. Opening archive...".to_string());
                    status.download_url = Some(download_url);
                }
                Err(error) => {
                    let mut status = status.write().unwrap();
                    status.phase = None;
                    status.error = Some(error.to_string());
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input.click();
}

/// Clears the current archive and asynchronously begins loading a new one.
pub fn start_archive_load(
    state: &mut crate::UIState,
    fsstate: &mut crate::state::FSState,
    zip_url: String,
    pending_file: Option<String>,
    source: crate::RecentFileSource,
) {
    if source == crate::RecentFileSource::Disk {
        let file_system = fsstate.file_system.clone();
        let pending_picker_file = state.archive.pending_picker_file.clone();
        let pending_picker_source = state.archive.pending_picker_source.clone();
        let pending_picker_recent_source = state.archive.pending_picker_recent_source.clone();
        let load_status = state.archive.archive_load_status.clone();
        spawn_local(async move {
            if let Some(file) = crate::ui::file::open_recent_file(&zip_url).await {
                crate::ui::top_panel::load_file(
                    file,
                    file_system,
                    pending_picker_file,
                    pending_picker_source,
                    pending_picker_recent_source,
                    load_status,
                )
                .await;
            }
        });
        return;
    }
    let pending_file = pending_file.map(|file_name| crate::state::file::normalize_path(&file_name));
    crate::state::query::update_query(&query::QueryState {
        path: zip_url.clone(),
        selected_file: pending_file
            .as_deref()
            .map(|s| s.into())
            .unwrap_or_default(),
        source,
        view_state: state.view.clone(),
    });
    state.archive.zip_url_input = zip_url.clone();
    state.archive.recent_source = crate::RecentFileSource::Url;
    state.archive.selected_file = None;
    state.archive.pending_file = pending_file;
    fsstate.file_system.write().unwrap().take();
    {
        let mut status = state.archive.archive_load_status.write().unwrap();
        status.phase = Some("Preparing download...".to_string());
        status.error = None;
    }
    fetch_archive(
        zip_url,
        fsstate.file_system.clone(),
        state.archive.archive_load_status.clone(),
    );
}

/// Fetches an archive asynchronously and publishes its files or error status.
fn fetch_archive(
    zip_url: String,
    file_system: Arc<RwLock<Option<Box<dyn crate::state::file::Filesystem>>>>,
    load_status: Arc<RwLock<crate::ArchiveLoadStatus>>,
) {
    spawn_local(async move {
        match crate::state::file::fetch_and_unzip(&zip_url, &load_status).await {
            Ok(files) => {
                bevy::log::info!("Zip fetched and parsed successfully.");
                *file_system.write().unwrap() = Some(Box::new(
                    crate::state::file::HashmapFS::new_from_vec("".into(), files),
                ));
            }
            Err(error) => {
                let message = error.to_string();
                bevy::log::error!("Error unzipping asset: {message}");
                let mut status = load_status.write().unwrap();
                status.phase = None;
                status.error = Some(message);
            }
        }
    });
}
