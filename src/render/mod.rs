use luminance::context::GraphicsContext;
use luminance::tess::{Mode, Tess, TessBuilder, TessSliceIndex};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::Surface;

#[allow(unused_imports)]
use log::{debug, error, info};
use luminance::pipeline::{BoundTexture, PipelineState};
use luminance::render_state::RenderState;
use serde_derive::{Deserialize, Serialize};

pub mod assets;
pub mod billboard;
pub mod debug;
pub mod lighting;
pub mod particle;
pub mod shaders;
pub mod sprite;
pub mod text;
use crate::camera::Camera;
use crate::colors::RgbColor;
use crate::ecs::Transform;
use crate::event::GameEvent;
use crate::gameplay::player::{MainPlayer, Player, PlayerState};
use crate::net::snapshot::Deltable;
use crate::render::assets::AssetManager;
use crate::render::billboard::BillboardRenderer;
use crate::render::debug::DebugRenderer;
use crate::render::lighting::LightingSystem;
use crate::render::particle::ParticleSystem;
use crate::render::shaders::Shaders;
use crate::render::sprite::SpriteRenderer;
use crate::render::text::TextRenderer;
use crate::resources::Resources;
use glyph_brush::{GlyphBrush, GlyphBrushBuilder};
use hecs::World;
use luminance::framebuffer::Framebuffer;
use luminance::pixel::{Depth32F, Floating, RGBA32F};
use luminance::shader::program::Uniform;
use luminance::texture::{Dim2, Sampler};
use luminance_derive::UniformInterface;
use luminance_glfw::GlfwSurface;
use shrev::{EventChannel, ReaderId};
use std::time::Duration;

const DEJA_VU: &'static [u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

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

#[derive(UniformInterface)]
pub struct CopyShaderInterface {
    pub source_texture: Uniform<&'static BoundTexture<'static, Dim2, Floating>>,
}

const FOVY: f32 = std::f32::consts::PI / 2.;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100.;

pub type OffscreenBuffer = Framebuffer<Dim2, (RGBA32F, RGBA32F), Depth32F>;
pub struct Renderer {
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    billboard_renderer: BillboardRenderer,
    debug_renderer: DebugRenderer,
    particle_renderer: ParticleSystem,
    light_renderer: LightingSystem,
    backbuffer: Framebuffer<Dim2, (), ()>,
    quad: Tess,
    offscreen_buffer: OffscreenBuffer,
    shaders: Shaders,

    projection: glam::Mat4,
    view: glam::Mat4,
    glyph_brush: GlyphBrush<'static, text::Instance>,

    // text updates.
    rdr_id: ReaderId<GameEvent>,

    debug: bool,
}

impl Renderer {
    pub fn new(surface: &mut GlfwSurface, resources: &mut Resources) -> Self {
        let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(DEJA_VU).build();

        let particle_renderer = ParticleSystem::new(surface);
        let sprite_renderer = SpriteRenderer::new(surface);
        let billboard_renderer = BillboardRenderer::new(surface);
        let text_renderer = TextRenderer::new(surface, &mut glyph_brush);
        let debug_renderer = DebugRenderer::new(surface);
        let light_renderer = LightingSystem::new(surface);
        let backbuffer = surface.back_buffer().unwrap();
        let rdr_id = {
            let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
            chan.register_reader()
        };
        let quad = TessBuilder::new(surface)
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        let shaders = Shaders::new();

        let projection = glam::Mat4::perspective_rh_gl(
            FOVY,
            surface.width() as f32 / surface.height() as f32,
            Z_NEAR,
            Z_FAR,
        );

        // offscreen buffer that we will render in the first place
        let (w, h) = (backbuffer.width(), backbuffer.height());
        let offscreen_buffer =
            OffscreenBuffer::new(surface, [w as u32, h as u32], 0, Sampler::default())
                .expect("framebuffer creation");

        Self {
            sprite_renderer,
            particle_renderer,
            billboard_renderer,
            text_renderer,
            debug_renderer,
            light_renderer,
            backbuffer,
            offscreen_buffer,
            shaders,
            projection,
            view: glam::Mat4::identity(),
            glyph_brush,
            rdr_id,
            debug: true,
            quad,
        }
    }

    // Update every frame. :)
    pub fn update(&mut self, world: &mut World, dt: Duration, resources: &mut Resources) {
        self.update_view_matrix(world);
        self.particle_renderer
            .update(world, dt.as_secs_f32(), resources);
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

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }
    pub fn render(&mut self, surface: &mut GlfwSurface, world: &World, resources: &Resources) {
        let assets = resources.fetch::<AssetManager>().unwrap();
        self.shaders.update();

        let color = [0., 0., 0., 1.];

        // FIXME maybe not the place for that.
        let should_render_player_ui = {
            if let Some((_, (_, p))) = world.query::<(&MainPlayer, &Player)>().iter().next() {
                p.state == PlayerState::Alive
            } else {
                false
            }
        };

        // I - FIRST RENDER THE DIFFUSE/DEPTH TO OFFSCREEN BUFFER
        // =========================================================================================
        surface.pipeline_builder().pipeline(
            &self.offscreen_buffer,
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
                            let mesh = assets.meshes.get(&mesh_name.mesh).unwrap();
                            tess_gate.render(mesh.slice(..));
                        });
                    }
                });

                self.particle_renderer.render(
                    &self.projection,
                    &self.view,
                    &mut shd_gate,
                    world,
                    &self.shaders,
                );

                self.billboard_renderer.render(
                    &self.projection,
                    &self.view,
                    &pipeline,
                    &mut shd_gate,
                    world,
                    &assets.sprites,
                    &self.shaders,
                );
            },
        );

        // II - Render to screen !
        // =========================================================================================
        surface.pipeline_builder().pipeline(
            &self.backbuffer,
            &PipelineState::default().set_clear_color(color),
            |pipeline, mut shd_gate| {
                self.light_renderer.render(
                    &pipeline,
                    &mut shd_gate,
                    world,
                    &self.offscreen_buffer,
                    &self.shaders,
                );
                //                // we must bind the offscreen framebuffer color content so that we can pass it to a shader
                //                let bound_texture = pipeline.bind_texture(&self.offscreen_buffer.color_slot().0);
                //
                //                shd_gate.shade(&self.shaders.copy_program, |iface, mut rdr_gate| {
                //                    iface.source_texture.update(&bound_texture);
                //                    rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                //                        // this will render the attributeless quad with the offscreen framebuffer color slot
                //                        // bound for the shader to fetch from
                //                        tess_gate.render(&self.quad);
                //                    });
                //                });

                if self.debug {
                    self.debug_renderer.render(
                        &self.projection,
                        &self.view,
                        &mut shd_gate,
                        world,
                        &self.shaders,
                    );
                }

                //
                if should_render_player_ui {
                    self.sprite_renderer.render(
                        &pipeline,
                        &mut shd_gate,
                        world,
                        &assets.sprites,
                        &self.shaders,
                    );

                    self.text_renderer
                        .render(&pipeline, &mut shd_gate, &self.shaders);
                }
            },
        );
        // swap buffer chain
        surface.swap_buffers();
    }
}
