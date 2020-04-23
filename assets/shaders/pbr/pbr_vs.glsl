in vec3 position;
in vec3 normal;
in vec4 tangent;
in vec2 tex_coord_0;
in vec2 tex_coord_1;

out vec2 v_tex_coords;
out vec3 v_world_pos;
out vec3 v_normal;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

void main()
{
    v_tex_coords = tex_coord_0;
    v_world_pos = vec3(model * vec4(position, 1.0));
    v_normal = mat3(model) * normal;

    gl_Position =  projection * view * vec4(v_world_pos, 1.0);
}