mod core;

pub use core::upgrade_message_schema;

use clap::Parser;
use serde_json::Value;
use self::core::*;

#[derive(Parser, Debug)]
pub struct JotArgs {
    /// Optional avatar name (defaults to 'main' if omitted)
    #[arg(index = 1)]
    pub avatar: Option<String>,

    /// Optional jot text
    #[arg(index = 2)]
    pub positional_text: Option<String>,

    /// Jot text (takes precedence over positional)
    #[arg(long)]
    pub text: Option<String>,

    /// Attach markdown file
    #[arg(long, alias = "file")]
    pub markdown: Option<String>,

    /// Attach image (PNG, JPG, etc.)
    #[arg(long)]
    pub img: Option<String>,

    /// Parent message ID (optional, for replies)
    #[arg(long)]
    pub parent: Option<String>,
}


pub fn run_jot(args: JotArgs) {

    let ctx = match load_context() {
        Ok(c) => c,
        Err(e) => return eprintln!("{}", e),
    };

    let (avatar, text_opt) = resolve_avatar_and_text(&ctx.avatars, &args);

    if let Err(e) = validate_inputs(&text_opt, &args.markdown) {
        return eprintln!("{}", e);
    }

    let msg = build_message(
        &avatar,
        text_opt.clone(),
        args.markdown.clone(),
        args.img.clone(),
        args.parent.clone(),
    );
    let msg_id = msg["id"].as_str().unwrap().to_string();

    apply_jot_effects(
        &ctx,
        &msg,
        &msg_id,
        args.parent.as_deref(),
        &avatar
    );
}


pub fn apply_jot_effects(
    ctx: &FurContext,
    msg: &Value,
    msg_id: &str,
    parent: Option<&str>,
    avatar: &str
) {
    save_message(&ctx.fur_dir, msg_id, msg);
    update_conversation(ctx, msg_id, parent);
    update_index(&ctx.fur_dir, msg_id);
    print_confirmation(&ctx.avatars, avatar, msg_id, &ctx.conversation_id);
}
