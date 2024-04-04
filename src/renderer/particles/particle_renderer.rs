use std::sync::Arc;

use crate::{
    make_shader_source, rgba_bind_group_layout_cached, texture::white_px_texture_cached,
    BindableTexture, Camera3dGR, GraphicsContext, HotReload, RenderFormat, ShaderCache,
    ShaderSource, ToRaw, Transform, TransformRaw, VertsLayout,
};
use wgpu::{util::RenderEncoder, RenderPass, ShaderStages};

use super::{ParticleSystem, RawParticle};

const SHADER_SOURCE: ShaderSource = make_shader_source!("particle.wgsl");

pub struct ParticleRenderer {
    pipeline: wgpu::RenderPipeline,
    render_format: RenderFormat,
    ctx: GraphicsContext,
    camera_layout: Arc<wgpu::BindGroupLayout>,
}

impl ParticleRenderer {
    pub fn new(
        ctx: &GraphicsContext,
        camera: &Camera3dGR,
        render_format: RenderFormat,
        cache: &mut ShaderCache,
    ) -> ParticleRenderer {
        let ctx = ctx.clone();
        let shader = cache.register(SHADER_SOURCE);
        let pipeline = create_pipeline(&shader, &ctx, camera.bind_group_layout(), render_format);
        let camera_layout = camera.bind_group_layout().clone();

        ParticleRenderer {
            pipeline,
            render_format,
            ctx,
            camera_layout,
        }
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        camera: &'a Camera3dGR,
        particle_system: &'a ParticleSystem,
    ) {
        let texture = particle_system
            .texture()
            .unwrap_or_else(|| white_px_texture_cached(&self.ctx));

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera.bind_group(), &[]);
        pass.set_bind_group(1, &texture.bind_group, &[]);
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(&[particle_system.transform.to_raw()]),
        );
        pass.set_vertex_buffer(0, particle_system.buffer().slice(..));
        pass.draw(0..4, 0..particle_system.n_particles() as u32);
    }
}

fn create_pipeline(
    shader: &wgpu::ShaderModule,
    ctx: &GraphicsContext,
    camera_layout: &wgpu::BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    let layout = ctx
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particles pipeline"),
            bind_group_layouts: &[camera_layout, rgba_bind_group_layout_cached(&ctx.device)],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<TransformRaw>() as u32,
            }],
        });

    let vertexes = VertsLayout::new().instance::<RawParticle>();

    ctx.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hex map pipline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: vertexes.layout(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_format.color,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: render_format.depth.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: render_format.msaa_sample_count,
                ..Default::default()
            },
            multiview: None,
        })
}

impl HotReload for ParticleRenderer {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule) {
        self.pipeline = create_pipeline(shader, &self.ctx, &self.camera_layout, self.render_format);
    }
}
