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
use crate::event::GameEvent;
use crate::gameplay::player::{MainPlayer, Player, PlayerState};
use crate::net::snapshot::Deltable;
use crate::render::shaders::Shaders;
use crate::render::sprite::SpriteRenderer;
use crate::render::text::TextRenderer;
use crate::resources::Resources;
use glyph_brush::{GlyphBrush, GlyphBrushBuilder};
use hecs::World;
use luminance::framebuffer::Framebuffer;
use luminance::texture::Dim2;
use luminance_glfw::GlfwSurface;
use shrev::{EventChannel, ReaderId};

const dejavu: &'static [u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// What mesh to use. with what kind of rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Render {
    pub mesh: String,
    pub enabled: bool,
}

impl Deltable for Render {
    type Delta = Render;

    fn compute_delta(&self, old: &Self) -> Option<Self::Delta> {
        if self.mesh == old.mesh && self.enabled == old.enabled {
            None
        } else {
            Some(self.clone())
        }
    }

    fn compute_complete(&self) -> Option<Self::Delta> {
        Some(self.clone())
    }

    fn apply_delta(&mut self, delta: &Self::Delta) {
        self.mesh = delta.mesh.clone();
        self.enabled = delta.enabled;
    }

    fn new_component(delta: &Self::Delta) -> Self {
        Render {
            mesh: delta.mesh.clone(),
            enabled: delta.enabled,
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
    text_renderer: TextRenderer,
    backbuffer: Framebuffer<Dim2, (), ()>,
    tess_cache: HashMap<String, Tess>,
    shaders: Shaders,

    projection: glam::Mat4,
    view: glam::Mat4,
    glyph_brush: GlyphBrush<'static, text::Instance>,

    // text updates.
    rdr_id: ReaderId<GameEvent>,
}

impl Renderer {
    pub fn new(surface: &mut GlfwSurface, resources: &mut Resources) -> Self {
        let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();

        let sprite_renderer = SpriteRenderer::new(surface);
        let text_renderer = TextRenderer::new(surface, &mut glyph_brush);
        let backbuffer = surface.back_buffer().unwrap();
        let rdr_id = {
            let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
            chan.register_reader()
        };

        let tess_cache = load_models(
            surface,
            &[
                std::env::var("ASSET_PATH").unwrap() + "models/monkey.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/axis.obj",
                std::env::var("ASSET_PATH").unwrap() + "models/cube.obj",
            ],
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
            text_renderer,
            backbuffer,
            tess_cache,
            shaders,
            projection,
            view: glam::Mat4::identity(),
            glyph_brush,
            rdr_id,
        }
    }

    pub fn update_view_matrix(&mut self, world: &World) {
        for (_, (t, c)) in world.query::<(&Transform, &Camera)>().iter() {
            if c.active {
                self.view = c.get_view(t.translation);
            }
        }
    }

    pub fn update_text(&mut self, surface: &mut GlfwSurface, world: &World) {
        self.text_renderer
            .update_text(surface, world, &mut self.glyph_brush);
    }

    pub fn check_updates(
        &mut self,
        surface: &mut GlfwSurface,
        world: &World,
        resources: &Resources,
    ) {
        let should_update = {
            let mut update = false;
            let chan = resources.fetch::<EventChannel<GameEvent>>().unwrap();
            for ev in chan.read(&mut self.rdr_id) {
                if let GameEvent::UpdateText = ev {
                    update = true;
                }
            }
            update
        };

        if should_update {
            self.update_text(surface, world);
        }
    }
    pub fn render(&mut self, surface: &mut GlfwSurface, world: &World) {
        self.shaders.update();

        let color = [0.95, 0.95, 0.95, 1.];

        // FIXME maybe not the place for that.
        let should_render_player_ui = {
            if let Some((e, (_, p))) = world.query::<(&MainPlayer, &Player)>().iter().next() {
                p.state == PlayerState::Alive
            } else {
                false
            }
        };
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
                        if !mesh_name.enabled {
                            continue;
                        }

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

                if should_render_player_ui {
                    self.sprite_renderer
                        .render(&pipeline, &mut shd_gate, world, &self.shaders);

                    self.text_renderer
                        .render(&pipeline, &mut shd_gate, &self.shaders);
                }
            },
        );
        // swap buffer chain
        surface.swap_buffers();
    }
}
