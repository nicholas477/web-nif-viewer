use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::math::bounding::BoundingVolume;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_panorbit_camera::PanOrbitCamera;
use tes3::nif::{
    NiStencilProperty, NiStream, NiTexturingProperty, NiTriShape, NiTriShapeData, TextureMap,
    TextureSource,
};

#[derive(Component)]
pub struct LoadedNifMesh;

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
        if colors.len() == mesh.count_vertices() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
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

        let mut material = StandardMaterial {
            unlit: true,
            ..Default::default()
        };

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

        if let Some(texture_path) = diffuse_texture_path(&stream, shape) {
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
            LoadedNifMesh,
        ));
        shape_count += 1;
    }

    if shape_count == 0 {
        return Err(format!("No renderable meshes were found in: {file_name}"));
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from {file_name}");
    Ok(())
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
