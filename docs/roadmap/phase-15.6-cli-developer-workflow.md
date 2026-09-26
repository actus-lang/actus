# Phase 15.6: CLI Developer Workflow Polish

This phase closes the remaining ergonomics gaps in the local Actus developer
workflow before Phase 16 work expands the compiler and toolchain. It does not
change language semantics, ownership rules, Arca manifest meaning, or the
compiler pipeline.

## Goals

- Make project-root commands work without repetitive file-path arguments.
- Make command failures and help output self-explanatory.
- Keep one-shot execution distinct from explicit file watching.
- Make generated projects start on `main` and explain the next commands.
- Verify the workflow through platform-aware CLI integration tests.

## Gate 1: Help and command contracts

- [x] Add `actus --help` with the complete command list and examples.
- [x] Add `actus -h` as an alias for top-level help.
- [x] Add help output for each subcommand, including options and defaults.
- [x] Return a successful exit status for help requests.
- [x] Keep unknown commands and malformed options on stderr with non-zero exits.
- [x] Keep usage text synchronized with the actual parser implementation.
- [x] Add positive and negative CLI tests for top-level and subcommand help.

## Gate 2: Project-root input discovery

- [x] Make `actus build` discover `Arca.toml` from the current directory and
      select its configured entry source.
- [x] Default to `src/main.act` when the manifest does not override the entry.
- [x] Preserve an explicit `.act` path as a command-line input override.
- [x] Make `actus build --emit exe` work without a positional input path.
- [x] Make `actus build --release` and `actus build --profile <name>` work
      without a positional input path.
- [x] Preserve the existing option form `actus build src/main.act --emit exe`.
- [x] Report a deterministic diagnostic when no manifest or entry source exists.
- [x] Add integration tests from the project root and from nested directories.

## Gate 3: Project creation and Git defaults

- [ ] Make `actus new` initialize the first branch as `main` when it creates a
      repository.
- [ ] Make `actus init` use the same branch policy when it initializes Git.
- [ ] Preserve the no-VCS controls `--no-git` and `--vcs none`.
- [ ] Avoid running `git init` inside an existing parent repository.
- [ ] Ensure generated `.gitignore` contains `/capsula/`, `*.o`, and `*.bin`.
- [ ] Add integration tests for new repositories, existing repositories, and
      disabled VCS initialization.
- [ ] Keep Git failures deterministic and separate from project generation
      failures.

## Gate 4: Post-creation guidance

- [ ] Print the created project path clearly.
- [ ] Print actionable next steps after `actus new` and `actus init`.
- [ ] Include `cd <project>`, `actus check`, `actus build`, and `actus run` in
      the generated guidance.
- [ ] Include the release command when documenting the build workflow.
- [ ] Ensure guidance is printed only after successful project initialization.
- [ ] Add output assertions to CLI integration tests.

## Gate 5: One-shot run experience

- [ ] Keep `actus run` as a one-shot build-and-execute command by default.
- [ ] Continue using a temporary executable without exposing implementation
      paths as the primary user-facing result.
- [ ] Replace the raw temporary `built /tmp/...` message with clear execution
      status output.
- [ ] Preserve the program's stdout and stderr without mixing compiler output
      into either stream unexpectedly.
- [ ] Return the executed program's exit code unchanged where the platform
      supports it.
- [ ] Add integration tests for stdout, stderr, exit status, and build failure.

## Gate 6: Explicit watch mode

- [ ] Add an explicit `actus watch` command rather than making `actus run`
      monitor files implicitly.
- [ ] Watch the project entry and all resolved module source files.
- [ ] Re-run checking/building only after a relevant source or manifest change.
- [ ] Keep diagnostics visible between rebuilds and clear stale diagnostics
      deterministically.
- [ ] Provide a clean interrupt path and return a meaningful process status.
- [ ] Avoid busy polling; use the target platform's filesystem notification
      mechanism or a bounded fallback.
- [ ] Add tests for change detection, ignored files, rebuild failure, and exit.

## Gate 7: Workflow consistency and quality

- [ ] Make `check`, `build`, `run`, `test`, and `fmt` share consistent project
      discovery and profile behavior.
- [ ] Ensure all commands respect `Arca.toml` source roots, targets, and entry
      contracts.
- [ ] Update the CLI documentation and Chapter 02 after implementation.
- [ ] Add regression coverage for Linux, macOS, and Windows path behavior.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run `scripts/check_source_limits.sh`.

## Completion Criteria

Phase 15.6 is complete when a user can run the following from a newly created
project without supplying `src/main.act` manually:

```text
actus new hello
cd hello
actus check
actus build --emit exe
actus run
```

The project starts on `main`, the commands provide accurate guidance, one-shot
execution has clear output, and file watching is available only through the
explicit watch command.
