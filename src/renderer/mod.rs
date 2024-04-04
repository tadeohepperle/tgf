pub mod color_mesh;
#[cfg(feature = "eguimod")]
pub mod egui;
pub mod gizmos;

pub mod bloom;
pub mod particles;
pub mod screen_textures;
pub mod sdf_sprite;
pub mod tone_mapping;
pub mod ui_3d;
pub mod ui_screen;

#[derive(Debug, Clone, Copy)]
pub struct RenderFormat {
    pub color: wgpu::TextureFormat,
    pub depth: Option<wgpu::TextureFormat>,
    pub msaa_sample_count: u32,
}

impl RenderFormat {
    pub const HDR_MSAA4: RenderFormat = RenderFormat {
        color: wgpu::TextureFormat::Rgba16Float,
        depth: Some(wgpu::TextureFormat::Depth32Float),
        msaa_sample_count: 4,
    };

    pub const LDR_NO_MSAA: RenderFormat = RenderFormat {
        color: wgpu::TextureFormat::Bgra8UnormSrgb,
        depth: None,
        msaa_sample_count: 1,
    };
}
