#!/usr/bin/env bash
set -euo pipefail

readonly script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_root}/resolve_builtin.sh"
readonly fake_root="$(mktemp -d)"
trap 'rm -rf "${fake_root}"' EXIT

candidate_root="${fake_root}/lib/clang/23/lib"
first_candidate="${candidate_root}/thumbv7em-none-eabi/libclang_rt.builtins.a"
second_candidate="${candidate_root}/thumbv7em-unknown-none-eabi/libclang_rt.builtins.a"
mkdir -p "$(dirname "${first_candidate}")" "$(dirname "${second_candidate}")"

if resolve_builtin "${fake_root}" eabi >/dev/null 2>&1; then
    echo "resolver accepted zero candidates" >&2
    exit 1
fi

printf x > "${first_candidate}"
[[ "$(resolve_builtin "${fake_root}" eabi)" == "${first_candidate}" ]]
printf x > "${second_candidate}"
if resolve_builtin "${fake_root}" eabi >/dev/null 2>&1; then
    echo "resolver accepted two candidates" >&2
    exit 1
fi
echo "bash builtin resolver cases passed"
