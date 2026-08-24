use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

const MARKER: [u8; 11] = *b"secure_safe";

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
    marker: &'static [u8; 11],
    path: Option<String>,
    path_len: Option<u64>,
    hash: Option<[u8; 32]>,
    _data: PhantomData<State>,
}

impl Header<Unconfigured> {
    pub const fn default() -> Self {
        Self {
            marker: &MARKER,
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
        contents.extend_from_slice(self.marker);

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
        Path::new(self.path.as_deref().unwrap()).file_name().unwrap().to_string_lossy().into_owned()
    }
}
pub fn atomic_write(contents: &[u8], path: &Path) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;

    file.write_all(contents)?;
    file.sync_all()?;

    fs::rename(tmp, path)?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }

    Ok(())
}

impl Header<Red> {
    #[cfg(feature = "fuzzing")]
    pub fn fuzzing_load_inner(data: &[u8]) -> Result<(Header<Red>, usize), io::Error> {
        Self::load_inner(io::Cursor::new(data))
    }
    fn load_inner<R: Read + Seek>(mut reader: R) -> Result<(Header<Red>, usize), io::Error> {
        let mut marker = [0u8; MARKER.len()];
        //a u64 is 8 bytes long
        let mut path_len = [0u8; 8];

        reader.read_exact(&mut marker)?;

        if marker != MARKER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "marker doesnt correspond to file  marker"));
        }

        reader.read_exact(&mut path_len)?;

        //get the path
        let path_len = u64::from_be_bytes(path_len);
        let path_start = reader.stream_position()?;
        let input_end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(path_start))?;

        // A valid header must still contain the 32-byte hash after the path.
        // Validate against the actual input before converting or allocating.
        let available_path_bytes = input_end.saturating_sub(path_start).saturating_sub(32);
        if path_len > available_path_bytes {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "path length exceeds input"));
        }

        let path_len_usize = usize::try_from(path_len).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "path length is too large"))?;
        let mut path = vec![0; path_len_usize];
        reader.read_exact(&mut path)?;

        let path = String::from_utf8(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "path is not valid UTF-8"))?;

        let mut hash = [0u8; 32];

        reader.read_exact(&mut hash)?;

        let file_ptr_location = reader.stream_position()? as usize;

        let mut rest = Vec::new();

        reader.read_to_end(&mut rest)?;
        let contents_hash = blake3::hash(&rest);
        let contents_hash = contents_hash.as_bytes();

        if hash != *contents_hash {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "currupted file"));
        }
        Ok((
            Self {
                marker: &MARKER,
                path_len: Some(path_len),
                path: Some(path),
                hash: Some(hash),
                _data: PhantomData,
            },
            file_ptr_location,
        ))
    }
}
//from verb 'read'
impl Load for Header<Red> {
    type Input = Path;
    type Output = (Self, usize);

    fn load(path: &Self::Input) -> io::Result<Self::Output> {
        let file = File::open(path)?;
        Self::load_inner(file)
    }
}

impl Header<Red> {
    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.path.as_deref().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::file_format::header::Configured;

    #[test]
    fn header() {
        let directory = tempdir().unwrap();
        let file_path = directory.path().join("test.tst");

        let header = Header::default();

        let header = header.configure(&file_path);

        let destiny = Header::<Configured> {
            marker: &MARKER,
            path: Some(file_path.to_string_lossy().to_string()),
            path_len: Some(file_path.to_string_lossy().to_string().len() as u64),
            hash: None,
            _data: PhantomData,
        };

        assert!(header == destiny);
    }

    #[test]
    fn rejects_path_length_larger_than_input() {
        let mut input = MARKER.to_vec();
        input.extend_from_slice(&u64::MAX.to_be_bytes());

        let error = Header::<Red>::load_inner(io::Cursor::new(input)).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_non_utf8_path() {
        let mut input = MARKER.to_vec();
        input.extend_from_slice(&1_u64.to_be_bytes());
        input.push(0xff);
        input.extend_from_slice(&[0; 32]);

        let error = Header::<Red>::load_inner(io::Cursor::new(input)).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
