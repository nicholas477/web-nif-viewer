mod file;
mod inspector;
pub mod mesh_selection;
mod settings;
mod top_panel;

use std::ops::DerefMut;

use bevy::{camera::Viewport, ecs::system::SystemParam, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiContexts, egui};
use egui::{LayerId, Ui, UiBuilder};

#[cfg(not(target_arch = "wasm32"))]
pub use file::initialize_default_mesh;
pub use settings::initialize_resources;

#[cfg(target_arch = "wasm32")]
pub use settings::initialize_default_mesh_after_resources;

use crate::{file::Filesystem, nif::LoadedNifMesh};

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
    pub pending_texture_loads: ResMut<'w, crate::nif::PendingTextureLoads>,
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
    crate::nif::apply_completed_texture_loads(
        &params.pending_texture_loads,
        &mut params.images,
        &mut params.materials,
    );
    let ctx = params.contexts.ctx_mut()?.clone();
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let loaded_file_names = params
        .fsstate
        .file_system
        .read()
        .unwrap()
        .as_ref()
        .map(|file_system| file_system.absolute_paths())
        .unwrap_or_default();
    let resource_paths = params.state.top_panel.resource_paths.clone();
    let resource_file_names = params
        .fsstate
        .resource_file_systems
        .read()
        .unwrap()
        .iter()
        .map(|file_system| file_system.absolute_paths())
        .collect::<Vec<_>>();
    let missing_paths = params.pending_texture_loads.missing_paths();

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
        file::start_archive_load(
            &mut params.state,
            &mut params.fsstate,
            download_url,
            None,
            crate::RecentFileSource::Url,
        );
    }

    let left_panel = egui::Panel::left("left_panel")
        .default_size(400.0)
        .resizable(true)
        .min_size(150.0)
        .show(&mut viewport_ui, |ui| {
            inspector::draw(
                ui,
                &loaded_file_names,
                &resource_paths,
                &resource_file_names,
                &missing_paths,
                &mut params.state,
            )
        });
    let mut left = left_panel.response.rect.width();

    if let Some(file_name) = left_panel.inner
        && file_name.to_lowercase().ends_with(".nif")
    {
        #[cfg(target_arch = "wasm32")]
            if params.state.archive.recent_source == crate::RecentFileSource::Url {
                crate::state::query::update_query(&crate::state::query::QueryState {
                    path: params.state.archive.zip_url_input.clone(),
                    selected_file: file_name.clone(),
                    source: params.state.archive.recent_source,
                    view_state: params.state.view.clone(),
                });
            }

        load_nif(&file_name, &mut params);
    }

    // Top panel
    let top = top_panel::top_panel(&mut viewport_ui, &mut params)
        .response
        .rect
        .height();

    let picked_file = params
        .state
        .archive
        .pending_picker_file
        .write()
        .unwrap()
        .take();
    let picked_source = params
        .state
        .archive
        .pending_picker_source
        .write()
        .unwrap()
        .take();
    let picked_recent_source = params
        .state
        .archive
        .pending_picker_recent_source
        .write()
        .unwrap()
        .take();
    let has_picked_source = picked_source.is_some();
    if let Some(source) = picked_source {
        params.state.archive.zip_url_input = source;
    }
    if let Some(source) = picked_recent_source {
        params.state.archive.recent_source = source;
    }
    if let Some(file_name) = picked_file {
        params.state.archive.selected_file = Some(file_name.clone());
        params.state.archive.pending_file = Some(file_name);
    }

    #[cfg(target_arch = "wasm32")]
    if params.state.archive.recent_source == crate::RecentFileSource::Disk
        && has_picked_source
    {
        crate::state::query::update_query(&crate::state::query::QueryState {
            path: params.state.archive.zip_url_input.clone(),
            selected_file: params
                .state
                .archive
                .pending_file
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            source: crate::RecentFileSource::Disk,
            view_state: params.state.view.clone(),
        });
    }

    load_pending_nif(&mut params);

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

    settings::draw_resources(&ctx, &mut params);

    #[cfg(target_arch = "wasm32")]
    file::draw_url_dialog(&ctx, &mut params.state, &mut params.fsstate);

    file::draw_load_status(&ctx, &params.state);
    file::draw_upload_status(&ctx, &params.state);
    file::draw_error_popup(&ctx, &mut params.state);
    file::draw_upload_result_popup(&ctx, &mut params.state);
    Ok(())
}

/// Loads a requested NIF once its containing archive has made the file available.
fn load_pending_nif(params: &mut UiSystemParams) {
    let Some(file_name) = params.state.archive.pending_file.clone() else {
        return;
    };
    let file_names = params.fsstate.absolute_paths();
    if !file_names.contains(&file_name) || !file_name.to_lowercase().ends_with(".nif") {
        return;
    }
    params.state.archive.pending_file = None;
    params.state.archive.selected_file = Some(file_name.clone());
    load_nif(&file_name, params);
}

/// Loads a NIF into Bevy assets, records failures, and frames the camera on success.
pub(crate) fn load_nif(file_name: &str, params: &mut UiSystemParams) {
    bevy::log::info!("Loading NIF file: {file_name}");
    params.pending_texture_loads.clear_missing_paths();
    for resource_file_system in params.fsstate.resource_file_systems.read().unwrap().iter() {
        resource_file_system.clear_cache();
    }

    let bytes = if let Some(fs) = params.fsstate.file_system.write().unwrap().deref_mut() {
        let bytes = bevy::tasks::futures_lite::future::block_on(
            bevy::tasks::futures_lite::future::poll_once(fs.read(file_name, true)),
        )
        .flatten()
        .map(|bytes| bytes.to_vec());

        if bytes.is_some() {
            fs.rebase_to_file(file_name);
        }

        bytes
    } else {
        bevy::log::warn!("No filesystem available");
        return;
    };

    let Some(bytes) = bytes else {
        bevy::log::warn!("Unable to find nif file: {file_name}");
        return;
    };

    match crate::nif::load_nif(crate::nif::NifMeshLoadParams::from_ui_state(
            bytes.as_slice(),
            params,
        )) {
            Ok(()) => {
                crate::state::recent_files::record_recent_file(
                    params.state.archive.recent_source,
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
