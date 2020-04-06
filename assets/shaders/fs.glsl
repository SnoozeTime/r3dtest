in vec3 v_normal;
out vec3 frag_color;

void main() {
    // object color
    vec3 obj_color = vec3(.6, .6, .6);

    // light direction
    vec3 light_dir = vec3(0., -1., -.5);

    // diffusion factor (hence the k)
    float kd = dot(v_normal, -light_dir);

    frag_color = obj_color * kd;
}