//! A common idiom in C is to provide a nul-terminated array of pointers.
//!
//! ```text
//!   ...   ...
//! ┌─╴↑╶─┬─╴↑╶─┬─────┐
//! │ *T₀ │ *T₁ │ nul │
//! └─────┴─────┴─────┘
//!    ↑
//!   base
//! ```
//!
//! The module provides iterators for these arrays.
//!
//! Note that the pointers MUST be aligned.

use core::{fmt, marker::PhantomData, ops::RangeFrom, ptr::NonNull};

/// Iterator of `&T`.
///
/// See [module documentation](mod@self) for more.
pub struct Iter<'a, T> {
    raw: Raw<T>,
    life: PhantomData<&'a T>,
}

impl<T> Iter<'_, T> {
    /// # Safety
    /// - Must have the layout described in [module documentation](mod@self).
    /// - Pointers must be valid for reads.
    /// - Objects must be valid for the given lifetime.
    pub unsafe fn new(base: NonNull<*const T>) -> Self {
        Self {
            raw: Raw::new(base.cast()),
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
pub struct IterMut<'a, T> {
    raw: Raw<T>,
    life: PhantomData<&'a mut T>,
}

impl<T> IterMut<'_, T> {
    /// # Safety
    /// - Must have the layout described in [module documentation](mod@self).
    /// - Pointers must be valid for writes.
    /// - Objects must be valid for the given lifetime.
    /// - Objects must not be written by other threads during the given lifetime.
    pub unsafe fn new(base: NonNull<*mut T>) -> Self {
        Self {
            raw: Raw::new(base),
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
pub struct Raw<T> {
    base: Option<NonNull<Option<NonNull<T>>>>,
    off: RangeFrom<usize>,
}

impl<T> Raw<T> {
    /// # Safety
    /// - Must have the layout described in [module documentation](mod@self).
    pub unsafe fn new(base: NonNull<*mut T>) -> Self {
        Self {
            base: Some(base.cast()),
            off: 0..,
        }
    }
}

impl<T> Clone for Raw<T> {
    fn clone(&self) -> Self {
        Self {
            base: self.base,
            off: self.off.clone(),
        }
    }
}
impl<T> Default for Raw<T> {
    fn default() -> Self {
        Self {
            base: None,
            off: 0..,
        }
    }
}
impl<T> fmt::Debug for Raw<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Raw")
            .field(&DebugList(self.clone()))
            .finish()
    }
}

impl<T> Iterator for Raw<T> {
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let Self { base, off } = self;
        unsafe {
            let nxt = base.as_mut()?.add(off.next()?);
            match nxt.as_ref() {
                Some(it) => {
                    debug_assert!(it.is_aligned());
                    Some(*it)
                }
                None => {
                    *base = None;
                    None
                }
            }
        }
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
