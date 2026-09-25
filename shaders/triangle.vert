#version 450

layout(location = 0) out vec3 color;

layout(push_constant) uniform Camera {
    vec4 camera_position_yaw;
    vec4 camera_pitch_entity_scale;
    vec4 entity_position_rotation;
    vec4 entity_color;
} camera;

vec2 positions[3] = vec2[](
    vec2(0.0, -0.65),
    vec2(0.65, 0.65),
    vec2(-0.65, 0.65)
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.1, 0.1),
    vec3(0.1, 1.0, 0.2),
    vec3(0.2, 0.4, 1.0)
);

void main() {
    float scale = camera.camera_pitch_entity_scale.y;
    vec2 local_position = positions[gl_VertexIndex] * scale;
    float entity_rotation = camera.entity_position_rotation.w;
    float entity_cos = cos(entity_rotation);
    float entity_sin = sin(entity_rotation);
    vec3 entity_position = vec3(
        local_position.x * entity_cos - local_position.y * entity_sin,
        local_position.x * entity_sin + local_position.y * entity_cos,
        0.0
    ) + camera.entity_position_rotation.xyz;

    vec3 relative = entity_position - camera.camera_position_yaw.xyz;
    float yaw = camera.camera_position_yaw.w;
    float yaw_cos = cos(yaw);
    float yaw_sin = sin(yaw);
    relative = vec3(
        relative.x * yaw_cos - relative.z * yaw_sin,
        relative.y,
        relative.x * yaw_sin + relative.z * yaw_cos
    );

    float pitch_angle = camera.camera_pitch_entity_scale.x;
    float pitch_cos = cos(pitch_angle);
    float pitch_sin = sin(pitch_angle);
    relative = vec3(
        relative.x,
        relative.y * pitch_cos - relative.z * pitch_sin,
        relative.y * pitch_sin + relative.z * pitch_cos
    );

    float perspective = max(-relative.z, 0.1);
    gl_Position = vec4(relative.xy / perspective, 0.0, 1.0);
    color = camera.entity_color.xyz * colors[gl_VertexIndex];
}
