use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    prelude::*,
};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext};
use wgpu_types::BlendState;

mod camera;
mod file;
mod input;
mod material;
mod nif;
mod state;
mod ui;

pub use material::PhongMaterial;
pub use state::*;

fn main() {
    App::new()
        .add_plugins(camera::CameraPlugin)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    prevent_default_event_handling: false,
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
        .add_plugins(MaterialPlugin::<PhongMaterial>::default())
        .add_systems(Startup, (setup_system, ui::initialize_from_url))
        .init_resource::<UIState>()
        .add_systems(EguiPrimaryContextPass, ui::ui_system)
        .add_systems(Update, input::input_system)
        .run();
}

fn setup_system(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // 3d grid
    commands.spawn((
        // You need to spawn an entity with this component
        InfiniteGrid,
        // Optional component you can use to configure the grid
        InfiniteGridSettings {
            fadeout_distance: 100.0 * 100.0, // 100 meters
            scale: 0.01, // Scale down by 0.01 to convert from centimeters to meters
            ..Default::default()
        },
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.8, -0.6)),
    ));
    commands.spawn((
        PointLight {
            intensity: 250_000.0,
            range: 2_000.0,
            color: Color::srgb(0.75, 0.85, 1.0),
            ..default()
        },
        Transform::from_xyz(-400.0, 300.0, 300.0),
    ));
    commands.spawn((
        PointLight {
            intensity: 150_000.0,
            range: 2_000.0,
            color: Color::srgb(1.0, 0.78, 0.62),
            ..default()
        },
        Transform::from_xyz(350.0, -250.0, 150.0),
    ));

    camera::spawn(&mut commands);

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
