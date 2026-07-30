use std::{
    env,
    fs::{self, File},
    io::{self, ErrorKind, Read},
    path::PathBuf,
    process::exit,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub trait Store {
    fn store(&self) -> anyhow::Result<()>;
}

const SETTINGS_NAME: &str = "secure_safe.settings";
const DEFAULT_ENC_DIR: &str = ".safe_dir";

fn settings_path() -> io::Result<PathBuf> {
    let home = env::home_dir().ok_or_else(|| io::Error::new(ErrorKind::NotFound, "impossible to get homedir"))?;

    Ok(home.join(SETTINGS_NAME).to_owned())
}

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub enc_dir: PathBuf,
}

impl Store for Settings {
    fn store(&self) -> anyhow::Result<()> {
        let toml = toml::to_string_pretty(self).context("settings file is wrongly formatted")?;
        fs::write(settings_path()?, toml)?;
        Ok(())
    }
}

impl Settings {
    fn default() -> io::Result<Self> {
        Ok(Self {
            enc_dir: settings_path()?.parent().ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "settings path has no parent"))?.join(DEFAULT_ENC_DIR),
        })
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = settings_path()?;

        let mut file = match File::open(path.clone()) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let settings = Settings::default()?;
                fs::create_dir_all(&settings.enc_dir)?;
                settings.store()?;
                return Ok(settings);
            },
            Err(err) => return Err(err.into()),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        if contents.is_empty() {
            println!("configure settings at {:?}", path);
            exit(crate::EXIT_SUCCESS);
        }

        let settings = toml::from_str::<Settings>(&contents).context(obfstr::obfstr!("toml file might be wrongly formatted").to_owned())?;
        fs::create_dir_all(&settings.enc_dir)?;
        Ok(settings)
    }
}
