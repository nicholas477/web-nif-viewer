use std::{collections::HashMap, sync::{Arc, RwLock}};

use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers}, dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings}, prelude::*, window::{PrimaryWindow, WindowMode},
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use egui::{LayerId, Ui, UiBuilder};
use wgpu_types::BlendState;

mod file;
mod ui;

#[derive(Resource, Default)]
pub struct MenuState {
    pub show_zip_popup: bool,
    pub zip_url_input: String,
    pub file_system: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pub selected_file: Option<String>,
}

#[derive(Component)]
pub struct LoadedNifMesh;

fn main() {
    App::new()
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    prevent_default_event_handling: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            InfiniteGridPlugin,
        ))
        .add_plugins(EguiPlugin {
            ..Default::default()
        }) // Hook egui into Bevy's loop
        .add_systems(Startup, setup_system)
        .init_resource::<MenuState>()
        .add_systems(EguiPrimaryContextPass, ui::ui_example_system)
        .run();
}

// Set up the example entities for the scene. The only important thing is a camera which
// renders directly to the window.
fn setup_system(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // 3d grid
    commands.spawn((
        // You need to spawn an entity with this component
        InfiniteGrid,
        // Optional component you can use to configure the grid
        InfiniteGridSettings::default(),
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));

    // World camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Z),
        PanOrbitCamera{
            axis: [Vec3::X, Vec3::Z, -Vec3::Y],
            ..Default::default()
        },
    ));

    // Egui camera.
    commands.spawn((
        // The `PrimaryEguiContext` component requires everything needed to render a primary context.
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
        Camera {
            order: 1,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
}