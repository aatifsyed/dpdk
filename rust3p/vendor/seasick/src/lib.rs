//! FFI-safe types for writing and transcribing C APIs.
//!
//! [`&CStr`] and [`CString`] are not FFI safe.
//! ```compile_fail
//! # use std::ffi::{CStr, CString};
//! #[deny(improper_ctypes)]
//! extern "C" {
//!     fn concat(_: &CStr, _: &CStr) -> CString;
//! }
//! ```
//! [`&SeaStr`] and [`SeaString`] are FFI-safe equivalents.
//! ```rust
//! # use seasick::{SeaStr, SeaString};
//! # #[deny(improper_ctypes)]
//! extern "C" {
//!     fn concat(_: &SeaStr, _: &SeaStr) -> SeaString;
//! }
//! ```
//! They use the non-null niche which is filled by [`Option::None`].
//! ```c
//! /** may return null */
//! char *foo(void);
//! ```
//! ```rust
//! # stringify! {
//! extern "C" fn foo() -> Option<SeaString> { .. }
//! # };
//! # use std::{ffi::c_char, mem::size_of}; use seasick::SeaString;
//! assert_eq!(size_of::<Option<SeaString>>(), size_of::<*mut c_char>());
//! ```
//!
//! [`SeaBox`] is an additional owned pointer type, with a pluggable [`Allocator`].
//! [`till_null`] contains iterators for nul-terminated arrays of pointers.
//!
//! [`&CStr`]: core::ffi::CStr
//! [`&SeaStr`]: SeaStr
//! [`CString`]: alloc::ffi::CString

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

mod _alloc;
mod _box;
mod _str;
mod _string;

pub use _alloc::*;
pub use _box::*;
pub use _str::*;
pub use _string::*;

pub mod till_null;

/// Compile-time assertions of equality for arity, offset, size and alignment
/// of struct members and function parameters.
///
/// Suppose you are implementing a C header file:
/// ```c
/// struct args
/// {
///     const char *left;
///     const char *right;
/// };
/// char *concat(struct args);
/// ```
///
/// You could use [`bindgen`](https://docs.rs/bindgen) to create simple bindings,
/// and then write nice rust APIs separately,
/// asserting that the two are ABI compatible:
///
/// ```
/// use seasick::{SeaStr, SeaString, assert_abi};
///
/// struct Args<'a> {
///     front: &'a SeaStr,
///     back: &'a SeaStr,
/// }
///
/// #[no_mangle]
/// # extern "C" fn concat(_: Args) -> Option<SeaString> { todo!() }
/// # const _: &str = stringify! {
/// extern "C" fn concat(args: Args) -> Option<SeaString> { .. }
/// # };
///
/// assert_abi! {
///     struct Args = bindings::args { front = left, back = right };
///     fn concat = bindings::concat as unsafe extern "C" fn (_) -> _;
/// }
///
/// mod bindings {
///     /* automatically generated by rust-bindgen */
///     #[repr(C)]
///     #[derive(Debug, Copy, Clone)]
///     pub struct args {
///         pub left: *const ::std::os::raw::c_char,
///         pub right: *const ::std::os::raw::c_char,
///     }
///     extern "C" {
///         pub fn concat(arg1: args) -> *mut ::std::os::raw::c_char;
///     }
/// }
/// ```
///
/// Compilation will fail if the ABI drifts out of sync.
///
/// ```compile_fail
/// # use bindings::Args;
/// # #[expect(improper_ctypes_definitions)]
/// extern "C" fn concat(args: Args) -> String { String::new() }
///                                  // ^^^^^^ different size and alignment
/// assert_abi! {
///     fn concat = bindings::concat as unsafe extern "C" fn(_) -> _;
/// }
/// # mod bindings { #[repr(C)] pub struct Args(()); extern "C" { pub fn concat(args: Args) -> *mut ::std::os::raw::c_char; } }
/// ```
///
/// <div class="warning">
///
/// This macro only detects ABI changes (e.g size, alignment), and cannot distinguish
/// e.g `Box` from `SeaString` - it is still up to you to write (or generate) your type mappings appropriately.
///
/// </div>
#[macro_export]
macro_rules! assert_abi {
    (fn $left:path = $right:path as $ty:ty $(; $($tt:tt)*)?) => {
        const _: () = {
            let _ = $left as $ty;
            let _ = $right as $ty;
            $crate::__private::assert!($crate::__private::abi_eq($left as $ty, $right as $ty));
        };
        $(
            $crate::assert_abi!($($tt)*);
        )?
    };
    (struct $left_ty:path = $right_ty:path {
        $($left_field:ident = $right_field:ident),* $(,)?
    } $(; $($tt:tt)*)?) => {
        const _: () = {
            use $crate::__private::*;

            let left = Layout::new::<$left_ty>();
            let right = Layout::new::<$right_ty>();

            assert! {
                left.size() == right.size(),
                concat!("size mismatch between ", stringify!($left_ty), " and ", stringify!($right_ty))
            };
            assert! {
                left.align() == right.align(),
                concat!("aligment mismatch between ", stringify!($left_ty), " and ", stringify!($right_ty))
            };

            $(
                assert! {
                    offset_of!($left_ty, $left_field) == offset_of!($right_ty, $right_field),
                    concat!("mismatched offsets between ", stringify!($left_field), " and ", stringify!($right_field))
                };

                let left = field_layout(|it: &$left_ty| &it.$left_field);
                let right = field_layout(|it: &$right_ty| &it.$right_field);

                assert! {
                    left.size() == right.size(),
                    concat!("size mismatch between ", stringify!($left_field), " and ", stringify!($right_field))
                };
                assert! {
                    left.align() == right.align(),
                    concat!("aligment mismatch between ", stringify!($left_field), " and ", stringify!($right_field))
                };
            )*


            fn exhaustive($left_ty { $($left_field: _),* }: $left_ty, $right_ty { $($right_field: _),* }: $right_ty) {}
            //             ^^^^^^^ must be :path not :ty
        };
        $(
            $crate::assert_abi!($($tt)*);
        )?
    };
    ($(;)?) => {}; // trailing semi
}

#[doc(hidden)]
pub mod __private {
    pub use ::core::{alloc::Layout, assert, concat, mem::offset_of, stringify};

    pub trait AbiEq<T> {
        const ABI_EQ: bool;
    }

    macro_rules! define {
        ($($l:ident $r:ident)*) => {
            impl<LR, RR, $($l, $r),*> AbiEq<fn($($l),*) -> LR> for fn($($r),*) -> RR {
                const ABI_EQ: bool = layout_eq::<LR, RR>() $(&& layout_eq::<$l, $r>())*;
            }
            impl<LR, RR, $($l, $r),*> AbiEq<unsafe fn($($l),*) -> LR> for unsafe fn($($r),*) -> RR {
                const ABI_EQ: bool = layout_eq::<LR, RR>() $(&& layout_eq::<$l, $r>())*;
            }
            impl<LR, RR, $($l, $r),*> AbiEq<extern "C" fn($($l),*) -> LR> for extern "C" fn($($r),*) -> RR {
                const ABI_EQ: bool = layout_eq::<LR, RR>() $(&& layout_eq::<$l, $r>())*;
            }
            impl<LR, RR, $($l, $r),*> AbiEq<unsafe extern "C" fn($($l),*) -> LR> for unsafe extern "C" fn($($r),*) -> RR {
                const ABI_EQ: bool = layout_eq::<LR, RR>() $(&& layout_eq::<$l, $r>())*;
            }
        };
    }

    define!();
    define!(L0 R0);
    define!(L0 R0 L1 R1);
    define!(L0 R0 L1 R1 L2 R2);
    define!(L0 R0 L1 R1 L2 R2 L3 R3);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10 L11 R11);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10 L11 R11 L12 R12);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10 L11 R11 L12 R12 L13 R13);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10 L11 R11 L12 R12 L13 R13 L14 R14);
    define!(L0 R0 L1 R1 L2 R2 L3 R3 L4 R4 L5 R5 L6 R6 L7 R7 L8 R8 L9 R9 L10 R10 L11 R11 L12 R12 L13 R13 L14 R14 L15 R15);

    const fn layout_eq<L, R>() -> bool {
        let left = Layout::new::<L>();
        let right = Layout::new::<R>();
        left.size() == right.size() && left.align() == right.align()
    }

    pub const fn field_layout<T, U>(_: fn(&U) -> &T) -> Layout {
        Layout::new::<T>()
    }

    pub const fn abi_eq<L, R>(_: L, _: R) -> bool
    where
        L: AbiEq<R>,
        L: Copy,
        R: Copy,
    {
        L::ABI_EQ
    }
}
