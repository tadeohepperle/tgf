use std::ops::{Add, Div, Mul};

use super::lerp::Lerp;
use glam::{vec2, DVec2, Vec2};

///  min_x, min_y form the top left corner.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Lerp for Rect {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Rect {
            pos: self.pos.lerp(other.pos, factor),
            size: self.size.lerp(other.size, factor),
        }
    }
}

impl Rect {
    pub const UNIT: Rect = Rect {
        pos: Vec2::ZERO,
        size: Vec2::ONE,
    };

    pub const ZERO: Rect = Rect {
        pos: Vec2::ZERO,
        size: Vec2::ZERO,
    };

    pub const fn new(pos: Vec2, size: Vec2) -> Self {
        Self { pos, size }
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.pos.x
            && pos.y >= self.pos.y
            && pos.x <= self.pos.x + self.size.x
            && pos.y <= self.pos.y + self.size.y
    }

    pub fn d_size(&self) -> DVec2 {
        self.size.as_dvec2()
    }
}

impl Add<Vec2> for Rect {
    type Output = Rect;

    fn add(self, rhs: Vec2) -> Self::Output {
        Rect {
            pos: self.pos + rhs,
            size: self.size,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Lerp for Aabb {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Aabb {
            min: self.min.lerp(other.min, factor),
            max: self.max.lerp(other.max, factor),
        }
    }
}

impl Add<Vec2> for Aabb {
    type Output = Aabb;

    fn add(self, rhs: Vec2) -> Self::Output {
        Aabb {
            min: self.min + rhs,
            max: self.max + rhs,
        }
    }
}

impl Mul<f32> for Aabb {
    type Output = Aabb;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.min *= rhs;
        self.max *= rhs;
        self
    }
}

impl Div<f32> for Aabb {
    type Output = Aabb;

    fn div(mut self, rhs: f32) -> Self::Output {
        self.min /= rhs;
        self.max /= rhs;
        self
    }
}

impl Div<Vec2> for Aabb {
    type Output = Aabb;

    fn div(mut self, rhs: Vec2) -> Self::Output {
        self.min /= rhs;
        self.max /= rhs;
        self
    }
}

impl Aabb {
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub const fn flipped_x(self) -> Self {
        Aabb {
            min: vec2(self.max.x, self.min.y),
            max: vec2(self.min.x, self.max.y),
        }
    }

    /// this is the default, assuming that a sprite is looking to the right, if the aabb has max_x > min_x
    pub fn looking_to_right(self) -> Self {
        if self.max.x < self.min.x {
            return self.flipped_x();
        } else {
            self
        }
    }

    pub fn looking_to_left(self) -> Self {
        if self.max.x > self.min.x {
            return self.flipped_x();
        } else {
            self
        }
    }

    pub fn overlap_area(&self, other: &Aabb) -> f32 {
        let width_overlap = self.max.x.min(other.max.x) - self.min.x.max(other.min.x);
        let height_overlap = self.max.y.min(other.max.y) - self.min.y.max(other.min.y);
        width_overlap.max(0.0) * height_overlap.max(0.0)
    }

    /// scales the Aabb around its center.
    ///
    /// Scaling with a factor of 2 results in an Aabb twice as large.
    ///
    /// Scaling with a factor of 0.5 creates a smaller Aabb, useful for zooming in at icon uv coords.
    pub fn scale(mut self, factor: f32) -> Self {
        let center = (self.min + self.max) * 0.5;
        self.min = center + (self.min - center) * factor;
        self.max = center + (self.max - center) * factor;
        self
    }

    pub fn scale_xy(mut self, factor: Vec2) -> Self {
        let center = (self.min + self.max) * 0.5;
        self.min = center + (self.min - center) * factor;
        self.max = center + (self.max - center) * factor;
        self
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.min.x && pos.y >= self.min.y && pos.x <= self.max.x && pos.y <= self.max.y
    }

    pub const UNIT: Aabb = Aabb {
        min: Vec2::ZERO,
        max: Vec2::ONE,
    };

    #[inline]
    pub fn square(center: Vec2, len: f32) -> Aabb {
        let half_len = Vec2::splat(len / 2.0);
        Aabb {
            min: center - half_len,
            max: center + half_len,
        }
    }
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[inline]
    pub fn center(&self) -> Vec2 {
        (self.max + self.min) / 2.0
    }

    /// returns a vector where x is guaranteed to be 1.0 and y is y/x so the aspect ratio.
    #[inline]
    pub fn aspect_ratio(&self) -> Vec2 {
        let size = self.size();
        vec2(1.0, size.y / size.x)
    }

    // pub fn flip_y(mut self) -> Self{
    //     self.min.y
    // }
}

impl From<Rect> for Aabb {
    fn from(rect: Rect) -> Self {
        Aabb {
            min: rect.pos,
            max: rect.pos + rect.size,
        }
    }
}

impl From<Aabb> for Rect {
    fn from(aabb: Aabb) -> Self {
        Rect {
            pos: aabb.min,
            size: aabb.max - aabb.min,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Aabb;
    #[test]
    fn scale_aabb() {
        let aabb = Aabb::UNIT.scale(0.5);
        dbg!(aabb);
    }
}
