use std::fs;
use std::path::Path;

use clap::Parser;
use rpassword::read_password;

use crate::security::{io, state};

#[derive(Parser, Clone)]
pub struct LockArgs {
    #[arg(long)]
    pub hide: bool,
}

pub fn run_lock(args: LockArgs) {

    if state::is_locked() {
        println!("🔒 Project already locked.");
        return;
    }

    let pass = if args.hide {
        println!("🔑 Enter password:");
        read_password().unwrap()
    } else {
        io::read_visible_password()
    };

    create_lockcheck();

    encrypt_project(&pass);

    state::write_lock();

    println!("🔐 Project encrypted.");
}

fn create_lockcheck() {
    let path = Path::new(".fur/.lockcheck");
    fs::write(path, "FUR_LOCK_CHECK_V1").unwrap();
}

fn encrypt_project(password: &str) {

    encrypt_dir(".fur/messages", password);
    encrypt_dir(".fur/threads", password);

    encrypt_file(".fur/index.json", password);
    encrypt_file(".fur/.lockcheck", password);

    encrypt_markdowns(password);
}

fn encrypt_dir(dir: &str, password: &str) {

    let path = Path::new(dir);

    if let Ok(entries) = fs::read_dir(path) {

        for e in entries.flatten() {

            let p = e.path();

            if p.is_file() {
                io::encrypt_file(&p, password);
            }
        }
    }
}

fn encrypt_file(path: &str, password: &str) {

    let p = Path::new(path);

    if p.exists() {
        io::encrypt_file(p, password);
    }
}

fn encrypt_markdowns(password: &str) {

    let chats = Path::new("chats");

    if !chats.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(chats) {

        for e in entries.flatten() {

            let p = e.path();

            if p.is_file() {
                io::encrypt_file(&p, password);
            }
        }
    }
}