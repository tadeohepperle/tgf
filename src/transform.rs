use std::f32::consts::PI;

use glam::{vec3, Affine3A, Mat4, Quat, Vec3};

use crate::{Lerp, ToRaw, VertexT};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Lerp for Transform {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, factor),
            rotation: self.rotation.slerp(other.rotation, factor),
            scale: self.scale.lerp(other.scale, factor),
        }
    }
}

impl Transform {
    pub const ZERO: Transform = Transform {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// New Transform from Position
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Transform {
            position: vec3(x, y, z),
            rotation: Default::default(),
            scale: Vec3::ONE,
        }
    }

    pub fn face_minus_z(mut self) -> Self {
        self.rotate_y(-PI / 2.0);
        self
    }

    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = Vec3::splat(s);
        self
    }

    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    /// Rotates around the given `axis` by `angle` (in radians).
    #[inline]
    pub fn rotate_axis(&mut self, axis: Vec3, angle: f32) {
        self.rotate(Quat::from_axis_angle(axis, angle));
    }

    /// Rotates around the `X` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_x(angle));
    }

    /// Rotates around the `Y` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_y(angle));
    }

    /// Rotates around the `Z` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_z(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_z(angle));
    }

    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        let input = point * self.scale;
        let quat = self.rotation * Quat::from_vec4(input.extend(0.0)) * self.rotation.conjugate();
        quat.xyz() + point
    }
}

impl From<Vec3> for Transform {
    fn from(translation: Vec3) -> Self {
        Transform {
            position: translation,
            rotation: Default::default(),
            scale: Vec3::ONE,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl ToRaw for Transform {
    type Raw = TransformRaw;

    fn to_raw(&self) -> Self::Raw {
        TransformRaw {
            affine: Affine3A::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.position,
            )
            .into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, bytemuck::Zeroable)]
#[repr(C)]
pub struct TransformRaw {
    affine: Mat4,
}

unsafe impl bytemuck::Pod for TransformRaw {}

impl TransformRaw {
    #[inline]
    pub fn compute_transform(&self) -> Transform {
        let (scale, rotation, translation) = self.affine.to_scale_rotation_translation();
        Transform {
            position: translation,
            rotation,
            scale,
        }
    }
}

impl VertexT for TransformRaw {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4, // "col1"
        wgpu::VertexFormat::Float32x4, // "col2"
        wgpu::VertexFormat::Float32x4, // "col3"
        wgpu::VertexFormat::Float32x4, // "translation"
    ];
}
