#version 330

layout (location = 0) in vec2 rel;
layout (location = 1) in vec2 pts;
layout (location = 2) in vec2 normal;
layout (location = 3) in vec4 color;

out vec4 vert_color;

void main() {
    gl_Position = vec4(rel, 0.0, 1.0);
    vert_color = color;
}
