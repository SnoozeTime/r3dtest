// Originally taken from https://github.com/KhronosGroup/glTF-WebGL-PBR
// Commit a94655275e5e4e8ae580b1d95ce678b74ab87426
in vec3 position;
in vec3 normal;
in vec2 tex_coord_0;
in vec2 tex_coord_1;
in vec4 color;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

out vec3 v_Position;
out vec2 v_UV[2];
out vec3 v_Normal;
out vec4 v_Color;

void main()
{
    vec4 pos =  model * vec4(position, 1.);
    v_Position = vec3(pos.xyz) / pos.w;
    v_Normal = normalize(vec3(model * vec4(normal.xyz, 0.0)));
    v_UV[0] = tex_coord_0;
    v_UV[1] = tex_coord_1;
    gl_Position = projection * view  * pos; // needs w for proper perspective correction
    v_Color = color;
}

