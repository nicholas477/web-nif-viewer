use bevy::{
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
};

pub fn export_loaded_meshes(
    file_name: Option<&str>,
    meshes: &Assets<Mesh>,
    loaded_meshes: &Query<(Entity, &crate::nif::LoadedNifMesh, &Transform)>,
) {
    let base_name = file_name
        .and_then(|name| name.rsplit(['/', '\\']).next())
        .and_then(|name| name.rsplit_once('.').map(|(name, _)| name))
        .filter(|name| !name.is_empty())
        .unwrap_or("mesh");
    let bytes = obj_bytes(meshes, loaded_meshes);

    if bytes.is_empty() {
        return;
    }
    save_bytes(&format!("{base_name}.obj"), bytes);
}

fn obj_bytes(
    meshes: &Assets<Mesh>,
    loaded_meshes: &Query<(Entity, &crate::nif::LoadedNifMesh, &Transform)>,
) -> Vec<u8> {
    let mut output = String::from("# Exported by esp-viewer\n");
    let mut vertex_offset = 1;

    for (entity, loaded_mesh, transform) in loaded_meshes.iter() {
        let Some(mesh) = meshes.get(&loaded_mesh.uncolored_mesh) else {
            continue;
        };
        let Some(positions) = positions(mesh) else {
            continue;
        };
        let normals = normals(mesh);
        let indices = indices(mesh, positions.len());
        if indices.len() < 3 {
            continue;
        }

        output.push_str(&format!("o nif_shape_{}\n", entity.index()));
        for position in positions {
            let position = transform.transform_point(Vec3::from_array(*position));
            output.push_str(&format!("v {} {} {}\n", position.x, position.y, position.z));
        }
        if let Some(normals) = normals {
            for normal in normals {
                let normal = (transform.rotation * Vec3::from_array(*normal)).normalize_or_zero();
                output.push_str(&format!("vn {} {} {}\n", normal.x, normal.y, normal.z));
            }
        }
        for triangle in indices.chunks_exact(3) {
            let face = triangle.iter().map(|index| index + vertex_offset).collect::<Vec<_>>();
            if normals.is_some() {
                output.push_str(&format!(
                    "f {0}//{0} {1}//{1} {2}//{2}\n",
                    face[0], face[1], face[2]
                ));
            } else {
                output.push_str(&format!("f {} {} {}\n", face[0], face[1], face[2]));
            }
        }
        vertex_offset += positions.len() as u32;
    }

    output.into_bytes()
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

fn indices(mesh: &Mesh, vertex_count: usize) -> Vec<u32> {
    match mesh.indices() {
        Some(Indices::U16(indices)) => indices.iter().map(|&index| u32::from(index)).collect(),
        Some(Indices::U32(indices)) => indices.clone(),
        None => (0..vertex_count as u32).collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_bytes(file_name: &str, bytes: Vec<u8>) {
    let Some(path) = rfd::FileDialog::new().set_file_name(file_name).save_file() else {
        return;
    };
    if let Err(error) = std::fs::write(&path, bytes) {
        bevy::log::error!("Could not export {}: {error}", path.display());
    }
}

#[cfg(target_arch = "wasm32")]
fn save_bytes(file_name: &str, bytes: Vec<u8>) {
    crate::ui::file::download_bytes(&bytes, file_name);
}
