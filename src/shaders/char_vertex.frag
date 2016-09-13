#version 330

in FragVert {
    vec2 tex_coord;
    vec4 char_color;
};

uniform sampler2D tex;

out vec4 frag_color;

void main() {
    float alpha = texture(tex, tex_coord).r;
    frag_color = char_color * vec4(1.0, 1.0, 1.0, alpha);
}