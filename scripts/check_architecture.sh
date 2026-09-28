#!/bin/sh

set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

failed=0

for filename in utils.rs helpers.rs common.rs misc.rs utils.act helpers.act common.act misc.act; do
    if find src library/std/src -type f -name "$filename" -print -quit | grep -q .; then
        echo "Forbidden generic implementation filename: $filename" >&2
        failed=1
    fi
done

check_facades() {
    directory=$1
    for child in "$directory"/*; do
        [ -d "$child" ] || continue
        has_act=false
        for source in "$child"/*.act; do
            [ -f "$source" ] || continue
            has_act=true
            break
        done
        if [ "$has_act" = true ]; then
            name=${child##*/}
            if [ ! -f "$child/$name.act" ]; then
                echo "Missing canonical facade: $child/$name.act" >&2
                failed=1
            fi
        fi
        check_facades "$child"
    done
}

check_facades library/std/src

check_layer() {
    directory=$1
    shift
    for token in "$@"; do
        if rg -n -F --glob '*.rs' -- "$token" "$directory" >/dev/null; then
            echo "Forbidden architecture dependency in $directory: $token" >&2
            failed=1
        fi
    done
}

check_layer src/lexer 'crate::parser' 'crate::ast' 'crate::semantic' 'crate::codegen'
check_layer src/parser 'crate::semantic' 'crate::codegen' 'OwnershipState' 'BorrowRecord' 'SemanticError'
check_layer src/ast 'crate::codegen' 'cranelift' 'NativeType'
check_layer src/semantic 'crate::codegen' 'cranelift' 'render_diagnostic' 'println!' 'eprintln!'
check_layer src/codegen 'SemanticErrorKind' 'SemanticError' 'Analyzer' 'render_diagnostic'

exit "$failed"
