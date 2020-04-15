in vec3 v_color;
in vec3 v_normal;
out vec3 frag_color;

void main() {
    frag_color = v_normal;
}