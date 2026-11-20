#!/bin/sh
# Install an Actus release on Linux x86-64 or macOS arm64.
#
#   curl -fsSL https://actus-lang.org/install.sh | sh
#   curl -fsSL https://actus-lang.org/install.sh | sh -s -- --version 0.1.0
#
# The archive is checked against the release's SHA256SUMS before anything is
# installed. Each version lives in $ACTUS_HOME/toolchains/<archive>; the
# $ACTUS_HOME/current link moves to a new version only after its compiler has
# run, so a failed install leaves the previous one in place.

set -eu

repository="https://github.com/actus-lang/actus"
actus_home="${ACTUS_HOME:-$HOME/.actus}"
version=""
modify_path=1

fail() {
    echo "actus-install: $*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: install.sh [--version <x.y.z>] [--prefix <dir>] [--no-modify-path]

  --version <x.y.z>  install this release instead of the latest one
  --prefix <dir>     install under <dir> instead of $ACTUS_HOME or ~/.actus
  --no-modify-path   do not add Actus to the shell profile
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) [ "$#" -ge 2 ] || fail "--version needs a value"; version="${2#v}"; shift 2 ;;
        --prefix) [ "$#" -ge 2 ] || fail "--prefix needs a value"; actus_home="$2"; shift 2 ;;
        --no-modify-path) modify_path=0; shift ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; fail "unknown option: $1" ;;
    esac
done

case "$(uname -s):$(uname -m)" in
    Linux:x86_64 | Linux:amd64) target="x86_64-unknown-linux-gnu" ;;
    Darwin:arm64 | Darwin:aarch64) target="aarch64-apple-darwin" ;;
    *) fail "no Actus release for $(uname -s) $(uname -m); releases cover Linux x86-64, macOS arm64 and Windows x86-64" ;;
esac

for command_name in curl tar mkdir mktemp; do
    command -v "$command_name" >/dev/null 2>&1 || fail "required command is missing: $command_name"
done
if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
    fail "required command is missing: sha256sum or shasum"
fi

# ACTUS_INSTALL_BASE_URL points at a directory holding the archive and
# SHA256SUMS, such as a file:// URL in release verification.
base_url="${ACTUS_INSTALL_BASE_URL:-}"
if [ -z "$version" ]; then
    [ -z "$base_url" ] || fail "--version is required with ACTUS_INSTALL_BASE_URL"
    latest="$(curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
        --output /dev/null --write-out '%{url_effective}' "$repository/releases/latest")" \
        || fail "cannot reach $repository/releases/latest"
    version="${latest##*/tag/v}"
    [ "$version" != "$latest" ] || fail "cannot read the latest release from $latest"
fi
case "$version" in
    *[!0-9A-Za-z.+-]* | "") fail "invalid version: $version" ;;
esac
[ -n "$base_url" ] || base_url="$repository/releases/download/v$version"

name="actus-$version-$target"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM

download() {
    curl --fail --silent --show-error --location --proto '=https,file' --tlsv1.2 \
        --retry 3 --output "$work/$1" "$base_url/$1" || fail "cannot download $base_url/$1"
}

echo "Installing Actus $version ($target)"
download SHA256SUMS
download "$name.tar.xz"
expected="$(awk -v file="$name.tar.xz" '$2 == file || $2 == "*" file { print $1 }' "$work/SHA256SUMS")"
[ -n "$expected" ] || fail "SHA256SUMS has no entry for $name.tar.xz"
actual="$(sha256 "$work/$name.tar.xz")"
[ "$actual" = "$expected" ] || fail "checksum mismatch for $name.tar.xz: expected $expected, got $actual"

mkdir -p "$actus_home/toolchains"
staging="$(mktemp -d "$actus_home/toolchains/.install.XXXXXX")"
tar -xJf "$work/$name.tar.xz" -C "$staging" || { rm -rf "$staging"; fail "cannot extract $name.tar.xz"; }
[ -x "$staging/$name/bin/actus" ] || { rm -rf "$staging"; fail "archive has no $name/bin/actus"; }
"$staging/$name/bin/actus" --version >/dev/null || { rm -rf "$staging"; fail "the installed compiler does not run"; }

destination="$actus_home/toolchains/$name"
rm -rf "$destination"
mv "$staging/$name" "$destination"
rm -rf "$staging"
ln -sfn "toolchains/$name" "$actus_home/current"

bin_dir="$actus_home/current/bin"
cat >"$actus_home/env" <<EOF
# Added by the Actus installer.
case ":\$PATH:" in
    *":$bin_dir:"*) ;;
    *) export PATH="$bin_dir:\$PATH" ;;
esac
EOF

profile=""
if [ "$modify_path" -eq 1 ]; then
    case "$(basename "${SHELL:-sh}")" in
        zsh) profile="${ZDOTDIR:-$HOME}/.zshrc" ;;
        bash) if [ "$target" = "aarch64-apple-darwin" ]; then profile="$HOME/.bash_profile"; else profile="$HOME/.bashrc"; fi ;;
        fish) profile="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/actus.fish" ;;
        *) profile="$HOME/.profile" ;;
    esac
    case "$profile" in
        *.fish) line="fish_add_path --global \"$bin_dir\"" ;;
        *) line=". \"$actus_home/env\"" ;;
    esac
    mkdir -p "$(dirname "$profile")"
    touch "$profile"
    grep -qxF "$line" "$profile" || printf '\n# Actus\n%s\n' "$line" >>"$profile"
fi

"$destination/bin/actus" --version
echo
echo "Actus $version is installed in $destination"
if [ -n "$profile" ]; then
    echo "Open a new terminal, or run: . \"$actus_home/env\""
else
    echo "Add $bin_dir to PATH, for example: . \"$actus_home/env\""
fi
