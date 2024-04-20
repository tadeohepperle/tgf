use std::sync::Arc;

use crate::{
    make_shader_source, rgba_bind_group_layout_cached, Camera3dGR, Color, HotReload, RenderFormat,
    ShaderCache, ShaderSource, ToRaw, Transform, TransformRaw, VertexT, VertsLayout,
};
use crate::{
    ui::{
        batching::{
            AlphaSdfRectRaw, Batch, BatchKind, ElementBatchesGR, GlyphRaw, RectRaw, TexturedRectRaw,
        },
        Board,
    },
    GraphicsContext,
};

use wgpu::{RenderPipelineDescriptor, TextureView, VertexState};

#[derive(Debug)]
pub struct Board3d {
    pub transform: Transform,
    pub board: Board,
    pub render_order_z_offset: f32,
    pub batches_gr: ElementBatchesGR,
    pub color: Color,
}

pub struct Ui3DRenderer {
    rect_pipeline: wgpu::RenderPipeline,
    textured_rect_pipeline: wgpu::RenderPipeline,
    alpha_sdf_rect_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    render_format: RenderFormat,
    ctx: GraphicsContext,
    camera_layout: Arc<wgpu::BindGroupLayout>,
}

const SHADER_SOURCE: ShaderSource =
    make_shader_source!("uniforms.wgsl", "ui.wgsl", "ui_3d.wgsl", "alpha_sdf.wgsl");

impl Ui3DRenderer {
    /// shader source should contains:
    ///
    /// - alpha_sdf_wgsl_path: String,
    /// - ui_wgsl_path: String,
    /// - ui_3d_wgsl_path: String,
    pub fn new(
        ctx: &GraphicsContext,
        camera: &Camera3dGR,
        render_format: RenderFormat,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let device = &ctx.device;
        let camera_layout = camera.bind_group_layout().clone();

        let shader = shader_cache.register(SHADER_SOURCE);

        let glyph_pipeline = create_glyph_pipeline(&shader, device, &camera_layout, render_format);
        let rect_pipeline = create_rect_pipeline(&shader, device, &camera_layout, render_format);
        let textured_rect_pipeline =
            create_textured_rect_pipeline(&shader, device, &camera_layout, render_format);

        let alpha_sdf_rect_pipeline =
            create_alpha_sdf_rect_pipeline(&shader, device, &camera_layout, render_format);

        Ui3DRenderer {
            rect_pipeline,
            textured_rect_pipeline,
            glyph_pipeline,
            render_format,
            alpha_sdf_rect_pipeline,
            camera_layout,
            ctx: ctx.clone(),
        }
    }

    pub fn new_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a TextureView,
    ) -> wgpu::RenderPass<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass
    }

    pub fn begin_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a wgpu::TextureView,
    ) -> wgpu::RenderPass<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass
    }

    pub fn render_board<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        board: &'a Board3d,
        camera: &'a Camera3dGR,
    ) {
        self.render_batches(
            pass,
            &board.batches_gr,
            &board.board.batches.batches,
            &board.transform,
            board.color,
            camera,
        )
    }

    pub fn render_batches<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        buffers: &'a ElementBatchesGR,
        batches: &'a Vec<Batch>,
        transform: &Transform,
        color: Color,
        camera: &'a Camera3dGR,
    ) {
        pass.set_bind_group(0, camera.bind_group(), &[]);

        const VERTEX_COUNT: u32 = 4;
        let push_constants = PushConstants {
            transform: transform.to_raw(),
            color,
        };
        for batch in batches.iter() {
            let range = batch.range.start as u32..batch.range.end as u32;
            match &batch.kind {
                BatchKind::Rect => {
                    pass.set_pipeline(&self.rect_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    pass.set_vertex_buffer(0, buffers.rects.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&[push_constants]),
                    );
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::TexturedRect(texture) => {
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.set_pipeline(&self.textured_rect_pipeline);
                    pass.set_vertex_buffer(0, buffers.textured_rects.buffer().slice(..));
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&[push_constants]),
                    );
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::AlphaSdfRect(texture) => {
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.set_pipeline(&self.alpha_sdf_rect_pipeline);
                    pass.set_vertex_buffer(0, buffers.alpha_sdf_rects.buffer().slice(..));
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&[push_constants]),
                    );
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::Glyph(text) => {
                    pass.set_bind_group(1, &text.atlas_texture().bind_group, &[]);
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_vertex_buffer(0, buffers.glyphs.buffer().slice(..));
                    pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&[push_constants]),
                    );
                    pass.draw(0..VERTEX_COUNT, range);
                }
            }
        }
    }
}

impl HotReload for Ui3DRenderer {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule) {
        let render_format = self.render_format;
        let camera = &self.camera_layout;
        let device = &self.ctx.device;

        self.glyph_pipeline = create_glyph_pipeline(&shader, device, camera, render_format);
        self.rect_pipeline = create_rect_pipeline(&shader, device, camera, render_format);
        self.textured_rect_pipeline =
            create_textured_rect_pipeline(&shader, device, camera, render_format);
        self.alpha_sdf_rect_pipeline =
            create_alpha_sdf_rect_pipeline(&shader, device, camera, render_format);
        println!("Hot reloaded Ui 3d Shader");
    }
}

fn create_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    create_pipeline::<RectRaw>(
        shader_module,
        "rect_vs_3d",
        "rect_fs",
        device,
        &[camera_layout],
        render_format,
    )
}

fn create_textured_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    create_pipeline::<TexturedRectRaw>(
        shader_module,
        "textured_rect_vs_3d",
        "textured_rect_fs",
        device,
        &[camera_layout, rgba_bind_group_layout_cached(device)],
        render_format,
    )
}

fn create_alpha_sdf_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    create_pipeline::<AlphaSdfRectRaw>(
        shader_module,
        "alpha_sdf_rect_vs_3d",
        "alpha_sdf_fs",
        device,
        &[camera_layout, rgba_bind_group_layout_cached(device)],
        render_format,
    )
}

fn create_glyph_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    create_pipeline::<GlyphRaw>(
        shader_module,
        "glyph_vs_3d",
        "glyph_fs",
        device,
        &[camera_layout, rgba_bind_group_layout_cached(device)],
        render_format,
    )
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    transform: TransformRaw,
    /// Note: alpha of color used as transparency
    color: Color,
}

pub fn create_pipeline<Instance: VertexT>(
    shader: &wgpu::ShaderModule,
    vs_entry: &str,
    fs_entry: &str,
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(std::any::type_name::<Instance>()),
        bind_group_layouts,
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX,
            range: 0..(std::mem::size_of::<PushConstants>() as u32),
        }],
    });

    let verts_layout = VertsLayout::new().instance::<Instance>();

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(std::any::type_name::<Instance>()),
        layout: Some(&layout),
        vertex: VertexState {
            module: shader,
            entry_point: vs_entry,
            buffers: verts_layout.layout(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: fs_entry,
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
            alpha_to_coverage_enabled: false,
            count: render_format.msaa_sample_count,
            mask: !0,
        },
        multiview: None,
    });
    pipeline
}
