//! Emoji browsing and selection, shared by `fur avatar new` and `fur onboard`.

use crate::commands::utils::input::ask_raw;

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
        .filter(|e| e.name().contains(&kw) || e.shortcodes().any(|s| s.contains(&kw)))
        .collect()
}

/// Interactive picker: numeric index into the library, or a keyword search.
///
/// viceroy: this was `choose_emoji` inside `commands/avatar.rs`, private and
/// unreachable from onboarding, which is why onboarding shipped with a
/// hardcoded table instead. One picker now, used by both.
///
/// `default` is returned when the user presses Enter, so an onboarding flow can
/// offer a suggestion without forcing a choice. Pass `None` to require one.
pub fn pick_emoji(prompt: &str, default: Option<&str>) -> String {
    preview_emojis(50);

    loop {
        let input = ask_raw(prompt);

        if input.is_empty() {
            if let Some(d) = default {
                return d.to_string();
            }
            println!("Please choose an emoji.");
            continue;
        }

        // A pasted emoji is a valid answer — don't force the menu.
        if emojis::get(&input).is_some() {
            return input;
        }

        // numeric index from global list
        if let Ok(idx) = input.parse::<usize>() {
            if let Some(e) = emojis::iter().nth(idx) {
                return e.to_string();
            }
            println!("Index out of range.");
            continue;
        }

        // keyword search
        let matches = search_emojis(&input);
        if matches.is_empty() {
            println!("No matches for '{}'. Try again.", input);
            continue;
        }

        println!("Matches for '{}':", input);
        for (i, emoji) in matches.iter().enumerate().take(60) {
            println!("#{:<2} {:<2}  — {}", i, emoji, emoji.name());
        }

        let pick = ask_raw("Pick a hash index from these results: ");
        if let Ok(i) = pick.parse::<usize>() {
            if let Some(e) = matches.get(i) {
                return e.to_string();
            }
        }

        println!("Invalid choice, looping again.");
    }
}