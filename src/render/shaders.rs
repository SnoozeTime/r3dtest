use super::sprite;
use crate::render::lighting::{AmbientLightProgram, DirectionalLightProgram, PointLightProgram};
use crate::render::particle::ParticleShaderInterface;
use crate::render::skybox::SkyboxProgram;
use crate::render::{billboard, debug, text, VertexSementics};
use luminance::linear::M44;
use luminance::shader::program::{Program, Uniform, UniformInterface};
use luminance::vertex::Semantics;
use luminance_derive::UniformInterface;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::Path;
use std::sync::mpsc::Receiver;

fn load_program<P, S, U>(vs_path: P, fs_path: P) -> Program<S, (), U>
where
    P: AsRef<Path>,
    S: Semantics,
    U: UniformInterface,
{
    let vs = fs::read_to_string(vs_path.as_ref())
        .unwrap_or_else(|_| panic!("{:?}", vs_path.as_ref().display()));
    let fs = fs::read_to_string(fs_path.as_ref())
        .unwrap_or_else(|_| panic!("{:?}", fs_path.as_ref().display()));
    Program::from_strings(None, &vs, None, &fs)
        .unwrap_or_else(|e| {
            panic!(
                "Shader compilation error for {:?}/{:?} = {:?}",
                vs_path.as_ref().display(),
                fs_path.as_ref().display(),
                e
            )
        })
        .ignore_warnings()
}

#[derive(Debug, UniformInterface)]
pub struct AxisShaderInterface {
    #[uniform(unbound)]
    pub projection: Uniform<M44>,

    #[uniform(unbound)]
    pub view: Uniform<M44>,

    #[uniform(unbound)]
    pub model: Uniform<M44>,

    #[uniform(unbound)]
    pub color: Uniform<[f32; 3]>,

    #[uniform(unbound)]
    pub emissive: Uniform<[f32; 3]>,
}

pub struct Shaders {
    pub regular_program: Program<VertexSementics, (), AxisShaderInterface>,
    pub sprite_program: Program<sprite::VertexSementics, (), sprite::ShaderInterface>,
    pub text_program: Program<text::VertexSemantics, (), text::ShaderInterface>,
    pub billboard_program: Program<(), (), billboard::ShaderInterface>,
    pub debug_program: Program<debug::VertexSemantics, (), debug::ShaderInterface>,
    pub copy_program: Program<(), (), super::CopyShaderInterface>,
    pub particle_program: Program<(), (), ParticleShaderInterface>,
    pub ambient_program: AmbientLightProgram,
    pub directional_program: DirectionalLightProgram,
    pub point_light_program: PointLightProgram,
    pub skybox_program: SkyboxProgram,

    rx: Receiver<Result<notify::Event, notify::Error>>,
    _watcher: RecommendedWatcher,
}

fn get_program_path(program_name: &str) -> String {
    format!("{}{}", std::env::var("ASSET_PATH").unwrap(), program_name)
}

impl Shaders {
    pub fn new() -> Self {
        let regular_program: Program<VertexSementics, (), AxisShaderInterface> = load_program(
            get_program_path("shaders/deferred_vs.glsl"),
            get_program_path("shaders/deferred_fs.glsl"),
        );
        let sprite_program = load_program(
            get_program_path("shaders/sprite_2_vs.glsl"),
            get_program_path("shaders/sprite_fs.glsl"),
        );
        let text_program = load_program(
            get_program_path("shaders/text_vs.glsl"),
            get_program_path("shaders/text_fs.glsl"),
        );
        let billboard_program = load_program(
            get_program_path("shaders/billboard_vs.glsl"),
            get_program_path("shaders/billboard_fs.glsl"),
        );

        let debug_program = load_program(
            get_program_path("shaders/debug_vs.glsl"),
            get_program_path("shaders/debug_fs.glsl"),
        );

        let copy_program = load_program(
            get_program_path("shaders/copy-vs.glsl"),
            get_program_path("shaders/copy-fs.glsl"),
        );
        let particle_program = load_program(
            get_program_path("shaders/particle_vs.glsl"),
            get_program_path("shaders/particle_fs.glsl"),
        );
        let ambient_program = load_program(
            get_program_path("shaders/copy-vs.glsl"),
            get_program_path("shaders/ambient_light_fs.glsl"),
        );
        let directional_program = load_program(
            get_program_path("shaders/copy-vs.glsl"),
            get_program_path("shaders/directional_light_fs.glsl"),
        );
        let point_light_program = load_program(
            get_program_path("shaders/copy-vs.glsl"),
            get_program_path("shaders/point_light_fs.glsl"),
        );
        let skybox_program = load_program(
            get_program_path("shaders/copy-vs.glsl"),
            get_program_path("shaders/skybox_fs.glsl"),
        );

        let (tx, rx) = std::sync::mpsc::channel();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.

        // Automatically select the best implementation for your platform.
        // You can also access each implementation directly e.g. INotifyWatcher.
        let mut watcher: RecommendedWatcher =
            Watcher::new_immediate(move |res| tx.send(res).unwrap()).unwrap();

        watcher
            .watch(get_program_path("shaders/"), RecursiveMode::Recursive)
            .unwrap();

        Self {
            regular_program,
            sprite_program,
            text_program,
            billboard_program,
            copy_program,
            debug_program,
            particle_program,
            ambient_program,
            directional_program,
            point_light_program,
            skybox_program,
            rx,
            _watcher: watcher,
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
            self.regular_program = load_program(
                get_program_path("shaders/deferred_vs.glsl"),
                get_program_path("shaders/deferred_fs.glsl"),
            );

            self.sprite_program = load_program(
                get_program_path("shaders/sprite_2_vs.glsl"),
                get_program_path("shaders/sprite_fs.glsl"),
            );
            self.billboard_program = load_program(
                get_program_path("shaders/billboard_vs.glsl"),
                get_program_path("shaders/billboard_fs.glsl"),
            );
            self.text_program = load_program(
                get_program_path("shaders/text_vs.glsl"),
                get_program_path("shaders/text_fs.glsl"),
            );
            self.debug_program = load_program(
                get_program_path("shaders/debug_vs.glsl"),
                get_program_path("shaders/debug_fs.glsl"),
            );

            self.copy_program = load_program(
                get_program_path("shaders/copy-vs.glsl"),
                get_program_path("shaders/copy-fs.glsl"),
            );
            self.particle_program = load_program(
                get_program_path("shaders/particle_vs.glsl"),
                get_program_path("shaders/particle_fs.glsl"),
            );
            self.ambient_program = load_program(
                get_program_path("shaders/copy-vs.glsl"),
                get_program_path("shaders/ambient_light_fs.glsl"),
            );
            self.directional_program = load_program(
                get_program_path("shaders/copy-vs.glsl"),
                get_program_path("shaders/directional_light_fs.glsl"),
            );
            self.point_light_program = load_program(
                get_program_path("shaders/copy-vs.glsl"),
                get_program_path("shaders/point_light_fs.glsl"),
            );
            self.skybox_program = load_program(
                get_program_path("shaders/copy-vs.glsl"),
                get_program_path("shaders/skybox_fs.glsl"),
            );
        }
    }
}
