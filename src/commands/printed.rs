use crate::commands::provenance::{self, Options, Scope};
use crate::commands::timeline::{run_timeline, TimelineArgs};
use colored::*;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// `fur printed` — exports the active conversation to Markdown or PDF.
///
/// With `--provenance`, exports the conversation together with the lineage it
/// draws on, as one Markdown document.
pub fn run_printed(
    out: Option<String>,
    verbose: bool,
    provenance_scope: Option<Scope>,
    id: Option<String>,
) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    let target = match resolve_target(&index, id.as_deref()) {
        Ok(t) => t,
        Err(e) => return eprintln!("❌ {}", e),
    };

    if let Some(scope) = provenance_scope {
        return run_provenance(fur_dir, &target, scope, out, verbose);
    }

    run_single(fur_dir, &target, out, verbose);
}

/// Resolve an explicit id or prefix, falling back to the active conversation.
fn resolve_target(index: &Value, requested: Option<&str>) -> Result<String, String> {
    let threads: Vec<String> = index["threads"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let Some(needle) = requested else {
        return index["active_thread"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| "No active conversation found.".to_string());
    };

    if threads.iter().any(|t| t == needle) {
        return Ok(needle.to_string());
    }

    let matches: Vec<&String> = threads.iter().filter(|t| t.starts_with(needle)).collect();

    match matches.as_slice() {
        [] => Err(format!("No conversation matches '{}'", needle)),
        [single] => Ok((*single).clone()),
        _ => Err(format!("Ambiguous prefix '{}'", needle)),
    }
}

/// The original behaviour: one conversation, Markdown or PDF.
fn run_single(fur_dir: &Path, target: &str, out: Option<String>, verbose: bool) {
    let convo_path = fur_dir.join("threads").join(format!("{}.json", target));
    let conversation_json: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let title = conversation_json["title"].as_str().unwrap_or("untitled");

    let out_path = out.unwrap_or_else(|| default_name(title, target, "md"));
    let is_pdf = out_path.to_lowercase().ends_with(".pdf");

    let args = TimelineArgs {
        verbose,
        contents: true,
        out: Some(out_path.clone()),
        conversation_override: Some(target.to_string()),
    };

    println!(
        "{}",
        format!("🖨️  Printing conversation: {} ({}) → {}", title, target, out_path)
            .bright_green()
            .bold()
    );

    run_timeline(args);

    println!(
        "{}",
        if is_pdf {
            format!("✔️  Exported PDF: {}", out_path)
        } else {
            format!("✔️  Exported Markdown: {}", out_path)
        }
        .bright_green()
        .bold()
    );
}

/// The conversation plus its lineage, as one Markdown document.
fn run_provenance(
    fur_dir: &Path,
    target: &str,
    scope: Scope,
    out: Option<String>,
    verbose: bool,
) {
    let options = Options {
        scope,
        contents: verbose,
    };

    let (body, count) = match provenance::render(fur_dir, target, &options) {
        Ok(result) => result,
        Err(e) => return eprintln!("❌ {}", e),
    };

    let convo_path = fur_dir.join("threads").join(format!("{}.json", target));
    let title = fs::read_to_string(&convo_path)
        .ok()
        .and_then(|c| serde_json::from_str::<Value>(&c).ok())
        .and_then(|v| v["title"].as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "untitled".to_string());

    let out_path = out.unwrap_or_else(|| default_name(&format!("{}_PROVENANCE", title), target, "md"));

    if out_path.to_lowercase().ends_with(".pdf") {
        eprintln!("❌ Provenance records are Markdown only.");
        return;
    }

    if let Err(e) = fs::write(&out_path, body) {
        return eprintln!("❌ Could not write {}: {}", out_path, e);
    }

    println!(
        "{}",
        format!(
            "🧬 Provenance record: {} conversation(s) → {}",
            count, out_path
        )
        .bright_green()
        .bold()
    );
}

/// ALLCAPS_TITLE_SHORTID.ext at the project root — a printed record, not part
/// of the archive.
fn default_name(title: &str, id: &str, ext: &str) -> String {
    let all_caps = title.to_uppercase().replace(' ', "_");
    let short_id = id.split('-').next().unwrap_or(id);
    format!("{}_{}.{}", all_caps, short_id, ext)
}