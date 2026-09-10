use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use arc_slice::ArcSlice;
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use tes3::nif::{
    NiCollisionSwitch, NiStencilProperty, NiStream, NiTriShape, NiTriShapeData, RootCollisionNode,
    Visitor,
};

use crate::nif::*;

#[derive(Component)]
pub struct LoadedNifMesh {
    pub uncolored_mesh: Handle<Mesh>,
    pub vertex_color_mesh: Handle<Mesh>,
    pub normal_mesh: Handle<Mesh>,
    pub diffuse_texture: Option<Handle<Image>>,
    pub diffuse_texture_path: Option<String>,
    pub is_collision: bool,
    pub nif_node_index: Option<usize>,
}

#[derive(Component)]
pub struct LoadedNifWireframe {
    pub is_collision: bool,
    pub nif_node_index: usize,
}

struct TextureLoadResult {
    path: String,
    bytes: Option<ArcSlice<[u8]>>,
    material: Handle<crate::PhongMaterial>,
}

/// Collects asynchronous texture reads until they can be applied to Bevy assets.
#[derive(Resource, Default, Clone)]
pub struct PendingTextureLoads {
    completed: Arc<Mutex<Vec<TextureLoadResult>>>,
    textures: Arc<Mutex<HashMap<AssetId<crate::PhongMaterial>, Handle<Image>>>>,
    texture_paths: Arc<Mutex<HashMap<AssetId<crate::PhongMaterial>, String>>>,
    missing_paths: Arc<Mutex<HashSet<String>>>,
}

impl PendingTextureLoads {
    pub fn request(
        &self,
        fsstate: crate::state::FSState,
        path: String,
        material: Handle<crate::PhongMaterial>,
    ) {
        let completed = Arc::clone(&self.completed);
        let lookup_paths = Self::texture_lookup_paths(&path);

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            for read_path in lookup_paths {
                bevy::log::info!("Loading texture asynchronously: {read_path}");
                if let Some(bytes) = crate::state::file::Filesystem::read(&fsstate, &read_path, false).await {
                    completed.lock().unwrap().push(TextureLoadResult {
                        path: read_path,
                        bytes: Some(bytes),
                        material,
                    });
                    return;
                }
            }
            completed.lock().unwrap().push(TextureLoadResult { path, bytes: None, material });
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut bytes = None;
            let mut resolved_path = path.clone();
            for read_path in lookup_paths {
                bytes = bevy::tasks::futures_lite::future::block_on(
                    crate::state::file::Filesystem::read(&fsstate, &read_path, false),
                );
                if bytes.is_some() {
                    resolved_path = read_path;
                    break;
                }
            }
            completed.lock().unwrap().push(TextureLoadResult {
                path: resolved_path,
                bytes,
                material,
            });
        }
    }

    fn take_completed(&self) -> Vec<TextureLoadResult> {
        std::mem::take(&mut *self.completed.lock().unwrap())
    }


    /// Returns texture paths in lookup priority order.
    fn texture_lookup_paths(path: &str) -> Vec<String> {
    let Some((stem, extension)) = path.rsplit_once('.') else {
            return vec![path.to_string()];
    };
        match extension.to_ascii_lowercase().as_str() {
            "bmp" | "tga" => vec![format!("{stem}.dds"), path.to_string()],
            _ => vec![path.to_string()],
        }
    }
    pub fn texture_for(&self, material: &Handle<crate::PhongMaterial>) -> Option<Handle<Image>> {
        self.textures.lock().unwrap().get(&material.id()).cloned()
    }

    pub fn texture_path_for(&self, material: &Handle<crate::PhongMaterial>) -> Option<String> {
        self.texture_paths.lock().unwrap().get(&material.id()).cloned()
    }

    /// Clears paths that were unresolved for the previously loaded NIF.
    pub fn clear_missing_paths(&self) {
        self.missing_paths.lock().unwrap().clear();
    }

    /// Returns texture references that were not found in the archive or resource folders.
    pub fn missing_paths(&self) -> Vec<String> {
        let mut paths = self.missing_paths.lock().unwrap().iter().cloned().collect::<Vec<_>>();
        paths.sort_unstable();
        paths
    }
}

/// Decodes completed asynchronous texture reads and assigns them to their materials.
pub fn apply_completed_texture_loads(
    pending: &PendingTextureLoads,
    images: &mut Assets<Image>,
    materials: &mut Assets<crate::PhongMaterial>,
) {
    for result in pending.take_completed() {
        let Some(bytes) = result.bytes else {
            bevy::log::warn!("Texture not found in archive or resource folders: {}", result.path);
            pending.missing_paths.lock().unwrap().insert(result.path);
            continue;
        };
        let extension = result.path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
        let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mipmap_filter: ImageFilterMode::Linear,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..Default::default()
        });
        match Image::from_buffer(
            &bytes,
            ImageType::Extension(&extension),
            CompressedImageFormats::all(),
            true,
            sampler,
            bevy::asset::RenderAssetUsages::default(),
        ) {
            Ok(image) => {
                let image = images.add(image);
                pending.textures.lock().unwrap().insert(result.material.id(), image.clone());
                pending.texture_paths.lock().unwrap().insert(result.material.id(), result.path.clone());
                if let Some(mut material) = materials.get_mut(&result.material) {
                    if material.settings.x != 0.0 {
                        material.color_texture = Some(image);
                    }
                    bevy::log::info!("Applied asynchronously loaded texture: {}", result.path);
                }
            }
            Err(error) => bevy::log::warn!("Could not decode texture {}: {error}", result.path),
        }
    }
}

pub struct NifMeshLoadParams<'f, 'a, 'w, 's> {
    pub file: &'f [u8],

    pub fsstate: &'a crate::state::FSState,
    pub nif_objects: &'a mut Vec<crate::NifObjectInfo>,
    pub nif_roots: &'a mut Vec<usize>,
    pub nif_selected_node: &'a mut Option<usize>,
    pub triangle_count: &'a mut usize,
    pub view_options: crate::ViewOptions,
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<crate::PhongMaterial>,
    pub pending_texture_loads: &'a mut PendingTextureLoads,
    pub loaded_meshes: &'a Query<'w, 's, (Entity, &'static LoadedNifMesh)>,
    pub loaded_wireframes: &'a Query<'w, 's, Entity, With<LoadedNifWireframe>>,
}

impl<'f, 'a, 'w, 's> NifMeshLoadParams<'f, 'a, 'w, 's> {
    /// Collects the NIF loader inputs from the UI system state.
    pub fn from_ui_state(
        file: &'f [u8],
        ui_state: &'a mut crate::ui::UiSystemParams<'w, 's>,
    ) -> Self {
        let view_options = crate::ViewOptions::from(&*ui_state.state);
        let inspector = &mut ui_state.state.inspector;

        Self {
            file,
            fsstate: &ui_state.fsstate,
            nif_objects: &mut inspector.nif_objects,
            nif_roots: &mut inspector.nif_roots,
            nif_selected_node: &mut inspector.selected_node,
            triangle_count: &mut inspector.triangle_count,
            view_options,
            commands: &mut ui_state.commands,
            meshes: &mut ui_state.meshes,
            materials: &mut ui_state.materials,
            pending_texture_loads: &mut ui_state.pending_texture_loads,
            loaded_meshes: &ui_state.loaded_meshes,
            loaded_wireframes: &ui_state.loaded_wireframe_entities,
        }
    }
}

/// Parses a NIF file, builds its inspector data, and replaces the rendered mesh entities.
pub fn load_nif(params: NifMeshLoadParams) -> Result<(), String> {
    let Ok(stream) = NiStream::from_bytes(params.file) else {
        return Err("Could not parse the NIF file".to_string());
    };

    let object_indices = stream
        .objects
        .keys()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<HashMap<_, _>>();
    *params.nif_objects = stream
        .objects
        .iter()
        .map(|(_, object)| crate::NifObjectInfo {
            type_name: String::from_utf8_lossy(object.type_name()).into_owned(),
            fields: format!("{object:#?}"),
            object: object.clone(),
            children: {
                let mut children = Vec::new();
                object.visitor(&mut |link| {
                    if let Some(index) = object_indices.get(&link) {
                        children.push(*index);
                    }
                });
                children
            },
        })
        .collect();

    *params.nif_roots = stream
        .roots
        .iter()
        .filter_map(|root| object_indices.get(&root.key).copied())
        .collect();
    *params.nif_selected_node = None;

    let collision_shapes = stream
        .objects_of_type::<RootCollisionNode>()
        .flat_map(|node| node.base.children_recursive(&stream))
        .chain(
            stream
                .objects_of_type::<NiCollisionSwitch>()
                .flat_map(|node| node.base.children_recursive(&stream)),
        )
        .map(|link| link.key)
        .collect::<HashSet<_>>();
    *params.triangle_count = 0;

    for (entity, _) in params.loaded_meshes.iter() {
        params.commands.entity(entity).despawn();
    }
    for entity in params.loaded_wireframes.iter() {
        params.commands.entity(entity).despawn();
    }

    let mut shape_count = 0;
    for (shape_link, shape) in stream.objects_of_type_with_link::<NiTriShape>() {
        let Some(data) = stream.get_as::<_, NiTriShapeData>(shape.base.base.geometry_data) else {
            continue;
        };
        let Some(nif_node_index) = object_indices.get(&shape_link.key).copied() else {
            continue;
        };

        if data.base.base.vertices.is_empty() || data.triangles.is_empty() {
            continue;
        }
        *params.triangle_count += data.triangles.len();

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
            .map(|uv| [uv.x, uv.y])
            .collect::<Vec<_>>();

        let indices = data
            .triangles
            .iter()
            .flat_map(|triangle| triangle.iter().copied())
            .collect::<Vec<_>>();

        let colors = data
            .base
            .base
            .vertex_colors
            .iter()
            .map(|color| [color.x, color.y, color.z, color.w])
            .collect::<Vec<_>>();
        let has_vertex_colors = colors.len() == data.base.base.vertices.len();

        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        if normals.len() == data.base.base.vertices.len() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
        }
        if uvs.len() == data.base.base.vertices.len() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        if has_vertex_colors {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone());
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
            scale: Vec3::splat(av_object.scale), // Scale down by 0.01 to convert from centimeters to meters
        };

        let mut material = crate::PhongMaterial {
            color: LinearRgba::WHITE,
            color_texture: None,
            settings: Vec4::ZERO,
            alpha_mode: AlphaMode::Opaque,
            cull_mode: Some(wgpu_types::Face::Back),
        };
        let diffuse_texture = None;

        // Find NiStencilProperty as a child of this NiTriShape, if it exists
        if let Some(stencil_property) = shape
            .base
            .base
            .base
            .get_property::<NiStencilProperty>(&stream)
        {
            material.cull_mode = match stencil_property.draw_mode {
                tes3::nif::DrawMode::Clockwise => Some(wgpu_types::Face::Front),
                tes3::nif::DrawMode::Both => None,
                _ => Some(wgpu_types::Face::Back),
            };
        }

        if let Some(alpha_property) = shape
            .base
            .base
            .base
            .get_property::<tes3::nif::NiAlphaProperty>(&stream)
        {
            if alpha_property.alpha_blending() {
                material.alpha_mode = AlphaMode::Blend;
            }
            if alpha_property.alpha_testing() {
                material.settings.w =
                    alpha_test_settings(alpha_property.test_mode(), alpha_property.test_ref);
                material.alpha_mode = AlphaMode::Mask(0.0);
            }
        }

        let texture_path = diffuse_texture_path(&stream, shape);

        let mut uncolored_mesh = mesh.clone();
        uncolored_mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        let uncolored_mesh = params.meshes.add(uncolored_mesh);
        let mut vertex_color_mesh = mesh.clone();
        if !has_vertex_colors {
            vertex_color_mesh.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                vec![[1.0, 1.0, 1.0, 1.0]; vertex_color_mesh.count_vertices()],
            );
        }
        let vertex_color_mesh = params.meshes.add(vertex_color_mesh);
        let mut normal_mesh = mesh;
        let normal_colors = normals
            .iter()
            .map(|normal| {
                [
                    normal[0] * 0.5 + 0.5,
                    normal[1] * 0.5 + 0.5,
                    normal[2] * 0.5 + 0.5,
                    1.0,
                ]
            })
            .collect::<Vec<_>>();
        if normal_colors.len() == normal_mesh.count_vertices() {
            normal_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, normal_colors);
        } else {
            normal_mesh.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                vec![[0.5, 0.5, 1.0, 1.0]; normal_mesh.count_vertices()],
            );
        }
        let normal_mesh = params.meshes.add(normal_mesh);
        let loaded_mesh = LoadedNifMesh {
            uncolored_mesh,
            vertex_color_mesh,
            normal_mesh,
            diffuse_texture,
            diffuse_texture_path: texture_path.clone(),
            is_collision: collision_shapes.contains(&shape_link.key),
            nif_node_index: Some(nif_node_index),
        };
        let is_collision = loaded_mesh.is_collision;
        apply_material_options(&mut material, params.view_options, &loaded_mesh, None);
        let base_visibility = visibility_for(params.view_options.collision, is_collision);
        let wireframe_transform = Transform {
            scale: transform.scale * 1.0001,
            ..transform
        };

        // Mesh spawned here
        let material_handle = params.materials.add(material);
        params.commands.spawn((
            Mesh3d(mesh_handle_for_options(params.view_options, &loaded_mesh)),
            MeshMaterial3d(material_handle.clone()),
            transform,
            base_visibility,
            loaded_mesh,
        ));

        if let Some(texture_path) = texture_path {
            params.pending_texture_loads.request(
                params.fsstate.clone(),
                texture_path,
                material_handle.clone(),
            );
        }

        let mut wireframe_mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::LineList,
            bevy::asset::RenderAssetUsages::default(),
        );
        wireframe_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            data.base
                .base
                .vertices
                .iter()
                .map(|vertex| [vertex.x, vertex.y, vertex.z])
                .collect::<Vec<_>>(),
        );

        wireframe_mesh.insert_indices(bevy::render::mesh::Indices::U16(
            data.triangles
                .iter()
                .flat_map(|triangle| {
                    [
                        triangle[0],
                        triangle[1],
                        triangle[1],
                        triangle[2],
                        triangle[2],
                        triangle[0],
                    ]
                })
                .collect(),
        ));

        params.commands.spawn((
            Mesh3d(params.meshes.add(wireframe_mesh)),
            MeshMaterial3d(params.materials.add(crate::PhongMaterial {
                color: LinearRgba::BLACK,
                color_texture: None,
                settings: Vec4::new(0.0, 0.0, 1.0, 0.0),
                alpha_mode: AlphaMode::Opaque,
                cull_mode: Some(wgpu_types::Face::Back),
            })),
            wireframe_transform,
            if params.view_options.wireframe {
                base_visibility
            } else {
                Visibility::Hidden
            },
            Pickable::IGNORE,
            LoadedNifWireframe {
                is_collision,
                nif_node_index,
            },
        ));
        shape_count += 1;
    }

    if shape_count == 0 {
        return Err("No renderable meshes were found in NIF file!".to_string());
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from NIF file");
    Ok(())
}
