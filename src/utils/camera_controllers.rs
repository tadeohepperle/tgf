use crate::{Camera3d, Input, Time};

pub struct FlyCamController {
    pub speed: f32,
    pub angle_speed: f32,
}

impl Default for FlyCamController {
    fn default() -> Self {
        FlyCamController {
            speed: 10.0,
            angle_speed: 1.8,
        }
    }
}

impl FlyCamController {
    pub fn new() -> Self {
        FlyCamController::default()
    }

    pub fn update(&self, input: &Input, time: &Time, camera: &mut Camera3d) {
        let wasd = input.wasd_vec();
        let arrows = input.arrow_vec();
        let updown = input.rf_updown();
        let delta_time = time.delta().as_secs_f32();
        let cam = &mut camera.transform;
        cam.pos += cam.forward() * wasd.y * self.speed * delta_time;
        cam.pos += cam.right() * wasd.x * self.speed * delta_time;
        cam.pos.y += updown * self.speed * delta_time;

        cam.pitch += arrows.y * self.angle_speed * delta_time;
        cam.yaw += arrows.x * self.angle_speed * delta_time;
    }
}
