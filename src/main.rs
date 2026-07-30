use std::{env::args, fs, io, process::exit};

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

const EXIT_SUCCESS: i32 = 0;
const EXIT_FAILURE: i32 = 1;
const COMMAND_INDEX: usize = 1;
const PATH_INDEX: usize = 2;
const HELP_COLUMN_WIDTH: usize = 18;

fn main() {
    let args = parse();

    let settings = Settings::load();
    let args = resolve_path(args, &settings);

    match args {
        ResolvedFlags::Add(name) => {
            let password = ask_password();
            let new = Safe::new(&name, password.as_bytes());
            new.store(&settings);
            delete_file(&name);
        },
        ResolvedFlags::Remove(name) => {
            let mut answer = String::new();
            println!("are you sure you want to remove {} permanently? y/n", name);
            io::stdin().read_line(&mut answer).unwrap();

            if answer.trim() == "y" {
                remove(&name, &settings);
            } else {
                println!("file {} not deleted", name);
            }
        },
        ResolvedFlags::Check => {
            let password = ask_password();
            check(password.as_bytes(), &settings);
        },
        ResolvedFlags::MoveOut(name) => {
            let mut password = ask_password();
            loop {
                match move_out(password.as_bytes(), &settings, &name) {
                    Ok(_) => break,
                    Err(EncError::UnZip(file_name)) => {
                        eprintln!("error unziping file: {}", file_name);
                        break;
                    },
                    Err(EncError::Decryption(file_name)) => {
                        eprintln!("error decrypting {}", file_name);
                        password = ask_password();
                        // try again with new password
                    },
                    Err(EncError::Read) => {
                        eprintln!("error reading file try again later");
                        exit(EXIT_SUCCESS);
                    },
                };
            }
        },
    }
}

enum Flags {
    Add(Option<String>),
    MoveOut(Option<String>),
    Remove(Option<String>),
    Check,
}

fn resolve_path(flag: Flags, settings: &Settings) -> ResolvedFlags {
    match flag {
        Flags::Add(path) => ResolvedFlags::Add(path.unwrap_or_else(|| explorer_path(select_source_file()))),
        Flags::MoveOut(name) => ResolvedFlags::MoveOut(name.unwrap_or_else(|| explorer_path(select_vault_entry(&settings.enc_dir)))),
        Flags::Remove(name) => ResolvedFlags::Remove(name.unwrap_or_else(|| explorer_path(select_vault_entry(&settings.enc_dir)))),
        Flags::Check => ResolvedFlags::Check,
    }
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

fn explorer_path(result: io::Result<std::path::PathBuf>) -> String {
    result
        .unwrap_or_else(|error| {
            eprintln!("could not open file explorer: {error}");
            exit(EXIT_FAILURE);
        })
        .into_os_string()
        .into_string()
        .unwrap_or_else(|_| {
            eprintln!("selected path is not valid UTF-8");
            exit(EXIT_FAILURE);
        })
}

fn ask_password() -> Zeroizing<String> {
    loop {
        let password = match rpassword::prompt_password(obfstr::obfstr!("Password: ")) {
            Ok(password) => Zeroizing::new(password),
            Err(_) => {
                eprintln!("impossible to read password");
                panic!();
            },
        };

        println!(
            "{}",
            obfstr::obfstr!("tip: if this is your first time using this program dont wory about not having a password (we know you dont have one set) just choose one and DONT forget it, it is SUPER important in the future !!!")
        );

        if password.is_empty() {
            println!("{}", obfstr::obfstr!("choose a password"));
        } else {
            return password;
        }
    }
}

fn parse() -> Flags {
    fn help() -> ! {
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

        exit(EXIT_SUCCESS);
    }
    let args: Vec<String> = args().collect();

    if args.is_empty() {
        println!("no args where provided");
        exit(EXIT_SUCCESS);
    }

    let arg = args.get(COMMAND_INDEX).unwrap_or_else(|| {
        println!("no args provided");
        help();
    });

    if arg == "--help" || arg == "help" || arg == "-h" || arg == "--h"{
        help();
    } else if arg == "--check" || arg == "check" {
        Flags::Check
    } else {
        let path = args.get(PATH_INDEX).cloned();

        command_flag(arg, path).unwrap_or_else(|| {
            println!("incorrect argument");
            exit(EXIT_FAILURE);
        })
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
