use glam::{DVec2, DVec3, Quat, Vec2, Vec3};

pub use tgf_macros::Lerp;

pub use simple_easing;

/// 0 -> 0
/// 0.5 -> 1
/// 1 -> 0
#[inline(always)]
pub fn quad_010(x: f32) -> f32 {
    let e = 2.0 * x - 1.0;
    1.0 - (e * e)
}

pub trait Lerp {
    fn lerp(&self, other: &Self, factor: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        *self + (*other - *self) * factor
    }
}

impl Lerp for f64 {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        *self + (*other - *self) * factor as f64
    }
}

impl Lerp for bool {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        if factor > 0.5 {
            *other
        } else {
            *self
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lerped<T: Lerp> {
    pub current: T,
    pub target: T,
}

impl<T: Lerp + Clone> Lerped<T> {
    pub fn lerp(&mut self, factor: f32) {
        self.current = self.current.lerp(&self.target, factor);
    }

    pub fn new(value: T) -> Self {
        Lerped {
            current: value.clone(),
            target: value,
        }
    }

    pub fn set_target(&mut self, value: T) {
        self.target = value;
    }

    pub fn set_current_to_target(&mut self) {
        self.current = self.target.clone();
    }
}

impl Lerp for Vec2 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Vec2::lerp(*self, *other, factor)
    }
}

impl Lerp for DVec2 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        DVec2::lerp(*self, *other, factor as f64)
    }
}

impl Lerp for Vec3 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Vec3::lerp(*self, *other, factor)
    }
}

impl Lerp for DVec3 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        DVec3::lerp(*self, *other, factor as f64)
    }
}

impl Lerp for Quat {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Quat::lerp(*self, *other, factor)
    }
}

macro_rules! impl_tuples {
    ($($id:ident $n:tt),*) => {
        impl<$( $id: Lerp ),*> Lerp for ($($id),*)
        {
            #[inline(always)]
            fn lerp(&self, other: &Self, factor: f32) -> Self {
                (
                    $( self.$n.lerp(&other.$n, factor) ),*
                )
            }
        }
    };
}

impl_tuples!(A 0, B 1);
impl_tuples!(A 0, B 1, C 2);
impl_tuples!(A 0, B 1, C 2, D 3);
impl_tuples!(A 0, B 1, C 2, D 3, E 4);
