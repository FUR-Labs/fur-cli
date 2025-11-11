use colored::Colorize;
use std::process::Command;
use std::path::PathBuf;

pub fn run_git_status(repo_root: &PathBuf) {
    println!("{}", "─────────────────────────────".bright_black());
    println!(
        "{} {}",
        "Git status (repo root):".bright_cyan().bold(),
        repo_root.display()
    );

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("-c")
        .arg("color.ui=always")
        .arg("status")
        .status();

    if let Err(_) = output {
        println!("{}", "❌ Failed to execute git status".red());
    }
}

