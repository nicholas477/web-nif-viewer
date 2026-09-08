use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;
use bevy_egui::egui;
use wasm_bindgen::{JsCast, prelude::*};

static RESOURCE_PERMISSIONS_READY: AtomicBool = AtomicBool::new(false);
static SHOW_PERMISSION_PROMPT: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen(inline_js = "
let resourceDirectories = [];
let resourceFilesystemUpdated = false;
const resourceDatabase = () => new Promise((resolve, reject) => {
  const request = indexedDB.open('nif-viewer-settings', 1);
  request.onupgradeneeded = () => request.result.createObjectStore('settings');
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
async function saveResourceDirectories() {
  const database = await resourceDatabase();
  const transaction = database.transaction('settings', 'readwrite');
  transaction.objectStore('settings').put(resourceDirectories, 'resourceDirectories');
}
export async function load_resource_directories() {
  const database = await resourceDatabase();
  const transaction = database.transaction('settings', 'readonly');
  const request = transaction.objectStore('settings').get('resourceDirectories');
    await new Promise((resolve, reject) => {
        request.onsuccess = () => {
            resourceDirectories = request.result || [];
            resolve();
        };
        request.onerror = () => reject(request.error);
    });
}
export async function log_resource_directory_permissions() {
        let needsPermission = false;
    for (const directory of resourceDirectories) {
        try {
            const permission = await directory.queryPermission({ mode: 'read' });
                        needsPermission ||= permission !== 'granted';
            console.info(`Resource folder ${directory.name}: ${permission}`);
        } catch (error) {
                        needsPermission = true;
            console.warn(`Could not check access to resource folder ${directory.name}:`, error);
        }
    }
        return needsPermission;
}
export function request_resource_directory_permissions() {
    Promise.all(resourceDirectories.map(directory => directory.requestPermission({ mode: 'read' })
            .then(permission => console.info(`Resource folder ${directory.name}: ${permission}`))
            .catch(error => console.warn(`Could not request access to resource folder ${directory.name}:`, error))))
        .then(() => { resourceFilesystemUpdated = true; });
}
export function take_resource_filesystem_update() {
    const updated = resourceFilesystemUpdated;
    resourceFilesystemUpdated = false;
    return updated;
}
export function resource_directory_handles() { return resourceDirectories; }
export function pick_resource_directory() {
  window.showDirectoryPicker().then(async handle => {
    resourceDirectories.push(handle);
    await saveResourceDirectories();
        resourceFilesystemUpdated = true;
  }).catch(() => {});
}
export function remove_resource_directory(index) {
  resourceDirectories.splice(index, 1);
  saveResourceDirectories().catch(() => {});
    resourceFilesystemUpdated = true;
}
export function move_resource_directory(index, destination) {
  [resourceDirectories[index], resourceDirectories[destination]] = [resourceDirectories[destination], resourceDirectories[index]];
  saveResourceDirectories().catch(() => {});
    resourceFilesystemUpdated = true;
}
export function load_key_bindings() { return localStorage.getItem('keyBindings'); }
export function save_key_bindings(bindings) { localStorage.setItem('keyBindings', bindings); }
")]
extern "C" {
    async fn load_resource_directories();
    async fn log_resource_directory_permissions() -> bool;
    fn request_resource_directory_permissions();
    fn take_resource_filesystem_update() -> bool;
    fn resource_directory_handles() -> js_sys::Array;
    fn pick_resource_directory();
    fn remove_resource_directory(index: u32);
    fn move_resource_directory(index: u32, destination: u32);
    fn load_key_bindings() -> Option<String>;
    fn save_key_bindings(bindings: &str);
}

pub fn initialize_resources(
    mut state: bevy::prelude::ResMut<crate::UIState>,
    _fsstate: bevy::prelude::ResMut<crate::state::FSState>,
) {
    if let Some(bindings) = load_key_bindings()
        .and_then(|bindings| serde_json::from_str(&bindings).ok())
        .and_then(crate::KeyBindings::from_config)
    {
        state.key_bindings = bindings;
    }
    RESOURCE_PERMISSIONS_READY.store(false, Ordering::Release);
    wasm_bindgen_futures::spawn_local(async {
        load_resource_directories().await;
        let needs_permission = log_resource_directory_permissions().await;
        bevy::log::info!("Saved resource folders require permission: {needs_permission}");
        RESOURCE_PERMISSIONS_READY.store(true, Ordering::Release);
        SHOW_PERMISSION_PROMPT.store(needs_permission, Ordering::Release);
        bevy::log::info!(
            "Restored resource folders from IndexedDB; permission may require user action"
        );
    });
}

pub fn save_keybindings(bindings: &crate::KeyBindings) {
    let Some(bindings) = bindings.to_config() else {
        bevy::log::warn!("Could not save unsupported keyboard binding.");
        return;
    };
    let Ok(bindings) = serde_json::to_string(&bindings) else {
        bevy::log::warn!("Could not serialize key bindings.");
        return;
    };
    save_key_bindings(&bindings);
}

/// Starts the default archive only after persisted resource permissions are resolved.
pub fn initialize_default_mesh_after_resources(
    mut initialized: Local<bool>,
    state: ResMut<crate::UIState>,
    fsstate: ResMut<crate::state::FSState>,
) {
    if *initialized || !RESOURCE_PERMISSIONS_READY.load(Ordering::Acquire) {
        return;
    }

    *initialized = true;
    crate::ui::file::initialize_default_mesh(state, fsstate);
}

fn refresh_resources(state: &mut crate::UIState, fsstate: &mut crate::state::FSState) {
    let handles = resource_directory_handles();
    let paths = handles
        .iter()
        .filter_map(|handle| handle.dyn_into::<web_sys::FileSystemDirectoryHandle>().ok())
        .collect::<Vec<_>>();
    let names = paths
        .iter()
        .map(|handle| {
            handle
                .clone()
                .unchecked_into::<web_sys::FileSystemHandle>()
                .name()
        })
        .collect::<Vec<_>>();
    if names == state.top_panel.resource_paths {
        return;
    }
    state.top_panel.resource_paths = names;

    let mut write = fsstate.resource_file_systems.write().unwrap();

    write.clear();
    for handle in handles {
        write.push(crate::state::file::RealFS::new(handle.into()));
    }
    state.archive.pending_file = state.archive.selected_file.clone();
    bevy::log::info!("Resource folders changed; reloading textures for the active NIF");
}

pub fn draw_resources(ctx: &bevy_egui::egui::Context, params: &mut crate::ui::UiSystemParams) {
    refresh_resources(&mut params.state, &mut params.fsstate);
    if take_resource_filesystem_update() {
        params.state.archive.pending_file = params.state.archive.selected_file.clone();
        bevy::log::info!(
            "Resource folder permissions changed; reloading textures for the active NIF"
        );
    }
    if SHOW_PERMISSION_PROMPT.swap(false, Ordering::AcqRel) {
        params.state.top_panel.show_resource_permission_prompt = true;
    }
    draw_startup_permission_prompt(
        ctx,
        &mut params.state.top_panel.show_resource_permission_prompt,
    );
    if !params.state.top_panel.show_resources {
        return;
    }

    let mut open = params.state.top_panel.show_resources;
    let mut selected = params.state.top_panel.selected_resource;
    let paths = params.state.top_panel.resource_paths.clone();
    bevy_egui::egui::Window::new("Resources")
        .open(&mut open)
        .default_width(520.0)
        .default_height(400.0)
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(
                "Resource folders are searched in this order when an archive references a texture.",
            );
            ui.separator();

            if !paths.is_empty() {
                egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(300.0).show(ui, |ui| {
                    for (index, path) in paths.iter().enumerate() {
                        if ui
                            .selectable_label(
                                selected == Some(index),
                                format!("{} - {path}", index + 1).to_string(),
                            )
                            .clicked()
                        {
                            selected = Some(index);
                        }
                    }
                });
            }
            else
            {
                ui.label("No resource folders configured.");
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add folder").clicked() {
                    pick_resource_directory();
                }
                if ui.button("Grant access").clicked() {
                    request_resource_directory_permissions();
                }
                if ui
                    .add_enabled(
                        selected.is_some(),
                        bevy_egui::egui::Button::new("Remove folder"),
                    )
                    .clicked()
                    && let Some(index) = selected
                {
                    remove_resource_directory(index as u32);
                    selected = None;
                }
                if ui
                    .add_enabled(
                        selected.is_some_and(|index| index > 0),
                        bevy_egui::egui::Button::new("Move up"),
                    )
                    .clicked()
                    && let Some(index) = selected
                {
                    move_resource_directory(index as u32, (index - 1) as u32);
                    selected = Some(index - 1);
                }
                if ui
                    .add_enabled(
                        selected.is_some_and(|index| index + 1 < paths.len()),
                        bevy_egui::egui::Button::new("Move down"),
                    )
                    .clicked()
                    && let Some(index) = selected
                {
                    move_resource_directory(index as u32, (index + 1) as u32);
                    selected = Some(index + 1);
                }
            });
        });
    params.state.top_panel.show_resources = open;
    params.state.top_panel.selected_resource = selected;
}

/// Prompts the user to restore read access to resource folders saved by the browser.
fn draw_startup_permission_prompt(ctx: &bevy_egui::egui::Context, open: &mut bool) {
    if !*open {
        return;
    }

    bevy_egui::egui::Window::new("Resource Folder Access")
        .collapsible(false)
        .resizable(false)
        .anchor(
            bevy_egui::egui::Align2::CENTER_CENTER,
            bevy_egui::egui::Vec2::ZERO,
        )
        .open(open)
        .show(ctx, |ui| {
            ui.label("Allow access to saved resource folders so their textures can be loaded.");
            ui.horizontal(|ui| {
                if ui.button("Grant access").clicked() {
                    request_resource_directory_permissions();
                    ui.close();
                }
                if ui.button("Not now").clicked() {
                    ui.close();
                }
            });
        });
}
