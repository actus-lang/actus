#!/usr/bin/env bash
set -euo pipefail

readonly llvm_version="23.1.2"
readonly archive_target="aarch64-apple-darwin"
readonly archive_name="llvm-dev-${llvm_version}-${archive_target}.tar.xz"
readonly verification_root="${RUNNER_TEMP:-/tmp}/actus-llvm-macos-verification"
readonly extracted_root="${verification_root}/extracted"
readonly deployment_target="${MACOSX_DEPLOYMENT_TARGET:-13.0}"

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
    llvm-nm --defined-only "${builtin}" | awk '{print $NF}' | grep -qx '__aeabi_uldivmod' || {
        echo "${builtin} does not define __aeabi_uldivmod" >&2
        exit 1
    }
done

python3 - "${llvm_config}" "${ld_lld}" "${deployment_target}" <<'PY'
import re
import subprocess
import sys

deployment = tuple(map(int, sys.argv[3].split(".")))
for binary in sys.argv[1:3]:
    load_commands = subprocess.check_output(["otool", "-l", binary], text=True)
    matches = re.findall(r"\bminos\s+(\d+\.\d+(?:\.\d+)?)", load_commands)
    if not matches:
        raise SystemExit(f"missing LC_BUILD_VERSION minos in {binary}")
    for version in matches:
        if tuple(map(int, version.split("."))) > deployment:
            raise SystemExit(f"{binary} requires macOS {version}, target is {sys.argv[3]}")

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
