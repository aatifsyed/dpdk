use ::core::ffi::{c_char, c_int, c_uint, c_void};

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
pub type arg_handler_t = ::core::option::Option<
    unsafe extern "C" fn(key: *const c_char, value: *const c_char, opaque: *mut c_void) -> c_int,
>;
/// A key/value association
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct rte_kvargs_pair {
    /// < the name (key) of the association
    pub key: *mut c_char,
    /// < the value associated to that key
    pub value: *mut c_char,
}
/// Store a list of key/value associations
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct rte_kvargs {
    /// < copy of the argument string
    pub str_: *mut c_char,
    /// < number of entries in the list
    pub count: c_uint,
    /// < list of key/values
    pub pairs: [rte_kvargs_pair; 32usize],
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
    args: *const c_char,
    valid_keys: *const *const c_char,
) -> *mut rte_kvargs {
    todo!()
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
    args: *const c_char,
    valid_keys: *const *const c_char,
    valid_ends: *const c_char,
) -> *mut rte_kvargs {
    todo!()
}

/// Free a rte_kvargs structure
///
///  Free a rte_kvargs structure previously allocated with
///  rte_kvargs_parse().
///
///  @param kvlist
///    The rte_kvargs structure. No error if NULL.
#[no_mangle]
pub extern "C" fn rte_kvargs_free(kvlist: *mut rte_kvargs) {
    todo!()
}

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
pub extern "C" fn rte_kvargs_get(kvlist: *const rte_kvargs, key: *const c_char) -> *const c_char {
    todo!()
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
pub extern "C" fn rte_kvargs_get_with_value(
    kvlist: *const rte_kvargs,
    key: *const c_char,
    value: *const c_char,
) -> *const c_char {
    todo!()
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
pub extern "C" fn rte_kvargs_process(
    kvlist: *const rte_kvargs,
    key_match: *const c_char,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    todo!()
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
pub extern "C" fn rte_kvargs_process_opt(
    kvlist: *const rte_kvargs,
    key_match: *const c_char,
    handler: arg_handler_t,
    opaque_arg: *mut c_void,
) -> c_int {
    todo!()
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
pub extern "C" fn rte_kvargs_count(kvlist: *const rte_kvargs, key_match: *const c_char) -> c_uint {
    todo!()
}
