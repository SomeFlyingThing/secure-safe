use std::{
    env::home_dir,
    fs::File,
    io::{self, ErrorKind, Read},
    path::Path,
};

use toml::ser::Error;

use crate::{
    Password,
    encryption::{
        contents::{self, Safe},
        password::Derived,
    },
    file_format::header::{Header, Save, atomic_write},
    generate_path,
};

pub fn add(password: &Password<Derived>, path: &Path, file_contents: &Vec<u8>) -> io::Result<()> {
    let header = Header::default();

    let mut header = header.configure(path);

    let safe = Safe::new(password, file_contents.clone());
    let safe = safe.encrypt()?;

    header.hash(&safe.extract());

    let file_name = header.file_name();

    let mut contents = header.save()?;
    safe.save(&mut contents)?;

    let path = generate_path().ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home dir not found"))?;
    let save_path = path.join(file_name);

    match path.file_name() {
        Some(name) => println!("file {} was succesfully saved", name.display()),
        None => println!("file was succesfully saved"),
    }

    atomic_write(&contents, &save_path)?;

    Ok(())
}
