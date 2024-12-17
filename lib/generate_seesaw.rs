use std::io::{self, Read as _, Write as _};

use rust3p::seesaw::{self, Destination, Trait};

fn main() -> io::Result<()> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    let mut o = io::stdout();
    writeln!(o, "use bindings::*;")?;
    seesaw::seesaw(
        Trait::new("KVargs").allow("rte_kvargs.*"),
        s,
        Destination::Writer(Box::new(o)),
    )
}
