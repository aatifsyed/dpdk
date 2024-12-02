use core::{alloc::Layout, fmt, ptr::NonNull};

/// A set of free allocation functions.
///
/// # Safety
/// - Must act like an allocator ;)
pub unsafe trait Allocator {
    /// # Safety
    /// - `size` must be less than [`isize::MAX`].
    unsafe fn alloc_unaligned(size: usize) -> Option<NonNull<u8>>;
    /// # Safety
    /// - `size` must be less than [`isize::MAX`].
    /// - `align` must be a power of two, and greater than `size_of::<c_void>()`.
    unsafe fn alloc_aligned(size: usize, align: usize) -> Option<NonNull<u8>>;
    /// # Safety
    /// - `ptr` must have been from a call to [`Allocator::alloc_aligned`] or [`Allocator::alloc_unaligned`].
    unsafe fn free(ptr: NonNull<u8>);
}

/// Use [`libc`]'s allocation functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg(feature = "libc")]
#[cfg_attr(docsrs, doc(cfg(feature = "libc")))]
pub struct Libc;

#[cfg(feature = "libc")]
#[cfg_attr(docsrs, doc(cfg(feature = "libc")))]
unsafe impl Allocator for Libc {
    unsafe fn alloc_unaligned(size: usize) -> Option<NonNull<u8>> {
        NonNull::new(unsafe { libc::malloc(size).cast::<u8>() })
    }
    unsafe fn alloc_aligned(size: usize, align: usize) -> Option<NonNull<u8>> {
        let mut ptr = core::ptr::null_mut();
        unsafe { libc::posix_memalign(&mut ptr, align, size) };
        NonNull::new(ptr.cast::<u8>())
    }
    unsafe fn free(ptr: NonNull<u8>) {
        unsafe { libc::free(ptr.as_ptr().cast()) }
    }
}

/// Returned from [`BufIn::try_of_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllocError(pub usize);

impl AllocError {
    pub fn into_layout(self) -> Layout {
        Layout::array::<u8>(self.0).unwrap()
    }
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    pub fn handle(self) -> ! {
        alloc::alloc::handle_alloc_error(self.into_layout())
    }
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("failed to allocate {} bytes", self.0))
    }
}

#[cfg(not(feature = "std"))]
impl core::error::Error for AllocError {}

#[cfg(feature = "std")]
impl std::error::Error for AllocError {}

#[cfg(feature = "std")]
impl From<AllocError> for std::io::ErrorKind {
    fn from(_: AllocError) -> Self {
        std::io::ErrorKind::OutOfMemory
    }
}
#[cfg(feature = "std")]
impl From<AllocError> for std::io::Error {
    fn from(value: AllocError) -> Self {
        std::io::Error::from(std::io::ErrorKind::from(value))
    }
}
