use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use serde_json::Value;

use crate::commands::timeline::TimelineArgs;
use crate::renderer::utils::load_message;

/// LaTeX preamble with fixes for Pandoc output + math
fn latex_preamble(thread_title: &str) -> String {
    format!(
r#"\documentclass[12pt]{{article}}
\usepackage[margin=1in]{{geometry}}
\usepackage{{parskip}}
\usepackage{{xcolor}}
\usepackage{{titlesec}}
\usepackage{{amsmath}}       % better math
\usepackage{{hyperref}}
\usepackage[T1]{{fontenc}}
\usepackage[utf8]{{inputenc}}
\usepackage{{lmodern}}

% Pandoc fix for lists
\providecommand{{\tightlist}}{{%
  \setlength{{\itemsep}}{{0pt}}\setlength{{\parskip}}{{0pt}}}}

% Message macro
\newcommand{{\MessageBlock}}[3]{{
  \vspace{{1em}}
  \noindent\textbf{{#1}} \hfill \textit{{#2}} \\
  #3
  \par
}}

\begin{{document}}
\begin{{center}}
    \LARGE\bfseries {title} \\
    \rule{{\linewidth}}{{0.4pt}}
\end{{center}}
"#,
        title = thread_title
    )
}

/// Document ending
fn latex_end() -> &'static str {
    r#"\end{document}"#
}

/// Strip emojis and other non-ASCII that break pdflatex
fn strip_emojis(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii() || c.is_alphanumeric() || c.is_whitespace()).collect()
}

/// Render a single message (recursively) into LaTeX
pub fn render_message_tex(
    fur_dir: &Path,
    msg_id: &str,
    label: String,        // e.g. "Root", "Root - Branch 1"
    args: &TimelineArgs,
    avatars: &Value,
    tex_out: &mut File,
    depth: usize,         // branch depth
) {
    let Some(msg) = load_message(fur_dir, msg_id, avatars) else { return };

    // Escape LaTeX special characters
    let escape = |s: &str| {
            s.replace("&", "\\&")
            .replace("%", "\\%")
            .replace("$", "\\$")
            .replace("#", "\\#")
            .replace("_", "\\_")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("~", "\\textasciitilde{}")
            .replace("^", "\\textasciicircum{}")
            .replace("\n", " \\\\\n")
    };


    // Handle message content safely
    let base_content = if args.verbose || args.contents {
        if let Some(path_str) = msg.markdown.clone() {
            let mut out = String::new();

            // Always show text if it's non-empty
            if !msg.text.trim().is_empty() {
                out += &format!("{}\n\n", escape(&strip_emojis(&msg.text)));
            }

            // Try to render markdown as LaTeX
            match Command::new("pandoc")
                .args(&["-f", "markdown", "-t", "latex", &path_str])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let latex_body = String::from_utf8_lossy(&output.stdout);
                    out += &format!(
                        "Attached document:\n\n\\begin{{quote}}\n{}\n\\end{{quote}}\n\\clearpage",
                        strip_emojis(&latex_body)
                    );
                }
                _ => {
                    // Fallback to raw contents if Pandoc fails
                    let fallback = fs::read_to_string(path_str)
                        .map(|s| escape(&strip_emojis(&s)))
                        .unwrap_or_else(|_| String::from("[Markdown file missing]"));
                    out += &format!("{}\n\\clearpage", fallback);
                }
            }

            out
        } else {
            escape(&strip_emojis(&msg.text))
        }
    } else {
        escape(&strip_emojis(&msg.text))
    };

    // Instead of indentation, show branch label explicitly
    writeln!(
        tex_out,
        "\\MessageBlock{{{}}}{{{} {} - {}}}{{{}}}",
        escape(&msg.name),
        msg.date_str,
        msg.time_str,
        label,
        base_content
    )
    .unwrap();

    // ✅ Recurse branch-aware
    for (bi, block) in msg.branches.iter().enumerate() {
        let branch_label = format!("{} - Branch {}", label, bi + 1);

        for cid in block {
            render_message_tex(fur_dir, cid, branch_label.clone(), args, avatars, tex_out, depth + 1);
        }
    }
}


/// Export a full thread to LaTeX and compile to PDF
pub fn export_to_pdf(
    fur_dir: &Path,
    thread_title: &str,
    root_msgs: &[Value],
    args: &TimelineArgs,
    avatars: &Value,
    out_path: &str,
) {
    let tex_file = out_path.replace(".pdf", ".tex");
    let mut file = File::create(&tex_file).expect("❌ Failed to create .tex file");

    // Write preamble
    file.write_all(latex_preamble(thread_title).as_bytes()).unwrap();

    // Write messages
    for mid in root_msgs {
        if let Some(mid_str) = mid.as_str() {
            render_message_tex(fur_dir, mid_str, "Root".to_string(), args, avatars, &mut file, 0);
        }
    }

    // End document
    file.write_all(latex_end().as_bytes()).unwrap();

    // Compile with pdflatex
    Command::new("pdflatex")
        .arg("-interaction=nonstopmode")
        .arg(&tex_file)
        .status()
        .expect("❌ Failed to run pdflatex");

    println!("✔️ Exported LaTeX to {}", out_path);
}
