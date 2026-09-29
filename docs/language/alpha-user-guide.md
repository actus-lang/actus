# Actus Alpha User Guide

This guide describes the implemented Alpha workflow. It does not describe
planned syntax or extension-platform features.

## 1. Create the package

An Alpha package has one `Actus.toml` manifest. The language edition is
explicitly `alpha`, and hosted applications use a `main` entry verb.

```toml
[package]
name = "my_program"
version = "0.1.0"
edition = "alpha"
entry = "main"

[build]
target = "host"
profile = "debug"
```

Standard-library modules are dependencies with manifest-relative paths:

```toml
[dependencies]
io = { path = "library/std" }
fs = { path = "library/std" }
path = { path = "library/std" }
```

Run `actus lock` after changing dependencies. `actus lock --check` verifies
that the committed `Actus.lock` matches the manifest and package checksums.

## 2. Ownership at the call site

Roles are semantic contracts, not comments:

```act
verb transform(ins buffer: Buffer) -> Result[Int, IoError] {
    return write(buffer: abs buffer);
}

verb consume(dat input: Buffer) -> Int {
    return input.length;
}

verb inspect(abs input: Buffer) -> Int {
    return input.length;
}

verb main() -> Int {
    erg buffer = Buffer[128];
    erg written = transform(buffer: ins buffer)?;
    inspect(input: abs buffer);
    consume(input: dat buffer);
    return written;
}
```

`erg` owns mutable storage, `abs` reads without consuming it, `ins` loans
exclusive mutation for the call and restores the caller, and `dat` transfers
terminal ownership. Borrowed views cannot escape their source scope. The
compiler rejects use-after-move, mutation through a frozen view, aliased
exclusive loans, and double cleanup before native code generation.

## 3. Standard-library errors and cleanup

Fallible `std::io`, `std::fs`, and `std::path` operations return typed
`Result` or `Option` values. Runtime and operating-system statuses do not form
the application API. Use `?` when the enclosing verb returns a compatible
error:

```act
verb read_file(abs path: Path) -> Result[Buffer, IoError] {
    erg file = file_open(path: abs path)?;
    erg contents = read_to_bytes(path: abs path)?;
    drop(file);
    return Result[Buffer, IoError].Ok(contents);
}
```

Owned files, buffers, and paths are cleaned deterministically in reverse
declaration order. Early `return`, `break`, `continue`, and `?` unwind the
affected scopes. The native evidence is covered by `tests/fs.rs`,
`tests/std_io.rs`, `tests/buffer_cli.rs`, `tests/cli.rs`, and the Phase 19
application tests.

## 4. Modules and facades

Imports resolve through the package graph and a module's canonical facade.
For a directory module, the facade has the directory's name:

```text
library/std/src/io/io.act
library/std/src/io/reader.act
library/std/src/io/writer.act
```

The facade exposes sibling declarations with extensionless `open` entries.
External code can use only declarations exposed by that facade; sibling files
share the internal module scope. Missing facades, unknown siblings, duplicate
declarations, and direct facade bypasses produce deterministic diagnostics.

## 5. Check, build, run, test, format, and watch

From the package root, the verified hosted workflow is:

```sh
cargo run --bin actus -- check examples/hello.act --strict
cargo run --bin actus -- build examples/hello.act --strict --emit exe -o target/hello
cargo run --bin actus -- run examples/console_application.act
cargo run --bin actus -- test --strict
cargo run --bin actus -- fmt --check
cargo run --bin actus -- watch --once
```

`check` validates without emitting native code. `build` emits an object or
executable, `run` removes its temporary executable after execution, `test`
discovers Actus `meta test` verbs, `fmt --check` rejects formatting drift, and
`watch --once` performs one deterministic change check. `--strict` is accepted
by `check`, `build`, and `test`; it is not an option for `run`, `fmt`, or
`watch`.

The real application workflow and exact output/exit-code assertions are in
`tests/applications.rs` and `docs/language/alpha-application-workflow.md`.
The CLI command and rejection matrix is in `tests/cli_workflow.rs` and
`tests/cli_strict.rs`.

## 6. Supported Alpha boundary

The hosted Alpha target is selected with `target = "host"`. The compiler
derives the host-native executable format, path conventions, and linker
contract. Linux application evidence is recorded in Phase 19; macOS and
Windows application evidence is supplied by their GitHub Actions runners.
Bare-metal substitutions and unavailable host services are explicitly
deferred until a target-specific contract is accepted.

The current Alpha language surface includes structs, enums, generic static
dispatch, ownership roles, deterministic cleanup, typed I/O and filesystem
APIs, paths, bounded arrays, packed layouts, checked casts, relational,
logical, equality, remainder, bitwise, and shift operators. Unsupported or
experimental behavior must not be inferred from this list; consult the
corresponding ADR and tests.

## 7. Accepted and rejected evidence

Accepted native examples live under `examples/`. Positive and negative
compiler behavior is covered by the organized suites under `tests/semantic/`,
`tests/codegen/`, `tests/cli*.rs`, `tests/fs.rs`, `tests/std_io.rs`, and
`tests/frontend_*`. Standard-library Actus fixtures live under
`tests/library/` and are discovered as `meta test` sources; they do not contain
Rust implementation logic.

Every documented command in this guide has a corresponding CLI, application,
or conformance test. A command or language guarantee must not be added here
until its implementation and evidence exist.
