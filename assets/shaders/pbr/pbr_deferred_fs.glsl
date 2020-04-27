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

#ifdef HAS_NORMAL_TEXTURE
uniform sampler2D normal_texture;
uniform int normal_texture_coord_set;
uniform float normal_scale;
#endif

#ifdef HAS_ROUGHNESS_METALLIC_MAP
uniform sampler2D roughness_metallic_texture;
uniform int roughness_metallic_texture_coord_set;
#endif

layout (location = 0) out vec3 frag_albedo;
// roughness, metal, ao
layout (location = 1) out vec3 frag_material;
layout (location = 2) out vec3 frag_normal;
layout (location = 3) out vec3 frag_pos;

//// Find the normal for this fragment, pulling either from a predefined normal map
//// or from the interpolated mesh normal and tangent attributes.
vec3 getNormal()
{
    // Retrieve the tangent space matrix

    vec3 pos_dx = dFdx(v_pos);
    vec3 pos_dy = dFdy(v_pos);
    vec3 tex_dx = dFdx(vec3(v_tex_coords[0], 0.0));
    vec3 tex_dy = dFdy(vec3(v_tex_coords[0], 0.0));
    vec3 t = (tex_dy.t * pos_dx - tex_dx.t * pos_dy) / (tex_dx.s * tex_dy.t - tex_dy.s * tex_dx.t);

    vec3 ng = normalize(v_normal);


    t = normalize(t - ng * dot(ng, t));
    vec3 b = normalize(cross(ng, t));
    mat3 tbn = mat3(t, b, ng);

    #ifdef HAS_NORMAL_TEXTURE
        vec2 normal_uv = v_tex_coords[normal_texture_coord_set];
        vec3 n = texture(normal_texture, normal_uv).rgb;
        n = normalize(tbn * ((2.0 * n - 1.0) * vec3(normal_scale, normal_scale, 1.0)));
    #else
        // The tbn matrix is linearly interpolated, so we need to re-normalize
        vec3 n = normalize(tbn[2].xyz);
    #endif

    // reverse backface normals
    // TODO!: correct/best place? -> https://github.com/KhronosGroup/glTF-WebGL-PBR/issues/51
    n *= (2.0 * float(gl_FrontFacing) - 1.0);

    return n;
}

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

    frag_normal = getNormal();

    frag_pos = v_pos;

    #ifdef HAS_ROUGHNESS_METALLIC_MAP
    float roughness_out = roughness * texture(roughness_metallic_texture, v_tex_coords[roughness_metallic_texture_coord_set]).g;
    float metallic_out = metallic * texture(roughness_metallic_texture, v_tex_coords[roughness_metallic_texture_coord_set]).b;
    frag_material = vec3(roughness_out, metallic_out, ao);
    #else
    frag_material = vec3(roughness, metallic, ao);
    #endif
}