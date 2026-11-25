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
actus test --file ie/runtime/tests/cursor.act
actus test persistence --module ie::runtime --exact
actus test --list
```

The positional filter, `--module`, and `--file` are substring filters by
default. Add `--exact` when the value must match exactly. `--list` discovers
and prints the selected canonical test identities without executing them.

Tests live in two places. A module's tests are the `.act` files of the `tests`
directory next to its facade (`src/ie/runtime/tests/*.act` tests `ie::runtime`);
they are compiled as part of the module and see its private declarations. The
project's tests are the `.act` files directly in `tests/`; they see only the
`open` declarations that facades export. A test is reported as
`<module path>::<verb>` or `tests::<verb>`, and `--module ie::runtime` selects
the tests of one module. All files of one `tests` directory share one namespace,
so they can share helpers; a duplicate name is an error that names both files.

`actus build`, `actus run` and `actus check` ignore every `tests` directory,
and `actus fmt` formats them. A `meta test` verb in a production source file, a
`.act` file in a subdirectory of `tests/`, a `src/tests` directory, a
subdirectory of a `tests` directory, and a `tests/tests.act` facade are layout
errors (`E1143`, `E1144`) that name where the test belongs. See
[`meta test`](../meta-attributes/test.md).

Publication failure fixtures may declare one test-only fault, for example
`meta fault("flush_failure")`. These faults require a compiler built with the
`test-fault-injection` feature; the normal runtime archive does not contain the
fault symbol or control path. The fault is installed by the test runner for
one child process and is consumed once at the named native filesystem phase.
Fault metadata is for strict publication tests only and is not an application
filesystem API. A normal compiler intentionally reports that a fault-bearing
test requires `--features test-fault-injection`; this is an expected build
boundary, not a test failure.

The currently supported fault names are fixed by the compiler and runtime; they
are not free-form labels:

- `staging_open_failure`: opening the staging file fails before any bytes are
  written;
- `write_failure`: the native write fails without reporting a partial count;
- `short_write`: the native write reports that fewer bytes were written;
- `flush_failure`: flushing the staged file fails;
- `close_failure`: closing the staged file fails after the write or flush;
- `cleanup_failure`: the pre-publication failure cleanup itself fails, so the
  root publication error and leftover staging state can be checked;
- `rename_failure`: publishing the staged file fails before replacement;
- `interrupted_before_rename`: the child terminates before the publication
  rename, so the old destination remains authoritative;
- `parent_open_failure`: opening the destination parent for synchronization
  fails after publication;
- `parent_sync_failure`: syncing the destination directory fails after the
  file publication step;
- `interrupted_publication`: the child terminates after the publication rename,
  so the fixture can verify post-rename crash-state recovery.

For example, a test may select exactly one of these values:

```actus
meta fault("short_write")
```

An unknown name, a duplicate `fault` declaration, or more than one fault on a
test is rejected during parsing or semantic test collection. Adding another
fault requires a compiler/runtime change and a corresponding strict fixture;
developers must not invent new names in test files.

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

LLVM is the default backend of the standard Actus compiler build. Therefore
these commands use the same backend when no project manifest overrides
`build.backend`:

```sh
actus build --emit obj
actus build --backend llvm --emit obj
```

Use `build.backend = "cranelift"` in `Actus.toml`, or pass
`--backend cranelift`, when the project explicitly requires the compatibility
backend. The override is validated before emission. If the compiler binary
was built without the requested feature, the command fails and does not switch
to another backend. The same rule applies to `actus run`.

The selected backend must be enabled in the compiler build and compatible with
the selected target. The backend name is recorded in native artifact metadata,
so switching backends invalidates a stale artifact instead of reusing it.
That metadata also includes the target contract, profile, runtime, source-limit
policy, and runtime identity, so changing any of those inputs triggers the
same stale-artifact protection.

Record exact failures. Do not call a build successful when a later native link,
execution, target, or hardware step has not run.
