use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;

use tree_sitter::{Language, Node, Parser};

mod cli;

#[cfg(test)]
mod tests;

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_ocaml() -> Language;
}

struct Cfg {
    // Use colors
    color: bool,
    // Print column number
    column: bool,
    // Group matches by file
    group: bool,
    // Pattern to search
    pattern: String,
    // tree-sitter node kind, when available search pattern in this kind of nodes
    node_kinds: cli::NodeKinds,
    // Match case sensitively?
    case_sensitive: bool,
    // Only match whole words?
    whole_word: bool,
    // tree-sitter parser
    parser: RefCell<Parser>,
    // Extension of files to search
    ext: &'static str,
    // Style to use for file paths
    file_path_style: ansi_term::Style,
    // Style to use for line numbres
    line_num_style: ansi_term::Style,
    // Style to use for highlighting matched parts
    match_style: ansi_term::Style,
}

fn main() {
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    let ret = run(&mut stdout_lock, &mut std::env::args_os());
    std::process::exit(ret);
}

pub(crate) fn run<W, I, T>(stdout: &mut W, args_iter: I) -> i32
where
    W: Write,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli::Args {
        mut pattern,
        path,
        column,
        nogroup,
        nocolor,
        casing,
        whole_word,
        node_kinds,
        matches,
    } = match cli::parse_args_safe(args_iter) {
        Err(err) => {
            eprintln!("{}", err.message);
            return 1;
        }
        Ok(args) => args,
    };

    let mut lang: Option<(Language, &'static str)> = None;

    if matches.is_present("rust") {
        lang = Some((unsafe { tree_sitter_rust() }, "rs"));
    }

    if matches.is_present("ocaml") {
        lang = Some((unsafe { tree_sitter_ocaml() }, "ml"));
    }

    let (lang, lang_ext) = match lang {
        None => {
            eprintln!("No language specified; aborting.");
            return 1;
        }
        Some(lang) => lang,
    };

    let mut parser = Parser::new();
    parser.set_language(lang).unwrap();

    let path = path
        .map(|s| s.into())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let case_sensitive = match casing {
        cli::Casing::Smart => pattern.chars().any(char::is_uppercase),
        cli::Casing::Sensitive => true,
        cli::Casing::Insensitive => {
            pattern = pattern.to_lowercase();
            false
        }
    };

    let cfg = Cfg {
        color: !nocolor,
        column,
        group: !nogroup,
        pattern,
        node_kinds,
        case_sensitive,
        whole_word,
        parser: RefCell::new(parser),
        ext: lang_ext,
        file_path_style: ansi_term::Colour::Green.bold(),
        line_num_style: ansi_term::Colour::Yellow.bold(),
        match_style: ansi_term::Colour::Black.on(ansi_term::Color::Yellow),
    };

    let mut first = true;

    if path.is_dir() {
        walk_path(stdout, &path, &cfg, &mut first);
    } else {
        search_file(stdout, &path, &cfg, &mut first);
    }

    0
}

fn walk_path<W: Write>(stdout: &mut W, path: &Path, cfg: &Cfg, first: &mut bool) {
    let dir_contents = match fs::read_dir(path) {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!(
                "Unable to read {} contents: {}",
                path.to_string_lossy(),
                err
            );
            return;
        }
    };

    for file in dir_contents {
        let file = match file {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Unable to read dir entry: {}", err);
                continue;
            }
        };

        let path = file.path();

        let meta = match file.metadata() {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Unable to get {} metadata: {}", path.to_string_lossy(), err);
                continue;
            }
        };

        if meta.is_dir() {
            walk_path(stdout, &path, cfg, first);
        } else if let Some(ext) = path.extension() {
            if ext == cfg.ext {
                search_file(stdout, &path, cfg, first);
            }
        }
    }
}

fn search_file<W: Write>(stdout: &mut W, path: &Path, cfg: &Cfg, first: &mut bool) {
    let contents = match fs::read_to_string(path) {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("Unable to read {}: {}", path.to_string_lossy(), err);
            return;
        }
    };

    let tree = match cfg.parser.borrow_mut().parse(contents.as_bytes(), None) {
        Some(ok) => ok,
        None => {
            eprintln!("Unable to parse {}", path.to_string_lossy());
            return;
        }
    };

    let root = tree.root_node();
    walk_ast(stdout, path, cfg, &contents, root, first);
}

fn walk_ast<W: Write>(
    stdout: &mut W,
    path: &Path,
    cfg: &Cfg,
    contents: &str,
    node: Node,
    first: &mut bool,
) {
    let bytes = contents.as_bytes();

    // TODO: Generate this lazily
    let lines: Vec<&str> = contents.lines().collect();

    let mut work = vec![node];

    // Did we print the file name? Only used with `cfg.group`
    let mut header_printed = false;

    while let Some(node) = work.pop() {
        let node_kind = node.kind();

        let mut search = false;
        let is_comment = node_kind == "block_comment" || node_kind == "line_comment";
        search |= is_comment && cfg.node_kinds.comment;
        search |= node_kind == "string_literal" && cfg.node_kinds.string;

        let is_id = !is_comment && node.child_count() == 0 && cfg.node_kinds.identifier;
        search |= is_id;

        if search {
            let token_str = match node.utf8_text(bytes) {
                Err(err) => {
                    eprintln!(
                        "Unable to decode token {:?} in {}",
                        err,
                        path.to_string_lossy()
                    );
                    continue;
                }
                Ok(token_str) => token_str,
            };

            for match_byte_idx in match_token(
                token_str,
                &cfg.pattern,
                is_id,
                cfg.whole_word,
                cfg.case_sensitive,
            ) {
                report_match(
                    stdout,
                    cfg,
                    path,
                    &node,
                    token_str,
                    &lines,
                    match_byte_idx,
                    &mut header_printed,
                    first,
                );
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            work.push(child);
        }
    }
}

fn get_token_line_col(token: &str, column0: usize, mut byte_idx: usize) -> (usize, usize, usize) {
    let mut chars = token.chars().peekable();

    let mut line = 0;
    let mut col = column0;
    let mut col_byte_idx = 0;

    while byte_idx != 0 {
        let c = chars.next().unwrap();
        byte_idx -= c.len_utf8();
        if c == '\r' {
            if let Some('\n') = chars.peek() {
                let _ = chars.next(); // consume '\n'
                byte_idx -= '\n'.len_utf8();
            }
            line += 1;
            col = 0;
            col_byte_idx = 0;
        } else if c == '\n' {
            line += 1;
            col = 0;
            col_byte_idx = 0;
        } else {
            col += 1;
            col_byte_idx += c.len_utf8();
        }
    }

    (line, col, col_byte_idx)
}

fn check_word_bounds(text: &str, match_begin: usize, match_end: usize) -> bool {
    if let Some(char) = text[..match_begin].chars().next_back() {
        if char.is_alphabetic() {
            return false;
        }
    }

    if let Some(char) = text[match_end..].chars().next() {
        if char.is_alphabetic() {
            return false;
        }
    }

    true
}

/// Returns byte indices of matches of `pattern` in `token`
fn match_token(
    token: &str,
    pattern: &str,
    is_id: bool,
    whole_word: bool,
    case_sensitive: bool,
) -> Vec<usize> {
    #[cfg(debug_assertions)]
    if !case_sensitive {
        assert_eq!(pattern, pattern.to_lowercase());
    }

    let token: Cow<'_, str> = if !case_sensitive {
        Cow::Owned(token.to_lowercase())
    } else {
        Cow::Borrowed(token)
    };

    // Special case for whole-word identifiers: don't look at word bounds, expect the whole token
    // to match
    if is_id && whole_word {
        return if token == pattern { vec![0] } else { vec![] };
    }

    // In other cases we'll find the pattern in the token (which may occur multiple times) and
    // check word boundaries when necessary
    token
        .match_indices(pattern)
        .flat_map(|(match_begin, _)| {
            if whole_word
                && !check_word_bounds(token.as_ref(), match_begin, match_begin + pattern.len())
            {
                None.into_iter()
            } else {
                Some(match_begin).into_iter()
            }
        })
        .collect()
}

/// # Arguments
///
/// * `stdout`: A `Write` implementation to write the report. This function does not use process
///   stdout directly, writes to this instead.
///
/// * `cfg`: User configuration (derived from defaults and CLI args)
///
/// * `path`: Path of the file with the match. Will be printed directly to `stdout`.
///
/// * `node`: tree-sitter node with the match. If you convert this node to string with
///   `node.utf8_text()` (use `token_str`), then the searched term will be in the string.
///
/// * `token_str`: `node.utf8_text()`
///
/// * `lines`: Lines of the file that `node` is in (the file at `path`).
///
/// * `match_byte_idx`: Byte indices (in `token_str`) of matches of the searched term in
///   `token_str`.
///
/// * `header_printed`: Whether we've printed a header for the matches in the current file. When
///   grouping matches (default, without `--nogroup`) we print one header per file. With
///   `--nogroup` we print the header for each match.
///
/// * `first`: When grouping (default, without `--nogroup`) we print one header per file, so we
///   keep track of whether the match is the first match. If it is, then we print the header
///   without `--nogroup`.
///
fn report_match<W: Write>(
    stdout: &mut W,
    cfg: &Cfg,
    path: &Path,
    node: &Node,
    token_str: &str,
    lines: &[&str],
    match_byte_idx: usize,
    header_printed: &mut bool,
    first: &mut bool,
) {
    let pos = node.start_position();

    let (token_line, column, mut column_byte) =
        get_token_line_col(token_str, pos.column, match_byte_idx);

    // If we didn't skip any lines, `column_byte` need to be added to the beginning of the token
    if token_line == 0 {
        // Find byte index of the line `node` starts
        let node_row: usize = pos.row;
        // TODO: Cache line start byte indices to avoid repeatedly computing this for matches in
        // the same file
        // TODO: This assumes one-character line ending
        let token_line_byte_idx: usize = lines[0..node_row].iter().map(|s| s.len() + 1).sum();
        column_byte += node.start_byte() - token_line_byte_idx;
    }

    let column_byte = column_byte;

    let line = pos.row + token_line;

    // Print header (if grouping)
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

    // Print file path for the match (if not grouping)
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

    // Print line number
    if cfg.color {
        let _ = write!(
            stdout,
            "{}{}{}:",
            cfg.line_num_style.prefix(),
            line + 1,
            cfg.line_num_style.suffix()
        );
    } else {
        let _ = write!(stdout, "{}:", line + 1);
    }

    // Print column number (if enabled)
    if cfg.column {
        let _ = write!(stdout, "{}:", column + 1);
    }

    // Print line
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

    let before_match = &line[0..column_byte];
    let match_ = &line[column_byte..column_byte + cfg.pattern.len()];
    let after_match = &line[column_byte + cfg.pattern.len()..];
    let _ = write!(stdout, "{}", before_match);
    if cfg.color {
        let _ = write!(
            stdout,
            "{}{}{}",
            cfg.match_style.prefix(),
            match_,
            cfg.match_style.suffix()
        );
    } else {
        let _ = write!(stdout, "{}", match_);
    }
    let _ = writeln!(stdout, "{}", after_match);
}

#[test]
fn test_word_bounds() {
    assert!(check_word_bounds("test", 0, 4));
    assert!(!check_word_bounds("test", 0, 3));
    assert!(!check_word_bounds("test", 1, 4));
    assert!(!check_word_bounds("test", 1, 3));
    assert!(!check_word_bounds("test", 1, 2));
    assert!(!check_word_bounds("test", 2, 3));
    assert!(!check_word_bounds("test", 2, 2));

    assert!(check_word_bounds("a b c", 2, 3));
    assert!(!check_word_bounds("a b c", 2, 4));
    assert!(check_word_bounds("a b c", 2, 5));
}

#[test]
fn test_match_token() {
    assert_eq!(match_token("test", "test", false, false, false), vec![0]);
    assert_eq!(match_token("test", "test", true, false, false), vec![0]);
    assert_eq!(match_token("test", "Test", true, true, true), vec![]);
    assert_eq!(match_token("Test", "Test", true, true, true), vec![0]);

    // Whole word
    assert_eq!(
        match_token("just testing", "test", false, false, false),
        vec![5]
    );
    assert_eq!(
        match_token("just testing", "test", false, true, false),
        vec![]
    );

    // Multiple occurrences in single token
    assert_eq!(
        match_token("tey te tey", "te", false, false, false),
        vec![0, 4, 7]
    );
    assert_eq!(match_token("tey te tey", "te", false, true, false), vec![4]);
    assert_eq!(match_token("tey Te tey", "Te", false, false, true), vec![4]);
}
