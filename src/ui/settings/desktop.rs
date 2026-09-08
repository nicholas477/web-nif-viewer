use std::{fs, path::PathBuf};

use bevy_egui::egui;
use directories::ProjectDirs;
use rfd::FileDialog;

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct ResourceConfig {
    resource_paths: Vec<String>,
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "nicholas477", "nif-viewer")
        .map(|project_dirs| project_dirs.config_dir().join("config.toml"))
}

pub fn initialize_resources(
    mut state: bevy::prelude::ResMut<crate::UIState>,
    mut fsstate: bevy::prelude::ResMut<crate::state::FSState>,
) {
    let paths = config_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| toml::from_str::<ResourceConfig>(&contents).ok())
        .unwrap_or_default()
        .resource_paths;
    state.top_panel.resource_paths = paths;
    rebuild_resources(&mut fsstate, &state.top_panel.resource_paths);
}

fn save_resources(paths: &[String]) {
    let Some(path) = config_path() else {
        bevy::log::warn!("Could not determine the configuration directory for resource folders.");
        return;
    };
    let config = ResourceConfig {
        resource_paths: paths.to_vec(),
    };
    let Ok(contents) = toml::to_string(&config) else {
        bevy::log::warn!("Could not serialize resource folder settings.");
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(error) = fs::write(path, contents) {
        bevy::log::warn!("Could not save resource folder settings: {error}");
    }
}

fn rebuild_resources(fsstate: &mut crate::state::FSState, paths: &[String]) {
    let mut write = fsstate.resource_file_systems.write().unwrap();

    write.clear();
    for path in paths {
        write.push(crate::state::file::RealFS::new(path.clone()));
    }
}

pub fn draw_resources(ctx: &bevy_egui::egui::Context, params: &mut crate::ui::UiSystemParams) {
    if !params.state.top_panel.show_resources {
        return;
    }

    let mut open = params.state.top_panel.show_resources;
    let mut changed = false;
    let mut selected = params.state.top_panel.selected_resource;
    let paths = params.state.top_panel.resource_paths.clone();
    bevy_egui::egui::Window::new("Resources")
        .open(&mut open)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label("Resource folders are searched in this order when an archive references a texture.");
            ui.separator();
            
            if !paths.is_empty() {
                egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(300.0).show(ui, |ui| {
                    for (index, path) in paths.iter().enumerate() {
                        if ui
                            .selectable_label(
                                selected == Some(index),
                                format!("{} - {path}", index + 1).to_string(),
                            )
                            .clicked()
                        {
                            selected = Some(index);
                        }
                    }
                });
            }
            else
            {
                ui.label("No resource folders configured.");
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add folder").clicked()
                    && let Some(path) = FileDialog::new().pick_folder()
                {
                    params.state.top_panel.resource_paths.push(path.display().to_string());
                    selected = Some(params.state.top_panel.resource_paths.len() - 1);
                    changed = true;
                }
                if ui.add_enabled(selected.is_some(), bevy_egui::egui::Button::new("Remove folder")).clicked()
                    && let Some(index) = selected
                {
                    params.state.top_panel.resource_paths.remove(index);
                    selected = index.checked_sub(1)
                        .or_else(|| (!params.state.top_panel.resource_paths.is_empty()).then_some(0));
                    changed = true;
                }
                if ui.add_enabled(selected.is_some_and(|index| index > 0), bevy_egui::egui::Button::new("Move up")).clicked()
                    && let Some(index) = selected
                {
                    params.state.top_panel.resource_paths.swap(index, index - 1);
                    selected = Some(index - 1);
                    changed = true;
                }
                if ui.add_enabled(selected.is_some_and(|index| index + 1 < params.state.top_panel.resource_paths.len()), bevy_egui::egui::Button::new("Move down")).clicked()
                    && let Some(index) = selected
                {
                    params.state.top_panel.resource_paths.swap(index, index + 1);
                    selected = Some(index + 1);
                    changed = true;
                }
            });
        });

    params.state.top_panel.show_resources = open;
    params.state.top_panel.selected_resource = selected;
    if changed {
        save_resources(&params.state.top_panel.resource_paths);
        rebuild_resources(&mut params.fsstate, &params.state.top_panel.resource_paths);
        params.state.archive.pending_file = params.state.archive.selected_file.clone();
        bevy::log::info!("Resource folders changed; reloading textures for the active NIF");
    }
}
