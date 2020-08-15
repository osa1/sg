use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_ocaml() -> Language;
}

fn main() {
    let mut version = crate_version!().to_owned();
    let commit_hash = env!("GIT_HASH");
    if !commit_hash.is_empty() {
        version = format!("{} ({})", version, commit_hash);
    }

    let m = App::new(crate_name!())
        .version(version.as_str())
        .about(crate_description!())
        .author(crate_authors!())
        .arg(Arg::with_name("rust").long("rust"))
        .arg(Arg::with_name("ocaml").long("ocaml"))
        .arg(Arg::with_name("pattern").takes_value(true).required(true))
        .arg(Arg::with_name("file").takes_value(true).required(true))
        .get_matches();

    let pattern = m.value_of("pattern").unwrap();
    let file = m.value_of("file").unwrap();

    let mut langs: Vec<Language> = vec![];
    if m.is_present("rust") {
        langs.push(unsafe { tree_sitter_rust() });
    }
    if m.is_present("ocaml") {
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
    walk(pattern, contents.as_bytes(), root, 0);
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
                        pos.row,
                        pos.column,
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
