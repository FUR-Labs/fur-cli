use std::path::PathBuf;

// Finds the nearest .git directory (Git's real behavior)
pub fn find_git_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        if dir.join(".git").is_dir() {
            return Some(dir);
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

