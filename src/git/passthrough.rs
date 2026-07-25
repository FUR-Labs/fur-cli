use crate::utils::git::find_git_root;
use colored::Colorize;
use std::process::Command;

pub fn passthrough(subcommand: &str, args: &[String]) {
    let repo_root = match find_git_root() {
        Some(r) => r,
        None => {
            eprintln!(
                "{}\n{}",
                "⚠️ No Git repository found.".yellow(),
                "   Run `git init` to enable Git commands in FUR.".bright_black()
            );
            return;
        }
    };

    let mut cmd = Command::new("git");

    // Attach to the actual terminal
    cmd.arg("-C").arg(repo_root);
    cmd.arg(subcommand);

    for a in args {
        cmd.arg(a);
    }

    // Inherit TTY streams; preserves ALL Git features
    let status = cmd.status();

    if let Err(_) = status {
        eprintln!("{}", "❌ Failed to run git command".red());
    }
}
