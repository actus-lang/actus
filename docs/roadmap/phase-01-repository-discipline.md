# Phase 1: Repository and Project Discipline

- [x] Protect the `main` branch from direct pushes.
- [x] Require pull requests for changes to `main`.
- [x] Require successful CI checks before merging.
- [ ] Require at least one code review for pull requests. (Deferred until a second maintainer joins.)
- [x] Add a pull request template.
- [x] Define and enforce the Conventional Commits format.
- [x] Add a commit-message checker.
- [x] Add local `pre-commit` checks.
- [x] Add local `pre-push` checks.
- [x] Add a pinned Rust toolchain with `rust-toolchain.toml`.
- [x] Configure `rustfmt` and enforce formatting in CI.
- [x] Configure Clippy and treat warnings as errors in CI.
- [x] Run `cargo check`, `cargo test`, and `cargo clippy` in GitHub Actions.
- [x] Add dependency auditing to CI.
- [x] Add dependency and license policy checks.
- [x] Add secret scanning to the repository workflow.
