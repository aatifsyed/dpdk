use std::ffi::{c_char, CStr};

/// # Safety
/// - If non-null, `ptr` must point to a nul-terminated string.
pub unsafe extern "C" fn len(ptr: *const c_char) -> usize {
    match ptr.is_null() {
        true => 0,
        false => CStr::from_ptr(ptr).to_bytes().len(),
    }
}
