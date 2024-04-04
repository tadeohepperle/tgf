use std::sync::Arc;

use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use tgf::{
    edit, renderer::screen_textures, utils::camera_controllers::FlyCamController, AppT, Bloom,
    Camera3d, Camera3dGR, Color, ColorMeshRenderer, DefaultWorld, Egui, Gizmos, GraphicsContext,
    Input, KeyCode, RenderFormat, Runner, Screen, ScreenGR, ScreenTextures, Time, ToneMapping,
    Transform, Window,
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
}

impl AppT for App {
    fn receive_window_event(&mut self, event: &tgf::WindowEvent) {
        self.world.receive_window_event(event);
    }

    fn update(&mut self, cb: &mut tgf::RunnerCallbacks) {
        self.world.start_frame();

        if self.world.input.close_requested() {
            cb.exit("exit");
        }

        self.main_update();
        self.world.render();
        self.world.end_frame();
    }
}

impl App {
    fn new(window: Arc<Window>) -> Self {
        let world = DefaultWorld::new(window);
        let some_cubes = random_cubes();

        Self { world, some_cubes }
    }

    fn main_update(&mut self) {
        self.world.gizmos.draw_xyz();

        if self.world.input.keys().just_pressed(KeyCode::Space) {
            self.some_cubes = random_cubes();
        }

        let delta = self.world.time.delta().as_secs_f32();
        let total = self.world.time.total().as_secs_f32();
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

        let speed: f32 = edit!(10.0, "speed");
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
