use std::{
    f32::consts::PI,
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{Aabb, Color, GraphicsContext, InstanceBuffer, Time, ToRaw, Transform, VertexT};
use glam::{vec2, vec3, Vec2, Vec3, Vec4};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

mod particle_renderer;
pub use particle_renderer::ParticleRenderer;

mod particle_system;
pub use particle_system::{ParticleSystem, ParticleSystemT};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawParticle {
    pub pos: Vec3,
    pub rotation: f32,
    pub size: Vec2,
    pub color: Color,
    pub uv: Aabb,
}

impl VertexT for RawParticle {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // pos and rotation
        wgpu::VertexFormat::Float32x2, // scale
        wgpu::VertexFormat::Float32x4, // color
        wgpu::VertexFormat::Float32x4, // uv aabb
    ];
}
