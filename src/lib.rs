#![feature(lazy_cell)]
#![feature(is_sorted)]

pub mod app;
pub mod buffer;
pub mod camera3d;

pub mod asset;
pub mod bucket_array;
pub mod color;
pub mod default_world;
pub mod graphics_context;
pub mod immediate_geometry;
pub mod input;
pub mod key_frames;
pub mod lerp;
pub mod rect;
pub mod renderer;
pub mod screen;
pub mod shader;
pub mod texture;
pub mod time;
pub mod transform;
pub mod ui;
pub mod utils;
pub mod vertex;
pub mod watcher;
pub mod yolo;

#[cfg(feature = "eguimod")]
pub use egui;
#[cfg(feature = "eguimod")]
pub use renderer::egui::Egui;
#[cfg(feature = "eguimod")]
pub use utils::global_values::{global_vals_get, global_vals_window};

pub use renderer::{
    bloom::{Bloom, BloomSettings, BloomTextures},
    gizmos::Gizmos,
    particles::{ParticleRenderer, ParticleSystem, ParticleSystemT, RawParticle},
    screen_textures::{DepthTexture, HdrTexture, ScreenTextures},
    sdf_sprite::{AlphaSdfParams, SdfSprite, SdfSpriteRenderer},
    tone_mapping::ToneMapping,
    RenderFormat,
};

pub use app::{AppT, Runner, RunnerCallbacks, RunnerConfig};
pub use asset::{AssetSource, AssetT, LoadingAsset};
pub use bucket_array::BucketArray;
pub use buffer::{GrowableBuffer, IndexBuffer, InstanceBuffer, ToRaw, UniformBuffer, VertexBuffer};
pub use camera3d::{Camera3DTransform, Camera3d, Camera3dGR, Camera3dRaw, Projection, Ray};
pub use color::Color;
pub use default_world::DefaultWorld;
pub use graphics_context::{GraphicsContext, GraphicsContextConfig};
pub use immediate_geometry::{ImmediateMeshQueue, ImmediateMeshRanges};
pub use input::{Input, KeyState, MouseButton, MouseButtonState, PressState};
pub use key_frames::{Easing, KeyFrames};
pub use lerp::{Lerp, Lerped};
pub use rect::{Aabb, Rect};
pub use renderer::color_mesh::ColorMeshRenderer;
pub use screen::{Screen, ScreenGR, ScreenRaw};
pub use shader::{HotReload, ShaderCache, ShaderFile, ShaderSource};
pub use texture::{
    create_white_px_texture, rgba_bind_group_layout_cached, rgba_bind_group_layout_msaa4_cached,
    BindableTexture, Texture,
};
pub use time::{Time, TimeStats};
pub use transform::{Transform, TransformRaw};
pub use vertex::{VertexT, VertsLayout};
pub use watcher::FileChangeWatcher;
pub use winit::{dpi::PhysicalSize, event::WindowEvent, keyboard::KeyCode, window::Window};
pub use yolo::{YoloCell, YoloRc};

pub mod ext {
    pub use anyhow;
    pub use bytemuck;
    pub use glam;
    pub use image;
    pub use smallvec;
    pub use tokio;
    pub use wgpu;
    pub use winit;
}
