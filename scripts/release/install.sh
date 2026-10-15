#!/usr/bin/env sh
set -eu

repository="__ACTUS_REPOSITORY__"
version="__ACTUS_VERSION__"
prefix="${ACTUS_INSTALL_PREFIX:-${HOME}/.local}"

case "$(uname -s):$(uname -m)" in
  Linux:x86_64) target="x86_64-unknown-linux-gnu" ;;
  Darwin:x86_64) target="x86_64-apple-darwin" ;;
  Darwin:arm64) target="aarch64-apple-darwin" ;;
  *)
    echo "Actus ${version} installer does not contain a binary for $(uname -s)/$(uname -m)." >&2
    exit 1
    ;;
esac

asset="actus-${version}-${target}.tar.gz"
base_url="https://github.com/${repository}/releases/download/${version}"
temporary_archive="$(mktemp)"
temporary_checksum="$(mktemp)"
install_root="${prefix}/lib/actus/${version}"

cleanup() {
  rm -f "${temporary_archive}" "${temporary_checksum}"
}
trap cleanup EXIT HUP INT TERM

curl --fail --location --silent --show-error "${base_url}/${asset}" --output "${temporary_archive}"
curl --fail --location --silent --show-error "${base_url}/${asset}.sha256" --output "${temporary_checksum}"

expected_checksum="$(awk '{print $1}' "${temporary_checksum}")"
if command -v sha256sum >/dev/null 2>&1; then
  actual_checksum="$(sha256sum "${temporary_archive}" | awk '{print $1}')"
else
  actual_checksum="$(shasum -a 256 "${temporary_archive}" | awk '{print $1}')"
fi
test "${expected_checksum}" = "${actual_checksum}"

mkdir -p "${install_root}"
tar -xzf "${temporary_archive}" --strip-components=1 -C "${install_root}"
mkdir -p "${prefix}/bin"
ln -sfn "${install_root}/bin/actus" "${prefix}/bin/actus"

echo "Actus ${version} installed at ${prefix}/bin/actus"
case ":${PATH}:" in
  *:"${prefix}/bin":*) ;;
  *) echo "Add ${prefix}/bin to PATH to use actus." ;;
esac
