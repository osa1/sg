use std::path::Path;

use once_cell::unsync::Lazy;
use tree_sitter::{Node, Query, QueryCursor};

use crate::report::{print_file_path, print_header, print_line_number};
use crate::Cfg;

pub(crate) fn run_query(
    path: &Path,
    query: &Query,
    captures: &[Option<String>], // Expected Values of captures, indexed by capture index
    cfg: &Cfg,
    file_contents: &str,
    node: Node,
    first: &mut bool,
) {
    let bytes = file_contents.as_bytes();

    // Did we print the file name? Only used with `cfg.group`
    let mut header_printed = false;

    let mut query_cursor = QueryCursor::new();

    let file_lines = Lazy::new(|| file_contents.lines().collect::<Vec<_>>());

    for (match_, _) in query_cursor.captures(query, node, ts_text_callback(file_contents)) {
        for capture in match_.captures.iter() {
            let node = capture.node;

            let report = match &captures[capture.index as usize] {
                None => true,
                Some(expected) => {
                    let node_str = match node.utf8_text(bytes) {
                        Err(err) => {
                            eprintln!(
                                "Unable to decode node {:?} in {}",
                                err,
                                path.to_string_lossy()
                            );
                            continue;
                        }
                        Ok(node_str) => node_str,
                    };

                    expected == node_str
                }
            };

            if report {
                report_node_match(
                    cfg,
                    path,
                    node,
                    Lazy::force(&file_lines).as_ref(),
                    file_contents,
                    &mut header_printed,
                    first,
                );
            }
        }
    }
}

fn ts_text_callback<'a>(source: &'a str) -> impl Fn(Node) -> &'a [u8] {
    move |n| &source.as_bytes()[n.byte_range()]
}

fn report_node_match(
    cfg: &Cfg,
    path: &Path,
    node: Node,
    lines: &[&str],
    file_contents: &str,
    header_printed: &mut bool,
    first: &mut bool,
) {
    let node_range = node.range();

    print_header(cfg, path, header_printed, first);
    print_file_path(cfg, path);
    print_line_number(cfg, node_range.start_point.row);

    let line_start = node_range.start_point.row;
    let col_start = node_range.start_point.column;

    let capture_str = &file_contents[node_range.start_byte..node_range.end_byte];
    let mut output = String::new();

    // Print part of the line before the match
    let mut first_line_chars = lines[line_start].chars();
    for _ in 0..col_start {
        output.push(first_line_chars.next().unwrap());
    }

    // Start highlighting
    output.push_str(&cfg.match_style.prefix().to_string());

    // Print captured
    for c in capture_str.chars() {
        output.push(c);
    }

    // Stop highlighting
    output.push_str(&cfg.match_style.suffix().to_string());

    // Print remaining part of the last line
    for c in file_contents[node_range.end_byte..].chars() {
        if c == '\n' {
            break;
        }

        output.push(c);
    }

    println!("{}", output);
}
