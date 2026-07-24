use std::{
    env,
    fs::{self, File},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    process::exit,
};

use serde::{Deserialize, Serialize};

pub trait Store {
    fn store(&self);
}

const SETTINGS_NAME: &str = "secure_safe.settings";
const DEFAULT_ENC_DIR: &str = ".safe_dir";

fn settings_path() -> PathBuf {
    let home = env::home_dir().expect("impossible to get homedir");

    home.join(SETTINGS_NAME).to_owned()
}

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub enc_dir: PathBuf,
}

impl Store for Settings {
    fn store(&self) {
        let toml = toml::to_string_pretty(self).expect("settings file is wrongly formatted");
        fs::write(settings_path(), toml).unwrap();
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enc_dir: settings_path().parent().unwrap().join(DEFAULT_ENC_DIR),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();

        let mut file = match File::open(path.clone()) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let settings = Settings::default();
                settings.store();
                return settings;
            },
            Err(_) => {
                panic!("unexpected error");
            },
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        if contents.is_empty() {
            println!("configure settings at {:?}", path);
            exit(0);
        }

        toml::from_str(&contents).expect(obfstr::obfstr!("toml file might be wrongly formatted"))
    }
}
