use std::{
    io::{self, Error},
    path::Path,
};

use inotify::{self, EventMask, Inotify, WatchMask};

use crate::{
    add::add,
    encryption::password::{Derived, Password},
};

pub fn watch_dir(path: &Path, password: &Password<Derived>) -> io::Result<!> {
    if path.is_file() {
        eprintln!("whatch is for a directory not a file");
        return Err(Error::new(io::ErrorKind::InvalidData, "whatch is for a directory not a file"));
    } else if !path.try_exists()? {
        eprintln!("path doesnt exist");
        return Err(Error::new(io::ErrorKind::NotFound, "path doesnt exist"));
    }

    let mut inotify = Inotify::init()?;

    inotify.watches().add(path, WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO)?;

    let mut events = [0u8; 1024];

    loop {
        let events = inotify.read_events(&mut events)?;

        for event in events {
            if !(event.mask.contains(EventMask::CLOSE_WRITE) || event.mask.contains(EventMask::MOVED_TO)) {
                continue;
            }

            let Some(name) = event.name else {
                continue;
            };
            let file_path = path.join(name);

            if !file_path.is_file() {
                continue;
            }

            let contents = std::fs::read(&file_path)?;
            add(password, &file_path, &contents)?;
        }
    }
}
