#!/usr/bin/env bash

resolve_builtin() {
    local install_root="$1"
    local abi="$2"
    local triple="thumbv7em-none-${abi}"
    local runtime_root="${install_root}/lib/clang/23/lib"
    local candidates=(
        "${runtime_root}/${triple}/libclang_rt.builtins.a"
        "${runtime_root}/thumbv7em-unknown-none-${abi}/libclang_rt.builtins.a"
    )
    local existing=()
    local candidate
    for candidate in "${candidates[@]}"; do
        if [[ -s "${candidate}" ]]; then
            existing+=("${candidate}")
        fi
    done
    if [[ "${#existing[@]}" -ne 1 ]]; then
        echo "expected exactly one compiler-rt builtin for ${triple}, found ${#existing[@]}" >&2
        echo "all discovered compiler-rt builtins:" >&2
        find "${install_root}" -type f -name 'libclang_rt.builtins*' -print >&2 || true
        return 1
    fi
    printf '%s\n' "${existing[0]}"
}
