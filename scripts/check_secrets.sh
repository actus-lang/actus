#!/bin/sh

set -eu

secret_pattern='-----BEGIN (RSA|OPENSSH|EC|DSA|PGP) PRIVATE KEY-----|(^|[^A-Za-z0-9])(ghp|gho|ghs|ghr)_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|(^|[^A-Za-z0-9])sk-[A-Za-z0-9]{20,}'

if matches=$(git grep -nI -E "$secret_pattern" -- . ':!Cargo.lock' 2>/dev/null); then
    echo "Potential secret detected in tracked files:" >&2
    printf '%s\n' "$matches" >&2
    exit 1
fi
