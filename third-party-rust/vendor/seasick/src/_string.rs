use core::{
    borrow::{Borrow, BorrowMut},
    cmp,
    ffi::c_char,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::MaybeUninit,
    ops,
    ptr::{self, NonNull},
    slice,
};

use crate::{AllocError, SeaStr, _alloc::Allocator};

/// Pointer-wide,
/// owned handle to a `nul`-terminated buffer,
/// allocated with [`malloc`](libc::malloc),
/// which is [`free`](libc::free)-ed on [`Drop`].
#[cfg(feature = "libc")]
#[cfg_attr(docsrs, doc(cfg(feature = "libc")))]
pub type SeaString = SeaStringIn<crate::_alloc::Libc>;

/// Pointer-wide,
/// owned handle to a `nul`-terminated buffer,
/// which is freed on [`Drop`].
///
/// The allocator is pluggable - see [`Allocator`].
///
/// `#[repr(transparent)]` such that `Option<Buf>` has the same layout as `*mut c_char`.
#[repr(transparent)]
pub struct SeaStringIn<A: Allocator> {
    ptr: NonNull<u8>,
    alloc: PhantomData<A>,
}

impl<A: Allocator> SeaStringIn<A> {
    /// # Safety
    /// - `ptr` must not be null.
    /// - Invariants on [`SeaString`] must be upheld.
    pub unsafe fn from_ptr(ptr: *mut c_char) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr.cast()),
            alloc: PhantomData,
        }
    }
    /// Copy `src` into the heap.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn new(src: &core::ffi::CStr) -> Self {
        Self::from_bytes(src.to_bytes())
    }
    /// This will add a nul terminator.
    ///
    /// If `src` contains an interior `0`,
    /// future methods on this [`SeaString`] will act truncated.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn from_bytes(src: &[u8]) -> Self {
        match Self::try_of_bytes(src) {
            Ok(it) => it,
            Err(e) => e.handle(),
        }
    }
    /// Copies `src` to the heap, appending an nul terminator.
    ///
    /// If `src` contains an interior `0`,
    /// future methods on this [`SeaString`] will act truncated.
    ///
    /// # Panics
    /// - if `src`s len is [`isize::MAX`].
    pub fn try_of_bytes(src: &[u8]) -> Result<Self, AllocError> {
        unsafe {
            Self::try_with_uninit(src.len(), |dst| {
                debug_assert_eq!(src.len(), dst.len());
                ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr().cast::<u8>(), dst.len());
            })
        }
    }
    /// Allocate a buffer of `len + 1`,
    /// passing a buffer of length `len` to the given function for initialization.
    ///
    /// If `f` writes (any) zeroes to the given buffer,
    /// future methods on this [`SeaString`] will act truncated.
    ///
    /// # Panics
    /// - if `len` is [`isize::MAX`].
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn with(len: usize, f: impl FnOnce(&mut [u8])) -> Self {
        match Self::try_with(len, f) {
            Ok(it) => it,
            Err(e) => e.handle(),
        }
    }
    /// Allocate a buffer of `len + 1`,
    /// passing a buffer of length `len` to the given function for initialization.
    ///
    /// # Panics
    /// - if `len` is [`isize::MAX`].
    pub fn try_with(len: usize, f: impl FnOnce(&mut [u8])) -> Result<Self, AllocError> {
        assert_ne!(len, isize::MAX as usize);
        unsafe {
            Self::try_with_uninit(len, |it| {
                let ptr = it.as_mut_ptr();
                let len = it.len();
                ptr::write_bytes(ptr, 0, len);
                f(slice::from_raw_parts_mut(ptr.cast::<u8>(), len))
            })
        }
    }
    /// Allocate a buffer of `len + 1`,
    /// passing a buffer of length `len` to the given function for initialization.
    ///
    /// # Safety
    /// - `f` must initialize the buffer it's passed.
    /// - `len` must be less than [`isize::MAX`].
    pub unsafe fn try_with_uninit(
        len: usize,
        f: impl FnOnce(&mut [MaybeUninit<u8>]),
    ) -> Result<Self, AllocError> {
        let len_with_nul = len + 1;
        let ptr = A::alloc_unaligned(len_with_nul)
            .ok_or(AllocError(len_with_nul))?
            .cast::<u8>();
        unsafe { ptr.add(len).write(0) }; // terminate
        let uninit =
            unsafe { slice::from_raw_parts_mut(ptr.cast::<MaybeUninit<u8>>().as_ptr(), len) };
        f(uninit);
        Ok(Self {
            ptr,
            alloc: PhantomData,
        })
    }
}

impl<A: Allocator> Drop for SeaStringIn<A> {
    fn drop(&mut self) {
        unsafe { A::free(self.ptr) }
    }
}

impl<A: Allocator> ops::Deref for SeaStringIn<A> {
    type Target = SeaStr;
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.cast::<SeaStr>().as_ref() }
    }
}
impl<A: Allocator> ops::DerefMut for SeaStringIn<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.cast::<SeaStr>().as_mut() }
    }
}

impl<A: Allocator> fmt::Debug for SeaStringIn<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SeaStr::fmt(self, f)
    }
}
impl<A: Allocator> fmt::Display for SeaStringIn<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SeaStr::fmt(self, f)
    }
}
impl<A1: Allocator, A2: Allocator> PartialEq<SeaStringIn<A2>> for SeaStringIn<A1> {
    fn eq(&self, other: &SeaStringIn<A2>) -> bool {
        SeaStr::eq(self, other)
    }
}
impl<A: Allocator> Eq for SeaStringIn<A> {}
impl<A: Allocator> Hash for SeaStringIn<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        SeaStr::hash(self, state)
    }
}
impl<A: Allocator> Ord for SeaStringIn<A> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        SeaStr::cmp(self, other)
    }
}
impl<A1: Allocator, A2: Allocator> PartialOrd<SeaStringIn<A2>> for SeaStringIn<A1> {
    fn partial_cmp(&self, other: &SeaStringIn<A2>) -> Option<cmp::Ordering> {
        SeaStr::partial_cmp(self, other)
    }
}
impl<A: Allocator> AsRef<SeaStr> for SeaStringIn<A> {
    fn as_ref(&self) -> &SeaStr {
        self
    }
}
impl<A: Allocator> Borrow<SeaStr> for SeaStringIn<A> {
    fn borrow(&self) -> &SeaStr {
        self
    }
}
impl<A: Allocator> AsMut<SeaStr> for SeaStringIn<A> {
    fn as_mut(&mut self) -> &mut SeaStr {
        self
    }
}
impl<A: Allocator> BorrowMut<SeaStr> for SeaStringIn<A> {
    fn borrow_mut(&mut self) -> &mut SeaStr {
        self
    }
}
impl<A: Allocator> AsRef<[u8]> for SeaStringIn<A> {
    fn as_ref(&self) -> &[u8] {
        SeaStr::as_ref(self)
    }
}
impl<A: Allocator> Borrow<[u8]> for SeaStringIn<A> {
    fn borrow(&self) -> &[u8] {
        SeaStr::borrow(self)
    }
}
impl<A: Allocator> AsMut<[u8]> for SeaStringIn<A> {
    fn as_mut(&mut self) -> &mut [u8] {
        SeaStr::as_mut(self)
    }
}
impl<A: Allocator> BorrowMut<[u8]> for SeaStringIn<A> {
    fn borrow_mut(&mut self) -> &mut [u8] {
        SeaStr::borrow_mut(self)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
impl<A: Allocator> From<&SeaStr> for SeaStringIn<A> {
    fn from(value: &SeaStr) -> Self {
        Self::new(value.as_cstr())
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
impl<A: Allocator> From<&core::ffi::CStr> for SeaStringIn<A> {
    fn from(value: &core::ffi::CStr) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "alloc")]
impl<A: Allocator> From<alloc::ffi::CString> for SeaStringIn<A> {
    fn from(value: alloc::ffi::CString) -> Self {
        Self::new(&value)
    }
}
