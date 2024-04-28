
use crate::{
    rgba_bind_group_layout_cached,
    rgba_bind_group_layout_msaa4_cached, BindableTexture, Color, RenderFormat,
    Texture,
};
use log::warn;
use winit::dpi::PhysicalSize;
pub struct ScreenTextures {
    pub render_format: RenderFormat,
    pub depth_texture: Option<DepthTexture>,
    pub hdr_msaa_texture: HdrTexture,
    pub hdr_resolve_target: HdrTexture,
}

impl ScreenTextures {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        render_format: RenderFormat,
    ) -> Self {
        let depth_texture = render_format.depth.map(|depth_format| {
            DepthTexture::create(
                device,
                width,
                height,
                depth_format,
                render_format.msaa_sample_count,
            )
        });
        let hdr_msaa_texture =
            HdrTexture::create(device, width, height, 4, render_format.color, "");
        let hdr_resolve_target =
            HdrTexture::create(device, width, height, 1, render_format.color, "");

        Self {
            render_format,
            depth_texture,
            hdr_msaa_texture,
            hdr_resolve_target,
        }
    }

    pub fn new_hdr_target_render_pass<'e>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
        color: Color,
    ) -> wgpu::RenderPass<'e> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: self.hdr_msaa_texture.view(),
            resolve_target: Some(self.hdr_resolve_target.view()),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color.into()),
                store: wgpu::StoreOp::Store,
            },
        };
        let main_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: self.depth_texture.as_ref().map(|depth_texture| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texture.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        main_render_pass
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        if let Some(depth_texture) = &mut self.depth_texture {
            depth_texture.recreate(device, size.width, size.height);
        }

        self.hdr_msaa_texture = HdrTexture::create(
            device,
            size.width,
            size.height,
            self.render_format.msaa_sample_count,
            self.render_format.color,
            "",
        );
        self.hdr_resolve_target = HdrTexture::create(
            device,
            size.width,
            size.height,
            1,
            self.render_format.color,
            "",
        );
    }
}

pub struct DepthTexture {
    texture: Texture,
    depth_format: wgpu::TextureFormat,
    sample_count: u32,
}

impl DepthTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture.view
    }

    pub fn create(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        depth_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let format = depth_format;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture: Texture {
                label: Some("Depth Texture".into()),
                texture,
                view,
                sampler,
                size,
            },
            depth_format,
            sample_count,
        }
    }

    pub fn recreate(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::create(device, width, height, self.depth_format, self.sample_count);
    }
}

#[derive(Debug)]
pub struct HdrTexture {
    texture: BindableTexture,
    /// for MSAA
    _unused_sample_count: u32,
}

impl HdrTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture.texture.view
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture.bind_group
    }

    pub fn create(
        device: &wgpu::Device,
        mut width: u32,
        mut height: u32,
        sample_count: u32,
        format: wgpu::TextureFormat,
        label: impl Into<String>,
    ) -> Self {
        let label: String = label.into();

        if width == 0 {
            warn!(
                "Attempted to create Hdr HdrTexture with size {width}x{height} with label {label}",
            );
            width = 1;
        }

        if height == 0 {
            warn!(
                "Attempted to create Hdr HdrTexture with size {width}x{height} with label {label}",
            );
            height = 1;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let descriptor = &wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[],
        };

        let texture = device.create_texture(descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let layout = match sample_count {
            1 => rgba_bind_group_layout_cached(device),
            4 => rgba_bind_group_layout_msaa4_cached(device),
            _ => panic!("Sample count {sample_count} not supported"),
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let texture = Texture {
            label: Some(label.into()),

            texture,
            view,
            sampler,
            size,
        };

        HdrTexture {
            texture: BindableTexture {
                texture,
                bind_group,
            },
            _unused_sample_count: sample_count,
        }
    }
}
