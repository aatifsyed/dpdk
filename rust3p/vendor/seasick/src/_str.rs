use core::{
    borrow::{Borrow, BorrowMut},
    cmp,
    ffi::{c_char, c_void, CStr},
    fmt::{self, Write as _},
    hash::{Hash, Hasher},
    ops, slice,
};

/// A run of bytes that is guaranteed to terminate at nul.
///
/// This type is not constructable,
/// and can only live behind a reference.
///
/// The backing buffer MUST live as long as that reference,
/// and have a length less than [`isize::MAX`].
///
/// The buffer MAY be shortened by writing nul bytes into it,
/// but MUST NOT be lengthened.
///
/// Unlike [`&CStr`](CStr),
/// an [`&SeaStr`](SeaStr) is a single pointer wide,
/// allowing it to traverse FFI boundaries.
#[repr(transparent)]
pub struct SeaStr(
    // Use c_void so users don't get `improper_ctypes` lints.
    // Ideally this would be uninhabited.
    c_void,
);

impl SeaStr {
    /// # Safety
    /// - `ptr` must not be null.
    /// - `ptr` must be valid for reads.
    /// - `ptr` must not be written to for the lifetime of self.
    /// - Invariants on [`SeaStr`] must be upheld.
    pub const unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a Self {
        &*(ptr as *const SeaStr)
    }
    /// The returned pointer MUST NOT outlive self.
    pub const fn as_ptr(&self) -> *const c_char {
        self as *const Self as _
    }
    /// # Safety
    /// - `ptr` must not be null.
    /// - `ptr` must be valid for reads and writes.
    /// - `ptr` must not be read from except through this reference for the lifetime of self.
    /// - Invariants on [`SeaStr`] must be upheld.
    pub unsafe fn from_ptr_mut<'a>(ptr: *mut c_char) -> &'a mut Self {
        &mut *(ptr as *mut SeaStr)
    }
    /// The returned pointer MUST NOT outlive self.
    pub fn as_ptr_mut(&mut self) -> *mut c_char {
        self as *mut Self as _
    }
    /// `true` if the buffer starts with nul.
    ///
    /// This is a constant-time check.
    pub const fn is_empty(&self) -> bool {
        unsafe { *self.as_ptr() == 0 }
    }
    /// The length of the buffer until (not including) the first nul.
    ///
    /// This is an `O(n)` check.
    pub const fn len(&self) -> usize {
        self.as_cstr().to_bytes().len() // going via CStr lets us be a const fn
    }
    /// The length of the buffer including the first nul.
    pub const fn len_with_nul(&self) -> usize {
        unsafe { self.len().unchecked_add(1) }
    }
    /// Return a shared reference to the buffer until (not including) the first nul.
    pub const fn bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr().cast::<u8>(), self.len()) }
    }
    /// Return a shared reference to the buffer including the first nul.
    pub const fn bytes_with_nul(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr().cast::<u8>(), self.len_with_nul()) }
    }
    pub const fn as_cstr(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.as_ptr()) }
    }
    pub const fn from_cstr(c: &CStr) -> &Self {
        unsafe { Self::from_ptr(c.as_ptr()) }
    }
    /// Access the raw bytes until (not including) the first nul.
    ///
    /// Writing a nul in this buffer will truncate it.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_ptr_mut().cast::<u8>(), self.len()) }
    }
    /// Access the raw bytes including the first nul.
    ///
    /// # Safety
    /// - This buffer MUST contain a `nul`.
    pub unsafe fn bytes_with_nul_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_ptr_mut().cast::<u8>(), self.len_with_nul()) }
    }
    /// # Panics
    /// - if `src` is empty.
    pub fn from_bytes_mut(src: &mut [u8]) -> &mut Self {
        let Some(terminator) = src.last_mut() else {
            panic!("cannot add terminator to empty slice")
        };
        *terminator = 0;
        unsafe { Self::from_ptr_mut(src.as_mut_ptr().cast::<c_char>()) }
    }
    /// If `self` is long enough,
    /// insert a `nul` such that the [`Self::len`] returns the given `len`
    pub fn trunate(&mut self, len: usize) {
        if let Some(dst) = self.get_mut(len + 1) {
            *dst = 0
        }
    }
}

impl fmt::Debug for SeaStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for chunk in self.bytes().utf8_chunks() {
            f.write_fmt(format_args!("{}", chunk.valid().escape_default()))?;
            if !chunk.invalid().is_empty() {
                f.write_char(char::REPLACEMENT_CHARACTER)?
            }
        }
        f.write_str("\"")?;
        Ok(())
    }
}
impl fmt::Display for SeaStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.bytes().utf8_chunks() {
            f.write_str(chunk.valid())?;
            if !chunk.invalid().is_empty() {
                f.write_char(char::REPLACEMENT_CHARACTER)?
            }
        }
        Ok(())
    }
}
impl PartialEq for SeaStr {
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}
impl Eq for SeaStr {}
impl Hash for SeaStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes().hash(state);
    }
}
impl Ord for SeaStr {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.bytes().cmp(other.bytes())
    }
}
impl PartialOrd for SeaStr {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl AsRef<[u8]> for SeaStr {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}
impl Borrow<[u8]> for SeaStr {
    fn borrow(&self) -> &[u8] {
        self.bytes()
    }
}
impl AsMut<[u8]> for SeaStr {
    fn as_mut(&mut self) -> &mut [u8] {
        self.bytes_mut()
    }
}
impl BorrowMut<[u8]> for SeaStr {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.bytes_mut()
    }
}
impl ops::Deref for SeaStr {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.bytes()
    }
}
impl ops::DerefMut for SeaStr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bytes_mut()
    }
}

impl<'a> From<&'a SeaStr> for &'a CStr {
    fn from(value: &'a SeaStr) -> Self {
        value.as_cstr()
    }
}
impl<'a> From<&'a CStr> for &'a SeaStr {
    fn from(value: &'a CStr) -> Self {
        SeaStr::from_cstr(value)
    }
}

impl Default for &'_ SeaStr {
    fn default() -> Self {
        SeaStr::from_cstr(c"")
    }
}
