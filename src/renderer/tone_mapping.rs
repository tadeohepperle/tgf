
use wgpu::{PushConstantRange, ShaderStages};

use crate::{
    make_shader_source, rgba_bind_group_layout_cached, HotReload, ShaderCache, ShaderSource,
};

pub struct ToneMapping {
    pub enabled: bool,
    pipeline: wgpu::RenderPipeline,
    output_format: wgpu::TextureFormat,
}

const SHADER_SOURCE: ShaderSource =
    make_shader_source!("uniforms.wgsl", "screen.wgsl", "tonemapping.wgsl");

impl ToneMapping {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let shader = shader_cache.register(SHADER_SOURCE, device);
        let pipeline = create_pipeline(&shader, device, output_format);
        Self {
            enabled: true,
            pipeline,
            output_format,
        }
    }

    /// Note: input texture should be hdr, output sdr
    pub fn apply<'e>(
        &'e mut self,
        encoder: &'e mut wgpu::CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    ) {
        let mut tone_mapping_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AcesToneMapping"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_texture,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        tone_mapping_pass.set_pipeline(&self.pipeline);
        tone_mapping_pass.set_bind_group(0, input_texture, &[]);
        tone_mapping_pass.set_push_constants(
            ShaderStages::FRAGMENT,
            0,
            bytemuck::cast_slice(&[PushContants {
                enabled: if self.enabled { 1 } else { 0 },
            }]),
        );
        tone_mapping_pass.draw(0..3, 0..1);
    }
}

impl HotReload for ToneMapping {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.pipeline = create_pipeline(shader, device, self.output_format)
    }
}

fn create_pipeline(
    shader: &wgpu::ShaderModule,
    device: &wgpu::Device,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[rgba_bind_group_layout_cached(device)],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..16,
        }],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{:?}", shader)),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    pipeline
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct PushContants {
    // 0 is off, 1 is enabled
    enabled: u32,
}
