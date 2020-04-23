in vec3 position;
in vec3 normal;
in vec4 tangent;
in vec2 tex_coord_0;
in vec2 tex_coord_1;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
uniform vec3 color;
uniform vec3 emissive;

out vec3 v_color;
out vec3 v_color_emissive;
out vec3 v_normal;
out vec3 v_pos;

void main() {
    v_color = color;
    v_normal = normal;
    v_color_emissive = emissive;
    v_pos = (model * vec4(position, 1.)).xyz;
    gl_Position = projection * view * model * vec4(position, 1.);
}