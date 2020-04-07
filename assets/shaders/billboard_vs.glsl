
const vec2[4] QUAD_POS = vec2[](
    vec2(-1., -1.),
    vec2( 1., -1.),
    vec2( 1.,  1.),
    vec2(-1.,  1.)
);

uniform mat4 projection;
uniform mat4 model;
// camera view. Should always face this one :)
uniform mat4 view;

out vec2 v_uv;

void main() {
    vec2 p = QUAD_POS[gl_VertexID];
//    vec2 uv = p * .5 + .5; // transform the position of the vertex into UV space
//    float u = sprite_coord.x / spritesheet_dimensions.x + uv.x * sprite_coord.z / spritesheet_dimensions.x;
//    float v = sprite_coord.y / spritesheet_dimensions.y + uv.y * sprite_coord.w / spritesheet_dimensions.y;
//    v_uv = vec2(u, v);
    gl_Position = projection * view * model * vec4(p, 1.0, 1.0);
}