#!/usr/bin/env bash
if ((BASH_VERSINFO[0] < 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] < 4))); then
    echo "Bash >= 4.4 is required" >&2
    exit 1
fi
shopt -s inherit_errexit
set -euo pipefail

readonly llvm_version="23.1.2"
readonly llvm_source_revision="${LLVM_SOURCE_REVISION:-85ac560262434c9ccfc0c183ec22d4138ed647fb}"
readonly archive_target="x86_64-unknown-linux-gnu"
readonly llvm_major="23"
readonly repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly work_root="${LLVM_BUILD_ROOT:-${RUNNER_TEMP:-${repository_root}/.llvm-build}}"
readonly source_root="${work_root}/llvm-project"
readonly build_root="${work_root}/build"
readonly install_root="${work_root}/install"
readonly staging_root="${work_root}/stage"
readonly output_root="${repository_root}/dist"
readonly archive_name="llvm-dev-${llvm_version}-${archive_target}.tar.xz"
readonly cmake_cache="${repository_root}/toolchains/llvm/llvm-${llvm_version}.cmake"
readonly image_digest="${ACTUS_GLIBC_IMAGE_DIGEST:?ACTUS_GLIBC_IMAGE_DIGEST is required}"

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "required command is missing: $1" >&2
        exit 1
    }
}

for command_name in git cmake make ccache python3 tar xz sha256sum gcc; do
    require_command "${command_name}"
done
source "${repository_root}/scripts/llvm/resolve_builtin.sh"

mkdir -p "${work_root}" "${output_root}"
export CCACHE_DIR="${CCACHE_DIR:-${work_root}/ccache}"
ccache --zero-stats >/dev/null

if [[ ! -d "${source_root}/.git" ]]; then
    git clone --filter=blob:none --no-checkout \
        https://github.com/llvm/llvm-project.git "${source_root}"
fi
git -C "${source_root}" fetch --depth=1 origin "${llvm_source_revision}"
git -C "${source_root}" checkout --detach "${llvm_source_revision}"
source_revision="$(git -C "${source_root}" rev-parse HEAD)"

rm -rf "${build_root}" "${install_root}" "${staging_root}"
cmake_args=(
    -S "${source_root}/llvm"
    -B "${build_root}"
    -G "Unix Makefiles"
    -C "${cmake_cache}"
    -DCMAKE_INSTALL_PREFIX="${install_root}"
    -DCMAKE_INSTALL_LIBDIR=lib
    -DLLVM_PARALLEL_LINK_JOBS=1
)
if [[ -n "${ACTUS_CMAKE_LAUNCHER:-}" ]]; then
    cmake_args+=(
        "-DCMAKE_C_COMPILER_LAUNCHER=${ACTUS_CMAKE_LAUNCHER}"
        "-DCMAKE_CXX_COMPILER_LAUNCHER=${ACTUS_CMAKE_LAUNCHER}"
    )
fi
cmake "${cmake_args[@]}"
if [[ "${ACTUS_LLVM_CONFIGURE_ONLY:-0}" == "1" ]]; then
    echo "LLVM CMake configure completed; build intentionally skipped"
    exit 0
fi
cmake --build "${build_root}" --target install-distribution \
    --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"

llvm_config="${install_root}/bin/llvm-config"
ld_lld="${install_root}/bin/ld.lld"
[[ -x "${llvm_config}" ]] || { echo "llvm-config was not installed" >&2; exit 1; }
[[ -x "${ld_lld}" ]] || { echo "ld.lld was not installed" >&2; exit 1; }
[[ "$("${llvm_config}" --version)" == "${llvm_version}" ]] || {
    echo "unexpected LLVM version: $("${llvm_config}" --version)" >&2
    exit 1
}

targets_built="$("${llvm_config}" --targets-built)"
for target_name in AArch64 ARM X86; do
    grep -Eq "(^|[; ])${target_name}($|[; ])" <<<"${targets_built}" || {
        echo "LLVM target is missing: ${target_name}; got: ${targets_built}" >&2
        exit 1
    }
done

thumb_eabihf_builtin="$(resolve_builtin "${install_root}" eabihf)"
thumb_eabi_builtin="$(resolve_builtin "${install_root}" eabi)"

mkdir -p "${staging_root}/lib/actus/bin" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabihf" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabi" \
    "${staging_root}/share/actus" \
    "${staging_root}/licenses"
cp -a "${install_root}/." "${staging_root}/"
cp "${ld_lld}" "${staging_root}/lib/actus/bin/ld.lld"
cp "${thumb_eabihf_builtin}" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
cp "${thumb_eabi_builtin}" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
cp "${source_root}/LICENSE.TXT" "${staging_root}/licenses/LLVM-LICENSE.TXT"
cp "${source_root}/clang/LICENSE.TXT" "${staging_root}/licenses/LLVM-CLANG-LICENSE.TXT"
cp "${source_root}/compiler-rt/LICENSE.TXT" \
    "${staging_root}/licenses/LLVM-COMPILER-RT-LICENSE.TXT"
LLVM_SOURCE_REVISION="${source_revision}" \
python3 - "${staging_root}/licenses/THIRD-PARTY-NOTICES.txt" <<'PY'
import os
import sys

with open(sys.argv[1], "w", encoding="utf-8", newline="\n") as handle:
    handle.write("LLVM 23.1.2 third-party license index\n")
    handle.write(f"Source revision: {os.environ['LLVM_SOURCE_REVISION']}\n")
    handle.write("Component license texts are included in this directory.\n")
PY

LLVM_VERSION="${llvm_version}" \
LLVM_SOURCE_REVISION="${source_revision}" \
LLVM_TARGETS_BUILT="${targets_built}" \
LLVM_CMAKE_CACHE="${cmake_cache}" \
LLVM_CMAKE_CACHE_SHA256="$(sha256sum "${cmake_cache}" | awk '{print $1}')" \
LLVM_IMAGE_DIGEST="${image_digest}" \
LLVM_GCC_VERSION="$(gcc --version | head -n 1)" \
LLVM_THUMB_EABIHF_BUILTINS="lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a" \
LLVM_THUMB_EABI_BUILTINS="lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a" \
LLVM_LLD="lib/actus/bin/ld.lld" \
python3 - "${staging_root}/share/actus/toolchain.json" <<'PY'
import json
import os
import sys

manifest = {
    "llvm_version": os.environ["LLVM_VERSION"],
    "source_revision": os.environ["LLVM_SOURCE_REVISION"],
    "targets": os.environ["LLVM_TARGETS_BUILT"].split(),
    "cmake_cache": os.path.basename(os.environ["LLVM_CMAKE_CACHE"]),
    "cmake_cache_sha256": os.environ["LLVM_CMAKE_CACHE_SHA256"],
    "image_digest": os.environ["LLVM_IMAGE_DIGEST"],
    "gcc_version": os.environ["LLVM_GCC_VERSION"],
    "cmake_flags": {
        "LLVM_ENABLE_PROJECTS": ["clang", "lld"],
        "LLVM_TARGETS_TO_BUILD": ["X86", "ARM", "AArch64"],
        "LLVM_BUILTIN_TARGETS": [
            "thumbv7em-none-eabihf",
            "thumbv7em-none-eabi",
        ],
        "LLVM_STATIC_LINK_CXX_STDLIB": True,
        "LLVM_BUILD_LLVM_DYLIB": False,
        "LLVM_ENABLE_ASSERTIONS": False,
        "LLVM_PARALLEL_LINK_JOBS": 1,
    },
    "disabled_components": [
        "zlib", "zstd", "libxml2", "terminfo", "libedit", "libpfm",
        "curl", "httplib", "ffi", "z3", "libfuzzer", "orc",
        "ctx_profile", "gwp_asan", "crt",
    ],
    "lld": os.environ["LLVM_LLD"],
    "compiler_rt_builtins": {
        "thumbv7em-none-eabihf": os.environ["LLVM_THUMB_EABIHF_BUILTINS"],
        "thumbv7em-none-eabi": os.environ["LLVM_THUMB_EABI_BUILTINS"],
    },
    "identity": "published-archive-sha256",
}
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

rm -f "${output_root}/${archive_name}" "${output_root}/${archive_name}.sha256"
tar -C "${staging_root}" --sort=name --mtime='UTC 1970-01-01' \
    --owner=0 --group=0 --numeric-owner -cJf "${output_root}/${archive_name}" .
sha256sum "${output_root}/${archive_name}" > "${output_root}/${archive_name}.sha256"
ccache --show-stats
echo "created ${output_root}/${archive_name}"
