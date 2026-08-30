use std::{
    env::home_dir,
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use secure_safe::file_format;

use crate::{
    add::{add, check_existence_n_handle},
    delete::{confirm_intents, resolve_stored_file},
    dir_watch::watch_dir,
    login::authenticate,
    parsing::{ParsedArgs, parse},
    settings::configs::Configs,
};

mod about;
mod add;
mod delete;
mod dir_watch;
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
            check_existence_n_handle(&path.file_name().unwrap().to_string_lossy())?;

            let file_contents = read_file(&path)?;
            add(&password, &path, &file_contents)?;
        },
        ParsedArgs::About => about::about(),

        ParsedArgs::WatchDir(path) => {
            watch_dir(&path, &password)?;
        },
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

#[cfg(test)]
mod test {
    use std::fs::{OpenOptions, create_dir_all, exists};

    use tempfile::{TempDir, tempdir};

    use super::*;

    fn create_space() {
        let path = generate_path().unwrap();

        create_dir_all(path);
    }
    #[test]
    fn existing_name() {
        let dir = tempdir().unwrap();
        let dir = dir.path();

        let filname = "potato";

        create_space();
        let path = dir.join(filname);
        let file = OpenOptions::new().create(true).write(true).open(&path).unwrap();

        let res = check_existence_n_handle(&path.to_string_lossy());
        assert!(res.is_err())
    }
}
