use hello_rust::len;

fn main() {
    dbg!(unsafe { len(c"hello".as_ptr()) });
}
