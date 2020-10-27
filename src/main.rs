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
    walk_ast(&cfg.pattern, contents.as_bytes(), root);
}

fn walk_ast(pattern: &str, src: &[u8], node: Node) {
    if node.is_extra() {
        return;
    }

    if node.child_count() == 0 {
        match node.utf8_text(src) {
            Err(err) => {
                panic!("Can't decode token: {:?}", err);
            }
            Ok(token_str) => {
                if let Some(_idx) = token_str.find(pattern) {
                    let pos = node.start_position();
                    println!(
                        "{}:{}: {}",
                        pos.row + 1,
                        pos.column + 1,
                        node.utf8_text(src).unwrap()
                    );
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_ast(pattern, src, child);
    }
}
