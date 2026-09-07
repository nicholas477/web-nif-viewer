use bevy::{
    input::{ButtonState, mouse::MouseButtonInput},
    picking::events::Click,
    prelude::*,
};

use crate::ui::UiSystemParams;

/// Selects the clicked NIF shape in the inspector and highlights its wireframe.
pub fn select_mesh(
    event: On<Pointer<Click>>,
    meshes: Query<&crate::nif::LoadedNifMesh>,
    mut wireframes: Query<
        (
            &crate::nif::LoadedNifWireframe,
            &MeshMaterial3d<crate::PhongMaterial>,
            &mut Visibility,
        ),
        Without<crate::nif::LoadedNifMesh>,
    >,
    mut materials: ResMut<Assets<crate::PhongMaterial>>,
    mut state: ResMut<crate::state::UIState>,
) {
    let Ok(mesh) = meshes.get(event.entity) else {
        return;
    };
    let Some(selected_node) = mesh.nif_node_index else {
        return;
    };

    state.inspector.selected_node = Some(selected_node);
    set_wireframe_highlight(&mut wireframes, &mut materials, &state, Some(selected_node));
}

/// Clears the mesh selection after a left click that misses every mesh in the 3D viewport.
pub fn clear_selection_on_viewport_click(
    mut mouse_buttons: MessageReader<MouseButtonInput>,
    mut pointer_clicks: MessageReader<Pointer<Click>>,
    mut params: UiSystemParams,
) {
    if !mouse_buttons.read().any(|event| {
        event.button == MouseButton::Left && event.state == ButtonState::Pressed
    }) {
        return;
    }
    let Some(cursor_position) = params.window.cursor_position() else {
        return;
    };
    let Some(viewport) = params.camera.viewport.as_ref() else {
        return;
    };
    let scale_factor = params.window.scale_factor();
    let viewport_min = viewport.physical_position.as_vec2() / scale_factor;
    let viewport_max = viewport_min + viewport.physical_size.as_vec2() / scale_factor;
    if cursor_position.cmplt(viewport_min).any() || cursor_position.cmpgt(viewport_max).any() {
        return;
    }

    let clicked_mesh = pointer_clicks
        .read()
        .any(|event| params.loaded_meshes.contains(event.entity));
    if clicked_mesh {
        return;
    }
    //state.inspector.selected_node = None;
    set_wireframe_highlight(&mut params.loaded_wireframes, &mut params.materials, &params.state, None);
}

fn set_wireframe_highlight(
    wireframes: &mut Query<
        (
            &crate::nif::LoadedNifWireframe,
            &MeshMaterial3d<crate::PhongMaterial>,
            &mut Visibility,
        ),
        Without<crate::nif::LoadedNifMesh>,
    >,
    materials: &mut Assets<crate::PhongMaterial>,
    state: &crate::state::UIState,
    selected_node: Option<usize>,
) {
    for (wireframe, material_handle, mut visibility) in wireframes.iter_mut() {
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.color = if Some(wireframe.nif_node_index) == selected_node {
                LinearRgba::new(0.0, 1.0, 0.0, 1.0)
            } else {
                LinearRgba::BLACK
            };
        }
        if Some(wireframe.nif_node_index) == selected_node {
            *visibility = bevy::camera::visibility::Visibility::Visible;
        }
        else {
            *visibility = if state.view.wireframe {
                bevy::camera::visibility::Visibility::Visible
            } else {
                bevy::camera::visibility::Visibility::Hidden
            };
        }
    }
}
