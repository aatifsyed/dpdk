use core::{
    ffi::{c_int, c_uint, c_void},
    option::Option,
};
use std::{mem::MaybeUninit, ptr::NonNull};

use seestr::{Buf, NulTerminated};

use crate::util::Base;

pub const RTE_KVARGS_MAX: u32 = 32;
pub const RTE_KVARGS_PAIRS_DELIM: &[u8; 2] = b",\0";
pub const RTE_KVARGS_KV_DELIM: &[u8; 2] = b"=\0";

/// Callback prototype used by rte_kvargs_process().
///
///  @param key
///    The key to consider, it will not be NULL.
///  @param value
///    The value corresponding to the key, it may be NULL (e.g. only with key)
///  @param opaque
///    An opaque pointer coming from the caller.
///  @return
///    - >=0 handle key success.
///    - <0 on error.
#[allow(non_camel_case_types)]
pub type arg_handler_t = unsafe extern "C" fn(
    key: &NulTerminated,
    value: Option<&NulTerminated>,
    opaque: *mut c_void,
) -> c_int;
/// A key/value association
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct rte_kvargs_pair {
    /// < the name (key) of the association
    pub key: NonNull<NulTerminated>,
    /// < the value associated to that key
    pub value: Option<NonNull<NulTerminated>>,
}
/// Store a list of key/value associations
#[repr(C)]
#[derive(Debug)]
pub struct rte_kvargs {
    /// < copy of the argument string
    pub str_: Option<Buf>,
    /// < number of entries in the list
    pub count: c_uint,
    /// < list of key/values
    pub pairs: [MaybeUninit<rte_kvargs_pair>; 32usize],
}

/// Allocate a rte_kvargs and store key/value associations from a string
///
///  The function allocates and fills a rte_kvargs structure from a given
///  string whose format is key1=value1,key2=value2,...
///
///  The structure can be freed with rte_kvargs_free().
///
///  @param args
///    The input string containing the key/value associations
///  @param valid_keys
///    A list of valid keys (table of const char *, the last must be NULL).
///    This argument is ignored if NULL
///
///  @return
///    - A pointer to an allocated rte_kvargs structure on success
///    - NULL on error
#[no_mangle]
pub extern "C" fn rte_kvargs_parse(
    parse_me: &NulTerminated,
    valid_keys: Option<Base<&NulTerminated>>,
) -> Option<Box<rte_kvargs>> {
    let mut fail = false;
    let mut args = trybox::or_drop(rte_kvargs {
        str_: None,
        count: 0,
        pairs: [const { MaybeUninit::zeroed() }; 32],
    })
    .ok()?;
    if parse_me.is_empty() {
        return Some(args);
    }
    let mut save = args.pairs.iter_mut().inspect(|_| args.count += 1);
    let buf = Buf::try_with(parse_me.len_with_nul(), |mut buf| {
        let parsed = parsing::for_each(parse_me.bytes(), |k, v| {
            if fail {
                return;
            }
            use std::io::Write as _;
            let Some(dst) = save.next() else {
                fail = true;
                return; // too many args
            };
            let mut src = rte_kvargs_pair {
                key: NonNull::from(&buf[0]).cast(),
                value: None,
            };
            let Ok(()) = buf.write_all(k) else {
                fail = true;
                return;
            };
            let Ok(()) = buf.write_all(b"\0") else {
                fail = true;
                return;
            };
            if let Some(v) = v {
                src.value = Some(NonNull::from(&buf[0]).cast());
                let Ok(()) = buf.write_all(v) else {
                    fail = true;
                    return;
                };
                let Ok(()) = buf.write_all(b"\0") else {
                    fail = true;
                    return;
                };
            }
            dst.write(src);
        });
        if parsed.is_err() {
            fail = true
        }
    })
    .ok()?;
    args.str_ = Some(buf);
    match fail {
        true => None,
        false => {
            if let Some(filter) = valid_keys {
                for (key, _) in unsafe { iter(&args) } {
                    if !filter.clone().into_iter().any(|it| it == key) {
                        return None;
                    }
                }
            }
            Some(args)
        }
    }
}

/// Allocate a rte_kvargs and store key/value associations from a string.
///  This version will consider any byte from valid_ends as a possible
///  terminating character, and will not parse beyond any of their occurrence.
///
///  The function allocates and fills an rte_kvargs structure from a given
///  string whose format is key1=value1,key2=value2,...
///
///  The structure can be freed with rte_kvargs_free().
///
///  @param args
///    The input string containing the key/value associations
///
///  @param valid_keys
///    A list of valid keys (table of const char *, the last must be NULL).
///    This argument is ignored if NULL
///
///  @param valid_ends
///    Acceptable terminating characters.
///    If NULL, the behavior is the same as ``rte_kvargs_parse``.
///
///  @return
///    - A pointer to an allocated rte_kvargs structure on success
///    - NULL on error
#[no_mangle]
pub extern "C" fn rte_kvargs_parse_delim(
    args: &NulTerminated,
    valid_keys: Option<Base<&NulTerminated>>,
    valid_ends: Option<&NulTerminated>,
) -> Option<Box<rte_kvargs>> {
    let _todo = (args, valid_keys, valid_ends);
    None
}

/// Free a rte_kvargs structure
///
///  Free a rte_kvargs structure previously allocated with
///  rte_kvargs_parse().
///
///  @param kvlist
///    The rte_kvargs structure. No error if NULL.
#[no_mangle]
pub extern "C" fn rte_kvargs_free(_: Option<Box<rte_kvargs>>) {}

/// Get the value associated with a given key.
///
///  If multiple keys match, the value of the first one is returned.
///
///  The memory returned is allocated as part of the rte_kvargs structure,
///  it must never be modified.
///
///  @param kvlist
///    A list of rte_kvargs pair of 'key=value'.
///  @param key
///    The matching key.
///
///  @return
///    NULL if no key matches the input,
///    a value associated with a matching key otherwise.
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_get<'a>(
    kvlist: &'a rte_kvargs,
    key: &NulTerminated,
) -> Option<&'a NulTerminated> {
    iter(kvlist).find_map(|(k, v)| match k == key {
        true => v,
        false => None,
    })
}

/// # Safety
/// - must have been allocated by [`rte_kvargs_parse`], and not modified.
unsafe fn iter(
    kvlist: &rte_kvargs,
) -> impl Iterator<Item = (&NulTerminated, Option<&NulTerminated>)> {
    kvlist.pairs.iter().take(kvlist.count as usize).map(|it| {
        let rte_kvargs_pair { key, value } = it.assume_init_ref();
        (key.as_ref(), value.map(|it| it.as_ref()))
    })
}

/// Get the value associated with a given key and value.
///
///  Find the first entry in the kvlist whose key and value match the
///  ones passed as argument.
///
///  The memory returned is allocated as part of the rte_kvargs structure,
///  it must never be modified.
///
///  @param kvlist
///    A list of rte_kvargs pair of 'key=value'.
///  @param key
///    The matching key. If NULL, any key will match.
///  @param value
///    The matching value. If NULL, any value will match.
///
///  @return
///    NULL if no key matches the input,
///    a value associated with a matching key otherwise.
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_get_with_value<'a>(
    kvlist: &'a rte_kvargs,
    key: Option<&NulTerminated>,
    value: Option<&NulTerminated>,
) -> Option<&'a NulTerminated> {
    let _todo = (kvlist, key, value);
    None
}

/// Call a handler function for each key=value matching the key
///
///  For each key=value association that matches the given key, calls the
///  handler function with the for a given arg_name passing the value on the
///  dictionary for that key and a given extra argument.
///
///  @note Compared to @see rte_kvargs_process_opt, this API will return -1
///  when handle only-key case (that is the matched key's value is NULL).
///
///  @param kvlist
///    The rte_kvargs structure.
///  @param key_match
///    The key on which the handler should be called, or NULL to process handler
///    on all associations
///  @param handler
///    The function to call for each matching key
///  @param opaque_arg
///    A pointer passed unchanged to the handler
///
///  @return
///    - 0 on success
///    - Negative on error
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_process(
    kvlist: &rte_kvargs,
    key_match: Option<&NulTerminated>,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    for (k, v) in iter(kvlist) {
        let call = match key_match {
            Some(filter) => filter == k,
            None => true,
        };
        if call {
            if v.is_none() {
                return -1;
            }
            if handler(k, v, opaque_arg) < 0 {
                return -1;
            }
        }
    }
    0
}

/// Call a handler function for each key=value or only-key matching the key
///
///  For each key=value or only-key association that matches the given key, calls
///  the handler function with the for a given arg_name passing the value on the
///  dictionary for that key and a given extra argument.
///
///  @param kvlist
///    The rte_kvargs structure.
///  @param key_match
///    The key on which the handler should be called, or NULL to process handler
///    on all associations
///  @param handler
///    The function to call for each matching key
///  @param opaque_arg
///    A pointer passed unchanged to the handler
///
///  @return
///    - 0 on success
///    - Negative on error
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_process_opt(
    kvlist: &rte_kvargs,
    key_match: Option<&NulTerminated>,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    for (k, v) in iter(kvlist) {
        let call = match key_match {
            Some(filter) => filter == k,
            None => true,
        };
        if call {
            if handler(k, v, opaque_arg) < 0 {
                return -1;
            }
        }
    }
    0
}

/// Count the number of associations matching the given key
///
///  @param kvlist
///    The rte_kvargs structure
///  @param key_match
///    The key that should match, or NULL to count all associations
///
///  @return
///    The number of entries
#[no_mangle]
pub unsafe extern "C" fn rte_kvargs_count(
    kvlist: &rte_kvargs,
    key_match: Option<&NulTerminated>,
) -> c_uint {
    let mut ct = 0;
    for (key, _) in iter(kvlist) {
        let should_ct = match key_match {
            Some(filter) => filter == key,
            None => true,
        };
        if should_ct {
            ct += 1
        }
    }
    ct
}

mod util {
    use core::{mem, ptr::NonNull};

    /// Walk an array of `T` until an entry filled with zeroes.
    #[repr(transparent)]
    pub struct Base<T> {
        ptr: NonNull<T>,
    }

    impl<T: Copy> Clone for Base<T> {
        fn clone(&self) -> Self {
            Self { ptr: self.ptr }
        }
    }

    impl<T> IntoIterator for Base<T> {
        type Item = T;
        type IntoIter = Iter<T>;
        fn into_iter(self) -> Self::IntoIter {
            Iter {
                base: self,
                offset: 0,
            }
        }
    }

    pub struct Iter<T> {
        base: Base<T>,
        offset: usize,
    }

    impl<T> Iterator for Iter<T> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            unsafe {
                let next = self.base.ptr.add(self.offset);
                let test = next.cast::<u8>();
                let mut zero = true;
                for byte in 0..mem::size_of::<T>() {
                    if test.add(byte).read() != 0 {
                        zero = false;
                        break;
                    }
                }
                match zero {
                    true => None,
                    false => {
                        self.offset += 1;
                        Some(next.read())
                    }
                }
            }
        }
    }
}

mod parsing {
    use core::cmp;
    use nom::{
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

    pub fn for_each<'a>(
        input: &'a [u8],
        mut f: impl FnMut(&'a [u8], Option<&'a [u8]>),
    ) -> Result<(), ()> {
        match callback_separated(kv, tag(&b","[..]), |(k, v)| f(k, v))(input) {
            Ok((b"", ())) => Ok(()),
            _ => Err(()),
        }
    }

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
}
