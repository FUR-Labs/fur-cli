use std::fs;
use std::path::Path;

use clap::Parser;
use rpassword::read_password;

use crate::security::{io, crypto, state};

#[derive(Parser, Clone)]
pub struct UnlockArgs {
    #[arg(long)]
    pub hide: bool,
}

pub fn run_unlock(args: UnlockArgs) {

    if !state::is_locked() {
        println!("🔓 Project already unlocked.");
        return;
    }

    let pass = if args.hide {
        println!("🔑 Enter password:");
        read_password().unwrap()
    } else {
        io::read_visible_password()
    };

    if !verify_password(&pass) {
        println!("❌ Incorrect password. Project remains locked.");
        return;
    }

    decrypt_project(&pass);

    state::remove_lock();

    println!("🔓 Project decrypted.");
}

fn verify_password(password: &str) -> bool {

    let check_path = Path::new(".fur/.lockcheck");

    let bytes = match std::fs::read(check_path) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let decrypted = match crypto::decrypt(&bytes, password) {
        Ok(d) => d,
        Err(_) => return false,
    };

    match std::str::from_utf8(&decrypted) {
        Ok(text) => text.trim() == "FUR_LOCK_CHECK_V1",
        Err(_) => false,
    }
}

fn decrypt_project(password: &str) {

    decrypt_dir(".fur/messages", password);
    decrypt_dir(".fur/threads", password);

    decrypt_file(".fur/index.json", password);
    decrypt_file(".fur/.lockcheck", password);

    decrypt_markdowns(password);
}

fn decrypt_dir(dir: &str, password: &str) {

    let path = Path::new(dir);

    if let Ok(entries) = fs::read_dir(path) {

        for e in entries.flatten() {

            let p = e.path();

            if p.is_file() {
                io::decrypt_file(&p, password);
            }
        }
    }
}

fn decrypt_file(path: &str, password: &str) {

    let p = Path::new(path);

    if p.exists() {
        io::decrypt_file(p, password);
    }
}

fn decrypt_markdowns(password: &str) {

    let chats = Path::new("chats");

    if !chats.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(chats) {

        for e in entries.flatten() {

            let p = e.path();

            if p.is_file() {
                io::decrypt_file(&p, password);
            }
        }
    }
}