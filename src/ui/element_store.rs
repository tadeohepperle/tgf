use std::{
    cell::UnsafeCell,
    collections::hash_map::Entry,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::ui::{
    allocator::{SlabAllocator, SlabPtr},
    element::{ComputedBounds, Div, DivComputed, Element, Text, TextComputed},
};

use super::element_id::ElementId;
use ahash::AHashMap;
use glam::DVec2;

thread_local! {
    static STORED_ELEMENTS : ElementStore =  ElementStore::new();
}

/// Not threadsafe but that is okay!
pub struct ElementStore {
    slab_alloc: UnsafeCell<SlabAllocator<StoredElement>>,
    id_hash_map: UnsafeCell<AHashMap<ElementId, SlabPtr<StoredElement>>>,
}

impl Default for ElementStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementStore {
    pub fn new() -> Self {
        const STORED_ELEMENTS_CAPACITY: usize = 4096;
        ElementStore {
            slab_alloc: UnsafeCell::new(SlabAllocator::new(STORED_ELEMENTS_CAPACITY)),
            id_hash_map: UnsafeCell::new(Default::default()),
        }
    }

    fn slab_alloc(&self) -> &mut SlabAllocator<StoredElement> {
        unsafe { &mut *self.slab_alloc.get() }
    }

    pub fn id_hash_map(&self) -> &mut AHashMap<ElementId, SlabPtr<StoredElement>> {
        unsafe { &mut *self.id_hash_map.get() }
    }

    pub fn get_computed_bounds(id: &ElementId) -> Option<ComputedBounds> {
        STORED_ELEMENTS.with(|e| {
            let hash_map = e.id_hash_map();
            let slab_ptr = hash_map.get(id)?;
            let element = unsafe { &*slab_ptr.as_ptr() };
            let bounds = match &element.element {
                ElementWithComputed::Div((_, c)) => c.bounds,
                ElementWithComputed::Text((_, c)) => c.bounds,
            };
            Some(bounds)
        })
    }

    pub fn get_all_element_ids() -> Vec<ElementId> {
        STORED_ELEMENTS.with(|e| {
            let hash_map = e.id_hash_map();
            hash_map.keys().copied().collect()
        })
    }
}

pub struct ElementBox {
    ptr: SlabPtr<StoredElement>,
}

pub trait IntoElementBox {
    fn store(self) -> ElementBox;

    fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox;
}

// impl IntoElementBox for ElementBox {
//     fn store(self) -> ElementBox {
//         self._deref_mut().id = ElementId::NONE;
//         self
//     }

//     fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox {
//         let id: ElementId = id.into();
//         self._deref_mut().id = id;
//         todo!()
//     }
// }

impl<T: Into<Element>> IntoElementBox for T {
    fn store(self) -> ElementBox {
        ElementBox::new(StoredElement {
            element: ElementWithComputed::from_element(self.into()),
            id: ElementId::NONE,
        })
    }

    fn store_with_id(self, id: impl Into<ElementId>) -> ElementBox {
        ElementBox::new(StoredElement {
            element: ElementWithComputed::from_element(self.into()),
            id: id.into(),
        })
    }
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
        let id = element.id;

        // allocate the element in the thred local slab allocator
        let ptr = STORED_ELEMENTS.with(|e| unsafe { e.slab_alloc().alloc(element) });

        // remember where the element is if some explicit id is set.
        if !id.is_none() {
            let copied_ptr = unsafe { ptr.copy() };
            STORED_ELEMENTS.with(|e| e.id_hash_map().insert(id, copied_ptr));
        }

        // println!("new {:?}", ptr.as_ptr());

        ElementBox { ptr }
    }
}

impl Drop for ElementBox {
    fn drop(&mut self) {
        // println!("drop element {:?}", self.ptr.as_ptr());
        // println!("dealloc {self:?}");
        if !self.id().is_none() {
            STORED_ELEMENTS.with(|e| {
                let hm = e.id_hash_map();
                // remove the entry from the hashmap only for this id, if it is still pointing to the same memory
                // (could already be pointing to memory of the next iteration of elements...);
                if let Entry::Occupied(entry) = hm.entry(self.id()) {
                    if entry.get().as_ptr() == self.ptr.as_ptr() {
                        entry.remove();
                    }
                }
            })
        }
        STORED_ELEMENTS.with(|e| unsafe { e.slab_alloc().dealloc(&self.ptr) });
    }
}

impl Deref for ElementBox {
    type Target = StoredElement;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr.as_ptr() }
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

    #[inline(always)]
    pub fn element(&self) -> &ElementWithComputed {
        &self.element
    }

    #[inline(always)]
    pub fn element_mut(&mut self) -> &mut ElementWithComputed {
        &mut self._deref_mut().element
    }
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
