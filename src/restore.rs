use std::{fs, fs::File, io, io::Write, path::Path};

use crate::{
    encryption::{
        contents,
        contents::Safe,
        password::{Derived, Password},
    },
    file_format::header::{Header, Load, Red},
    generate_path,
};

pub fn restore(name: &str, pass: &Password<Derived>) -> io::Result<()> {
    let file_path = generate_path().unwrap().join(name);

    let (_, file_ptr_location) = Header::<Red>::load(&file_path)?;

    let contents = Safe::load(pass, &file_path, file_ptr_location)?;

    normal_attomic_write(&file_path, &contents)?;

    println!("file was sucessfully restored");
    Ok(())
}
fn normal_attomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = File::open(&tmp)?;

    file.write_all(contents)?;
    file.sync_all()?;

    fs::rename(tmp, path)?;

    file.sync_all()?;

    Ok(())
}
