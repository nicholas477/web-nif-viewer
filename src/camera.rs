use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    transform::TransformSystems,
    window::PrimaryWindow,
};

#[derive(Component)]
pub struct PanOrbitCamera {
    pub target_focus: Vec3,
    pub target_radius: f32,
    yaw: f32,
    pitch: f32,
    initialized: bool,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        Self {
            target_focus: Vec3::ZERO,
            target_radius: 1.0,
            yaw: 0.0,
            pitch: 0.0,
            initialized: false,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_camera.before(TransformSystems::Propagate),
        );
    }
}

pub fn spawn(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(300.0, 300.0, 500.0).looking_at(Vec3::ZERO, Vec3::Z),
        PanOrbitCamera {
            ..Default::default()
        },
    ));
}

fn update_camera(
    mut cameras: Query<(&mut Transform, &Projection, &mut PanOrbitCamera)>,
    window: Single<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
) {
    let mouse_delta = mouse_motion.read().map(|event| event.delta).sum::<Vec2>();
    let (scroll_line, scroll_pixel) = mouse_wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => (event.y, 0.0),
            MouseScrollUnit::Pixel => (0.0, event.y * 0.005),
        })
        .fold((0.0, 0.0), |total, delta| {
            (total.0 + delta.0, total.1 + delta.1)
        });
    let window = window.into_inner();

    for (mut transform, projection, mut camera) in &mut cameras {
        if !camera.initialized {
            let offset = transform.translation - camera.target_focus;
            camera.target_radius = offset.length().max(0.05);
            camera.yaw = offset.x.atan2(offset.y);
            camera.pitch = (offset.z / camera.target_radius).asin();
            camera.initialized = true;
        }

        if buttons.pressed(MouseButton::Left) {
            camera.yaw += mouse_delta.x / window.width() * std::f32::consts::TAU;
            camera.pitch = (camera.pitch + mouse_delta.y / window.height() * std::f32::consts::PI)
                .clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
        }

        if buttons.pressed(MouseButton::Right) {
            let pan_scale = match projection {
                Projection::Perspective(perspective) => {
                    perspective.fov * camera.target_radius / window.height()
                }
                Projection::Orthographic(projection) => projection.area.height() / window.height(),
                Projection::Custom(_) => 0.0,
            };
            camera.target_focus += transform.right() * -mouse_delta.x * pan_scale;
            camera.target_focus += transform.up() * mouse_delta.y * pan_scale;
        }

        if (scroll_line + scroll_pixel).abs() > 0.0 {
            let line_delta = -scroll_line * camera.target_radius * 0.2;
            let pixel_delta = -scroll_pixel * camera.target_radius * 0.2;
            camera.target_radius = (camera.target_radius + line_delta + pixel_delta).max(0.05);
        }

        let horizontal_radius = camera.target_radius * camera.pitch.cos();
        let offset = Vec3::new(
            horizontal_radius * camera.yaw.sin(),
            horizontal_radius * camera.yaw.cos(),
            camera.target_radius * camera.pitch.sin(),
        );
        transform.translation = camera.target_focus + offset;
        transform.look_at(camera.target_focus, Vec3::Z);
    }
}

pub fn focus_loaded_meshes(
    meshes: &Assets<Mesh>,
    projection: &Projection,
    window: &Window,
    pan_orbit_camera: &mut PanOrbitCamera,
) {
    crate::nif::center_camera_on_mesh(meshes, projection, window, pan_orbit_camera);
}
