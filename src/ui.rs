use crate::nif;
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiContexts, egui};
use bevy_panorbit_camera::PanOrbitCamera;
use egui::{LayerId, Ui, UiBuilder};

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

const ZIP_QUERY_PARAMETER: &str = "zip";
const FILE_QUERY_PARAMETER: &str = "file";
const RECENT_FILES_COOKIE: &str = "esp_viewer_recent_files";
const MAX_RECENT_FILES: usize = 10;

pub fn initialize_from_url(mut state: ResMut<crate::UIState>) {
    let Some((zip_url, selected_file)) = query_state() else {
        return;
    };

    state.zip_url_input = zip_url.clone();
    state.pending_file = selected_file;

    let file_system = state.file_system.clone();
    let load_status = state.archive_load_status.clone();
    spawn_local(async move {
        match crate::file::fetch_and_unzip(&zip_url, &load_status).await {
            Ok(files) => {
                bevy::log::info!("Zip fetched and parsed successfully.");
                file_system.write().unwrap().extend(files);
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

// This function runs every frame. Therefore, updating the viewport after drawing the gui.
// With a resource which stores the dimensions of the panels, the update of the Viewport can
// be done in another system.
pub fn ui_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    camera3d: Single<(&mut Camera3d, &Projection, &mut PanOrbitCamera), Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loaded_meshes: Query<Entity, With<nif::LoadedNifMesh>>,
    mut state: ResMut<crate::UIState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    let file_names = state
        .file_system
        .read()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    let window: &mut Window = window.into_inner().into_inner();
    let (_, projection, mut pan_orbit) = camera3d.into_inner();

    if let Some(pending_file) = state.pending_file.clone()
        && file_names
            .iter()
            .any(|file_name| file_name == &pending_file)
        && pending_file.to_lowercase().ends_with(".nif")
    {
        state.pending_file = None;
        state.selected_file = Some(pending_file.clone());

        if let Err(error) = nif::load_nif(
            &pending_file,
            &state.file_system,
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &loaded_meshes,
        ) {
            state.nif_load_error = Some(error);
        } else {
            record_recent_file(&state.zip_url_input, &pending_file);
            nif::center_camera_on_mesh(&meshes, projection, window, &mut pan_orbit);
        }
    }

    let left_panel = egui::Panel::left("left_panel")
        .default_size(400.0)
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            draw_file_selector(ui, &file_names, &mut state.selected_file)
        });
    let mut left = left_panel.response.rect.width();

    if let Some(file_name) = left_panel.inner
        && file_name.to_lowercase().ends_with(".nif")
    {
        update_query(&state.zip_url_input, Some(&file_name));
        if let Err(error) = nif::load_nif(
            &file_name,
            &state.file_system,
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &loaded_meshes,
        ) {
            state.nif_load_error = Some(error);
        } else {
            record_recent_file(&state.zip_url_input, &file_name);
            nif::center_camera_on_mesh(&meshes, projection, window, &mut pan_orbit);
        }
    }

    let mut top = egui::Panel::top("top_panel")
        .resizable(false)
        .show(&mut viewport_ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked() {
                    state.show_zip_popup = true;
                }
                draw_recent_menu(ui, &mut state);
            });
        })
        .response
        .rect
        .height();

    left *= window.scale_factor();
    top *= window.scale_factor();

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height()) - pos;

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });

    if state.show_zip_popup {
        draw_zip_popup(ctx, &mut state);
    }

    draw_load_status(ctx, &state);
    draw_error_popup(ctx, &mut state);

    Ok(())
}

fn draw_load_status(ctx: &egui::Context, state: &crate::UIState) {
    let status = state.archive_load_status.read().unwrap();
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

fn draw_error_popup(ctx: &egui::Context, state: &mut crate::UIState) {
    let archive_error = state.archive_load_status.read().unwrap().error.clone();
    let error = state.nif_load_error.clone().or(archive_error);
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
                state.nif_load_error = None;
                state.archive_load_status.write().unwrap().error = None;
            }
        });
}

fn draw_recent_menu(ui: &mut Ui, state: &mut crate::UIState) {
    let recent_files = recent_files();
    ui.menu_button("Recent", |ui| {
        if recent_files.is_empty() {
            ui.add_enabled(false, egui::Button::new("No recent files"));
            return;
        }

        for recent in recent_files {
            let label = format!("{}\n{}", recent.file_name, recent.zip_url);
            if ui.button(label).clicked() {
                start_archive_load(state, recent.zip_url, Some(recent.file_name));
                ui.close();
            }
        }
    });
}

fn draw_file_selector(
    ui: &mut Ui,
    file_names: &[String],
    selected_file: &mut Option<String>,
) -> Option<String> {
    ui.heading("Files");
    ui.separator();

    if file_names.is_empty() {
        ui.label("No files loaded");
        return None;
    }

    let mut sorted_file_names = file_names.to_vec();
    sorted_file_names.sort_unstable();
    let mut clicked_file = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for file_name in sorted_file_names {
            let is_selected = selected_file.as_deref() == Some(file_name.as_str());

            if ui.selectable_label(is_selected, &file_name).clicked() {
                *selected_file = Some(file_name.clone());
                clicked_file = Some(file_name);
            }
        }
    });

    clicked_file
}

fn query_state() -> Option<(String, Option<String>)> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let zip_url = params.get(ZIP_QUERY_PARAMETER)?;

    Some((zip_url, params.get(FILE_QUERY_PARAMETER)))
}

fn update_query(zip_url: &str, selected_file: Option<&str>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(current_url) = window.location().href() else {
        return;
    };
    let Ok(url) = web_sys::Url::new(&current_url) else {
        return;
    };

    let params = url.search_params();
    if zip_url.is_empty() {
        params.delete(ZIP_QUERY_PARAMETER);
    } else {
        params.set(ZIP_QUERY_PARAMETER, zip_url);
    }

    if let Some(selected_file) = selected_file {
        params.set(FILE_QUERY_PARAMETER, selected_file);
    } else {
        params.delete(FILE_QUERY_PARAMETER);
    }

    let Ok(history) = window.history() else {
        return;
    };

    if let Err(error) = history.replace_state_with_url(&JsValue::NULL, "", Some(&url.href())) {
        bevy::log::warn!("Could not update page URL: {:?}", error);
    }
}

fn start_archive_load(state: &mut crate::UIState, zip_url: String, pending_file: Option<String>) {
    update_query(&zip_url, pending_file.as_deref());
    state.zip_url_input = zip_url.clone();
    state.selected_file = None;
    state.pending_file = pending_file;
    *state.file_system.write().unwrap() = Default::default();
    {
        let mut status = state.archive_load_status.write().unwrap();
        status.phase = Some("Preparing download...".to_string());
        status.error = None;
    }

    let file_system = state.file_system.clone();
    let load_status = state.archive_load_status.clone();
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

fn recent_files() -> Vec<crate::RecentFile> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let Some(document) = window.document() else {
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

fn record_recent_file(zip_url: &str, file_name: &str) {
    if zip_url.is_empty() {
        return;
    }

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };

    let mut files = recent_files();

    // Only retain files that don't match the current zip_url, then insert the new one at the front
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

fn draw_zip_popup(ctx: &egui::Context, state: &mut crate::UIState) {
    egui::Window::new("Load Compressed Archive")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)) // Center on browser canvas
        .show(ctx, |ui| {
            ui.label("Enter the direct URL of the target .zip archive:");

            // Text input linked directly to our Bevy state resource
            ui.text_edit_singleline(&mut state.zip_url_input);

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Download & Extract").clicked() {
                    let url_to_load = state.zip_url_input.clone();
                    start_archive_load(state, url_to_load, None);

                    state.show_zip_popup = false;
                }

                if ui.button("Cancel").clicked() {
                    state.show_zip_popup = false;
                }
            });
        });
}
