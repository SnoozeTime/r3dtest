//! PBR materials are using the same shaders with a bit of difference. The same
//! Shader file is used, but depending whether a material has some texture, different
//! part of the shader will be used. This is done by using defines in the shader files.

use luminance::shader::program::Program;
use std::collections::HashMap;
use std::fs;

bitflags! {
    /// Attached to material to help choosing the shader to use.
    pub struct ShaderFlags: u32 {
        const HAS_COLOR_TEXTURE = 0b0000001;
        const HAS_NORMAL_TEXTURE = 0b0000010;
    }
}

impl ShaderFlags {
    pub fn id(&self) -> u32 {
        self.bits
    }

    /// Will give a list of defines to add at the top of the shaders.
    pub fn to_defines(&self) -> Vec<String> {
        let mut defines = vec![];

        if self.contains(ShaderFlags::HAS_COLOR_TEXTURE) {
            defines.push("HAS_COLOR_TEXTURE".to_string());
        }

        if self.contains(ShaderFlags::HAS_NORMAL_TEXTURE) {
            defines.push("HAS_NORMAL_TEXTURE".to_string());
        }

        defines
    }
}

/// Will store all the shaders for the PBR rendering. There can be in total
/// 2^(ShaderFlags variants) number of shaders.
#[derive(Default)]
pub struct PbrShaders {
    shaders: HashMap<u32, Program<super::VertexSemantics, (), super::deferred::PbrShaderInterface>>,
}

impl PbrShaders {
    /// Will compile the shaders with the given flags and store it. If it already exists, this
    /// is a no-op
    pub fn add_shader(&mut self, flags: ShaderFlags) {
        if self.shaders.contains_key(&flags.id()) {
            return;
        }
    }

    fn load_with_defines(defines: Vec<String>) {
        let vs = fs::read_to_string("shaders/pbr/pbr_deferred_vs.glsl")
            .expect("Could not load the PBR vertex shader");
        let fs = fs::read_to_string("shaders/pbr/pbr_deferred_fs.glsl")
            .expect("Could not load the PBR fragment shader");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_id() {
        let flags1 = ShaderFlags::HAS_COLOR_TEXTURE;
        let flags2 = ShaderFlags::HAS_NORMAL_TEXTURE;
        let flags3 = ShaderFlags::HAS_COLOR_TEXTURE | ShaderFlags::HAS_NORMAL_TEXTURE;

        assert_eq!(1, flags1.id());
        assert_eq!(2, flags2.id());
        assert_eq!(3, flags3.id());
    }

    #[test]
    fn to_defines() {
        let flags1 = ShaderFlags::HAS_COLOR_TEXTURE;
        let flags2 = ShaderFlags::HAS_NORMAL_TEXTURE;
        let flags3 = ShaderFlags::HAS_COLOR_TEXTURE | ShaderFlags::HAS_NORMAL_TEXTURE;

        let defines1 = flags1.to_defines();
        let defines2 = flags2.to_defines();
        let defines3 = flags3.to_defines();
        assert_eq!(1, defines1.len());
        assert_eq!(1, defines2.len());
        assert_eq!(2, defines3.len());

        assert!(defines1.contains(&"HAS_COLOR_TEXTURE".to_string()));
        assert!(defines2.contains(&"HAS_NORMAL_TEXTURE".to_string()));

        assert!(defines3.contains(&"HAS_COLOR_TEXTURE".to_string()));
        assert!(defines3.contains(&"HAS_NORMAL_TEXTURE".to_string()));
    }
}
