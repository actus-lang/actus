#!/usr/bin/env bash
set -euo pipefail

readonly llvm_version="23.1.2"
readonly llvm_source_revision="${LLVM_SOURCE_REVISION:-85ac560262434c9ccfc0c183ec22d4138ed647fb}"
readonly archive_target="aarch64-apple-darwin"
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
readonly deployment_target="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
readonly runner_image="${ACTUS_MACOS_RUNNER_IMAGE:-macos-15}"

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "required command is missing: $1" >&2
        exit 1
    }
}

for command_name in git cmake ninja ccache python3 xcodebuild xcrun shasum; do
    require_command "${command_name}"
done

[[ -f "${cmake_cache}" ]] || {
    echo "LLVM CMake cache is missing: ${cmake_cache}" >&2
    exit 1
}
[[ "$(xcodebuild -version | head -n 1)" == "Xcode 16.4" ]] || {
    echo "expected Xcode 16.4" >&2
    xcodebuild -version >&2
    exit 1
}
sdk_version="$(xcrun --sdk macosx --show-sdk-version)"

mkdir -p "${work_root}" "${output_root}"
export MACOSX_DEPLOYMENT_TARGET="${deployment_target}"
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
    -G Ninja
    -C "${cmake_cache}"
    -DCMAKE_INSTALL_PREFIX="${install_root}"
    -DCMAKE_INSTALL_LIBDIR=lib
    -DCMAKE_OSX_ARCHITECTURES=arm64
    -DCMAKE_OSX_DEPLOYMENT_TARGET="${deployment_target}"
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
    --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-4}"

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

resolve_builtin() {
    local abi="$1"
    local triple="thumbv7em-none-${abi}"
    local runtime_root="${install_root}/lib/clang/${llvm_major}/lib"
    local candidates=(
        "${runtime_root}/${triple}/libclang_rt.builtins.a"
        "${runtime_root}/thumbv7em-unknown-none-${abi}/libclang_rt.builtins.a"
    )
    local existing=()
    local candidate
    for candidate in "${candidates[@]}"; do
        [[ -s "${candidate}" ]] && existing+=("${candidate}")
    done
    if [[ "${#existing[@]}" -ne 1 ]]; then
        echo "expected exactly one compiler-rt builtin for ${triple}, found ${#existing[@]}" >&2
        echo "all discovered compiler-rt builtins:" >&2
        find "${install_root}" -type f -name 'libclang_rt.builtins*' -print >&2 || true
        exit 1
    fi
    printf '%s\n' "${existing[0]}"
}

thumb_eabihf_builtin="$(resolve_builtin eabihf)"
thumb_eabi_builtin="$(resolve_builtin eabi)"
mkdir -p "${staging_root}/lib/actus/bin" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabihf" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabi" \
    "${staging_root}/share/actus"
cp -a "${install_root}/." "${staging_root}/"
cp "${ld_lld}" "${staging_root}/lib/actus/bin/ld.lld"
cp "${thumb_eabihf_builtin}" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
cp "${thumb_eabi_builtin}" \
    "${staging_root}/lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"

LLVM_VERSION="${llvm_version}" \
LLVM_SOURCE_REVISION="${source_revision}" \
LLVM_TARGETS_BUILT="${targets_built}" \
LLVM_CMAKE_CACHE="${cmake_cache}" \
LLVM_CMAKE_CACHE_SHA256="$(shasum -a 256 "${cmake_cache}" | awk '{print $1}')" \
LLVM_RUNNER_IMAGE="${runner_image}" \
LLVM_XCODE_VERSION="$(xcodebuild -version | tr '\n' ';')" \
LLVM_MACOS_SDK="${sdk_version}" \
LLVM_DEPLOYMENT_TARGET="${deployment_target}" \
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
    "runner_image": os.environ["LLVM_RUNNER_IMAGE"],
    "xcode_version": os.environ["LLVM_XCODE_VERSION"],
    "macos_sdk": os.environ["LLVM_MACOS_SDK"],
    "macos_deployment_target": os.environ["LLVM_DEPLOYMENT_TARGET"],
    "host_triple": "aarch64-apple-darwin",
    "cxx_runtime": "system-libc++",
    "cmake_flags": {
        "LLVM_ENABLE_PROJECTS": ["clang", "lld"],
        "LLVM_TARGETS_TO_BUILD": ["X86", "ARM", "AArch64"],
        "LLVM_BUILTIN_TARGETS": [
            "thumbv7em-none-eabihf",
            "thumbv7em-none-eabi",
        ],
        "LLVM_STATIC_LINK_CXX_STDLIB": False,
        "LLVM_BUILD_LLVM_DYLIB": False,
        "LLVM_ENABLE_ASSERTIONS": False,
        "LLVM_PARALLEL_LINK_JOBS": 1,
        "CMAKE_OSX_ARCHITECTURES": ["arm64"],
        "CMAKE_OSX_DEPLOYMENT_TARGET": os.environ["LLVM_DEPLOYMENT_TARGET"],
    },
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
python3 "${repository_root}/scripts/llvm/package_archive.py" \
    --input-root "${staging_root}" \
    --output "${output_root}/${archive_name}" \
    --format tar.xz
ccache --show-stats
echo "created ${output_root}/${archive_name}"
