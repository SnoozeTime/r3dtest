in vec2 v_uv;

// These are the diffuse, normals and depth that we have renderer to some buffers the
// previous render subpass.
uniform sampler2D diffuse;
uniform sampler2D normal;
uniform sampler2D position;

uniform vec3 light_color;
uniform vec3 light_position;

out vec4 f_color;

void main() {
    vec3 norm = normalize(texture(normal, v_uv).rgb);
    vec3 lightDir = normalize(light_position.xyz - texture(position, v_uv).rgb);
    float diff = max(dot(norm, lightDir), 0.0);

    float light_distance = length(light_position.xyz - texture(position, v_uv).rgb);
    // Further decrease light_percent based on the distance with the light position.

    float attenuation = 1.0 / (1.0 + 0.01 * light_distance);
    diff *= attenuation;
    vec3 diffuse = diff * light_color * texture(diffuse, v_uv).rgb;
    f_color = vec4(diffuse, 1.0);
}