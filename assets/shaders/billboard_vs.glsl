#define M_PI 3.1415926535897932384626433832795

const vec2[4] QUAD_POS = vec2[](
    vec2(-1., -1.),
    vec2( 1., -1.),
    vec2( 1.,  1.),
    vec2(-1.,  1.)
);

uniform vec3 camera_position;
uniform vec3 center;

uniform mat4 projection;
uniform mat4 model;
// camera view. Should always face this one :)
uniform mat4 view;
// sprite coordinates.
uniform vec2 spritesheet_dimensions;
uniform vec4 sprite_coord;

out vec2 v_uv;

mat4 rotationY(float angle) {
    return mat4(
        cos(angle), 0., sin(angle), 0.,
        0., 1., 0., 0.,
        -sin(angle), 0., cos(angle), 0.,
        0., 0., 0., 1.
    );
}

void main() {
    vec2 p = QUAD_POS[gl_VertexID];
    vec2 uv = p * .5 + .5; // transform the position of the vertex into UV space
    float u = sprite_coord.x / spritesheet_dimensions.x + uv.x * sprite_coord.z / spritesheet_dimensions.x;
    float v = sprite_coord.y / spritesheet_dimensions.y + uv.y * sprite_coord.w / spritesheet_dimensions.y;
    v_uv = vec2(u, v);

    // front vector for the quad is (0, 0, 1)
    vec3 obj_to_camera = normalize(vec3(camera_position.x, 0.0, camera_position.z) - center);
    vec3 billboard_dir = vec3(0., 0., 1.);

    // Unsigned angle.
    float cos_angle = dot(billboard_dir, obj_to_camera);
    float angle = acos(cos_angle);

    // sign between the two vectors.
    vec3 s = sign(cross(billboard_dir, obj_to_camera));

    // angle between 0 and 2pi.
    float theta = mod(angle * -s.y, 2*M_PI);

    //
    mat4 rotation = rotationY(theta);

    gl_Position = projection * view *  model * rotation * vec4(p, 1.0, 1.0);
}