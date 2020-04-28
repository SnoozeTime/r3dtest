use imgui::DrawData;
#[allow(unused_imports)]
use log::{debug, error, info};
use luminance::context::GraphicsContext;
use luminance::pipeline::{BoundTexture, PipelineState};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::Surface;
use serde_derive::{Deserialize, Serialize};
pub mod assets;
pub mod billboard;
pub mod debug;
pub mod lighting;
pub mod mesh;
pub mod particle;
pub mod shaders;
pub mod skybox;
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
use crate::render::mesh::deferred::DeferredRenderer;
use crate::render::particle::ParticleSystem;
use crate::render::shaders::Shaders;
use crate::render::skybox::SkyboxRenderer;
use crate::render::sprite::SpriteRenderer;
use crate::render::text::TextRenderer;
use crate::resources::Resources;
use glyph_brush::{GlyphBrush, GlyphBrushBuilder};
use hecs::World;
use luminance::framebuffer::Framebuffer;
use luminance::pixel::{Depth32F, Floating, RGBA32F};
use luminance::shader::program::Uniform;
use luminance::texture::Dim2;
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

/// Offscreen buffer for deferred rendering:
/// 1. Diffuse buffer,
/// 2. normal buffer
/// 3. Emissive light buffer (for glow).
/// 4. fragment world position
pub type OffscreenBuffer = Framebuffer<Dim2, (RGBA32F, RGBA32F, RGBA32F, RGBA32F), Depth32F>;

pub struct Renderer {
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    _billboard_renderer: BillboardRenderer,
    debug_renderer: DebugRenderer,
    particle_renderer: ParticleSystem,
    _skybox_renderer: SkyboxRenderer,
    deferred_pbr_renderer: DeferredRenderer,
    backbuffer: Framebuffer<Dim2, (), ()>,
    // offscreen_buffer: OffscreenBuffer,
    shaders: Shaders,

    projection: glam::Mat4,
    view: glam::Mat4,
    glyph_brush: GlyphBrush<'static, text::Instance>,

    // text updates.
    rdr_id: ReaderId<GameEvent>,

    debug: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    sky_color: RgbColor,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            sky_color: RgbColor::new(0, 0, 0),
        }
    }
}

impl Renderer {
    pub fn new(surface: &mut GlfwSurface, resources: &mut Resources) -> Self {
        let render_config = resources
            .fetch::<RenderConfig>()
            .and_then(|f| Some((*f).clone()))
            .unwrap_or_default();
        let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(DEJA_VU).build();
        let deferred_pbr_renderer = DeferredRenderer::new(surface);
        let particle_renderer = ParticleSystem::new(surface);
        let sprite_renderer = SpriteRenderer::new(surface);
        let billboard_renderer = BillboardRenderer::new(surface);
        let text_renderer = TextRenderer::new(surface, &mut glyph_brush);
        let debug_renderer = DebugRenderer::new(surface);
        let skybox_renderer = SkyboxRenderer::new(surface, render_config.sky_color);
        let backbuffer = surface.back_buffer().unwrap();
        let rdr_id = {
            let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
            chan.register_reader()
        };
        let shaders = Shaders::new();

        let projection = glam::Mat4::perspective_rh_gl(
            FOVY,
            surface.width() as f32 / surface.height() as f32,
            Z_NEAR,
            Z_FAR,
        );

        // offscreen buffer that we will render in the first place
        //        let (w, h) = (backbuffer.width(), backbuffer.height());
        //        let offscreen_buffer =
        //            OffscreenBuffer::new(surface, [w as u32, h as u32], 0, Sampler::default())
        //                .expect("framebuffer creation");

        Self {
            sprite_renderer,
            particle_renderer,
            _billboard_renderer: billboard_renderer,
            text_renderer,
            debug_renderer,
            deferred_pbr_renderer,
            _skybox_renderer: skybox_renderer,
            backbuffer,
            shaders,
            projection,
            view: glam::Mat4::identity(),
            glyph_brush,
            rdr_id,
            debug: true,
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
                info!("Update view matrix = {:?}", self.view);
            }
        }
    }

    pub fn update_text(&mut self, surface: &mut GlfwSurface, world: &World) {
        self.text_renderer
            .update_text(surface, world, &mut self.glyph_brush);
    }

    pub fn next_blending_mod_lighting(&mut self) {
        self.deferred_pbr_renderer.next_blending_mode();
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

    pub fn render(
        &mut self,
        surface: &mut GlfwSurface,
        world: &World,
        resources: &Resources,
        editor: Option<(&imgui_luminance::Renderer, &DrawData)>,
    ) {
        let assets = resources.fetch::<AssetManager>().unwrap();
        self.shaders.update();

        let color = [0.8, 0.8, 0.8, 1.];

        // FIXME maybe not the place for that.
        let should_render_player_ui = {
            if let Some((_, (_, p))) = world.query::<(&MainPlayer, &Player)>().iter().next() {
                p.state == PlayerState::Alive
            } else {
                false
            }
        };

        // I - Render to screen !
        // =========================================================================================
        surface.pipeline_builder().pipeline(
            &self.backbuffer,
            &PipelineState::default().set_clear_color(color),
            |pipeline, mut shd_gate| {
                //                self.skybox_renderer.render(
                //                    &pipeline,
                //                    &mut shd_gate,
                //                    &self.offscreen_buffer,
                //                    &self.shaders,
                //                );
                self.deferred_pbr_renderer.render(
                    &pipeline,
                    &mut shd_gate,
                    &self.projection,
                    &self.view,
                    world,
                );

                if self.debug {
                    self.debug_renderer.render(
                        &self.projection,
                        &self.view,
                        &mut shd_gate,
                        world,
                        &self.shaders,
                    );
                }

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

                if let Some((editor, draw_data)) = editor {
                    editor.render(&pipeline, &mut shd_gate, draw_data);
                }
            },
        );
    }
}
