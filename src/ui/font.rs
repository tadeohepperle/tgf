use std::fmt::Debug;

use ahash::AHashMap;

use crate::{utils::next_pow2_number, Aabb, BindableTexture, Texture};
use etagere::Size;
use fontdue::LineMetrics;
use glam::vec2;
use image::GenericImage;
use sdfer::{Image2d, Unorm8};
use wgpu::Extent3d;

pub type SdfFontRef = &'static SdfFont;

/// An SdfFont is meant to be created once with all the characters that you need.
/// A
pub struct SdfFont {
    font: fontdue::Font,
    /// fontsize the sdf is rasterized at. 32 or 64 is recommended.
    font_size: u32,
    /// How far out the pad_size should extend in each of the 4 directions. A value of font_size / 8 is recommended.
    pad_size: u32,
    glyphs: AHashMap<char, GlyphInfo>,
    /// a subset of glyphs
    sdf_glyphs: AHashMap<char, SdfGlyph>,
    atlas_allocator: etagere::AtlasAllocator,
    atlas_image: image::GrayImage,
    _atlas_dbg: image::RgbaImage,
    atlas_texture: BindableTexture,
}

impl Debug for SdfFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdfFont")
            .field("font", &self.font)
            .field("fontsize", &self.font_size)
            .finish()
    }
}

fn create_sdf_atlas_texture(width: u32, height: u32, device: &wgpu::Device) -> BindableTexture {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let texture = Texture {
        label: None,
        texture,
        view,
        sampler,
        size,
    };

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
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

impl SdfFont {
    pub fn new(font: fontdue::Font, font_size: u32, pad_size: u32, device: &wgpu::Device) -> Self {
        let atlas_size = next_pow2_number((font_size + 2 * pad_size) as usize * 16); // this gives us space for at least 256 glyphs, which should be enough in most cases
        let atlas_allocator =
            etagere::AtlasAllocator::new(Size::new(atlas_size as i32, atlas_size as i32));
        let atlas_image = image::GrayImage::new(atlas_size as u32, atlas_size as u32);
        let atlas_texture = create_sdf_atlas_texture(atlas_size as u32, atlas_size as u32, device);

        SdfFont {
            font,
            font_size,
            glyphs: AHashMap::new(),
            sdf_glyphs: AHashMap::new(),
            atlas_allocator,
            atlas_image,
            atlas_texture,
            _atlas_dbg: image::RgbaImage::new(atlas_size as u32, atlas_size as u32),
            pad_size,
        }
    }

    pub fn from_bytes(data: &[u8], device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let font =
            fontdue::Font::from_bytes(data, Default::default()).expect("data must be valid ttf");
        let sdf_font = Self::new_with_default_chars(font, 64, 16, device, queue);
        sdf_font
    }

    pub fn new_with_default_chars(
        font: fontdue::Font,
        fontsize: u32,
        pad_size: u32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut sdf_font = Self::new(font, fontsize, pad_size, device);

        // rasterize all the letters in the given alphabet, currently do not support any other letters:
        const ALPHABET: &str =
          "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,!:;/?|(){}[]!+-_=* \n\t'\"><~`";
        for ch in ALPHABET.chars() {
            sdf_font.add_char(ch);
        }
        sdf_font.write_atlas_to_texture(queue);
        // sdf_font.atlas_image.save("atlas.png");
        sdf_font
    }

    pub fn atlas_texture(&self) -> &BindableTexture {
        &self.atlas_texture
    }

    /// Copies the atlas image that contains all glyphs to the gpu.
    /// Should be called, after all characters that you might want have been added to the font
    pub fn write_atlas_to_texture(&self, queue: &wgpu::Queue) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.atlas_texture.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            },
            &self.atlas_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.atlas_image.width()),
                rows_per_image: None,
            },
            self.atlas_texture.texture.size,
        );
    }

    /// Adds a char to this sdf font. If it is not whitespace it is rasterized and an sdf image is computed.
    pub fn add_char(&mut self, ch: char) {
        if ch.is_whitespace() {
            let metrics = self.font.metrics(ch, self.font_size as f32);
            let metrics = Metrics::from(metrics);
            let glyph = GlyphInfo { metrics, uv: None };
            self.glyphs.insert(ch, glyph);
        } else {
            let sdf_glyph = SdfGlyph::new(ch, &self.font, self.font_size, self.pad_size);

            let (w, h) = sdf_glyph.sdf.dimensions();
            let allocation = self
                .atlas_allocator
                .allocate(Size::new(w as i32, h as i32))
                .expect("allocation failed");
            let atlas_size = self.atlas_allocator.size();
            let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);
            let uv_min_pos = vec2(
                allocation.rectangle.min.x as f32,
                allocation.rectangle.min.y as f32,
            );
            let uv_max_pos = uv_min_pos + vec2(w as f32, h as f32);
            // warning: the allocation.rectangle might be larger than the (w,h) of the sdf image.
            // so we can only use the top left corner reliably, and need to add the width and height on top ourselves.
            let uv = Aabb::new(uv_min_pos / atlas_size, uv_max_pos / atlas_size);

            // write the sdf into the big texture image
            self.atlas_image
                .copy_from(
                    &sdf_glyph.sdf,
                    allocation.rectangle.min.x as u32,
                    allocation.rectangle.min.y as u32,
                )
                .expect("copy from sdf_glyph image to atlas_image failed");

            let glyph = GlyphInfo {
                metrics: sdf_glyph.metrics_with_pad,
                uv: Some(uv),
            };
            self.sdf_glyphs.insert(ch, sdf_glyph);
            self.glyphs.insert(ch, glyph);
        }
    }

    pub fn line_metrics(&self, font_size_px: f32) -> LineMetrics {
        self.font
            .horizontal_line_metrics(font_size_px)
            .expect("Line Metrics need to be found")
    }

    pub fn glyph_info(&self, ch: char, font_size_px: f32) -> GlyphInfo {
        if let Some(glyph) = self.glyphs.get(&ch) {
            let scale = font_size_px / self.font_size as f32;
            GlyphInfo {
                metrics: glyph.metrics.scale(scale),
                uv: glyph.uv,
            }
        } else {
            panic!("the character {ch} is not rasterized yet");
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub xmin: f32,
    pub ymin: f32,
    pub width: f32,
    pub height: f32,
    // advance of the glyph in x directon
    pub advance: f32,
}

impl Metrics {
    #[inline(always)]
    pub fn scale(&self, scale: f32) -> Metrics {
        Metrics {
            xmin: self.xmin * scale,
            ymin: self.ymin * scale,
            width: self.width * scale,
            height: self.height * scale,
            advance: self.advance * scale,
        }
    }
}

impl From<fontdue::Metrics> for Metrics {
    fn from(metrics: fontdue::Metrics) -> Self {
        let bounds = metrics.bounds;
        Metrics {
            xmin: bounds.xmin,
            ymin: bounds.ymin,
            width: bounds.width,
            height: bounds.height,
            advance: metrics.advance_width,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    pub metrics: Metrics,
    /// None if whitespace
    pub uv: Option<Aabb>,
}

struct SdfGlyph {
    _char: char,
    _font_size: u32,
    _metrics: Metrics,
    // pad is font_size/8 in each direction. This does not depend on the glyphs size.
    _pad: u32,
    // metrics but with pad px in all directions. This does not affect the advance.
    metrics_with_pad: Metrics,
    _gray: image::GrayImage,
    sdf: image::GrayImage,
}

impl SdfGlyph {
    pub fn new(ch: char, font: &fontdue::Font, font_size: u32, pad: u32) -> Self {
        assert!(!ch.is_whitespace());
        let (metrics, img) = font.rasterize(ch, font_size as f32);
        let gray =
            image::GrayImage::from_raw(metrics.width as u32, metrics.height as u32, img).unwrap();

        let metrics = Metrics::from(metrics);
        let metrics_with_pad = Metrics {
            xmin: metrics.xmin - pad as f32,
            ymin: metrics.ymin - pad as f32,
            width: metrics.width + (2 * pad) as f32,
            height: metrics.height + (2 * pad) as f32,
            advance: metrics.advance,
        };

        let mut gray_for_sdfer: Image2d<Unorm8> = From::from(gray.clone());

        let (sdf_glyph, _) = sdfer::esdt::glyph_to_sdf(
            &mut gray_for_sdfer,
            sdfer::esdt::Params {
                pad: pad as usize,
                radius: pad as f32,
                cutoff: 0.5,
                solidify: true,
                preprocess: true,
            },
            None,
        );

        let sdf = image::GrayImage::from(sdf_glyph);

        SdfGlyph {
            _char: ch,
            _font_size: font_size,
            _metrics: metrics,
            _pad: pad,
            metrics_with_pad,
            _gray: gray,
            sdf,
        }
    }
}
