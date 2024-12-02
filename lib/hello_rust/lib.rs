use seasick::{SeaStr, SeaString};

#[no_mangle]
pub unsafe extern "C" fn hello_rust_len(ptr: Option<&SeaStr>) -> usize {
    ptr.map(SeaStr::len).unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn hello_rust_concat(left: &SeaStr, right: &SeaStr) -> Option<SeaString> {
    SeaString::try_with(left.len().checked_add(right.len())?, |initme| {
        let (left_dst, right_dst) = initme.split_at_mut(left.len());
        left_dst.copy_from_slice(left);
        right_dst.copy_from_slice(right);
    })
    .ok()
}

#[no_mangle]
pub extern "C" fn hello_rust_free(_: Option<SeaString>) {}
