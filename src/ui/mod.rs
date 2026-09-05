mod file;
mod inspector;

#[cfg(target_arch = "wasm32")]
use crate::state::query;
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiContexts, egui};
use egui::{LayerId, Ui, UiBuilder};

#[cfg(target_arch = "wasm32")]
pub use file::initialize_from_url;

/// Draws the viewer UI, processes file selection, and updates the 3D viewport bounds.
pub fn ui_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    camera3d: Single<
        (
            &mut Camera3d,
            &Projection,
            &mut crate::camera::PanOrbitCamera,
        ),
        Without<EguiContext>,
    >,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::PhongMaterial>>,
    loaded_meshes: Query<Entity, With<crate::nif::LoadedNifMesh>>,
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
    loaded_wireframe_entities: Query<Entity, With<crate::nif::LoadedNifWireframe>>,
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
        .archive
        .file_system
        .read()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let window = window.into_inner().into_inner();
    let (_, projection, mut pan_orbit) = camera3d.into_inner();

    #[cfg(target_arch = "wasm32")]
    let uploaded_download_url = {
        state
            .archive
            .upload_status
            .write()
            .unwrap()
            .download_url
            .take()
    };
    #[cfg(target_arch = "wasm32")]
    if let Some(download_url) = uploaded_download_url {
        file::start_archive_load(&mut state, download_url, None);
    }

    load_pending_nif(
        &mut state,
        &file_names,
        &mut commands,
        &mut meshes,
        &mut images,
        &mut materials,
        &loaded_meshes,
        &loaded_wireframe_entities,
        projection,
        window,
        &mut pan_orbit,
    );

    let left_panel = egui::Panel::left("left_panel")
        .default_size(400.0)
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            inspector::draw(ui, &file_names, &mut state)
        });
    let mut left = left_panel.response.rect.width();

    if let Some(file_name) = left_panel.inner
        && file_name.to_lowercase().ends_with(".nif")
    {
        #[cfg(target_arch = "wasm32")]
        query::update_query(&crate::state::query::QueryState {
            zip_url: state.archive.zip_url_input.clone(),
            selected_file: file_name.clone(),
            view_state: state.view.clone(),
        });

        load_nif(
            &file_name,
            &mut state,
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &loaded_meshes,
            &loaded_wireframe_entities,
            projection,
            window,
            &mut pan_orbit,
        );
    }

    let top = egui::Panel::top("top_panel")
        .resizable(false)
        .show(&mut viewport_ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked() {
                    file::open_archive_picker(&mut state);
                }
                #[cfg(target_arch = "wasm32")]
                if ui.button("Upload File").clicked() {
                    file::open_upload_picker(&mut state);
                }
                file::draw_recent_menu(ui, &mut state);
                draw_view_controls(
                    ui,
                    &mut state,
                    &mut materials,
                    &mut loaded_materials,
                    &mut loaded_wireframes,
                );
            });
        })
        .response
        .rect
        .height();

    left *= window.scale_factor();
    let top = top * window.scale_factor();
    let position = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height()) - position;
    camera.viewport = Some(Viewport {
        physical_position: position,
        physical_size: size,
        ..default()
    });

    #[cfg(target_arch = "wasm32")]
    if state.archive.show_zip_popup {
        file::draw_zip_popup(ctx, &mut state);
    }
    file::draw_load_status(ctx, &state);
    file::draw_upload_status(ctx, &state);
    file::draw_error_popup(ctx, &mut state);
    file::draw_upload_result_popup(ctx, &mut state);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Loads a requested NIF once its containing archive has made the file available.
fn load_pending_nif(
    state: &mut crate::UIState,
    file_names: &[String],
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<crate::PhongMaterial>,
    loaded_meshes: &Query<Entity, With<crate::nif::LoadedNifMesh>>,
    loaded_wireframes: &Query<Entity, With<crate::nif::LoadedNifWireframe>>,
    projection: &Projection,
    window: &Window,
    pan_orbit: &mut crate::camera::PanOrbitCamera,
) {
    let Some(file_name) = state.archive.pending_file.clone() else {
        return;
    };
    if !file_names.contains(&file_name) || !file_name.to_lowercase().ends_with(".nif") {
        return;
    }
    state.archive.pending_file = None;
    state.archive.selected_file = Some(file_name.clone());
    load_nif(
        &file_name,
        state,
        commands,
        meshes,
        images,
        materials,
        loaded_meshes,
        loaded_wireframes,
        projection,
        window,
        pan_orbit,
    );
}

#[allow(clippy::too_many_arguments)]
/// Loads a NIF into Bevy assets, records failures, and frames the camera on success.
fn load_nif(
    file_name: &str,
    state: &mut crate::UIState,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<crate::PhongMaterial>,
    loaded_meshes: &Query<Entity, With<crate::nif::LoadedNifMesh>>,
    loaded_wireframes: &Query<Entity, With<crate::nif::LoadedNifWireframe>>,
    projection: &Projection,
    window: &Window,
    pan_orbit: &mut crate::camera::PanOrbitCamera,
) {
    let file_system = state.archive.file_system.clone();
    let view_options = crate::ViewOptions::from(&*state);
    let inspector = &mut state.inspector;
    match crate::nif::load_nif(
        file_name,
        &file_system,
        &mut inspector.nif_objects,
        &mut inspector.nif_roots,
        &mut inspector.triangle_count,
        view_options,
        commands,
        meshes,
        images,
        materials,
        loaded_meshes,
        loaded_wireframes,
    ) {
        Ok(()) => {
            file::record_recent_file(&state.archive.zip_url_input, file_name);
            crate::camera::focus_loaded_meshes(meshes, projection, window, pan_orbit);
        }
        Err(error) => state.archive.nif_load_error = Some(error),
    }
}

/// Draws composable rendering controls and applies changed options to loaded entities.
fn draw_view_controls(
    ui: &mut Ui,
    state: &mut crate::UIState,
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
            query::update_query(&query::QueryState {
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
