use std::{
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Seek, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

use anyhow::Context;
use argon2::{Argon2, password_hash::SaltString};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::settings::Settings;

fn read_file(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}

const SALT_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const NOUNCE_SIZE: usize = 24;
const MAX_PATH_SIZE: usize = 16 * 1024;
const PATH_SIZE_BYTES: usize = size_of::<u32>();
const COMPRESSION_LEVEL: i32 = 5;

fn sync_parent(path: &Path) -> io::Result<()> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()
}

fn temporary_path(final_path: &Path) -> io::Result<PathBuf> {
    let name = final_path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut random = [0u8; 8];
    OsRng.fill_bytes(&mut random);

    let mut temporary_name = name.to_os_string();
    temporary_name.push(format!(".secure_safe.tmp.{:016x}", u64::from_le_bytes(random)));
    Ok(final_path.with_file_name(temporary_name))
}

struct TemporaryFile {
    path: PathBuf,
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub struct Raw {
    pub salt: [u8; SALT_SIZE],
    pub nounce: [u8; NOUNCE_SIZE],
    pub ciphertext: Vec<u8>,
    pub name: String,
    pub path: Vec<u8>,
}

const MARKER: &[u8] = b"secure-safe";

pub struct Safe<State> {
    pub state: State,
}

impl Safe<Raw> {
    pub fn new(path: &str, password: &[u8]) -> anyhow::Result<Self> {
        //read and compress
        let contents = read_file(path)?;
        let contents = zstd::encode_all(Cursor::new(&contents), COMPRESSION_LEVEL)?;

        let name = PathBuf::from(path);
        let name = name.file_name().context("not valid file name")?;

        let salt = SaltString::generate(&mut OsRng);

        let mut key = Zeroizing::new([0u8; KEY_SIZE]);
        let mut salt_bytes = [0u8; SALT_SIZE];
        salt.as_salt().decode_b64(&mut salt_bytes)?;
        Argon2::default()
            .hash_password_into(password, &salt_bytes, &mut *key)
            .context(obfstr::obfstr!("error deriving password").to_owned())?;

        let cipher = XChaCha20Poly1305::new_from_slice(&*key)?;

        let mut nounce = [0u8; NOUNCE_SIZE];
        OsRng.fill_bytes(&mut nounce);

        let ciphertext = cipher
            .encrypt(&nounce.into(), Payload { msg: &contents, aad: path.as_bytes() })
            .map_err(|_| anyhow::anyhow!("encryption failed"))?;

        Ok(Self {
            state: Raw {
                salt: salt_bytes,
                name: name.to_str().context("file name is not valid UTF-8")?.to_owned(),
                ciphertext,
                path: path.as_bytes().to_vec(),
                nounce,
            },
        })
    }
}

impl Safe<Raw> {
    #[allow(clippy::cast_possible_truncation)]
    pub fn store(self, settins: &Settings) -> io::Result<()> {
        let path = settins.enc_dir.join(&self.state.name);
        if path.try_exists()? {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "vault entry already exists"));
        }

        let temporary = TemporaryFile { path: temporary_path(&path)? };
        let mut file = OpenOptions::new().mode(0o600).create_new(true).write(true).open(&temporary.path)?;

        file.write_all(MARKER)?;
        file.write_all(&self.state.salt)?;
        file.write_all(&self.state.nounce)?;
        file.write_all(&(self.state.path.len() as u32).to_le_bytes())?;
        file.write_all(&self.state.path)?;
        file.write_all(&self.state.ciphertext)?;
        file.sync_all()?;

        // Publishing with a hard link is atomic and, unlike rename(), cannot
        // replace an entry created by another process after the check above.
        fs::hard_link(&temporary.path, &path)?;
        sync_parent(&path)?;

        fs::remove_file(&temporary.path)?;
        sync_parent(&path)?;

        Ok(())
    }
}
#[allow(clippy::cast_possible_truncation)]
pub fn overwrite(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).read(true).open(path)?;

    let size = file.metadata()?.size();

    let bytes = vec![0u8; size as usize];

    file.seek(std::io::SeekFrom::Start(0))?;
    file.write_all(&bytes)?;
    file.sync_all()?;

    Ok(())
}
pub fn remove(name: &str, settings: &Settings) -> io::Result<()> {
    if Path::new(name).file_name().and_then(|name| name.to_str()) != Some(name) {
        eprintln!("invalid stored file name");
        return Ok(());
    }

    let path = settings.enc_dir.join(name);
    overwrite(&path)?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn check(password: &[u8], settings: &Settings) -> anyhow::Result<()> {
    let dir = &settings.enc_dir;

    for item in fs::read_dir(dir)? {
        let entry = item?;
        let path = entry.path();
        if path.is_file() {
            let mut file = File::open(&path)?;

            let mut marker = [0u8; MARKER.len()];
            let mut salt = [0u8; SALT_SIZE];
            let mut nounce = [0u8; NOUNCE_SIZE];
            let mut path_size = [0u8; PATH_SIZE_BYTES];

            file.read_exact(&mut marker)?;

            if marker != MARKER {
                handle_unkown_file(&path)?;
            }

            if file.read_exact(&mut salt).is_err() || file.read_exact(&mut nounce).is_err() || file.read_exact(&mut path_size).is_err() {
                eprintln!("invalid stored file: {}", path.display());
                continue;
            }

            let path_size = u32::from_le_bytes(path_size) as usize;
            if path_size > MAX_PATH_SIZE {
                eprintln!("invalid stored file: {}", path.display());
                continue;
            }

            let mut path_bytes = vec![0; path_size];
            let mut ciphertext = Vec::new();

            if file.read_exact(&mut path_bytes).is_err() || file.read_to_end(&mut ciphertext).is_err() {
                eprintln!("invalid stored file: {}", path.display());
                continue;
            }

            let mut key = Zeroizing::new([0u8; KEY_SIZE]);
            Argon2::default().hash_password_into(password, &salt, &mut *key)?;
            let cipher = XChaCha20Poly1305::new_from_slice(&*key)?;
            if cipher.decrypt(&XNonce::from(nounce), Payload { msg: &ciphertext, aad: &path_bytes }).is_err() {
                eprintln!("failed integrity check: {}", path.display());
                continue;
            }

            println!("{}", path.file_name().context("stored path has no file name")?.display());
        }
    }
    Ok(())
}

fn handle_unkown_file(file: &Path) -> io::Result<()> {
    println!("detected unkown file in special directory");
    println!("moving to parent dir");
    move_to_parent(file)?;

    Ok(())
}

fn move_to_parent(file: &Path) -> io::Result<()> {
    let file_name = file.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let destini = Path::new("..").join(file_name);
    fs::rename(file, destini)?;

    Ok(())
}

#[derive(Debug)]
pub enum EncError {
    Decryption(String),
    UnZip(String),
    Read,
    Crypto(String),
    Io(io::Error),
}

impl std::fmt::Display for EncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decryption(file_name) => write!(formatter, "Decryption({file_name:?})"),
            Self::UnZip(error) => write!(formatter, "UnZip({error:?})"),
            Self::Read => write!(formatter, "Read"),
            Self::Crypto(error) => write!(formatter, "Crypto({error:?})"),
            Self::Io(error) => write!(formatter, "Io({error:?})"),
        }
    }
}

impl std::error::Error for EncError {}

pub fn move_out(password: &[u8], settins: &Settings, name: &str) -> Result<(), EncError> {
    if Path::new(name).file_name().and_then(|name| name.to_str()) != Some(name) {
        return Err(EncError::Read);
    }

    let mut file = File::open(settins.enc_dir.join(name)).map_err(|_| EncError::Read)?;

    let mut marker = [0u8; MARKER.len()];

    let mut salt = [0u8; SALT_SIZE];
    let mut nounce = [0u8; NOUNCE_SIZE];
    let mut path_size = [0u8; PATH_SIZE_BYTES];
    let mut cypehr = Vec::new();

    file.read_exact(&mut marker).map_err(|_| EncError::Read)?;
    if marker != MARKER {
        handle_unkown_file(Path::new(name)).map_err(|_| EncError::Read)?;
    }
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

    let mut key = Zeroizing::new([0u8; KEY_SIZE]);
    Argon2::default().hash_password_into(password, &salt, &mut *key).map_err(|error| EncError::Crypto(error.to_string()))?;

    let path = String::from_utf8(path).map_err(|_| EncError::Read)?;

    // will be used for the file name
    let path_to_name = PathBuf::from(path.clone());

    let cipher = XChaCha20Poly1305::new_from_slice(&*key).map_err(|error| EncError::Crypto(error.to_string()))?;
    let file_name = path_to_name.file_name().and_then(|name| name.to_str()).ok_or(EncError::Read)?.to_owned();
    let decrypted = cipher
        .decrypt(&XNonce::from(nounce), Payload { msg: &cypehr, aad: path.as_bytes() })
        .map_err(|_| EncError::Decryption(file_name))?;

    let decomp = zstd::decode_all(Cursor::new(decrypted)).map_err(|error| EncError::UnZip(error.to_string()))?;
    let temporary = PathBuf::from(&path).with_extension("secure_safe.tmp");
    let mut file = OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|_| EncError::Read)?;
    file.write_all(&decomp).map_err(|_| EncError::Read)?;
    file.sync_all().map_err(EncError::Io)?;
    fs::rename(&temporary, &path).map_err(|_| EncError::Read)?;
    sync_parent(Path::new(&path)).map_err(EncError::Io)?;

    remove(name, settins).map_err(EncError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Context;

    use super::{Safe, move_out};
    use crate::settings::Settings;

    #[test]
    #[allow(clippy::panic_in_result_fn)]
    fn stores_and_restores_binary_files() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("secure_safe-{}", std::process::id()));
        let source = dir.join("source.bin");
        let settings = Settings { enc_dir: dir.join("vault") };
        let contents = [0, 159, 146, 150, 255];

        fs::create_dir_all(&settings.enc_dir)?;
        fs::write(&source, contents)?;

        Safe::new(source.to_str().context("source path is not valid UTF-8")?, b"password")?.store(&settings)?;
        assert_eq!(fs::read_dir(&settings.enc_dir)?.count(), 1);
        fs::remove_file(&source)?;
        move_out(b"password", &settings, "source.bin")?;

        assert_eq!(fs::read(&source)?, contents);
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    #[allow(clippy::panic_in_result_fn, clippy::unwrap_used)]
    fn store_does_not_replace_an_existing_entry() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("secure-safe-collision-{}", std::process::id()));
        let source = dir.join("source.bin");
        let settings = Settings { enc_dir: dir.join("vault") };
        let stored = settings.enc_dir.join("source.bin");

        fs::create_dir_all(&settings.enc_dir)?;
        fs::write(&source, b"new contents")?;
        fs::write(&stored, b"existing entry")?;

        let error = Safe::new(source.to_str().context("source path is not valid UTF-8")?, b"password")?.store(&settings).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&stored)?, b"existing entry");
        assert_eq!(fs::read_dir(&settings.enc_dir)?.count(), 1);
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
