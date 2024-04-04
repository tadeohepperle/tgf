use std::{fmt::Debug, rc::Rc};

use crate::{BindableTexture, Camera3DTransform, Time, Transform};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

use super::RawParticle;

pub trait ParticleSystemT {
    /// Returns true if the system is finished and should be deallocated.
    fn update(&mut self, time: &Time) -> bool;

    /// The number returned from this should stay constant throughout the lifetime of the system.
    /// Otherwise we might overrun the fixed size buffer later.
    fn max_particles_number(&self) -> usize;

    /// The raw_particles passed in here is assumed to be empty.
    fn fill_raw_particles(&mut self, raw_particles: &mut Vec<RawParticle>);

    fn texture(&self) -> Option<&BindableTexture> {
        None
    }
}

pub struct ParticleSystem {
    pub face_camera_flag: bool,
    pub transform: Transform,
    raw_particles: Vec<RawParticle>,
    buffer: wgpu::Buffer,
    max_particles: usize,
    system: Box<dyn ParticleSystemT>,
    changed_since_last_prepare: bool,
}

impl Debug for ParticleSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParticleSystem")
            .field("face_camera_flag", &self.face_camera_flag)
            .field("transform", &self.transform)
            .field("raw_particles", &self.raw_particles)
            .field("buffer", &self.buffer)
            .field("max_particles", &self.max_particles)
            .finish()
    }
}

impl ParticleSystem {
    pub fn new(
        transform: Transform,
        mut system: Box<dyn ParticleSystemT>,
        device: &wgpu::Device,
    ) -> Self {
        let mut raw_particles: Vec<RawParticle> = vec![];
        system.fill_raw_particles(&mut raw_particles);
        let max_number = system.max_particles_number();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (max_number * std::mem::size_of::<RawParticle>()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        Self {
            transform,
            raw_particles,
            buffer,
            max_particles: max_number,
            system,
            face_camera_flag: true,
            changed_since_last_prepare: true,
        }
    }

    /// Returns true if the system is finished and should be deallocated.
    pub fn update(&mut self, time: &Time) -> bool {
        let finished = self.system.update(time);
        self.raw_particles.clear();
        self.system.fill_raw_particles(&mut self.raw_particles);
        self.changed_since_last_prepare = true;
        finished
    }

    /// writes the raw particles to the queue.
    pub fn prepare(&mut self, queue: &wgpu::Queue) {
        if self.changed_since_last_prepare {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.raw_particles));
            self.changed_since_last_prepare = false;
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn n_particles(&self) -> usize {
        self.raw_particles.len()
    }

    pub fn texture(&self) -> Option<&BindableTexture> {
        self.system.texture()
    }
}
