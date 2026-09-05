use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use std::collections::{HashMap, HashSet};
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
    pub is_collision: bool,
}

#[derive(Component)]
pub struct LoadedNifWireframe {
    pub is_collision: bool,
}

/// Parses a NIF file, builds its inspector data, and replaces the rendered mesh entities.
pub fn load_nif(
    file_name: &str,
    file_system: &crate::file::FS,
    nif_objects: &mut Vec<crate::NifObjectInfo>,
    nif_roots: &mut Vec<usize>,
    nif_selected_node: &mut Option<usize>,
    triangle_count: &mut usize,
    view_options: crate::ViewOptions,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<crate::PhongMaterial>,
    loaded_meshes: &Query<Entity, With<LoadedNifMesh>>,
    loaded_wireframes: &Query<Entity, With<LoadedNifWireframe>>,
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
        .map(|(_, object)| crate::NifObjectInfo {
            type_name: String::from_utf8_lossy(object.type_name()).into_owned(),
            fields: format!("{object:#?}"),
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
    *nif_roots = stream
        .roots
        .iter()
        .filter_map(|root| object_indices.get(&root.key).copied())
        .collect();
    *nif_selected_node = None;

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
    *triangle_count = 0;

    for entity in loaded_meshes.iter() {
        commands.entity(entity).despawn();
    }
    for entity in loaded_wireframes.iter() {
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
        *triangle_count += data.triangles.len();

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
        let mut diffuse_texture = None;

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

        if let Some(texture_path) = diffuse_texture_path(&stream, shape) {
            if let Some(texture_bytes) = crate::file::find_file(file_system, file_name, &texture_path) {
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
                        material.color_texture = Some(texture.clone());
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

        let mut uncolored_mesh = mesh.clone();
        uncolored_mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        let uncolored_mesh = meshes.add(uncolored_mesh);
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
            uncolored_mesh,
            vertex_color_mesh,
            normal_mesh,
            diffuse_texture,
            is_collision: collision_shapes.contains(&shape_link.key),
        };
        let is_collision = loaded_mesh.is_collision;
        apply_material_options(&mut material, view_options, &loaded_mesh);
        let base_visibility = visibility_for(view_options.collision, is_collision);

        commands.spawn((
            Mesh3d(mesh_handle_for_options(view_options, &loaded_mesh)),
            MeshMaterial3d(materials.add(material)),
            transform,
            base_visibility,
            loaded_mesh,
        ));

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

        commands.spawn((
            Mesh3d(meshes.add(wireframe_mesh)),
            MeshMaterial3d(materials.add(crate::PhongMaterial {
                color: LinearRgba::BLACK,
                color_texture: None,
                settings: Vec4::new(0.0, 0.0, 1.0, 0.0),
                alpha_mode: AlphaMode::Opaque,
                cull_mode: Some(wgpu_types::Face::Back),
            })),
            transform,
            if view_options.wireframe {
                base_visibility
            } else {
                Visibility::Hidden
            },
            LoadedNifWireframe { is_collision },
        ));
        shape_count += 1;
    }

    if shape_count == 0 {
        return Err(format!("No renderable meshes were found in: {file_name}"));
    }

    bevy::log::info!("Spawned {shape_count} NiTriShape meshes from {file_name}");
    Ok(())
}
