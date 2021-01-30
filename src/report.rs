use std::path::Path;

use crate::Cfg;

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

pub(crate) fn print_line_number(cfg: &Cfg, line: usize) {
    if cfg.color {
        print!(
            "{}{}{}:",
            cfg.line_num_style.prefix(),
            line + 1,
            cfg.line_num_style.suffix()
        );
    } else {
        print!("{}:", line + 1);
    }
}
