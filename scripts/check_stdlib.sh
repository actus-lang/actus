#!/bin/sh

set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if [ -n "${ACTUS_CI_LINKER:-}" ]; then
    export ACTUS_LINKER="$ACTUS_CI_LINKER"
fi

strict_object="$repo_root/target/actus-stdlib-strict.o"
repeat_object="$repo_root/target/actus-stdlib-strict-repeat.o"
trap 'rm -f "$strict_object" "$repeat_object"' EXIT

cargo run --bin actus -- check examples/hello.act --strict
cargo run --bin actus -- build examples/hello.act --strict --emit obj -o "$strict_object"
cargo run --bin actus -- build examples/hello.act --strict --emit obj -o "$repeat_object"
cmp "$strict_object" "$repeat_object"

cargo test --test stdlib_conformance --all-features -- --nocapture
cargo test --test documentation --all-features -- --nocapture
cargo run --bin actus -- lock --check
cargo run --bin actus -- test --strict
