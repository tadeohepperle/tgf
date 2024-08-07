use std::sync::Arc;

use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3, Vec4Swizzles};
use winit::dpi::PhysicalSize;

use crate::{GraphicsContext, Lerp, ToRaw};

use crate::UniformBuffer;

pub struct Camera3dGR {
    uniform: UniformBuffer<Camera3dRaw>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: Arc<wgpu::BindGroupLayout>,
}

impl Camera3dGR {
    pub fn new(ctx: &GraphicsContext, camera: &Camera3d) -> Camera3dGR {
        let uniform = UniformBuffer::new(camera.to_raw(), &ctx.device);

        let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera BindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None, // ??? is this right?
                },
                count: None,
            }],
        };
        let bind_group_layout = ctx.device.create_bind_group_layout(&layout_descriptor);
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera BindGroup"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.buffer().as_entire_binding(),
            }],
        });

        let bind_group_layout = Arc::new(bind_group_layout);

        Camera3dGR {
            uniform,
            bind_group,
            bind_group_layout,
        }
    }
    pub fn bind_group_layout(&self) -> &Arc<wgpu::BindGroupLayout> {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera3d) {
        self.uniform.update_and_prepare(camera.to_raw(), queue)
    }
}

#[derive(Debug, Clone)]
pub struct Camera3d {
    pub transform: Camera3DTransform,
    pub projection: Projection,
}

impl Camera3d {
    /// Default perspective camera
    pub fn new(width: u32, height: u32) -> Self {
        let transform = Camera3DTransform::new(vec3(-5.0, 1.0, 0.0), 0.0, 0.0);
        let projection = Projection::new_perspective(width, height, 0.8, 0.1, 5000.0);
        Self {
            transform,
            projection,
        }
    }

    /// Note: probably a bit redundant and not efficient with the matrix muls into ndc coords and remul later,
    /// but gets the job done.
    /// Returns the px_coords on the screen where a world point is projected.
    pub fn project_world_pos_to_screen_pos(&self, world_pos: Vec3) -> Vec2 {
        let view_proj = self.projection.calc_matrix() * self.transform.calc_matrix();
        let projected = view_proj * world_pos.extend(1.0);
        let xy_0_to_1 = (vec2(projected.x, -projected.y) + 1.0) / 2.0;
        xy_0_to_1 * vec2(self.projection.width as f32, self.projection.height as f32)
    }

    pub fn ray_from_screen_pos(&self, mut screen_pos: Vec2) -> Ray {
        let projection = &self.projection;
        let transform = &self.transform;

        let screen_size = vec2(projection.width as f32, projection.height as f32);
        // flip the y:
        screen_pos.y = screen_size.y - screen_pos.y;
        let ndc = screen_pos * 2.0 / screen_size - Vec2::ONE;
        let ndc_to_world = transform.calc_matrix().inverse() * projection.calc_matrix().inverse();
        let world_far_plane = ndc_to_world.project_point3(ndc.extend(1.));
        let world_near_plane: Vec3 = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

        assert!(!world_near_plane.is_nan());
        assert!(!world_far_plane.is_nan());

        let direction = (world_far_plane - world_near_plane).normalize();

        Ray {
            origin: world_near_plane,
            direction,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.projection.resize(size.width, size.height);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera3DTransform {
    pub pos: Vec3,
    /// rotation up and down
    pub pitch: f32,
    /// rotation to the sides
    pub yaw: f32,
}

impl Lerp for Camera3DTransform {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Camera3DTransform {
            pos: self.pos.lerp(other.pos, factor),
            pitch: self.pitch.lerp(&other.pitch, factor),
            yaw: self.yaw.lerp(&other.yaw, factor),
        }
    }
}

impl Camera3DTransform {
    pub fn new(pos: Vec3, pitch: f32, yaw: f32) -> Self {
        Camera3DTransform { pos, pitch, yaw }
    }

    pub fn position(&self) -> Vec3 {
        self.pos
    }

    #[inline]
    pub fn direction(&self) -> Vec3 {
        pitch_and_yaw_to_direction(self.pitch, self.yaw)
    }

    pub fn direction_ray(&self) -> Ray {
        let direction = self.direction();
        Ray {
            origin: self.pos,
            direction,
        }
    }

    /// model matrix of the camera
    pub fn calc_matrix(&self) -> Mat4 {
        let dir = self.direction();
        Mat4::look_to_rh(self.pos, dir, Vec3::Y)
    }

    pub fn forward(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();

        vec3(yaw_cos, 0.0, yaw_sin).normalize()
    }

    #[inline(always)]
    pub fn right(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();

        vec3(-yaw_sin, 0.0, yaw_cos).normalize()
    }

    #[inline(always)]
    pub fn rotation_facing_camera(&self) -> Quat {
        Quat::from_axis_angle(self.right(), self.pitch)
            * Quat::from_axis_angle(Vec3::Y, -self.yaw - std::f32::consts::PI * 0.5)
    }
}

#[inline(always)]
pub fn pitch_and_yaw_to_direction(pitch: f32, yaw: f32) -> Vec3 {
    let (pitch_sin, pitch_cos) = pitch.sin_cos();
    let (yaw_sin, yaw_cos) = yaw.sin_cos();
    vec3(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize()
}

#[derive(Debug, Clone, Copy)]
pub struct Projection {
    pub width: u32,
    pub height: u32,
    /// width / height
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
    pub kind: ProjectionKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectionKind {
    Perspective {
        fov_y_radians: f32,
    },
    Orthographic {
        // how tall the rectangle covered by the camera is in world space.
        y_height: f32,
    },
}

impl ProjectionKind {
    /// only available on perspective projection.
    pub fn fov_y_radians(&self) -> Option<f32> {
        match self {
            ProjectionKind::Perspective { fov_y_radians } => Some(*fov_y_radians),
            ProjectionKind::Orthographic { .. } => None,
        }
    }
    /// only available on perspective projection.
    pub fn fov_y_radians_mut(&mut self) -> Option<&mut f32> {
        match self {
            ProjectionKind::Perspective { fov_y_radians } => Some(fov_y_radians),
            ProjectionKind::Orthographic { .. } => None,
        }
    }
}

impl Projection {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.aspect = width as f32 / height as f32;
    }

    /// Projection Matrix
    pub fn calc_matrix(&self) -> Mat4 {
        match self.kind {
            ProjectionKind::Perspective { fov_y_radians } => {
                // perspective transform
                Mat4::perspective_rh(fov_y_radians, self.aspect, self.znear, self.zfar)
            }
            ProjectionKind::Orthographic { y_height } => {
                let top = y_height * 0.5;
                let bottom = -top;
                let right = self.aspect * top;
                let left = -right;
                Mat4::orthographic_rh(left, right, bottom, top, self.znear, self.zfar)
            }
        }
    }

    #[inline(always)]
    pub fn screen_size(&self) -> Vec2 {
        vec2(self.width as f32, self.height as f32)
    }

    pub fn new_perspective(
        width: u32,
        height: u32,
        fov_y_radians: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Projection {
            width,
            height,
            aspect: width as f32 / height as f32,
            znear,
            zfar,
            kind: ProjectionKind::Perspective { fov_y_radians },
        }
    }

    pub fn new_orthographic(width: u32, height: u32, y_height: f32, znear: f32, zfar: f32) -> Self {
        Projection {
            width,
            height,
            aspect: width as f32 / height as f32,
            znear,
            zfar,
            kind: ProjectionKind::Orthographic { y_height },
        }
    }
}

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    /// Shout out to bevy_math
    #[inline(always)]
    pub fn intersect_plane(&self, plane_origin: Vec3, plane_normal: Vec3) -> Option<f32> {
        let denominator = plane_normal.dot(self.direction);
        if denominator.abs() > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(plane_normal) / denominator;
            if distance > f32::EPSILON {
                return Some(distance);
            }
        }
        None
    }

    /// Shout out to bevy_math
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

impl ToRaw for Camera3d {
    type Raw = Camera3dRaw;

    fn to_raw(&self) -> Self::Raw {
        Camera3dRaw::new(&self.transform, &self.projection)
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera3dRaw {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
}

impl Camera3dRaw {
    fn new(camera: &Camera3DTransform, projection: &Projection) -> Self {
        let mut new = Camera3dRaw {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view: Mat4::IDENTITY.to_cols_array_2d(),
            proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        new.update_view_proj(camera, projection);
        new
    }

    fn update_view_proj(&mut self, camera: &Camera3DTransform, projection: &Projection) {
        // homogenous position:
        self.view_position = camera.pos.extend(1.0).into();

        let projection = projection.calc_matrix();
        let view = camera.calc_matrix();

        self.view_proj = (projection * view).to_cols_array_2d();
        self.view = view.to_cols_array_2d();
        self.proj = projection.to_cols_array_2d();
    }
}
