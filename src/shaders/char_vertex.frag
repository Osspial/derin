#version 330

in FragVert {
    vec2 tex_coord;
};

// uniform sampler2D tex;

out vec4 color;

void main() {
    color = vec4(0.0, 0.0, 1.0, 1.0);
    // color = texture(tex, tex_coord);
}