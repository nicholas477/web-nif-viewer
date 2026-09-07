mod file;
mod inspector;
mod top_panel;

use bevy::{camera::Viewport, ecs::system::SystemParam, prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiContexts, egui};
use egui::{LayerId, Ui, UiBuilder};

pub use file::initialize_default_mesh;

// #[derive(SystemParam)]
// pub struct UiSystemParams<'w, 's> {
//     pub state: ResMut<'w, crate::state::UIState>,
//     pub fsstate: ResMut<'w, crate::state::FSState>,
//     pub commands: Commands<'w, 's>,
//     pub camera: Single<'w, 's, &'static mut Camera, Without<EguiContext>>,
//     pub camera3d: Single<'w, 's, (
//         &'static mut Camera3d,
//         &'static Projection,
//         &'static mut crate::camera::PanOrbitCamera,
//     ), Without<EguiContext>>,
//     pub window: Single<'w, 's, &'static mut Window, With<PrimaryWindow>>,
//     pub meshes: ResMut<'w, Assets<Mesh>>,
//     pub images: ResMut<'w, Assets<Image>>,
//     pub materials: ResMut<'w, Assets<crate::PhongMaterial>>,
//     pub loaded_meshes: Query<'w, 's, Entity, With<crate::nif::LoadedNifMesh>>,
//     pub loaded_materials: Query<'w, 's, (
//         &'static mut Mesh3d,
//         &'static MeshMaterial3d<crate::PhongMaterial>,
//         &'static mut Visibility,
//         &'static crate::nif::LoadedNifMesh,
//     ), Without<crate::nif::LoadedNifWireframe>>,
//     pub loaded_wireframes: Query<'w, 's, (
//         &'static mut Visibility,
//         &'static crate::nif::LoadedNifWireframe,
//     ), Without<crate::nif::LoadedNifMesh>>,
//     pub loaded_wireframe_entities: Query<'w, 's, Entity, With<crate::nif::LoadedNifWireframe>>,
// }

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
    mut state: ResMut<crate::state::UIState>,
    mut fsstate: ResMut<crate::state::FSState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let file_names = fsstate.file_system.read().unwrap().paths();
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
        file::start_archive_load(&mut state, &mut fsstate, download_url, None);
    }

    load_pending_nif(
        &mut state,
        &fsstate,
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
        .min_size(150.0)
        .show(&mut viewport_ui, |ui| {
            inspector::draw(ui, &file_names, &mut state)
        });
    let mut left = left_panel.response.rect.width();

    if let Some(file_name) = left_panel.inner
        && file_name.to_lowercase().ends_with(".nif")
    {
        #[cfg(target_arch = "wasm32")]
        crate::state::query::update_query(&crate::state::query::QueryState {
            zip_url: state.archive.zip_url_input.clone(),
            selected_file: file_name.clone(),
            view_state: state.view.clone(),
        });

        load_nif(
            &file_name,
            &mut state,
            &fsstate,
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

    // Top panel
    let top = top_panel::top_panel(
        &mut viewport_ui,
        &mut state,
        &mut fsstate,
        materials,
        loaded_materials,
        loaded_wireframes,
    )
    .response
    .rect
    .height();

    left *= window.scale_factor();
    let top = top * window.scale_factor();
    let position = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(
        window.physical_width().saturating_sub(left as u32),
        window.physical_height().saturating_sub(top as u32),
    );
    camera.viewport = Some(Viewport {
        physical_position: position,
        physical_size: size,
        ..default()
    });

    #[cfg(target_arch = "wasm32")]
    if state.archive.show_zip_popup {
        file::draw_zip_popup(ctx, &mut state, &mut fsstate);
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
    state: &mut crate::state::UIState,
    fsstate: &crate::state::FSState,
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
        fsstate,
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
    state: &mut crate::state::UIState,
    fsstate: &crate::state::FSState,
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
    let view_options = crate::ViewOptions::from(&*state);
    let inspector = &mut state.inspector;
    match crate::nif::load_nif(
        file_name,
        fsstate.file_system.read().unwrap().as_ref() as &dyn crate::state::file::Filesystem,
        &mut inspector.nif_objects,
        &mut inspector.nif_roots,
        &mut inspector.selected_node,
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
            crate::state::recent_files::record_recent_file(&state.archive.zip_url_input, file_name);
            crate::camera::focus_loaded_meshes(meshes, projection, window, pan_orbit);
        }
        Err(error) => state.archive.nif_load_error = Some(error),
    }
}
