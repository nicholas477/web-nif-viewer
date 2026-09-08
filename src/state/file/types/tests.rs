
use super::get_nif_base_dir;

#[test]
fn test_get_nif_base_dir() {
    assert_eq!(get_nif_base_dir("mods/example/meshes/tree.nif"), "mods/example");
    assert_eq!(get_nif_base_dir("mods/example/meshes/"), "mods/example");
    assert_eq!(get_nif_base_dir("mods/example/meshes"), "mods/example");
    assert_eq!(get_nif_base_dir("tree.nif"), "");
    assert_eq!(get_nif_base_dir(""), "");
}
