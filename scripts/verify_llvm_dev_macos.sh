#!/usr/bin/env bash
if ((BASH_VERSINFO[0] < 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] < 4))); then
    echo "Bash >= 4.4 is required" >&2
    exit 1
fi
shopt -s inherit_errexit
set -euo pipefail

readonly llvm_version="23.1.2"
readonly archive_target="aarch64-apple-darwin"
readonly archive_name="llvm-dev-${llvm_version}-${archive_target}.tar.xz"
readonly verification_root="${RUNNER_TEMP:-/tmp}/actus-llvm-macos-verification"
readonly extracted_root="${verification_root}/extracted"
readonly deployment_target="${MACOSX_DEPLOYMENT_TARGET:-13.0}"

macos_version_at_most() {
    local candidate_version="$1"
    local target_version="$2"
    local -a candidate_parts=()
    local -a target_parts=()
    local index
    local candidate_part
    local target_part

    [[ "${candidate_version}" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || return 2
    [[ "${target_version}" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || return 2
    IFS=. read -r -a candidate_parts <<<"${candidate_version}"
    IFS=. read -r -a target_parts <<<"${target_version}"
    for ((index = 0; index < 3; index += 1)); do
        candidate_part="${candidate_parts[index]:-0}"
        target_part="${target_parts[index]:-0}"
        if ((10#${candidate_part} > 10#${target_part})); then
            return 1
        fi
        if ((10#${candidate_part} < 10#${target_part})); then
            return 0
        fi
    done
    return 0
}

if ! macos_version_at_most 13 13.0; then
    echo "macOS version self-check failed: 13 must be at most 13.0" >&2
    exit 1
fi
if ! macos_version_at_most 13.0 13.1; then
    echo "macOS version self-check failed: 13.0 must be at most 13.1" >&2
    exit 1
fi
if macos_version_at_most 14 13.0; then
    echo "macOS version self-check failed: 14 must not be at most 13.0" >&2
    exit 1
fi

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "required verification command is missing: $1" >&2
        exit 1
    }
}

for command_name in tar shasum python3 otool xcrun; do
    require_command "${command_name}"
done

rm -rf "${verification_root}"
mkdir -p "${extracted_root}"
cp "dist/${archive_name}" "${verification_root}/"
cp "dist/${archive_name}.sha256" "${verification_root}/"
expected_digest="$(awk '{print $1}' "${verification_root}/${archive_name}.sha256")"
actual_digest="$(shasum -a 256 "${verification_root}/${archive_name}" | awk '{print $1}')"
[[ "${actual_digest}" == "${expected_digest}" ]] || {
    echo "archive checksum mismatch" >&2
    exit 1
}
tar -xJf "${verification_root}/${archive_name}" -C "${extracted_root}"

prefix="${extracted_root}"
export PATH="${prefix}/bin:${PATH}"
llvm_config="${prefix}/bin/llvm-config"
ld_lld="${prefix}/lib/actus/bin/ld.lld"
eabihf_builtins="${prefix}/lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
eabi_builtins="${prefix}/lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
manifest="${prefix}/share/actus/toolchain.json"

for command_name in llvm-config llvm-nm llvm-objdump llc; do
    resolved="$(command -v "${command_name}")"
    [[ "${resolved}" == "${prefix}/bin/${command_name}" ]] || {
        echo "${command_name} resolved outside archive prefix: ${resolved}" >&2
        exit 1
    }
done
[[ -x "${ld_lld}" ]] || { echo "bundled ld.lld is missing" >&2; exit 1; }
verify_tool_symlink() {
    local link_path="$1"
    [[ -L "${link_path}" ]] || { echo "expected symlink is missing: ${link_path}" >&2; exit 1; }
    local link_target
    link_target="$(readlink "${link_path}")"
    [[ "${link_target}" != /* ]] || { echo "absolute tool symlink: ${link_path}" >&2; exit 1; }
    local resolved_target
    resolved_target="$(python3 - "${link_path}" "${prefix}" <<'PY'
import pathlib
import sys

link = pathlib.Path(sys.argv[1])
prefix_path = pathlib.Path(sys.argv[2]).resolve()
target = link.resolve()
if prefix_path not in target.parents:
    raise SystemExit("tool symlink escapes archive prefix")
if not target.exists():
    raise SystemExit("tool symlink target does not exist")
print(target)
PY
)"
    [[ -x "${resolved_target}" ]] || { echo "tool symlink target is not executable: ${resolved_target}" >&2; exit 1; }
}
for tool_name in ld.lld ld64.lld wasm-ld lld-link; do
    verify_tool_symlink "${prefix}/bin/${tool_name}"
done
[[ "$("${llvm_config}" --version)" == "${llvm_version}" ]] || {
    echo "llvm-config version mismatch" >&2
    exit 1
}
targets_built="$("${llvm_config}" --targets-built)"
for target_name in AArch64 ARM X86; do
    grep -Eq "(^|[; ])${target_name}($|[; ])" <<<"${targets_built}" || {
        echo "LLVM target is missing: ${target_name}; got: ${targets_built}" >&2
        exit 1
    }
done

for builtin in "${eabihf_builtins}" "${eabi_builtins}"; do
    [[ -s "${builtin}" ]] || { echo "missing builtin archive: ${builtin}" >&2; exit 1; }
    llvm-nm --quiet --defined-only "${builtin}" | awk '{print $NF}' | grep -qx '__aeabi_uldivmod' || {
        echo "${builtin} does not define __aeabi_uldivmod" >&2
        exit 1
    }
done

for binary in "${llvm_config}" "${ld_lld}"; do
    minos_versions="$(otool -l "${binary}" | sed -nE 's/^[[:space:]]*minos[[:space:]]+([0-9]+([.][0-9]+){0,2})[[:space:]]*$/\1/p')"
    [[ -n "${minos_versions}" ]] || {
        echo "missing LC_BUILD_VERSION minos in ${binary}" >&2
        exit 1
    }
    while IFS= read -r version; do
        macos_version_at_most "${version}" "${deployment_target}" || {
            echo "${binary} requires macOS ${version}, target is ${deployment_target}" >&2
            exit 1
        }
    done <<<"${minos_versions}"
done

python3 - "${llvm_config}" "${ld_lld}" <<'PY'
import subprocess
import sys

for binary in sys.argv[1:3]:
    dependencies = subprocess.check_output(["otool", "-L", binary], text=True).splitlines()[1:]
    for dependency in dependencies:
        path = dependency.strip().split(" ", 1)[0]
        if any(fragment in path for fragment in ("/opt/homebrew", "/usr/local", "libz", "libzstd", "libxml2")):
            raise SystemExit(f"forbidden non-system dependency in {binary}: {path}")
        if not (path.startswith("/usr/lib/") or path.startswith("/System/")):
            raise SystemExit(f"non-system dependency in {binary}: {path}")
PY

thumb_object="${verification_root}/thumb-zero-float.o"
llc -mtriple=thumbv7em-none-eabihf -mcpu=cortex-m4 -filetype=obj \
    "tests/toolchain/thumb_zero_float.ll" -o "${thumb_object}"
disassembly="$(llvm-objdump -d "${thumb_object}")"
if grep -Eiq '\<(vadd|vsub|vmul|vdiv|vmov|vldr|vstr|vpush|vpop|vmla|vmsr|vmrs)[[:alnum:]._]*\>' <<<"${disassembly}"; then
    echo "Thumb zero-float fixture contains a VFP instruction" >&2
    exit 1
fi

python3 - "${manifest}" "${targets_built}" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)
assert manifest["llvm_version"] == "23.1.2"
assert manifest["host_triple"] == "aarch64-apple-darwin"
assert manifest["cxx_runtime"] == "system-libc++"
assert manifest["macos_deployment_target"] == "13.0"
assert manifest["identity"] == "published-archive-sha256"
assert manifest["cmake_flags"]["LLVM_TARGETS_TO_BUILD"] == ["X86", "ARM", "AArch64"]
assert manifest["targets"] == sys.argv[2].split()
assert len(manifest["cmake_cache_sha256"]) == 64
PY
echo "verified ${archive_name} with archive-prefix and macOS dependency checks"
