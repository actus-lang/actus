# ADR-0116: Release Installers

## Status

Accepted.

## Context

Each Actus release publishes one relocatable archive per supported host,
with the compiler, its bundled LLVM, the standard library, the runtime and
the linker inside, together with `SHA256SUMS`. Installing one by hand means
picking the archive, checking its digest, extracting it, and adding its
`bin/` directory to `PATH`. Projects that build with Actus, and their CI,
need a single command that does this the same way every time.

`actus` finds its toolchain relative to its own executable, so the
executable must stay inside the extracted archive; a link to the executable
alone is not enough on every host.

## Decision

The repository ships two installers, `scripts/install.sh` for Linux x86-64
and macOS arm64 and `scripts/install.ps1` for Windows x86-64, served as
`https://actus-lang.org/install.sh` and `https://actus-lang.org/install.ps1`.

An installer:

1. picks the host archive of the latest release, or of the version given
   with `--version` (`-Version`);
2. downloads the archive and the release's `SHA256SUMS` and stops before
   extracting anything when the digest differs;
3. extracts the archive into `<home>/toolchains/<archive name>`, where
   `<home>` is `~/.actus` (or `%LOCALAPPDATA%\Actus`), `ACTUS_HOME`, or the
   `--prefix` (`-Prefix`) directory, and runs the extracted compiler;
4. only then points `<home>/current` at the new toolchain, a symbolic link
   on Unix and a directory junction on Windows, so a failed install leaves
   the previous one usable;
5. puts `<home>/current/bin` on `PATH`: through a sourced `<home>/env` file
   from the shell profile on Unix and through the user `Path` on Windows,
   unless `--no-modify-path` (`-NoModifyPath`) is given.

Because `PATH` names a directory inside the toolchain through `current`,
the executable keeps its archive layout and switching versions changes one
link.

`ACTUS_INSTALL_BASE_URL` replaces the release download location with a
directory holding an archive and `SHA256SUMS`. The release workflow uses it
to install every freshly built archive with the installer for its host,
check the installed compiler's identity, and build a program with it before
anything is published.

Release builds set `ACTUS_SOURCE_COMMIT` to the source commit, because a
build container may not be allowed to read the checkout's git metadata;
`actus --version` reports that commit, and release verification rejects an
archive whose reported commit is not the one it was built from.

## Consequences

- Installing or updating Actus is one command per host, and CI uses the
  same archive users install.
- The installers are part of every release run and fail it when they
  cannot install the archive just built.
- `actus-lang.org` must serve the two scripts from the repository's main
  branch.
- Uninstalling removes `<home>` and the profile line or user `Path` entry
  the installer added.
