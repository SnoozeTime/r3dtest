//! PBR materials are using the same shaders with a bit of difference. The same
//! Shader file is used, but depending whether a material has some texture, different
//! part of the shader will be used. This is done by using defines in the shader files.

use crate::render::mesh::PbrShaderInterface;
use luminance::shader::program::Program;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::sync::mpsc::Receiver;

bitflags! {
    /// Attached to material to help choosing the shader to use.
    pub struct ShaderFlags: u32 {
        const HAS_COLOR_TEXTURE = 0b0000001;
        const HAS_NORMAL_TEXTURE = 0b0000010;
        const HAS_ROUGHNESS_METALLIC_MAP = 0b0000100;
    }
}

impl Default for ShaderFlags {
    fn default() -> Self {
        Self::empty()
    }
}

impl ShaderFlags {
    /// Will give a list of defines to add at the top of the shaders.
    pub fn to_defines(&self) -> Vec<String> {
        let mut defines = vec![];

        if self.contains(ShaderFlags::HAS_COLOR_TEXTURE) {
            defines.push("HAS_COLOR_TEXTURE".to_string());
        }

        if self.contains(ShaderFlags::HAS_NORMAL_TEXTURE) {
            defines.push("HAS_NORMAL_TEXTURE".to_string());
        }

        if self.contains(ShaderFlags::HAS_ROUGHNESS_METALLIC_MAP) {
            defines.push("HAS_ROUGHNESS_METALLIC_MAP".to_string());
        }

        defines
    }
}

/// Will store all the shaders for the PBR rendering. There can be in total
/// 2^(ShaderFlags variants) number of shaders.
pub struct PbrShaders {
    pub shaders: HashMap<ShaderFlags, Program<super::VertexSemantics, (), PbrShaderInterface>>,

    rx: Receiver<Result<notify::Event, notify::Error>>,
    _watcher: RecommendedWatcher,
}

fn get_program_path(program_name: &str) -> String {
    format!("{}{}", std::env::var("ASSET_PATH").unwrap(), program_name)
}

impl PbrShaders {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher: RecommendedWatcher =
            Watcher::new_immediate(move |res| tx.send(res).unwrap()).unwrap();

        watcher
            .watch(get_program_path("shaders/pbr"), RecursiveMode::Recursive)
            .unwrap();

        Self {
            shaders: HashMap::default(),
            rx,
            _watcher: watcher,
        }
    }
    /// Will compile the shaders with the given flags and store it. If it already exists, this
    /// is a no-op
    pub fn add_shader(&mut self, flags: ShaderFlags) {
        if self.shaders.contains_key(&flags) {
            return;
        } else {
            let shader = PbrShaders::load_with_defines(flags.to_defines());
            self.shaders.insert(flags, shader);
        }
    }

    fn load_with_defines(
        defines: Vec<String>,
    ) -> Program<super::VertexSemantics, (), PbrShaderInterface> {
        let vs =
            fs::read_to_string(std::env::var("ASSET_PATH").unwrap() + "shaders/pbr/pbr_vs.glsl")
                .expect("Could not load the PBR vertex shader");
        let fs =
            fs::read_to_string(std::env::var("ASSET_PATH").unwrap() + "shaders/pbr/pbr_fs.glsl")
                .expect("Could not load the PBR fragment shader");

        let mut final_fs = String::new();
        for d in defines {
            final_fs.push_str("#define ");
            final_fs.push_str(&d);
            final_fs.push_str("\n");
        }
        final_fs.push_str(&fs);

        Program::from_strings(None, &vs, None, &final_fs)
            .unwrap()
            .ignore_warnings()
    }
    pub fn reload(&mut self) {
        for (k, v) in &mut self.shaders {
            let new_shader = PbrShaders::load_with_defines(k.to_defines());
            *v = new_shader;
        }
    }

    pub fn update(&mut self) {
        let mut should_reload = false;
        for res in &self.rx.try_recv() {
            match res {
                Ok(Event {
                    kind: EventKind::Modify(..),
                    ..
                }) => should_reload = true,
                _ => (),
            }
        }

        if should_reload {
            self.reload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
