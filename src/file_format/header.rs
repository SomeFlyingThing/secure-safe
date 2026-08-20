use std::{
    fs::{self, File},
    io::{self, Write},
    marker::PhantomData,
    ops::Deref,
    path::Path,
};

const MARKER: &str = "secure_safe";

pub trait Save {
    fn check_to_save(&self);
    fn save(self) -> io::Result<Vec<u8>>;
}

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

        contents.extend_from_slice(self.path.unwrap().as_bytes());
        contents.extend_from_slice(&self.path_len.unwrap().to_be_bytes());
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
    let mut file = File::open(&tmp)?;

    file.write_all(contents)?;
    file.sync_all()?;

    fs::rename(tmp, path)?;

    Ok(())
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
