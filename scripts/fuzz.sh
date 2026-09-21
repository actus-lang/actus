#!/bin/sh

set -eu

target=${1:-parser}
shift || true

case "$target" in
    lexer|parser) ;;
    *)
        echo "error: unsupported fuzz target \`$target\`; expected lexer or parser" >&2
        exit 2
        ;;
esac

cargo fuzz run "$target" "$@"
