//! Publish a canonical FUR conversation to a configured registry.

use std::path::Path;

use serde::Deserialize;

use crate::schema::snapshot::{build_publish_intent, PublishIntent};

#[derive(Debug, Deserialize)]
struct PublicationCreated {
    publication_id: String,
    revision_id: String,
    registry_id: String,
    snapshot_digest: String,
    published_at: String,
}

pub fn run_publish(conversation: Option<&str>, registry: &str) {
    match build_publish_intent(Path::new("."), conversation)
        .and_then(|intent| submit_publish_intent(registry, &intent))
    {
        Ok(created) => {
            println!("✔ Published conversation");
            println!("  Publication: {}", created.publication_id);
            println!("  Revision:    {}", created.revision_id);
            println!("  Registry:    {}", created.registry_id);
            println!("  Snapshot:    {}", created.snapshot_digest);
            println!("  Published:   {}", created.published_at);
        }
        Err(error) => eprintln!("❌ Registry publication failed: {}", error),
    }
}

fn submit_publish_intent(
    registry: &str,
    intent: &PublishIntent,
) -> Result<PublicationCreated, String> {
    let url = format!("{}/api/v2/publish", registry.trim_end_matches('/'));
    let response = reqwest::blocking::Client::new()
        .post(&url)
        .json(intent)
        .send()
        .map_err(|e| format!("cannot reach {}: {}", url, e))?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().unwrap_or_default();
        return Err(format!("registry returned HTTP {}: {}", status, detail));
    }

    response
        .json::<PublicationCreated>()
        .map_err(|e| format!("invalid publication receipt: {}", e))
}
