use crate::nif;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn input_system(
    camera_query: Single<(&Projection, &mut PanOrbitCamera)>,
    window: Single<&Window>,
    meshes: Res<Assets<Mesh>>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    if keys.just_pressed(KeyCode::KeyF) {
        let (projection, mut pan_orbit) = camera_query.into_inner();
        nif::center_camera_on_mesh(&meshes, projection, window.into_inner(), &mut pan_orbit);
    }

    Ok(())
}
