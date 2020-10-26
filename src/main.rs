use tree_sitter::{Language, Node, Parser};

mod cli;

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_ocaml() -> Language;
}

fn main() {
    let cli::Args {
        pattern,
        file,
        matches,
    } = cli::parse_args();

    let mut langs: Vec<Language> = vec![];
    if matches.is_present("rust") {
        langs.push(unsafe { tree_sitter_rust() });
    }
    if matches.is_present("ocaml") {
        langs.push(unsafe { tree_sitter_ocaml() });
    }

    if langs.is_empty() {
        eprintln!("No language specified; aborting.");
        ::std::process::exit(1);
    }

    if langs.len() > 1 {
        eprintln!("TODO can't process more than one langauge currently; aborting.");
        ::std::process::exit(1);
    }

    let lang = langs.pop().unwrap();

    let contents = ::std::fs::read_to_string(file).unwrap();
    let mut parser = Parser::new();
    parser.set_language(lang).unwrap();

    let tree = parser.parse(contents.as_bytes(), None).unwrap();

    let root = tree.root_node();
    walk(&pattern, contents.as_bytes(), root, 0);
}

fn walk(pattern: &str, src: &[u8], node: Node, level: usize) {
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
                    // indent(level);
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
        walk(pattern, src, child, level + 4);
    }
}

#[allow(dead_code)]
fn indent(level: usize) {
    for _ in 0..level {
        print!(" ");
    }
}
