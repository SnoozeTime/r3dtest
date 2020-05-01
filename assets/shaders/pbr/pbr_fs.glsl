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
uniform vec3 u_AmbientLightColor;
uniform float u_AmbientLightIntensity;

const float PI = 3.14159265359;


vec3[4] lights = vec3[](
vec3(1., 1.0, 0.),
vec3(-1., 1.0, 0.),
vec3(0., 4.0, 0.),
vec3(0., 1.0, 1.)
);

mat3 inverse3x3(mat3 M) {
    // The original was written in HLSL, but this is GLSL,
    // therefore
    // - the array index selects columns, so M_t[0] is the
    // first row of M, etc.
    // - the mat3 constructor assembles columns, so
    // cross( M_t[1], M_t[2] ) becomes the first column
    // of the adjugate, etc.
    // - for the determinant, it does not matter whether it is
    // computed with M or with M_t; but using M_t makes it
    // easier to follow the derivation in the text
    mat3 M_t = transpose(M);
    float det = dot(cross(M_t[0], M_t[1]), M_t[2]);
    mat3 adjugate = mat3(cross(M_t[1], M_t[2]), cross(M_t[2], M_t[0]), cross(M_t[0], M_t[1]));
    return adjugate / det;
}


mat3 cotangent_frame(vec3 N, vec3 p, vec2 uv) {
    // get edge vectors of the pixel triangle
    vec3 dp1 = dFdx(p);
    vec3 dp2 = dFdy(p);
    vec2 duv1 = dFdx(uv);
    vec2 duv2 = dFdy(uv);
    // solve the linear system
    vec3 dp2perp = cross(dp2, N);
    vec3 dp1perp = cross(N, dp1);
    vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;
    // construct a scale-invariant frame
    float invmax = inversesqrt(max(dot(T, T), dot(B, B)));
    return mat3(T * invmax, B * invmax, N);
}


// ----------------------------------------------------------------------------
// Easy trick to get tangent-normals to world-space to keep PBR code simplified.
// Don't worry if you don't get what's going on; you generally want to do normal
// mapping the usual way for performance anways; I do plan make a note of this
// technique somewhere later in the normal mapping tutorial.
vec3 getNormal() {
    #ifdef HAS_NORMAL_TEXTURE
    vec2 uv = v_UV[u_NormalTexCoord];
    vec3 map = texture2D( u_NormalSampler, uv ).xyz;

//    vec3 tangentNormal = texture(u_NormalSampler, uv).xyz * 2.0 - 1.0;
//
//    vec3 Q1  = dFdx(v_Position);
//    vec3 Q2  = dFdy(v_Position);
//    vec2 st1 = dFdx(v_UV[0]);
//    vec2 st2 = dFdy(v_UV[0]);
//
   vec3 N   = normalize(v_Normal);
    vec3 V = normalize(u_Camera - v_Position);

    //    vec3 T  = normalize(Q1*st2.t - Q2*st1.t);
//    vec3 B  = -normalize(cross(N, T));
    //mat3 TBN = mat3(T, B, N);
    mat3 TBN  = cotangent_frame( N, -V, uv );
    vec3 n = normalize( TBN * map ); //normalize(TBN * tangentNormal * vec3(u_NormalScale, u_NormalScale, 1.0));
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

    return nom / max(denom, 0.001);// prevent divide by zero for roughness=0.0 and NdotH=1.0
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
    vec3 N = v_Normal;//getNormal();
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
    //vec3 Lo = vec3(0.0);
    // For each light, we want to calculate the Cook-Torrance specular BRDF
    // for (int i = 0; i < 1; i++) {
    // DIRECTIONAL LIGHT
    //    vec3 L = normalize(lightPositions[i] - WorldPos);
    //    vec3 H = normalize(V + L);
    vec3 L = normalize(u_LightDirection - v_Position);
    vec3 H = normalize(V + L);
    //float distance = length(u_LightDirection - v_Position);
    //float attenuation = 1.0 / (distance * distance);
    vec3 radiance = u_LightColor;// * attenuation;

    // Cook-Torrance BRDF
    float NDF = DistributionGGX(N, H, roughness);
    float G   = GeometrySmith(N, V, L, roughness);
    vec3 F    = fresnelSchlick(clamp(dot(H, V), 0.0, 1.0), F0);

    vec3 nominator    = NDF * G * F;
    float NdotL = clamp(dot(N, L), 0.001, 1.0);
    float NdotV = clamp(abs(dot(N, V)), 0.001, 1.0);
    float denominator = 4 * NdotL * NdotV;
    vec3 specular = nominator / max(denominator, 0.001); // prevent divide by zero for NdotV=0.0 or NdotL=0.0
    // vec3 specular = vec3(1.0) / getNormal();

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

    vec3 radiance_out = radiance * NdotL;
    vec3 albedo_out = (kD * albedo / PI)* radiance_out;
    //vec3 specular_out = nominator;
    vec3 specular_out = specular * radiance_out;
    // add to outgoing radiance Lo
    vec3 Lo = specular_out + albedo_out;// note that we already multiplied the BRDF by the Fresnel (kS) so we won't multiply by kS again
    // }

    // stop loop here.
    vec3 ambient = u_AmbientLightColor * vec3(u_AmbientLightIntensity) * albedo;
    vec3 color   =  Lo + ambient;
    color = color / (color + vec3(1.0));
    //fragColor = vec4(ambient + specular_out, 1.0);
    //fragColor = vec4(NdotV);
    fragColor = vec4(pow(color, vec3(1.0/2.2)), 1.0);
}
