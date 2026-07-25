use regex::Regex;

pub fn normalize_tag(raw: &str) -> String {
    let s = raw.trim().to_lowercase();
    let re = Regex::new(r"[^a-z0-9]+").unwrap();
    let cleaned = re.replace_all(&s, "-");
    cleaned.trim_matches('-').to_string()
}

pub fn parse_tag_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|t| normalize_tag(t))
        .filter(|t| !t.is_empty())
        .collect()
}
