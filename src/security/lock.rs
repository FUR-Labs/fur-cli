use std::fs;
use std::path::Path;

use clap::Parser;
use walkdir::WalkDir;

use crate::schema::rebuild::{LOCK_CHECK, LOCK_SENTINEL};
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

    // viceroy: `--hide` is now a no-op kept for compatibility; input is
    // always hidden, and lock always confirms.
    let _ = args.hide;

    let Some(pass) = io::read_password_confirmed() else {
        return;
    };

    create_lockcheck();
    create_sentinel(&pass);

    encrypt_project(&pass);

    state::write_lock();

    println!("🔐 Project encrypted.");
}

fn create_lockcheck() {
    let path = Path::new(".fur/.lockcheck");
    fs::write(path, "FUR_LOCK_CHECK_V1").unwrap();
}

/// Lock artifacts that live with the data they describe.
///
/// `.fur-locked` is plaintext, so a rebuild that finds only `chats/` knows the
/// documents are ciphertext and refuses to parse them. `.fur-lockcheck` is the
/// encrypted verifier, duplicated out of `.fur/` so a deleted index cannot
/// strand an encrypted archive.
fn create_sentinel(password: &str) {
    let chats = Path::new("chats");

    if !chats.exists() {
        return;
    }

    let _ = fs::write(chats.join(LOCK_SENTINEL), "FUR_LOCKED_V1\n");

    let check = chats.join(LOCK_CHECK);
    let _ = fs::write(&check, "FUR_LOCK_CHECK_V1");
    io::encrypt_file(&check, password);
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

/// viceroy: was a flat `read_dir` over `chats/`. Conversation documents now
/// live in `chats/<slug>/`, so a flat walk encrypted nothing and silently left
/// the whole archive in plaintext. Recurses now, skipping the sentinel.
fn encrypt_markdowns(password: &str) {
    let chats = Path::new("chats");

    if !chats.exists() {
        return;
    }

    for entry in WalkDir::new(chats)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();

        if !p.is_file() || is_sentinel(p) {
            continue;
        }

        io::encrypt_file(p, password);
    }
}

fn is_sentinel(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|f| f.to_str()),
        Some(LOCK_SENTINEL) | Some(LOCK_CHECK)
    )
}