use std::sync::{Arc, LazyLock, OnceLock};

use crate::{
    input::InputRaw, Camera3d, Camera3dRaw, GraphicsContext, Input, Screen, ScreenRaw, Time,
    TimeRaw, ToRaw, UniformBuffer,
};

static GLOBAL_UNIFORMS_BIND_GROUP_LAYOUT: OnceLock<Arc<wgpu::BindGroupLayout>> = OnceLock::new();

pub struct Uniforms {
    camera: UniformBuffer<Camera3dRaw>,
    screen: UniformBuffer<ScreenRaw>,
    time: UniformBuffer<TimeRaw>,
    input: UniformBuffer<InputRaw>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: Arc<wgpu::BindGroupLayout>,
}

impl Uniforms {
    pub fn cached_layout() -> &'static Arc<wgpu::BindGroupLayout> {
        GLOBAL_UNIFORMS_BIND_GROUP_LAYOUT
            .get()
            .expect("GlobalUniforms not initialized yet!")
    }

    pub fn new(
        ctx: &GraphicsContext,
        camera: &Camera3d,
        screen: &Screen,
        time: &Time,
        input: &Input,
    ) -> Self {
        let bind_group_layout = GLOBAL_UNIFORMS_BIND_GROUP_LAYOUT
            .get_or_init(|| {
                let entry = |binding: u32| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                };

                let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
                    label: Some("Globals BindGroupLayout"),
                    entries: &[entry(0), entry(1), entry(2), entry(3)],
                };
                let bind_group_layout =
                    Arc::new(ctx.device.create_bind_group_layout(&layout_descriptor));
                bind_group_layout
            })
            .clone();

        let camera = UniformBuffer::new(camera.to_raw(), &ctx.device);
        let screen = UniformBuffer::new(screen.to_raw(), &ctx.device);
        let time = UniformBuffer::new(time.to_raw(), &ctx.device);
        let input = UniformBuffer::new(input.to_raw(), &ctx.device);

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globals BindGroup"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: screen.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: time.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: input.buffer().as_entire_binding(),
                },
            ],
        });

        Self {
            camera,
            screen,
            time,
            input,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera3d,
        screen: &Screen,
        time: &Time,
        input: &Input,
    ) {
        self.camera.update_and_prepare(camera.to_raw(), queue);
        self.screen.update_and_prepare(screen.to_raw(), queue);
        self.time.update_and_prepare(time.to_raw(), queue);
        self.input.update_and_prepare(input.to_raw(), queue);
    }

    pub fn bind_group_layout(&self) -> &Arc<wgpu::BindGroupLayout> {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
