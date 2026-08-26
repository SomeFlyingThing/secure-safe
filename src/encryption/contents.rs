///the file format is HEADER then NOUNCE and CONTENTS
use std::{
    fs::File,
    io::{self, Read, Seek},
    marker::PhantomData,
    path::Path,
};

use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand_core::{OsRng, RngCore};

use crate::{
    encryption::password::{Derived, Password},
    file_format::header::Red,
};

const PASSWORD_LEN: usize = 32;
const NOUNCE_SIZE: usize = 12;
pub const SALT_PATH: &str = "salt.tmp";

pub struct Raw;
pub struct Encrypted;

pub struct Safe<'a, State> {
    password: &'a Password<Derived>,
    contents: Option<Vec<u8>>,
    nonce: Option<[u8; NOUNCE_SIZE]>,
    _data: PhantomData<State>,
}

impl<'a> Safe<'a, Red> {
    pub fn load(pass: &Password<Derived>, contents_path: &Path, file_ptr_location: usize, path: &[u8]) -> io::Result<Vec<u8>> {
        let mut file = File::open(contents_path)?;

        let mut contents = Vec::new();
        file.seek(io::SeekFrom::Start(file_ptr_location as u64))?;

        let mut nonce = [0u8; NOUNCE_SIZE];
        file.read_exact(&mut nonce)?;

        file.read_to_end(&mut contents)?;

        let decrypted = decrypt(pass, &nonce, &contents, path)?;

        Ok(decrypted)
    }
}

impl<'a> Safe<'a, Raw> {
    pub const fn new(password: &'a Password<Derived>, contents: Vec<u8>) -> Self {
        Self {
            password,
            contents: Some(contents),
            nonce: None,
            _data: PhantomData,
        }
    }
    pub fn configure(self, contents: Vec<u8>) -> Safe<'a, Encrypted> {
        Safe::<Encrypted> {
            password: self.password,
            contents: Some(contents),
            nonce: None,
            _data: PhantomData,
        }
    }
    pub fn encrypt(&self, path: &[u8]) -> io::Result<Safe<'_, Encrypted>> {
        let cipher = ChaCha20Poly1305::new_from_slice(self.password.extract()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "password to big"))?;

        let mut nounce = [0u8; NOUNCE_SIZE];
        OsRng.fill_bytes(&mut nounce);

        let nonce = Nonce::from(nounce);
        let plaintext = self.contents.as_deref().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid data"))?;

        let ciphertext = cipher
            .encrypt(&nonce, Payload { msg: plaintext, aad: path })
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid data"))?;

        Ok(Safe::<Encrypted> {
            contents: Some(ciphertext),
            nonce: Some(nounce),
            password: self.password,
            _data: PhantomData,
        })
    }
}

fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();

    let mut contents = Vec::with_capacity(size as usize);
    file.read_to_end(&mut contents)?;

    Ok(contents)
}
impl<'a> Safe<'a, Encrypted> {
    pub fn save(&self, vec: &mut Vec<u8>) -> io::Result<()> {
        vec.extend_from_slice(&self.nonce.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid data"))?);
        vec.extend_from_slice(&self.contents.clone().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid data"))?);

        Ok(())
    }
    pub fn extract(&self) -> Vec<u8> {
        self.contents.clone().unwrap()
    }
}

fn decrypt(pass: &Password<Derived>, nonce: &[u8; 12], contents: &[u8], path: &[u8]) -> io::Result<Vec<u8>> {
    //decryption

    let cipher = ChaCha20Poly1305::new_from_slice(pass.extract()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "decryption failed"))?;

    let nonce = Nonce::from(*nonce);
    cipher
        .decrypt(&nonce, Payload { msg: contents, aad: path })
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "decryption failed"))
}
