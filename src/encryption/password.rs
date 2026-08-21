use std::{io, marker::PhantomData, ops::Deref};

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

impl Password<Default> {
    pub fn new() -> io::Result<Self> {
        Password::ask_password()
    }

    pub fn recover() -> io::Result<Self> {
        Password::ask_used_pass()
    }

    fn ask_used_pass() -> io::Result<Self> {
        loop {
            println!("password:");

            let mut password = Zeroizing::new(String::new());
            println!("password:");

            io::stdin().read_line(&mut password)?;
            if password.len() > PASS_SIZE {
                println!("choose a smaller password");
                continue;
            }

            let password = password.trim_matches(&['\r', '\n'][..]);
            let bytes: [u8; PASS_SIZE] = password
                .as_bytes()
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "password must be 32 bytes long"))?;

            return Ok(Password::<Default> {
                pass: Zeroizing::new(bytes),
                salt: None,
                _data: PhantomData,
            });
        }
    }
    fn ask_password() -> io::Result<Self> {
        loop {
            println!("what password do you wish to use, remember it");
            let mut password = Zeroizing::new(String::new());
            println!("password:");

            io::stdin().read_line(&mut password)?;
            if password.len() > PASS_SIZE {
                println!("choose a smaller password");
                continue;
            }

            let password = password.trim_matches(&['\r', '\n'][..]);
            let bytes: [u8; PASS_SIZE] = password
                .as_bytes()
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "password must be 32 bytes long"))?;

            return Ok(Password::<Default> {
                pass: Zeroizing::new(bytes),
                salt: None,
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
        let mut salt = [0u8; 16];

        OsRng.fill_bytes(&mut salt);

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
        ret.save_salt()?;

        Ok(ret)
    }
}

pub trait SaveSalt {
    fn save_salt(&self) -> io::Result<()>;
}

impl SaveSalt for Password<Derived> {
    fn save_salt(&self) -> io::Result<()> {
        let saltloc = std::env::home_dir().unwrap().join(SALT_PATH);
        atomic_write(&self.salt.unwrap(), &saltloc)?;

        Ok(())
    }
}
impl Password<Derived> {
    pub fn extract(&self) -> &[u8] {
        self.pass.deref()
    }
}
