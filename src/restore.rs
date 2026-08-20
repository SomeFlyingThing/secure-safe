use std::io;

use crate::{
    file_format::header::{Header, Load, Red},
    generate_path,
};

fn restore(name: &str) -> io::Result<()> {
    let file_path = generate_path().unwrap().join(name);

    let header  = Header::<Red>::load(&file_path)?;

//TODO read the rest
    Ok(())
}
