use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    thread::sleep,
    time::Duration,
};

use owo_colors::OwoColorize;

use crate::settings::configs::Configs;

pub fn resolve_stored_file(base_path: &Path, name: &str) -> io::Result<std::path::PathBuf> {
    if !fs::symlink_metadata(base_path)?.file_type().is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid vault directory"));
    }

    if Path::new(name).file_name().and_then(|file_name| file_name.to_str()) != Some(name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid stored file name"));
    }

    Ok(base_path.join(name))
}

pub fn confirm_intents(configs: &Configs, file_path: &Path) -> io::Result<()> {
    println!("{}", "this is a destructive action are you sure you want to continue?".red().bold());

    sleep(Duration::from_secs(3));
    println!("{}", "press YES if you want to proceed".red());

    let mut answer = String::new();

    io::stdin().read_line(&mut answer).unwrap();

    let answer = answer.trim();

    if answer == "YES" {
        wipe(configs, file_path)?;
    }
    Ok(())
}

fn wipe(configs: &Configs, file_path: &Path) -> io::Result<()> {
    ensure_safe_to_wipe(&fs::symlink_metadata(file_path)?)?;

    let times = configs.overwrite_times();

    match times {
        0 => {
            fs::remove_file(file_path)?;
        },
        times => {
            for _ in 0..times {
                let mut file = open_for_wipe(file_path)?;
                let file_size = file.metadata()?.size();

                let dead_data = vec![0u8; file_size as usize];

                file.write_all(&dead_data)?;
                file.sync_all()?;
            }
            fs::remove_file(file_path)?;

            println!("file was permanently removed");
        },
    }
    Ok(())
}

fn open_for_wipe(file_path: &Path) -> io::Result<File> {
    let file = OpenOptions::new().write(true).custom_flags(libc::O_NOFOLLOW).open(file_path)?;

    ensure_safe_to_wipe(&file.metadata()?)?;
    Ok(file)
}

fn ensure_safe_to_wipe(metadata: &fs::Metadata) -> io::Result<()> {
    if !metadata.file_type().is_file() || metadata.nlink() != 1 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "refusing to wipe a linked or non-regular file"));
    }

    Ok(())
}
