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
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use egui::{LayerId, Ui, UiBuilder};
use tes3::nif::{NiStream, NiTexturingProperty, NiTriShape, NiTriShapeData, TextureMap, TextureSource};

use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

const ZIP_QUERY_PARAMETER: &str = "zip";
const FILE_QUERY_PARAMETER: &str = "file";

pub fn initialize_from_url(mut state: ResMut<crate::MenuState>) {
    let Some((zip_url, selected_file)) = query_state() else {
        return;
    };

    state.zip_url_input = zip_url.clone();
    state.pending_file = selected_file;

    let file_system = state.file_system.clone();
    spawn_local(async move {
        match crate::file::fetch_and_unzip(&zip_url).await {
            Ok(files) => {
                bevy::log::info!("Zip fetched and parsed successfully.");
                file_system.write().unwrap().extend(files);
            }
            Err(error) => bevy::log::error!("Error unzipping asset: {:?}", error),
        }
    });
}

// This function runs every frame. Therefore, updating the viewport after drawing the gui.
// With a resource which stores the dimensions of the panels, the update of the Viewport can
// be done in another system.
pub fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loaded_meshes: Query<Entity, With<crate::LoadedNifMesh>>,
    mut state: ResMut<crate::MenuState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    let file_names = state
        .file_system
        .read()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    if let Some(pending_file) = state.pending_file.clone()
        && file_names.iter().any(|file_name| file_name == &pending_file)
    {
        state.pending_file = None;
        state.selected_file = Some(pending_file.clone());
        load_nif(
            &pending_file,
            &state.file_system,
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &loaded_meshes,
        );
    }

    let left_panel = egui::Panel::left("left_panel")
        .resizable(false)
        .show(&mut viewport_ui, |ui| {
            draw_file_selector(ui, &file_names, &mut state.selected_file)
        });
    let mut left = left_panel.response.rect.width(); // height is ignored, as the panel has a height of 100% of the screen

    if let Some(file_name) = left_panel.inner {
        update_query(&state.zip_url_input, Some(&file_name));
        load_nif(
            &file_name,
            &state.file_system,
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            &loaded_meshes,
        );
    }

    let mut top = egui::Panel::top("top_panel")
        .resizable(false)
        .show(&mut viewport_ui, |ui| {
            if ui.button("Open File").clicked() {
                state.show_zip_popup = true;
            }
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen

    left *= window.scale_factor();
    top *= window.scale_factor();

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height()) - pos;

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });

    if state.show_zip_popup {
        draw_zip_popup(contexts, state)?;
    }

    Ok(())
}

fn draw_file_selector(
    ui: &mut Ui,
    file_names: &[String],
    selected_file: &mut Option<String>,
) -> Option<String> {
    ui.heading("Files");
    ui.separator();

    if file_names.is_empty() {
        ui.label("No files loaded");
        return None;
    }

    let mut sorted_file_names = file_names.to_vec();
    sorted_file_names.sort_unstable();
    let mut clicked_file = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for file_name in sorted_file_names {
            let is_selected = selected_file.as_deref() == Some(file_name.as_str());

            if ui.selectable_label(is_selected, &file_name).clicked() {
                *selected_file = Some(file_name.clone());
                clicked_file = Some(file_name);
            }
        }
    });

    clicked_file
}

fn load_nif(
    file_name: &str,
    file_system: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    loaded_meshes: &Query<Entity, With<crate::LoadedNifMesh>>,
) {
    bevy::log::info!("Loading NIF file: {file_name}");
    
    let file_bytes = {
        let file_system = file_system.read().unwrap();
        file_system.get(file_name).cloned()
    };

    let Some(file_bytes) = file_bytes else {
        bevy::log::error!("Selected file is no longer in the file system: {file_name}");
        return;
    };

    if !file_name.to_ascii_lowercase().ends_with(".nif") {
        return;
    }

    let Ok(stream) = NiStream::from_bytes(&file_bytes) else {
        bevy::log::error!("Could not parse NIF file: {file_name}");
        return;
    };

    for entity in loaded_meshes.iter() {
        commands.entity(entity).despawn();
    }

    let mut shape_count = 0;
    for shape in stream.objects_of_type::<NiTriShape>() {
        let Some(data) = stream.get_as::<_, NiTriShapeData>(shape.base.base.geometry_data) else {
            continue;
        };

        if data.base.base.vertices.is_empty() || data.triangles.is_empty() {
            continue;
        }

        let positions = data
            .base
            .base
            .vertices
            .iter()
            .map(|vertex| [vertex.x, vertex.y, vertex.z])
            .collect::<Vec<_>>();
        let normals = data
            .base
            .base
            .normals
            .iter()
            .map(|normal| [normal.x, normal.y, normal.z])
            .collect::<Vec<_>>();
        let uvs = data
            .base
            .base
            .uv_set(0)
            .unwrap_or(&[])
            .iter()
            .map(|uv| [uv.x, 1.0 - uv.y])
            .collect::<Vec<_>>();
        let indices = data
            .triangles
            .iter()
            .flat_map(|triangle| triangle.iter().copied())
            .collect::<Vec<_>>();

        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        if normals.len() == data.base.base.vertices.len() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
        if uvs.len() == data.base.base.vertices.len() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        mesh.insert_indices(bevy::render::mesh::Indices::U16(indices));

        let av_object = &shape.base.base.base;
        let rotation = Mat3::from_cols_array(&av_object.rotation.to_cols_array()).transpose();
        let transform = Transform {
            translation: Vec3::new(
                av_object.translation.x,
                av_object.translation.y,
                av_object.translation.z,
            ),
            rotation: Quat::from_mat3(&rotation),
            scale: Vec3::splat(av_object.scale * 0.01), // Scale down by 0.01 to convert from centimeters to meters
        };

        let mut material = StandardMaterial::default();
        if let Some(texture_path) = diffuse_texture_path(&stream, shape) {
            if let Some(texture_bytes) = find_file(file_system, &texture_path) {
                let extension = texture_path
                    .rsplit('.')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                match Image::from_buffer(
                    &texture_bytes,
                    ImageType::Extension(&extension),
                    CompressedImageFormats::all(),
                    true,
                    ImageSampler::linear(),
                    bevy::asset::RenderAssetUsages::default(),
                ) {
                    Ok(image) => {
                        material.base_color_texture = Some(images.add(image));
                    }
                    Err(error) => {
                        bevy::log::warn!(
                            "Could not decode texture {texture_path} for {file_name}: {error}"
                        );
                    }
                }
            } else {
                bevy::log::warn!("Texture not found in archive: {texture_path}");
            }
        }

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            transform,
            crate::LoadedNifMesh,
        ));
        shape_count += 1;
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from {file_name}");
}

fn diffuse_texture_path(stream: &NiStream, shape: &NiTriShape) -> Option<String> {
    let property = shape
        .base
        .base
        .base
        .get_property::<NiTexturingProperty>(stream)?;
    let texture_map = property.texture_maps.first()?.as_ref()?;
    let texture_link = match texture_map {
        TextureMap::Map(map) => map.texture,
        TextureMap::BumpMap(map) => map.base.texture,
    };
    let texture = stream.get(texture_link)?;

    match &texture.source {
        TextureSource::External(path) => Some(path.clone()),
        TextureSource::Internal(_) => None,
    }
}

fn find_file(
    file_system: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    requested_path: &str,
) -> Option<Vec<u8>> {
    let requested_path = normalize_path(requested_path);
    let file_system = file_system.read().ok()?;

    file_system.iter().find_map(|(path, bytes)| {
        (normalize_path(path) == requested_path).then(|| bytes.clone())
    })
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_ascii_lowercase()
}

fn query_state() -> Option<(String, Option<String>)> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let zip_url = params.get(ZIP_QUERY_PARAMETER)?;

    Some((zip_url, params.get(FILE_QUERY_PARAMETER)))
}

fn update_query(zip_url: &str, selected_file: Option<&str>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(current_url) = window.location().href() else {
        return;
    };
    let Ok(url) = web_sys::Url::new(&current_url) else {
        return;
    };

    let params = url.search_params();
    if zip_url.is_empty() {
        params.delete(ZIP_QUERY_PARAMETER);
    } else {
        params.set(ZIP_QUERY_PARAMETER, zip_url);
    }

    if let Some(selected_file) = selected_file {
        params.set(FILE_QUERY_PARAMETER, selected_file);
    } else {
        params.delete(FILE_QUERY_PARAMETER);
    }

    let Ok(history) = window.history() else {
        return;
    };

    if let Err(error) = history.replace_state_with_url(&JsValue::NULL, "", Some(&url.href())) {
        bevy::log::warn!("Could not update page URL: {:?}", error);
    }
}

fn draw_zip_popup(mut contexts: EguiContexts, mut state: ResMut<crate::MenuState>) -> Result {
    let ctx = contexts.ctx_mut()?;

    let show_window = &mut state.show_zip_popup;

    egui::Window::new("Load Compressed Archive")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)) // Center on browser canvas
        .show(ctx, |ui| {
            ui.label("Enter the direct URL of the target .zip archive:");

            // Text input linked directly to our Bevy state resource
            ui.text_edit_singleline(&mut state.zip_url_input);

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Download & Extract").clicked() {
                    let url_to_load = state.zip_url_input.clone();
                    update_query(&url_to_load, None);
                    state.selected_file = None;
                    state.pending_file = None;

                    let state_fs = state.file_system.clone(); // Clone the Arc<RwLock<...>> to move into the async block

                    //let state = state.clone(); // Clone the state to move into the async block
                    // Hand off the execution to the browser's async event loop
                    spawn_local(async move {
                        // Call your previously implemented zip loading code here
                        match crate::file::fetch_and_unzip(&url_to_load).await {
                            Ok(fs) => {
                                bevy::log::info!("Zip fetched and parsed successfully.");
                                state_fs.write().unwrap().extend(fs); // Update the shared file system
                            },
                            Err(e) => bevy::log::error!("Error unzipping asset: {:?}", e),
                        }
                    });

                    state.show_zip_popup = false;
                }

                if ui.button("Cancel").clicked() {
                    state.show_zip_popup = false;
                }
            });
        });

    Ok(())
}
