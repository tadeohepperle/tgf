use std::{
    cell::Cell,
    ops::Deref,
    sync::{Arc, Mutex},
};

use glam::DVec2;
use wgpu::SurfaceConfiguration;
use winit::{dpi::PhysicalSize, window::Window};

use crate::ShaderCache;

#[derive(Debug, Clone)]
pub struct GraphicsContext(Arc<GraphicsContextInner>);

impl Deref for GraphicsContext {
    type Target = GraphicsContextInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct GraphicsContextInner {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: Mutex<SurfaceConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct GraphicsContextConfig {
    pub features: wgpu::Features,
    pub present_mode: wgpu::PresentMode,
    pub max_push_constant_size: u32,
    pub surface_format: wgpu::TextureFormat,
}

impl Default for GraphicsContextConfig {
    fn default() -> Self {
        Self {
            features: wgpu::Features::MULTIVIEW
                | wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | wgpu::Features::TEXTURE_BINDING_ARRAY,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            max_push_constant_size: 64,
            surface_format: wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }
}

impl GraphicsContext {
    pub fn new(
        config: GraphicsContextConfig,
        rt: &tokio::runtime::Runtime,
        window: &Window,
    ) -> anyhow::Result<Self> {
        let graphics_context =
            rt.block_on(async move { initialize_graphics_context(config, window).await })?;
        Ok(graphics_context)
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        let config = self.surface_config.lock().unwrap();
        PhysicalSize::new(config.width, config.height)
    }

    pub async fn new_async(config: GraphicsContextConfig, window: &Window) -> anyhow::Result<Self> {
        initialize_graphics_context(config, window).await
    }

    pub fn new_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            })
    }

    pub fn new_surface_texture_and_view(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let output = self
            .surface
            .get_current_texture()
            .expect("wgpu surface error");
        let view = output.texture.create_view(&Default::default());
        (output, view)
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let mut config = self.surface_config.lock().unwrap();
        config.width = size.width;
        config.height = size.height;
        self.surface.configure(&self.device, &config);
    }

    pub fn set_present_mode(&mut self, present_mode: wgpu::PresentMode) {
        let mut config = self.surface_config.lock().unwrap();
        config.present_mode = present_mode;
        self.surface.configure(&self.device, &config);
    }
}

pub async fn initialize_graphics_context(
    config: GraphicsContextConfig,
    window: &Window,
) -> anyhow::Result<GraphicsContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window)?)?
    };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: config.features,
                required_limits: wgpu::Limits {
                    max_push_constant_size: config.max_push_constant_size,
                    ..Default::default()
                },
            },
            None,
        )
        .await
        .unwrap();

    let surface_format = config.surface_format;
    let surface_caps = surface.get_capabilities(&adapter);
    if surface_caps
        .formats
        .iter()
        .all(|f| *f != config.surface_format)
    {
        panic!("SURFACE_FORMAT {surface_format:?} not found in surface caps ",)
    }

    let size = window.inner_size();
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: config.present_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);
    let surface_config = Mutex::new(surface_config);

    let ctx = GraphicsContextInner {
        instance,
        adapter,
        device,
        queue,
        surface,
        surface_config,
        surface_format,
    };
    Ok(GraphicsContext(Arc::new(ctx)))
}
