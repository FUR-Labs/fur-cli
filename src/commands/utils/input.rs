use std::io::{self, Write};

pub fn ask_string(prompt: &str, default: Option<&str>) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    let s = buf.trim();
    if s.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        s.to_string()
    }
}

pub fn ask_raw(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

pub fn default_yes(s: &str) -> bool {
    s.is_empty() || s == "y" || s == "yes"
}

pub fn ask_yes_no(prompt: &str, parser: fn(&str) -> bool) -> bool {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    parser(buf.trim().to_lowercase().as_str())
}
