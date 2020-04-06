in vec3 position;
in vec3 normal;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
out vec3 v_normal;

void main() {
    v_normal = normal;
    gl_Position = projection * view * model * vec4(position, 1.);
}