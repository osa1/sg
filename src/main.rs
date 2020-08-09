use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use tree_sitter::Language;

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
        .get_matches();

    // let rust = unsafe { tree_sitter_rust() };
    // let ocaml = unsafe { tree_sitter_ocaml() };

    // println!("Hello, world!");
}
