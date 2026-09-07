use crate::ui::file;
use bevy::{
    asset::Assets, camera::visibility::Visibility, ecs::system::Query, mesh::Mesh3d,
    pbr::MeshMaterial3d,
};
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use bevy_egui::egui::{self, InnerResponse, Ui};
use bevy_egui::{EguiContext, EguiContexts};
use egui::{LayerId, UiBuilder};

pub fn top_panel(
    viewport_ui: &mut Ui,
    state: &mut crate::state::UIState,
    fsstate: &mut crate::state::FSState,
    mut materials: ResMut<Assets<crate::PhongMaterial>>,
    mut loaded_materials: Query<
        (
            &mut Mesh3d,
            &MeshMaterial3d<crate::PhongMaterial>,
            &mut Visibility,
            &crate::nif::LoadedNifMesh,
        ),
        Without<crate::nif::LoadedNifWireframe>,
    >,
    mut loaded_wireframes: Query<
        (&mut Visibility, &crate::nif::LoadedNifWireframe),
        Without<crate::nif::LoadedNifMesh>,
    >,
) -> InnerResponse<()> {
    egui::Panel::top("top_panel")
        .resizable(false)
        .show(viewport_ui, |ui| {
            ui.horizontal(|ui| {
                #[cfg(target_arch = "wasm32")]
                if ui.button("Load URL").clicked() {
                    file::open_archive_picker(state, fsstate);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Open File").clicked() {
                    file::open_archive_picker(state, fsstate);
                }
                #[cfg(target_arch = "wasm32")]
                if ui.button("Upload File").clicked() {
                    file::open_upload_picker(state);
                }
                file::draw_recent_menu(ui, state, fsstate);
                draw_view_controls(
                    ui,
                    state,
                    &mut materials,
                    &mut loaded_materials,
                    &mut loaded_wireframes,
                );
            });
        })
}

/// Draws composable rendering controls and applies changed options to loaded entities.
fn draw_view_controls(
    ui: &mut Ui,
    state: &mut crate::state::UIState,
    materials: &mut Assets<crate::PhongMaterial>,
    loaded_meshes: &mut Query<
        (
            &mut Mesh3d,
            &MeshMaterial3d<crate::PhongMaterial>,
            &mut Visibility,
            &crate::nif::LoadedNifMesh,
        ),
        Without<crate::nif::LoadedNifWireframe>,
    >,
    loaded_wireframes: &mut Query<
        (&mut Visibility, &crate::nif::LoadedNifWireframe),
        Without<crate::nif::LoadedNifMesh>,
    >,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let previous_options = crate::ViewOptions::from(&*state);

        ui.label(format!("{} triangles", state.inspector.triangle_count));
        ui.checkbox(&mut state.view.wireframe, "Wireframe");
        egui::ComboBox::from_label("Collision")
            .selected_text(state.view.collision.label())
            .show_ui(ui, |ui| {
                for mode in crate::DisplayMode::ALL {
                    ui.selectable_value(&mut state.view.collision, mode, mode.label());
                }
            });

        ui.add_enabled_ui(
            state.view.shading_mode != crate::ShadingMode::Normals,
            |ui| {
                egui::ComboBox::from_label("Vertex colors")
                    .selected_text(state.view.vertex_colors.label())
                    .show_ui(ui, |ui| {
                        for mode in crate::DisplayMode::ALL {
                            ui.selectable_value(&mut state.view.vertex_colors, mode, mode.label());
                        }
                    });
            },
        );

        for mode in [
            crate::ShadingMode::Normals,
            crate::ShadingMode::Unlit,
            crate::ShadingMode::Lit,
        ] {
            ui.selectable_value(&mut state.view.shading_mode, mode, mode.label());
        }

        let view_options = crate::ViewOptions::from(&*state);
        if view_options != previous_options {
            crate::nif::apply_view_options(
                view_options,
                materials,
                loaded_meshes,
                loaded_wireframes,
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
