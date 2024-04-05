use std::{rc::Rc, sync::Arc};

use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use tgf::{
    edit,
    renderer::screen_textures,
    ui::{div, Align, SdfFont, TextSection},
    utils::camera_controllers::FlyCamController,
    AppT, Bloom, Camera3d, Camera3dGR, Color, ColorMeshRenderer, DefaultWorld, Egui, Gizmos,
    GraphicsContext, Input, KeyCode, RenderFormat, Runner, Screen, ScreenGR, ScreenTextures, Time,
    ToneMapping, Transform, Window,
};
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor};

pub fn main() {
    let runner = Runner::new(Default::default());
    let mut app = App::new(runner.window());
    runner.run(&mut app).unwrap();
}

struct App {
    world: DefaultWorld,
    some_cubes: Vec<Cube>,
    font: Rc<SdfFont>,
}

impl AppT for App {
    fn receive_window_event(&mut self, event: &tgf::WindowEvent) {
        self.world.receive_window_event(event);
    }

    fn update(&mut self, cb: &mut tgf::RunnerCallbacks) {
        self.world.start_frame();
        self.main_update(cb);
        self.world.render();
        self.world.end_frame();
    }
}

impl App {
    fn new(window: Arc<Window>) -> Self {
        let world = DefaultWorld::new(window);
        let some_cubes = random_cubes();
        let font =
            SdfFont::from_bytes(include_bytes!("../assets/MarkoOne-Regular.ttf"), &world.ctx);
        Self {
            world,
            some_cubes,
            font: Rc::new(font),
        }
    }

    fn main_update(&mut self, cb: &mut tgf::RunnerCallbacks) {
        if self.world.input.close_requested() {
            cb.exit("exit");
        }

        let delta = self.world.time.delta().as_secs_f32();
        let total = self.world.time.total().as_secs_f32();

        let shadow_intensity = edit!(1.7, 0.0..10.0, "shadow intensity");
        let font_size = edit!(64.0, 0.0..300.0, "font size");
        self.world.ui.set_element(
            div()
                .full()
                .style(|s| {
                    s.padding.top = 32.0 + (total * 2.0).sin() as f64 * 8.0;
                    s.cross_align = Align::Center;
                })
                .child(TextSection {
                    string: "Move with WASD. Turn with arrow keys.".into(),
                    font: self.font.clone(),
                    color: Color::WHITE,
                    font_size,
                    shadow_intensity,
                })
                .store(),
        );

        self.world.gizmos.draw_xyz();

        if self.world.input.keys().just_pressed(KeyCode::Space) {
            self.some_cubes = random_cubes();
        }

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
        self.world.color_renderer.draw_cubes(&cube_instances);

        let speed: f32 = edit!(10.0, 0.0..100.0, "speed");
        let angle_speed: f32 = edit!(2.0, "angle speed");
        let cam_controller = FlyCamController { speed, angle_speed };
        cam_controller.update(&self.world.input, &self.world.time, &mut self.world.camera);

        egui::Window::new("Fps").show(&self.world.egui.context(), |ui| {
            ui.label(format!(
                "Fps: {:.0} / {:.3} ms",
                self.world.time.fps(),
                self.world.time.delta().as_secs_f32() * 1000.0
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
