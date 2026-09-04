use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::math::bounding::BoundingVolume;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_panorbit_camera::PanOrbitCamera;
use std::collections::{HashMap, HashSet};
use tes3::nif::{
    NiCollisionSwitch, NiLink, NiNode, NiStencilProperty, NiStream, NiTexturingProperty,
    NiTriShape, NiTriShapeData, RootCollisionNode, TextureMap, TextureSource,
};

#[derive(Component)]
pub struct LoadedNifMesh {
    default_mesh: Handle<Mesh>,
    vertex_color_mesh: Handle<Mesh>,
    normal_mesh: Handle<Mesh>,
    diffuse_texture: Option<Handle<Image>>,
    is_collision: bool,
}

fn combine_aabbs(aabbs: &[Aabb]) -> Option<Aabb> {
    if aabbs.is_empty() {
        return None;
    }

    // Initialize min and max using the first AABB's actual bounds
    let first_min = aabbs[0].center - aabbs[0].half_extents;
    let first_max = aabbs[0].center + aabbs[0].half_extents;

    let (min, max) = aabbs.iter().skip(1).fold(
        (first_min, first_max),
        |(mut current_min, mut current_max), aabb| {
            let aabb_min = aabb.center - aabb.half_extents;
            let aabb_max = aabb.center + aabb.half_extents;

            // Expand the bounds to encompass the new AABB
            current_min = current_min.min(aabb_min);
            current_max = current_max.max(aabb_max);

            (current_min, current_max)
        },
    );

    // Bevy provides a helper to build an Aabb from its min and max points
    Some(Aabb::from_min_max(min.to_vec3(), max.to_vec3()))
}

// Center camera on loaded mesh
pub fn center_camera_on_mesh(
    meshes: &Assets<Mesh>,
    camera_projection: &Projection,
    window: &Window,
    pan_orbit_camera: &mut PanOrbitCamera,
) {
    let aabbs = meshes
        .iter()
        .flat_map(|(_, mesh)| mesh.compute_aabb())
        .collect::<Vec<_>>();

    let aabb = combine_aabbs(&aabbs);

    let Some(aabb) = aabb else {
        bevy::log::warn!("No meshes found to center the camera on.");
        return;
    };

    pan_orbit_camera.target_focus = aabb.center.to_vec3();

    // Calculate the radius of the bounding sphere that encompasses the AABB
    // The sphere should fit within the camera's field of view, so we use the diagonal of the AABB to determine the distance
    let bounding_sphere_radius = aabb.half_extents.length();

    if let Projection::Perspective(perspective) = camera_projection {
        let fov_v = perspective.fov; // Vertical FOV in radians
        let aspect_ratio = window.width() / window.height();

        // Calculate horizontal FOV from vertical FOV and aspect ratio
        let fov_h = 2.0 * ((fov_v / 2.0).tan() * aspect_ratio).atan();

        // Distance required to fit vertically and horizontally
        let distance_v = bounding_sphere_radius / (fov_v / 2.0).sin();
        let distance_h = bounding_sphere_radius / (fov_h / 2.0).sin();

        // Choose the larger distance to prevent clipping on any side
        let required_distance = distance_v.max(distance_h);

        // Update your camera's orbit distance
        pan_orbit_camera.target_radius = required_distance;
    }
}

pub fn load_nif(
    file_name: &str,
    file_system: &crate::file::FS,
    nif_objects: &mut Vec<crate::NifObjectInfo>,
    nif_roots: &mut Vec<usize>,
    view_mode: crate::ViewMode,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    loaded_meshes: &Query<Entity, With<LoadedNifMesh>>,
) -> Result<(), String> {
    bevy::log::info!("Loading NIF file: {file_name}");

    let file_bytes = {
        let file_system = file_system.read().unwrap();
        file_system.get(file_name).cloned()
    };

    let Some(file_bytes) = file_bytes else {
        return Err(format!("Selected file is no longer available: {file_name}"));
    };

    if !file_name.to_ascii_lowercase().ends_with(".nif") {
        return Err(format!("The selected file is not a NIF: {file_name}"));
    }

    let Ok(stream) = NiStream::from_bytes(&file_bytes) else {
        return Err(format!("Could not parse the NIF file: {file_name}"));
    };

    let object_indices = stream
        .objects
        .keys()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<HashMap<_, _>>();
    *nif_objects = stream
        .objects
        .iter()
        .map(|(key, object)| crate::NifObjectInfo {
            type_name: String::from_utf8_lossy(object.type_name()).into_owned(),
            fields: format!("{object:#?}"),
            children: stream
                .get_as::<_, NiNode>(NiLink::<NiNode>::new(key))
                .map(|node| {
                    node.children
                        .iter()
                        .filter_map(|child| object_indices.get(&child.key).copied())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    *nif_roots = stream
        .roots
        .iter()
        .filter_map(|root| object_indices.get(&root.key).copied())
        .collect();

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

    for entity in loaded_meshes.iter() {
        commands.entity(entity).despawn();
    }

    let mut shape_count = 0;
    for (shape_link, shape) in stream.objects_of_type_with_link::<NiTriShape>() {
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

        let mut material = material_for_view_mode(view_mode, has_vertex_colors);
        let mut diffuse_texture = None;

        // Find NiStencilProperty as a child of this NiTriShape, if it exists
        if let Some(stencil_property) = shape
            .base
            .base
            .base
            .get_property::<NiStencilProperty>(&stream)
        {
            // Placeholder for any logic that might use the stencil property
            bevy::log::info!("Found NiStencilProperty for shape: {:?}", stencil_property);

            // Regular direction with bevy is CCW
            material.cull_mode = match stencil_property.draw_mode {
                tes3::nif::DrawMode::Clockwise => Some(wgpu_types::Face::Front),
                tes3::nif::DrawMode::Both => None,
                _ => Some(wgpu_types::Face::Back),
            }
        }

        if let Some(alpha_property) = shape
            .base
            .base
            .base
            .get_property::<tes3::nif::NiAlphaProperty>(&stream)
        {
            alpha_property.alpha_blending().then(|| {
                material.alpha_mode = AlphaMode::Blend;
            });

            alpha_property.alpha_testing().then(|| {
                material.alpha_mode = AlphaMode::Mask(0.5);
            });
        }

        if matches!(view_mode, crate::ViewMode::Lit | crate::ViewMode::Unlit)
            && let Some(texture_path) = diffuse_texture_path(&stream, shape)
        {
            if let Some(texture_bytes) = crate::file::find_file(file_system, &texture_path) {
                let extension = texture_path
                    .rsplit('.')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    mipmap_filter: ImageFilterMode::Linear,
                    mag_filter: ImageFilterMode::Linear,
                    min_filter: ImageFilterMode::Linear,
                    ..Default::default()
                });

                // Use the ImageAddressMode::Repeat for the texture to ensure it tiles correctly on the mesh
                match Image::from_buffer(
                    &texture_bytes,
                    ImageType::Extension(&extension),
                    CompressedImageFormats::all(),
                    true,
                    sampler,
                    bevy::asset::RenderAssetUsages::default(),
                ) {
                    Ok(image) => {
                        let texture = images.add(image);
                        material.base_color_texture = Some(texture.clone());
                        diffuse_texture = Some(texture);
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

        let default_mesh = meshes.add(mesh.clone());
        let mut vertex_color_mesh = mesh.clone();
        if !has_vertex_colors {
            vertex_color_mesh.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                vec![[1.0, 1.0, 1.0, 1.0]; vertex_color_mesh.count_vertices()],
            );
        }
        let vertex_color_mesh = meshes.add(vertex_color_mesh);
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
        let normal_mesh = meshes.add(normal_mesh);
        let loaded_mesh = LoadedNifMesh {
            default_mesh,
            vertex_color_mesh,
            normal_mesh,
            diffuse_texture,
            is_collision: collision_shapes.contains(&shape_link.key),
        };

        commands.spawn((
            Mesh3d(mesh_handle_for_view_mode(&loaded_mesh, view_mode)),
            MeshMaterial3d(materials.add(material)),
            transform,
            if view_mode == crate::ViewMode::Collision && !loaded_mesh.is_collision {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
            loaded_mesh,
        ));
        shape_count += 1;
    }

    if shape_count == 0 {
        return Err(format!("No renderable meshes were found in: {file_name}"));
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from {file_name}");
    Ok(())
}

pub fn apply_view_mode(
    view_mode: crate::ViewMode,
    materials: &mut Assets<StandardMaterial>,
    loaded_meshes: &mut Query<
        (&mut Mesh3d, &MeshMaterial3d<StandardMaterial>, &mut Visibility, &LoadedNifMesh),
    >,
) {
    for (mut mesh, material_handle, mut visibility, loaded_mesh) in loaded_meshes.iter_mut() {
        mesh.0 = mesh_handle_for_view_mode(loaded_mesh, view_mode);
        *visibility = if view_mode == crate::ViewMode::Collision && !loaded_mesh.is_collision {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            match view_mode {
                crate::ViewMode::Lit => {
                    material.unlit = false;
                    material.base_color_texture = loaded_mesh.diffuse_texture.clone();
                }
                crate::ViewMode::Unlit => {
                    material.unlit = true;
                    material.base_color_texture = loaded_mesh.diffuse_texture.clone();
                }
                crate::ViewMode::VertexColors => {
                    material.unlit = true;
                    material.base_color = Color::WHITE;
                    material.base_color_texture = None;
                }
                crate::ViewMode::Normals => {
                    material.unlit = true;
                    material.base_color = Color::srgb(0.5, 0.5, 1.0);
                    material.base_color_texture = None;
                }
                crate::ViewMode::Collision => {
                    material.unlit = true;
                    material.base_color = Color::srgb(0.9, 0.2, 0.1);
                    material.base_color_texture = None;
                }
            }
        }
    }
}

fn mesh_handle_for_view_mode(loaded_mesh: &LoadedNifMesh, view_mode: crate::ViewMode) -> Handle<Mesh> {
    match view_mode {
        crate::ViewMode::VertexColors => loaded_mesh.vertex_color_mesh.clone(),
        crate::ViewMode::Normals => loaded_mesh.normal_mesh.clone(),
        _ => loaded_mesh.default_mesh.clone(),
    }
}

fn material_for_view_mode(view_mode: crate::ViewMode, has_vertex_colors: bool) -> StandardMaterial {
    match view_mode {
        crate::ViewMode::Lit => StandardMaterial::default(),
        crate::ViewMode::Unlit => StandardMaterial {
            unlit: true,
            ..Default::default()
        },
        crate::ViewMode::VertexColors => StandardMaterial {
            unlit: true,
            base_color: if has_vertex_colors {
                Color::WHITE
            } else {
                Color::srgb(0.45, 0.45, 0.45)
            },
            ..Default::default()
        },
        crate::ViewMode::Normals => StandardMaterial {
            unlit: true,
            base_color: Color::srgb(0.5, 0.5, 1.0),
            ..Default::default()
        },
        crate::ViewMode::Collision => StandardMaterial {
            unlit: true,
            base_color: Color::srgb(0.9, 0.2, 0.1),
            ..Default::default()
        },
    }
}

pub fn diffuse_texture_path(stream: &NiStream, shape: &NiTriShape) -> Option<String> {
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
