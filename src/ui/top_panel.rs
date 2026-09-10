use crate::ui::{UiSystemParams, file};
use bevy::prelude::*;
use bevy_egui::egui::{self, InnerResponse, Ui};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Reads a selected NIF or ZIP file and makes its contents available to the viewer.
pub(crate) async fn load_file(
    file: Box<dyn crate::ui::file::PickerFile>,
    file_system: Arc<RwLock<Option<Box<dyn crate::state::file::Filesystem>>>>,
    pending_picker_file: Arc<RwLock<Option<String>>>,
    pending_picker_source: Arc<RwLock<Option<String>>>,
    pending_picker_recent_source: Arc<RwLock<Option<crate::RecentFileSource>>>,
    load_status: crate::state::file::ArchiveLoadStatus,
) {
    let file_name = crate::state::file::normalize_path(&file.name());
    let source = file.source();
    let mime_type = file.mime_type();
    let Some(bytes) = file.read().await else {
        load_status.write().unwrap().error = Some(format!("Could not read {file_name}"));
        return;
    };

    let is_nif = file_name.ends_with(".nif");
    let files = if file_name.ends_with(".zip")
        || file_name.ends_with(".bsa")
        || mime_type == "application/zip"
    {
        match crate::state::file::extract_archive(bytes, &load_status) {
            Ok(files) => files,
            Err(error) => {
                let mut status = load_status.write().unwrap();
                status.phase = None;
                status.error = Some(format!("Could not open {file_name}: {error}"));
                return;
            }
        }
    } else {
        HashMap::from([(file_name.clone(), bytes)])
    };

    *file_system.write().unwrap() = Some(Box::new(crate::state::file::HashmapFS::new_from_vec(
        String::new(),
        files,
    )));
    load_status.write().unwrap().error = None;
    *pending_picker_source.write().unwrap() = source;
    *pending_picker_recent_source.write().unwrap() = Some(crate::RecentFileSource::Disk);
    if is_nif {
        *pending_picker_file.write().unwrap() = Some(file_name);
    }
}

pub fn top_panel(viewport_ui: &mut Ui, params: &mut UiSystemParams) -> InnerResponse<()> {
    egui::Panel::top("top_panel")
        .resizable(false)
        .show(viewport_ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open File").clicked() {
                        let file_system = params.fsstate.file_system.clone();
                        let pending_picker_file = params.state.archive.pending_picker_file.clone();
                        let pending_picker_source =
                            params.state.archive.pending_picker_source.clone();
                        let pending_picker_recent_source = params
                            .state
                            .archive
                            .pending_picker_recent_source
                            .clone();
                        let load_status = params.state.archive.archive_load_status.clone();

                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen_futures::spawn_local;

                            spawn_local(async move {
                                if let Some(file) =
                                    file::pick_single_file(".nif,.zip,.bsa,application/zip").await
                                {
                                    load_file(
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
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(file) = futures::executor::block_on(file::pick_single_file(
                            ".nif,.zip,.bsa,application/zip",
                        )) {
                            params.state.archive.recent_source = crate::RecentFileSource::Disk;
                            futures::executor::block_on(load_file(
                                file,
                                file_system,
                                pending_picker_file,
                                pending_picker_source,
                                pending_picker_recent_source,
                                load_status,
                            ));
                        }

                        ui.close();
                    }

                    file::draw_recent_menu(ui, &mut params.state, &mut params.fsstate);

                    let can_export = !params.export_meshes.is_empty();
                    ui.menu_button("Export", |ui| {
                        if ui
                            .add_enabled(can_export, egui::Button::new("OBJ"))
                            .clicked()
                        {
                            crate::ui::export::export_loaded_meshes(
                                params.state.archive.selected_file.as_deref(),
                                &params.meshes,
                                &params.export_meshes,
                                &params.pending_texture_loads,
                            );
                            ui.close();
                        }
                    });

                    #[cfg(target_arch = "wasm32")]
                    ui.separator();

                    #[cfg(target_arch = "wasm32")]
                    if ui.button("Load URL").clicked() {
                        file::open_url_dialog(&mut params.state, &mut params.fsstate);
                        ui.close();
                    }

                    #[cfg(target_arch = "wasm32")]
                    if ui
                        .add_enabled(
                            params.state.archive.recent_source == crate::RecentFileSource::Url,
                            egui::Button::new("Download Archive"),
                        )
                        .clicked()
                    {
                        file::download_archive(params.state.archive.zip_url_input.clone());
                        ui.close();
                    }

                    #[cfg(target_arch = "wasm32")]
                    if ui
                        .add_enabled(
                            params.state.archive.recent_source == crate::RecentFileSource::Url
                                && params.state.archive.selected_file.is_some(),
                            egui::Button::new("Download File"),
                        )
                        .clicked()
                        && let Some(file_name) = params.state.archive.selected_file.as_deref()
                    {
                        file::download_file(&params.fsstate, file_name);
                        ui.close();
                    }

                    #[cfg(target_arch = "wasm32")]
                    if ui.button("Upload File").clicked() {
                        file::open_upload_picker(&mut params.state);
                    }
                });

                ui.menu_button("Settings", |ui| {
                    if ui.button("Key Bindings").clicked() {
                        params.state.top_panel.show_keybindings = true;
                        ui.close();
                    }
                    if ui.button("Resources").clicked() {
                        params.state.top_panel.show_resources = true;
                        ui.close();
                    }
                });

                draw_view_controls(ui, params);
            });
        })
}

/// Draws composable rendering controls and applies changed options to loaded entities.
fn draw_view_controls(ui: &mut Ui, params: &mut UiSystemParams) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let previous_options = crate::ViewOptions::from(&*params.state);

        ui.label(format!(
            "{} triangles",
            params.state.inspector.triangle_count
        ));
        ui.checkbox(&mut params.state.view.wireframe, "Wireframe");
        egui::ComboBox::from_label("Collision")
            .selected_text(params.state.view.collision.label())
            .show_ui(ui, |ui| {
                for mode in crate::DisplayMode::ALL {
                    ui.selectable_value(&mut params.state.view.collision, mode, mode.label());
                }
            });

        ui.add_enabled_ui(
            params.state.view.shading_mode != crate::ShadingMode::Normals,
            |ui| {
                egui::ComboBox::from_label("Vertex colors")
                    .selected_text(params.state.view.vertex_colors.label())
                    .show_ui(ui, |ui| {
                        for mode in crate::DisplayMode::ALL {
                            ui.selectable_value(
                                &mut params.state.view.vertex_colors,
                                mode,
                                mode.label(),
                            );
                        }
                    });
            },
        );

        for mode in [
            crate::ShadingMode::Normals,
            crate::ShadingMode::Unlit,
            crate::ShadingMode::Lit,
        ] {
            ui.selectable_value(&mut params.state.view.shading_mode, mode, mode.label());
        }

        let view_options = crate::ViewOptions::from(&*params.state);
        if view_options != previous_options {
            crate::nif::apply_view_options(
                view_options,
                &mut params.materials,
                &params.pending_texture_loads,
                &mut params.loaded_materials,
                &mut params.loaded_wireframes,
            );

            #[cfg(target_arch = "wasm32")]
            if params.state.archive.recent_source == crate::RecentFileSource::Url {
                crate::state::query::update_query(&crate::state::query::QueryState {
                    path: params.state.archive.zip_url_input.clone(),
                    selected_file: params
                        .state
                        .archive
                        .selected_file
                        .as_deref()
                        .map(|s| s.into())
                        .unwrap_or_default(),
                    source: params.state.archive.recent_source,
                    view_state: params.state.view.clone(),
                });
            }
        }
    });
}
