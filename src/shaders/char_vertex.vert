#version 330

uniform vec2 base_location;
uniform vec2 pts_rat_scale;

layout (location = 0) in vec2 tex_upleft;
layout (location = 1) in vec2 tex_lowright;
layout (location = 3) in vec2 offset;
layout (location = 4) in vec2 size;

out CharData {
    vec2 position;
    vec2 tex_upleft;
    vec2 tex_lowright;
    vec2 size;    
} char_data;

void main() {
    char_data.position = base_location + offset * pts_rat_scale;
    char_data.tex_upleft = tex_upleft;
    char_data.tex_lowright = tex_lowright;
    char_data.size = size;
}
