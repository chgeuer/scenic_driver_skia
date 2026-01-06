use skia_safe::{
    Color, ColorType, Font, FontMgr, FontStyle, Paint, Rect, Surface,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
};

fn create_skia_surface(
    dimensions: (i32, i32),
    fb_info: FramebufferInfo,
    gr_context: &mut skia_safe::gpu::DirectContext,
    num_samples: usize,
    stencil_size: usize,
) -> Surface {
    let backend_render_target =
        backend_render_targets::make_gl(dimensions, num_samples, stencil_size, fb_info);

    gpu::surfaces::wrap_backend_render_target(
        gr_context,
        &backend_render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .expect("Could not create Skia surface")
}

pub struct Renderer {
    surface: Surface,
    gr_context: skia_safe::gpu::DirectContext,
    fb_info: FramebufferInfo,
    num_samples: usize,
    stencil_size: usize,
}

impl Renderer {
    pub fn new(
        dimensions: (u32, u32),
        fb_info: FramebufferInfo,
        gr_context: skia_safe::gpu::DirectContext,
        num_samples: usize,
        stencil_size: usize,
    ) -> Self {
        let mut gr_context = gr_context;
        let surface = create_skia_surface(
            (dimensions.0 as i32, dimensions.1 as i32),
            fb_info,
            &mut gr_context,
            num_samples,
            stencil_size,
        );

        Self {
            surface,
            gr_context,
            fb_info,
            num_samples,
            stencil_size,
        }
    }

    pub fn redraw(&mut self) {
        let canvas = self.surface.canvas();
        canvas.clear(Color::WHITE);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::BLACK);

        let mut p = Paint::default();
        p.set_anti_alias(true);
        p.set_color(Color::from_argb(255, 255, 0, 0));
        canvas.draw_rect(Rect::from_xywh(40.0, 40.0, 200.0, 120.0), &p);

        let fm = FontMgr::new();
        let tf = fm
            .match_family_style("DejaVu Sans", FontStyle::normal())
            .or_else(|| fm.match_family_style("Sans", FontStyle::normal()))
            .expect("No system fonts found");

        let font = Font::new(tf, 48.0);
        canvas.draw_str("Hello, Wayland", (40, 120), &font, &paint);

        self.gr_context.flush_and_submit();
    }

    pub fn resize(&mut self, dimensions: (u32, u32)) {
        self.surface = create_skia_surface(
            (dimensions.0 as i32, dimensions.1 as i32),
            self.fb_info,
            &mut self.gr_context,
            self.num_samples,
            self.stencil_size,
        );
    }
}
