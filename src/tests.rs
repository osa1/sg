use std::convert::TryFrom;
use std::ffi::OsString;

use crate::run;

fn run_args(args: &[&str]) -> String {
    let mut stdout: Vec<u8> = vec![];

    let ret = run(
        &mut stdout,
        args.iter().map(|arg| OsString::try_from(arg).unwrap()),
    );

    assert_eq!(ret, 0);

    let str = String::from_utf8(stdout).unwrap().to_owned();

    str
}

#[test]
fn simple() {
    // All occurrences of 'test'
    let str = run_args(&[
        "sg",
        "--rust",
        "test",
        "test_files/simple",
        "-k",
        "string,identifier,comment",
        "--nocolor",
    ]);

    assert_eq!(
        str,
        "test_files/simple/simple.rs\n\
         3:    let s = \"test\";\n\
         2:    let s = \"testtest\";\n\
         2:    let s = \"testtest\";\n\
         1:fn test() {\n"
    );
}

#[test]
fn simple_word() {
    // All occurrences of 'test', only whole words
    let str = run_args(&[
        "sg",
        "--rust",
        "test",
        "test_files/simple",
        "-k",
        "string,identifier,comment",
        "--nocolor",
        "-w",
    ]);

    assert_eq!(
        str,
        "test_files/simple/simple.rs\n\
         3:    let s = \"test\";\n\
         1:fn test() {\n"
    );
}

#[test]
fn simple_word_id() {
    // All occurrences of 'test', only identifiers
    let str = run_args(&[
        "sg",
        "--rust",
        "test",
        "test_files/simple",
        "--nocolor",
        "-w",
    ]);

    assert_eq!(
        str,
        "test_files/simple/simple.rs\n\
         1:fn test() {\n"
    );
}

#[test]
fn simple_query_string() {
    let str = run_args(&[
        "sg",
        "--rust",
        "(function_item name: (identifier) @id)",
        "--qs",
        "test_files/simple",
        "--nocolor",
    ]);

    assert_eq!(
        str,
        "test_files/simple/simple.rs\n\
         0:fn test() {\n\
         6:fn another_function() {\n\
         7:    fn inner_function() {}\n"
    );
}
