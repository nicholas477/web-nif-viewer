use std::collections::HashSet;

use bevy_egui::egui::{self, Ui};

/// Draws the file list and selectable NIF object hierarchy.
pub fn draw(ui: &mut Ui, file_names: &[String], state: &mut crate::UIState) -> Option<String> {
    ui.heading("Files");
    ui.separator();

    if file_names.is_empty() {
        ui.label("No files loaded");
        return None;
    }

    let mut sorted_file_names = file_names.to_vec();
    sorted_file_names.sort_unstable();
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
                    for file_name in sorted_file_names {
                        let is_selected =
                            state.archive.selected_file.as_deref() == Some(file_name.as_str());
                        if ui.selectable_label(is_selected, &file_name).clicked() {
                            state.archive.selected_file = Some(file_name.clone());
                            clicked_file = Some(file_name);
                        }
                    }
                });
        });

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

/// Draws details for the selected object below the NIF hierarchy.
pub fn draw_node_panel(ui: &mut Ui, state: &crate::UIState) {
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
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("nif_node_details_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.code(&object.fields);
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
        ui.selectable_label(*selected_node == Some(index), label)
            .clicked()
    } else {
        egui::CollapsingHeader::new(&label)
            .id_salt(index)
            .show(ui, |ui| {
                for &child in &object.children {
                    draw_object(ui, objects, selected_node, ancestor_nodes, child);
                }
            })
            .header_response
            .clicked()
    };
    if clicked {
        *selected_node = Some(index);
    }
    ancestor_nodes.remove(&index);
}
