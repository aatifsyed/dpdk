use core::{
    borrow::{Borrow, BorrowMut},
    cmp, fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops,
    ptr::{self, NonNull},
};

use crate::{AllocError, Allocator};

/// Owned, aligned pointer to a `T`
/// allocated with [`malloc`](libc::malloc),
/// which is [`free`](libc::free)-ed on [`Drop`].
#[cfg(feature = "libc")]
#[cfg_attr(docsrs, doc(cfg(feature = "libc")))]
pub type SeaBox<T> = SeaBoxIn<T, crate::_alloc::Libc>;

/// Owned, aligned pointer to a `T`,
/// which is freed on [`Drop`].
///
/// The allocator is pluggable - see [`Allocator`].
///
/// `#[repr(transparent)]` such that `Option<SeaBox<T>>` has the same layout as `*mut T`.
#[repr(transparent)]
pub struct SeaBoxIn<T, A: Allocator> {
    ptr: NonNull<T>,
    own: PhantomData<T>,
    alloc: PhantomData<A>,
}
impl<T, A: Allocator> SeaBoxIn<T, A> {
    /// Copy `t` into the heap.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn new(t: T) -> Self {
        match Self::try_new(t) {
            Ok(it) => it,
            Err(e) => e.handle(),
        }
    }
    pub fn try_new(t: T) -> Result<Self, AllocError> {
        unsafe {
            Self::try_emplace(|u| {
                u.write(t);
            })
        }
    }
    /// # Safety
    /// - Must initialize the given memory.
    pub unsafe fn try_emplace(f: impl FnOnce(&mut MaybeUninit<T>)) -> Result<Self, AllocError> {
        let size = mem::size_of::<T>();
        let mut ptr = A::alloc_aligned(size, mem::align_of::<T>())
            .ok_or(AllocError(size))?
            .cast::<MaybeUninit<T>>();
        debug_assert!(ptr.is_aligned());
        f(unsafe { ptr.as_mut() });
        Ok(Self {
            ptr: ptr.cast(),
            own: PhantomData,
            alloc: PhantomData,
        })
    }
    pub fn unbox(self) -> T {
        let t = unsafe { self.ptr.read() };
        unsafe { A::free(self.ptr.cast::<u8>()) };
        mem::forget(self);
        t
    }
    /// # Safety
    /// - Must have been allocated by `A`.
    /// - Must be a valid `T`.
    /// - This must be the only pointer the allocation.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            own: PhantomData,
            alloc: PhantomData,
        }
    }
    pub fn into_raw(this: Self) -> *mut T {
        let ptr = this.ptr.as_ptr();
        mem::forget(this);
        ptr
    }
}

impl<T, A: Allocator> ops::Deref for SeaBoxIn<T, A> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}
impl<T, A: Allocator> ops::DerefMut for SeaBoxIn<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}
impl<T, A: Allocator> Drop for SeaBoxIn<T, A> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place::<T>(&mut **self);
            A::free(self.ptr.cast());
        }
    }
}

impl<T: fmt::Debug, A: Allocator> fmt::Debug for SeaBoxIn<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}
impl<T: fmt::Display, A: Allocator> fmt::Display for SeaBoxIn<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}
impl<T1, T2, A1: Allocator, A2: Allocator> PartialEq<SeaBoxIn<T2, A2>> for SeaBoxIn<T1, A1>
where
    T1: PartialEq<T2>,
{
    fn eq(&self, other: &SeaBoxIn<T2, A2>) -> bool {
        T1::eq(self, other)
    }
}
impl<T: Eq, A: Allocator> Eq for SeaBoxIn<T, A> {}
impl<T: Hash, A: Allocator> Hash for SeaBoxIn<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        T::hash(self, state)
    }
}
impl<T: Ord, A: Allocator> Ord for SeaBoxIn<T, A> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        T::cmp(self, other)
    }
}
impl<T1, T2, A1: Allocator, A2: Allocator> PartialOrd<SeaBoxIn<T2, A2>> for SeaBoxIn<T1, A1>
where
    T1: PartialOrd<T2>,
{
    fn partial_cmp(&self, other: &SeaBoxIn<T2, A2>) -> Option<cmp::Ordering> {
        T1::partial_cmp(self, other)
    }
}
impl<T, A: Allocator> AsRef<T> for SeaBoxIn<T, A> {
    fn as_ref(&self) -> &T {
        self
    }
}
impl<T, A: Allocator> Borrow<T> for SeaBoxIn<T, A> {
    fn borrow(&self) -> &T {
        self
    }
}
impl<T, A: Allocator> AsMut<T> for SeaBoxIn<T, A> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}
impl<T, A: Allocator> BorrowMut<T> for SeaBoxIn<T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}
