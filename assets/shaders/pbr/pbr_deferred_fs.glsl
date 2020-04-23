in vec3 v_normal;
in vec3 v_pos;
in vec2[2] v_tex_coords;

uniform vec3  albedo;
uniform float metallic;
uniform float roughness;
uniform float ao;

#ifdef HAS_COLOR_TEXTURE
uniform sampler2D color_texture;
uniform int color_texture_coord_set;
#endif

layout (location = 0) out vec3 frag_albedo;
// roughness, metal, ao
layout (location = 1) out vec3 frag_material;
layout (location = 2) out vec3 frag_normal;
layout (location = 3) out vec3 frag_pos;

void main() {

    /*
        if the material contains a color texture, then we use it for the albedo. Otherwise,
        only the base color will be used.
    */
    #ifdef HAS_COLOR_TEXTURE
    vec2 uv = v_tex_coords[color_texture_coord_set];
    frag_albedo = texture(color_texture, uv).rgb;
    #else
    frag_albedo = albedo;
    #endif

    frag_normal = v_normal;
    frag_pos = v_pos;
    frag_material = vec3(roughness, metallic, ao);
}