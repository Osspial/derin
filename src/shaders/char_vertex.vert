#version 330

uniform mat3 transform_matrix;
uniform vec2 pts_rat_scale;
uniform vec2 base_location;

layout (location = 0) in vec2 tex_upleft;
layout (location = 1) in vec2 tex_lowright;
layout (location = 2) in vec2 offset;
layout (location = 3) in vec2 size;

out CharData {
    vec2 position;
    vec2 tex_upleft;
    vec2 tex_lowright;
    vec2 size;    
} char_data;

void main() {
    char_data.position = (transform_matrix * vec3(base_location + offset * pts_rat_scale, 1.0)).xy;
    char_data.tex_upleft = tex_upleft;
    char_data.tex_lowright = tex_lowright;
    char_data.size = (transform_matrix * vec3(size * pts_rat_scale, 0.0)).xy;
}
