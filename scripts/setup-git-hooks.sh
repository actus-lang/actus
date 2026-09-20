#!/bin/sh

set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

git config core.hooksPath .githooks

chmod +x .githooks/commit-msg .githooks/pre-commit .githooks/pre-push

echo "Actus Git hooks enabled via .githooks."
