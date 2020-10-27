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
    // tree-sitter parser
    parser: RefCell<Parser>,
    // Extension of files to search
    ext: &'static str,
    // Style to use for file paths
    file_path_style: ansi_term::Style,
    // Style to use for line numbres
    line_num_style: ansi_term::Style,
}

fn main() {
    let cli::Args {
        pattern,
        path,
        column,
        nogroup,
        nocolor,
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

    let cfg = Cfg {
        color: !nocolor,
        column,
        group: !nogroup,
        pattern,
        parser: RefCell::new(parser),
        ext: lang_ext,
        file_path_style: ansi_term::Colour::Green.bold(),
        line_num_style: ansi_term::Colour::Yellow.bold(),
    };

    if path.is_dir() {
        walk_path(&path, &cfg);
    } else {
        search_file(&path, &cfg);
    }
}

fn walk_path(path: &Path, cfg: &Cfg) {
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
            walk_path(&path, cfg);
        } else {
            if let Some(ext) = path.extension() {
                if ext == cfg.ext {
                    search_file(&path, cfg);
                }
            }
        }
    }
}

fn search_file(path: &Path, cfg: &Cfg) {
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
    walk_ast(path, cfg, &contents, root);
}

fn walk_ast(path: &Path, cfg: &Cfg, contents: &str, node: Node) {
    let bytes = contents.as_bytes();

    // TODO: Generate this lazily
    let lines: Vec<&str> = contents.lines().collect();

    let mut work = vec![node];

    // Did we print the file name? Only used with `cfg.group`
    let mut header_printed = false;

    while let Some(node) = work.pop() {
        if node.is_extra() {
            // Comments, brackets, etc.
            return;
        }

        if node.child_count() == 0 {
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
                    if let Some(_idx) = token_str.find(&cfg.pattern) {
                        let pos = node.start_position();

                        // Print header (if grouping)
                        if !header_printed && cfg.group {
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
                                pos.row + 1,
                                cfg.line_num_style.suffix()
                            );
                        } else {
                            print!("{}:", pos.row + 1);
                        }

                        // Print column number (if enabled)
                        if cfg.column {
                            print!("{}:", pos.column + 1);
                        }

                        // Print line TODO highlight match
                        println!("{}", lines[pos.row]);
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
