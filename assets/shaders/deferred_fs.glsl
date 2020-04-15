in vec3 v_color;
in vec3 v_normal;

layout (location = 0) out vec3 frag_color;
layout (location = 1) out vec3 frag_normal;

void main() {
    frag_color = v_color;
    frag_normal = v_normal;
}