in vec2 v_uv;
in vec3 v_color;

out vec4 frag;


void main() {
    frag = vec4(v_color, 1.0);
}