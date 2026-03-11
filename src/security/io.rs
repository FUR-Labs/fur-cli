use std::fs;
use std::path::Path;
use std::io::{self, Write};

use crate::security::crypto;
use crate::security::state;

pub fn encrypt_file(path: &Path, password: &str) {

    if let Ok(bytes) = fs::read(path) {

        if let Ok(enc) = crypto::encrypt(&bytes, password) {

            let _ = fs::write(path, enc);
        }
    }
}

pub fn decrypt_file(path: &Path, password: &str) {

    if let Ok(bytes) = fs::read(path) {

        if let Ok(dec) = crypto::decrypt(&bytes, password) {

            let _ = fs::write(path, dec);
        }
    }
}

pub fn read_text_file(path: &Path) -> Option<String> {

    if state::is_locked() {
        eprintln!("🔒 Project locked. Run `fur unlock`.");
        return None;
    }

    fs::read_to_string(path).ok()
}

pub fn read_visible_password() -> String {

    print!("🔑 Enter password: ");
    io::stdout().flush().unwrap();

    let mut pass = String::new();
    io::stdin().read_line(&mut pass).unwrap();

    pass.trim().to_string()
}