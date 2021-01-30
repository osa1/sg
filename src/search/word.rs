use std::borrow::Cow;
use std::path::Path;

use once_cell::unsync::Lazy;
use tree_sitter::Node;

use crate::report::{print_file_path, print_header, print_line_number};
use crate::Cfg;

pub(crate) fn search_file(
    path: &Path,
    pattern: &str,
    case_sensitive: bool,
    cfg: &Cfg,
    contents: &str,
    node: Node,
    first: &mut bool,
) {
    let bytes = contents.as_bytes();

    let file_lines = Lazy::new(|| contents.lines().collect::<Vec<_>>());

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

            for match_ in match_token(token_str, pattern, is_id, cfg.whole_word, case_sensitive) {
                report_match(
                    cfg,
                    pattern,
                    path,
                    &node,
                    token_str,
                    Lazy::force(&file_lines).as_ref(),
                    match_,
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

fn report_match(
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
