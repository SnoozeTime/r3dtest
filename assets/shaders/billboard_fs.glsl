in vec2 v_uv;
out vec4 frag;

uniform sampler2D tex;

void main() {
    vec4 texture = texture(tex, v_uv);
    if ( texture.a < 0.1 ) discard;
    frag = texture;
}