use bevy::prelude::*;
use tes3::nif::{
    AlphaTestFunction, NiStream, NiTexturingProperty, NiTriShape, TextureMap, TextureSource,
};

pub mod mesh;
pub use mesh::*;

/// Applies the selected shading, vertex-color, collision, and wireframe options to loaded entities.
pub fn apply_view_options(
    view_options: crate::ViewOptions,
    materials: &mut Assets<crate::PhongMaterial>,
    loaded_meshes: &mut Query<
        (
            &mut Mesh3d,
            &MeshMaterial3d<crate::PhongMaterial>,
            &mut Visibility,
            &LoadedNifMesh,
        ),
        Without<LoadedNifWireframe>,
    >,
    wireframes: &mut Query<(&mut Visibility, &LoadedNifWireframe), Without<LoadedNifMesh>>,
) {
    for (mut mesh, material_handle, mut visibility, loaded_mesh) in loaded_meshes.iter_mut() {
        mesh.0 = mesh_handle_for_options(view_options, loaded_mesh);
        *visibility = visibility_for(view_options.collision, loaded_mesh.is_collision);
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            apply_material_options(&mut material, view_options, loaded_mesh);
        }
    }

    for (mut visibility, wireframe) in wireframes.iter_mut() {
        *visibility = if view_options.wireframe {
            visibility_for(view_options.collision, wireframe.is_collision)
        } else {
            Visibility::Hidden
        };
    }
}

/// Selects the mesh variant needed for the current shading and vertex-color options.
fn mesh_handle_for_options(
    view_options: crate::ViewOptions,
    loaded_mesh: &LoadedNifMesh,
) -> Handle<Mesh> {
    match view_options.shading_mode {
        crate::ShadingMode::Normals => loaded_mesh.normal_mesh.clone(),
        _ if view_options.vertex_colors != crate::DisplayMode::Off => {
            loaded_mesh.vertex_color_mesh.clone()
        }
        _ => loaded_mesh.uncolored_mesh.clone(),
    }
}

/// Updates a material's shader flags and texture binding for the selected view options.
fn apply_material_options(
    material: &mut crate::PhongMaterial,
    view_options: crate::ViewOptions,
    loaded_mesh: &LoadedNifMesh,
) {
    let use_vertex_colors = view_options.shading_mode == crate::ShadingMode::Normals
        || view_options.vertex_colors != crate::DisplayMode::Off;
    let use_texture = !matches!(view_options.shading_mode, crate::ShadingMode::Normals)
        && view_options.vertex_colors != crate::DisplayMode::Only;
    let unlit = view_options.shading_mode != crate::ShadingMode::Lit
        || view_options.vertex_colors == crate::DisplayMode::Only;
    material.color = LinearRgba::WHITE;
    material.color_texture = if use_texture {
        loaded_mesh.diffuse_texture.clone()
    } else {
        None
    };
    material.settings.x = use_texture as u32 as f32;
    material.settings.y = use_vertex_colors as u32 as f32;
    material.settings.z = unlit as u32 as f32;
}

/// Packs a NIF alpha comparison function and reference value into a shader uniform.
fn alpha_test_settings(test_function: AlphaTestFunction, test_ref: u8) -> f32 {
    let function = match test_function {
        AlphaTestFunction::Less => 1,
        AlphaTestFunction::Equal => 2,
        AlphaTestFunction::LessEqual => 3,
        AlphaTestFunction::Greater => 4,
        AlphaTestFunction::NotEqual => 5,
        AlphaTestFunction::GreaterEqual => 6,
        AlphaTestFunction::Never => 7,
        AlphaTestFunction::Always => 0,
    };
    function as f32 + f32::from(test_ref) / 256.0
}

/// Determines whether a regular or collision entity is visible for a collision display mode.
fn visibility_for(display_mode: crate::DisplayMode, is_collision: bool) -> Visibility {
    match display_mode {
        crate::DisplayMode::Off if is_collision => Visibility::Hidden,
        crate::DisplayMode::Only if !is_collision => Visibility::Hidden,
        _ => Visibility::Inherited,
    }
}

/// Resolves the external diffuse texture path referenced by a NIF shape.
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
