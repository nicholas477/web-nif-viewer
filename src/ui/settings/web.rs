use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(inline_js = "
let resourceDirectories = [];
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
  request.onsuccess = () => { resourceDirectories = request.result || []; };
}
export function resource_directory_handles() { return resourceDirectories; }
export function pick_resource_directory() {
  window.showDirectoryPicker().then(async handle => {
    resourceDirectories.push(handle);
    await saveResourceDirectories();
  }).catch(() => {});
}
export function remove_resource_directory(index) {
  resourceDirectories.splice(index, 1);
  saveResourceDirectories().catch(() => {});
}
export function move_resource_directory(index, destination) {
  [resourceDirectories[index], resourceDirectories[destination]] = [resourceDirectories[destination], resourceDirectories[index]];
  saveResourceDirectories().catch(() => {});
}
")]
extern "C" {
    fn load_resource_directories();
    fn resource_directory_handles() -> js_sys::Array;
    fn pick_resource_directory();
    fn remove_resource_directory(index: u32);
    fn move_resource_directory(index: u32, destination: u32);
}

pub fn initialize_resources(
    _state: bevy::prelude::ResMut<crate::UIState>,
    _fsstate: bevy::prelude::ResMut<crate::state::FSState>,
) {
    load_resource_directories();
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

    // fsstate.set_resource_file_systems(
    //     paths
    //         .into_iter()
    //         .map(|handle| Box::new(crate::state::file::RealFS::new(handle)) as Box<_>)
    //         .collect(),
    // );
}

pub fn draw_resources(ctx: &bevy_egui::egui::Context, params: &mut crate::ui::UiSystemParams) {
    refresh_resources(&mut params.state, &mut params.fsstate);
    if !params.state.top_panel.show_resources {
        return;
    }

    let mut open = params.state.top_panel.show_resources;
    let mut selected = params.state.top_panel.selected_resource;
    let paths = params.state.top_panel.resource_paths.clone();
    bevy_egui::egui::Window::new("Resources")
        .open(&mut open)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label(
                "Resource folders are searched in this order when an archive references a texture.",
            );
            ui.separator();
            for (index, path) in paths.iter().enumerate() {
                if ui.selectable_label(selected == Some(index), path).clicked() {
                    selected = Some(index);
                }
            }
            if paths.is_empty() {
                ui.label("No resource folders configured.");
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add folder").clicked() {
                    pick_resource_directory();
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
