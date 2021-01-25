use std::borrow::Cow;
use std::cell::RefCell;
use std::fs;
use std::path::Path;

use once_cell::unsync::Lazy;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, Range};

mod cli;

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
    // tree-sitter node kind, when available search pattern in this kind of nodes
    node_kinds: cli::NodeKinds,
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

#[derive(Debug)]
enum SearchMode {
    /// Word search
    Pattern(Pat),
    /// A tree-sitter query
    Query(Query),
}

#[derive(Debug)]
struct Pat {
    /// The word to search
    word: String,
    /// Whether to match case sensitively. When this is `false`, `pat` is lowercase.
    case_sensitive: bool,
}

fn mk_search_mode(
    lang: Language,
    pat: Option<String>,
    query: Option<cli::Query>,
    casing: cli::Casing,
) -> SearchMode {
    // Returns case sensitivity of the pattern, and adjusts it to lowercase if the it's case
    // insensitive.
    let get_pat_sensitivity = move |pat: &mut String| -> bool {
        match casing {
            cli::Casing::Smart => pat.chars().any(char::is_uppercase),
            cli::Casing::Sensitive => true,
            cli::Casing::Insensitive => {
                *pat = pat.to_lowercase();
                false
            }
        }
    };

    match query {
        None => match pat {
            None => {
                eprintln!("At least a pattern or a query (`--qn` or `--qs`) should be specified.");
                ::std::process::exit(1);
            }
            Some(mut pat) => {
                let case_sensitive = get_pat_sensitivity(&mut pat);
                SearchMode::Pattern(Pat {
                    word: pat,
                    case_sensitive,
                })
            }
        },
        Some(query) => match query {
            cli::Query::Literal(mut query_str) => {
                // Add a dummy capture to capture the full node
                query_str.push_str(" @node");
                match Query::new(lang, &query_str) {
                    Err(err) => panic!("Unable to parse tree-sitter query: {:?}", err),
                    Ok(query) => SearchMode::Query(query),
                }
            }
            cli::Query::Name(_q) => {
                todo!()
            }
        },
    }
}

fn main() {
    let cli::Args {
        pattern,
        path,
        column,
        nogroup,
        nocolor,
        casing,
        whole_word,
        node_kinds,
        query,
        matches,
    } = cli::parse_args();

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
            ::std::process::exit(1);
        }
        Some(lang) => lang,
    };

    let search_mode = mk_search_mode(lang, pattern, query, casing);

    let mut parser = Parser::new();
    parser.set_language(lang).unwrap();

    let path = path
        .map(|s| s.into())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let cfg = Cfg {
        color: !nocolor,
        column,
        group: !nogroup,
        node_kinds,
        whole_word,
        parser: RefCell::new(parser),
        ext: lang_ext,
        file_path_style: ansi_term::Colour::Green.bold(),
        line_num_style: ansi_term::Colour::Yellow.bold(),
        match_style: ansi_term::Colour::Black.on(ansi_term::Color::Yellow),
    };

    let mut first = true;

    if path.is_dir() {
        walk_path(&search_mode, &path, &cfg, &mut first);
    } else {
        search_file(&search_mode, &path, &cfg, &mut first);
    }
}

fn walk_path(search_mode: &SearchMode, path: &Path, cfg: &Cfg, first: &mut bool) {
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
            walk_path(search_mode, &path, cfg, first);
        } else if let Some(ext) = path.extension() {
            if ext == cfg.ext {
                search_file(search_mode, &path, cfg, first);
            }
        }
    }
}

fn search_file(search_mode: &SearchMode, path: &Path, cfg: &Cfg, first: &mut bool) {
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

    match search_mode {
        SearchMode::Pattern(Pat {
            word,
            case_sensitive,
        }) => walk_ast(path, word, *case_sensitive, cfg, &contents, root, first),
        SearchMode::Query(query) => run_query(path, query, cfg, &contents, root, first),
    }
}

fn walk_ast(
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

fn run_query(
    path: &Path,
    query: &Query,
    cfg: &Cfg,
    file_contents: &str,
    node: Node,
    first: &mut bool,
) {
    // Did we print the file name? Only used with `cfg.group`
    let mut header_printed = false;

    let file_lines = Lazy::new(|| file_contents.lines().collect::<Vec<_>>());

    let mut query_cursor = QueryCursor::new();

    for (match_, _) in query_cursor.captures(query, node, ts_text_callback(file_contents)) {
        let mut captures_iter = match_.captures.iter();

        // There will be at least one capture as we add a dummy capture to the whole pattern when
        // processing the CLI arg in `mk_search_mode`

        let parent = captures_iter
            .next()
            .expect("Match doesn't have any captures")
            .node;

        // Print the parent node, highlight locations of captures
        let mut capture_ranges: Vec<Range> = captures_iter.map(|c| c.node.range()).collect();
        capture_ranges.sort_by_key(|range| range.start_byte); // TODO: is this needed?
        report_node_match(
            cfg,
            path,
            parent,
            file_contents,
            Lazy::force(&file_lines).as_ref(),
            &capture_ranges,
            &mut header_printed,
            first,
        );
    }
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

fn report_node_match(
    cfg: &Cfg,
    path: &Path,
    node: Node,
    file_contents: &str,
    file_lines: &[&str],
    capture_ranges: &[Range],
    header_printed: &mut bool,
    first: &mut bool,
) {
    // TODO: This looks bad. I think we shouldn't show the top node as it can be e.g. a function,
    // which can take even a hundred lines sometimes.

    let node_str = match node.utf8_text(file_contents.as_bytes()) {
        Err(err) => {
            eprintln!(
                "Unable to decode token {:?} in {}",
                err,
                path.to_string_lossy()
            );
            return;
        }
        Ok(node_str) => node_str,
    };

    let node_range = node.range();

    print_header(cfg, path, header_printed, first);
    print_file_path(cfg, path);
    let indent = print_line_number(cfg, node_range.start_point.row);

    let mut output = String::with_capacity(node_str.len() * 2);
    let mut chars = node_str.char_indices().peekable();

    for range in capture_ranges.iter() {
        // Push stuff up to the next range
        while chars.peek().unwrap().0 != range.start_byte {
            let c = chars.next().unwrap().1;
            output.push(c);
            if c == '\n' {
                for _ in 0..indent {
                    output.push(' ');
                }
            }
        }

        // Highlight the capture
        // TODO: fast path when color is disabled
        if cfg.color {
            output.push_str(&cfg.match_style.prefix().to_string());
        }

        while chars.peek().unwrap().0 != range.end_byte {
            let c = chars.next().unwrap().1;
            output.push(c);
            if c == '\n' {
                for _ in 0..indent {
                    output.push(' ');
                }
            }
        }

        if cfg.color {
            output.push_str(&cfg.match_style.suffix().to_string());
        }
    }

    // Push rest of the node
    while let Some((_, c)) = chars.next() {
        output.push(c);
        if c == '\n' {
            for _ in 0..indent {
                output.push(' ');
            }
        }
    }

    println!("{}", output);
}

fn ts_text_callback<'a>(source: &'a str) -> impl Fn(Node) -> &'a [u8] {
    move |n| &source.as_bytes()[n.byte_range()]
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

// Print header (if grouping, and header not already printed)
fn print_header(cfg: &Cfg, path: &Path, header_printed: &mut bool, first: &mut bool) {
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
fn print_file_path(cfg: &Cfg, path: &Path) {
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

fn print_line_number(cfg: &Cfg, line: usize) -> usize {
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
