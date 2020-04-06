use super::sprite;
use crate::render::{text, VertexSementics};
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
    let vs = fs::read_to_string(vs_path).unwrap();
    let fs = fs::read_to_string(fs_path).unwrap();
    Program::from_strings(None, &vs, None, &fs)
        .unwrap()
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
}

pub struct Shaders {
    pub regular_program: Program<VertexSementics, (), AxisShaderInterface>,
    pub sprite_program: Program<sprite::VertexSementics, (), sprite::ShaderInterface>,
    pub text_program: Program<text::VertexSemantics, (), text::ShaderInterface>,
    rx: Receiver<Result<notify::Event, notify::Error>>,
    _watcher: RecommendedWatcher,
}

fn get_program_path(program_name: &str) -> String {
    format!("{}{}", std::env::var("ASSET_PATH").unwrap(), program_name)
}

impl Shaders {
    pub fn new() -> Self {
        let regular_program: Program<VertexSementics, (), AxisShaderInterface> = load_program(
            get_program_path("shaders/axis_vs.glsl"),
            get_program_path("shaders/axis_fs.glsl"),
        );

        let sprite_program = load_program(
            get_program_path("shaders/sprite_vs.glsl"),
            get_program_path("shaders/sprite_fs.glsl"),
        );
        let text_program = load_program(
            get_program_path("shaders/text_vs.glsl"),
            get_program_path("shaders/text_fs.glsl"),
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
            println!("Reload shaders");
            self.regular_program = load_program("shaders/axis_vs.glsl", "shaders/axis_fs.glsl");
            self.sprite_program = load_program("shaders/sprite_vs.glsl", "shaders/sprite_fs.glsl");
        }
    }
}
