use std::{env::home_dir, fs, io, marker::PhantomData, ops::Deref};

use argon2::Argon2;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::file_format::header::atomic_write;

pub const SALT_PATH: &str = "salt.sf";
const PASS_SIZE: usize = 32;

pub struct Default;
pub struct Derived;

pub struct Password<State> {
    pass: Zeroizing<[u8; PASS_SIZE]>,
    salt: Option<[u8; 16]>,
    _data: PhantomData<State>,
}

fn pad_password(password: &str) -> io::Result<[u8; PASS_SIZE]> {
    if password.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "password cannot be empty"));
    }
    if password.len() > PASS_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "password is longer than 32 bytes"));
    }

    let mut bytes = [0u8; PASS_SIZE];
    bytes[..password.len()].copy_from_slice(password.as_bytes());
    Ok(bytes)
}

impl Password<Default> {
    pub fn new() -> io::Result<Self> {
        Password::ask_password()
    }

    pub fn recover() -> io::Result<Self> {
        Password::ask_used_pass()
    }

    fn ask_used_pass() -> io::Result<Self> {
        let saltloc = home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir not found"))?.join(SALT_PATH);
        let salt: [u8; 16] = fs::read(saltloc)?.try_into().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid salt"))?;

        Self::ask_password_with_salt(Some(salt), false)
    }
    fn ask_password() -> io::Result<Self> {
        Self::ask_password_with_salt(None, true)
    }

    fn ask_password_with_salt(salt: Option<[u8; 16]>, new_password: bool) -> io::Result<Self> {
        loop {
            if new_password {
                println!("what password do you wish to use, remember it");
            }
            println!("password:");

            let password = Zeroizing::new(rpassword::read_password()?);
            let bytes = match pad_password(&password) {
                Ok(bytes) => bytes,
                Err(_) => {
                    println!("choose a smaller password");
                    continue;
                },
            };

            return Ok(Password::<Default> {
                pass: Zeroizing::new(bytes),
                salt,
                _data: PhantomData,
            });
        }
    }

    #[cfg(any(test, kani))]
    pub(crate) fn test_create_pass(bytes: [u8; 32]) -> Password<Default> {
        return Password::<Default> {
            pass: Zeroizing::new(bytes),
            salt: None,
            _data: PhantomData,
        };
    }
    pub fn derive(self) -> io::Result<Password<Derived>> {
        #[cfg(not(test))]
        let should_save_salt = self.salt.is_none();
        let salt = self.salt.unwrap_or_else(|| {
            let mut salt = [0u8; 16];
            OsRng.fill_bytes(&mut salt);
            salt
        });

        let mut key = [0u8; PASS_SIZE];
        Argon2::default()
            .hash_password_into(self.pass.iter().as_ref(), &salt, &mut key)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "couldnt derive password"))?;

        let ret = Password::<Derived> {
            pass: Zeroizing::new(key),
            salt: Some(salt),
            _data: PhantomData,
        };

        #[cfg(not(test))]
        if should_save_salt {
            ret.save_salt()?;
        }

        Ok(ret)
    }
}

pub trait SaveSalt {
    fn save_salt(&self) -> io::Result<()>;
}

impl SaveSalt for Password<Derived> {
    fn save_salt(&self) -> io::Result<()> {
        let saltloc = home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir not found"))?.join(SALT_PATH);
        let salt = self.salt.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing salt"))?;
        atomic_write(&salt, &saltloc)?;

        Ok(())
    }
}

impl Password<Derived> {
    pub fn extract(&self) -> &[u8] {
        self.pass.deref()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, io::Write};

use toml::ser::Error;

    use super::*;
    use crate::{add::add, restore::restore};

    #[test]
    fn pads_short_password() {
        let password = pad_password("secret").unwrap();

        assert_eq!(&password[..6], b"secret");
        assert_eq!(&password[6..], &[0u8; PASS_SIZE - 6]);
    }

    #[test]
    fn rejects_empty_password() {
        assert_eq!(pad_password("").unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn derived_password_reuses_existing_salt() {
        let salt = [3u8; 16];
        let password = Password::<Default> {
            pass: Zeroizing::new([7u8; PASS_SIZE]),
            salt: Some(salt),
            _data: PhantomData,
        };

        let password = password.derive().unwrap();

        assert_eq!(password.salt, Some(salt));
    }

    #[test]
    fn wrong_password() {
        const CONTENTS: &[u8] = b"bandjfkjfkadjf";
        const PATH: &[u8] = b"potato";

        let salt = [4; 16];
        let derive_password = |password| {
            Password::<Default> {
                pass: Zeroizing::new(pad_password(password).unwrap()),
                salt: Some(salt),
                _data: PhantomData,
            }
            .derive()
            .unwrap()
        };

        let password = derive_password("BANANAN!");
        let safe = crate::encryption::contents::Safe::new(&password, CONTENTS.to_vec());
        let encrypted = safe.encrypt(PATH).unwrap();

        let mut encrypted_contents = Vec::new();
        encrypted.save(&mut encrypted_contents).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let encrypted_file = temp_dir.path().join("potato");
        std::fs::write(&encrypted_file, encrypted_contents).unwrap();

        let wrong_password = derive_password("boooo");

        let result = crate::encryption::contents::Safe::load(&wrong_password, &encrypted_file, 0, PATH);

        assert!(result.is_err());
    }
}
