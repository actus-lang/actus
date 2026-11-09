#!/usr/bin/env bash
set -euo pipefail

readonly llvm_version="23.1.2"
readonly repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest_path="${repository_root}/toolchains/llvm/llvm-${llvm_version}.json"
install_root="${repository_root}/.actus-toolchain/llvm-${llvm_version}"
offline=0

while (($#)); do
    case "$1" in
        --manifest) manifest_path="$2"; shift 2 ;;
        --install-root) install_root="$2"; shift 2 ;;
        --offline) offline=1; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

case "$(uname -s):$(uname -m)" in
    Linux:x86_64) host_triple="x86_64-unknown-linux-gnu" ;;
    Darwin:arm64) host_triple="aarch64-apple-darwin" ;;
    *) echo "unsupported host; bundled LLVM bootstrap supports Linux x86-64 and macOS arm64" >&2; exit 1 ;;
esac

read -r archive_name archive_url expected_size expected_sha256 < <(
    python3 - "$manifest_path" "$host_triple" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
record = manifest.get("archives", {}).get(sys.argv[2])
if not record:
    raise SystemExit(f"manifest has no archive for {sys.argv[2]}")
values = (record.get("file"), record.get("release_url"), record.get("size"), record.get("sha256"))
if not values[0] or "/" in values[0] or not str(values[1]).startswith("https://") or not isinstance(values[2], int) or not values[3]:
    raise SystemExit(f"manifest record for {sys.argv[2]} is incomplete or unsafe")
print(*values)
PY
)

validate_install() {
    local candidate_root="$1"
    local llvm_config="${candidate_root}/bin/llvm-config"
    [[ -x "${llvm_config}" ]] || return 1
    [[ "$("${llvm_config}" --version)" == "${llvm_version}" ]] || return 1
    local targets_built
    targets_built="$("${llvm_config}" --targets-built)"
    for target_name in AArch64 ARM X86; do
        grep -Eq "(^|[; ])${target_name}($|[; ])" <<<"${targets_built}" || return 1
    done
    [[ -x "${candidate_root}/lib/actus/bin/ld.lld" ]] || return 1
    for abi in eabihf eabi; do
        local builtin="${candidate_root}/lib/actus/targets/thumbv7em-none-${abi}/libclang_rt.builtins.a"
        [[ -s "${builtin}" ]] || return 1
        "${candidate_root}/bin/llvm-nm" --quiet --defined-only "${builtin}" | grep -q '__aeabi_uldivmod' || return 1
    done
    [[ -s "${candidate_root}/licenses/LLVM-LICENSE.TXT" ]] || return 1
    [[ -s "${candidate_root}/licenses/THIRD-PARTY-NOTICES.txt" ]] || return 1
}

if [[ -d "${install_root}" ]]; then
    if validate_install "${install_root}"; then
        echo "verified LLVM ${llvm_version} already exists at ${install_root}; no download needed"
        exit 0
    fi
    echo "existing LLVM directory is not verified; refusing to modify ${install_root}" >&2
    exit 1
fi
if ((offline)); then
    echo "offline bootstrap requested but no verified LLVM directory exists at ${install_root}" >&2
    exit 1
fi

for command_name in curl python3 tar; do
    command -v "${command_name}" >/dev/null || { echo "required command is missing: ${command_name}" >&2; exit 1; }
done
parent_root="$(dirname "${install_root}")"
mkdir -p "${parent_root}"
temporary_root="$(mktemp -d "${parent_root}/.llvm-${llvm_version}.XXXXXX")"
trap 'rm -rf "${temporary_root}"' EXIT
archive_path="${temporary_root}/${archive_name}"
archive_cache_path="${ACTUS_LLVM_ARCHIVE_CACHE:-}"
archive_cache_hit=false
if [[ -n "${archive_cache_path}" && -f "${archive_cache_path}" ]]; then
    archive_path="${archive_cache_path}"
    archive_cache_hit=true
    echo "using cached LLVM archive ${archive_path}"
else
    curl_retry_args=(--retry 5 --retry-delay 10 --connect-timeout 30)
    if curl --help all 2>/dev/null | grep -q -- '--retry-all-errors'; then
        curl_retry_args+=(--retry-all-errors)
    fi
    curl --fail --location --proto '=https' --tlsv1.2 \
        "${curl_retry_args[@]}" \
        "${archive_url}" -o "${archive_path}"
fi
if command -v sha256sum >/dev/null; then
    actual_sha256="$(sha256sum "${archive_path}" | awk '{print $1}')"
else
    actual_sha256="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
fi
[[ "${actual_sha256}" == "${expected_sha256}" ]] || { echo "LLVM archive SHA-256 mismatch: expected ${expected_sha256}, got ${actual_sha256}" >&2; exit 1; }
actual_size="$(python3 -c 'import os, sys; print(os.path.getsize(sys.argv[1]))' "${archive_path}")"
[[ "${actual_size}" == "${expected_size}" ]] || { echo "LLVM archive size mismatch: expected ${expected_size}, got ${actual_size}" >&2; exit 1; }
if [[ -n "${archive_cache_path}" && "${archive_cache_hit}" == false ]]; then
    mkdir -p "$(dirname "${archive_cache_path}")"
    cp "${archive_path}" "${archive_cache_path}"
    archive_path="${archive_cache_path}"
fi
extract_root="${temporary_root}/extracted"
mkdir "${extract_root}"
tar -xJf "${archive_path}" -C "${extract_root}"
validate_install "${extract_root}"
mv "${extract_root}" "${install_root}"
trap - EXIT
rm -rf "${temporary_root}"
echo "installed verified LLVM ${llvm_version} at ${install_root}"
