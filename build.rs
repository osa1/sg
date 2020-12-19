// https://github.com/tree-sitter/tree-sitter/blob/master/lib/binding_rust/README.md

struct LangDir {
    lang_name: &'static str,
    path: &'static str,
    scanner_name: &'static str,
    scanner_cplusplus: bool,
}

static OCAML_LANG: LangDir = LangDir {
    lang_name: "ocaml",
    path: "parsers/ocaml/ocaml/src",
    scanner_name: "scanner.cc",
    scanner_cplusplus: true,
};

static RUST_LANG: LangDir = LangDir {
    lang_name: "rust",
    path: "parsers/rust/src",
    scanner_name: "scanner.c",
    scanner_cplusplus: false,
};

impl LangDir {
    fn scanner_path(&self) -> String {
        format!("{}/{}", self.path, self.scanner_name)
    }

    fn parser_path(&self) -> String {
        format!("{}/parser.c", self.path)
    }
}

static LANGS: [&LangDir; 2] = [&OCAML_LANG, &RUST_LANG];

fn main() {
    for lang in LANGS.iter() {
        let scanner_path = lang.scanner_path();
        let parser_path = lang.parser_path();

        println!("cargo:rerun-if-changed={}", scanner_path);
        println!("cargo:rerun-if-changed={}", parser_path);

        cc::Build::new()
            .include(lang.path)
            .file(scanner_path)
            .cpp(lang.scanner_cplusplus)
            .compile(&format!("{}_scanner", lang.lang_name));

        cc::Build::new()
            .include(lang.path)
            .file(parser_path)
            .compile(&format!("{}_parser", lang.lang_name));
    }

    let hash = rustc_tools_util::get_commit_hash().unwrap_or_default();
    println!("cargo:rustc-env=GIT_HASH={}", hash);
}
