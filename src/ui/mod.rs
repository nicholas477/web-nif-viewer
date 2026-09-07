mod file;
mod inspector;
pub mod picking;
mod top_panel;

use bevy::{camera::Viewport, ecs::system::SystemParam, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiContexts, egui};
use egui::{LayerId, Ui, UiBuilder};

pub use file::initialize_default_mesh;

use crate::nif::LoadedNifMesh;

/// System parameter struct of doom and despair
#[derive(SystemParam)]
pub struct UiSystemParams<'w, 's> {
    pub contexts: EguiContexts<'w, 's>,
    pub state: ResMut<'w, crate::state::UIState>,
    pub fsstate: ResMut<'w, crate::state::FSState>,
    pub commands: Commands<'w, 's>,
    pub camera: Single<'w, 's, &'static mut Camera, Without<EguiContext>>,
    pub camera3d: Single<
        'w,
        's,
        (
            &'static mut Camera3d,
            &'static Projection,
            &'static mut crate::camera::PanOrbitCamera,
        ),
        Without<EguiContext>,
    >,
    pub window: Single<'w, 's, &'static mut Window, With<PrimaryWindow>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub materials: ResMut<'w, Assets<crate::PhongMaterial>>,
    pub loaded_meshes: Query<'w, 's, (Entity, &'static LoadedNifMesh)>,
    pub loaded_materials: Query<
        'w,
        's,
        (
            &'static mut Mesh3d,
            &'static MeshMaterial3d<crate::PhongMaterial>,
            &'static mut Visibility,
            &'static crate::nif::LoadedNifMesh,
        ),
        Without<crate::nif::LoadedNifWireframe>,
    >,
    pub loaded_wireframes: Query<
        'w,
        's,
        (
            &'static crate::nif::LoadedNifWireframe,
            &'static MeshMaterial3d<crate::PhongMaterial>,
            &'static mut Visibility,
        ),
        Without<crate::nif::LoadedNifMesh>,
    >,
    pub loaded_wireframe_entities: Query<'w, 's, Entity, With<crate::nif::LoadedNifWireframe>>,
}

/// Draws the viewer UI, processes file selection, and updates the 3D viewport bounds.
pub fn ui_system(mut params: UiSystemParams) -> Result {
    let ctx = params.contexts.ctx_mut()?.clone();
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let file_names = params.fsstate.file_system.read().unwrap().paths();

    #[cfg(target_arch = "wasm32")]
    let uploaded_download_url = {
        params
            .state
            .archive
            .upload_status
            .write()
            .unwrap()
            .download_url
            .take()
    };
    #[cfg(target_arch = "wasm32")]
    if let Some(download_url) = uploaded_download_url {
        file::start_archive_load(&mut params.state, &mut params.fsstate, download_url, None);
    }

    load_pending_nif(&file_names, &mut params);

    let left_panel = egui::Panel::left("left_panel")
        .default_size(400.0)
        .resizable(true)
        .min_size(150.0)
        .show(&mut viewport_ui, |ui| {
            inspector::draw(ui, &file_names, &mut params.state)
        });
    let mut left = left_panel.response.rect.width();

    if let Some(file_name) = left_panel.inner
        && file_name.to_lowercase().ends_with(".nif")
    {
        #[cfg(target_arch = "wasm32")]
        crate::state::query::update_query(&crate::state::query::QueryState {
            zip_url: params.state.archive.zip_url_input.clone(),
            selected_file: file_name.clone(),
            view_state: params.state.view.clone(),
        });

        load_nif(&file_name, &mut params);
    }

    // Top panel
    let top = top_panel::top_panel(&mut viewport_ui, &mut params)
        .response
        .rect
        .height();

    left *= params.window.scale_factor();
    let top = top * params.window.scale_factor();
    let position = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(
        params.window.physical_width().saturating_sub(left as u32),
        params.window.physical_height().saturating_sub(top as u32),
    );
    params.camera.viewport = Some(Viewport {
        physical_position: position,
        physical_size: size,
        ..default()
    });

    #[cfg(target_arch = "wasm32")]
    if params.state.archive.show_zip_popup {
        file::draw_zip_popup(&ctx, &mut params.state, &mut params.fsstate);
    }
    file::draw_load_status(&ctx, &params.state);
    file::draw_upload_status(&ctx, &params.state);
    file::draw_error_popup(&ctx, &mut params.state);
    file::draw_upload_result_popup(&ctx, &mut params.state);
    Ok(())
}

/// Loads a requested NIF once its containing archive has made the file available.
fn load_pending_nif(file_names: &[String], params: &mut UiSystemParams) {
    let Some(file_name) = params.state.archive.pending_file.clone() else {
        return;
    };
    if !file_names.contains(&file_name) || !file_name.to_lowercase().ends_with(".nif") {
        return;
    }
    params.state.archive.pending_file = None;
    params.state.archive.selected_file = Some(file_name.clone());
    load_nif(&file_name, params);
}

/// Loads a NIF into Bevy assets, records failures, and frames the camera on success.
fn load_nif(file_name: &str, params: &mut UiSystemParams) {
    match crate::nif::load_nif(crate::nif::NifMeshLoadParams::from_ui_state(
        file_name, params,
    )) {
        Ok(()) => {
            crate::state::recent_files::record_recent_file(
                &params.state.archive.zip_url_input,
                file_name,
            );
            let (_, projection, pan_orbit) = &mut *params.camera3d;
            crate::camera::focus_loaded_meshes(
                &params.meshes,
                projection,
                &params.window,
                pan_orbit,
            );
        }
        Err(error) => params.state.archive.nif_load_error = Some(error),
    }
}
