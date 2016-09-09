#version 330

layout(points) in;
layout(triangle_strip, max_vertices=4) out;

in CharData {
    vec2 position;
    vec2 tex_upleft;
    vec2 tex_lowright;
    vec2 size;    
} char_data[];

out FragVert {
    vec2 tex_coord;
};

void main() {
    vec2 position = char_data[0].position;
    vec2 size = char_data[0].size;

    // Upper-right vertex
    tex_coord = vec2(char_data[0].tex_lowright.x, char_data[0].tex_upleft.y);
    gl_Position = vec4(position + 0.5 * size, 0.0, 1.0);
    gl_PrimitiveID = gl_PrimitiveIDIn;
    EmitVertex();

    // Upper-left vertex
    tex_coord = char_data[0].tex_upleft;
    gl_Position = vec4(position + 0.5 * vec2(-size.x, size.y), 0.0, 1.0);
    gl_PrimitiveID = gl_PrimitiveIDIn;
    EmitVertex();

    // Lower-right vertex
    tex_coord = char_data[0].tex_lowright;
    gl_Position = vec4(position + 0.5 * vec2(size.x, -size.y), 0.0, 1.0);
    gl_PrimitiveID = gl_PrimitiveIDIn;
    EmitVertex();

    // Lower-left vertex
    tex_coord = vec2(char_data[0].tex_upleft.x, char_data[0].tex_lowright.y);
    gl_Position = vec4(position + 0.5 * -size, 0.0, 1.0);
    gl_PrimitiveID = gl_PrimitiveIDIn;
    EmitVertex();
}