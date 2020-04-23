in vec3 position;
in vec3 normal;
in vec2 tex_coord_0;
in vec2 tex_coord_1;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

out vec3 v_normal;
out vec3 v_pos;
out vec2[2] v_tex_coords;


void main() {
    v_normal = normal;
    v_pos = (model * vec4(position, 1.)).xyz;
    gl_Position = projection * view * model * vec4(position, 1.);
    v_tex_coords[0] = tex_coord_0;
    v_tex_coords[1] = tex_coord_1;
}
