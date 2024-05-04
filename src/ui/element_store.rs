use std::{
    cell::UnsafeCell,
    collections::hash_map::Entry,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::{
    ui::{
        allocator::{SlabAllocator, SlabPtr},
        element::{ComputedBounds, Div, DivComputed, Element, Text, TextComputed},
    },
    YoloCell,
};

use super::element_id::ElementId;
use glam::DVec2;

const STORED_ELEMENTS_CAPACITY: usize = 4096;
thread_local! {
    static STORED_ELEMENTS : YoloCell<SlabAllocator<StoredElement>> = YoloCell::new(SlabAllocator::new(STORED_ELEMENTS_CAPACITY));
}

pub struct ElementBox {
    ptr: SlabPtr<StoredElement>,
}

pub trait IntoElementBox {
    fn store(self) -> ElementBox;

    fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox;
}

impl Debug for ElementBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stored_element: &StoredElement = self.deref();
        f.debug_struct("Element")
            .field("ptr", &format!("{:p}", self.ptr.as_ptr()))
            .field("element", stored_element)
            .finish()
    }
}

impl ElementBox {
    pub fn new(element: StoredElement) -> Self {
        // allocate the element in the thred local slab allocator
        let ptr = STORED_ELEMENTS.with(|e| unsafe { e.get_mut().alloc(element) });
        ElementBox { ptr }
    }
}

impl Drop for ElementBox {
    fn drop(&mut self) {
        STORED_ELEMENTS.with(|e| unsafe { e.get_mut().dealloc(&self.ptr) });
    }
}

impl Deref for ElementBox {
    type Target = StoredElement;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl DerefMut for ElementBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr.as_ptr() }
    }
}

impl ElementBox {
    #[inline(always)]
    fn _deref_mut(&self) -> &mut StoredElement {
        unsafe { &mut *self.ptr.as_ptr() }
    }

    #[inline(always)]
    pub fn id(&self) -> ElementId {
        self.id
    }

    // #[inline(always)]
    // pub fn element(&self) -> &ElementWithComputed {
    //     &self.element
    // }

    // #[inline(always)]
    // pub fn element_mut(&mut self) -> &mut ElementWithComputed {
    //     &mut self._deref_mut().element
    // }
}

#[derive(Debug)]
pub struct StoredElement {
    pub element: ElementWithComputed,
    pub id: ElementId,
}

#[repr(C)]
#[derive(Debug)]
pub enum ElementWithComputed {
    Div((Div, DivComputed)),
    Text((Text, TextComputed)),
}

impl ElementWithComputed {
    pub fn from_element(element: Element) -> Self {
        match element {
            Element::Div(div) => ElementWithComputed::Div((div, Default::default())),
            Element::Text(text) => ElementWithComputed::Text((text, Default::default())),
        }
    }

    pub fn computed_bounds_mut(&mut self) -> &mut ComputedBounds {
        match self {
            ElementWithComputed::Div((_, c)) => &mut c.bounds,
            ElementWithComputed::Text((_, c)) => &mut c.bounds,
        }
    }

    #[inline(always)]
    pub fn computed_size(&self) -> DVec2 {
        match self {
            ElementWithComputed::Div((_, c)) => c.bounds.size,
            ElementWithComputed::Text((_, c)) => c.bounds.size,
        }
    }

    pub fn div(&mut self) -> Option<&mut (Div, DivComputed)> {
        match self {
            ElementWithComputed::Div(e) => Some(e),
            ElementWithComputed::Text(_) => None,
        }
    }

    pub fn text(&mut self) -> Option<&mut (Text, TextComputed)> {
        match self {
            ElementWithComputed::Div(_) => None,
            ElementWithComputed::Text(e) => Some(e),
        }
    }
}

/// works because memory layout of Element and ElementWithComputed is the same at the start.
impl Deref for ElementWithComputed {
    type Target = Element;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const ElementWithComputed as *const Element) }
    }
}

impl DerefMut for ElementWithComputed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut ElementWithComputed as *mut Element) }
    }
}

impl IntoElementBox for ElementWithComputed {
    fn store(self) -> ElementBox {
        ElementBox::new(StoredElement {
            element: self,
            id: ElementId::NONE,
        })
    }

    fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox {
        let id: ElementId = id.into();
        ElementBox::new(StoredElement { element: self, id })
    }
}

#[cfg(test)]
pub mod tests {

    #[test]
    pub fn test_allocation() {}
}
