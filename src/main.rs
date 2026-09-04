use std::sync::{Arc, RwLock};

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    reflect::TypePath,
    render::render_resource::{AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    shader::ShaderRef,
};
use bevy_egui::{
    EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use wgpu_types::BlendState;

mod file;
mod nif;
mod input;
mod ui;

const PHONG_SHADER_PATH: &str = "shaders/phong.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(PhongMaterialKey)]
pub struct PhongMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(3)]
    pub settings: Vec4,
    pub alpha_mode: AlphaMode,
    pub cull_mode: Option<Face>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PhongMaterialKey {
    cull_mode: Option<Face>,
}

impl From<&PhongMaterial> for PhongMaterialKey {
    fn from(material: &PhongMaterial) -> Self {
        Self {
            cull_mode: material.cull_mode,
        }
    }
}

impl Material for PhongMaterial {
    fn fragment_shader() -> ShaderRef {
        PHONG_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct ArchiveLoadStatus {
    pub phase: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct UploadStatus {
    pub phase: Option<String>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Clone)]
pub struct RecentFile {
    pub zip_url: String,
    pub file_name: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShadingMode {
    Lit,
    #[default]
    Unlit,
    Normals,
}

impl ShadingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lit => "Lit",
            Self::Unlit => "Unlit",
            Self::Normals => "Normals",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Off,
    On,
    Only,
}

impl DisplayMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::On, Self::Only];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::On => "On",
            Self::Only => "Only",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewOptions {
    pub shading_mode: ShadingMode,
    pub vertex_colors: DisplayMode,
    pub collision: DisplayMode,
    pub wireframe: bool,
}

impl From<&UIState> for ViewOptions {
    fn from(state: &UIState) -> Self {
        Self {
            shading_mode: state.shading_mode,
            vertex_colors: state.vertex_colors,
            collision: state.collision,
            wireframe: state.wireframe,
        }
    }
}

#[derive(Clone)]
pub struct NifObjectInfo {
    pub type_name: String,
    pub fields: String,
    pub children: Vec<usize>,
}

#[derive(Resource, Default)]
pub struct UIState {
    pub show_zip_popup: bool,
    pub zip_url_input: String,
    pub file_system: file::FS,
    pub selected_file: Option<String>,
    pub pending_file: Option<String>,
    pub archive_load_status: Arc<RwLock<ArchiveLoadStatus>>,
    pub nif_load_error: Option<String>,
    pub upload_status: Arc<RwLock<UploadStatus>>,
    pub nif_objects: Vec<NifObjectInfo>,
    pub nif_roots: Vec<usize>,
    pub triangle_count: usize,
    pub shading_mode: ShadingMode,
    pub vertex_colors: DisplayMode,
    pub collision: DisplayMode,
    pub wireframe: bool,
}

fn main() {
    App::new()
        .add_plugins(PanOrbitCameraPlugin)
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

fn setup_system(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // 3d grid
    commands.spawn((
        // You need to spawn an entity with this component
        InfiniteGrid,
        // Optional component you can use to configure the grid
        InfiniteGridSettings{
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

    // World camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(300.0, 300.0, 500.0).looking_at(Vec3::ZERO, Vec3::Z),
        PanOrbitCamera {
            axis: [Vec3::X, Vec3::Z, -Vec3::Y],
            allow_upside_down: true,
            orbit_smoothness: 0.0,
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
