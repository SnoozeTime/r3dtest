in vec2 v_uv;

uniform vec3 color;
uniform sampler2D depth_buffer;

out vec3 frag_color;

void main() {
    float depth = texture(depth_buffer, v_uv).r;
    if (depth <= 0.999999) {
        discard;
    }
    frag_color = color;
}
