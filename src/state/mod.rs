use std::sync::{Arc, RwLock};

use arc_slice::ArcSlice;
use bevy::prelude::*;
use tes3::nif::NiType;

pub use crate::file::Filesystem;

// URL query state, only on wasm
#[cfg(target_arch = "wasm32")]
pub mod query;

pub mod file;
pub mod recent_files;

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
    pub files: Vec<RecentFile>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum DisplayMode {
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
    pub object: NiType,
    pub children: Vec<usize>,
}

#[derive(Clone, Default)]
pub struct ArchiveState {
    pub show_zip_popup: bool,
    pub zip_url_input: String,
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

#[derive(Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ViewState {
    pub shading_mode: ShadingMode,
    pub vertex_colors: DisplayMode,
    pub collision: DisplayMode,
    pub wireframe: bool,
}

impl std::default::Default for ViewState {
    fn default() -> Self {
        Self {
            shading_mode: ShadingMode::Unlit,
            vertex_colors: DisplayMode::On,
            collision: DisplayMode::Off,
            wireframe: false,
        }
    }
}

#[derive(Clone, Default)]
pub struct TopPanelState {
    pub show_nif_popup: bool,
    pub show_resources: bool,
    pub resource_paths: Vec<String>,
    pub selected_resource: Option<usize>,
}

#[derive(Resource, Default, Clone)]
pub struct UIState {
    pub archive: ArchiveState,
    pub inspector: InspectorState,
    pub top_panel: TopPanelState,
    pub view: ViewState,
}

#[derive(Resource, Clone)]
pub struct FSState {
    /// Filesystems probably loaded by the user, including zips and other external resources.
    pub file_system: Arc<RwLock<Option<Box<dyn crate::state::file::Filesystem>>>>,

    /// Filesystems imported from the resources settings.
    /// These always have lower priority than the user-loaded filesystems.
    pub resource_file_systems: Arc<RwLock<Vec<crate::state::file::RealFS>>>,
}

impl FSState {
    /// Read from the resource file systems
    pub fn resource_read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        for fs in self.resource_file_systems.read().unwrap().iter() {
            if let Some(data) = fs.read(path, false) {
                return Some(data);
            }
        }
        None
    }
}

impl Default for FSState {
    fn default() -> Self {
        Self {
            file_system: Arc::new(RwLock::new(None)),
            resource_file_systems: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl file::Filesystem for FSState {
    fn read(&self, path: &str, absolute_paths: bool) -> Option<ArcSlice<[u8]>> {
        if let Some(fs) = self.file_system.read().unwrap().as_ref()
            && let Some(data) = fs.read(path, absolute_paths) {
                return Some(data);
            }
        self.resource_read(path)
    }

    fn absolute_paths(&self) -> Vec<String> {
        let mut hash_set: std::collections::HashSet<_> =
            if let Some(fs) = self.file_system.read().unwrap().as_ref() {
                fs.absolute_paths().into_iter().collect()
            } else {
                std::collections::HashSet::new()
            };

        hash_set.extend(
            self.resource_file_systems
                .read()
                .unwrap()
                .iter()
                .flat_map(|fs| fs.absolute_paths()),
        );
        hash_set.into_iter().collect()
    }
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
