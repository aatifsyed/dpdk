//! Iterators for nul-terminated arrays of pointers.
//!
//! A common C idiom is to have interfaces like this
//! ```c
//! /** count the number of strings in nul-terminated `arr`. */
//! unsigned int count(char *arr[]);
//!
//! char *arr[] = { "hello", "world", NULL };
//!                                // ^^^^
//! assert(count(arr) == 2);
//! ```
//!
//! Which translates to the following.
//!
//! ```
//! # use seasick::*;
//! # use std::ffi::*;
//! mod bindings {
//!     # use std::ffi::*;
//!     extern "C" { pub fn count(arr: *mut *mut c_char) -> c_uint; }
//! }
//!
//! #[no_mangle]
//! extern "C" fn count(arr: Option<till_null::Iter<SeaStr>>) -> c_uint {
//!     arr.unwrap_or_default().count() as _
//! }
//!
//! assert_abi! { fn count = bindings::count as unsafe extern "C" fn(_) -> _ }
//!
//! ```
//!
//! Note that the niche optimization for [`Option`] is guaranteed,
//! and empty [`Default`] iterators are provided.
//!
//! # Layout
//!
//! ```text
//!   ...   ...
//! ┌─╴↑╶─┬─╴↑╶─┬─────┐
//! │ *T₀ │ *T₁ │ nul │
//! └─────┴─────┴─────┘
//!    ↑
//!   base, an aligned pointer
//! ```

use core::{fmt, marker::PhantomData, ptr::NonNull};

/// Iterator of `&T`.
///
/// See [module documentation](mod@self) for more.
#[repr(transparent)]
pub struct Iter<'a, T> {
    raw: Raw<T>,
    life: PhantomData<&'a T>,
}

impl<T> Iter<'_, T> {
    /// # Safety
    /// - `base` must be valid for [reads](std::ptr::read) (aligned, initialized, etc).
    /// - `base` must point to a nul-terminated array of pointers,
    ///   as described in the [module documentation](mod@self).
    /// - Iterated objects must not be written to for the provided lifetime.
    pub unsafe fn new(base: NonNull<*const T>) -> Self {
        Self {
            raw: Raw::new(base.cast()),
            life: PhantomData,
        }
    }
}

impl<T> Default for Iter<'_, T> {
    fn default() -> Self {
        Self {
            raw: Raw::default(),
            life: PhantomData,
        }
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            life: self.life,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        Some(unsafe { self.raw.next()?.as_ref() })
    }
}

impl<T: fmt::Debug> fmt::Debug for Iter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter")
            .field(&DebugList(self.clone()))
            .finish()
    }
}
/// Iterator of `&mut T`.
///
/// See [module documentation](mod@self) for more.
#[repr(transparent)]
pub struct IterMut<'a, T> {
    raw: Raw<T>,
    life: PhantomData<&'a mut T>,
}

impl<T> IterMut<'_, T> {
    /// # Safety
    /// - `base` must be valid for [reads](std::ptr::read) (aligned, initialized, etc).
    /// - `base` must point to a nul-terminated array of pointers,
    ///   as described in the [module documentation](mod@self).
    /// - Must have exclusive access to the iterated objects for the provided lifetime.
    pub unsafe fn new(base: NonNull<*mut T>) -> Self {
        Self {
            raw: Raw::new(base),
            life: PhantomData,
        }
    }
}

impl<T> Default for IterMut<'_, T> {
    fn default() -> Self {
        Self {
            raw: Raw::default(),
            life: PhantomData,
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        Some(unsafe { self.raw.next()?.as_mut() })
    }
}

impl<T: fmt::Debug> fmt::Debug for IterMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rest = Iter {
            raw: self.raw.clone(),
            life: PhantomData,
        };
        f.debug_tuple("IterMut").field(&DebugList(rest)).finish()
    }
}

impl<'a, T> From<IterMut<'a, T>> for Iter<'a, T> {
    fn from(value: IterMut<'a, T>) -> Self {
        let IterMut { raw, life: _ } = value;
        Self {
            raw,
            life: PhantomData,
        }
    }
}

/// Iterator of [`NonNull<T>`].
///
/// See [module documentation](mod@self) for more.
#[repr(transparent)]
pub struct Raw<T> {
    ptr: NonNull<Option<NonNull<T>>>,
}

impl<T> Raw<T> {
    /// # Safety
    /// - `base` must be valid for [reads](std::ptr::read) (aligned, initialized, etc).
    /// - `base` must point to a nul-terminated array of pointers,
    ///   as described in the [module documentation](mod@self).
    pub unsafe fn new(base: NonNull<*mut T>) -> Self {
        assert!(base.is_aligned());
        Self { ptr: base.cast() }
    }
}

impl<T> Iterator for Raw<T> {
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let here = unsafe { self.ptr.read() }?;
        unsafe { self.ptr = self.ptr.add(1) }
        Some(here)
    }
}

impl<T> Clone for Raw<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Default for Raw<T> {
    /// Empty by default.
    fn default() -> Self {
        trait Empty {
            const ARR: [Option<NonNull<Self>>; 1] = [None];
        }
        impl<T: ?Sized> Empty for T {}
        let base = NonNull::<Option<NonNull<Self>>>::from(&Self::ARR[0]);
        unsafe { Raw::new(base.cast()) }
    }
}
impl<T> fmt::Debug for Raw<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Raw")
            .field(&DebugList(self.clone()))
            .finish()
    }
}

impl<T> From<Iter<'_, T>> for Raw<T> {
    fn from(value: Iter<T>) -> Self {
        value.raw
    }
}

impl<T> From<IterMut<'_, T>> for Raw<T> {
    fn from(value: IterMut<T>) -> Self {
        value.raw
    }
}

struct DebugList<T>(T);
impl<T: Clone + IntoIterator> fmt::Debug for DebugList<T>
where
    T::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.clone()).finish()
    }
}

#[test]
fn test() {
    type T = u8;
    const LEN: usize = 2;

    let arr: [*mut T; LEN + 1] = core::array::from_fn(|ix| match ix == LEN {
        true => 0 as _,
        false => 1 as _,
    });
    let ct = unsafe { Raw::<T>::new(NonNull::from(&arr[0])) }.count();
    assert_eq!(ct, LEN);
    assert_eq!(Raw::<T>::default().count(), 0);
}
