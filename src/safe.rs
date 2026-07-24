use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use argon2::{
    Argon2,
    password_hash::{Salt, SaltString},
};
use chacha20poly1305::{ChaChaPoly1305, KeyInit, XChaCha20Poly1305, XNonce, aead::Aead};
use rand_core::{OsRng, RngCore};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    PASSWORD_SIZE,
    settings::{self, Settings, Store},
};

fn read_file(path: &str) -> Vec<u8> {
    let mut file = File::open(path).expect("couldnt read the file check the path");

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    bytes
}

const NOUNCE_SIZE: usize = 24;
pub struct Raw {
    pub salt: [u8; 16],
    pub nounce: [u8; NOUNCE_SIZE],
    pub ciphertext: Vec<u8>,
    pub name: String,
    pub path: [u8; 54],
}

pub struct Cooked {
    pub text: String,
    pub name: String,
    pub path: String,
}

pub struct Safe<State> {
    pub state: State,
}

impl Safe<Raw> {
    pub fn new(path: &str, password: &[u8; PASSWORD_SIZE]) -> Self {
        //read and compress
        let contents = read_file(path);
        let contents = zstd::encode_all(Cursor::new(&contents), 5).unwrap();

        let name = PathBuf::from(path);
        let name = name.file_name().expect("not valid file name");

        let salt = SaltString::generate(&mut OsRng);

        let mut key = Zeroizing::new([0u8; 34]);
        Argon2::default()
            .hash_password_into(password, salt.to_string().as_bytes(), &mut *key)
            .expect(obfstr::obfstr!("error deriving password"));

        let cipher = XChaCha20Poly1305::new_from_slice(&*key).unwrap();

        let mut nounce = [0u8; NOUNCE_SIZE];
        OsRng.fill_bytes(&mut nounce);

        let ciphertext = cipher
            .encrypt(&nounce.into(), contents.as_ref())
            .map_err(|_| "encryption failed")
            .unwrap();

        let mut buf = [0u8; 16];

        Self {
            state: Raw {
                salt: salt
                    .as_salt()
                    .decode_b64(&mut buf)
                    .unwrap()
                    .try_into()
                    .unwrap(),
                name: name.to_str().unwrap().to_owned(),
                ciphertext,
                path: path.as_bytes().try_into().unwrap(),
                nounce,
            },
        }
    }
}

impl Safe<Raw> {
    pub fn store(self, settins: &Settings) {
        let path = settins.enc_dir.join(self.state.name.clone());

        let mut file = File::create(path).unwrap();
        file.write_all(&self.state.salt).unwrap();
        file.write_all(&self.state.nounce).unwrap();
        file.write_all(&self.state.path).unwrap();
        file.write_all(&self.state.ciphertext).unwrap();
    }
}

pub fn remove(name: &str, settings: &Settings) {
    let path = settings.enc_dir.join(name);
    fs::remove_file(path).unwrap();
}

pub fn check(settings: &Settings) {
    let dir = &settings.enc_dir;

    for item in fs::read_dir(dir).unwrap() {
        let entry = item.unwrap();
        let path = entry.path();
        if path.is_file() {
            let mut file = File::open(path).unwrap();

            let mut path = [0u8; 54];

            file.seek(SeekFrom::Start(16 + NOUNCE_SIZE as u64)).unwrap();
            file.read_exact(&mut path).unwrap();

            let path = Path::new(std::str::from_utf8(&path).unwrap());
            let name = path.file_name().unwrap();
            println!("{:?}", name);
        }
    }
}
pub fn move_out(password: &[u8; PASSWORD_SIZE], settins: &Settings, name: &str) {
    let mut file = File::open(settins.enc_dir.join(name)).expect("check if the name is correct");

    let mut salt = [0u8; 16];
    let mut nounce = [0u8; NOUNCE_SIZE];
    let mut path = [0u8; 54];
    let mut cypehr = Vec::new();
    file.read_exact(&mut salt).unwrap();
    file.read_exact(&mut nounce).unwrap();
    file.read_exact(&mut path).unwrap();

    file.read_to_end(&mut cypehr).unwrap();

    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::default()
        .hash_password_into(password, &salt, &mut *key)
        .unwrap();

    let cipher = XChaCha20Poly1305::new_from_slice(&*key).unwrap();
    let decrypted = cipher
        .decrypt(&XNonce::from(nounce), cypehr.as_ref())
        .unwrap();
    let decomp = zstd::decode_all(Cursor::new(decrypted)).unwrap();
    let text = String::from_utf8(decomp).unwrap();
    let path = String::from_utf8(path.to_vec()).unwrap();

    let safe = Safe {
        state: Cooked {
            text,
            name: name.to_owned(),
            path,
        },
    };

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(safe.state.path)
        .unwrap();

    file.write_all(safe.state.text.as_bytes().as_ref()).unwrap();
    remove(name, settins);
}
