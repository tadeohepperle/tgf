use std::sync::Arc;

use glam::vec2;
use glam::vec3;
use glam::Vec3;
use wgpu::BindGroupLayout;
use wgpu::BufferUsages;
use wgpu::FragmentState;
use wgpu::PrimitiveState;
use wgpu::ShaderModuleDescriptor;
use wgpu::VertexState;

use crate::make_shader_source;
use crate::uniforms::Uniforms;
use crate::Aabb;
use crate::Camera3dGR;
use crate::Color;
use crate::GraphicsContext;
use crate::GrowableBuffer;
use crate::HotReload;
use crate::ShaderCache;
use crate::ShaderSource;
use crate::VertexT;
use crate::VertsLayout;

use super::RenderFormat;

const SHADER_SOURCE: ShaderSource = make_shader_source!("uniforms.wgsl", "gizmos.wgsl");

pub struct GizmosVertexQueue(pub Vec<Vertex>);

impl GizmosVertexQueue {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        self.0.push(Vertex { pos: from, color });
        self.0.push(Vertex { pos: to, color });
    }

    pub fn draw_xyz(&mut self) {
        self.0.push(Vertex {
            pos: Vec3::ZERO,
            color: Color::RED,
        });
        self.0.push(Vertex {
            pos: Vec3::X,
            color: Color::RED,
        });

        self.0.push(Vertex {
            pos: Vec3::ZERO,
            color: Color::GREEN,
        });
        self.0.push(Vertex {
            pos: Vec3::Y,
            color: Color::GREEN,
        });

        self.0.push(Vertex {
            pos: Vec3::ZERO,
            color: Color::BLUE,
        });
        self.0.push(Vertex {
            pos: Vec3::Z,
            color: Color::BLUE,
        });
    }

    pub fn draw_cube(&mut self, position: Vec3, side_len: f32, color: Color) {
        let l = side_len / 2.0;

        let v1 = position + vec3(-l, -l, -l);
        let v2 = position + vec3(l, -l, -l);
        let v3 = position + vec3(l, -l, l);
        let v4 = position + vec3(-l, -l, l);
        let v5 = position + vec3(-l, l, -l);
        let v6 = position + vec3(l, l, -l);
        let v7 = position + vec3(l, l, l);
        let v8 = position + vec3(-l, l, l);
        let lines = [
            (v1, v2),
            (v2, v3),
            (v3, v4),
            (v4, v1),
            (v5, v6),
            (v6, v7),
            (v7, v8),
            (v8, v5),
            (v1, v5),
            (v2, v6),
            (v3, v7),
            (v4, v8),
        ];

        for (from, to) in lines {
            self.0.push(Vertex { pos: from, color });
            self.0.push(Vertex { pos: to, color });
        }
    }

    pub fn draw_aabb(&mut self, aabb: Aabb, color: Color) {
        let a = aabb.min.extend(0.0);
        let b = vec2(aabb.max.x, aabb.min.y).extend(0.0);
        let c = aabb.max.extend(0.0);
        let d = vec2(aabb.min.x, aabb.max.y).extend(0.0);

        self.draw_line(a, b, color);
        self.draw_line(b, c, color);
        self.draw_line(c, d, color);
        self.draw_line(d, a, color);
    }
}

pub struct Gizmos {
    /// immediate vertices, written to vertex_buffer every frame.
    vertex_queue: GizmosVertexQueue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: GrowableBuffer<Vertex>,
    ctx: GraphicsContext,
    render_format: RenderFormat,
}

impl Gizmos {
    pub fn new(
        ctx: &GraphicsContext,
        render_format: RenderFormat,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let vertex_buffer = GrowableBuffer::new(&ctx.device, 256, BufferUsages::VERTEX);

        let shader = shader_cache.register(SHADER_SOURCE, &ctx.device);
        let pipeline = create_pipeline(&shader, &ctx.device, render_format);
        Gizmos {
            pipeline,
            vertex_queue: GizmosVertexQueue::new(),
            vertex_buffer,
            ctx: ctx.clone(),
            render_format,
        }
    }

    pub fn render<'encoder>(
        &'encoder self,
        render_pass: &mut wgpu::RenderPass<'encoder>,
        uniforms: &'encoder Uniforms,
    ) {
        if self.vertex_buffer.len() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, uniforms.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer().slice(..));
        render_pass.draw(0..self.vertex_buffer.len() as u32, 0..1);
    }

    pub fn prepare(&mut self) {
        self.vertex_buffer
            .prepare(&self.vertex_queue.0, &self.ctx.device, &self.ctx.queue);
        self.vertex_queue.0.clear();
    }

    #[inline]
    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        self.vertex_queue.draw_line(from, to, color)
    }

    #[inline]
    pub fn draw_xyz(&mut self) {
        self.vertex_queue.draw_xyz();
    }

    #[inline]
    pub fn draw_cube(&mut self, position: Vec3, side_len: f32, color: Color) {
        self.vertex_queue.draw_cube(position, side_len, color)
    }

    #[inline]
    pub fn draw_aabb(&mut self, aabb: Aabb, color: Color) {
        self.vertex_queue.draw_aabb(aabb, color);
    }
}

impl HotReload for Gizmos {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.pipeline = create_pipeline(shader, device, self.render_format);
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Color,
}

impl VertexT for Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] =
        &[wgpu::VertexFormat::Float32x3, wgpu::VertexFormat::Float32x4];
}

pub fn create_pipeline(
    shader: &wgpu::ShaderModule,
    device: &wgpu::Device,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    let label = "Gizmos";
    let vertexes = VertsLayout::new().vertex::<Vertex>();

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[Uniforms::cached_layout()],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertexes.layout(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: render_format.color,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: render_format.depth.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
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
