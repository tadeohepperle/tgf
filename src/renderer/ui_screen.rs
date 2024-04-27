use std::sync::Arc;

use crate::{
    make_shader_source, rgba_bind_group_layout_cached, uniforms::Uniforms, GraphicsContext,
    HotReload, ScreenGR, ShaderCache, ShaderSource, VertexT, VertsLayout,
};

use wgpu::{RenderPipelineDescriptor, TextureView, VertexState};

use crate::ui::batching::{
    AlphaSdfRectRaw, Batch, BatchKind, ElementBatchesGR, GlyphRaw, RectRaw, TexturedRectRaw,
};

const SHADER_SOURCE: ShaderSource =
    make_shader_source!("uniforms.wgsl", "ui.wgsl", "alpha_sdf.wgsl");

pub struct UiScreenRenderer {
    rect_pipeline: wgpu::RenderPipeline,
    textured_rect_pipeline: wgpu::RenderPipeline,
    alpha_sdf_rect_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
}

impl UiScreenRenderer {
    /// The shader source should include `ui.wgsl` and `alpha_sdf.wgsl`.
    pub fn new(device: &wgpu::Device, shader_cache: &mut ShaderCache) -> Self {
        let shader = shader_cache.register(SHADER_SOURCE, device);
        let glyph_pipeline = create_glyph_pipeline(&shader, device);
        let rect_pipeline = create_rect_pipeline(&shader, device);
        let textured_rect_pipeline = create_textured_rect_pipeline(&shader, device);
        let alpha_sdf_rect_pipeline = create_alpha_sdf_rect_pipeline(&shader, device);

        UiScreenRenderer {
            rect_pipeline,
            textured_rect_pipeline,
            alpha_sdf_rect_pipeline,
            glyph_pipeline,
        }
    }

    pub fn render_in_new_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a TextureView,
        buffers: &'a ElementBatchesGR,
        batches: &'a Vec<Batch>,
        uniforms: &'a Uniforms,
    ) {
        let mut pass = self.new_render_pass(encoder, view);
        self.render_batches(&mut pass, buffers, batches, uniforms);
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

    pub fn render_batches<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        buffers: &'a ElementBatchesGR,
        batches: &'a Vec<Batch>,
        uniforms: &'a Uniforms,
    ) {
        if batches.is_empty() {
            return;
        }
        pass.set_bind_group(0, uniforms.bind_group(), &[]);

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;

        for batch in batches.iter() {
            let range = batch.range.start as u32..batch.range.end as u32;

            match &batch.kind {
                BatchKind::Rect => {
                    pass.set_pipeline(&self.rect_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    pass.set_vertex_buffer(0, buffers.rects.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::TexturedRect(texture) => {
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.set_pipeline(&self.textured_rect_pipeline);
                    pass.set_vertex_buffer(0, buffers.textured_rects.buffer().slice(..));
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::AlphaSdfRect(texture) => {
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.set_pipeline(&self.alpha_sdf_rect_pipeline);
                    pass.set_vertex_buffer(0, buffers.alpha_sdf_rects.buffer().slice(..));
                    pass.draw(0..VERTEX_COUNT, range);
                }
                BatchKind::Glyph(text) => {
                    pass.set_bind_group(1, &text.atlas_texture().bind_group, &[]);
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_vertex_buffer(0, buffers.glyphs.buffer().slice(..));
                    pass.draw(0..VERTEX_COUNT, range);
                }
            }
        }
    }
}
impl HotReload for UiScreenRenderer {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.glyph_pipeline = create_glyph_pipeline(&shader, device);
        self.rect_pipeline = create_rect_pipeline(&shader, device);
        self.textured_rect_pipeline = create_textured_rect_pipeline(&shader, device);
        self.alpha_sdf_rect_pipeline = create_alpha_sdf_rect_pipeline(&shader, device);
    }
}

fn create_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
) -> wgpu::RenderPipeline {
    create_pipeline::<RectRaw>(
        shader_module,
        "rect_vs",
        "rect_fs",
        device,
        &[Uniforms::cached_layout()],
    )
}

fn create_textured_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
) -> wgpu::RenderPipeline {
    create_pipeline::<TexturedRectRaw>(
        shader_module,
        "textured_rect_vs",
        "textured_rect_fs",
        device,
        &[
            Uniforms::cached_layout(),
            rgba_bind_group_layout_cached(device),
        ],
    )
}

fn create_alpha_sdf_rect_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
) -> wgpu::RenderPipeline {
    create_pipeline::<AlphaSdfRectRaw>(
        shader_module,
        "alpha_sdf_rect_vs",
        "alpha_sdf_fs",
        device,
        &[
            Uniforms::cached_layout(),
            rgba_bind_group_layout_cached(device),
        ],
    )
}

fn create_glyph_pipeline(
    shader_module: &wgpu::ShaderModule,
    device: &wgpu::Device,
) -> wgpu::RenderPipeline {
    create_pipeline::<GlyphRaw>(
        shader_module,
        "glyph_vs",
        "glyph_fs",
        device,
        &[
            Uniforms::cached_layout(),
            rgba_bind_group_layout_cached(device),
        ],
    )
}

pub fn create_pipeline<Instance: VertexT>(
    shader: &wgpu::ShaderModule,
    vs_entry: &str,
    fs_entry: &str,
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(std::any::type_name::<Instance>()),
        bind_group_layouts,
        push_constant_ranges: &[],
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
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            alpha_to_coverage_enabled: false,
            count: 1,
            mask: !0,
        },
        multiview: None,
    });
    pipeline
}
