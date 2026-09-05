use crate::camera;
use bevy::prelude::*;
use bevy_egui::egui;

pub fn input_system(
    camera_query: Single<(&Projection, &mut camera::PanOrbitCamera)>,
    window: Single<&Window>,
    meshes: Res<Assets<Mesh>>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    if keys.just_pressed(KeyCode::KeyF) {
        let (projection, mut pan_orbit) = camera_query.into_inner();
        camera::focus_loaded_meshes(&meshes, projection, window.into_inner(), &mut pan_orbit);
    }

    Ok(())
}
