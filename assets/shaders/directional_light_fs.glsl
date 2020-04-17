in vec2 v_uv;

// These are the diffuse, normals and depth that we have renderer to some buffers the
// previous render subpass.
uniform sampler2D diffuse;
uniform sampler2D normal;
uniform sampler2D depth;

// For the directional light
uniform vec3 direction;
uniform vec3 color;
uniform float intensity;

out vec4 f_color;

void main() {
    vec3 norm = normalize(texture(normal, v_uv).rgb);
    vec3 lightDir = normalize(direction.xyz);
    float diff = max(dot(norm, lightDir), 0.0);

    vec3 diffuse = intensity * diff * color * texture(diffuse, v_uv).rgb;
    f_color = vec4(diffuse, 1.0);
}