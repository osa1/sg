mod report;
mod search;

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use tree_sitter::{Language, Parser, Query};

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
        captures,
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
        }) => search::word::search_file(path, word, *case_sensitive, cfg, &contents, root, first),
        SearchMode::Query(query) => {
            search::query::run_query(path, query, cfg, &contents, root, first)
        }
    }
}
