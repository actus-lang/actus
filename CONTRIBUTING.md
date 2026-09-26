# Contributing to Actus

Thank you for your interest in Actus. Actus is an experimental open-source systems programming language, and contributions are welcome in compiler implementation, language design, tooling, testing, and documentation.

Before contributing, read:

- [AGENTS.md](AGENTS.md) for engineering and architecture rules;
- [ROADMAP.md](ROADMAP.md) for planned work;
- [MANIFESTO.md](MANIFESTO.md) for the language direction.

## Development model

The repository uses one permanent branch:

```text
main
```

There is intentionally no permanent `develop` branch. `main` must remain buildable, testable, and suitable for release at all times.

Work must be done in a temporary branch created from the latest `main`:

```text
feature/<short-name>
fix/<short-name>
docs/<short-name>
test/<short-name>
refactor/<short-name>
build/<short-name>
```

Direct pushes to `main` are not part of the normal contribution workflow. Changes should be submitted through a pull request.

## Contribution workflow

1. Check existing issues and discussions before starting work.
2. For a new language feature or architectural change, open a design discussion or issue first.
3. Fork the repository if you are an external contributor.
4. Create a temporary branch from the latest `main`.
5. Implement one focused change.
6. Add or update tests and documentation.
7. Run the required local checks.
8. Push the branch and open a pull request.
9. Respond to review and CI feedback.
10. Keep the branch up to date with `main` when requested.

Pull requests are merged with **Squash and merge** so that each logical change becomes one clear commit on `main`. Branches should be deleted after merging.

## Required local checks

Run these commands before opening a pull request:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

To generate the CI-compatible local coverage report, install
`cargo-llvm-cov` and run:

```sh
cargo llvm-cov --all-targets --all-features --lcov --output-path lcov.info
```

Snapshot files are checked by the test suite and are not rewritten during
ordinary tests. When an intentional snapshot update is required, run:

```sh
sh scripts/update_snapshots.sh
```

Review the resulting snapshot diff as part of the same commit.

Lexer and parser fuzzing requires the `cargo-fuzz` subcommand. Run the parser
target with:

```sh
./scripts/fuzz.sh parser
```

Use `./scripts/fuzz.sh lexer` for lexer-only fuzzing. Fuzz artifacts are local
and must not be committed.

Use `./scripts/fuzz.sh delimiters` for malformed block and call delimiters, or
`./scripts/fuzz.sh literals` for malformed strings and comments.

Run the frontend benchmarks with:

```sh
cargo bench --bench frontend
```

Set `ACTUS_BENCH_ITERATIONS` to change the iteration count for local runs.

Enable the repository commit hook once per checkout:

```sh
./scripts/setup-git-hooks.sh
```

## Conventional Commits

Commit messages must use this format:

```text
type(scope): description
```

Allowed types:

```text
feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
```

Examples:

```text
feat(lexer): add identifier tokens
fix(parser): reject unclosed blocks
test(semantic): cover frozen owner mutation
docs(contributing): explain branch workflow
```

Keep commits small and logically focused. Do not mix unrelated refactoring with a feature or bug fix.

## Pull request requirements

Every pull request must:

- describe what changed and why;
- identify relevant tests and their results;
- include regression tests for bug fixes;
- update documentation when behavior or public interfaces change;
- follow the module boundaries and size limits in `AGENTS.md`;
- pass all required CI checks;
- avoid unrelated files and changes.

At this stage, a second maintainer review is not a required merge condition because the project currently has one Core Maintainer. CI remains required. Review requirements will be strengthened when additional maintainers join the project.

## Design changes

Changes to ownership, borrowing, AST structure, compiler phase boundaries, diagnostics, generated-code contracts, or public CLI behavior require explicit design discussion before implementation.

Large architectural decisions should be recorded under `docs/decisions/` and reflected in the manifesto, roadmap, or other affected documentation.

## Code organization

Actus follows this one-directional compiler pipeline:

```text
lexer -> parser -> ast -> semantic -> codegen
```

Reverse dependencies are not allowed. Avoid generic modules such as `utils.rs`, `helpers.rs`, and `misc.rs`. Keep files and functions within the size limits defined in `AGENTS.md`.

## Reporting security issues

Do not open a public issue for an undisclosed security vulnerability. Follow the private reporting process described in [SECURITY.md](SECURITY.md) when it becomes available.

## License and contribution sign-off

By contributing to Actus, you agree that your contributions are submitted for
licensing under the project's MIT OR Apache-2.0 licensing policy, subject to
your authority to grant those rights.

Contributors should sign commits with the Developer Certificate of Origin:

```sh
git commit -s
```

The sign-off records that the contributor has the right to submit the work
under the project contribution terms. It does not transfer copyright or grant
trademark rights. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
