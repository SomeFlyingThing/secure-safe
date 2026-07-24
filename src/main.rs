use std::{env::args, io, process::exit};

use owo_colors::OwoColorize;
use zeroize::Zeroizing;

use crate::{
    safe::{EncError, Safe, check, move_out, remove},
    settings::Settings,
};

mod safe;
mod settings;

fn main() {
    let args = parse();

    let settings = Settings::load();

    match args {
        Flags::Add(name) => {
            let password = ask_password();
            let new = Safe::new(&name, password.as_bytes());
            new.store(&settings);
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
                        exit(0);
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

        {:<18} Encrypt and securely store a file
        {:<18} Permanently remove a stored file
                          {}
        {:<18} Decrypt and restore a stored file
        {:<18} Verify the integrity of the database
        {:<18} Display this help message

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

        exit(0);
    }
    let args: Vec<String> = args().collect();

    if args.is_empty() {
        println!("no args where provided");
        exit(0);
    }

    let arg = args.get(1).unwrap_or_else(|| {
        println!("no args provided");
        help();
    });

    if arg == "--help" || arg == "help" {
        help();
    } else if arg == "--check" || arg == "check" {
        Flags::Check
    } else {
        let Some(path) = args.get(2) else {
            println!("missing path or name in arguments");
            exit(1);
        };

        if arg == "--add" || arg == "add" {
            Flags::Add(path.to_string())
        } else if arg == "--rm" || arg == "rm" {
            Flags::Remove(path.to_string())
        } else if arg == "--mo" || arg == "mo" {
            Flags::MoveOut(path.to_string())
        } else {
            println!("incorrect argument");
            exit(1);
        }
    }
}
