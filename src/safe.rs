use std::{ffi::OsString, fs::File, io::Read};
use chacha20poly1305::{ChaChaPoly1305, KeyInit};
use argon2::password_hash::{Salt, SaltString};



fn read_file(path: &str) -> Vec<u8> {
    let mut file = File::open(path).expect("couldnt read the file check the path");

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();

    bytes
}

struct Safe {
    salt: SaltString,
    contents: Vec<u8>,
    name: String,
    path: String,
}


impl Safe {
    fn new(){
        
        let salt  = SaltString::generate(&mut Osrng);

        
    }
}