use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use smallvec::{smallvec, SmallVec};

use crate::{GraphicsContext, ToRaw, UniformBuffer};

const CACHED_DELTA_TIMES_COUNT: usize = 20;

#[derive(Debug)]
pub struct Time {
    frame_count: usize,
    frame_time: Instant,
    delta_time: Duration,
    total_time: Duration,
    start_time: Instant,
    delta_times: VecDeque<Duration>,
    stats: TimeStats,
}

#[derive(Debug, Default)]
pub struct TimeStats {
    fps: Stats,
    delta_ms: Stats,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub max: f64,
    pub min: f64,
    pub avg: f64,
    pub std: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    pub fn new() -> Self {
        let mut delta_times = VecDeque::new();
        delta_times.push_back(Duration::from_millis(10));
        Time {
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            frame_count: 0,
            frame_time: Instant::now() - Duration::from_millis(10),
            delta_time: Duration::from_millis(10),
            delta_times,
            stats: TimeStats::default(),
        }
    }

    pub fn frame_time(&self) -> Instant {
        self.frame_time
    }

    pub fn start_frame(&mut self) {
        self.total_time = Instant::now() - self.start_time;
        let this_frame = Instant::now();
        if self.delta_times.len() >= CACHED_DELTA_TIMES_COUNT {
            self.delta_times.pop_back();
        }
        self.delta_time = this_frame.duration_since(self.frame_time);
        self.delta_times.push_front(self.delta_time);
        self.frame_time = this_frame;
        self.frame_count += 1;
        self.stats.recalculate(&self.delta_times);
    }
}

impl Time {
    pub fn fps(&self) -> f64 {
        self.stats.fps.avg
    }

    pub fn worst_fps(&self) -> f64 {
        self.stats.fps.min
    }

    #[inline(always)]
    pub fn delta(&self) -> &Duration {
        &self.delta_time
    }

    pub fn total(&self) -> &Duration {
        &self.total_time
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    // pub fn egui_time_stats(&mut self, mut egui_ctx: egui::Context) {
    //     egui::Window::new("Time Stats").show(&mut egui_ctx, |ui| {
    //         ui.label(format!(
    //             "{} fps / {:.1} ms",
    //             self.stats.fps.avg as i64, self.stats.delta_ms.avg,
    //         ));
    //         if ui.button("Log Time Stats").clicked() {
    //             dbg!(&self);
    //         }
    //     });
    // }
}

impl TimeStats {
    fn recalculate(&mut self, delta_times: &VecDeque<Duration>) {
        assert!(!delta_times.is_empty());
        assert!(delta_times.len() <= CACHED_DELTA_TIMES_COUNT);

        let mut delta_ms: SmallVec<[f64; CACHED_DELTA_TIMES_COUNT]> = smallvec![];
        let mut fps: SmallVec<[f64; CACHED_DELTA_TIMES_COUNT]> = smallvec![];
        for d in delta_times {
            let secs = d.as_secs_f64();
            delta_ms.push(secs * 1000.0);
            fps.push(1.0 / secs);
        }

        self.delta_ms = Stats::new(&delta_ms);
        self.fps = Stats::new(&fps);
    }
}

impl Stats {
    fn new(nums: &[f64]) -> Self {
        let mut max: f64 = f64::NAN;
        let mut min: f64 = f64::NAN;
        let mut sum: f64 = 0.0;
        let mut sqsum: f64 = 0.0;
        for e in nums {
            sum += *e;
            sqsum += *e * *e;
            if !(*e < max) {
                max = *e;
            }

            if !(*e > min) {
                min = *e;
            }
        }
        let len = nums.len() as f64;
        let avg = sum / len;
        let var = (sqsum / len) - ((sum / len) * (sum / len));
        let std = var.sqrt();
        Stats { max, min, avg, std }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]

pub struct TimeRaw {
    /// in seconds
    delta: f32,
    /// in seconds
    total: f32,
    frame_count: u32,
}

impl ToRaw for Time {
    type Raw = TimeRaw;

    fn to_raw(&self) -> Self::Raw {
        TimeRaw {
            delta: self.delta_time.as_secs_f32(),
            total: self.total_time.as_secs_f32(),
            frame_count: self.frame_count as u32,
        }
    }
}

pub struct TimeGR {
    uniform: UniformBuffer<TimeRaw>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: Arc<wgpu::BindGroupLayout>,
}

impl TimeGR {
    pub fn new(ctx: &GraphicsContext, time: &Time) -> Self {
        let uniform = UniformBuffer::new(time.to_raw(), &ctx.device);

        let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("Time BindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        };
        let bind_group_layout = Arc::new(ctx.device.create_bind_group_layout(&layout_descriptor));
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Time BindGroup"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.buffer().as_entire_binding(),
            }],
        });

        Self {
            uniform,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, time: &Time) {
        self.uniform.update_and_prepare(time.to_raw(), queue);
    }

    pub fn bind_group_layout(&self) -> &Arc<wgpu::BindGroupLayout> {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
