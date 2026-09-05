use std::sync::{Arc, RwLock};

use crate::state::query;
use bevy::prelude::*;

use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;

const RECENT_FILES_COOKIE: &str = "esp_viewer_recent_files";

/// Initializes archive and selected-file state from the URL or default mesh.
pub fn initialize_from_url(mut state: ResMut<crate::UIState>) {
    let query_state =
        crate::state::query::query_state().unwrap_or_else(|| crate::state::query::QueryState {
            zip_url: super::DEFAULT_MESH.0.to_string(),
            selected_file: super::DEFAULT_MESH.1.to_string(),
            view_state: state.view.clone(),
        });

    state.archive.zip_url_input = query_state.zip_url.clone();
    state.archive.pending_file = Some(query_state.selected_file);
    fetch_archive(
        query_state.zip_url,
        state.archive.file_system.clone(),
        state.archive.archive_load_status.clone(),
    );
}

/// Opens the browser archive URL dialog.
pub fn open_archive_picker(state: &mut crate::UIState) {
    state.archive.show_zip_popup = true;
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
            match crate::file::upload_file(file, &status).await {
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
    zip_url: String,
    pending_file: Option<String>,
) {
    crate::state::query::update_query(&query::QueryState {
        zip_url: zip_url.clone(),
        selected_file: pending_file
            .as_deref()
            .map(|s| s.into())
            .unwrap_or_default(),
        view_state: state.view.clone(),
    });
    state.archive.zip_url_input = zip_url.clone();
    state.archive.selected_file = None;
    state.archive.pending_file = pending_file;
    *state.archive.file_system.write().unwrap() = Default::default();
    {
        let mut status = state.archive.archive_load_status.write().unwrap();
        status.phase = Some("Preparing download...".to_string());
        status.error = None;
    }
    fetch_archive(
        zip_url,
        state.archive.file_system.clone(),
        state.archive.archive_load_status.clone(),
    );
}

/// Fetches an archive asynchronously and publishes its files or error status.
fn fetch_archive(
    zip_url: String,
    file_system: crate::file::FS,
    load_status: Arc<RwLock<crate::ArchiveLoadStatus>>,
) {
    spawn_local(async move {
        match crate::file::fetch_and_unzip(&zip_url, &load_status).await {
            Ok(files) => {
                bevy::log::info!("Zip fetched and parsed successfully.");
                *file_system.write().unwrap() = files;
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

/// Reads persisted recent archive/file pairs from the browser cookie.
pub fn recent_files() -> crate::RecentFiles {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(cookies) = document.cookie() else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Some(encoded_value) = cookies
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{RECENT_FILES_COOKIE}=")))
    else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(value) = js_sys::decode_uri_component(encoded_value) else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Some(value) = value.as_string() else {
        return crate::RecentFiles { files: Vec::new() };
    };

    crate::RecentFiles {
        files: value
            .lines()
            .filter_map(|entry| {
                let (zip_url, file_name) = entry.split_once('\t')?;
                (!zip_url.is_empty() && !file_name.is_empty()).then(|| crate::RecentFile {
                    zip_url: zip_url.to_string(),
                    file_name: file_name.to_string(),
                })
            })
            .collect(),
    }
}

/// Stores a successful archive/file selection at the front of the recent-files cookie.
pub fn record_recent_file(zip_url: &str, file_name: &str) {
    if zip_url.is_empty() {
        return;
    }
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };

    let mut files = recent_files();
    files.files.retain(|recent| recent.zip_url != zip_url);
    files.files.insert(
        0,
        crate::RecentFile {
            zip_url: zip_url.to_string(),
            file_name: file_name.to_string(),
        },
    );
    files.files.truncate(super::MAX_RECENT_FILES);

    let value = files
        .files
        .iter()
        .map(|recent| format!("{}\t{}", recent.zip_url, recent.file_name))
        .collect::<Vec<_>>()
        .join("\n");
    let encoded_value = js_sys::encode_uri_component(&value);
    let cookie =
        format!("{RECENT_FILES_COOKIE}={encoded_value}; Max-Age=31536000; Path=/; SameSite=Lax");
    if let Err(error) = document.set_cookie(&cookie) {
        bevy::log::warn!("Could not save recent files cookie: {:?}", error);
    }
}
