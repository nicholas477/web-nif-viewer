use std::sync::{Arc, RwLock};

use crate::state::query;
use bevy::prelude::*;
use bevy_egui::egui;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;

const RECENT_FILES_COOKIE: &str = "esp_viewer_recent_files";
const MAX_RECENT_FILES: usize = 10;

const DEFAULT_MESH: (&str, &str) = (
    "assets/tr_mw_flora_tree_indoril_elm.zip",
    "meshes\\tr\\f\\tr_f_indoril_elm_01.nif",
);

/// Initializes archive and selected-file state from the URL or default mesh.
pub fn initialize_from_url(mut state: ResMut<crate::UIState>) {
    let query_state =
        crate::state::query::query_state().unwrap_or_else(|| crate::state::query::QueryState {
            zip_url: DEFAULT_MESH.0.to_string(),
            selected_file: DEFAULT_MESH.1.to_string(),
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

/// Draws the current archive download/extraction progress indicator.
pub fn draw_load_status(ctx: &egui::Context, state: &crate::UIState) {
    let status = state.archive.archive_load_status.read().unwrap();
    let Some(phase) = status.phase.as_deref() else {
        return;
    };

    egui::Area::new("archive_load_status".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label(phase);
                });
            });
        });
}

/// Displays and clears the highest-priority archive, NIF, or upload error.
pub fn draw_error_popup(ctx: &egui::Context, state: &mut crate::UIState) {
    let archive_error = state
        .archive
        .archive_load_status
        .read()
        .unwrap()
        .error
        .clone();
    let upload_error = state.archive.upload_status.read().unwrap().error.clone();
    let error = state
        .archive
        .nif_load_error
        .clone()
        .or(archive_error)
        .or(upload_error);
    let Some(error) = error else {
        return;
    };

    egui::Window::new("Unable to Load File")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(error);
            ui.add_space(8.0);
            if ui.button("Close").clicked() {
                state.archive.nif_load_error = None;
                state.archive.archive_load_status.write().unwrap().error = None;
                state.archive.upload_status.write().unwrap().error = None;
            }
        });
}

/// Draws the current archive upload progress indicator.
pub fn draw_upload_status(ctx: &egui::Context, state: &crate::UIState) {
    let status = state.archive.upload_status.read().unwrap();
    let Some(phase) = status.phase.as_deref() else {
        return;
    };

    egui::Area::new("upload_status".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -56.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label(phase);
                });
            });
        });
}

/// Displays the upload completion message until the user dismisses it.
pub fn draw_upload_result_popup(ctx: &egui::Context, state: &mut crate::UIState) {
    let success = state.archive.upload_status.read().unwrap().success.clone();
    let Some(success) = success else {
        return;
    };

    egui::Window::new("Upload Complete")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(success);
            ui.add_space(8.0);
            if ui.button("Close").clicked() {
                state.archive.upload_status.write().unwrap().success = None;
            }
        });
}

/// Opens the browser's ZIP file picker and starts the upload after selection.
pub fn open_upload_picker(status: Arc<RwLock<crate::UploadStatus>>) {
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

/// Draws recent archive/file pairs and starts loading the selected entry.
pub fn draw_recent_menu(ui: &mut egui::Ui, state: &mut crate::UIState) {
    ui.menu_button("Recent", |ui| {
        let recent_files = recent_files();
        if recent_files.is_empty() {
            ui.add_enabled(false, egui::Button::new("No recent files"));
            return;
        }

        for recent in recent_files {
            let file_name = recent.file_name;
            let zip_url = recent.zip_url;
            let file_name_response =
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::Label::new(egui::RichText::new(&file_name).size(14.0))
                            .halign(egui::Align::Min)
                            .sense(egui::Sense::click()),
                    )
                });

            // TODO: Add in a copy url feature?
            let _url_button = ui.add(
                egui::Label::new(
                    egui::RichText::new(&zip_url)
                        .size(11.0)
                        .color(ui.visuals().weak_text_color()),
                )
                .halign(egui::Align::Min)
                .wrap(),
            );
            ui.add_space(4.0);

            if file_name_response.inner.clicked() {
                start_archive_load(state, zip_url, Some(file_name));
                ui.close();
            }
        }
    });
}

/// Draws the modal used to enter an archive URL.
pub fn draw_zip_popup(ctx: &egui::Context, state: &mut crate::UIState) {
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
                    start_archive_load(state, url, None);
                    state.archive.show_zip_popup = false;
                }
                if ui.button("Cancel").clicked() {
                    state.archive.show_zip_popup = false;
                }
            });
        });
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
fn recent_files() -> Vec<crate::RecentFile> {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return Vec::new();
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return Vec::new();
    };
    let Ok(cookies) = document.cookie() else {
        return Vec::new();
    };
    let Some(encoded_value) = cookies
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{RECENT_FILES_COOKIE}=")))
    else {
        return Vec::new();
    };
    let Ok(value) = js_sys::decode_uri_component(encoded_value) else {
        return Vec::new();
    };
    let Some(value) = value.as_string() else {
        return Vec::new();
    };

    value
        .lines()
        .filter_map(|entry| {
            let (zip_url, file_name) = entry.split_once('\t')?;
            (!zip_url.is_empty() && !file_name.is_empty()).then(|| crate::RecentFile {
                zip_url: zip_url.to_string(),
                file_name: file_name.to_string(),
            })
        })
        .collect()
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
    files.retain(|recent| recent.zip_url != zip_url);
    files.insert(
        0,
        crate::RecentFile {
            zip_url: zip_url.to_string(),
            file_name: file_name.to_string(),
        },
    );
    files.truncate(MAX_RECENT_FILES);

    let value = files
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
