# sg

sg is a syntax-aware grep-like tool. It allows searching for patterns in the
specified syntax of a language, e.g. `sg test --code --rust` finds occurrences
of "test" in Rust files, excluding occurrences in comments.

sg uses [tree-sitter][1] parsers under the hood.

[1]: https://github.com/tree-sitter/tree-sitter
