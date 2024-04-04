use std::{mem::MaybeUninit, ptr::NonNull};

#[derive(Debug)]
pub struct BucketPtr<T> {
    ptr: NonNull<T>,
    bucket_index: u32,
    slot_index: u32,
}

impl<T> BucketPtr<T> {
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T> Clone for BucketPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            bucket_index: self.bucket_index,
            slot_index: self.slot_index,
        }
    }
}

impl<T> Copy for BucketPtr<T> {}

/// Inserted elements are never moved in memory.
pub struct BucketArray<T> {
    bucket_size: usize,
    len: usize,
    full_buckets: Vec<Bucket<T>>,
    unfull_buckets: Vec<Bucket<T>>,
}

struct Bucket<T> {
    bucket_index: u32,
    /// both `occupied` and `elements` are allocated once and never resized.
    occupied: Vec<bool>,
    elements: Vec<MaybeUninit<T>>,
    lowest_idx_maybe_not_occupied: usize,
    occupied_count: usize,
}

impl<T> Bucket<T> {
    fn new(bucket_index: u32, bucket_size: usize) -> Self {
        let elements: Vec<MaybeUninit<T>> = (0..bucket_size)
            .map(|_| MaybeUninit::<T>::uninit())
            .collect();

        Bucket {
            occupied: vec![false; bucket_size],
            elements,
            lowest_idx_maybe_not_occupied: 0,
            occupied_count: 0,
            bucket_index,
        }
    }

    fn insert(&mut self, element: T) -> BucketPtr<T> {
        assert!(self.occupied_count < self.elements.len());
        for i in self.lowest_idx_maybe_not_occupied..self.elements.len() {
            if !self.occupied[i] {
                // this is where we want to insert:
                let slot_ptr = &mut self.elements[i];
                *slot_ptr = MaybeUninit::new(element);

                self.lowest_idx_maybe_not_occupied += 1;
                self.occupied_count += 1;
                self.occupied[i] = true;

                let ptr = slot_ptr as *mut MaybeUninit<T> as *mut T;
                let ptr = unsafe { NonNull::new_unchecked(ptr) };
                return BucketPtr {
                    ptr,
                    bucket_index: self.bucket_index,
                    slot_index: i as u32,
                };
            }
        }
        panic!("could not find a slot to insert in the BucketVec!")
    }

    fn remove(&mut self, ptr: BucketPtr<T>) -> T {
        assert_eq!(ptr.bucket_index, self.bucket_index);
        let slot_ptr = &mut self.elements[ptr.slot_index as usize];
        assert_eq!(slot_ptr as *mut MaybeUninit<T> as *mut T, ptr.ptr.as_ptr());

        let element = std::mem::replace(slot_ptr, MaybeUninit::<T>::uninit());
        let element = unsafe { element.assume_init() };

        let i = ptr.slot_index as usize;
        if self.lowest_idx_maybe_not_occupied > i {
            self.lowest_idx_maybe_not_occupied = i;
        }
        self.occupied_count -= 1;
        self.occupied[i] = false;

        element
    }

    fn is_full(&self) -> bool {
        self.occupied_count == self.occupied.len()
    }
}

impl<T> Drop for Bucket<T> {
    fn drop(&mut self) {
        for i in 0..self.occupied.len() {
            if self.occupied[i] {
                let element = std::mem::replace(&mut self.elements[i], MaybeUninit::<T>::uninit());
                let _element = unsafe { element.assume_init() };
            }
        }
    }
}

impl<T> BucketArray<T> {
    pub fn new(bucket_size: usize) -> Self {
        BucketArray {
            bucket_size,
            full_buckets: vec![],
            unfull_buckets: vec![],
            len: 0,
        }
    }

    pub fn insert(&mut self, element: T) -> BucketPtr<T> {
        self.len += 1;
        if self.unfull_buckets.is_empty() {
            let next_bucket_index = (self.full_buckets.len() + self.unfull_buckets.len()) as u32;
            self.unfull_buckets
                .push(Bucket::new(next_bucket_index, self.bucket_size));
        }
        let bucket_to_insert = self
            .unfull_buckets
            .last_mut()
            .expect("we just created one above if was empty before");
        let ptr = bucket_to_insert.insert(element);
        if bucket_to_insert.is_full() {
            let bucket = self.unfull_buckets.pop().expect("just modified above");
            self.full_buckets.push(bucket);
        }
        return ptr;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn remove(&mut self, ptr: BucketPtr<T>) -> T {
        self.len -= 1;
        // first search through the unfull buckets:
        for b in self.unfull_buckets.iter_mut() {
            if b.bucket_index == ptr.bucket_index {
                let element = b.remove(ptr);
                return element;
            }
        }
        // then search all full buckets for the right one.
        for i in 0..self.full_buckets.len() {
            let b = &mut self.full_buckets[i];
            if b.bucket_index == ptr.bucket_index {
                let element = b.remove(ptr);

                // the button needs to be removed from full buckets and added to unfull buckets:
                let b = self.full_buckets.swap_remove(i);
                self.unfull_buckets.push(b);
                return element;
            }
        }

        panic!(
            "Bucket with id {} not found! Cannot remove element.",
            ptr.bucket_index
        )
    }

    pub fn find<'a>(&'a self, mut f: impl FnMut(&'a T) -> bool) -> Option<&'a T> {
        for b in self.full_buckets.iter() {
            for i in 0..self.bucket_size {
                let element = unsafe { b.elements[i].assume_init_ref() };
                if f(element) {
                    return Some(element);
                }
            }
        }

        for b in self.unfull_buckets.iter() {
            for i in 0..self.bucket_size {
                if b.occupied[i] {
                    let element = unsafe { b.elements[i].assume_init_ref() };
                    if f(element) {
                        return Some(element);
                    }
                }
            }
        }

        None
    }

    pub fn foreach<'a>(&'a self, mut f: impl FnMut(&'a T)) {
        for b in self.full_buckets.iter() {
            for i in 0..self.bucket_size {
                assert!(b.occupied[i]);
                let element = unsafe { b.elements[i].assume_init_ref() };
                f(element);
            }
        }

        for b in self.unfull_buckets.iter() {
            for i in 0..self.bucket_size {
                if b.occupied[i] {
                    let element = unsafe { b.elements[i].assume_init_ref() };
                    f(element);
                }
            }
        }
    }

    pub fn foreach_mut(&mut self, mut f: impl FnMut(&mut T)) {
        for b in self.full_buckets.iter_mut() {
            for i in 0..self.bucket_size {
                assert!(b.occupied[i]);
                let element = unsafe { b.elements[i].assume_init_mut() };
                f(element);
            }
        }

        for b in self.unfull_buckets.iter_mut() {
            for i in 0..self.bucket_size {
                if b.occupied[i] {
                    let element = unsafe { b.elements[i].assume_init_mut() };
                    f(element);
                }
            }
        }
    }
}

// pub struct BucketArrayIter<'a, T> {
//     bucket_array: &'a BucketArray<T>,
//     /// iterate first over full buckets, then unfull
//     in_unfull_already: bool,
//     bucket_next_i: usize,
//     slot_next_i: usize,
// }

// impl<'a, T> Iterator for BucketArrayIter<'a, T> {
//     type Item = &'a T;

//     fn next(&mut self) -> Option<Self::Item> {
//         let arr = if self.in_unfull_already {
//             &self.bucket_array.unfull_buckets
//         } else {
//             &self.bucket_array.full_buckets
//         };
//         arr[]
//     }
// }

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::{bucket_array::BucketPtr, BucketArray};

    #[test]
    fn test_insert_and_remove() {
        use rand::thread_rng;

        let mut objects: Vec<BucketPtr<String>> = vec![];

        let mut bucket_array = BucketArray::<String>::new(64);

        let mut rng = thread_rng();

        for _ in 0..10000 {
            if objects.is_empty() || (rng.gen::<f32>() > 0.4) {
                // add object
                let string: String = [
                    rng.gen::<char>(),
                    rng.gen::<char>(),
                    rng.gen::<char>(),
                    rng.gen::<char>(),
                ]
                .into_iter()
                .collect();
                let object = bucket_array.insert(string);
                objects.push(object);
            } else {
                // remove random object
                let i = rng.gen_range(0..objects.len());
                let object = objects.swap_remove(i);
                bucket_array.remove(object);
            }
        }
    }
}
