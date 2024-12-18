use std::{
    cmp,
    ffi::CStr,
    io,
    mem::{self, MaybeUninit},
    ops::RangeFrom,
    os::raw::{c_char, c_int, c_uint, c_void},
    pin::Pin,
    ptr::{self, NonNull},
};

use bindings::*;
use rust3p::{
    seasick::{Allocator, Libc, SeaBox, SeaBoxIn, SeaStr, SeaString},
    seesaw::no_mangle as no_mangle_all,
};

struct Impl;

#[no_mangle_all]
impl seesaw_dpdk::KVargs for Impl {
    unsafe extern "C" fn rte_kvargs_parse(
        args: *const c_char,
        valid_keys: *const *const c_char,
    ) -> *mut rte_kvargs {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_parse_delim(
        args: *const c_char,
        valid_keys: *const *const c_char,
        valid_ends: *const c_char,
    ) -> *mut rte_kvargs {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_free(kvlist: *mut rte_kvargs) {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_get(
        kvlist: *const rte_kvargs,
        key: *const c_char,
    ) -> *const c_char {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_get_with_value(
        kvlist: *const rte_kvargs,
        key: *const c_char,
        value: *const c_char,
    ) -> *const c_char {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_process(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
        handler: arg_handler_t,
        opaque_arg: *mut c_void,
    ) -> c_int {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_process_opt(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
        handler: arg_handler_t,
        opaque_arg: *mut c_void,
    ) -> c_int {
        todo!()
    }

    unsafe extern "C" fn rte_kvargs_count(
        kvlist: *const rte_kvargs,
        key_match: *const c_char,
    ) -> c_uint {
        todo!()
    }
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
    fn new(s: &CStr) -> Option<Self> {
        let mut pairs = [const { MaybeUninit::zeroed() }; 32];
        let mut count = 0;
        if s.is_empty() {
            return Some(Self {
                buf: None,
                count,
                pairs,
            });
        }
        let mut buf = SeaString::try_with(s.to_bytes_with_nul().len(), |it| it.fill(1)).ok()?;
        let mut iter_pairs = pairs.iter_mut().inspect(|_| count += 1);
        {
            let mut s = s.to_bytes();
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

fn callback_separated<I, O, E, SepO, ParseP, SepP>(
    mut parser: ParseP,
    mut separator: SepP,
    mut fold: impl FnMut(O),
) -> impl FnMut(I) -> IResult<I, (), E>
where
    ParseP: Parser<I, O, E>,
    SepP: Parser<I, SepO, E>,
    I: Clone,
{
    move |input: I| -> IResult<I, (), E> {
        let (mut input, first) = parser.parse(input)?;
        fold(first);
        while let Ok((after_sep, _sep)) = separator.parse(input.clone()) {
            let (next_input, item) = parser.parse(after_sep)?;
            fold(item);
            input = next_input;
        }
        Ok((input, ()))
    }
}
