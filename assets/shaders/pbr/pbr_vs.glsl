in vec3 position;
in vec3 normal;
in vec4 tangent;
in vec2 tex_coord_0;
in vec2 tex_coord_1;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

out vec3 v_Position;
out vec2 v_UV[2];
out vec3 v_Normal;

void main()
{
    v_UV[0] = tex_coord_0;
    v_UV[1] = tex_coord_1;
    // v_Position = vec3(position.xyz) / position.w;

    v_Position = vec3(model * vec4(position, 1.0));
    v_Normal = normalize(vec3(model * vec4(normal.xyz, 0.0)));

    gl_Position =  projection * view * vec4(v_Position, 1.0);
}