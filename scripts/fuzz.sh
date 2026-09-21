#!/bin/sh

set -eu

target=${1:-parser}
shift || true

case "$target" in
    lexer|parser|delimiters|literals) ;;
    *)
        echo "error: unsupported fuzz target \`$target\`; expected lexer, parser, delimiters, or literals" >&2
        exit 2
        ;;
esac

cargo fuzz run "$target" "$@"
