in vec3 v_color;
in vec3 v_normal;
in vec3 v_color_emissive;
in vec3 v_pos;


layout (location = 0) out vec3 frag_color;
layout (location = 1) out vec3 frag_normal;
layout (location = 2) out vec3 frag_emissive;
layout (location = 3) out vec3 frag_pos;

void main() {
    frag_color = v_color;
    frag_normal = v_normal;
    frag_emissive = v_color_emissive;
    frag_pos = v_pos;
}