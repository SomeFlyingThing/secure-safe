use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::Path,
    thread::sleep,
    time::Duration,
};

use owo_colors::OwoColorize;

use crate::settings::configs::Configs;
pub fn confirm_intents(configs: &Configs, file_path: &Path) ->io::Result<()>{
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
    let times = configs.overwrite_times();

    match times {
        0 => {
            fs::remove_file(file_path)?;
        },
        times => {
            for _ in 0..times {
                let mut file = OpenOptions::new().write(true).open(file_path)?;

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
