use crate::nif;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use egui::{LayerId, Ui, UiBuilder};
use tes3::nif::{
    NiStream, NiTexturingProperty, NiTriShape, NiTriShapeData, TextureMap, TextureSource,
};

use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

const ZIP_QUERY_PARAMETER: &str = "zip";
const FILE_QUERY_PARAMETER: &str = "file";

pub fn initialize_from_url(mut state: ResMut<crate::MenuState>) {
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
    mut camera3d: Single<(&mut Camera3d, &Projection, &mut PanOrbitCamera), Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loaded_meshes: Query<Entity, With<nif::LoadedNifMesh>>,
    mut state: ResMut<crate::MenuState>,
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

    if let Some(pending_file) = state.pending_file.clone()
        && file_names
            .iter()
            .any(|file_name| file_name == &pending_file)
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
        }

        let (_, projection, mut pan_orbit) = camera3d.into_inner();

        nif::center_camera_on_mesh(
            &meshes,
            projection,
            window,
            &mut pan_orbit,
        );
    }

    let left_panel = egui::Panel::left("left_panel")
        .default_size(400.0)
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            draw_file_selector(ui, &file_names, &mut state.selected_file)
        });
    let mut left = left_panel.response.rect.width(); // height is ignored, as the panel has a height of 100% of the screen

    if let Some(file_name) = left_panel.inner {
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
        }
    }

    let mut top = egui::Panel::top("top_panel")
        .resizable(false)
        .show(&mut viewport_ui, |ui| {
            if ui.button("Open File").clicked() {
                state.show_zip_popup = true;
            }
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen

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

fn draw_load_status(ctx: &egui::Context, state: &crate::MenuState) {
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

fn draw_error_popup(ctx: &egui::Context, state: &mut crate::MenuState) {
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

fn draw_zip_popup(ctx: &egui::Context, state: &mut crate::MenuState) {
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
                    update_query(&url_to_load, None);
                    state.selected_file = None;
                    state.pending_file = None;
                    {
                        let mut status = state.archive_load_status.write().unwrap();
                        status.phase = Some("Preparing download...".to_string());
                        status.error = None;
                    }

                    let state_fs = state.file_system.clone(); // Clone the Arc<RwLock<...>> to move into the async block
                    let load_status = state.archive_load_status.clone();

                    //let state = state.clone(); // Clone the state to move into the async block
                    // Hand off the execution to the browser's async event loop
                    spawn_local(async move {
                        // Call your previously implemented zip loading code here
                        match crate::file::fetch_and_unzip(&url_to_load, &load_status).await {
                            Ok(fs) => {
                                bevy::log::info!("Zip fetched and parsed successfully.");
                                *state_fs.write().unwrap() = fs;
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

                    state.show_zip_popup = false;
                }

                if ui.button("Cancel").clicked() {
                    state.show_zip_popup = false;
                }
            });
        });
}
