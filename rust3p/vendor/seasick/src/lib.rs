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

pub mod nul_terminated;
