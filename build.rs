// https://github.com/tree-sitter/tree-sitter/blob/master/lib/binding_rust/README.md

struct LangDir {
    lang_name: &'static str,
    path: &'static str,
    scanner_name: &'static str,
    scanner_cplusplus: bool,
}

static OCAML_LANG: LangDir = LangDir {
    lang_name: "ocaml",
    path: "/home/omer/tree-sitter/tree-sitter-ocaml/src",
    scanner_name: "scanner.cc",
    scanner_cplusplus: true,
};

static RUST_LANG: LangDir = LangDir {
    lang_name: "rust",
    path: "/home/omer/tree-sitter/tree-sitter-rust/src",
    scanner_name: "scanner.c",
    scanner_cplusplus: false,
};

static LANGS: [&'static LangDir; 2] = [&OCAML_LANG, &RUST_LANG];

fn main() {
    for lang in LANGS.iter() {
        cc::Build::new()
            .include(lang.path)
            .file(format!("{}/{}", lang.path, lang.scanner_name))
            .cpp(lang.scanner_cplusplus)
            .compile(&format!("{}_scanner", lang.lang_name));

        cc::Build::new()
            .include(lang.path)
            .file(format!("{}/{}", lang.path, "parser.c"))
            .compile(&format!("{}_parser", lang.lang_name));
    }

    let hash = rustc_tools_util::get_commit_hash().unwrap_or_default();
    println!("cargo:rustc-env=GIT_HASH={}", hash);
}
