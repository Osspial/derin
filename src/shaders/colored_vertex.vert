#version 330

uniform mat3 transform_matrix;
uniform vec2 abs_rel_scale;

layout (location = 0) in vec2 rel;
layout (location = 1) in vec2 pts;
layout (location = 2) in vec2 normal;
layout (location = 3) in vec4 color;

out vec4 vert_color;

void main() {
    vec3 pos = transform_matrix * vec3(rel + pts * abs_rel_scale, 1.0);
    gl_Position = vec4(pos.xy, 0.0, 1.0);
    vert_color = color;
}
