in vec3 v_color;
in vec3 v_normal;
out vec3 frag_color;

void main() {

    vec3 lightColor = vec3(1.0, 1.0, 1.0);
    vec3 ambient = 0.3 * lightColor;

    // light direction
    vec3 light_pos = vec3(1., -1., -1.);
    vec3 norm = normalize(v_normal);
    vec3 light_dir = normalize(-light_pos);

    float diff = max(dot(norm, light_dir), 0.0);
    vec3 diffuse = diff * lightColor;
    frag_color = v_color * (ambient + diffuse);
}