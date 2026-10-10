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
cargo test --all-targets --all-features -- --test-threads=1
```

The repository config defaults `RUST_TEST_THREADS` to `1`. This is deliberate:
native integration fixtures start compiler and linker subprocesses, and an
unbounded libtest fan-out can make those processes compete for the same host
resources. The explicit flag documents the same bounded execution contract in
CI and review commands.

Record exact failures. Do not call a build successful when a later native link,
execution, target, or hardware step has not run.
