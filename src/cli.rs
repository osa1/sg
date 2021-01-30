use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgMatches};
use fxhash::FxHashMap;

#[derive(Debug)]
pub(crate) struct Args<'a> {
    pub(crate) pattern: Option<String>,
    pub(crate) path: Option<String>,
    /// Show column number
    pub(crate) column: bool,
    /// Don't group matches by files
    pub(crate) nogroup: bool,
    /// Colored output
    pub(crate) nocolor: bool,
    /// Case sensitivity
    pub(crate) casing: Casing,
    /// Only match whole words?
    pub(crate) whole_word: bool,
    /// tree-sitter node kinds. When specified only search the pattern in these kinds of nodes.
    pub(crate) node_kinds: NodeKinds,
    /// A query literal or name
    pub(crate) query: Option<Query>,
    /// When using `query`, specifies values of captures. Keys are capture names, values are
    /// expected values.
    pub(crate) captures: FxHashMap<String, String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeKinds {
    /// Search in identifiers and keywords
    pub(crate) identifier: bool,
    /// Search in string literals
    pub(crate) string: bool,
    /// Search in comments
    pub(crate) comment: bool,
}

#[derive(Debug)]
pub(crate) enum Query {
    /// An actual tree-sitter query
    Literal(String),
    /// A query name
    Name(String),
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
        .arg(Arg::with_name("PATTERN").takes_value(true).required(false))
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
        .arg(
            Arg::with_name("word")
            .takes_value(false)
            .long("word")
            .short("w")
            .help("Only match whole words")
        )
        .arg(
            Arg::with_name("kind")
            .takes_value(true)
            .required(false)
            .short("k")
            .long("kind")
            .long_help(KIND_HELP)
        )
        .arg(
            Arg::with_name("query-name")
            .takes_value(true)
            .required(false)
            .long("qn")
            .long_help(QUERY_NAME_HELP)
            .overrides_with("query-str")
        )
        .arg(
            Arg::with_name("query-str")
            .takes_value(true)
            .required(false)
            .long("qs")
            .long_help(QUERY_STR_HELP)
        )
        .arg(
            Arg::with_name("capture")
            .short("c")
            .takes_value(true)
            .required(false)
            .multiple(true)
            .number_of_values(1)
        )
        .after_help(HELP_MORE)
        .get_matches();

    let pattern = m.value_of("PATTERN").map(str::to_owned);
    let path = m.value_of("PATH").map(str::to_owned);
    let column = m.is_present("column");
    let nogroup = m.is_present("nogroup");
    let nocolor = m.is_present("nocolor");
    let whole_word = m.is_present("word");
    let qs = m.value_of("query-str").map(str::to_owned);
    let qn = m.value_of("query-name").map(str::to_owned);

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

    let node_kinds = match m.value_of("kind") {
        Some(val) => {
            let mut kinds = NodeKinds {
                identifier: false,
                comment: false,
                string: false,
            };
            for kind in val.trim().split(',') {
                match kind {
                    "identifier" => {
                        kinds.identifier = true;
                    }
                    "comment" => {
                        kinds.comment = true;
                    }
                    "string" => {
                        kinds.string = true;
                    }
                    other => {
                        eprintln!("Invalid kind: {}, expected a comma-separated list of: 'identifier', 'comment', 'string'", other);
                        ::std::process::exit(1);
                    }
                }
            }
            kinds
        }
        None => NodeKinds {
            identifier: true,
            comment: false,
            string: false,
        },
    };

    let query = match (qs, qn) {
        (None, None) => None,
        (Some(qs), None) => Some(Query::Literal(qs)),
        (None, Some(qn)) => Some(Query::Name(qn)),
        (Some(_), Some(_)) => {
            // Should be caught by clap
            panic!("Both query name and string were specified")
        }
    };

    let captures: FxHashMap<String, String> = match m.values_of("capture") {
        Some(captures) if query.is_some() => {
            let mut map: FxHashMap<String, String> = Default::default();
            for capture in captures {
                // TODO: Use split_once after it's stabilized
                match capture.find('=') {
                    Some(split_idx) => {
                        let k = capture[..split_idx].to_owned();
                        let v = capture[split_idx + 1..].to_owned();
                        map.insert(k, v);
                    }
                    None => {
                        eprintln!(
                            "`{}` is not a valid capture, should be in fork `<name>=<value>`",
                            capture
                        );
                        ::std::process::exit(1);
                    }
                }
            }
            map
        }
        _ => Default::default(),
    };

    Args {
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
        matches: m,
    }
}

// TODO: mention these
//
// - A query should have at least one capture otherwise it will be useless. Captures do not have to
//   have a value.
// - Capture values are OR'ed (ie. matched ones are reported, not all of them need to match)

#[rustfmt::skip]
static HELP_MORE: &str = "\
QUERIES:
    Queries are used to search for multiple tokens in the AST, or for more complicated \
    syntax-aware matching, using tree-sitter query syntax [1]. sg comes with a set of \
    built-in queries on Rust. TODO say more

    [1]: https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax


EXAMPLES:
    Search for 'fun' in Rust (.rs) files
        sg --rust fun

    Search for 'fun' in Rust (.rs) files, in comments and string literals
        sg --rust fun --kind comment,string

    Search for 'fun' case sensitively in OCaml files in given directory or file
        sg --ocaml fun path -s

    TODO: query exaples";

#[rustfmt::skip]
static KIND_HELP: &str = 
    "Comma-separated list of AST node kinds. When specified only search pattern in these kind of \
     tree-sitter nodes. Possible values: 'identifier' (for identifiers and keywords), 'comment' \
     (for comments), 'string' (for string literals). Default is 'identifier'.

Example: --kind identifier,comment,string";

static QUERY_NAME_HELP: &str =
    "Name of the tree-sitter query to run on the AST. See \"queries\" below for details.";

static QUERY_STR_HELP: &str =
    "tree-sitter query to run on the AST. See \"queries\" below for details.";
