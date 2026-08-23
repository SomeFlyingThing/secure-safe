use std::{
    env::home_dir,
    fs::File,
    hint::assert_unchecked,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use encryption::password::Password;
use secure_safe::file_format;

use crate::{
    add::add,
    delete::confirm_intents,
    login::has_login,
    parsing::{ParsedArgs, parse},
    settings::configs::Configs,
};

mod about;
mod add;
mod delete;
mod encryption;
mod login;
mod parsing;
mod restore;
mod settings;

const PATH_NAME: &str = "secure-safe";

fn generate_path() -> Option<PathBuf> {
    Some(home_dir()?.join(PATH_NAME))
}

fn main() -> anyhow::Result<()> {
    let args = parse().map_err(|_error| io::Error::new(io::ErrorKind::InvalidInput, "no args"))?;

    let basep = generate_path().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "couldnt get homedir"))?;

    let password = match has_login() {
        true => Password::recover()?,
        false => Password::new()?,
    };

    let password = password.derive()?;

    //load configs
    let configs = Configs::load()?;
    match args {
        ParsedArgs::Restore(name) => {
            restore::restore(&name, &password)?;
        },

        ParsedArgs::Delete(path) => {
            if !path.starts_with(basep) {
                eprintln!("invalid path");
                return Err(io::Error::new(io::ErrorKind::InvalidFilename, "invalid path").into());
            }
            confirm_intents(&configs, &path)?;
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
