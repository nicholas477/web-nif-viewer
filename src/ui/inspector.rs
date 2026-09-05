use bevy_egui::egui::{self, Ui};

/// Draws the resizable file list and the scrollable NIF object hierarchy.
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
        ui.heading("NIF Inspector");
        ui.separator();
        let inspector_size = egui::vec2(ui.available_width(), ui.available_height());
        ui.allocate_ui_with_layout(
            inspector_size,
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("nif_inspector_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for &root in &state.inspector.nif_roots {
                            draw_object(ui, &state.inspector.nif_objects, root);
                        }
                    });
            },
        );
    }

    clicked_file
}

/// Recursively draws one NIF object and its node children.
fn draw_object(ui: &mut Ui, objects: &[crate::NifObjectInfo], index: usize) {
    let Some(object) = objects.get(index) else {
        return;
    };

    egui::CollapsingHeader::new(format!("{index}: {}", object.type_name))
        .id_salt(index)
        .show(ui, |ui| {
            ui.code(&object.fields);
            for &child in &object.children {
                draw_object(ui, objects, child);
            }
        });
}
