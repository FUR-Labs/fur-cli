use std::fs;
use std::path::Path;

use clap::Parser;
use walkdir::WalkDir;

use crate::schema::rebuild::{LOCK_CHECK, LOCK_SENTINEL};
use crate::security::{crypto, io, state};

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

    let _ = args.hide;

    let pass = io::read_password("🔑 Enter password: ");

    if !verify_password(&pass) {
        println!("❌ Incorrect password. Project remains locked.");
        return;
    }

    decrypt_project(&pass);

    remove_sentinel();
    state::remove_lock();

    println!("🔓 Project decrypted.");
}

/// Verify in order of durability: the checker beside the data, then the one in
/// `.fur/`, then — if both are gone — a trial decryption of a real document.
///
/// viceroy: the trial path is the recovery case. An archive locked before this
/// fix has both checkers only inside `.fur/`, so deleting the index left the
/// password unverifiable and the data unreachable.
fn verify_password(password: &str) -> bool {
    for check_path in [
        Path::new("chats").join(LOCK_CHECK),
        Path::new(".fur/.lockcheck").to_path_buf(),
    ] {
        if let Ok(bytes) = fs::read(&check_path) {
            if let Ok(decrypted) = crypto::decrypt(&bytes, password) {
                if std::str::from_utf8(&decrypted)
                    .map(|t| t.trim() == "FUR_LOCK_CHECK_V1")
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            return false;
        }
    }

    trial_decrypt(password)
}

/// Last resort: if any encrypted document decrypts cleanly, the password is
/// right. Nothing is written here — this only answers yes or no.
fn trial_decrypt(password: &str) -> bool {
    for entry in WalkDir::new("chats")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();

        if !p.is_file() || is_sentinel(p) {
            continue;
        }

        if let Ok(bytes) = fs::read(p) {
            if crypto::decrypt(&bytes, password).is_ok() {
                return true;
            }
        }
    }

    eprintln!("⚠ No verifiable encrypted content found under chats/.");
    false
}

/// Removed only after decryption succeeds, so an interrupted unlock still
/// leaves the archive correctly marked as locked.
fn remove_sentinel() {
    for name in [LOCK_SENTINEL, LOCK_CHECK] {
        let path = Path::new("chats").join(name);

        if path.exists() {
            let _ = fs::remove_file(path);
        }
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

/// viceroy: mirrors the recursion fix in `lock.rs` — a flat walk would leave
/// `chats/<slug>/` documents encrypted forever.
fn decrypt_markdowns(password: &str) {
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

        io::decrypt_file(p, password);
    }
}

fn is_sentinel(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|f| f.to_str()),
        Some(LOCK_SENTINEL) | Some(LOCK_CHECK)
    )
}