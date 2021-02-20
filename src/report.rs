use std::io::Write;
use std::path::Path;

use crate::Cfg;

// Print header (if grouping, and header not already printed)
pub(crate) fn print_header<W: Write>(
    stdout: &mut W,
    cfg: &Cfg,
    path: &Path,
    header_printed: &mut bool,
    first: &mut bool,
) {
    if !*header_printed && cfg.group {
        if *first {
            *first = false;
        } else {
            let _ = writeln!(stdout);
        }

        if cfg.color {
            let _ = writeln!(
                stdout,
                "{}{}{}",
                cfg.file_path_style.prefix(),
                path.to_string_lossy(),
                cfg.file_path_style.suffix()
            );
        } else {
            let _ = writeln!(stdout, "{}", path.to_string_lossy());
        }
        *header_printed = true;
    }
}

// Print file path for the match (if not grouping)
pub(crate) fn print_file_path<W: Write>(stdout: &mut W, cfg: &Cfg, path: &Path) {
    if !cfg.group {
        if cfg.color {
            let _ = write!(
                stdout,
                "{}{}{}:",
                cfg.file_path_style.prefix(),
                path.to_string_lossy(),
                cfg.file_path_style.suffix()
            );
        } else {
            let _ = write!(stdout, "{}:", path.to_string_lossy());
        }
    }
}

pub(crate) fn print_line_number<W: Write>(stdout: &mut W, cfg: &Cfg, line: usize) {
    if cfg.color {
        let _ = write!(
            stdout,
            "{}{}{}:",
            cfg.line_num_style.prefix(),
            line,
            cfg.line_num_style.suffix()
        );
    } else {
        let _ = write!(stdout, "{}:", line);
    }
}
