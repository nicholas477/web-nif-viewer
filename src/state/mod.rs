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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum RecentFileSource {
    Disk,
    #[default]
    Url,
}

impl RecentFileSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disk => "File",
            Self::Url => "URL",
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct RecentFile {
    pub path: String,

    /// File name inside the path, if the path is a zip
    pub file_name: String,
    #[serde(default)]
    pub source: RecentFileSource,
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
    pub recent_source: RecentFileSource,
    pub selected_file: Option<String>,
    pub file_search_query: String,
    pub pending_file: Option<String>,
    pub pending_picker_file: Arc<RwLock<Option<String>>>,
    pub pending_picker_source: Arc<RwLock<Option<String>>>,
    pub pending_picker_recent_source: Arc<RwLock<Option<RecentFileSource>>>,
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
    pub show_keybindings: bool,
    pub capturing_frame_mesh_key: bool,
    pub capturing_mouse_binding: Option<MouseBinding>,
    pub show_resource_permission_prompt: bool,
    pub resource_paths: Vec<String>,
    pub selected_resource: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseBinding {
    Orbit,
    Pan,
    Zoom,
}

#[derive(Clone)]
pub struct KeyBindings {
    pub frame_mesh: KeyCode,
    pub orbit: MouseButton,
    pub pan: MouseButton,
    pub zoom: Option<MouseButton>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            frame_mesh: KeyCode::KeyF,
            orbit: MouseButton::Left,
            pan: MouseButton::Right,
            zoom: None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct KeyBindingsConfig {
    frame_mesh: String,
    orbit: String,
    pan: String,
    zoom: Option<String>,
}

impl KeyBindings {
    pub fn to_config(&self) -> Option<KeyBindingsConfig> {
        Some(KeyBindingsConfig {
            frame_mesh: key_code_name(self.frame_mesh)?.to_string(),
            orbit: mouse_button_name(self.orbit),
            pan: mouse_button_name(self.pan),
            zoom: self.zoom.map(mouse_button_name),
        })
    }

    pub fn from_config(config: KeyBindingsConfig) -> Option<Self> {
        Some(Self {
            frame_mesh: key_code_from_name(&config.frame_mesh)?,
            orbit: mouse_button_from_name(&config.orbit)?,
            pan: mouse_button_from_name(&config.pan)?,
            zoom: match config.zoom {
                Some(button) => Some(mouse_button_from_name(&button)?),
                None => None,
            },
        })
    }
}

fn key_code_name(key: KeyCode) -> Option<&'static str> {
    macro_rules! key_codes {
        ($($key:ident),* $(,)?) => {
            match key {
                $(KeyCode::$key => Some(stringify!($key)),)*
                _ => None,
            }
        };
    }
    key_codes!(
        Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3,
        Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, KeyA, KeyB, KeyC, KeyD, KeyE,
        KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS,
        KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Period, Quote, Semicolon, Slash,
        AltLeft, AltRight, Backspace, CapsLock, ControlLeft, ControlRight, Enter, SuperLeft,
        SuperRight, ShiftLeft, ShiftRight, Space, Tab, Delete, End, Home, Insert, PageDown,
        PageUp, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Numpad0, Numpad1, Numpad2, Numpad3,
        Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDecimal,
        NumpadDivide, NumpadEnter, NumpadMultiply, NumpadSubtract, Escape, PrintScreen,
        ScrollLock, Pause, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15,
        F16, F17, F18, F19, F20, F21, F22, F23, F24
    )
}

fn key_code_from_name(name: &str) -> Option<KeyCode> {
    macro_rules! key_codes {
        ($($key:ident),* $(,)?) => {
            match name {
                $(stringify!($key) => Some(KeyCode::$key),)*
                _ => None,
            }
        };
    }
    key_codes!(
        Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3,
        Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, KeyA, KeyB, KeyC, KeyD, KeyE,
        KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS,
        KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Period, Quote, Semicolon, Slash,
        AltLeft, AltRight, Backspace, CapsLock, ControlLeft, ControlRight, Enter, SuperLeft,
        SuperRight, ShiftLeft, ShiftRight, Space, Tab, Delete, End, Home, Insert, PageDown,
        PageUp, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Numpad0, Numpad1, Numpad2, Numpad3,
        Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDecimal,
        NumpadDivide, NumpadEnter, NumpadMultiply, NumpadSubtract, Escape, PrintScreen,
        ScrollLock, Pause, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15,
        F16, F17, F18, F19, F20, F21, F22, F23, F24
    )
}

fn mouse_button_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "Left".to_string(),
        MouseButton::Right => "Right".to_string(),
        MouseButton::Middle => "Middle".to_string(),
        MouseButton::Back => "Back".to_string(),
        MouseButton::Forward => "Forward".to_string(),
        MouseButton::Other(button) => format!("Other:{button}"),
    }
}

fn mouse_button_from_name(name: &str) -> Option<MouseButton> {
    match name {
        "Left" => Some(MouseButton::Left),
        "Right" => Some(MouseButton::Right),
        "Middle" => Some(MouseButton::Middle),
        "Back" => Some(MouseButton::Back),
        "Forward" => Some(MouseButton::Forward),
        _ => name
            .strip_prefix("Other:")?
            .parse()
            .ok()
            .map(MouseButton::Other),
    }
}

#[derive(Resource, Default, Clone)]
pub struct UIState {
    pub archive: ArchiveState,
    pub inspector: InspectorState,
    pub top_panel: TopPanelState,
    pub key_bindings: KeyBindings,
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
    pub async fn resource_read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        for fs in self.resource_file_systems.read().unwrap().iter() {
            if let Some(data) = fs.read(path, false).await {
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
    fn read<'a>(
        &'a self,
        path: &'a str,
        absolute_paths: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<ArcSlice<[u8]>>> + 'a>> {
        Box::pin(async move {
            if let Some(fs) = self.file_system.read().unwrap().as_ref()
                && let Some(data) = fs.read(path, absolute_paths).await
            {
                return Some(data);
            }
            self.resource_read(path).await
        })
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
