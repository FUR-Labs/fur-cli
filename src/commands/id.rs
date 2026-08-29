//! Who this machine writes as: `fur id`.
//!
//! Attribution, not authentication. There is no password, no verification, and
//! no server-side identity — a person states who they are and the archive
//! records it honestly. Institutional identity is a deployment concern.
//!
//! Resolution order, first hit wins:
//!   1. `.fur/identity`                 — this project, this checkout
//!   2. `$XDG_CONFIG_HOME/fur/identity` — this user, every project
//!   3. `avatars.json` → `main`         — the pre-existing behaviour
//!
//! Rule 3 is why nothing breaks: a machine that never runs `fur id` resolves
//! exactly as it did before this command existed.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::*;
use serde_json::json;

use crate::avatars::{get_random_emoji_for_name, load_avatars, save_avatars};

#[derive(Parser, Debug)]
pub struct IdArgs {
    /// Avatar to write as. Omit to show the current identity.
    pub name: Option<String>,

    /// Apply to this project only, instead of this user
    #[arg(long)]
    pub project: bool,

    /// Remove the override
    #[arg(long)]
    pub clear: bool,
}

pub fn user_identity_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;

    Some(base.join("fur").join("identity"))
}

fn project_identity_path() -> PathBuf {
    Path::new(".fur").join("identity")
}

fn read_identity(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let name = text.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// The avatar this machine writes as, and where that came from.
pub fn current_identity() -> Option<(String, &'static str)> {
    if let Some(name) = read_identity(&project_identity_path()) {
        return Some((name, "proyecto (.fur/identity)"));
    }
    if let Some(name) = user_identity_path().as_deref().and_then(read_identity) {
        return Some((name, "usuario (config)"));
    }
    load_avatars()
        .get("main")
        .and_then(|v| v.as_str())
        .map(|name| (name.to_string(), "avatars.json (main)"))
}

pub fn run_id(args: IdArgs) {
    if args.clear {
        let path = if args.project {
            project_identity_path()
        } else {
            match user_identity_path() {
                Some(p) => p,
                None => return eprintln!("❌ No config directory available."),
            }
        };
        let _ = fs::remove_file(&path);
        println!("✔ Identidad eliminada ({})", path.display());
        return show();
    }

    let Some(name) = args.name else {
        return show();
    };

    // Create the avatar rather than failing: stating who you are should not
    // require a separate setup step.
    let mut avatars = load_avatars();
    if avatars.get(&name).is_none() {
        let emoji = get_random_emoji_for_name(&name);
        avatars[name.clone()] = json!(emoji);
        save_avatars(&avatars);
        println!("[OK] Avatar '{}' creado {}", name, emoji);
    }

    let path = if args.project {
        project_identity_path()
    } else {
        match user_identity_path() {
            Some(p) => p,
            None => return eprintln!("❌ No config directory available."),
        }
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(&path, format!("{}\n", name)) {
        Ok(_) => println!(
            "✔ Escribiendo como {} ({})",
            name.bright_yellow().bold(),
            path.display()
        ),
        Err(e) => eprintln!("❌ No se pudo escribir {}: {}", path.display(), e),
    }
}

fn show() {
    match current_identity() {
        Some((name, source)) => println!(
            "{} {}\n  {}",
            "Escribiendo como".bright_black(),
            name.bright_yellow().bold(),
            source.bright_black()
        ),
        None => println!("Sin identidad. Usa `fur id <nombre>`."),
    }
}