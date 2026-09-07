use crate::ui::{UiSystemParams, file};
use bevy::prelude::*;
use bevy_egui::egui::{self, InnerResponse, Ui};

pub fn top_panel(
    viewport_ui: &mut Ui,
    params: &mut UiSystemParams,
) -> InnerResponse<()> {
    egui::Panel::top("top_panel")
        .resizable(false)
        .show(viewport_ui, |ui| {
            ui.horizontal(|ui| {
                #[cfg(target_arch = "wasm32")]
                if ui.button("Load URL").clicked() {
                    file::open_archive_picker(&mut params.state, &mut params.fsstate);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Open File").clicked() {
                    file::open_archive_picker(&mut params.state, &mut params.fsstate);
                }
                #[cfg(target_arch = "wasm32")]
                if ui.button("Upload File").clicked() {
                    file::open_upload_picker(state);
                }
                file::draw_recent_menu(ui, &mut params.state, &mut params.fsstate);
                draw_view_controls(
                    ui,
                    params,
                );
            });
        })
}

/// Draws composable rendering controls and applies changed options to loaded entities.
fn draw_view_controls(
    ui: &mut Ui,
    params: &mut UiSystemParams,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let previous_options = crate::ViewOptions::from(&*params.state);

        ui.label(format!("{} triangles", params.state.inspector.triangle_count));
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
                            ui.selectable_value(&mut params.state.view.vertex_colors, mode, mode.label());
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
                &mut params.loaded_materials,
                &mut params.loaded_wireframes,
            );

            #[cfg(target_arch = "wasm32")]
            crate::state::query::update_query(&crate::state::query::QueryState {
                zip_url: state.archive.zip_url_input.clone(),
                selected_file: state
                    .archive
                    .selected_file
                    .as_deref()
                    .map(|s| s.into())
                    .unwrap_or_default(),
                view_state: state.view.clone(),
            });
        }
    });
}
