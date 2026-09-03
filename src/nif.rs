use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
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
use egui::{LayerId, Ui, UiBuilder};
use tes3::nif::{
    NiStencilProperty, NiStream, NiTexturingProperty, NiTriShape, NiTriShapeData, TextureMap,
    TextureSource,
};

pub fn load_nif(
    file_name: &str,
    file_system: &crate::file::FS,
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
            scale: Vec3::splat(av_object.scale * 0.01), // Scale down by 0.01 to convert from centimeters to meters
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
            crate::LoadedNifMesh,
        ));
        shape_count += 1;
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from {file_name}");
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
