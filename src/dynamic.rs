//! Implement loading parser from shared libraries

use std::path::Path;

pub(crate) enum Error {
    IO(std::io::Error),
    Goblin(goblin::error::Error),
    Libloading(libloading::Error),
    CantFindLangName,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<goblin::error::Error> for Error {
    fn from(err: goblin::error::Error) -> Self {
        Error::Goblin(err)
    }
}

impl From<libloading::Error> for Error {
    fn from(err: libloading::Error) -> Self {
        Error::Libloading(err)
    }
}

pub(crate) fn load_parser(
    lib_path: &Path,
    lang_name: Option<&str>,
) -> Result<tree_sitter::Language, Error> {
    let lang_name = match lang_name {
        None => match find_lang_sym(lib_path)? {
            Some(lang_name) => lang_name,
            None => {
                return Err(Error::CantFindLangName);
            }
        },
        Some(lang_name) => lang_name.to_owned(),
    };

    let library = libloading::Library::new(lib_path)?;

    let language: tree_sitter::Language = unsafe {
        let language_fn: libloading::Symbol<unsafe extern "C" fn() -> tree_sitter::Language> =
            library.get(lang_name.as_bytes())?;
        language_fn()
    };

    Ok(language)
}

fn find_lang_sym(lib_path: &Path) -> Result<Option<String>, Error> {
    let contents = std::fs::read(lib_path)?;
    let obj = goblin::Object::parse(&contents)?;

    if let goblin::Object::Elf(elf) = obj {
        for sym in elf.dynsyms.iter() {
            if let Some(Ok(sym_name)) = elf.dynstrtab.get(sym.st_name) {
                if sym_name.starts_with(TS_SYM_PREFIX) {
                    let mut ts_sym = false;

                    for ts_suffix in TS_SYM_SUFFIXES.iter() {
                        if sym_name.ends_with(ts_suffix) {
                            ts_sym = true;
                            break;
                        }
                    }

                    if !ts_sym {
                        return Ok(Some(sym_name[TS_SYM_PREFIX.len()..].to_owned()));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Language symbol starts with this
static TS_SYM_PREFIX: &str = "tree_sitter_";

/// Language symbol should not end with any these
static TS_SYM_SUFFIXES: [&str; 6] = [
    "external_scanner_create",
    "external_scanner_deserialize",
    "external_scanner_destroy",
    "external_scanner_reset",
    "external_scanner_scan",
    "external_scanner_serialize",
];
