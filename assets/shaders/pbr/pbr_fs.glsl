out vec4 fragColor;

in vec3 v_Position;
in vec2 v_UV[2];
in vec3 v_Normal;

uniform vec3 u_Camera;

// material
uniform vec3 u_BaseColorFactor;
uniform vec2 u_MetallicRoughnessValues;
uniform float ao;

#ifdef HAS_NORMAL_TEXTURE
uniform sampler2D u_NormalSampler;
uniform int u_NormalTexCoord;
uniform float u_NormalScale;
#endif

#ifdef HAS_ROUGHNESS_METALLIC_MAP
uniform sampler2D u_MetallicRoughnessSampler;
uniform int u_MetallicRoughnessTexCoord;
#endif

#ifdef HAS_METALLIC_MAP
uniform sampler2D u_MetallicSampler;
uniform int u_MetallicTexCoord;
#endif

#ifdef HAS_COLOR_TEXTURE
uniform sampler2D u_BaseColorSampler;
uniform int u_BaseColorTexCoord;
#endif

// direct lights
uniform vec3 u_LightDirection;
uniform vec3 u_LightColor;


const float PI = 3.14159265359;


vec3[4] lights = vec3[](
    vec3(1.,  1.0, 0.),
    vec3(-1.,  1.0, 0.),
    vec3(0.,  4.0, 0.),
    vec3(0.,  1.0, 1.)
);

// ----------------------------------------------------------------------------
// Easy trick to get tangent-normals to world-space to keep PBR code simplified.
// Don't worry if you don't get what's going on; you generally want to do normal
// mapping the usual way for performance anways; I do plan make a note of this
// technique somewhere later in the normal mapping tutorial.
vec3 getNormal() {
    #ifdef HAS_NORMAL_TEXTURE
        vec2 uv = v_UV[u_NormalTexCoord];
        vec3 tangentNormal = texture(u_NormalSampler, uv).xyz * 2.0 - 1.0;

        vec3 Q1  = dFdx(v_Position);
        vec3 Q2  = dFdy(v_Position);
        vec2 st1 = dFdx(v_UV[0]);
        vec2 st2 = dFdy(v_UV[0]);

        vec3 N   = normalize(v_Normal);
        vec3 T  = normalize(Q1*st2.t - Q2*st1.t);
        vec3 B  = -normalize(cross(N, T));
        mat3 TBN = mat3(T, B, N);
        vec3 n = normalize(TBN * tangentNormal);
    #else
        vec3 n = normalize(v_Normal);
    #endif
    return n;
}

/*
#ifdef HAS_ROUGHNESS_METALLIC_MAP
uniform sampler2D u_metallic_roughness_sampler;
uniform int u_metallic_roughness_tex_coord;
#endif*
*/
float getRoughness() {
    #ifdef HAS_ROUGHNESS_METALLIC_MAP
        float r = texture(u_MetallicRoughnessSampler, v_UV[u_MetallicRoughnessTexCoord]).r;
    #else
        float r = u_MetallicRoughnessValues.y;
    #endif

    return r;
}

float getMetallic() {
    #ifdef HAS_METALLIC_MAP
    float m = texture(u_MetallicSampler, v_UV[u_MetallicTexCoord]).r;
    #else
    float m = u_MetallicRoughnessValues.x;
    #endif

    return m;
}

vec3 getAlbedo() {
    #ifdef HAS_COLOR_TEXTURE
        vec3 albedo = texture(u_BaseColorSampler, v_UV[u_BaseColorTexCoord]).rgb;
    #else
        vec3 albedo = u_BaseColorFactor;
    #endif

    return albedo;
}

vec2 getRoughnessMetallic() {
    #ifdef HAS_ROUGHNESS_METALLIC_MAP
    float r = texture(u_MetallicRoughnessSampler, v_UV[u_MetallicRoughnessTexCoord]).g;
    float m = texture(u_MetallicRoughnessSampler, v_UV[u_MetallicRoughnessTexCoord]).b;
    #else
    float r = u_MetallicRoughnessValues.y;
    float m = u_MetallicRoughnessValues.x;
    #endif

    return vec2(r, m);
}

// ----------------------------------------------------------------------------
float DistributionGGX(vec3 N, vec3 H, float roughness)
{
    float a = roughness*roughness;
    float a2 = a*a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;

    float nom   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return nom / max(denom, 0.001); // prevent divide by zero for roughness=0.0 and NdotH=1.0
}
// ----------------------------------------------------------------------------
float GeometrySchlickGGX(float NdotV, float roughness)
{
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float nom   = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return nom / denom;
}
// ----------------------------------------------------------------------------
float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = GeometrySchlickGGX(NdotV, roughness);
    float ggx1 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}
// ----------------------------------------------------------------------------
vec3 fresnelSchlick(float cosTheta, vec3 F0)
{
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

void main() {
    vec3 N = getNormal();
    vec3 V = normalize(u_Camera - v_Position);


    vec3 albedo = getAlbedo();

    vec2 roughnessMetallic = getRoughnessMetallic();
    float metallic = roughnessMetallic.y;
    float roughness = roughnessMetallic.x;

    // calculate reflectance at normal incidence; if dia-electric (like plastic) use F0
    // of 0.04 and if it's a metal, use the albedo color as F0 (metallic workflow)
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);

    // reflectance equation
    vec3 Lo = vec3(0.0);
    // For each light, we want to calculate the Cook-Torrance specular BRDF
    for (int i = 0; i < 4; i++) {
        vec3 L = normalize(lights[i] - v_Position);
        vec3 H = normalize(V + L);
        float distance = length(lights[i] - v_Position);
        //float attenuation = 1.0 / (distance * distance);
        vec3 radiance = u_LightColor; // * attenuation;

        // Cook-Torrance BRDF
        float NDF = DistributionGGX(N, H, roughness);
        float G   = GeometrySmith(N, V, L, roughness);
        vec3 F    = fresnelSchlick(clamp(dot(H, V), 0.0, 1.0), F0);

        vec3 nominator    = NDF * G * F;
        float denominator = 4 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0);
        vec3 specular = nominator / max(denominator, 0.001); // prevent divide by zero for NdotV=0.0 or NdotL=0.0

        // kS is equal to Fresnel
        vec3 kS = F;
        // for energy conservation, the diffuse and specular light can't
        // be above 1.0 (unless the surface emits light); to preserve this
        // relationship the diffuse component (kD) should equal 1.0 - kS.
        vec3 kD = vec3(1.0) - kS;
        // multiply kD by the inverse metalness such that only non-metals
        // have diffuse lighting, or a linear blend if partly metal (pure metals
        // have no diffuse light).
        kD *= 1.0 - metallic;

        // scale light by NdotL
        float NdotL = max(dot(N, L), 0.0);

        // add to outgoing radiance Lo
        Lo += (kD * albedo / PI + specular) * radiance * NdotL;  // note that we already multiplied the BRDF by the Fresnel (kS) so we won't multiply by kS again
    }

    // stop loop here.
    vec3 ambient = vec3(0.03) * albedo;
    vec3 color   = ambient + Lo;
    color = color / (color + vec3(1.0));
    fragColor = vec4(pow(color, vec3(1.0/2.2)), 1.0);
}
