use std::{env::home_dir, path::Path};

use crate::encryption::contents::SALT_PATH;

pub fn has_login() -> bool {
    let saltloc = home_dir().unwrap().join(SALT_PATH);

    saltloc.is_file()
}
