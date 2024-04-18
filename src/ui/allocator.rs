use std::mem::size_of;
use std::ptr::{read, write, NonNull};

/// The slab allocator allocates a region of memory upfront.
/// You can allocate an element into it, or remove it from the allocator.
/// This can leave holes in the allocated space, but they are filled as
/// soon as the next element is allocated, because all the holes form a
/// linked list of indices, the head of the list being `self.next_slot`
/// and the tail of the list always being the usize::MAX value.
pub struct SlabAllocator<T> {
    ptr: *mut u8,

    /// we have reserved memory for `cap` elements.
    cap: usize,
    /// there are len elements with actual data in them
    len: usize,
    /// There are max_len elements in total, some of them filled, some of them empty.
    /// Always: len <= max_len <= cap.
    max_len: usize,
    next_slot: usize,
    marker: std::marker::PhantomData<T>,
}

#[derive(Debug)]
pub struct SlabPtr<T>(NonNull<T>);

impl<T> SlabPtr<T> {
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    #[inline(always)]
    pub unsafe fn copy(&self) -> Self {
        SlabPtr(self.0)
    }
}

impl<T> SlabAllocator<T> {
    pub fn new(cap: usize) -> Self {
        // (important to to have empty cells be big enough to write usize values into them pointing at the next free slot).
        assert!(size_of::<T>() >= size_of::<usize>());
        let layout = std::alloc::Layout::array::<T>(cap).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        unsafe {
            write(ptr as *mut usize, 1);
        }

        SlabAllocator {
            cap,
            len: 0,
            max_len: 0,
            ptr,
            next_slot: usize::MAX,
            marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn alloc(&mut self, value: T) -> SlabPtr<T> {
        let slot = self.next_slot;
        // Note: slot == usize::MAX indicates all slabs up to len == max_len are full
        if slot == usize::MAX {
            debug_assert_eq!(self.len, self.max_len);
            if self.max_len >= self.cap {
                panic!("Slab Allocator is out of memory. Regrowing is not implemented yet.");
            }
            let slot_ptr = self.ptr.add(size_of::<T>() * self.max_len) as *mut T;
            write(slot_ptr, value);
            self.len += 1;
            self.max_len += 1;
            SlabPtr(NonNull::new_unchecked(slot_ptr))
        } else {
            // Otherwise, slot points to an empty cell. In that cell the index next empty cell is found (usize::MAX if no more empty cells up to len_end).
            let slot_ptr = self.ptr.add(size_of::<T>() * slot) as *mut usize;
            self.next_slot = read(slot_ptr);
            let slot_ptr = slot_ptr as *mut T;
            write(slot_ptr, value);
            self.len += 1;
            SlabPtr(NonNull::new_unchecked(slot_ptr))
        }
    }

    pub unsafe fn dealloc(&mut self, element: &SlabPtr<T>) {
        // read, such that it is dropped properly.

        std::mem::drop(read(element.0.as_ptr()));
        let byte_offset = element.0.as_ptr() as usize - self.ptr as usize;

        debug_assert_eq!(byte_offset % size_of::<T>(), 0);
        let element_index = byte_offset / size_of::<T>();

        self.len -= 1;
        // Note: self.max_len never shrinks. Even if all elements before are filled, it is okay to have the last element being an empty element, pointing at usize::MAX instead.

        // insert into the linked list:
        write(element.0.as_ptr() as *mut usize, self.next_slot);
        self.next_slot = element_index;
    }
}

/*


maintain an usize | usize::MAX freepos

if this is usize::MAX, just insert an element at the end of this array.
if this is not empty, it points to a free position, which in turn points at the next free position, or at usize::MAX

When we want to insert a new value, we should just be able to add a value at the free_pos

what is the next free_pos?
if value at free_pos is




*/
