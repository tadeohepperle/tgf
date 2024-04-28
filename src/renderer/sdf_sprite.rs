use std::{ops::Range, rc::Rc, sync::Arc, vec};

use crate::{
    make_shader_source, rgba_bind_group_layout_cached,
    shader::{ShaderCache},
    utils::rc_addr_as_u64,
    Aabb, BindableTexture, Camera3d, Camera3dGR, Color, GraphicsContext, GrowableBuffer, HotReload,
    RenderFormat, ShaderSource, ToRaw, Transform, TransformRaw, VertexT, VertsLayout,
};

use glam::Vec2;
use wgpu::{BindGroupLayout, BufferUsages, RenderPipeline};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct AlphaSdfParams {
    pub border_color: Color,
    pub in_to_border_cutoff: f32, // 0.5 by default
    pub in_to_border_smooth: f32,
    pub border_to_out_cutoff: f32, // always < inside_sdf_cutoff, e.g. 0.4. 0.1 will result in a very thick border.
    pub border_to_out_smooth: f32,
}

impl Default for AlphaSdfParams {
    fn default() -> Self {
        Self {
            border_color: Color::BLACK,
            in_to_border_cutoff: 0.5,
            in_to_border_smooth: 0.001,
            border_to_out_cutoff: 0.45,
            border_to_out_smooth: 0.1,
        }
    }
}

const SHADER_SOURCE: ShaderSource =
    make_shader_source!("uniforms.wgsl", "alpha_sdf.wgsl", "sdf_sprite.wgsl");
/// Immediate Mode batches Sprite Rendering.
pub struct SdfSpriteRenderer {
    instances: Vec<SpriteRaw>,
    instance_buffer: GrowableBuffer<SpriteRaw>,
    batches: Vec<SpriteBatch>,
    ctx: GraphicsContext,
    render_format: RenderFormat,
    pipeline: RenderPipeline,
    camera_layout: Arc<wgpu::BindGroupLayout>,
}

impl SdfSpriteRenderer {
    pub fn new(
        ctx: &GraphicsContext,
        camera: &Camera3dGR,
        render_format: RenderFormat,
        cache: &mut ShaderCache,
    ) -> Self {
        let ctx = ctx.clone();
        let instance_buffer = GrowableBuffer::new(&ctx.device, 32, BufferUsages::VERTEX);
        let shader = cache.register(SHADER_SOURCE, &ctx.device);

        let camera_layout = camera.bind_group_layout().clone();
        let pipeline = create_pipeline(&shader, &ctx.device, &camera_layout, render_format);

        SdfSpriteRenderer {
            instances: vec![],
            instance_buffer,
            batches: vec![],
            ctx,
            pipeline,
            render_format,
            camera_layout,
        }
    }

    /// pass the unsorted sprites to this, they will be sorted in here.
    pub fn prepare(&mut self, sprites: &mut [&SdfSprite], camera: &Camera3d) {
        // todo! frustum culling and all..
        let (instances, batches) = batch_sprites(sprites, camera);
        self.instances = instances;
        self.batches = batches;
        self.instance_buffer
            .prepare(&self.instances, &self.ctx.device, &self.ctx.queue);
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a Camera3dGR) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera.bind_group(), &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));
        for batch in self.batches.iter() {
            pass.set_bind_group(1, &batch.texture.bind_group, &[]);
            pass.draw(0..4, batch.range.clone());
        }
    }
}

impl HotReload for SdfSpriteRenderer {
    fn source(&self) -> ShaderSource {
        SHADER_SOURCE
    }

    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device) {
        self.pipeline = create_pipeline(shader, device, &self.camera_layout, self.render_format)
    }
}

pub fn batch_sprites(
    sprites: &mut [&SdfSprite],
    camera: &Camera3d,
) -> (Vec<SpriteRaw>, Vec<SpriteBatch>) {
    if sprites.is_empty() {
        return (vec![], vec![]);
    }

    sprites.sort_by(|a, b| {
        let da = a.transform.position.distance_squared(camera.transform.pos);
        let db = b.transform.position.distance_squared(camera.transform.pos);
        db.partial_cmp(&da).unwrap()
    });

    let mut instances: Vec<SpriteRaw> = vec![];
    let mut batches: Vec<SpriteBatch> = vec![];
    let mut current_batch = SpriteBatch {
        range: 0..0,
        texture: sprites.first().unwrap().texture.clone(),
    };
    for s in sprites {
        instances.push(s.to_raw());

        if s.batch_key() != current_batch.batch_key() {
            let new_batch = SpriteBatch {
                range: current_batch.range.end..(current_batch.range.end + 1),
                texture: s.texture.clone(),
            };
            let old_batch = std::mem::replace(&mut current_batch, new_batch);
            batches.push(old_batch);
        } else {
            current_batch.range.end += 1;
        }
    }
    batches.push(current_batch);

    (instances, batches)
}

pub struct SpriteBatch {
    range: Range<u32>,
    texture: Rc<BindableTexture>,
}

impl SpriteBatch {
    fn batch_key(&self) -> u64 {
        rc_addr_as_u64(&self.texture)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, PartialEq)]
pub struct SpriteRaw {
    transform: TransformRaw,
    offset: Vec2,
    size: Vec2,
    uv: Aabb,
    color: Color,
    sdf_params: AlphaSdfParams,
}

impl VertexT for SpriteRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "col1"
        wgpu::VertexFormat::Float32x4, // "col2"
        wgpu::VertexFormat::Float32x4, // "col3"
        wgpu::VertexFormat::Float32x4, // "translation"
        wgpu::VertexFormat::Float32x4, // "offset" and "size"
        wgpu::VertexFormat::Float32x4, // "uv"
        wgpu::VertexFormat::Float32x4, // "color"
        wgpu::VertexFormat::Float32x4, // "border_color"
        wgpu::VertexFormat::Float32x4, // in_to_border_cutoff, in_to_border_smooth, border_to_out_cutoff, border_to_out_smooth
    ];
}

#[derive(Debug, Clone)]
pub struct SdfSprite {
    pub texture: Rc<BindableTexture>,
    pub transform: Transform,
    pub offset: Vec2,
    pub size: Vec2,
    pub uv: Aabb,
    pub color: Color,
    pub sdf_params: AlphaSdfParams,
}

impl SdfSprite {
    fn batch_key(&self) -> u64 {
        rc_addr_as_u64(&self.texture)
    }
}

impl ToRaw for SdfSprite {
    type Raw = SpriteRaw;

    fn to_raw(&self) -> Self::Raw {
        SpriteRaw {
            transform: self.transform.to_raw(),
            offset: self.offset,
            size: self.size,
            uv: self.uv,
            color: self.color,
            sdf_params: self.sdf_params,
        }
    }
}

fn create_pipeline(
    shader: &wgpu::ShaderModule,
    device: &wgpu::Device,
    camera_layout: &BindGroupLayout,
    render_format: RenderFormat,
) -> wgpu::RenderPipeline {
    let bind_group_layouts = &[
        camera_layout,
        rgba_bind_group_layout_cached(device), // texture
    ];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sprite pipeline layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let verts_layout = VertsLayout::new().instance::<SpriteRaw>(); // no vertex type and no instances!
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: verts_layout.layout(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "alpha_sdf_fs",
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
