// `meson test kvargs --print-errorlogs`

use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    ptr,
};

use bindings::*;

fn main() {
    pass(c"simple", &[(c"simple", None)]);
    pass(c"key=value", &[(c"key", Some(c"value"))]);
    pass(
        c"key=value,simple",
        &[(c"key", Some(c"value")), (c"simple", None)],
    );
    pass(
        c"simple,key=value",
        &[(c"simple", None), (c"key", Some(c"value"))],
    );
    pass(c"simple,simple", &[(c"simple", None), (c"simple", None)]);
    pass(
        c"key=value1,key=value2",
        &[(c"key", Some(c"value1")), (c"key", Some(c"value2"))],
    );
    pass(
        c"key=value1,simple,key=value2",
        &[
            (c"key", Some(c"value1")),
            (c"simple", None),
            (c"key", Some(c"value2")),
        ],
    );
    pass(
        c"simple,key=value,simple",
        &[
            (c"simple", None),
            (c"key", Some(c"value")),
            (c"simple", None),
        ],
    );

    fail_with_keys(c"allowed,notallowed", &[c"allowed"]);

    pass_with_keys(c"hello", &[c"hello"], &[(c"hello", None)]);
    pass_with_keys(
        c"hello,world",
        &[c"hello", c"world"],
        &[(c"hello", None), (c"world", None)],
    );
}

#[track_caller]
fn pass(input: &CStr, expected: &[(&CStr, Option<&CStr>)]) {
    assert_eq!(
        parse(input, None),
        Some(
            expected
                .into_iter()
                .map(|(k, v)| ((*k).into(), v.map(Into::into)))
                .collect::<Vec<_>>()
        )
    )
}
#[track_caller]
fn pass_with_keys(input: &CStr, keys: &[&CStr], expected: &[(&CStr, Option<&CStr>)]) {
    assert_eq!(
        parse(input, Some(keys)),
        Some(
            expected
                .into_iter()
                .map(|(k, v)| ((*k).into(), v.map(Into::into)))
                .collect::<Vec<_>>()
        )
    )
}

#[track_caller]
fn fail_with_keys(input: &CStr, keys: &[&CStr]) {
    assert_eq!(parse(input, Some(keys)), None);
}

/// Safe rust wrapper aroung [`rte_kvargs_parse`].
fn parse(input: &CStr, valid_keys: Option<&[&CStr]>) -> Option<Vec<(CString, Option<CString>)>> {
    let mut _valid_keys = vec![];
    let valid_keys = match valid_keys {
        Some(it) => {
            _valid_keys.extend(it.iter().map(|it| it.as_ptr()));
            _valid_keys.as_ptr()
        }
        None => ptr::null(),
    };
    unsafe {
        let ptr = rte_kvargs_parse(input.as_ptr(), valid_keys);
        match ptr.is_null() {
            true => None,
            false => {
                let mut actual = vec![];
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
                    actual.push((CString::from(key), value.map(CString::from)));
                };
                let mut f = &mut f as &mut dyn FnMut(&CStr, Option<&CStr>);
                let f = &mut f as *mut _ as *mut c_void; // fat -> thin -> opaque
                rte_kvargs_process_opt(ptr, ptr::null(), Some(process), f);
                sanity_check(&actual, ptr);
                rte_kvargs_free(ptr);
                Some(actual)
            }
        }
    }
}

/// # Safety
/// - `theirs` must be returned from [`rte_kvargs_parse`], and alive.
unsafe fn sanity_check(ours: &[(CString, Option<CString>)], theirs: *const rte_kvargs) {
    assert_eq!(ours.len(), rte_kvargs_count(theirs, ptr::null()) as usize);
    for (key, _) in ours {
        assert!(rte_kvargs_count(theirs, key.as_ptr()) >= 1);
    }
}
