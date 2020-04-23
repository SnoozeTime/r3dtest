in vec2 v_uv;
out vec4 frag;

uniform sampler2D lighting_map;
uniform sampler2D albedo_map;

void main() {
    // stop loop here.
    vec3 ambient = vec3(0.03) * texture(albedo_map, v_uv).rgb;
    vec3 color   = ambient + texture(lighting_map, v_uv).rgb;;
    color = color / (color + vec3(1.0));
    frag = vec4(pow(color, vec3(1.0/2.2)), 1.0);
}
