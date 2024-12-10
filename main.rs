use std::ptr;

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    unused,
    clippy::upper_case_acronyms
)]
mod bindings;
fn main() {
    unsafe { bindings::osdep_iface_index_get(c"".as_ptr()) };
    dbg!(unsafe { bindings::rte_vdev_count() });
}
