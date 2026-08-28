use std::{env::home_dir, io};

use crate::{
    add::add,
    encryption::password::{Derived, Password, SALT_PATH},
    generate_path,
    restore::restore_in_mem,
};

const PASS_CHECK_NAME: &str = "pass-check";
const PASS_CHECK_CONTENTS: &[u8] = b"secure-safe-verified";

fn has_login() -> bool {
    home_dir().is_some_and(|home| home.join(SALT_PATH).is_file())
}

pub fn authenticate() -> io::Result<Password<Derived>> {
    if !has_login() {
        let password = Password::new()?.derive()?;
        create_pass_verifier(&password)?;
        return Ok(password);
    }

    loop {
        let password = Password::recover()?.derive()?;
        if password_is_correct(&password)? {
            return Ok(password);
        }
        eprintln!("incorrect password");
    }
}

fn password_is_correct(password: &Password<Derived>) -> io::Result<bool> {
    match restore_in_mem(PASS_CHECK_NAME, password) {
        Ok(contents) => Ok(contents == PASS_CHECK_CONTENTS),
        Err(error) if error.kind() == io::ErrorKind::InvalidData => Ok(false),
        Err(error) => Err(error),
    }
}

fn create_pass_verifier(password: &Password<Derived>) -> io::Result<()> {
    let path = generate_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir not found"))?.join(PASS_CHECK_NAME);

    add(password, &path, PASS_CHECK_CONTENTS)
}
