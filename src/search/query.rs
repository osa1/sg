use std::path::Path;

use once_cell::unsync::Lazy;
use tree_sitter::{Node, Query, QueryCursor, Range};

use crate::report::{print_file_path, print_header, print_line_number};
use crate::Cfg;

pub(crate) fn run_query(
    path: &Path,
    query: &Query,
    cfg: &Cfg,
    file_contents: &str,
    node: Node,
    first: &mut bool,
) {
    // Did we print the file name? Only used with `cfg.group`
    let mut header_printed = false;

    let file_lines = Lazy::new(|| file_contents.lines().collect::<Vec<_>>());

    let mut query_cursor = QueryCursor::new();

    for (match_, _) in query_cursor.captures(query, node, ts_text_callback(file_contents)) {
        let mut captures_iter = match_.captures.iter();

        // There will be at least one capture as we add a dummy capture to the whole pattern when
        // processing the CLI arg in `mk_search_mode`

        let parent = captures_iter
            .next()
            .expect("Match doesn't have any captures")
            .node;

        // Print the parent node, highlight locations of captures
        let mut capture_ranges: Vec<Range> = captures_iter.map(|c| c.node.range()).collect();
        capture_ranges.sort_by_key(|range| range.start_byte); // TODO: is this needed?
        report_node_match(
            cfg,
            path,
            parent,
            file_contents,
            Lazy::force(&file_lines).as_ref(),
            &capture_ranges,
            &mut header_printed,
            first,
        );
    }
}

fn report_node_match(
    cfg: &Cfg,
    path: &Path,
    node: Node,
    file_contents: &str,
    file_lines: &[&str],
    capture_ranges: &[Range],
    header_printed: &mut bool,
    first: &mut bool,
) {
    // TODO: This looks bad. I think we shouldn't show the top node as it can be e.g. a function,
    // which can take even a hundred lines sometimes.

    let node_str = match node.utf8_text(file_contents.as_bytes()) {
        Err(err) => {
            eprintln!(
                "Unable to decode token {:?} in {}",
                err,
                path.to_string_lossy()
            );
            return;
        }
        Ok(node_str) => node_str,
    };

    let node_range = node.range();

    print_header(cfg, path, header_printed, first);
    print_file_path(cfg, path);
    let indent = print_line_number(cfg, node_range.start_point.row);

    let mut output = String::with_capacity(node_str.len() * 2);
    let mut chars = node_str.char_indices().peekable();

    for range in capture_ranges.iter() {
        // Push stuff up to the next range
        while chars.peek().unwrap().0 != range.start_byte {
            let c = chars.next().unwrap().1;
            output.push(c);
            if c == '\n' {
                for _ in 0..indent {
                    output.push(' ');
                }
            }
        }

        // Highlight the capture
        // TODO: fast path when color is disabled
        if cfg.color {
            output.push_str(&cfg.match_style.prefix().to_string());
        }

        while chars.peek().unwrap().0 != range.end_byte {
            let c = chars.next().unwrap().1;
            output.push(c);
            if c == '\n' {
                for _ in 0..indent {
                    output.push(' ');
                }
            }
        }

        if cfg.color {
            output.push_str(&cfg.match_style.suffix().to_string());
        }
    }

    // Push rest of the node
    while let Some((_, c)) = chars.next() {
        output.push(c);
        if c == '\n' {
            for _ in 0..indent {
                output.push(' ');
            }
        }
    }

    println!("{}", output);
}

fn ts_text_callback<'a>(source: &'a str) -> impl Fn(Node) -> &'a [u8] {
    move |n| &source.as_bytes()[n.byte_range()]
}
