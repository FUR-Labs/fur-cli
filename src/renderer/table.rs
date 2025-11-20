use colored::*;

pub fn render_table(
    title: &str,
    headers: &[&str],
    rows: Vec<Vec<String>>,
    active_idx: Option<usize>,
) {
    // Compute column widths
    let col_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let max_cell = rows
                .iter()
                .map(|r| r.get(i).map(|s| s.len()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            max_cell.max(h.len())
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

    // Rows
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
        println!();
    }

    // Bottom border
    println!("{}", "-".repeat(total_width));
}
