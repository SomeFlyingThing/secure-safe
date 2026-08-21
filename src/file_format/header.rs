use std::{
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Seek, Write},
    marker::PhantomData,
    ops::Deref,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use crate::{encryption::contents, read_file};

const MARKER: &str = "secure_safe";

pub trait Save {
    fn check_to_save(&self);
    fn save(self) -> io::Result<Vec<u8>>;
}

pub trait Load {
    type Input: ?Sized;
    type Output;

    fn load(path: &Self::Input) -> io::Result<Self::Output>;
}

#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Red;
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Unconfigured;

#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Configured;

#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Header<State> {
    marker: &'static str,
    path: Option<String>,
    path_len: Option<u64>,
    hash: Option<[u8; 32]>,
    _data: PhantomData<State>,
}

impl Header<Unconfigured> {
    pub const fn default() -> Self {
        Self {
            marker: MARKER,
            path: None,
            path_len: None,
            hash: None,
            _data: PhantomData,
        }
    }
}

impl Header<Unconfigured> {
    /// only call the sub functions
    /// return the configed version
    pub fn configure(mut self, path: &Path) -> Header<Configured> {
        self.with_path(path);

        Header::<Configured> {
            marker: self.marker,
            path: self.path,
            path_len: self.path_len,
            hash: self.hash,
            _data: PhantomData,
        }
    }
    fn with_path(&mut self, path: &Path) {
        let path = path.to_string_lossy().to_string();
        let len = path.len() as u64;

        self.path = Some(path);
        self.path_len = Some(len);
    }
}

impl Save for Header<Configured> {
    fn check_to_save(&self) {
        if self.hash.is_none() || self.path.is_none() || self.path_len.is_none() {
            panic!("missing data in header");
        }
    }
    fn save(self) -> io::Result<Vec<u8>> {
        self.check_to_save();

        let mut contents = Vec::new();
        contents.extend_from_slice(self.marker.as_bytes());

        contents.extend_from_slice(&self.path_len.unwrap().to_be_bytes());
        contents.extend_from_slice(self.path.unwrap().as_bytes());
        contents.extend_from_slice(&self.hash.unwrap());

        Ok(contents)
    }
}

impl Header<Configured> {
    pub fn hash(&mut self, contents: &[u8]) {
        let hash = blake3::hash(contents);
        let hash_b = hash.as_bytes();

        self.hash = Some(*hash_b);
    }
    pub fn file_name(&self) -> String {
        self.path.clone().unwrap()
    }
}
pub fn atomic_write(contents: &[u8], path: &Path) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = OpenOptions::new().write(true).truncate(true).mode(0o600).open(&tmp)?;

    file.write_all(contents)?;

    fs::rename(tmp, path)?;
    file.sync_all()?;

    Ok(())
}

//from verb 'read'
impl Load for Header<Red> {
    type Input = Path;
    type Output = (Self, usize);

    fn load(path:& Self::Input) -> io::Result<Self::Output> {
        let mut file = File::open(path)?;

        let mut marker = [0u8; MARKER.len()];
        //a u64 is 8 bytes long
        let mut path_len = [0u8; 8];

        file.read_exact(&mut marker)?;

        if marker != MARKER.as_bytes() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "marker doesnt correspond to file  marker"));
        }

        file.read_exact(&mut path_len)?;

        //get the path
        let path_len = u64::from_be_bytes(path_len);
        let mut path = Vec::with_capacity(path_len as usize);
        file.read_exact(&mut path)?;

        let path = String::from_utf8(path).expect("path isnt valid utf8");

        let mut hash = [0u8; 32];

        file.read_exact(&mut hash)?;

        let mut rest = Vec::new();

        file.read_to_end(&mut rest)?;
        let contents_hash = blake3::hash(&rest);
        let contents_hash = contents_hash.as_bytes();

        if hash != *contents_hash {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "currupted file"));
        }
        Ok((
            Self {
                marker: MARKER,
                path_len: Some(path_len),
                path: Some(path),
                hash: Some(hash),
                _data: PhantomData,
            },
            file.stream_position()? as usize,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::io::stdin;

    use tempfile::tempdir;

    use super::*;
    use crate::{Password, file_format::header::Configured};

    #[test]
    fn header() {
        let directory = tempdir().unwrap();
        let file_path = directory.path().join("test.tst");

        let header = Header::default();

        let header = header.configure(&file_path);

        let destiny = Header::<Configured> {
            marker: MARKER,
            path: Some(file_path.to_string_lossy().to_string()),
            path_len: Some(file_path.to_string_lossy().to_string().len() as u64),
            hash: None,
            _data: PhantomData,
        };

        assert!(header == destiny);
    }
}
