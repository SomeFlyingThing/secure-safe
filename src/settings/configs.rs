use std::{
    env::home_dir,
    fs::File,
    io::{self, ErrorKind, Read},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::file_format::header::Load;

const CONFIG_NAME: &str = "secure-safe.toml";

fn construct_settings_path() -> PathBuf {
    let home = home_dir().expect("error finding home dir");

    home.join(CONFIG_NAME)
}

#[derive(Serialize, Deserialize, Default)]
pub struct Configs {
    overwrite_times: u8,
}

impl Load for Configs {
    type Input = ();
    type Output = Self;

    fn load(_: &Self::Input) -> std::io::Result<Self::Output> {
        let path = construct_settings_path();

        let mut data = Vec::new();

        if !path.exists() {
            return Ok(Configs::default());
        }

        let mut file = File::open(path)?;

        file.read_to_end(&mut data)?;

        let config = toml::from_slice(&data).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;

        Ok(config)
    }
}
impl Configs {
    pub fn load() -> io::Result<Configs> {
        <Configs as Load>::load(&())
    }
    pub const fn overwrite_times(&self) -> u8 {
        self.overwrite_times
    }
}
