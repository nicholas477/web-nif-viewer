use std::collections::HashSet;

use bevy_egui::egui::{self, Ui};
use tes3::nif::Inspect;

/// Draws the file list and selectable NIF object hierarchy.
pub fn draw(
    ui: &mut Ui,
    loaded_file_names: &[String],
    archive_base: Option<&str>,
    resource_paths: &[String],
    resource_file_names: &[Vec<String>],
    missing_paths: &[String],
    state: &mut crate::state::UIState,
) -> Option<String> {
    let mut clicked_file = None;

    let file_list_width = ui.available_width();
    let file_list_max_height = (ui.available_height() - 120.0).max(72.0);

    egui::Resize::default()
        .id_salt("file_list_resize")
        .default_width(file_list_width)
        .default_height(180.0)
        .min_width(file_list_width)
        .min_height(72.0)
        .max_width(file_list_width)
        .max_height(file_list_max_height)
        .resizable([false, true])
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("file_list_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Loaded files")
                        .default_open(true)
                        .show(ui, |ui| {
                            if let Some(archive_base) = archive_base.filter(|base| !base.is_empty()) {
                                ui.label(format!("Archive base: {archive_base}"));
                            }
                            if loaded_file_names.is_empty() {
                                ui.label("No archive or NIF loaded");
                            } else {
                                clicked_file = draw_file_list(ui, loaded_file_names, state);
                            }
                        });

                    egui::CollapsingHeader::new("Resources")
                        .default_open(true)
                        .show(ui, |ui| {
                            if resource_paths.is_empty() {
                                ui.label("No resource folders configured");
                            } else {
                                for (index, path) in resource_paths.iter().enumerate() {
                                    egui::CollapsingHeader::new(path)
                                        .id_salt(("resource_files", index))
                                        .show(ui, |ui| {
                                            let files = resource_file_names
                                                .get(index)
                                                .map(Vec::as_slice)
                                                .unwrap_or_default();
                                            if files.is_empty() {
                                                ui.label("No referenced files loaded");
                                            } else {
                                                draw_reference_list(
                                                    ui,
                                                    files,
                                                    ("resource_file_list", index),
                                                );
                                            }
                                        });
                                }
                            }
                        });

                    egui::CollapsingHeader::new("Missing references")
                        .default_open(true)
                        .show(ui, |ui| {
                            if missing_paths.is_empty() {
                                ui.label("No missing references");
                            } else {
                                for path in missing_paths {
                                    ui.label(path);
                                }
                            }
                        });
                })
        });

    if loaded_file_names.is_empty() {
        return clicked_file;
    }

    ui.add_space(12.0);

    if !state.inspector.nif_objects.is_empty() {
        ui.add_space(12.0);
        ui.heading("Hierarchy");
        ui.separator();
        let hierarchy_width = ui.available_width();
        let hierarchy_max_height = (ui.available_height() - 160.0).max(96.0);
        egui::Resize::default()
            .id_salt("nif_hierarchy_resize")
            .default_width(hierarchy_width)
            .default_height(240.0)
            .min_width(hierarchy_width)
            .min_height(96.0)
            .max_width(hierarchy_width)
            .max_height(hierarchy_max_height)
            .resizable([false, true])
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("nif_hierarchy_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut root_nodes = HashSet::new();
                        for &root in &state.inspector.nif_roots {
                            root_nodes.insert(root);
                            draw_object(
                                ui,
                                &state.inspector.nif_objects,
                                &mut state.inspector.selected_node,
                                &mut HashSet::new(),
                                root,
                            );
                        }

                        let referenced_nodes = state
                            .inspector
                            .nif_objects
                            .iter()
                            .flat_map(|object| &object.children)
                            .copied()
                            .collect::<HashSet<_>>();
                        let unparented_nodes = (0..state.inspector.nif_objects.len())
                            .filter(|index| {
                                !root_nodes.contains(index) && !referenced_nodes.contains(index)
                            })
                            .collect::<Vec<_>>();
                        if !unparented_nodes.is_empty() {
                            egui::CollapsingHeader::new("Unparented Nodes")
                                .id_salt("unparented_nif_nodes")
                                .default_open(true)
                                .show(ui, |ui| {
                                    for index in unparented_nodes {
                                        draw_object(
                                            ui,
                                            &state.inspector.nif_objects,
                                            &mut state.inspector.selected_node,
                                            &mut HashSet::new(),
                                            index,
                                        );
                                    }
                                });
                        }
                    });
            });

        ui.add_space(12.0);
        draw_node_panel(ui, state);
    }

    clicked_file
}

fn draw_file_list(
    ui: &mut Ui,
    file_names: &[String],
    state: &mut crate::state::UIState,
) -> Option<String> {
    let mut sorted_file_names = file_names.to_vec();
    sorted_file_names.sort_unstable();
    let mut clicked_file = None;
    // let file_list_width = ui.available_width();
    // let file_list_max_height = (ui.available_height() - 120.0).max(72.0);

    // egui::Resize::default()
    //     .id_salt("file_list_resize")
    //     .default_width(file_list_width)
    //     .default_height(180.0)
    //     .min_width(file_list_width)
    //     .min_height(72.0)
    //     .max_width(file_list_width)
    //     .max_height(file_list_max_height)
    //     .resizable([false, true])
    //     .show(ui, |ui| {
    //         egui::ScrollArea::vertical()
    //             .id_salt("file_list_scroll")
    //             .auto_shrink([false, false])
    //             .show(ui, |ui| {
    for file_name in sorted_file_names {
        let is_selected = state.archive.selected_file.as_deref() == Some(file_name.as_str());
        if ui.selectable_label(is_selected, &file_name).clicked() {
            state.archive.selected_file = Some(file_name.clone());
            clicked_file = Some(file_name);
        }
    }
    //         });
    // });

    clicked_file
}

fn draw_reference_list(
    ui: &mut Ui,
    file_names: &[String],
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) {
    let mut sorted_file_names = file_names.to_vec();
    sorted_file_names.sort_unstable();

    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(180.0)
        .show(ui, |ui| {
            for file_name in sorted_file_names {
                ui.label(file_name);
            }
        });
}

/// Draws details for the selected object below the NIF hierarchy.
pub fn draw_node_panel(ui: &mut Ui, state: &crate::state::UIState) {
    ui.heading("Node");
    ui.separator();

    let Some(index) = state.inspector.selected_node else {
        ui.label("Select a node in the hierarchy");
        return;
    };
    let Some(object) = state.inspector.nif_objects.get(index) else {
        ui.label("Selected node is no longer available");
        return;
    };

    ui.label(format!("{index}: {}", object.type_name));

    egui::ScrollArea::both()
        .id_salt("nif_node_details_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("nif_node_details_grid")
                .striped(true)
                .num_columns(3)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Value");
                    ui.strong("Type");
                    ui.end_row();

                    let all_properties = object.object.properties();

                    for property in all_properties {
                        let text = property
                            .value
                            .to_string()
                            .chars()
                            .take(50)
                            .collect::<String>();
                        ui.label(property.name);
                        ui.label(text);
                        ui.label(property.type_name);
                        ui.end_row();
                    }
                });
        });
}

/// Recursively draws one selectable NIF object and its node children.
fn draw_object(
    ui: &mut Ui,
    objects: &[crate::NifObjectInfo],
    selected_node: &mut Option<usize>,
    ancestor_nodes: &mut HashSet<usize>,
    index: usize,
) {
    if !ancestor_nodes.insert(index) {
        ui.label(format!("{index}: cyclic reference"));
        return;
    }
    let Some(object) = objects.get(index) else {
        return;
    };

    let label = format!("{index}: {}", object.type_name);
    let clicked = if object.children.is_empty() {
        ui.horizontal(|ui| {
            ui.allocate_space(egui::vec2(18.0, 18.0));
            ui.selectable_label(*selected_node == Some(index), label)
                .clicked()
        })
        .inner
    } else {
        let id = ui.make_persistent_id(("nif_hierarchy_node", index));
        let mut collapsing_state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
        let header_response = ui.horizontal(|ui| {
            let arrow = if collapsing_state.is_open() { "v" } else { ">" };
            if ui
                .add_sized(
                    [18.0, 18.0],
                    egui::Label::new(arrow).sense(egui::Sense::click()),
                )
                .clicked()
            {
                collapsing_state.toggle(ui);
            }
            ui.selectable_label(*selected_node == Some(index), &label)
                .clicked()
        });
        collapsing_state.show_body_indented(&header_response.response, ui, |ui| {
            for &child in &object.children {
                draw_object(ui, objects, selected_node, ancestor_nodes, child);
            }
        });
        header_response.inner
    };
    if clicked {
        *selected_node = Some(index);
    }
    ancestor_nodes.remove(&index);
}
