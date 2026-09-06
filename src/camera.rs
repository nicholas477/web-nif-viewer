use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::prelude::*;
pub use bevy_panorbit_camera::PanOrbitCamera;

/// Spawns the viewer's Z-up perspective orbit camera.
pub fn spawn(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(300.0, 300.0, 500.0).looking_at(Vec3::ZERO, Vec3::Z),
        PanOrbitCamera {
            axis: [Vec3::X, Vec3::Z, -Vec3::Y],
            allow_upside_down: true,
            orbit_smoothness: 0.0,
            ..Default::default()
        },
    ));
}

/// Combines mesh bounds into one axis-aligned bounding box.
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

/// Centers and frames the active camera around the currently loaded meshes.
pub fn focus_loaded_meshes(
    meshes: &Assets<Mesh>,
    projection: &Projection,
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

    if let Projection::Perspective(perspective) = projection {
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
