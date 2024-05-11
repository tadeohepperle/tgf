use std::{rc::Rc, sync::Arc};

pub mod camera_controllers;

#[cfg(feature = "eguimod")]
pub mod global_values;
#[cfg(feature = "eguimod")]
pub use global_values::{global_vals_get, global_vals_show_only, global_vals_window};

/// Returns the next _^2 number such that it is greater or euqual to n.
/// Is at least 2.
pub fn next_pow2_number(n: usize) -> usize {
    let mut e = 2;
    loop {
        if e >= n {
            return e;
        }
        e *= 2;
    }
}

#[inline(always)]
pub fn center_offset(idx: usize, total: usize) -> f32 {
    (idx as f32) - ((total as f32) - 1.0) / 2.0
}

#[inline(always)]
pub fn center_offset_f64(idx: usize, total: usize) -> f64 {
    (idx as f64) - ((total as f64) - 1.0) / 2.0
}

#[inline(always)]
pub fn center_offset_f(idx_f: f32, total_f: f32) -> f32 {
    (idx_f) - ((total_f) - 1.0) / 2.0
}

pub fn rc_addr_as_u64<T>(rc: &Rc<T>) -> u64 {
    let ptr_to_rc = rc as *const Rc<T> as *const u64;
    unsafe { *ptr_to_rc }
}

pub fn arc_addr_as_u64<T>(arc: &Arc<T>) -> u64 {
    let ptr_to_rc = arc as *const Arc<T> as *const u64;
    unsafe { *ptr_to_rc }
}
