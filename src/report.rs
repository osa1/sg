use std::path::Path;

use tree_sitter::Node;

use crate::Cfg;

pub(crate) fn report_match(
    cfg: &Cfg,
    pattern: &str,
    path: &Path,
    node: &Node,
    token_str: &str,
    lines: &[&str],
    match_: usize,
    header_printed: &mut bool,
    first: &mut bool,
) {
    let pos = node.start_position();

    let (token_line, token_col) = get_token_line_col(token_str.as_ref(), pos.column, match_);

    let line = pos.row + token_line;
    let column = token_col;

    print_header(cfg, path, header_printed, first);
    print_file_path(cfg, path);
    print_line_number(cfg, line + 1);

    // Print column number (if enabled)
    if cfg.column {
        print!("{}:", column + 1);
    }

    // Print line, highlighting the match
    let line = match lines.get(line) {
        Some(ok) => ok,
        None => {
            eprintln!(
                "Unable to get line {} in {}",
                pos.row,
                path.to_string_lossy()
            );
            return;
        }
    };

    let before_match = &line[0..column];
    let match_ = &line[column..column + pattern.len()];
    let after_match = &line[column + pattern.len()..];
    print!("{}", before_match);
    if cfg.color {
        print!(
            "{}{}{}",
            cfg.match_style.prefix(),
            match_,
            cfg.match_style.suffix()
        );
    } else {
        print!("{}", match_);
    }
    println!("{}", after_match);
}

fn get_token_line_col(token: &str, column0: usize, idx: usize) -> (usize, usize) {
    let mut chars = token.chars();

    let mut line = 0;
    let mut col = column0;

    for _ in 0..idx {
        let c = chars.next();
        if c == Some('\n') {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

// Print header (if grouping, and header not already printed)
pub(crate) fn print_header(cfg: &Cfg, path: &Path, header_printed: &mut bool, first: &mut bool) {
    if !*header_printed && cfg.group {
        if *first {
            *first = false;
        } else {
            println!();
        }

        if cfg.color {
            println!(
                "{}{}{}",
                cfg.file_path_style.prefix(),
                path.to_string_lossy(),
                cfg.file_path_style.suffix()
            );
        } else {
            println!("{}", path.to_string_lossy());
        }
        *header_printed = true;
    }
}

// Print file path for the match (if not grouping)
pub(crate) fn print_file_path(cfg: &Cfg, path: &Path) {
    if !cfg.group {
        if cfg.color {
            print!(
                "{}{}{}:",
                cfg.file_path_style.prefix(),
                path.to_string_lossy(),
                cfg.file_path_style.suffix()
            );
        } else {
            print!("{}:", path.to_string_lossy());
        }
    }
}

pub(crate) fn print_line_number(cfg: &Cfg, line: usize) -> usize {
    let s = format!("{}", line + 1);
    if cfg.color {
        print!(
            "{}{}{}:",
            cfg.line_num_style.prefix(),
            s,
            cfg.line_num_style.suffix()
        );
    } else {
        print!("{}:", s);
    }

    s.len() + 1
}
