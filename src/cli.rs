use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgMatches};

#[derive(Debug)]
pub(crate) struct Args<'a> {
    pub(crate) pattern: String,
    pub(crate) path: Option<String>,
    /// Show column number
    pub(crate) column: bool,
    /// Don't group matches by files
    pub(crate) nogroup: bool,
    /// Colored output
    pub(crate) nocolor: bool,
    /// Case sensitivity
    pub(crate) casing: Casing,
    /// Rest of the matches (`--rust`, `--ocaml` etc.)
    pub(crate) matches: ArgMatches<'a>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Casing {
    /// Match case sensitively unless the pattern contains uppercase chars
    Smart,
    /// Match case sensitively
    Sensitive,
    /// Match case insensitively
    Insensitive,
}

pub(crate) fn parse_args<'a>() -> Args<'a> {
    let mut version = crate_version!().to_owned();
    let commit_hash = env!("GIT_HASH");
    if !commit_hash.is_empty() {
        version = format!("{} ({})", version, commit_hash);
    }

    let m = App::new(crate_name!())
        .version(version.as_str())
        .about(crate_description!())
        .author(crate_authors!())
        .arg(
            Arg::with_name("rust")
                .long("rust")
                .help("Search Rust files"),
        )
        .arg(
            Arg::with_name("ocaml")
                .long("ocaml")
                .help("Search OCaml files"),
        )
        .arg(Arg::with_name("PATTERN").takes_value(true).required(true))
        .arg(Arg::with_name("PATH").takes_value(true).required(false))
        .arg(
            Arg::with_name("color")
                .takes_value(false)
                .long("color")
                .help("Colored output (enabled by default)")
                .overrides_with("nocolor"),
        )
        .arg(
            Arg::with_name("nocolor")
                .takes_value(false)
                .long("nocolor")
                .help("Disable colored output"),
        )
        .arg(
            Arg::with_name("group")
                .takes_value(false)
                .long("group")
                .help("Group matches by file name, print file name once before matches (enabled by default)")
                .overrides_with("nogroup"),
        )
        .arg(Arg::with_name("nogroup").takes_value(false).long("nogroup").help("Print file name in each match"))
        .arg(
            Arg::with_name("column")
                .takes_value(false)
                .long("column")
                .help("Print column numbers in results (disabled by default)"),
        )
        .arg(
            Arg::with_name("smart-case")
                .takes_value(false)
                .long("smart-case")
                .help("Match case insensitively unless PATTERN contains uppercase characters (enabled by default)")
                .short("S"),
        )
        .arg(
            Arg::with_name("case-sensitive")
                .takes_value(false)
                .long("case-sensitive")
                .help("Match case sensitively")
                .short("s"),
        )
        .arg(
            Arg::with_name("ignore-case")
                .takes_value(false)
                .long("ignore-case")
                .help("Match case insensitively")
                .short("i"),
        )
        .after_help(EXAMPLES_STR)
        .get_matches();

    let pattern = m.value_of("pattern").unwrap().to_owned();
    let path = m.value_of("path").map(|s| s.to_owned());
    let column = m.is_present("column");
    let nogroup = m.is_present("nogroup");
    let nocolor = m.is_present("nocolor");

    let smart_case_pos = m.index_of("smart-case").map(|idx| (Casing::Smart, idx));
    let case_sensitive_pos = m
        .index_of("case-sensitive")
        .map(|idx| (Casing::Sensitive, idx));
    let ignore_case_pos = m
        .index_of("ignore-case")
        .map(|idx| (Casing::Insensitive, idx));

    let mut casing_args = vec![smart_case_pos, case_sensitive_pos, ignore_case_pos];
    casing_args.sort_by_key(|arg| arg.as_ref().map(|(_, idx)| *idx));

    let casing = match casing_args.last().unwrap() {
        None => Casing::Smart,
        Some((casing, _)) => *casing,
    };

    Args {
        pattern,
        path,
        column,
        nogroup,
        nocolor,
        casing,
        matches: m,
    }
}

#[rustfmt::skip]
static EXAMPLES_STR: &str = "\
EXAMPLES:
    Search for 'needle' in Rust (.rs) files
        sg --rust needle

    Search for 'needle' case sensitively in OCaml files in  given directory or file
        sg --ocaml needle path -s";
