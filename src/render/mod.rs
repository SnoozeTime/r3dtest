use luminance::context::GraphicsContext;
use luminance::tess::{Mode, Tess, TessBuilder, TessError, TessSliceIndex};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::Surface;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use try_guard::verify;
use wavefront_obj::obj;

#[allow(unused_imports)]
use log::{debug, error, info};
use luminance::pipeline::PipelineState;
use luminance::render_state::RenderState;
use serde_derive::{Deserialize, Serialize};

pub mod shaders;
pub mod sprite;
pub mod text;

use crate::camera::Camera;
use crate::colors::RgbColor;
use crate::ecs::Transform;
use crate::gameplay::player::{MainPlayer, Player};
use crate::net::snapshot::Deltable;
use crate::render::shaders::Shaders;
use crate::render::sprite::SpriteRenderer;
use hecs::World;
use luminance::framebuffer::Framebuffer;
use luminance::texture::Dim2;
use luminance_glfw::GlfwSurface;

/// What mesh to use. with what kind of rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Render {
    pub mesh: String,
}

impl Render {
    pub fn compute_delta(&self, old: &Render) -> Option<String> {
        return if old.mesh == self.mesh {
            None
        } else {
            Some(self.mesh.clone())
        };
    }

    pub fn compute_delta_from_empty(&self) -> Option<String> {
        Some(self.mesh.clone())
    }
}

impl Deltable for Render {
    type Delta = String;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self.mesh == old.mesh {
            None
        } else {
            Some(self.mesh.clone())
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(self.mesh.clone())
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.mesh = delta.clone();
    }

    fn new_component(delta: &Self::Delta) -> Self {
        Render {
            mesh: delta.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSementics {
    #[sem(name = "position", repr = "[f32; 3]", wrapper = "VertexPosition")]
    Position,

    #[sem(name = "normal", repr = "[f32; 3]", wrapper = "VertexNormal")]
    Normal,
}
#[allow(dead_code)]
#[derive(Vertex, Debug)]
#[vertex(sem = "VertexSementics")]
pub struct Vertex {
    position: VertexPosition,
    normal: VertexNormal,
}

type VertexIndex = u32;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Error while loading obj file = {0}")]
    LoadObjError(String),
}
#[derive(Debug)]
pub struct Obj {
    vertices: Vec<Vertex>,
    indices: Vec<VertexIndex>,
}

impl Obj {
    pub fn to_tess<C>(self, ctx: &mut C) -> Result<Tess, TessError>
    where
        C: GraphicsContext,
    {
        TessBuilder::new(ctx)
            .set_mode(Mode::Triangle)
            .add_vertices(self.vertices)
            .set_indices(self.indices)
            .build()
    }

    pub fn load<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let file_content = {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            content
        };

        let obj_set =
            obj::parse(file_content).map_err(|e| Error::LoadObjError(format!("{:?}", e)))?;
        let objects = obj_set.objects;

        info!("{} objects", objects.len());
        for obj in &objects {
            info!("name -> {}, geom -> {}", obj.name, obj.geometry.len());
        }
        verify!(objects.len() >= 1).ok_or(Error::LoadObjError(
            "expecting at least one object".to_owned(),
        ))?;

        let object = objects.into_iter().next().unwrap();
        verify!(object.geometry.len() == 1).ok_or(Error::LoadObjError(
            "expecting a single geometry".to_owned(),
        ))?;

        let geometry = object.geometry.into_iter().next().unwrap();
        info!("Loading {}", object.name);
        info!("{} vertices", object.vertices.len());
        info!("{} shapes", geometry.shapes.len());

        // build up vertices; for this to work, we remove duplicated vertices by putting them in a
        // map associating the vertex with its ID
        let mut vertex_cache: HashMap<obj::VTNIndex, VertexIndex> = HashMap::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<VertexIndex> = Vec::new();

        for shape in geometry.shapes {
            if let obj::Primitive::Triangle(a, b, c) = shape.primitive {
                for key in &[a, b, c] {
                    if let Some(vertex_index) = vertex_cache.get(key) {
                        indices.push(*vertex_index);
                    } else {
                        let p = object.vertices[key.0];
                        let position = VertexPosition::new([p.x as f32, p.y as f32, p.z as f32]);
                        let n = object.normals[key.2.ok_or(Error::LoadObjError(
                            "Missing normal for a vertex".to_owned(),
                        ))?];
                        let normal = VertexNormal::new([n.x as f32, n.y as f32, n.z as f32]);
                        let vertex = Vertex { position, normal };
                        let vertex_index = vertices.len() as VertexIndex;

                        vertex_cache.insert(*key, vertex_index);
                        vertices.push(vertex);
                        indices.push(vertex_index);
                    }
                }
            } else {
                return Err(Error::LoadObjError(
                    "unsupported non-triangle shape".to_owned(),
                ));
            }
        }

        Ok(Obj { vertices, indices })
    }
}

fn load_models<P: AsRef<Path>, S: GraphicsContext>(
    surface: &mut S,
    models: &[P],
) -> HashMap<String, Tess> {
    let mut cache = HashMap::new();

    for model in models {
        let model_path: &Path = model.as_ref();
        let filename = model_path.file_stem().unwrap().to_str().unwrap().to_owned();
        info!("Will load mesh {} at {}", filename, model_path.display());

        let mesh = Obj::load(model).unwrap();
        let mesh = mesh.to_tess(surface).unwrap();
        cache.insert(filename, mesh);
    }

    cache
}

const FOVY: f32 = std::f32::consts::PI / 2.;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100.;

pub struct Renderer {
    sprite_renderer: SpriteRenderer,
    backbuffer: Framebuffer<Dim2, (), ()>,
    tess_cache: HashMap<String, Tess>,
    shaders: Shaders,

    projection: glam::Mat4,
    view: glam::Mat4,
}

impl Renderer {
    pub fn new(surface: &mut GlfwSurface) -> Self {
        let sprite_renderer = SpriteRenderer::new(surface);
        let backbuffer = surface.back_buffer().unwrap();

        let tess_cache = load_models(
            surface,
            &["models/monkey.obj", "models/axis.obj", "models/cube.obj"],
        );
        let shaders = Shaders::new();

        let projection = glam::Mat4::perspective_rh_gl(
            FOVY,
            surface.width() as f32 / surface.height() as f32,
            Z_NEAR,
            Z_FAR,
        );

        Self {
            sprite_renderer,
            backbuffer,
            tess_cache,
            shaders,
            projection,
            view: glam::Mat4::identity(),
        }
    }

    pub fn update_view_matrix(&mut self, world: &World) {
        for (_, (t, c)) in world.query::<(&Transform, &Camera)>().iter() {
            if c.active {
                self.view = c.get_view(t.translation);
            }
        }
    }

    pub fn render(&mut self, surface: &mut GlfwSurface, world: &World) {
        self.shaders.update();

        let color = [0.95, 0.95, 0.95, 1.];
        surface.pipeline_builder().pipeline(
            &self.backbuffer,
            &PipelineState::default().set_clear_color(color),
            |pipeline, mut shd_gate| {
                shd_gate.shade(&self.shaders.regular_program, |iface, mut rdr_gate| {
                    iface.projection.update(self.projection.to_cols_array_2d());
                    iface.view.update(self.view.to_cols_array_2d());
                    for (e, (transform, mesh_name, color)) in
                        world.query::<(&Transform, &Render, &RgbColor)>().iter()
                    {
                        if let Ok(_) = world.get::<MainPlayer>(e) {
                            continue; // do not render yourself.
                        }
                        rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                            iface.model.update(transform.to_model().to_cols_array_2d());
                            iface.color.update(color.to_normalized());
                            let mesh = self.tess_cache.get(&mesh_name.mesh).unwrap();
                            tess_gate.render(mesh.slice(..));
                        });
                    }
                });

                self.sprite_renderer
                    .render(&pipeline, &mut shd_gate, world, &self.shaders);
            },
        );
        // swap buffer chain
        surface.swap_buffers();
    }
}
