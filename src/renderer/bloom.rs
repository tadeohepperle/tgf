use std::{borrow::Cow, sync::Arc};

use crate::{
    graphics_context::{GraphicsContext, GraphicsContextInner},
    make_shader_source, rgba_bind_group_layout_cached,
    uniforms::Uniforms,
    HdrTexture, HotReload, ScreenGR, ShaderCache, ShaderSource,
};
use wgpu::{BindGroupLayout, BlendComponent, BlendFactor, BlendOperation, BlendState};
use winit::dpi::PhysicalSize;

#[derive(Debug, Clone, PartialEq)]
pub struct BloomSettings {
    pub activated: bool,
    pub blend_factor: f64,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            activated: true,
            blend_factor: 0.10,
        }
    }
}

/// The input to the BloomPipeline is an HDR texture A that has a bindgroup.
/// We need to be able to use this texture A as a render attachment.
/// The steps this bloom pipeline takes, each bullet point is one render pass:
///
/// B1 has 1/2 the resolution of the original image, levels[0] has 1/4 the resolution and so on...
///
/// # 1. Downsampling:
///
/// - threshold and downsample the image, store result in B1
/// - downsample B1 store the result in levels[0]
/// - downsample levels[0] store the result in B3
/// - downsample B3 store the result in levels[1]
///
/// note: we need to be able to use B1..BX as bindgroups of textures, to sample them in fragment shaders.
/// # 2. Upsampling:
///
/// - upsample levels[1] and add it to B3
/// - upsample B3 and add it to levels[0]
/// - upsample levels[0] and add it to B1
/// - upsample B1 and add it to the original HDR image A.
///
/// This should result in a bloom.
pub struct Bloom {
    bloom_textures: BloomTextures,
    bloom_pipelines: BloomPipelines,
    settings: BloomSettings,
    color_format: wgpu::TextureFormat,
}

const SHADER_SOURCE: ShaderSource =
    make_shader_source!("uniforms.wgsl", "screen.wgsl", "bloom.wgsl");

impl Bloom {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        color_format: wgpu::TextureFormat,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let bloom_textures = BloomTextures::create(device, width, height, color_format);

        let shader = shader_cache.register(SHADER_SOURCE, device);
        let bloom_pipelines = BloomPipelines::new(&shader, device, color_format);

        Bloom {
            bloom_textures,
            bloom_pipelines,
            settings: Default::default(),
            color_format,
        }
    }

    pub fn settings_mut(&mut self) -> &mut BloomSettings {
        &mut self.settings
    }

    /// make sure this is called after graphics context is reconfigured (to match the ctx configs size)
    pub fn resize(&mut self, size: PhysicalSize<u32>, device: &wgpu::Device) {
        // recreate the textures on the gpu with the appropriate sizes
        let width = size.width;
        let height = size.height;
        self.bloom_textures = BloomTextures::create(device, width, height, self.color_format);
    }

    pub fn apply<'e>(
        &'e mut self,
        encoder: &'e mut wgpu::CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
        uniforms: &'e Uniforms,
    ) {
        if !self.settings.activated {
            return;
        }

        fn run_screen_render_pass<'e>(
            label: &str,
            encoder: &'e mut wgpu::CommandEncoder,
            input_texture: &'e wgpu::BindGroup,
            output_texture: &'e wgpu::TextureView,
            uniforms: &'e Uniforms,
            pipeline: &'e wgpu::RenderPipeline,
        ) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_texture,
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
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, uniforms.bind_group(), &[]);
            pass.set_bind_group(1, input_texture, &[]);
            pass.draw(0..3, 0..1);
        }

        // /////////////////////////////////////////////////////////////////////////////
        // downsample
        // /////////////////////////////////////////////////////////////////////////////

        run_screen_render_pass(
            "1 -> 1/2 downsample and threshold",
            encoder,
            input_texture,
            self.bloom_textures.levels[0].view(),
            uniforms,
            &self.bloom_pipelines.downsample_threshold_pipeline,
        );
        run_screen_render_pass(
            "1/2 -> 1/4 downsample",
            encoder,
            self.bloom_textures.levels[0].bind_group(),
            self.bloom_textures.levels[1].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );
        run_screen_render_pass(
            "1/4 -> 1/8 downsample",
            encoder,
            self.bloom_textures.levels[1].bind_group(),
            self.bloom_textures.levels[2].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );
        run_screen_render_pass(
            "1/8 -> 1/16 downsample",
            encoder,
            self.bloom_textures.levels[2].bind_group(),
            self.bloom_textures.levels[3].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/16 -> 1/32 downsample",
            encoder,
            self.bloom_textures.levels[3].bind_group(),
            self.bloom_textures.levels[4].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/32 -> 1/64 downsample",
            encoder,
            self.bloom_textures.levels[4].bind_group(),
            self.bloom_textures.levels[5].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/64 -> 1/128 downsample",
            encoder,
            self.bloom_textures.levels[5].bind_group(),
            self.bloom_textures.levels[6].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/128 -> 1/256 downsample",
            encoder,
            self.bloom_textures.levels[6].bind_group(),
            self.bloom_textures.levels[7].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/256 -> 1/512 downsample",
            encoder,
            self.bloom_textures.levels[7].bind_group(),
            self.bloom_textures.levels[8].view(),
            uniforms,
            &self.bloom_pipelines.downsample_pipeline,
        );

        // /////////////////////////////////////////////////////////////////////////////
        // upsample
        // /////////////////////////////////////////////////////////////////////////////

        run_screen_render_pass(
            "1/512 -> 1/256 upsample and add",
            encoder,
            self.bloom_textures.levels[8].bind_group(),
            self.bloom_textures.levels[7].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/256 -> 1/128 upsample and add",
            encoder,
            self.bloom_textures.levels[7].bind_group(),
            self.bloom_textures.levels[6].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/128 -> 1/64 upsample and add",
            encoder,
            self.bloom_textures.levels[6].bind_group(),
            self.bloom_textures.levels[5].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/64 -> 1/32 upsample and add",
            encoder,
            self.bloom_textures.levels[5].bind_group(),
            self.bloom_textures.levels[4].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/32 -> 1/16 upsample and add",
            encoder,
            self.bloom_textures.levels[4].bind_group(),
            self.bloom_textures.levels[3].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/16 -> 1/8 upsample and add",
            encoder,
            self.bloom_textures.levels[3].bind_group(),
            self.bloom_textures.levels[2].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/8 -> 1/4 upsample and add",
            encoder,
            self.bloom_textures.levels[2].bind_group(),
            self.bloom_textures.levels[1].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/4 -> 1/2 upsample and add",
            encoder,
            self.bloom_textures.levels[1].bind_group(),
            self.bloom_textures.levels[0].view(),
            uniforms,
            &self.bloom_pipelines.upsample_pipeline,
        );

        // /////////////////////////////////////////////////////////////////////////////
        // Final pass, now with blend factor to add to original image
        // /////////////////////////////////////////////////////////////////////////////

        let blend_factor = self.settings.blend_factor;
        let blend_factor = wgpu::Color {
            r: blend_factor,
            g: blend_factor,
            b: blend_factor,
            a: blend_factor,
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("1/2 -> 1 upsample and add"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_texture,
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
        pass.set_pipeline(&self.bloom_pipelines.final_upsample_pipeline);
        pass.set_blend_constant(blend_factor);
        pass.set_bind_group(0, uniforms.bind_group(), &[]);
        pass.set_bind_group(1, self.bloom_textures.levels[0].bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}

struct BloomPipelines {
    downsample_threshold_pipeline: wgpu::RenderPipeline,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    final_upsample_pipeline: wgpu::RenderPipeline,
}

impl BloomPipelines {
    pub fn new(
        shader: &wgpu::ShaderModule,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Uniforms::cached_layout(),
                rgba_bind_group_layout_cached(device),
            ],
            push_constant_ranges: &[],
        });

        let create_pipeline = |label: &str,
                               entry_point: &str,
                               blend: Option<wgpu::BlendState>|
         -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend,
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
            })
        };

        let downsample_threshold_pipeline =
            create_pipeline("Downsample Threshold", "threshold_downsample", None);
        let downsample_pipeline = create_pipeline("Downsample", "downsample", None);

        let up_blend_state = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        });

        let final_up_blend_state = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        });

        let upsample_pipeline = create_pipeline("Bloom shader", "upsample", up_blend_state);
        // only differs from upsample pipeline in the use of a constant for blending it back into the orginial image (the render target of this pipeline)
        let final_upsample_pipeline =
            create_pipeline("Bloom shader", "upsample", final_up_blend_state);

        Self {
            downsample_threshold_pipeline,
            downsample_pipeline,
            upsample_pipeline,
            final_upsample_pipeline,
        }
    }
}

const N_SIZES: usize = 9;
pub struct BloomTextures {
    levels: [HdrTexture; N_SIZES],
}

impl BloomTextures {
    pub fn create(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let level = |level: u32| -> HdrTexture {
            let size = u32::pow(2, level + 1); // level 0 -> 2, level 1 -> 4, etc..
            HdrTexture::create(
                device,
                width / size,
                height / size,
                1,
                color_format,
                format!("bloom texture level {level} (1/{})", u32::pow(2, level + 1)),
            )
        };

        BloomTextures {
            levels: [
                level(0),
                level(1),
                level(2),
                level(3),
                level(4),
                level(5),
                level(6),
                level(7),
                level(8),
            ],
        }
    }
}

impl HotReload for Bloom {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.bloom_pipelines = BloomPipelines::new(shader, device, self.color_format);
    }
}
