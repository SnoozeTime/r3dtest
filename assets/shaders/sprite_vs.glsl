
const vec2[4] QUAD_POS = vec2[](
vec2(-1., -1.),
vec2( 1., -1.),
vec2( 1.,  1.),
vec2(-1.,  1.)
);

uniform mat4 projection;
uniform mat4 model;

out vec2 v_uv;

void main() {
    vec2 p = QUAD_POS[gl_VertexID];
    v_uv = p * .5 + .5; // transform the position of the vertex into UV space
    gl_Position = projection * model * vec4(p, 1.0, 1.0);
}