use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clap::Parser;
use colored::*;
use serde_json::{json, Value};

use crate::avatars::emojis::pick_emoji;
use crate::avatars::{get_random_emoji_for_name, PLACEHOLDER};
use crate::commands::utils::input::{ask_raw, ask_yes_no, default_yes};
use crate::renderer::table::render_table;

#[derive(Parser, Debug)]
pub struct OnboardArgs {
    /// Run even when identities are already set
    #[arg(long)]
    pub force: bool,
}

/// `fur onboard` — fill in what the documents cannot carry.
///
/// Documents record avatar *names*; the emoji mapping and the `main` pointer
/// are reader preferences that live only in `.fur/avatars.json`. After a
/// rebuild every avatar is a placeholder until this runs.
///
/// Two questions, ordered by whether FUR can answer them itself: `main` is
/// unanswerable and changes behaviour, so it goes first; emoji is answerable
/// and only affects display, so it is offered second and optional.
pub fn run_onboard(args: OnboardArgs) {
    let fur_dir = Path::new(".fur");
    let avatars_path = fur_dir.join("avatars.json");

    if !fur_dir.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let mut avatars: Value = fs::read_to_string(&avatars_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| json!({}));

    let counts: BTreeMap<String, usize> = crate::commands::avatar::count_messages_per_avatar()
        .into_iter()
        .collect();

    if counts.is_empty() {
        println!("📭 No messages yet — nothing to set up.");
        return;
    }

    if !args.force && !needs_onboarding(&avatars, &counts) {
        println!("✔ Identities already set. Use --force to change them.");
        return;
    }

    println!("\n{}", "== Who's who ==".bright_magenta().bold());
    println!(
        "{}",
        "These names came from your conversations. FUR needs to know which one is you,\n\
         because it's the voice used whenever you jot without naming an avatar."
            .bright_cyan()
    );

    show_avatar_table(&avatars, &counts);

    if let Some(main) = ask_main(&counts, current_main(&avatars)) {
        avatars["main"] = json!(main);
        println!("{}", format!("[OK] You are: {}", main).bright_green().bold());
    } else {
        println!("{}", "Skipped — keeping the current guess.".bright_black());
    }

    apply_emoji(&mut avatars, &counts);

    fs::write(
        &avatars_path,
        serde_json::to_string_pretty(&avatars).unwrap(),
    )
    .expect("❌ Could not write avatars.json");

    println!("\n✔ Saved. Change anything later with `fur onboard --force`.");
}

/// True when `main` is missing, points at nobody, or any avatar is still a
/// placeholder.
pub fn needs_onboarding(avatars: &Value, counts: &BTreeMap<String, usize>) -> bool {
    let main_ok = avatars["main"]
        .as_str()
        .map(|m| counts.contains_key(m))
        .unwrap_or(false);

    if !main_ok {
        return true;
    }

    counts.keys().any(|name| {
        avatars[name.as_str()]
            .as_str()
            .map(|e| e == PLACEHOLDER)
            .unwrap_or(true)
    })
}

fn current_main(avatars: &Value) -> Option<String> {
    avatars["main"].as_str().map(|s| s.to_string())
}

fn show_avatar_table(avatars: &Value, counts: &BTreeMap<String, usize>) {
    let main = current_main(avatars);

    let mut rows = Vec::new();
    let mut active_idx = None;

    for (i, (name, count)) in counts.iter().enumerate() {
        let emoji = avatars[name.as_str()]
            .as_str()
            .unwrap_or(PLACEHOLDER)
            .to_string();

        rows.push(vec![
            format!("{}", i + 1),
            name.clone(),
            emoji,
            count.to_string(),
        ]);

        if Some(name.clone()) == main {
            active_idx = Some(i);
        }
    }

    render_table(
        "Avatars",
        &["#", "Name", "Emoji", "Messages"],
        rows,
        active_idx,
    );
}

/// Pick from the scanned list rather than free-typing, so a typo cannot create
/// a phantom avatar with no messages while the real one stays unclaimed.
fn ask_main(counts: &BTreeMap<String, usize>, current: Option<String>) -> Option<String> {
    let names: Vec<&String> = counts.keys().collect();

    let default_hint = current
        .as_deref()
        .map(|c| format!(" [{}]", c))
        .unwrap_or_default();

    let answer = ask_raw(&format!(
        "Which number is you?{} (Enter to skip): ",
        default_hint
    ));

    if answer.is_empty() {
        return None;
    }

    if let Ok(idx) = answer.parse::<usize>() {
        if idx >= 1 && idx <= names.len() {
            return Some(names[idx - 1].clone());
        }
    }

    if let Some(hit) = names.iter().find(|n| n.as_str() == answer) {
        return Some((*hit).clone());
    }

    println!("{}", "Not one of the listed avatars — skipping.".yellow());
    None
}

/// Suggest a face per avatar, then let the user accept the lot or walk through
/// the real emoji library one at a time.
fn apply_emoji(avatars: &mut Value, counts: &BTreeMap<String, usize>) {
    println!("\n{}", "== Faces ==".bright_magenta().bold());

    let suggestions: Vec<(String, String)> = counts
        .keys()
        .map(|name| {
            let existing = avatars[name.as_str()].as_str().unwrap_or(PLACEHOLDER);

            // Keep a face the user already chose; only fill placeholders.
            let emoji = if existing == PLACEHOLDER {
                get_random_emoji_for_name(name)
            } else {
                existing.to_string()
            };

            (name.clone(), emoji)
        })
        .collect();

    let rows = suggestions
        .iter()
        .map(|(name, emoji)| vec![name.clone(), emoji.clone()])
        .collect();

    render_table("Suggested", &["Name", "Emoji"], rows, None);

    if ask_yes_no("Keep these? [Y/n]: ", default_yes) {
        for (name, emoji) in suggestions {
            avatars[name] = json!(emoji);
        }
        return;
    }

    // viceroy: onboarding used to offer a small hardcoded emoji table. It now
    // opens the same library picker as `fur avatar new`, with the suggestion as
    // the Enter-to-accept default.
    for (name, suggested) in suggestions {
        println!(
            "\n{}",
            format!("Emoji for '{}' — Enter to keep {}", name, suggested)
                .bright_cyan()
                .bold()
        );

        let chosen = pick_emoji("Your choice: ", Some(&suggested));
        avatars[name] = json!(chosen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_placeholders() {
        let avatars = json!({ "main": "andrew", "andrew": "🐾", "gpt5": "🐾" });
        let counts = BTreeMap::from([("andrew".to_string(), 2), ("gpt5".to_string(), 1)]);
        assert!(needs_onboarding(&avatars, &counts));
    }

    #[test]
    fn accepts_a_finished_setup() {
        let avatars = json!({ "main": "andrew", "andrew": "🦊", "gpt5": "🤖" });
        let counts = BTreeMap::from([("andrew".to_string(), 2), ("gpt5".to_string(), 1)]);
        assert!(!needs_onboarding(&avatars, &counts));
    }

    #[test]
    fn main_pointing_at_a_ghost_needs_fixing() {
        let avatars = json!({ "main": "nobody", "andrew": "🦊" });
        let counts = BTreeMap::from([("andrew".to_string(), 2)]);
        assert!(needs_onboarding(&avatars, &counts));
    }
}