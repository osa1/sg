# sg

sg (syntax-aware grep) is a grep-like code search tool that allows searching in
identifiers and keywords, string literals, comments, or a combination of them.
For example, the following command searches for 'test' in string literals and
comments in Rust files:

    sg --rust test -k string,comment

By default (without `-k` or `--kind`, or with `-k identifier`) sg searches in
identifiers and keywords, ignoring string literals and comments.

sg aims to be a drop-in replacement for [ag][2], though a lot of flags are currently
missing.

Under the hood sg uses [tree-sitter][1] parsers. Currently sg comes with Rust
and OCaml parsers, which are enabled with `--rust` and `--ocaml` flags,
respectively.

(For languages that are not built-in to sg we could implement loading parsers
from shared libraries, but that's currently not implemented)

Here are some example uses:

- Search for "fun" in Rust files, ignoring comments and string literals, case
  sensitively:
  ```
  sg fun --rust -s
  ```

- Search for "fun" in OCaml comments and strings, case insensitively:
  ```
  sg fun --ocaml -S -k comment,string
  ```

See also `sg --help`.

sg does not try to be perfect. I haven't benchmarked, but it should be slower
than [ag][2], [rg][3], ack, or grep. tree-sitter can parse incomplete programs,
but not perfectly. Still, I found sg to be useful when a searched word occurs in
comments and strings but I'm only interested in uses in identifiers.

[1]: https://github.com/tree-sitter/tree-sitter
[2]: https://github.com/ggreer/the_silver_searcher
[3]: https://github.com/BurntSushi/ripgrep
