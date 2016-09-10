#version 330

uniform vec2 base_location;
uniform vec2 viewport_size_px;

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
    char_data.position = base_location + offset / (viewport_size_px / 2.0);
    char_data.tex_upleft = tex_upleft;
    char_data.tex_lowright = tex_lowright;
    char_data.size = size / (viewport_size_px / 2.0);
}
