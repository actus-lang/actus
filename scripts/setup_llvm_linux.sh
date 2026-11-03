#!/usr/bin/env bash

set -euo pipefail

llvm_version="${1:?LLVM major version is required}"
tmpdir="$(mktemp -d)"
trap 'rm -rf -- "$tmpdir"' EXIT

archive_version="23.1.2"
archive_name="LLVM-${archive_version}-Linux-X64.tar.xz"
archive_sha256="b5ed9675149cc837c282e9b6962c276c9fa62863d5b2f91537b60848552995b7"

download_with_retry() {
    local url="$1"
    local destination="$2"

    for attempt in 1 2 3 4 5; do
        if curl \
            --fail \
            --silent \
            --show-error \
            --location \
            --retry 3 \
            --retry-all-errors \
            --retry-delay 5 \
            --connect-timeout 20 \
            --max-time 300 \
            "$url" \
            --output "$destination"; then
            return 0
        fi
        echo "LLVM bootstrap download attempt ${attempt}/5 failed; retrying..." >&2
        sleep $((attempt * 5))
    done

    echo "failed to download LLVM bootstrap script after 5 attempts: ${url}" >&2
    return 1
}

bootstrap="$tmpdir/llvm.sh"
if download_with_retry "https://apt.llvm.org/llvm.sh" "$bootstrap"; then
    chmod +x "$bootstrap"

    # Keep the package conflict policy used by the pinned setup action.
    sudo env DPKG_FORCE=overwrite "$bootstrap" "$llvm_version" all
    llvm_root="/usr/lib/llvm-${llvm_version}"
else
    archive="$tmpdir/$archive_name"
    archive_url="https://github.com/llvm/llvm-project/releases/download/llvmorg-${archive_version}/${archive_name}"
    download_with_retry "$archive_url" "$archive"
    printf '%s  %s\n' "$archive_sha256" "$archive" | sha256sum --check --status
    llvm_root="${RUNNER_TEMP}/llvm-${archive_version}"
    rm -rf -- "$llvm_root"
    mkdir -p "$llvm_root"
    tar -xJf "$archive" --strip-components=1 -C "$llvm_root"
fi

llvm_bin="${llvm_root}/bin"
test -x "${llvm_bin}/llvm-config"
echo "$llvm_bin" >>"${GITHUB_PATH}"
export PATH="${llvm_bin}:${PATH}"

installed_version="$(llvm-config --version)"
test "${installed_version%%.*}" = "$llvm_version"
echo "LLVM version: ${installed_version}"
echo "LLVM_PATH=${llvm_root}" >>"${GITHUB_ENV}"
