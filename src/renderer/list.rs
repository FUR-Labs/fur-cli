use colored::*;

/// Render a section header with rows aligned like a clean log.
pub fn render_list(title: &str, headers: &[&str], rows: Vec<Vec<String>>, active_idx: Option<usize>) {
    // Header
    println!("{}", format!("=== {} ===", title).bold().bright_cyan());
    println!("{}", "-".repeat(28));

    // Column headers
    let header_line = headers.join("    ");
    println!("{}", header_line.bold());

    println!("{}", "=".repeat(28));

    // Compute column widths for alignment
    let col_widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            rows.iter()
                .map(|r| r.get(i).map(|s| s.len()).unwrap_or(0))
                .max()
                .unwrap_or(0)
        })
        .collect();

    // Print each row
    for (i, row) in rows.iter().enumerate() {
        let mut line = String::new();
        for (j, cell) in row.iter().enumerate() {
            let width = col_widths[j].max(headers[j].len());
            let padded = format!("{:width$}", cell, width = width + 2);
            line.push_str(&padded);
        }
        if Some(i) == active_idx {
            println!("{}", line.bold().bright_yellow());
        } else {
            println!("{}", line);
        }
        println!(); // blank line for spacing
    }

    println!("{}", "-".repeat(28));
}
