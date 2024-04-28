
use glam::{vec3, Vec3};
use wgpu::{
    BufferUsages, FragmentState, PrimitiveState,
    RenderPipelineDescriptor, VertexState,
};

use crate::{
    make_shader_source, uniforms::Uniforms, Color, GraphicsContext, GrowableBuffer,
    HotReload, ImmediateMeshQueue, ImmediateMeshRanges, RenderFormat, ShaderCache, ShaderSource,
    ToRaw, Transform, TransformRaw, VertexT, VertsLayout,
};

const SHADER_SOURCE: ShaderSource = make_shader_source!("uniforms.wgsl", "color_mesh.wgsl");

#[derive(Debug)]
pub struct ColorMeshRenderer {
    pipeline: wgpu::RenderPipeline,
    /// immediate geometry, cleared every frame
    color_mesh_queue: ImmediateMeshQueue<Vertex, (Transform, Color)>,
    /// information about index ranges
    render_data: RenderData,
    ctx: GraphicsContext,
    config: ColorMeshRendererConfig,
}

#[derive(Debug, Clone)]
pub struct ColorMeshRendererConfig {
    pub render_format: RenderFormat,
    pub depth_write_enabled: bool,
    pub depth_compare: wgpu::CompareFunction,
    pub blend_state: wgpu::BlendState,
}

impl Default for ColorMeshRendererConfig {
    fn default() -> Self {
        Self {
            render_format: RenderFormat::HDR_MSAA4,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            blend_state: wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}
impl ColorMeshRenderer {
    /// Use Replace or Alpha Blending for the blend mode.
    pub fn new(
        ctx: &GraphicsContext,
        config: ColorMeshRendererConfig,
        cache: &mut ShaderCache,
    ) -> Self {
        let shader = cache.register(SHADER_SOURCE, &ctx.device);
        let pipeline = create_render_pipeline(&shader, &ctx.device, &config);

        ColorMeshRenderer {
            pipeline,
            color_mesh_queue: ImmediateMeshQueue::default(),
            render_data: RenderData::new(&ctx.device),
            ctx: ctx.clone(),
            config,
        }
    }

    #[inline(always)]
    pub fn draw_geometry(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
        instances: &[(Transform, Color)],
    ) {
        self.color_mesh_queue.add_mesh(vertices, indices, instances);
    }

    pub fn draw_cubes(&mut self, instances: &[(Transform, Color)]) {
        const P: f32 = 0.5;
        const M: f32 = -0.5;
        let positions = vec![
            [M, M, M],
            [P, M, M],
            [P, M, P],
            [M, M, P],
            [M, P, M],
            [P, P, M],
            [P, P, P],
            [M, P, P],
        ];

        let vertices: Vec<Vertex> = positions
            .into_iter()
            .map(|p| {
                let x = p[0];
                let y = p[1];
                let z = p[2];
                Vertex {
                    pos: vec3(x, y, z),
                    color: Color::WHITE,
                }
            })
            .collect();

        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 7, 6, 4, 6, 5, 1, 5, 6, 1, 6, 2, 0, 3, 7, 0, 7, 4, 2, 6, 3, 6, 7,
            3, 0, 4, 1, 4, 5, 1,
        ];
        self.draw_geometry(&vertices, &indices, instances)
    }

    pub fn prepare(&mut self) {
        let device = &self.ctx.device;
        let queue = &self.ctx.queue;
        self.render_data
            .vertex_buffer
            .prepare(self.color_mesh_queue.vertices(), device, queue);
        self.render_data
            .index_buffer
            .prepare(self.color_mesh_queue.indices(), device, queue);
        self.render_data
            .instance_buffer
            .prepare(self.color_mesh_queue.instances(), device, queue);
        self.color_mesh_queue
            .clear_and_take_meshes(&mut self.render_data.mesh_ranges);
    }

    pub fn render<'encoder>(
        &'encoder self,
        render_pass: &mut wgpu::RenderPass<'encoder>,
        uniforms: &'encoder Uniforms,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, uniforms.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.render_data.vertex_buffer.buffer().slice(..));
        render_pass.set_index_buffer(
            self.render_data.index_buffer.buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_vertex_buffer(1, self.render_data.instance_buffer.buffer().slice(..));
        for mesh in self.render_data.mesh_ranges.iter() {
            render_pass.draw_indexed(mesh.index_range.clone(), 0, mesh.instance_range.clone())
        }
    }
}

impl HotReload for ColorMeshRenderer {
    fn source(&self) -> crate::ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.pipeline = create_render_pipeline(shader, device, &self.config)
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Render Pipeline
// /////////////////////////////////////////////////////////////////////////////

/// buffers for immediate geometry
#[derive(Debug)]
struct RenderData {
    mesh_ranges: Vec<ImmediateMeshRanges>,
    vertex_buffer: GrowableBuffer<Vertex>,
    index_buffer: GrowableBuffer<u32>,
    instance_buffer: GrowableBuffer<Instance>,
}

impl RenderData {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            mesh_ranges: vec![],
            vertex_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
            index_buffer: GrowableBuffer::new(device, 512, BufferUsages::INDEX),
            instance_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Color,
}

impl VertexT for Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x3, // "pos"
        wgpu::VertexFormat::Float32x4, // "color"
    ];
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, PartialEq)]
pub struct Instance {
    transform: TransformRaw,
    color: Color,
}

impl VertexT for Instance {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "col1"
        wgpu::VertexFormat::Float32x4, // "col2"
        wgpu::VertexFormat::Float32x4, // "col3"
        wgpu::VertexFormat::Float32x4, // "translation"
        wgpu::VertexFormat::Float32x4, // "color"
    ];
}

impl ToRaw for (Transform, Color) {
    type Raw = Instance;

    fn to_raw(&self) -> Self::Raw {
        Instance {
            transform: self.0.to_raw(),
            color: self.1,
        }
    }
}

fn create_render_pipeline(
    shader: &wgpu::ShaderModule,
    device: &wgpu::Device,
    config: &ColorMeshRendererConfig,
) -> wgpu::RenderPipeline {
    let label = "ColorMeshRenderer";

    let verts = VertsLayout::new().vertex::<Vertex>().instance::<Instance>();

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[Uniforms::cached_layout()],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(&format!("{label} Pipeline")),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: verts.layout(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.render_format.color,
                blend: Some(config.blend_state),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: config
            .render_format
            .depth
            .map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: config.depth_write_enabled,
                depth_compare: config.depth_compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
        multisample: wgpu::MultisampleState {
            count: config.render_format.msaa_sample_count,
            ..Default::default()
        },
        multiview: None,
    })
}
