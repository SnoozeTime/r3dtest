use crate::colors::RgbColor;
use crate::render::shaders::Shaders;
use crate::render::sprite::ScreenPosition;
use glyph_brush::{rusttype::*, *};
use log::info;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, Pipeline, ShadingGate};
use luminance::pixel::{NormR8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::Uniform;
use luminance::tess::{Mode, TessBuilder};
use luminance::tess::{Tess, TessSliceIndex};
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::GlfwSurface;
use luminance_windowing::Surface;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "left_top", repr = "[f32; 3]", wrapper = "VertexLeftTop")]
    LeftTop,

    #[sem(
        name = "right_bottom",
        repr = "[f32; 2]",
        wrapper = "VertexRightBottom"
    )]
    RightBottom,

    #[sem(name = "tex_left_top", repr = "[f32; 2]", wrapper = "TextureLeftTop")]
    TexLeftTop,

    #[sem(
        name = "tex_right_bottom",
        repr = "[f32; 2]",
        wrapper = "TextureRightBottom"
    )]
    TexRightBottom,

    #[sem(name = "color", repr = "[f32; 4]", wrapper = "TextColor")]
    Color,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Vertex, Debug, Clone)]
#[vertex(sem = "VertexSemantics", instanced = "true")]
pub struct Instance {
    left_top: VertexLeftTop,
    right_bottom: VertexRightBottom,
    tex_left_top: TextureLeftTop,
    tex_right_bottom: TextureRightBottom,
    color: TextColor,
}

#[derive(UniformInterface)]
pub struct ShaderInterface {
    pub tex: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
    pub transform: Uniform<M44>,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub content: String,
    pub font_size: f32,
}

pub struct TextRenderer {
    projection: glam::Mat4,
    texture: Texture<Dim2, NormR8UI>,
    tess: Tess,
    render_state: RenderState,
}

impl TextRenderer {
    pub fn new(surface: &mut GlfwSurface, glyph_brush: &mut GlyphBrush<'static, Instance>) -> Self {
        let projection = glam::Mat4::orthographic_rh_gl(
            0.0,
            surface.width() as f32,
            0.0,
            surface.height() as f32,
            1.0,
            -1.0,
        );

        let render_state = RenderState::default()
            .set_blending((Equation::Additive, Factor::SrcAlpha, Factor::Zero))
            .set_depth_test(None);
        let tex: Texture<Dim2, NormR8UI> = Texture::new(
            surface,
            [
                glyph_brush.texture_dimensions().0,
                glyph_brush.texture_dimensions().1,
            ],
            0,
            Sampler::default(),
        )
        .expect("luminance texture creation");

        // the first argument disables mipmap generation (we don’t care so far)
        //tex.upload_raw(GenMipmaps::No, &texels).unwrap();

        //let mut glyph_texture = GlGlyphTexture::new(glyph_brush.texture_dimensions());
        let width = surface.width() as f32;
        let height = surface.height() as f32;
        let mut font_size: f32 = 60.0;

        let scale = Scale::uniform((font_size).round());

        glyph_brush.queue(Section {
            text: "Bonjour",
            scale,
            screen_position: (width, height),
            bounds: (width / 3.15, height),
            color: [0.3, 0.3, 0.9, 1.0],
            layout: Layout::default()
                .h_align(HorizontalAlign::Right)
                .v_align(VerticalAlign::Bottom),
            ..Section::default()
        });

        let action = glyph_brush
            .process_queued(
                |rect, tex_data| unsafe {
                    // Update part of gpu texture with new glyph alpha values
                    tex.upload_part_raw(
                        GenMipmaps::No,
                        [rect.min.x as u32, rect.min.y as u32],
                        [rect.width() as u32, rect.height() as u32],
                        tex_data,
                    );
                },
                |vertex_data| to_vertex(vertex_data),
            )
            .unwrap();

        let tess = match action {
            BrushAction::Draw(v) => {
                println!("FIRST TIME {:?}", v);

                TessBuilder::new(surface)
                    .set_vertex_nb(4)
                    .add_instances(v)
                    .set_mode(Mode::TriangleStrip)
                    .build()
                    .unwrap()
            }
            _ => panic!("WUUUT"),
        };

        // -------------------------------------------------------------------------------------------------------------------
        glyph_brush.queue(Section {
            text: "100",
            scale,
            screen_position: (width, height),
            bounds: (width / 3.15, height),
            color: [0.3, 0.3, 0.9, 1.0],
            layout: Layout::default()
                .h_align(HorizontalAlign::Right)
                .v_align(VerticalAlign::Bottom),
            ..Section::default()
        });

        glyph_brush.queue(Section {
            text: "Au revoir",
            scale,
            screen_position: (width / 2.0, height),
            bounds: (width / 3.15, height),
            color: [0.3, 0.3, 0.9, 1.0],
            layout: Layout::default()
                .h_align(HorizontalAlign::Right)
                .v_align(VerticalAlign::Bottom),
            ..Section::default()
        });

        let action = glyph_brush
            .process_queued(
                |rect, tex_data| unsafe {
                    // Update part of gpu texture with new glyph alpha values
                    tex.upload_part_raw(
                        GenMipmaps::No,
                        [rect.min.x as u32, rect.min.y as u32],
                        [rect.width() as u32, rect.height() as u32],
                        tex_data,
                    );
                },
                |vertex_data| to_vertex(vertex_data),
            )
            .unwrap();

        let tess = match action {
            BrushAction::Draw(v) => {
                println!("SECOND TIME {:?}", v);
                TessBuilder::new(surface)
                    .set_vertex_nb(4)
                    .add_instances(v)
                    .set_mode(Mode::TriangleStrip)
                    .build()
                    .unwrap()
            }
            _ => panic!("WUUUT"),
        };
        // _-------------------------------------

        Self {
            projection,
            texture: tex,
            tess,
            render_state,
        }
    }

    pub fn update_text(
        &mut self,
        surface: &mut GlfwSurface,
        world: &hecs::World,
        glyph_brush: &mut GlyphBrush<'static, Instance>,
    ) {
        let width = surface.width() as f32;
        let height = surface.height() as f32;
        let mut font_size: f32 = 60.0;

        for (_, (text, position, color)) in
            world.query::<(&Text, &ScreenPosition, &RgbColor)>().iter()
        {
            // screen position is left-bottom origin, and value is between 0 and 1.
            let pos_x = width * position.x;
            let pos_y = height * (1.0 - position.y);

            let scale = Scale::uniform(text.font_size.round());
            glyph_brush.queue(Section {
                text: text.content.as_str(),
                scale,
                screen_position: (pos_x, pos_y),
                bounds: (width / 3.15, height),
                color: color.to_rgba_normalized(),
                layout: Layout::default()
                    .h_align(HorizontalAlign::Left)
                    .v_align(VerticalAlign::Bottom),
                ..Section::default()
            });
        }

        let action = glyph_brush
            .process_queued(
                |rect, tex_data| {
                    // Update part of gpu texture with new glyph alpha values
                    self.texture.upload_part_raw(
                        GenMipmaps::No,
                        [rect.min.x as u32, rect.min.y as u32],
                        [rect.width() as u32, rect.height() as u32],
                        tex_data,
                    );
                },
                |vertex_data| to_vertex(vertex_data),
            )
            .unwrap();

        match action {
            BrushAction::Draw(v) => {
                self.tess = TessBuilder::new(surface)
                    .set_vertex_nb(4)
                    .add_instances(v)
                    .set_mode(Mode::TriangleStrip)
                    .build()
                    .unwrap();
            }
            BrushAction::ReDraw => (),
        };
    }

    pub fn render<S>(&self, pipeline: &Pipeline, shd_gate: &mut ShadingGate<S>, shaders: &Shaders)
    where
        S: GraphicsContext,
    {
        shd_gate.shade(&shaders.text_program, |iface, mut rdr_gate| {
            iface.transform.update(self.projection.to_cols_array_2d());
            let texture = pipeline.bind_texture(&self.texture);
            iface.tex.update(&texture);

            rdr_gate.render(&self.render_state, |mut tess_gate| {
                tess_gate.render(self.tess.slice(..));
            });
        });
    }
}

#[inline]
fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        color,
        z,
    }: glyph_brush::GlyphVertex,
) -> Instance {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    let v = Instance {
        left_top: VertexLeftTop::new([gl_rect.min.x, gl_rect.max.y, z]),
        right_bottom: VertexRightBottom::new([gl_rect.max.x, gl_rect.min.y]),
        tex_left_top: TextureLeftTop::new([tex_coords.min.x, tex_coords.max.y]),
        tex_right_bottom: TextureRightBottom::new([tex_coords.max.x, tex_coords.min.y]),
        color: TextColor::new(color),
    };

    info!("vertex -> {:?}", v);
    v
}

fn load_from_disk(surface: &mut GlfwSurface, img: image::RgbaImage) -> Texture<Dim2, NormR8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    // create the luminance texture; the third argument is the number of mipmaps we want (leave it
    // to 0 for now) and the latest is the sampler to use when sampling the texels in the
    // shader (we’ll just use the default one)
    let tex = Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    // the first argument disables mipmap generation (we don’t care so far)
    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
