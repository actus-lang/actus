#!/usr/bin/env bash
if ((BASH_VERSINFO[0] < 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] < 4))); then
    echo "Bash >= 4.4 is required" >&2
    return 1 2>/dev/null || exit 1
fi
shopt -s inherit_errexit

resolve_builtin() {
    local resolver_install_root="$1"
    local resolver_abi="$2"
    local resolver_triple="thumbv7em-none-${resolver_abi}"
    local resolver_runtime_root="${resolver_install_root}/lib/clang/23/lib"
    local resolver_candidates=(
        "${resolver_runtime_root}/${resolver_triple}/libclang_rt.builtins.a"
        "${resolver_runtime_root}/thumbv7em-unknown-none-${resolver_abi}/libclang_rt.builtins.a"
    )
    local resolver_existing=()
    local resolver_candidate
    for resolver_candidate in "${resolver_candidates[@]}"; do
        if [[ -s "${resolver_candidate}" ]]; then
            resolver_existing+=("${resolver_candidate}")
        fi
    done
    if [[ "${#resolver_existing[@]}" -ne 1 ]]; then
        echo "expected exactly one compiler-rt builtin for ${resolver_triple}, found ${#resolver_existing[@]}" >&2
        echo "all discovered compiler-rt builtins:" >&2
        find "${resolver_install_root}" -type f -name 'libclang_rt.builtins*' -print >&2 || true
        return 1
    fi
    printf '%s\n' "${resolver_existing[0]}"
}
