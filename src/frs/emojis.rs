use emojis;

/// Show a preview of the emoji library (first N entries).
pub fn preview_emojis(count: usize) {
    println!("== Emoji Library Preview ==");
    for (i, emoji) in emojis::iter().take(count).enumerate() {
        print!("#{:<3} {:<2}  ", i, emoji);
        if i % 10 == 9 {
            println!();
        }
    }
    println!("\n(Type hash to select, or type a keyword to filter)");
}

/// Search for emojis by keyword in name or shortcodes.
pub fn search_emojis(keyword: &str) -> Vec<&'static emojis::Emoji> {
    let kw = keyword.to_lowercase();
    emojis::iter()
        .filter(|e| {
            e.name().contains(&kw)
                || e.shortcodes().any(|s| s.contains(&kw))
        })
        .collect()
}
