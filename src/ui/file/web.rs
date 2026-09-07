use std::sync::{Arc, RwLock};

use crate::{state::query, ui::UiSystemParams};
use bevy::prelude::*;
use bevy_egui::egui;
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// Initializes archive and selected-file state from the URL or default mesh.
pub fn initialize_default_mesh(
    mut state: ResMut<crate::UIState>,
    fsstate: ResMut<crate::state::FSState>,
) {
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
        fsstate.file_system.clone(),
        state.archive.archive_load_status.clone(),
    );
}

/// Opens the browser archive URL dialog.
pub fn open_archive_picker(state: &mut crate::UIState, _fsstate: &mut crate::state::FSState) {
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
    fsstate.set_filesystem(Box::new(crate::state::file::NullFS));
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
    file_system: Arc<RwLock<Box<dyn crate::state::file::Filesystem>>>,
    load_status: Arc<RwLock<crate::ArchiveLoadStatus>>,
) {
    spawn_local(async move {
        match crate::state::file::fetch_and_unzip(&zip_url, &load_status).await {
            Ok(files) => {
                bevy::log::info!("Zip fetched and parsed successfully.");
                *file_system.write().unwrap() =
                    Box::new(crate::state::file::HashmapFS::new_from_vec(files));
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

/// Draws the modal used to enter an archive URL.
pub fn draw_zip_popup(
    ctx: &egui::Context,
    state: &mut crate::UIState,
    fsstate: &mut crate::state::FSState,
) {
    egui::Window::new("Load Compressed Archive")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label("Enter the direct URL of the target .zip archive:");
            ui.text_edit_singleline(&mut state.archive.zip_url_input);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Download & Extract").clicked() {
                    let url = state.archive.zip_url_input.clone();
                    start_archive_load(state, fsstate, url, None);
                    state.archive.show_zip_popup = false;
                }
                if ui.button("Cancel").clicked() {
                    state.archive.show_zip_popup = false;
                }
            });
        });
}

fn open_nif_picker() {
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
    input.set_accept(".nif,application/nif");

    let on_change = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        // let Some(input) = event
        //     .target()
        //     .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
        // else {
        //     return;
        // };
        // let Some(file) = input.files().and_then(|files| files.get(0)) else {
        //     return;
        // };

        // let status = status.clone();
        // {
        //     let mut status = status.write().unwrap();
        //     status.phase = Some("Preparing upload...".to_string());
        //     status.error = None;
        //     status.success = None;
        // }
        // spawn_local(async move {
        //     match crate::state::file::upload_file(file, &status).await {
        //         Ok(download_url) => {
        //             let mut status = status.write().unwrap();
        //             status.success = Some("Upload complete. Opening archive...".to_string());
        //             status.download_url = Some(download_url);
        //         }
        //         Err(error) => {
        //             let mut status = status.write().unwrap();
        //             status.phase = None;
        //             status.error = Some(error.to_string());
        //         }
        //     }
        // });
    }) as Box<dyn FnMut(_)>);
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
    input.click();
}

#[wasm_bindgen]
pub async fn pick_directory() -> Result<(), JsValue> {
    // 1. Get the global window object
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No global window found"))?;

    // 2. Call show_directory_picker (Returns a JS Promise)
    // Note: This must be triggered by a direct user interaction (like a button click)
    let picker_promise = window.show_directory_picker()?;

    // 3. Await the JavaScript Promise inside Rust
    let result_js_value = JsFuture::from(picker_promise).await?;

    // 4. Cast the result into a FileSystemDirectoryHandle
    let dir_handle: web_sys::FileSystemDirectoryHandle = result_js_value.unchecked_into();

    // 5. Interact with the chosen folder
    let dir_name = dir_handle.name();
    web_sys::console::log_1(&format!("Selected Directory: {}", dir_name).into());

    Ok(())
}

pub fn draw_file_popup(ctx: &egui::Context, params: &mut UiSystemParams) {
    egui::Window::new("Load Nif")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label("Enter the direct URL of the target .zip archive:");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Nif file").clicked() {
                    open_nif_picker();
                }

                if ui.button("Base directory").clicked() {
                    //open_base_directory_picker();
                }
            });

            ui.horizontal(|ui| {
                ui.button("Open");
                if ui.button("Cancel").clicked() {
                    params.state.top_panel.show_nif_popup = false;
                }
            });
        });
}
