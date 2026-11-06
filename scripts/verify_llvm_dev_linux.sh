#!/usr/bin/env bash
set -euo pipefail

readonly llvm_version="23.1.2"
readonly archive_target="x86_64-unknown-linux-gnu"
readonly archive_name="llvm-dev-${llvm_version}-${archive_target}.tar.xz"
readonly verification_root="${RUNNER_TEMP:-/tmp}/actus-llvm-verification"
readonly extracted_root="${verification_root}/extracted"
readonly expected_image_digest="${ACTUS_GLIBC_IMAGE_DIGEST:?ACTUS_GLIBC_IMAGE_DIGEST is required}"

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "required verification command is missing: $1" >&2
        exit 1
    }
}

for command_name in tar xz sha256sum python3 ldd objdump; do
    require_command "${command_name}"
done

rm -rf "${verification_root}"
mkdir -p "${extracted_root}"
cp "dist/${archive_name}" "${verification_root}/"
cp "dist/${archive_name}.sha256" "${verification_root}/"
(cd "${verification_root}" && sha256sum --check "${archive_name}.sha256")
tar -xJf "${verification_root}/${archive_name}" -C "${extracted_root}"

prefix="${extracted_root}"
export PATH="${prefix}/bin:${PATH}"
llvm_config="${prefix}/bin/llvm-config"
ld_lld="${prefix}/lib/actus/bin/ld.lld"
eabihf_builtins="${prefix}/lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
eabi_builtins="${prefix}/lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
manifest="${prefix}/share/actus/toolchain.json"
for command_name in llvm-config ld.lld llvm-nm llvm-objdump llc; do
    require_command "${command_name}"
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
[[ -x "${ld_lld}" ]] || { echo "bundled ld.lld is missing" >&2; exit 1; }
[[ -s "${eabihf_builtins}" ]] || { echo "Thumb eabihf builtins are missing" >&2; exit 1; }
[[ -s "${eabi_builtins}" ]] || { echo "Thumb eabi builtins are missing" >&2; exit 1; }
for builtin in "${eabihf_builtins}" "${eabi_builtins}"; do
    llvm-nm --defined-only "${builtin}" | awk '{print $NF}' | grep -qx '__aeabi_uldivmod' || {
        echo "${builtin} does not define __aeabi_uldivmod" >&2
        exit 1
    }
done

for binary in "${llvm_config}" "${ld_lld}"; do
    dependencies="$(ldd "${binary}" 2>&1 || true)"
    for forbidden in libstdc++.so libz.so libzstd.so libxml2.so libtinfo.so; do
        if grep -Fq "${forbidden}" <<<"${dependencies}"; then
            echo "${binary} has forbidden dependency ${forbidden}" >&2
            exit 1
        fi
    done
    objdump -T "${binary}" 2>/dev/null | python3 -c '
import re
import sys

for line in sys.stdin:
    for major, minor in re.findall(r"GLIBC_(\d+)\.(\d+)", line):
        if (int(major), int(minor)) > (2, 28):
            raise SystemExit(f"unsupported GLIBC_{major}.{minor}")
'
done

thumb_object="${verification_root}/thumb-zero-float.o"
llc -mtriple=thumbv7em-none-eabihf -mcpu=cortex-m4 -filetype=obj \
    "tests/toolchain/thumb_zero_float.ll" -o "${thumb_object}"
disassembly="$(llvm-objdump -d "${thumb_object}")"
if grep -Eiq '\<(vadd|vsub|vmul|vdiv|vmov|vldr|vstr|vpush|vpop|vmla|vmsr|vmrs)[[:alnum:]._]*\>' <<<"${disassembly}"; then
    echo "Thumb zero-float fixture contains a VFP instruction" >&2
    exit 1
fi

EXPECTED_IMAGE_DIGEST="${expected_image_digest}" python3 - "${manifest}" <<'PY'
import json
import os
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)
assert manifest["llvm_version"] == "23.1.2"
assert manifest["identity"] == "published-archive-sha256"
assert manifest["cmake_flags"]["LLVM_TARGETS_TO_BUILD"] == ["X86", "ARM", "AArch64"]
assert manifest["cmake_flags"]["LLVM_BUILTIN_TARGETS"] == [
    "thumbv7em-none-eabihf",
    "thumbv7em-none-eabi",
]
assert manifest["compiler_rt_builtins"]["thumbv7em-none-eabihf"] == (
    "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
)
assert manifest["compiler_rt_builtins"]["thumbv7em-none-eabi"] == (
    "lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
)
assert manifest["lld"] == "lib/actus/bin/ld.lld"
assert manifest["image_digest"].startswith("sha256:")
assert manifest["image_digest"] == os.environ["EXPECTED_IMAGE_DIGEST"]
assert len(manifest["cmake_cache_sha256"]) == 64
PY
echo "verified ${archive_name} in a clean glibc 2.28 container"
