use std::{
    cmp,
    mem::MaybeUninit,
    os::raw::{c_char, c_int, c_uint, c_void},
    ptr::{self, NonNull},
};

use bindings::*;
use rust3p::{
    seasick::{nul_terminated, SeaBox, SeaStr, SeaString},
    seesaw::no_mangle as no_mangle_all,
};

pub struct Impl;

#[no_mangle_all]
impl seesaw_dpdk::KVargs for Impl {
    unsafe extern "C" fn rte_kvargs_parse(
        args: *const c_char,
        valid_keys: *const *const c_char,
    ) -> *mut rte_kvargs {
        let Some(args) = maybe::c_str(args) else {
            return ptr::null_mut();
        };
        new_with_allowlist(
            args.to_bytes(),
            maybe::iter(valid_keys as *const *const SeaStr),
        )
        .map(SeaBox::into_raw)
        .unwrap_or(ptr::null_mut())
        .cast()
    }

    unsafe extern "C" fn rte_kvargs_parse_delim(
        args: *const c_char,
        valid_keys: *const *const c_char,
        valid_ends: *const c_char,
    ) -> *mut rte_kvargs {
        let Some(valid_ends) = maybe::c_str(valid_ends) else {
            return Self::rte_kvargs_parse(args, valid_keys);
        };
        let Some(args) = maybe::c_str(args) else {
            return ptr::null_mut();
        };
        let args = match args
            .to_bytes()
            .iter()
            .position(|it| valid_ends.to_bytes().contains(it))
        {
            Some(ix) => &args.to_bytes()[..ix],
            None => args.to_bytes(),
        };
        new_with_allowlist(args, maybe::iter(valid_keys as *const *const SeaStr))
            .map(SeaBox::into_raw)
            .unwrap_or(ptr::null_mut())
            .cast()
    }

    unsafe extern "C" fn rte_kvargs_free(kvlist: *mut rte_kvargs) {
        drop(maybe::seabox(kvlist as *mut KVargs))
    }

    unsafe extern "C" fn rte_kvargs_get(
        kvlist: *const rte_kvargs,
        key: *const c_char,
    ) -> *const c_char {
        if let Some(kvlist) = kvlist.cast::<KVargs>().as_ref() {
            if let Some(needle) = maybe::c_str(key) {
                if let Some((_, Some(found))) = kvlist.iter().find(|(k, _)| k.as_cstr() == needle) {
                    return found.as_ptr();
                }
            }
        }
        ptr::null()
    }

    unsafe extern "C" fn rte_kvargs_get_with_value(
        kvlist: *const rte_kvargs,
        key: *const c_char,
        value: *const c_char,
    ) -> *const c_char {
        if let Some(kvlist) = kvlist.cast::<KVargs>().as_ref() {
            let key = maybe::c_str(key);
            let value = maybe::c_str(value);
            for (k, v) in kvlist.iter() {
                if let Some(key) = key {
                    if key != k.as_cstr() {
                        continue;
                    }
                }
                if let (Some(value), Some(v)) = (value, v) {
                    if value != v.as_cstr() {
                        continue;
                    }
                }
                if let Some(v) = v {
                    return v.as_ptr();
                }
            }
        }
        ptr::null()
    }

    unsafe extern "C" fn rte_kvargs_process(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
        handler: arg_handler_t,
        opaque_arg: *mut c_void,
    ) -> c_int {
        process(kvlist, key_match, handler, opaque_arg, false)
    }

    unsafe extern "C" fn rte_kvargs_process_opt(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
        handler: arg_handler_t,
        opaque_arg: *mut c_void,
    ) -> c_int {
        process(kvlist, key_match, handler, opaque_arg, true)
    }

    unsafe extern "C" fn rte_kvargs_count(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
    ) -> c_uint {
        let mut count = 0;
        let needle = maybe::c_str(key_match);
        if let Some(kvlist) = kvlist.cast::<KVargs>().as_ref() {
            for (k, _) in kvlist.iter() {
                let inc = match needle {
                    Some(it) => it == k.as_cstr(),
                    None => true,
                };
                if inc {
                    count += 1
                }
            }
        }
        count
    }
}

fn new_with_allowlist(
    s: &[u8],
    allowlist: Option<nul_terminated::Iter<SeaStr>>,
) -> Option<SeaBox<KVargs>> {
    let kvlist = SeaBox::try_new(KVargs::new(s)?).ok()?;
    if let Some(allowlist) = allowlist {
        for (checkme, _) in kvlist.iter() {
            if !allowlist.clone().into_iter().any(|allow| checkme != allow) {
                return None;
            }
        }
    }
    Some(kvlist)
}

unsafe fn process(
    kvlist: *const rte_kvargs,
    key_match: *const c_char,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
    allow_missing_value: bool,
) -> c_int {
    let needle = maybe::c_str(key_match);
    let (Some(kvlist), Some(cb)) = (kvlist.cast::<KVargs>().as_ref(), handler) else {
        return -1;
    };
    for (k, v) in kvlist.iter() {
        let call = match needle {
            Some(it) => it == k.as_cstr(),
            None => true,
        };
        if call {
            let rc = match allow_missing_value {
                true => {
                    let Some(v) = v else { return -1 };
                    cb(k.as_ptr(), v.as_ptr(), opaque_arg)
                }
                false => cb(
                    k.as_ptr(),
                    match v {
                        Some(it) => it.as_ptr(),
                        None => ptr::null(),
                    },
                    opaque_arg,
                ),
            };

            if rc < 0 {
                return rc;
            }
        }
    }
    0
}

/// ABI-compatible with [`rte_kvargs`].
#[repr(C)]
struct KVargs {
    /// Concatenated nul-terminated buffers for [`Self::pairs`].
    buf: Option<SeaString>,
    count: c_uint,
    /// The first [`Self::count`] are initialized,
    /// where the [`Pair::key`] and [`Pair::value`] are safe to dereference.
    pairs: [MaybeUninit<Pair>; 32],
}

/// ABI-compatible with [`rte_kvargs_pair`].
#[repr(C)]
struct Pair {
    key: NonNull<SeaStr>,
    value: Option<NonNull<SeaStr>>,
}

impl KVargs {
    fn new(mut s: &[u8]) -> Option<Self> {
        let mut pairs = [const { MaybeUninit::zeroed() }; 32];
        let mut count = 0;
        if s.is_empty() {
            return Some(Self {
                buf: None,
                count,
                pairs,
            });
        }
        let mut buf = SeaString::try_with(s.len().checked_add(1)?, |_| {}).ok()?;
        let mut iter_pairs = pairs.iter_mut().inspect(|_| count += 1);
        {
            let buf = &mut **buf;
            let occ = &mut 0;
            let mut parse = |s: &mut &[u8]| {
                let (rest, (k, v)) = kv(s).ok()?;
                let fillme = iter_pairs.next()?;
                fillme.write(Pair {
                    key: NonNull::from(bump_insert(buf, occ, k)?).cast(),
                    value: match v {
                        Some(v) => Some(NonNull::from(bump_insert(buf, occ, v)?).cast()),
                        None => None,
                    },
                });
                *s = rest;
                Some(())
            };
            parse(&mut s)?;
            while !s.is_empty() {
                let (rest, _) = tag::<_, _, rust3p::nom::error::Error<_>>(",")(s).ok()?;
                s = rest;
                parse(&mut s)?;
            }
        }
        Some(Self {
            buf: Some(buf),
            count,
            pairs,
        })
    }
    fn iter(&self) -> impl Iterator<Item = (&SeaStr, Option<&SeaStr>)> {
        self.pairs.iter().take(self.count as _).map(|it| unsafe {
            // SAFETY:
            // - `Self::new` is the only safe ctor.
            // - `Self::new` initializes `self.count` `Pair`s with pointers into `self.buf`.
            // - Lifetimes of the returned `CStr`s are tied to `self.buf`, their backing storage.
            let Pair { key, value } = it.assume_init_ref();
            (key.as_ref(), value.map(|it| it.as_ref()))
        })
    }
}

/// Copies `v` into `buf`, adding a nul-terminator, advancing `occ` returning a pointer to the copy.
///
/// Repeated calls to this function will not overlap.
fn bump_insert<'a>(buf: &'a mut [u8], occ: &mut usize, v: &[u8]) -> Option<&'a mut [u8]> {
    let b = &mut buf[*occ..];
    let len_with_null = v.len().checked_add(1)?;
    let (nullme, fillme) = b.get_mut(..len_with_null)?.split_first_mut()?;
    *nullme = 0;
    fillme.copy_from_slice(v);
    *occ += len_with_null;
    Some(fillme)
}

// Parsing
// -------

use rust3p::nom::{
    branch::alt,
    bytes::{
        complete::{is_not, tag, take_till1},
        streaming::take_until,
    },
    combinator::{recognize, rest, verify},
    multi::fold_many0,
    sequence::{delimited, separated_pair},
    IResult, Parser,
};

type ParseResult<'a, O = &'a [u8]> = IResult<&'a [u8], O>;

fn kv(input: &[u8]) -> ParseResult<(&[u8], Option<&[u8]>)> {
    alt((
        separated_pair(key, tag(&b"="[..]), value).map(|(k, v)| (k, Some(v))),
        key.map(|k| (k, None)),
    ))(input)
}

fn key(input: &[u8]) -> ParseResult<'_> {
    take_till1(|it| it == b',' || it == b'=')(input)
}

fn value(input: &[u8]) -> ParseResult<'_> {
    recognize(fold_many0(alt((list, bare)), || (), |(), _el| ()))(input)
}

fn list(input: &[u8]) -> ParseResult<'_> {
    delimited(tag(&b"["[..]), is_not(&b"]"[..]), tag(&b"]"[..]))(input)
}

fn bare<'a>(input: &'a [u8]) -> ParseResult<'a> {
    verify(
        |input: &'a [u8]| match (
            take_until::<_, _, ()>(&b","[..])(input),
            take_until::<_, _, ()>(&b"["[..])(input),
        ) {
            (Ok(by_comma), Ok(by_bracket)) => {
                Ok(cmp::min_by_key(by_comma, by_bracket, |(_, val)| val.len()))
            }
            (Ok(ok), Err(_)) | (Err(_), Ok(ok)) => Ok(ok),
            (Err(_), Err(_)) => rest(input),
        },
        |it: &[u8]| !it.is_empty(),
    )(input)
}

mod maybe {
    use std::{
        ffi::{c_char, CStr},
        ptr::NonNull,
    };

    use rust3p::seasick::{nul_terminated::Iter, SeaBox};

    pub unsafe fn c_str<'a>(ptr: *const c_char) -> Option<&'a CStr> {
        match ptr.is_null() {
            true => None,
            false => Some(CStr::from_ptr(ptr)),
        }
    }
    pub unsafe fn iter<'a, T>(base: *const *const T) -> Option<Iter<'a, T>> {
        match NonNull::new(base.cast_mut()) {
            Some(base) => Some(Iter::new(base)),
            None => None,
        }
    }
    pub unsafe fn seabox<T>(ptr: *mut T) -> Option<SeaBox<T>> {
        match ptr.is_null() {
            true => None,
            false => Some(SeaBox::from_raw(ptr)),
        }
    }
}

#[test]
fn test() {
    assert!(KVargs::new(b"hello=world,goodbye").unwrap().iter().eq([
        (
            SeaStr::from_cstr(c"hello"),
            Some(SeaStr::from_cstr(c"world")),
        ),
        (SeaStr::from_cstr(c"goodbye"), None),
    ]));
}
