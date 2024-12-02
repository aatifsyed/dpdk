<!-- cargo-rdme start -->

FFI-safe types for writing and transcribing C APIs.

[`&CStr`] and [`CString`] are not FFI safe.
```rust
#[deny(improper_ctypes)]
extern "C" {
    fn concat(_: &CStr, _: &CStr) -> CString;
}
```
[`&SeaStr`] and [`SeaString`] are FFI-safe equivalents.
```rust
extern "C" {
    fn concat(_: &SeaStr, _: &SeaStr) -> SeaString;
}
```
They use the non-null niche which is filled by [`Option::None`].
```c
/** may return null */
char *foo(void);
```
```rust
extern "C" fn foo() -> Option<SeaString> { .. }
assert_eq!(size_of::<Option<SeaString>>(), size_of::<*mut c_char>());
```

[`SeaBox`] is an additional owned pointer type, with a pluggable [`Allocator`].

[`&CStr`]: core::ffi::CStr
[`&SeaStr`]: SeaStr
[`CString`]: alloc::ffi::CString

<!-- cargo-rdme end -->
