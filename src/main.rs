use std::cell::RefCell;
use std::fs;
use std::path::Path;

use tree_sitter::{Language, Node, Parser};

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
        walk_path(&path, &cfg, &mut first);
    } else {
        search_file(&path, &cfg, &mut first);
    }
}

fn walk_path(path: &Path, cfg: &Cfg, first: &mut bool) {
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
            walk_path(&path, cfg, first);
        } else {
            if let Some(ext) = path.extension() {
                if ext == cfg.ext {
                    search_file(&path, cfg, first);
                }
            }
        }
    }
}

fn search_file(path: &Path, cfg: &Cfg, first: &mut bool) {
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
    walk_ast(path, cfg, &contents, root, first);
}

fn walk_ast(path: &Path, cfg: &Cfg, contents: &str, node: Node, first: &mut bool) {
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
            match node.utf8_text(bytes) {
                Err(err) => {
                    eprintln!(
                        "Unable to decode token {:?} in {}",
                        err,
                        path.to_string_lossy()
                    );
                    continue;
                }
                Ok(token_str) => {
                    let match_: Option<usize> = if cfg.case_sensitive {
                        if is_id && cfg.whole_word {
                            if token_str == cfg.pattern {
                                Some(0)
                            } else {
                                None
                            }
                        } else {
                            token_str.find(&cfg.pattern)
                        }
                    } else {
                        if is_id && cfg.whole_word {
                            if token_str.to_lowercase() == cfg.pattern {
                                Some(0)
                            } else {
                                None
                            }
                        } else {
                            token_str.to_lowercase().find(&cfg.pattern)
                        }
                    };

                    if let Some(match_start) = match_ {
                        let pos = node.start_position();

                        let (token_line, token_col) =
                            get_token_line_col(token_str, pos.column, match_start);

                        let line = pos.row + token_line;
                        let column = token_col;

                        // Print header (if grouping)
                        if !header_printed && cfg.group {
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
                            header_printed = true;
                        }

                        // Print file path for the match (if not grouping)
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

                        // Print line number
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

                        // Print column number (if enabled)
                        if cfg.column {
                            print!("{}:", column + 1);
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
                                continue;
                            }
                        };

                        let before_match = &line[0..column];
                        let match_ = &line[column..column + cfg.pattern.len()];
                        let after_match = &line[column + cfg.pattern.len()..];
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
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            work.push(child);
        }
    }
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
