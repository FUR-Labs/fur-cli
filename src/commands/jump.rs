use clap::Args;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// JumpArgs allow specifying relative or absolute jumps
#[derive(Args, Debug)]
pub struct JumpArgs {
    #[arg(short, long)]
    pub past: Option<usize>,

    #[arg(short, long)]
    pub child: Option<usize>,

    #[arg(short, long)]
    pub id: Option<String>,
}

pub fn run_jump(args: JumpArgs) -> Result<(), Box<dyn std::error::Error>> {
    let index_path = Path::new(".fur/index.json");
    let index_data = fs::read_to_string(index_path).expect("❌ Couldn't read index.json");
    let mut index: Value = serde_json::from_str(&index_data).unwrap();

    let conversation_id = index["active_thread"].as_str().unwrap();
    let convo_path = Path::new(".fur/threads").join(format!("{}.json", conversation_id));
    let conversation_data =
        fs::read_to_string(convo_path).expect("❌ Couldn't read conversation file");
    let conversation: Value = serde_json::from_str(&conversation_data).unwrap();

    let current_id = index["current_message"].as_str().unwrap_or_default();

    let messages = conversation["messages"].as_array().unwrap();

    // Locate current message
    let current_msg_id = current_id;
    let current_msg = messages.iter().find_map(|id| {
        if id.as_str()? == current_msg_id {
            let msg_path = Path::new(".fur/messages").join(format!("{}.json", current_msg_id));
            let msg_data = fs::read_to_string(msg_path).ok()?;
            serde_json::from_str::<Value>(&msg_data).ok()
        } else {
            None
        }
    });

    if current_msg.is_none() {
        eprintln!("❌ Current message not found in conversation.");
        return Ok(());
    }
    let current = current_msg.unwrap();

    // Handle jump --past
    if let Some(n) = args.past {
        let mut current_id = current["id"].as_str().unwrap_or_default().to_string();
        let mut jumped = 0;

        while jumped < n {
            // println!("🔍 Loop iteration {} — current_id: {}", jumped, current_id);

            let msg_path = Path::new(".fur/messages").join(format!("{}.json", current_id));
            let msg_data = fs::read_to_string(&msg_path);
            if msg_data.is_err() {
                eprintln!("❌ Failed to load message: {}", current_id);
                return Ok(());
            }

            let msg_json: Value = serde_json::from_str(&msg_data.unwrap()).unwrap();

            // let parent_raw = &msg_json["parent"];
            // println!("   ↪️ parent field raw: {}", parent_raw);

            match msg_json["parent"].as_str() {
                Some(pid) if !pid.is_empty() => {
                    let in_conversation = conversation["messages"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .any(|val| val.as_str() == Some(pid));

                    if !in_conversation {
                        println!("\x1b[91m📜 You've reached the origin of this conversation. No earlier messages exist.\x1b[0m");
                        println!("\x1b[93m🌱 To start a new conversation, run:\n    fur new \"Title of your new conversation\"\x1b[0m");
                        return Ok(());
                    }

                    // println!("   🧬 Jumping to parent_id: {}", pid);
                    current_id = pid.to_string();
                    jumped += 1;
                }
                _ => {
                    println!("\x1b[91m📜 You've reached the origin of this conversation. No earlier messages exist.\x1b[0m");
                    println!("\x1b[93m🌱 To start a new conversation, run:\n    fur new \"Title of your new conversation\"\x1b[0m");
                    return Ok(());
                }
            }
        }

        index["current_message"] = Value::String(current_id.clone());
        fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
        println!(
            "⏪ Jumped back {} message{} to {}",
            jumped,
            if jumped == 1 { "" } else { "s" },
            current_id
        );
        return Ok(());
    }

    // Handle jump --child
    if let Some(n) = args.child {
        let current_id = current["id"].as_str().unwrap_or_default();

        let children: Vec<String> = messages
            .iter()
            .filter_map(|msg_id| {
                let msg_path =
                    Path::new(".fur/messages").join(format!("{}.json", msg_id.as_str()?));
                let msg_data = fs::read_to_string(msg_path).ok()?;
                let msg_json: Value = serde_json::from_str(&msg_data).ok()?;
                if msg_json["parent"].as_str()? == current_id {
                    Some(msg_json["id"].as_str()?.to_string())
                } else {
                    None
                }
            })
            .collect();

        if let Some(child_id) = children.get(n) {
            index["current_message"] = Value::String(child_id.to_string());
            fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
            println!("⏩ Jumped to child [{}]: {}", n, child_id);
            return Ok(());
        } else {
            eprintln!("❌ No such child at index {}", n);
            return Ok(());
        }
    }

    // Handle jump --id
    if let Some(ref target_id) = args.id {
        if messages
            .iter()
            .any(|m| m["id"].as_str() == Some(&target_id))
        {
            index["current_message"] = Value::String(target_id.to_string());
            fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
            println!("🎯 Jumped directly to message ID {}", target_id);
            return Ok(());
        } else {
            eprintln!("❌ Message ID not found: {}", target_id);
            return Ok(());
        }
    }

    eprintln!("❗ No jump argument provided. Use --past, --child, or --id.");
    Ok(())
}
