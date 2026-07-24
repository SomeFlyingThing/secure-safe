use std::{
    env::args, io, mem::zeroed, ops::IndexMut, process::exit, thread::sleep, time::Duration,
};

use obfstr::obfcstr;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    safe::{Safe, check, move_out, remove},
    settings::Settings,
};

mod safe;
mod settings;

fn main() {
    let args = parse();

    let settings = Settings::load();
    let password = ask_password();

    match args {
        Flags::Add(name) => {
            let new = Safe::new(&name, &password);
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
            check(&settings);
        },
        Flags::MoveOut(name) => {
            move_out(&password, &settings, &name);
        },
    }
}

enum Flags {
    Add(String),
    MoveOut(String),
    Remove(String),
    Check,
}

const PASSWORD_SIZE: usize = 32;
fn ask_password() -> Zeroizing<[u8; 32]> {
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
            obfstr::obfstr!(
                "tip: if this is your first time using this program dont wory about not having a password (we know you dont have one set) just choose one and DONT forget it, it is SUPER important in the future !!!"
            )
        );

        if password.len() < PASSWORD_SIZE {
            return Zeroizing::new(password.as_bytes().try_into().unwrap());
        } else {
            println!("{}", obfstr::obfstr!("choose a smaller password"));
        }
    }
}

fn parse() -> Flags {
    let args: Vec<String> = args().collect();

    if args.is_empty() {
        println!("no args where provided");
        exit(0);
    }

    let arg = args.get(1).unwrap();

    let Some(path) = args.get(2) else {
        println!("missing path or name in arguments");
        exit(1);
    };

    let password = ask_password();

    if arg == "--add" || arg == "add" {
        Flags::Add(args[2].to_string())
    } else if arg == "--rm" || arg == "rm" {
        Flags::Remove(args[2].to_string())
    } else if arg == "--check" || arg == "check" {
        Flags::Check
    } else if arg == "--mo" || arg == "mo" {
        Flags::MoveOut(args[2].to_string())
    } else {
        println!("incorrect argument");
        exit(0);
    }
}
