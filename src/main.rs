use std::{env::args, fs, io, path::PathBuf};

use anyhow::Context;
use owo_colors::OwoColorize;
use zeroize::Zeroizing;

use crate::{
    navigation::file_explorer::{select_source_file, select_vault_entry},
    safe::{EncError, Safe, check, move_out, remove},
    settings::Settings,
};

mod navigation;
mod safe;
mod settings;

const COMMAND_INDEX: usize = 1;
const PATH_INDEX: usize = 2;
const HELP_COLUMN_WIDTH: usize = 18;

fn main() -> anyhow::Result<()> {
    let Some(args) = parse() else {
        return Ok(());
    };

    let settings = Settings::load()?;
    let Some(args) = resolve_path(args, &settings)? else {
        return Ok(());
    };

    match args {
        ResolvedFlags::Add(name) => {
            let password = ask_password()?;
            let new = Safe::new(&name, password.as_bytes())?;
            new.store(&settings)?;
            delete_file(&name);
            println!("{} was successfully saved", PathBuf::from(name).file_name().context("path has no file name")?.display());
        },
        ResolvedFlags::Remove(name) => {
            let mut answer = String::new();
            println!("are you sure you want to remove {name} permanently? y/n");
            io::stdin().read_line(&mut answer)?;

            if answer.trim() == "y" {
                remove(&name, &settings)?;
            } else {
                println!("file {name} not deleted");
            }
            println!("{} was successfully removed", PathBuf::from(name).file_name().context("path has no file name")?.display());
        },
        ResolvedFlags::Check => {
            let password = ask_password()?;
            check(password.as_bytes(), &settings)?;
        },
        ResolvedFlags::MoveOut(name) => {
            let mut password = ask_password()?;
            loop {
                match move_out(password.as_bytes(), &settings, &name) {
                    Ok(()) => break,
                    Err(EncError::UnZip(file_name)) => {
                        eprintln!("error unziping file: {file_name}");
                        break;
                    },
                    Err(EncError::Decryption(file_name)) => {
                        eprintln!("error decrypting {file_name}");
                        password = ask_password()?;
                        // try again with new password
                    },
                    Err(EncError::Read) => {
                        eprintln!("error reading file try again later");
                        return Ok(());
                    },
                    Err(error) => return Err(error.into()),
                }
            }
        },
    }
    Ok(())
}

enum Flags {
    Add(Option<String>),
    MoveOut(Option<String>),
    Remove(Option<String>),
    Check,
}

fn resolve_path(flag: Flags, settings: &Settings) -> io::Result<Option<ResolvedFlags>> {
    Ok(match flag {
        Flags::Add(Some(path)) => Some(ResolvedFlags::Add(path)),
        Flags::Add(None) => explorer_path(select_source_file())?.map(ResolvedFlags::Add),
        Flags::MoveOut(Some(name)) => Some(ResolvedFlags::MoveOut(name)),
        Flags::MoveOut(None) => explorer_path(select_vault_entry(&settings.enc_dir))?.map(ResolvedFlags::MoveOut),
        Flags::Remove(Some(name)) => Some(ResolvedFlags::Remove(name)),
        Flags::Remove(None) => explorer_path(select_vault_entry(&settings.enc_dir))?.map(ResolvedFlags::Remove),
        Flags::Check => Some(ResolvedFlags::Check),
    })
}

enum ResolvedFlags {
    Add(String),
    MoveOut(String),
    Remove(String),
    Check,
}

fn command_flag(command: &str, path: Option<String>) -> Option<Flags> {
    match command {
        "--add" | "add" => Some(Flags::Add(path)),
        "--rm" | "rm" => Some(Flags::Remove(path)),
        "--mo" | "mo" => Some(Flags::MoveOut(path)),
        _ => None,
    }
}

fn explorer_path(result: io::Result<Option<std::path::PathBuf>>) -> io::Result<Option<String>> {
    result
        .map_err(|error| io::Error::new(error.kind(), format!("could not open file explorer: {error}")))?
        .map(|path| {
            path.into_os_string()
                .into_string()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "selected path is not valid UTF-8"))
        })
        .transpose()
}

fn ask_password() -> io::Result<Zeroizing<String>> {
    loop {
        let password = Zeroizing::new(rpassword::prompt_password(obfstr::obfstr!("Password: "))?);

        println!(
            "{}",
            obfstr::obfstr!(
                "tip: if this is your first time using this program dont wory about not having a password (we know you dont have one set) just choose one and DONT forget it, it is SUPER important in the future !!!"
            )
        );

        if password.is_empty() {
            println!("{}", obfstr::obfstr!("choose a password"));
        } else {
            let sec_password = Zeroizing::new(rpassword::prompt_password(obfstr::obfstr!("Confirm Password: "))?);

            if password != sec_password {
                println!("{}", obfstr::obfstr!("passwords are diffrent, input again"));
                continue;
            }
            if password == sec_password {
                return Ok(password);
            }
        }
    }
}

fn parse() -> Option<Flags> {
    fn help() {
        println!(
            "\
    {}

    {}

    {}

        secure_safe <COMMAND> [PATH]

    {}

        {:<HELP_COLUMN_WIDTH$} Encrypt, store, and remove the original file
        {:<HELP_COLUMN_WIDTH$} Permanently remove a stored file
                          {}
        {:<HELP_COLUMN_WIDTH$} Decrypt and restore a stored file
        {:<HELP_COLUMN_WIDTH$} Verify the integrity of the database
        {:<HELP_COLUMN_WIDTH$} Display this help message

    {}

        secure_safe add secret.txt
        secure_safe rm secret.txt
        secure_safe mo secret.txt
        secure_safe check
    ",
            "secure_safe".bold().bright_white(),
            "Simple Super Safe encrypted file vault.".italic(),
            "USAGE".bold().cyan(),
            "COMMANDS".bold().cyan(),
            "add,   --add <PATH>",
            "rm,    --rm <NAME>",
            "WARNING: This action is irreversible.".red().bold(),
            "mo,    --mo <NAME>",
            "check, --check",
            "help,  --help",
            "EXAMPLES".bold().cyan(),
        );
    }
    let args: Vec<String> = args().collect();

    let Some(arg) = args.get(COMMAND_INDEX) else {
        println!("no command provided");
        help();
        return None;
    };

    if arg == "--help" || arg == "help" || arg == "-h" || arg == "--h" {
        help();
        None
    } else if arg == "--check" || arg == "check" {
        Some(Flags::Check)
    } else {
        let path = args.get(PATH_INDEX).cloned();

        if let Some(flag) = command_flag(arg, path) {
            Some(flag)
        } else {
            println!("incorrect argument");
            None
        }
    }
}

fn delete_file(path: &str) {
    fs::remove_file(path).unwrap_or_else(|_| {
        println!("coundnt remove the file, remove it manually");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_without_paths_preserve_their_explorer_mode() {
        assert!(matches!(command_flag("add", None), Some(Flags::Add(None))));
        assert!(matches!(command_flag("rm", None), Some(Flags::Remove(None))));
        assert!(matches!(command_flag("mo", None), Some(Flags::MoveOut(None))));
        assert!(command_flag("unknown", None).is_none());
    }
}
