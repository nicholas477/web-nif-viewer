
use super::*;

fn file_system(paths: &[(&str, &[u8])]) -> HashmapFS {
    HashmapFS::new(
        paths
            .iter()
            .map(|(path, bytes)| (normalize_path(path), ArcSlice::from(*bytes)))
            .collect(),
    )
}

#[test]
fn finds_asset_below_the_nif_directory() {
    let file_system = file_system(&[("mods/example/textures/tree.dds", b"nearest")]);

    assert_eq!(
        find_file(
            &file_system,
            "mods/example/meshes/tree.nif",
            "textures/tree.dds",
        ),
        Some(b"nearest".to_vec()),
    );
}

#[test]
fn finds_asset_below_a_parent_directory() {
    let file_system = file_system(&[("mods/textures/tree.dds", b"parent")]);

    assert_eq!(
        find_file(
            &file_system,
            "mods/example/meshes/tree.nif",
            "textures/tree.dds",
        ),
        Some(b"parent".to_vec()),
    );
}

#[test]
fn resolves_parent_segments_relative_to_the_nif_directory() {
    let file_system = file_system(&[("mods/example/textures/tree.dds", b"relative")]);

    assert_eq!(
        find_file(
            &file_system,
            "mods/example/meshes/tree.nif",
            "../textures/tree.dds",
        ),
        Some(b"relative".to_vec()),
    );
}
