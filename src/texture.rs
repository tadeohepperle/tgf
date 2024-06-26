// use image::GenericImageView;

use std::{borrow::Cow, sync::OnceLock};

use glam::{vec2, Vec2};
use image::RgbaImage;
use wgpu::{BindGroupDescriptor, BindGroupLayout};

use crate::GraphicsContext;

pub type BindableTextureRef = &'static BindableTexture;

#[derive(Debug)]
pub struct BindableTexture {
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub fn white_px_texture_cached(ctx: &GraphicsContext) -> &'static BindableTexture {
    static WHITE_PX_TEXURE_CACHED: OnceLock<BindableTexture> = OnceLock::new();
    WHITE_PX_TEXURE_CACHED.get_or_init(|| {
        let mut white_px = RgbaImage::new(1, 1);
        white_px.get_pixel_mut(0, 0).0 = [255, 255, 255, 255];
        let texture = Texture::from_image(
            &ctx.device,
            &ctx.queue,
            &white_px,
            wgpu::FilterMode::Linear,
            wgpu::AddressMode::Repeat,
        );
        BindableTexture::new(&ctx.device, texture)
    })
}

/// cached bind group layout for rgba images
pub fn rgba_bind_group_layout_cached(device: &wgpu::Device) -> &'static BindGroupLayout {
    /// ugly, use resources cache in the future.
    static _RGBA_BIND_GROUP_LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
    _RGBA_BIND_GROUP_LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    })
}

/// cached bind group layout for rgba images, with msaa 4x
pub fn rgba_bind_group_layout_msaa4_cached(device: &wgpu::Device) -> &'static BindGroupLayout {
    static _RGBA_BIND_GROUP_LAYOUT_MSAA4: OnceLock<BindGroupLayout> = OnceLock::new();
    _RGBA_BIND_GROUP_LAYOUT_MSAA4.get_or_init(|| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false, // filterable needs to be false for multisampled textures.
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    })
}

impl BindableTexture {
    pub fn size(&self) -> Vec2 {
        vec2(
            self.texture.size.width as f32,
            self.texture.size.height as f32,
        )
    }

    /// always uses RgbaBindGroupLayout.get() to get the default bind group layout without multisampling
    pub fn new(device: &wgpu::Device, texture: Texture) -> Self {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: rgba_bind_group_layout_cached(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        BindableTexture {
            texture,
            bind_group,
        }
    }
}

pub fn create_white_px_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> BindableTexture {
    let texture = Texture::create_white_px_texture(device, queue);
    BindableTexture::new(device, texture)
}

#[derive(Debug)]
pub struct Texture {
    pub label: Option<Cow<'static, str>>,

    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
}

impl Texture {
    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|e| e.as_ref())
    }

    pub fn create_white_px_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut white_px = RgbaImage::new(1, 1);
        white_px.get_pixel_mut(0, 0).0 = [255, 255, 255, 255];
        Self::from_image(
            device,
            queue,
            &white_px,
            wgpu::FilterMode::Nearest,
            wgpu::AddressMode::Repeat,
        )
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &RgbaImage,
        filter_mode: wgpu::FilterMode,
        address_move: wgpu::AddressMode,
    ) -> Self {
        let dimensions = rgba.dimensions();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let size = wgpu::Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        };
        let texture = Self::create_2d_texture(
            device,
            size.width,
            size.height,
            format,
            usage,
            filter_mode,
            address_move,
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        texture
    }

    pub fn create_2d_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        mag_filter: wgpu::FilterMode,
        address_move: wgpu::AddressMode,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        Self::create_texture(
            device,
            size,
            format,
            usage,
            wgpu::TextureDimension::D2,
            mag_filter,
            address_move,
        )
    }

    fn create_texture(
        device: &wgpu::Device,
        size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        dimension: wgpu::TextureDimension,
        mag_filter: wgpu::FilterMode,
        address_move: wgpu::AddressMode,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: address_move,
            address_mode_v: address_move,
            address_mode_w: address_move,
            mag_filter,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            label: None,
        }
    }
}
