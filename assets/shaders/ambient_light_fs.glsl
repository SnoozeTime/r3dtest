in vec2 v_uv;
out vec4 frag;

uniform sampler2D diffuse;
uniform vec3 color;
uniform float intensity;

void main() {
    vec3 diff = texture(diffuse, v_uv).rgb;
    vec3 ambient_color = intensity  * color;
    frag = vec4(ambient_color * diff, 1.0);
}