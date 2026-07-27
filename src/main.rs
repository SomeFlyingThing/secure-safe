use std::{env::args, io, process::exit};

use owo_colors::OwoColorize;
use zeroize::Zeroizing;

use crate::{
    safe::{EncError, Safe, check, move_out, remove},
    settings::Settings,
};

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

    match args {
        Flags::Add(name) => {
            let password = ask_password();
            let new = Safe::new(&name, password.as_bytes());
            new.store(&settings);
            remove(&name, &settings);
        },
        Flags::Remove(name) => {
            let mut answer = String::new();
            println!("are you sure you want to remove {} permanently? y/n", name);
            io::stdin().read_line(&mut answer).unwrap();

            if answer.trim() == "y" {
                remove(&name, &settings);
            } else {
                println!("file {} not deleted", name);
            }
        },
        Flags::Check => {
            let password = ask_password();
            check(password.as_bytes(), &settings);
        },
        Flags::MoveOut(name) => {
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
    Add(String),
    MoveOut(String),
    Remove(String),
    Check,
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

    if arg == "--help" || arg == "help" {
        help();
    } else if arg == "--check" || arg == "check" {
        Flags::Check
    } else {
        let Some(path) = args.get(PATH_INDEX) else {
            println!("missing path or name in arguments");
            exit(EXIT_FAILURE);
        };

        if arg == "--add" || arg == "add" {
            Flags::Add(path.to_string())
        } else if arg == "--rm" || arg == "rm" {
            Flags::Remove(path.to_string())
        } else if arg == "--mo" || arg == "mo" {
            Flags::MoveOut(path.to_string())
        } else {
            println!("incorrect argument");
            exit(EXIT_FAILURE);
        }
    }
}
