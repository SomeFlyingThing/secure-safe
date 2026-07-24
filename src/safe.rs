use std::{
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use argon2::{Argon2, password_hash::SaltString};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::settings::Settings;

fn read_file(path: &str) -> Vec<u8> {
    let mut file = File::open(path).expect("couldnt read the file check the path");

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    bytes
}

const NOUNCE_SIZE: usize = 24;
const MAX_PATH_SIZE: usize = 16 * 1024;
pub struct Raw {
    pub salt: [u8; 16],
    pub nounce: [u8; NOUNCE_SIZE],
    pub ciphertext: Vec<u8>,
    pub name: String,
    pub path: Vec<u8>,
}

pub struct Safe<State> {
    pub state: State,
}

impl Safe<Raw> {
    pub fn new(path: &str, password: &[u8]) -> Self {
        //read and compress
        let contents = read_file(path);
        let contents = zstd::encode_all(Cursor::new(&contents), 5).unwrap();

        let name = PathBuf::from(path);
        let name = name.file_name().expect("not valid file name");

        let salt = SaltString::generate(&mut OsRng);

        let mut key = Zeroizing::new([0u8; 32]);
        let mut salt_bytes = [0u8; 16];
        salt.as_salt().decode_b64(&mut salt_bytes).unwrap();
        Argon2::default().hash_password_into(password, &salt_bytes, &mut *key).expect(obfstr::obfstr!("error deriving password"));

        let cipher = XChaCha20Poly1305::new_from_slice(&*key).unwrap();

        let mut nounce = [0u8; NOUNCE_SIZE];
        OsRng.fill_bytes(&mut nounce);

        let ciphertext = cipher.encrypt(&nounce.into(), Payload { msg: &contents, aad: path.as_bytes() }).map_err(|_| "encryption failed").unwrap();

        Self {
            state: Raw {
                salt: salt_bytes,
                name: name.to_str().unwrap().to_owned(),
                ciphertext,
                path: path.as_bytes().to_vec(),
                nounce,
            },
        }
    }
}

impl Safe<Raw> {
    pub fn store(self, settins: &Settings) {
        let path = settins.enc_dir.join(self.state.name.clone());

        let mut file = OpenOptions::new().create_new(true).write(true).open(path).unwrap();
        file.write_all(&self.state.salt).unwrap();
        file.write_all(&self.state.nounce).unwrap();
        file.write_all(&(self.state.path.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&self.state.path).unwrap();
        file.write_all(&self.state.ciphertext).unwrap();
    }
}

pub fn remove(name: &str, settings: &Settings) {
    if Path::new(name).file_name().and_then(|name| name.to_str()) != Some(name) {
        eprintln!("invalid stored file name");
        return;
    }

    let path = settings.enc_dir.join(name);
    fs::remove_file(path).unwrap();
}

pub fn check(password: &[u8], settings: &Settings) {
    let dir = &settings.enc_dir;

    for item in fs::read_dir(dir).unwrap() {
        let entry = item.unwrap();
        let path = entry.path();
        if path.is_file() {
            let mut file = File::open(&path).unwrap();

            let mut salt = [0u8; 16];
            let mut nounce = [0u8; NOUNCE_SIZE];
            let mut path_size = [0u8; 4];
            if file.read_exact(&mut salt).is_err() || file.read_exact(&mut nounce).is_err() || file.read_exact(&mut path_size).is_err() {
                eprintln!("invalid stored file: {:?}", path);
                continue;
            }

            let path_size = u32::from_le_bytes(path_size) as usize;
            if path_size > MAX_PATH_SIZE {
                eprintln!("invalid stored file: {:?}", path);
                continue;
            }

            let mut path_bytes = vec![0; path_size];
            let mut ciphertext = Vec::new();
            if file.read_exact(&mut path_bytes).is_err() || file.read_to_end(&mut ciphertext).is_err() {
                eprintln!("invalid stored file: {:?}", path);
                continue;
            }

            let mut key = Zeroizing::new([0u8; 32]);
            Argon2::default().hash_password_into(password, &salt, &mut *key).unwrap();
            let cipher = XChaCha20Poly1305::new_from_slice(&*key).unwrap();
            if cipher.decrypt(&XNonce::from(nounce), Payload { msg: &ciphertext, aad: &path_bytes }).is_err() {
                eprintln!("failed integrity check: {:?}", path);
                continue;
            }

            println!("{:?}", path.file_name().unwrap());
        }
    }
}
#[derive(Debug)]
pub enum EncError {
    Decryption(String),
    UnZip(String),
    Read,
}

pub fn move_out(password: &[u8], settins: &Settings, name: &str) -> Result<(), EncError> {
    if Path::new(name).file_name().and_then(|name| name.to_str()) != Some(name) {
        return Err(EncError::Read);
    }

    let mut file = File::open(settins.enc_dir.join(name)).map_err(|_| EncError::Read)?;

    let mut salt = [0u8; 16];
    let mut nounce = [0u8; NOUNCE_SIZE];
    let mut path_size = [0u8; 4];
    let mut cypehr = Vec::new();
    file.read_exact(&mut salt).map_err(|_| EncError::Read)?;
    file.read_exact(&mut nounce).map_err(|_| EncError::Read)?;
    file.read_exact(&mut path_size).map_err(|_| EncError::Read)?;

    let path_size = u32::from_le_bytes(path_size) as usize;
    if path_size > MAX_PATH_SIZE {
        return Err(EncError::Read);
    }

    let mut path = vec![0; path_size];
    file.read_exact(&mut path).map_err(|_| EncError::Read)?;

    file.read_to_end(&mut cypehr).map_err(|_| EncError::Read)?;

    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::default().hash_password_into(password, &salt, &mut *key).unwrap();

    let path = String::from_utf8(path).map_err(|_| EncError::Read)?;

    // will be used for the file name
    let path_to_name = PathBuf::from(path.clone());

    let cipher = XChaCha20Poly1305::new_from_slice(&*key).unwrap();
    let decrypted = cipher
        .decrypt(&XNonce::from(nounce), Payload { msg: &cypehr, aad: path.as_bytes() })
        .map_err(|_| EncError::Decryption(path_to_name.file_name().unwrap().to_str().unwrap().to_owned()))?;

    let decomp = zstd::decode_all(Cursor::new(decrypted)).map_err(|error| EncError::UnZip(error.to_string()))?;
    let temporary = PathBuf::from(&path).with_extension("secure_safe.tmp");
    let mut file = OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|_| EncError::Read)?;
    file.write_all(&decomp).map_err(|_| EncError::Read)?;
    fs::rename(temporary, path).map_err(|_| EncError::Read)?;
    remove(name, settins);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Safe, move_out};
    use crate::settings::Settings;

    #[test]
    fn stores_and_restores_binary_files() {
        let dir = std::env::temp_dir().join(format!("secure_safe-{}", std::process::id()));
        let source = dir.join("source.bin");
        let settings = Settings { enc_dir: dir.join("vault") };
        let contents = [0, 159, 146, 150, 255];

        fs::create_dir_all(&settings.enc_dir).unwrap();
        fs::write(&source, contents).unwrap();

        Safe::new(source.to_str().unwrap(), b"password").store(&settings);
        fs::remove_file(&source).unwrap();
        move_out(b"password", &settings, "source.bin").unwrap();

        assert_eq!(fs::read(&source).unwrap(), contents);
        fs::remove_dir_all(dir).unwrap();
    }
}
