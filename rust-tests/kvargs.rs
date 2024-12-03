use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    ptr,
};

use bindings::*;

fn main() {
    test_kvargs(c"hello", None, Some(vec![(c"hello", None)]));
    test_kvargs(c"hello=world", None, Some(vec![(c"hello", Some(c"world"))]));
    test_kvargs(c"hello=[world]", None, Some(vec![(c"hello", Some(c"[world]"))]));
    test_kvargs(c"hello=[world],hello", None, Some(vec![(c"hello", Some(c"[world]")), (c"hello", None)]));
    test_kvargs(c"hello,notallowed", Some(&[c"hello"]), None);
}

#[track_caller]
fn test_kvargs(
    input: &CStr,
    valid_keys: Option<&[&CStr]>,
    expected: Option<Vec<(&CStr, Option<&CStr>)>>,
) {
    let mut _valid_keys = vec![];
    let valid_keys = match valid_keys {
        Some(it) => {
            _valid_keys.extend(it.iter().map(|it| it.as_ptr()));
            _valid_keys.as_ptr()
        }
        None => ptr::null(),
    };
    let mut _actual = vec![];
    let actual = unsafe {
        let ptr = bindings::rte_kvargs_parse(input.as_ptr(), valid_keys);
        match ptr.is_null() {
            true => None,
            false => {
                unsafe extern "C" fn process(
                    key: *const c_char,
                    value: *const c_char,
                    opaque: *mut c_void,
                ) -> c_int {
                    let f = &mut **opaque.cast::<&mut dyn FnMut(&CStr, Option<&CStr>)>();
                    f(
                        CStr::from_ptr(key),
                        match value.is_null() {
                            true => None,
                            false => Some(CStr::from_ptr(value)),
                        },
                    );
                    0
                }
                let mut f = |key: &CStr, value: Option<&CStr>| {
                    _actual.push((CString::from(key), value.map(CString::from)));
                };
                let mut f = &mut f as &mut dyn FnMut(&CStr, Option<&CStr>);
                let f = &mut f as *mut _ as *mut c_void; // fat -> thin -> opaque
                rte_kvargs_process_opt(ptr, ptr::null(), Some(process), f);
                rte_kvargs_free(ptr);
                Some(
                    _actual
                        .iter()
                        .map(|(k, v)| (k.as_c_str(), v.as_ref().map(CString::as_c_str)))
                        .collect::<Vec<_>>(),
                )
            }
        }
    };
    assert_eq!(actual, expected);
}
