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
fn issue_5_1() {
    let str = run_args(&[
        "sg",
        "--ocaml",
        "-k",
        "identifier",
        "1",
        "test_files/issue_5_1.ml",
        "--nocolor",
    ]);

    assert_eq!(
        str,
        "test_files/issue_5_1.ml\n\
         4:1\n"
    );
}

#[test]
fn issue_5_2() {
    let str = run_args(&[
        "sg",
        "--ocaml",
        "-k",
        "identifier",
        "1",
        "test_files/issue_5_2.ml",
        "--column",
    ]);

    assert_eq!(
        str,
        "\u{1b}[1;32mtest_files/issue_5_2.ml\u{1b}[0m\n\
         \u{1b}[1;33m3\u{1b}[0m:32:let checkpoint_max_count = ref \u{1b}[43;30m1\u{1b}[0m5\n"
    );
}
