#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> material_settings: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = material_color;
#ifdef VERTEX_UVS_A
    if (material_settings.x > 0.5) {
        color *= textureSample(color_texture, color_sampler, in.uv);
    }
#endif
#ifdef VERTEX_COLORS
    if (material_settings.y > 0.5) {
        color *= in.color;
    }
#endif

    let alpha_test_function = i32(floor(material_settings.w));
    let alpha_test_reference = fract(material_settings.w);
    let alpha_passes = select(
        select(
            select(
                select(
                    select(color.a > alpha_test_reference, color.a >= alpha_test_reference, alpha_test_function == 6),
                    color.a != alpha_test_reference,
                    alpha_test_function == 5,
                ),
                color.a < alpha_test_reference,
                alpha_test_function == 1,
            ),
            color.a <= alpha_test_reference,
            alpha_test_function == 3,
        ),
        color.a == alpha_test_reference,
        alpha_test_function == 2,
    );
    if (alpha_test_function == 7 || (alpha_test_function != 0 && !alpha_passes)) {
        discard;
    }

    if (material_settings.z > 0.5) {
        return color;
    }

    let normal = normalize(in.world_normal);
    let light_direction = normalize(vec3<f32>(-0.45, 0.65, 0.6));
    let view_direction = normalize(view.world_position.xyz - in.world_position.xyz);
    let reflected_light = reflect(-light_direction, normal);
    let diffuse = max(dot(normal, light_direction), 0.0);
    let specular = pow(max(dot(view_direction, reflected_light), 0.0), 24.0);
    let illumination = 0.2 + 0.7 * diffuse + 0.25 * specular;

    return vec4<f32>(color.rgb * illumination, color.a);
}
