use std::fs;
use std::io::{self, Write};
use std::path::Path;

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

/// Read a password without echoing it.
///
/// viceroy: the old `read_visible_password` printed the passphrase in clear
/// text and left it in scrollback and terminal history. Hiding is now the
/// default rather than something `--hide` opts into.
pub fn read_password(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    match rpassword::read_password() {
        Ok(p) => p.trim().to_string(),
        Err(_) => {
            // No TTY (piped input, CI). Fall back to a plain line read so
            // scripts keep working, and say so rather than failing silently.
            eprintln!("⚠ No terminal available — password will not be hidden.");
            let mut buf = String::new();
            io::stdin().read_line(&mut buf).unwrap_or_default();
            buf.trim().to_string()
        }
    }
}

/// Read a password twice and require the two to match.
///
/// Only `lock` uses this. A typo while locking encrypts the archive under a
/// passphrase nobody knows; a typo while unlocking simply fails and can be
/// retried, so confirmation there would only be friction.
pub fn read_password_confirmed() -> Option<String> {
    let first = read_password("🔑 Enter password: ");

    if first.is_empty() {
        eprintln!("❌ Empty password. Aborting.");
        return None;
    }

    let second = read_password("🔑 Confirm password: ");

    if first != second {
        eprintln!("❌ Passwords do not match. Nothing was encrypted.");
        return None;
    }

    Some(first)
}