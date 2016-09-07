#version 330

in FragVert {
    vec2 tex_coord;
};

uniform sampler2D tex;

out vec4 color;

void main() {
    color = texture(tex, tex_coord);
}