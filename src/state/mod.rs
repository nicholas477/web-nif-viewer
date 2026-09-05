use std::sync::{Arc, RwLock};

use bevy::prelude::*;

// URL query state, only on wasm
#[cfg(target_arch = "wasm32")]
pub mod query;

#[derive(Clone, Default, Debug)]
pub struct ArchiveLoadStatus {
    pub phase: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Default, Debug)]
pub struct UploadStatus {
    pub phase: Option<String>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct RecentFile {
    pub zip_url: String,
    pub file_name: String,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct RecentFiles {
    pub files : Vec<RecentFile>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ShadingMode {
    Lit,
    #[default]
    Unlit,
    Normals,
}

impl ShadingMode {
    /// Returns the label displayed for this shading mode in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lit => "Lit",
            Self::Unlit => "Unlit",
            Self::Normals => "Normals",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum DisplayMode {
    #[default]
    Off,
    On,
    Only,
}

impl DisplayMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::On, Self::Only];

    /// Returns the label displayed for this display mode in the UI.
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

#[derive(Clone)]
pub struct NifObjectInfo {
    pub type_name: String,
    pub fields: String,
    pub children: Vec<usize>,
}

#[derive(Default, Clone)]
pub struct ArchiveState {
    pub show_zip_popup: bool,
    pub zip_url_input: String,
    pub file_system: crate::file::FS,
    pub selected_file: Option<String>,
    pub pending_file: Option<String>,
    pub archive_load_status: Arc<RwLock<ArchiveLoadStatus>>,
    pub nif_load_error: Option<String>,
    pub upload_status: Arc<RwLock<UploadStatus>>,
}

#[derive(Default, Clone)]
pub struct InspectorState {
    pub nif_objects: Vec<NifObjectInfo>,
    pub nif_roots: Vec<usize>,
    pub selected_node: Option<usize>,
    pub triangle_count: usize,
}

// Generic helper function to check if a value is the default
// fn is_default<T: Default + PartialEq>(value: &T) -> bool {
//     value == &T::default()
// }

#[derive(Default, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ViewState {
    pub shading_mode: ShadingMode,
    pub vertex_colors: DisplayMode,
    pub collision: DisplayMode,
    pub wireframe: bool,
}

#[derive(Resource, Default, Clone)]
pub struct UIState {
    pub archive: ArchiveState,
    pub inspector: InspectorState,
    pub view: ViewState,
}

impl From<&UIState> for ViewOptions {
    /// Creates an immutable renderer-facing snapshot of the selected view controls.
    fn from(state: &UIState) -> Self {
        Self {
            shading_mode: state.view.shading_mode,
            vertex_colors: state.view.vertex_colors,
            collision: state.view.collision,
            wireframe: state.view.wireframe,
        }
    }
}
