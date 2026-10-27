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

## Focused tests and internal fixtures

`actus test` accepts a Rust-style positional substring filter, so a large test
suite does not need to run in full:

```sh
actus test persistence
actus test --module ie::runtime
actus test --file ie/runtime/training/auxiliary/cursor.act
actus test persistence --module ie::runtime --exact
actus test --list
```

The positional filter, `--module`, and `--file` are substring filters by
default. Add `--exact` when the value must match exactly. `--list` discovers
and prints the selected canonical test identities without executing them.

Tests should mirror the source tree. For example,
`tests/ie/runtime/training/auxiliary/cursor.act` is paired with the matching
source path under `src/`, or with that path's canonical facade when the source
module is a directory module. Use `--mirror` to enforce this mapping:

```sh
actus test --mirror --strict
```

When a mirrored file contains a `meta test`, the runner compiles that test
with the corresponding module's internal implementation scope. This permits
testing private helpers without exporting them through a facade or changing
ordinary importer visibility. The friend scope exists only in the temporary
test artifact; it does not make private declarations public.

Package-root test files remain root-scope compatibility fixtures. New nested
test files should use the mirrored layout and should pass the `--mirror` gate.

Publication failure fixtures may declare one test-only fault, for example
`meta fault("flush_failure")`. These faults require a compiler built with the
`test-fault-injection` feature; the normal runtime archive does not contain the
fault symbol or control path. The fault is installed by the test runner for
one child process and is consumed once at the named native filesystem phase.
Fault metadata is for strict publication tests only and is not an application
filesystem API. A normal compiler intentionally reports that a fault-bearing
test requires `--features test-fault-injection`; this is an expected build
boundary, not a test failure.

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

The backend can be selected for one build through the same validated
configuration path used by `Actus.toml`:

```sh
actus build --backend cranelift
actus build --backend llvm
actus run --backend cranelift
actus run --backend llvm
```

The selected backend must be enabled in the compiler build and compatible with
the selected target. The backend name is recorded in native artifact metadata,
so switching backends invalidates a stale artifact instead of reusing it.

Record exact failures. Do not call a build successful when a later native link,
execution, target, or hardware step has not run.
