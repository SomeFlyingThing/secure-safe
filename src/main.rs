use std::{
    any,
    env::home_dir,
    error::Error,
    fs::File,
    hint::assert_unchecked,
    io::{self, Read},
    ops::Deref,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use chacha20poly1305::consts::False;
use encryption::password::Password;

use crate::{
    add::add,
    login::has_login,
    parsing::{ParsedArgs, parse},
};

mod about;
mod add;
mod encryption;
mod file_format;
mod login;
mod parsing;
mod restore;

const PATH_NAME: &str = "secure-safe";

fn generate_path() -> Option<PathBuf> {
    Some(home_dir()?.join(PATH_NAME))
}

fn main() -> anyhow::Result<()> {
    let args = parse().map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, "no args"))?;

    let basep = generate_path().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "couldnt get homedir"))?;

    let password = match has_login() {
        true => Password::recover()?,
        false => Password::new()?,
    };

    let password = password.derive()?;

    match args {
        ParsedArgs::Restore(path) => {
            
            todo!()
        },

        ParsedArgs::Delete(path) => {
            if !path.starts_with(basep) {
                eprintln!("invalid path");
                return Err(io::Error::new(io::ErrorKind::InvalidFilename, "invalid path").into());
            }
            todo!()
        },
        ParsedArgs::Add(path) => {
            let file_contents = read_file(&path)?;
            add(&password, &path, &file_contents)?;
        },
        ParsedArgs::About => about::about(),
    }
    Ok(())
}

fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let size = file.metadata()?.size();
    let mut vec = Vec::with_capacity(size as usize);

    file.read_to_end(&mut vec)?;

    Ok(vec)
}
