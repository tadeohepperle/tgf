use std::sync::Arc;

use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use tgf::{
    edit, renderer::screen_textures, utils::camera_controllers::FlyCamController, AcesToneMapping,
    AppT, Bloom, Camera3d, Camera3dGR, Color, ColorMeshRenderer, Egui, Gizmos, GraphicsContext,
    Input, KeyCode, RenderFormat, Runner, Screen, ScreenGR, ScreenTextures, Time, Transform,
    Window,
};
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor};

pub fn main() {
    let runner = Runner::new(Default::default());
    let mut app = App::new(runner.window());
    runner.run(&mut app).unwrap();
}

struct App {
    rt: tokio::runtime::Runtime,
    ctx: GraphicsContext,
    window: Arc<Window>,
    time: Time,
    input: Input,
    screen_textures: ScreenTextures,
    camera: Camera3d,
    camera_gr: Camera3dGR,
    screen: Screen,
    screen_gr: ScreenGR,
    bloom: Bloom,
    tone_mapping: AcesToneMapping,
    egui: tgf::Egui,
    color_renderer: ColorMeshRenderer,
    gizmos: Gizmos,
    some_cubes: Vec<Cube>,
}

impl AppT for App {
    fn receive_window_event(&mut self, event: &tgf::WindowEvent) {
        self.input.receive_window_event(event);
        self.egui.receive_window_event(event);
        if let Some(size) = self.input.resized() {
            self.ctx.resize(size);
            self.screen_textures.resize(&self.ctx, size);
            self.camera.resize(size);
            self.screen.resize(size);
            self.bloom.resize(size);
        }
    }

    fn update(&mut self, cb: &mut tgf::RunnerCallbacks) {
        self.time.start_frame();
        self.egui.begin_frame();
        if self.input.close_requested() {
            cb.exit("exit");
        }
        self.main_update();
        self.render();
        self.input.end_frame();
    }
}

impl App {
    fn new(window: Arc<Window>) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        let ctx = GraphicsContext::new(Default::default(), &rt, &window).unwrap();

        let mut camera = Camera3d::new(window.inner_size().width, window.inner_size().height);
        camera.transform.pos.x = -70.0;
        let camera_gr = Camera3dGR::new(&ctx, &camera);

        let screen = Screen::new(&window);
        let screen_gr = ScreenGR::new(&ctx, &screen);

        let screen_textures = ScreenTextures::new(&ctx, RenderFormat::HDR_MSAA4);
        let tone_mapping = AcesToneMapping::new(
            &ctx,
            &screen_textures.screen_vertex_shader,
            RenderFormat::LDR_NO_MSAA.color,
        );
        let bloom = Bloom::new(
            &ctx,
            &screen_textures.screen_vertex_shader,
            &screen_gr,
            RenderFormat::HDR_MSAA4.color,
        );
        let egui = Egui::new(&ctx, &window);
        let color_renderer = ColorMeshRenderer::new(&ctx, &camera_gr, Default::default());
        let gizmos = Gizmos::new(&ctx, &camera_gr, RenderFormat::HDR_MSAA4);

        let time = Time::new();
        let input = Input::new();

        let some_cubes = random_cubes();

        Self {
            rt,
            ctx,
            window,
            time,
            input,
            egui,
            screen_textures,
            camera,
            camera_gr,
            screen,
            screen_gr,
            bloom,
            tone_mapping,
            color_renderer,
            gizmos,
            some_cubes,
        }
    }

    fn render(&mut self) {
        tgf::utils::global_values::global_vals_window(&mut self.egui.context());

        let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
        self.prepare(&mut encoder);

        let (surface, view) = self.ctx.new_surface_texture_and_view();
        let clear_color = edit!(Color::DARKGREY * 0.1, "clear color");
        let mut pass = self
            .screen_textures
            .new_hdr_target_render_pass(&mut encoder, clear_color);
        self.color_renderer.render(&mut pass, &self.camera_gr);
        self.gizmos.render(&mut pass, &self.camera_gr);
        drop(pass);

        self.bloom.apply(
            &mut encoder,
            &self.screen_textures.hdr_resolve_target.bind_group(),
            &self.screen_textures.hdr_resolve_target.view(),
            &self.screen_gr,
        );
        self.tone_mapping.apply(
            &mut encoder,
            self.screen_textures.hdr_resolve_target.bind_group(),
            &view,
        );
        self.egui.render(&mut encoder, &view);

        self.ctx.queue.submit([encoder.finish()]);
        surface.present();
    }

    fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.color_renderer.prepare();
        self.gizmos.prepare();
        self.camera_gr.prepare(&self.ctx.queue, &self.camera);
        self.egui.prepare(&self.ctx, encoder);
    }

    fn main_update(&mut self) {
        self.gizmos.draw_xyz();

        if self.input.keys().just_pressed(KeyCode::Space) {
            self.some_cubes = random_cubes();
        }

        let delta = self.time.delta().as_secs_f32();
        let total = self.time.total().as_secs_f32();
        for c in self.some_cubes.iter_mut() {
            c.color.r = ((total + c.position.x % 2.0) * 4.0).sin() + 1.0 * 5.0;
            c.position += c.velocity * delta;
        }

        let cube_instances: Vec<(Transform, Color)> = self
            .some_cubes
            .iter()
            .map(|c| {
                (
                    Transform {
                        position: c.position,
                        rotation: Quat::from_scaled_axis(c.position),
                        scale: Vec3::splat(c.size),
                    },
                    c.color,
                )
            })
            .collect();
        self.color_renderer.draw_cubes(&cube_instances);

        let speed: f32 = edit!(10.0, "speed");
        let angle_speed: f32 = edit!(2.0, "angle speed");
        let cam_controller = FlyCamController { speed, angle_speed };
        cam_controller.update(&self.input, &self.time, &mut self.camera);

        egui::Window::new("Fps").show(&self.egui.context(), |ui| {
            ui.label(format!(
                "Fps: {:.0} / {:.3} ms",
                self.time.fps(),
                self.time.delta().as_secs_f32() * 1000.0
            ));
        });
    }
}

struct Cube {
    velocity: Vec3,
    position: Vec3,
    color: Color,
    size: f32,
}

fn random_cubes() -> Vec<Cube> {
    let mut rng = thread_rng();
    (0..1024)
        .map(|_| Cube {
            velocity: rng.gen::<Vec3>(),
            position: rng.gen::<Vec3>() * 100.0 - 50.0,
            color: Color::from_hsv(rng.gen::<f64>() * 360.0, 1.0, 1.0),
            size: rng.gen::<f32>() * 1.4,
        })
        .collect()
}
