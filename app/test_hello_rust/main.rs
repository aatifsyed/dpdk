use hello_rust::hello_rust_len;

fn main() {
    dbg!(unsafe { hello_rust_len(Some(c"hello".into())) });
}
