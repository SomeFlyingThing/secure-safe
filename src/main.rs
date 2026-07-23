use std::{env::args, io, mem::zeroed, ops::IndexMut, process::exit};
use zeroize::Zeroize;

mod safe;

fn main() {
    let args = parse();
}

enum Flags {
    Add(String),
    Remove(String),
}

struct Password(String);
impl Drop for Password {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl Password {
    pub const fn from(password: String) -> Password {
        Password(password)
    }
}

fn ask_password() ->Password{
    let mut password = String::new();

    println!("what is the password");
    io::stdin().read_line(&mut password).unwrap();

    password.zeroize();
    Password::from(password)



}
fn parse() -> Flags {
    let args: Vec<String> = args().collect();

    if args.is_empty() {
        println!("no args where provided");
        exit(0);
    }

    let arg = args.get(1).unwrap();

    let Some(path) = args.get(2) else {
        println!("missing path");
        exit(1);
    };

    let password = ask_password();
    
    if arg == "--add" {
        Flags::Add(args[2].to_string())
    } else if arg == "--rm" {
        Flags::Remove(args[2].to_string())
    } else {
        println!("incorrect argument");
        exit(0);
    }
}
