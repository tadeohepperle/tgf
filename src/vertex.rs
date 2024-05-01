use std::cell::UnsafeCell;

use smallvec::{smallvec, SmallVec};
use wgpu::{VertexAttribute, VertexBufferLayout, VertexStepMode};

pub struct VertsLayout {
    shader_location_offset: u32,
    vertexes_and_instances: SmallVec<[VertexOrInstance; 2]>,
    _tmp: UnsafeCell<SmallVec<[VertexBufferLayout<'static>; 2]>>,
}

impl Default for VertsLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl VertsLayout {
    pub fn new() -> Self {
        VertsLayout {
            shader_location_offset: 0,
            vertexes_and_instances: smallvec![],
            _tmp: UnsafeCell::new(smallvec![]),
        }
    }

    pub fn vertex<T: VertexT>(self) -> Self {
        self.add::<T>(wgpu::VertexStepMode::Vertex)
    }

    pub fn instance<T: VertexT>(self) -> Self {
        self.add::<T>(wgpu::VertexStepMode::Instance)
    }

    fn add<T: VertexT>(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        let v = VertexOrInstance {
            array_stride: std::mem::size_of::<T>() as u64,
            step_mode,
            attributes: attributes::<T>(self.shader_location_offset),
        };
        self.shader_location_offset += v.attributes.len() as u32;
        self.vertexes_and_instances.push(v);
        self
    }

    pub fn layout<'a>(&'a self) -> &[VertexBufferLayout<'a>] {
        let tmp: &'a mut _ = unsafe { &mut *self._tmp.get() };
        *tmp = smallvec![];
        for v in &self.vertexes_and_instances {
            let attributes: &'a [VertexAttribute] = &v.attributes;
            let attributes_static: &'static [VertexAttribute] =
                unsafe { std::mem::transmute(attributes) };

            tmp.push(VertexBufferLayout {
                array_stride: v.array_stride,
                step_mode: v.step_mode,
                attributes: attributes_static,
            });
        }
        tmp
    }
}

fn attributes<T: VertexT>(shader_position_offset: u32) -> SmallVec<[VertexAttribute; 8]> {
    let mut attributes: SmallVec<[VertexAttribute; 8]> = smallvec![];
    let mut offset: u64 = 0;
    let mut location = shader_position_offset;

    for a in T::ATTRIBUTES {
        attributes.push(VertexAttribute {
            format: *a,
            offset,
            shader_location: location,
        });
        offset += a.size();
        location += 1;
    }

    attributes
}

struct VertexOrInstance {
    /// The stride, in bytes, between elements of this buffer.
    pub array_stride: u64,
    /// How often this vertex buffer is "stepped" forward.
    pub step_mode: VertexStepMode,
    /// The list of attributes which comprise a single vertex.
    pub attributes: SmallVec<[VertexAttribute; 8]>,
}

pub trait VertexT: 'static + Sized + bytemuck::Pod + bytemuck::Zeroable {
    const ATTRIBUTES: &'static [wgpu::VertexFormat];
}
