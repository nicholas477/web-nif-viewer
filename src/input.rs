use crate::camera;
use bevy::prelude::*;

/// Handles viewer keyboard shortcuts.
pub fn input_system(
    camera_query: Single<(&Projection, &mut camera::PanOrbitCamera)>,
    window: Single<&Window>,
    meshes: Res<Assets<Mesh>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<crate::UIState>,
) -> Result {
    if !state.top_panel.capturing_frame_mesh_key
        && keys.just_pressed(state.key_bindings.frame_mesh)
    {
        let (projection, mut pan_orbit) = camera_query.into_inner();
        camera::focus_loaded_meshes(&meshes, projection, window.into_inner(), &mut pan_orbit);
    }

    Ok(())
}
