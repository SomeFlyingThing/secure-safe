use std::{fs, fs::OpenOptions, io, io::Write, path::Path};

use crate::{
    encryption::{
        contents::Safe,
        password::{Derived, Password},
    },
    file_format::header::{Header, Load, Red},
    generate_path,
};

pub fn restore(name: &str, pass: &Password<Derived>) -> io::Result<()> {
    let safe_path = generate_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir not found"))?;
    restore_at(name, pass, &safe_path)
}

fn restore_at(name: &str, pass: &Password<Derived>, safe_path: &Path) -> io::Result<()> {
    let file_path = safe_path.join(name);

    let (header, file_ptr_location) = Header::<Red>::load(&file_path)?;

    let contents = Safe::load(pass, &file_path, file_ptr_location)?;

    normal_attomic_write(&header.path(), &contents)?;

    println!("file was sucessfully restored");
    Ok(())
}
fn normal_attomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&tmp)?;

    file.write_all(contents)?;
    file.sync_all()?;

    fs::rename(tmp, path)?;

    file.sync_all()?;

    Ok(())
}


#[cfg(test)]
mod simple_test{
    use tempfile;

    use super::*;
    use crate::{add::add_at, encryption::password::Default};

    #[test]
    fn round_trip() {
        let tmp = tempfile::tempdir().unwrap();

        // Make Secure Safe use this as its managed directory.
        let safe_dir = tmp.path().join("safe");
        std::fs::create_dir(&safe_dir).unwrap();

        let original = tmp.path().join("hello.bin");
        let file_contents = vec![0u8; 9000];

        std::fs::write(&original, &file_contents).unwrap();

        let password =
            Password::<Default>::test_create_pass([20u8; 32])
                .derive()
                .unwrap();

        // Configure your app so its storage directory == `safe_dir`.
        //
        // Then exercise the REAL API:
        add_at(&password, &original, &file_contents, &safe_dir).unwrap();

        // whatever add() normally does to the source
        // ...

        restore_at("hello.bin", &password, &safe_dir).unwrap();

        let restored = std::fs::read(&original).unwrap();

        assert_eq!(restored, file_contents);
    }
}
