use std::{
    hash::{Hash, Hasher},
    num::NonZeroU64,
    ops::Add,
};

use ahash::AHasher;

/// Inner value should be a hash, not something directly chosen.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementId(NonZeroU64);

impl ElementId {
    pub fn is_none(&self) -> bool {
        *self == ElementId::NONE
    }

    pub(crate) const NONE: ElementId = ElementId(unsafe { NonZeroU64::new_unchecked(u64::MAX) });
}

impl Hash for ElementId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // because the inner value is already considered to be hashed.
        state.write_u64(self.0.into())
    }
}

impl ElementId {
    #[inline]
    pub fn combine<T: Hash>(&self, child_id: T) -> ElementId {
        let mut hasher = AHasher::default();
        self.0.hash(&mut hasher);
        child_id.hash(&mut hasher);
        ElementId(unsafe { NonZeroU64::new_unchecked(hasher.finish()) })
    }
}

impl<T> Add<T> for ElementId
where
    T: Hash,
{
    type Output = ElementId;

    fn add(self, rhs: T) -> Self::Output {
        self.combine(rhs)
    }
}

macro_rules! into_element_id {
    ($($tt:tt)*) => {
        impl From<$($tt)*> for ElementId {
            fn from(value: $($tt)*) -> Self {
                let mut hasher = AHasher::default();
                value.hash(&mut hasher);
                ElementId(unsafe { NonZeroU64::new_unchecked(hasher.finish()) })
            }
        }
    };
}

into_element_id!(&str);
into_element_id!(u32);
into_element_id!(u64);
