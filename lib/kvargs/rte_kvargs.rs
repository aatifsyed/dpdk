use std::{
    cmp,
    mem::MaybeUninit,
    num::Saturating,
    os::raw::{c_int, c_uint, c_void},
    ptr::{self, NonNull},
};

use bindings::*;
use rust3p::seasick::{assert_abi, till_null, SeaBox, SeaStr, SeaString};

assert_abi!(fn bindings::rte_kvargs_parse = rte_kvargs_parse as unsafe extern "C" fn(_, _) -> _);
#[no_mangle]
pub extern "C" fn rte_kvargs_parse(
    args: Option<&SeaStr>,
    valid_keys: Option<till_null::Iter<SeaStr>>,
) -> Option<SeaBox<KVargs>> {
    new_with_allowlist(args?.bytes(), valid_keys)
}

assert_abi!(fn bindings::rte_kvargs_parse_delim = rte_kvargs_parse_delim as unsafe extern "C" fn(_, _, _) -> _);
#[no_mangle]
extern "C" fn rte_kvargs_parse_delim(
    args: Option<&SeaStr>,
    valid_keys: Option<till_null::Iter<SeaStr>>,
    valid_ends: Option<&SeaStr>,
) -> Option<SeaBox<KVargs>> {
    let Some(trim_to) = valid_ends else {
        return rte_kvargs_parse(args, valid_keys);
    };
    let args: &[u8] = args?.bytes();

    let args = match args.iter().position(|it| trim_to.bytes().contains(it)) {
        Some(ix) => &args[..ix],
        None => args,
    };
    new_with_allowlist(args, valid_keys)
}

assert_abi!(fn bindings::rte_kvargs_free = rte_kvargs_free as unsafe extern "C" fn(_));
#[no_mangle]
extern "C" fn rte_kvargs_free(_: Option<SeaBox<KVargs>>) {} // dropped

assert_abi!(fn bindings::rte_kvargs_get = rte_kvargs_get as unsafe extern "C" fn(_, _) -> _);
#[no_mangle]
extern "C" fn rte_kvargs_get<'a>(
    kvlist: Option<&'a KVargs>,
    key: Option<&SeaStr>,
) -> Option<&'a SeaStr> {
    let needle = key?;
    kvlist?
        .iter()
        .find_map(|(haystack, val)| match haystack == needle {
            true => val,
            false => None,
        })
}

assert_abi!(fn bindings::rte_kvargs_get_with_value = rte_kvargs_get_with_value as unsafe extern "C" fn(_, _, _) -> _);
#[no_mangle]
extern "C" fn rte_kvargs_get_with_value<'a>(
    kvlist: Option<&'a KVargs>,
    key: Option<&SeaStr>,
    value: Option<&SeaStr>,
) -> Option<&'a SeaStr> {
    for (k, v) in kvlist?.iter() {
        if let Some(needle) = key {
            if k != needle {
                continue;
            }
        }
        if let (Some(needle), Some(v)) = (value, v) {
            if needle != v {
                continue;
            }
        }
        if v.is_some() {
            return v;
        }
    }
    None
}

assert_abi!(fn bindings::rte_kvargs_count = rte_kvargs_count as unsafe extern "C" fn(_, _) -> _);
#[no_mangle]
extern "C" fn rte_kvargs_count(kvlist: Option<&KVargs>, key_match: Option<&SeaStr>) -> c_uint {
    let mut count = Saturating(0);
    if let Some(kvlist) = kvlist {
        for (k, _) in kvlist.iter() {
            let inc = match key_match {
                Some(needle) => needle == k,
                None => true,
            };
            if inc {
                count += 1
            }
        }
    }
    count.0
}

assert_abi!(fn bindings::rte_kvargs_process = rte_kvargs_process as unsafe extern "C" fn(_, _, _, _) -> _);
/// # Safety
/// - `handler` must be safe to call.
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_process(
    kvlist: Option<&KVargs>,
    key_match: Option<&SeaStr>,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    process(kvlist, key_match, handler, opaque_arg, false)
}

assert_abi!(fn bindings::rte_kvargs_process_opt = rte_kvargs_process_opt as unsafe extern "C" fn(_, _, _, _) -> _);
/// # Safety
/// - `handler` must be safe to call.
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_process_opt(
    kvlist: Option<&KVargs>,
    key_match: Option<&SeaStr>,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    process(kvlist, key_match, handler, opaque_arg, true)
}

fn new_with_allowlist(
    s: &[u8],
    allowlist: Option<till_null::Iter<SeaStr>>,
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

/// # Safety
/// - Must be safe to call `handler`.
unsafe fn process(
    kvlist: Option<&KVargs>,
    key_match: Option<&SeaStr>,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
    allow_missing_value: bool,
) -> c_int {
    let (Some(kvlist), Some(cb)) = (kvlist, handler) else {
        return -1;
    };
    for (k, v) in kvlist.iter() {
        let call = match key_match {
            Some(needle) => needle == k,
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

assert_abi! {
    struct KVargs = bindings::rte_kvargs { buf = str_, count = count, pairs = pairs };
    struct Pair = bindings::rte_kvargs_pair { key = key, value = value };
}

#[repr(C)]
pub struct KVargs {
    /// Concatenated nul-terminated buffers for [`Self::pairs`].
    buf: Option<SeaString>,
    count: c_uint,
    /// The first [`Self::count`] are initialized,
    /// where the [`Pair::key`] and [`Pair::value`] are safe to dereference.
    pairs: [MaybeUninit<Pair>; 32],
}

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
