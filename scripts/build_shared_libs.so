#!/bin/bash

# Generates shared libraries for the OCaml and Rust parsers in parsers/

SCRIPT_DIR=$(dirname "$0")
PARSER_DIR=$SCRIPT_DIR/../parsers
OCAML_DIR=$PARSER_DIR/ocaml/ocaml
RUST_DIR=$PARSER_DIR/rust

set -e
set -x

(cd $RUST_DIR;
 clang src/parser.c src/scanner.c -O -fPIC -Isrc -shared -o libtree_sitter_rust.so)

(cd $OCAML_DIR;
 clang++ src/scanner.cc -O -c -fPIC -Isrc -o scanner.o -stdlib=libc++;
 clang src/parser.c -O -c -fPIC -Isrc -o parser.o;
 clang++ scanner.o parser.o -shared -o libtree_sitter_ocaml.so -stdlib=libc++)
