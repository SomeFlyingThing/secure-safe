use std::{
    env::home_dir,
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use secure_safe::file_format;

use crate::{
    add::{add, exists},
    delete::{confirm_intents, resolve_stored_file},
    login::authenticate,
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

    let password = authenticate()?;

    //load configs
    let configs = Configs::load()?;
    match args {
        ParsedArgs::Restore(name) => {
            restore::restore(&name, &password)?;
        },

        ParsedArgs::Delete(name) => {
            let item = resolve_stored_file(&basep, &name)?;
            confirm_intents(&configs, &item)?;
        },
        ParsedArgs::Add(path) => {
            if exists(&path.file_name().unwrap().to_string_lossy()) {
                eprintln!("a file with that name already exists");
            }
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
