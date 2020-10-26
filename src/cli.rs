use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgMatches};

#[derive(Debug)]
pub(crate) struct Args<'a> {
    pub(crate) pattern: String,
    pub(crate) file: String,
    pub(crate) matches: ArgMatches<'a>,
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
        .arg(Arg::with_name("rust").long("rust"))
        .arg(Arg::with_name("ocaml").long("ocaml"))
        .arg(Arg::with_name("pattern").takes_value(true).required(true))
        .arg(Arg::with_name("file").takes_value(true).required(true))
        .get_matches();

    let pattern = m.value_of("pattern").unwrap().to_owned();
    let file = m.value_of("file").unwrap().to_owned();

    Args {
        pattern,
        file,
        matches: m,
    }
}
