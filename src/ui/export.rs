use bevy::{
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
};
use std::collections::HashMap;

pub fn export_loaded_meshes(
    file_name: Option<&str>,
    meshes: &Assets<Mesh>,
    loaded_meshes: &Query<(
        Entity,
        &crate::nif::LoadedNifMesh,
        &Transform,
        &MeshMaterial3d<crate::PhongMaterial>,
    )>,
    pending_texture_loads: &crate::nif::PendingTextureLoads,
) {
    let base_name = file_name
        .and_then(|name| name.rsplit(['/', '\\']).next())
        .and_then(|name| name.rsplit_once('.').map(|(name, _)| name))
        .filter(|name| !name.is_empty())
        .unwrap_or("mesh");
    let (obj, mtl) = obj_bytes(meshes, loaded_meshes, pending_texture_loads, base_name);

    if obj.is_empty() {
        return;
    }
    save_export(base_name, obj, mtl);
}

fn obj_bytes(
    meshes: &Assets<Mesh>,
    loaded_meshes: &Query<(
        Entity,
        &crate::nif::LoadedNifMesh,
        &Transform,
        &MeshMaterial3d<crate::PhongMaterial>,
    )>,
    pending_texture_loads: &crate::nif::PendingTextureLoads,
    base_name: &str,
) -> (Vec<u8>, Vec<u8>) {
    let mut output = format!("# Exported by esp-viewer\nmtllib {base_name}.mtl\n");
    let mut materials = String::from("# Exported by esp-viewer\n");
    let mut material_names: HashMap<Option<String>, String> = HashMap::new();
    let mut vertex_offset = 1;
    let mut texcoord_offset = 1;

    for (entity, loaded_mesh, transform, material_handle) in loaded_meshes.iter() {
        let Some(mesh) = meshes.get(&loaded_mesh.uncolored_mesh) else {
            continue;
        };
        let Some(positions) = positions(mesh) else {
            continue;
        };
        let normals = normals(mesh);
        let uvs = uvs(mesh);
        let has_uvs = uvs.is_some_and(|uvs| uvs.len() == positions.len());
        let indices = indices(mesh, positions.len());
        if indices.len() < 3 {
            continue;
        }

        output.push_str(&format!("o nif_shape_{}\n", entity.index()));
        let texture_path = pending_texture_loads
            .texture_path_for(&material_handle.0)
            .or_else(|| loaded_mesh.diffuse_texture_path.clone())
            .map(|path| path.replace('\\', "/"));
        let material_name = match material_names.get(&texture_path) {
            Some(name) => name.clone(),
            None => {
                let name = format!("nif_material_{}", material_names.len());
                materials.push_str(&format!("newmtl {name}\nKd 1.0 1.0 1.0\n"));
                if let Some(texture_path) = &texture_path {
                    materials.push_str(&format!("map_Kd {texture_path}\n"));
                }
                materials.push('\n');
                material_names.insert(texture_path, name.clone());
                name
            }
        };
        output.push_str(&format!("usemtl {material_name}\n"));
        for position in positions {
            let position = transform.transform_point(Vec3::from_array(*position));
            output.push_str(&format!("v {} {} {}\n", position.x, position.y, position.z));
        }
        if has_uvs && let Some(uvs) = uvs {
            for uv in uvs {
                output.push_str(&format!("vt {} {}\n", uv[0], uv[1]));
            }
        }
        if let Some(normals) = normals {
            for normal in normals {
                let normal = (transform.rotation * Vec3::from_array(*normal)).normalize_or_zero();
                output.push_str(&format!("vn {} {} {}\n", normal.x, normal.y, normal.z));
            }
        }
        for triangle in indices.chunks_exact(3) {
            let face = triangle.iter().map(|index| index + vertex_offset).collect::<Vec<_>>();
            let texcoords = triangle.iter().map(|index| index + texcoord_offset).collect::<Vec<_>>();
            if has_uvs && normals.is_some() {
                output.push_str(&format!(
                    "f {0}/{3}/{0} {1}/{4}/{1} {2}/{5}/{2}\n",
                    face[0], face[1], face[2], texcoords[0], texcoords[1], texcoords[2]
                ));
            } else if has_uvs {
                output.push_str(&format!(
                    "f {0}/{3} {1}/{4} {2}/{5}\n",
                    face[0], face[1], face[2], texcoords[0], texcoords[1], texcoords[2]
                ));
            } else if normals.is_some() {
                output.push_str(&format!(
                    "f {0}//{0} {1}//{1} {2}//{2}\n",
                    face[0], face[1], face[2]
                ));
            } else {
                output.push_str(&format!("f {} {} {}\n", face[0], face[1], face[2]));
            }
        }
        vertex_offset += positions.len() as u32;
        if has_uvs {
            texcoord_offset += positions.len() as u32;
        }
    }

    (output.into_bytes(), materials.into_bytes())
}

fn positions(mesh: &Mesh) -> Option<&[[f32; 3]]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION)? {
        VertexAttributeValues::Float32x3(values) => Some(values),
        _ => None,
    }
}

fn normals(mesh: &Mesh) -> Option<&[[f32; 3]]> {
    match mesh.attribute(Mesh::ATTRIBUTE_NORMAL)? {
        VertexAttributeValues::Float32x3(values) => Some(values),
        _ => None,
    }
}

fn uvs(mesh: &Mesh) -> Option<&[[f32; 2]]> {
    match mesh.attribute(Mesh::ATTRIBUTE_UV_0)? {
        VertexAttributeValues::Float32x2(values) => Some(values),
        _ => None,
    }
}

fn indices(mesh: &Mesh, vertex_count: usize) -> Vec<u32> {
    match mesh.indices() {
        Some(Indices::U16(indices)) => indices.iter().map(|&index| u32::from(index)).collect(),
        Some(Indices::U32(indices)) => indices.clone(),
        None => (0..vertex_count as u32).collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_export(base_name: &str, obj: Vec<u8>, mtl: Vec<u8>) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{base_name}.obj"))
        .save_file()
    else {
        return;
    };
    if let Err(error) = std::fs::write(&path, obj) {
        bevy::log::error!("Could not export {}: {error}", path.display());
        return;
    }
    if let Err(error) = std::fs::write(path.with_extension("mtl"), mtl) {
        bevy::log::error!("Could not export material file: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
fn save_export(base_name: &str, obj: Vec<u8>, mtl: Vec<u8>) {
    crate::ui::file::download_bytes(&obj, &format!("{base_name}.obj"));
    crate::ui::file::download_bytes(&mtl, &format!("{base_name}.mtl"));
}
