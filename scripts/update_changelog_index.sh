#!/usr/bin/env bash

set -euo pipefail

index_path="CHANGELOG.md"
changelog_directory="docs/changelog"

{
    printf '%s\n\n' '# Changelog'
    printf '%s\n\n' 'Actus release notes are generated from Conventional Commits after version tags are created.'
    printf '%s\n\n' 'See the [versioned changelogs](docs/changelog/README.md).'

    mapfile -t release_paths < <(find "$changelog_directory" -maxdepth 1 -type f -name 'v*.md' -print | sort -V)
    if ((${#release_paths[@]} == 0)); then
        printf '%s\n' 'There are no tagged releases yet.'
    else
        printf '%s\n\n' '## Releases'
        for release_path in "${release_paths[@]}"; do
            release_name="${release_path##*/}"
            release_name="${release_name%.md}"
            printf -- '- [%s](%s)\n' "$release_name" "$release_path"
        done
    fi
} >"$index_path"
