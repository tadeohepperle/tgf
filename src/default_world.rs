use std::sync::Arc;

use crate::{
    edit,
    renderer::ui_screen::UiScreenRenderer,
    ui::{batching::ElementBatchesGR, div, Board, ElementContext, REFERENCE_SCREEN_SIZE_D},
    uniforms::Uniforms,
    AppT, Bloom, Camera3d, Color, ColorMeshRenderer, Egui, Gizmos, GraphicsContext, Input,
    RenderFormat, Runner, RunnerCallbacks, Screen, ScreenTextures, ShaderCache, Time, ToneMapping,
    Window,
};
use winit::{dpi::PhysicalSize, event::WindowEvent};

/// use it like this.
pub fn main() {
    let runner = Runner::new(Default::default());
    let mut app = DefaultWorld::new(runner.window());
    runner.run(&mut app).unwrap();
}

/// This struct is meant to be copy-pasted to your own project to add relevant fields and adjust control flow.
/// We could have put it into the examples, but sometimes you might just want to drop in the DefaultWorld to get things going quickly.
pub struct DefaultWorld {
    pub window: Arc<Window>,
    pub rt: tokio::runtime::Runtime,
    pub ctx: GraphicsContext,
    pub shader_cache: ShaderCache,
    pub time: Time,
    pub input: Input,
    pub screen_textures: ScreenTextures,
    pub camera: Camera3d,
    pub screen: Screen,
    pub uniforms: Uniforms,
    pub bloom: Bloom,
    pub tone_mapping: ToneMapping,
    pub egui: crate::Egui,
    pub color_renderer: ColorMeshRenderer,
    pub gizmos: Gizmos,
    pub ui_renderer: UiScreenRenderer,
    pub ui: Board,
    pub ui_gr: ElementBatchesGR,
}

impl AppT for DefaultWorld {
    fn receive_window_event(&mut self, event: &WindowEvent) {
        self.input.receive_window_event(event);
        self.egui.receive_window_event(event);
        if let Some(size) = self.input.resized() {
            self.resize(size);
        }
    }

    fn update(&mut self, _cb: &mut RunnerCallbacks) {
        self.start_frame();
        // /////////////////////////////////////////////////////////////////////////////
        // Your update logic here!
        // /////////////////////////////////////////////////////////////////////////////
        self.render();
        self.end_frame();
    }
}

impl DefaultWorld {
    pub fn new(window: Arc<Window>) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        let ctx = GraphicsContext::new(Default::default(), &rt, &window).unwrap();
        let mut shader_cache = ShaderCache::new(Some("./hotreload"));

        let mut camera = Camera3d::new(window.inner_size().width, window.inner_size().height);
        camera.transform.pos.x = -70.0;

        let size = window.inner_size();
        let screen = Screen::new(size, window.scale_factor());
        let time = Time::new();

        let input = Input::new();

        let uniforms = Uniforms::new(&ctx.device);

        let screen_textures = ScreenTextures::new(
            &ctx.device,
            size.width,
            size.height,
            RenderFormat::HDR_MSAA4,
        );
        let tone_mapping = ToneMapping::new(
            &ctx.device,
            RenderFormat::LDR_NO_MSAA.color,
            &mut shader_cache,
        );
        let bloom = Bloom::new(
            &ctx.device,
            size.width,
            size.height,
            RenderFormat::HDR_MSAA4.color,
            &mut shader_cache,
        );
        let egui = Egui::new(&ctx.device, ctx.surface_format, &window);
        let color_renderer = ColorMeshRenderer::new(&ctx, Default::default(), &mut shader_cache);
        let gizmos = Gizmos::new(&ctx, RenderFormat::HDR_MSAA4, &mut shader_cache);

        let ui_renderer = UiScreenRenderer::new(&ctx.device, &mut shader_cache);
        let ui = Board::new(div().store(), REFERENCE_SCREEN_SIZE_D);
        let ui_gr = ElementBatchesGR::new(&ui.batches, &ctx.device);

        Self {
            window,
            rt,
            ctx,
            shader_cache,
            time,
            input,
            egui,
            screen_textures,
            camera,
            screen,
            uniforms,
            bloom,
            tone_mapping,
            color_renderer,
            gizmos,
            ui_renderer,
            ui,
            ui_gr,
        }
    }

    pub fn start_frame(&mut self) {
        self.time.start_frame();
        self.egui.begin_frame();
        self.shader_cache.hot_reload(
            &mut [
                &mut self.color_renderer,
                &mut self.gizmos,
                &mut self.bloom,
                &mut self.tone_mapping,
                &mut self.ui_renderer,
            ],
            &self.ctx.device,
        );
        self.ui.ctx.start_frame_scaled_to_fixed_height(
            self.input.cursor_pos().as_dvec2(),
            self.input.mouse_buttons(),
            PhysicalSize::new(self.screen.width, self.screen.height),
            REFERENCE_SCREEN_SIZE_D.y,
        );
    }

    pub fn end_frame(&mut self) {
        self.input.end_frame();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.ctx.resize(size);
        self.camera.resize(size);
        self.screen.resize(size);
        self.bloom.resize(size, &self.ctx.device);
        self.screen_textures.resize(&self.ctx.device, size);
        self.ui.resize_scaled_to_fixed_height(size);
    }

    pub fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.color_renderer.prepare();
        self.gizmos.prepare();

        self.egui
            .prepare(&self.ctx.device, &self.ctx.queue, encoder);
        self.ui_gr
            .prepare(&self.ui.batches, &self.ctx.device, &self.ctx.queue);
        self.uniforms.prepare(
            &self.ctx.queue,
            &self.camera,
            &self.screen,
            &self.time,
            &self.input,
        );
    }

    pub fn render(&mut self) {
        self.gizmos.draw_xyz();
        crate::utils::global_vals_window(&mut self.egui.context());
        self.show_fps();

        let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
        self.prepare(&mut encoder);

        let (surface, view) = self.ctx.new_surface_texture_and_view();
        let clear_color = edit!(Color::DARKGREY * 0.1, "clear color");
        let mut pass = self
            .screen_textures
            .new_hdr_target_render_pass(&mut encoder, clear_color);
        self.color_renderer.render(&mut pass, &self.uniforms);
        self.gizmos.render(&mut pass, &self.uniforms);
        drop(pass);

        self.bloom.apply(
            &mut encoder,
            &self.screen_textures.hdr_resolve_target.bind_group(),
            &self.screen_textures.hdr_resolve_target.view(),
            &self.uniforms,
        );
        self.tone_mapping.apply(
            &mut encoder,
            self.screen_textures.hdr_resolve_target.bind_group(),
            &view,
        );
        self.ui_renderer.render_in_new_pass(
            &mut encoder,
            &view,
            &self.ui_gr,
            &self.ui.batches.batches,
            &self.uniforms,
        );
        self.egui.render(&mut encoder, &view);

        self.ctx.queue.submit([encoder.finish()]);
        surface.present();
    }

    pub fn show_fps(&mut self) {
        egui::Window::new("Fps").show(&self.egui.context(), |ui| {
            ui.label(format!(
                "Fps: {:.0} / {:.3} ms",
                self.time.fps(),
                self.time.delta().as_secs_f32() * 1000.0
            ));
        });
    }
}
