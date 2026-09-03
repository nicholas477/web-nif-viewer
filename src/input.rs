use crate::nif;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use egui::{LayerId, Ui, UiBuilder};
use tes3::nif::{
    NiStream, NiTexturingProperty, NiTriShape, NiTriShapeData, TextureMap, TextureSource,
};

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
