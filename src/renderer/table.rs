use colored::*;

pub fn render_table(
    title: &str,
    headers: &[&str],
    rows: Vec<Vec<String>>,
    active_idx: Option<usize>,
) {
    // Compute column widths
    // Rust pads `{:width$}` by character count, so widths must be measured the
    // same way. Byte length silently over-pads any cell containing box-drawing
    // glyphs or accented text.
    let width_of = |s: &str| s.chars().count();

    let col_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let max_cell = rows
                .iter()
                .map(|r| r.get(i).map(|s| width_of(s)).unwrap_or(0))
                .max()
                .unwrap_or(0);
            max_cell.max(width_of(h))
        })
        .collect();

    // Total table width
    let total_width: usize = col_widths.iter().map(|w| w + 4).sum::<usize>() + 2;

    // Top title bar
    println!("{}", format!("=== {} ===", title).bold().bright_cyan());
    println!("{}", "-".repeat(total_width));

    // Header
    let mut header_line = String::new();
    for (i, h) in headers.iter().enumerate() {
        header_line.push_str(&format!("{:width$}    ", h, width = col_widths[i]));
    }
    println!("{}", header_line.bold());
    println!("{}", "=".repeat(total_width));

    // Rows. No blank line between them: in a nested listing the vertical gap
    // competes with the indentation for the reader's sense of structure.
    for (i, row) in rows.iter().enumerate() {
        let mut line = String::new();
        for (j, cell) in row.iter().enumerate() {
            line.push_str(&format!("{:width$}    ", cell, width = col_widths[j]));
        }

        if Some(i) == active_idx {
            println!("{}", line.bold().bright_yellow());
        } else {
            println!("{}", line);
        }
    }

    // Bottom border
    println!("{}", "-".repeat(total_width));
}
