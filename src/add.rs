use std::{
    io::{self, ErrorKind},
    path::Path,
};


use crate::{
    Password,
    encryption::{
        contents::Safe,
        password::Derived,
    },
    file_format::header::{Header, Save, atomic_write},
    generate_path,
};

pub fn add(password: &Password<Derived>, path: &Path, file_contents: &Vec<u8>) -> io::Result<()> {
    let safe_path = generate_path().ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home dir not found"))?;
    add_at(password, path, file_contents, &safe_path)
}

pub(crate) fn add_at(password: &Password<Derived>, path: &Path, file_contents: &Vec<u8>, safe_path: &Path) -> io::Result<()> {
    let header = Header::default();

    let mut header = header.configure(path);

    let safe = Safe::new(password, file_contents.clone());
    let safe = safe.encrypt()?;

    let mut encrypted_contents = Vec::new();
    safe.save(&mut encrypted_contents)?;
    header.hash(&encrypted_contents);

    let file_name = header.file_name();

    let mut contents = header.save()?;
    contents.extend_from_slice(&encrypted_contents);

    std::fs::create_dir_all(safe_path)?;
    let save_path = safe_path.join(file_name);

    atomic_write(&contents, &save_path)?;
    if path != save_path.as_path() {
        std::fs::remove_file(path)?;
    }

    match safe_path.file_name() {
        Some(name) => println!("file {} was succesfully saved", name.display()),
        None => println!("file was succesfully saved"),
    }

    Ok(())
}
