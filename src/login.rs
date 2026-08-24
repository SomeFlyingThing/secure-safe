use std::env::home_dir;

use crate::encryption::password::SALT_PATH;

pub fn has_login() -> bool {
    home_dir().is_some_and(|home| home.join(SALT_PATH).is_file())
}
