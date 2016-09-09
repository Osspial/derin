#version 330

in FragVert {
    vec2 tex_coord;
};

uniform sampler2D tex;

out vec4 color;

void main() {
    float alpha = texture(tex, tex_coord).r;
    color = vec4(1.0, 1.0, 1.0, alpha);
}