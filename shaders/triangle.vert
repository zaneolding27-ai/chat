#version 450

layout(location = 0) out vec3 color;

layout(push_constant) uniform Camera {
    vec4 position_yaw;
    vec4 pitch;
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
    vec3 relative = vec3(positions[gl_VertexIndex], 0.0) - camera.position_yaw.xyz;
    float yaw = camera.position_yaw.w;
    float yaw_cos = cos(yaw);
    float yaw_sin = sin(yaw);
    relative = vec3(
        relative.x * yaw_cos - relative.z * yaw_sin,
        relative.y,
        relative.x * yaw_sin + relative.z * yaw_cos
    );

    float pitch_angle = camera.pitch.x;
    float pitch_cos = cos(pitch_angle);
    float pitch_sin = sin(pitch_angle);
    relative = vec3(
        relative.x,
        relative.y * pitch_cos - relative.z * pitch_sin,
        relative.y * pitch_sin + relative.z * pitch_cos
    );

    float perspective = max(-relative.z, 0.1);
    gl_Position = vec4(relative.xy / perspective, 0.0, 1.0);
    color = colors[gl_VertexIndex];
}
