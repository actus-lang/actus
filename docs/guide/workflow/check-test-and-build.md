# Check, test, and build

Use the narrowest command that answers the current question, then run the full
required checks before handoff.

```sh
actus check --strict
actus test --strict
actus build --strict --emit exe -o /tmp/program
```

Use the package manifest from the project root. Check validates source and
configuration, test runs the package's tests, and build emits native output for
the selected target and runtime.

For repository compiler work, also use the repository's Rust checks:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Record exact failures. Do not call a build successful when a later native link,
execution, target, or hardware step has not run.
