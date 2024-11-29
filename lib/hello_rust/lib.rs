use std::ffi::{c_char, CStr};

use seestr::{Buf, NulTerminated};

/// # Safety
/// - If non-null, `ptr` must point to a nul-terminated string.
#[no_mangle]
pub unsafe extern "C" fn len(ptr: *const c_char) -> usize {
    match ptr.is_null() {
        true => 0,
        false => CStr::from_ptr(ptr).to_bytes().len(),
    }
}

#[no_mangle]
pub extern "C" fn concat(left: &NulTerminated, right: &NulTerminated) -> Option<Buf> {
    Buf::try_with(left.len() + right.len(), |buf| {
        let (left_dst, right_dst) = buf.split_at_mut(left.len());
        left_dst.copy_from_slice(left);
        right_dst.copy_from_slice(right);
    })
    .ok()
}

#[no_mangle]
pub extern "C" fn free_buf(_: Option<Buf>) {}
