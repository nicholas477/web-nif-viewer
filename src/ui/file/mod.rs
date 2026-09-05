use bevy_egui::egui;

#[cfg(target_arch = "wasm32")]
pub const MAX_RECENT_FILES: usize = 10;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;

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
#[cfg(target_arch = "wasm32")]
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
