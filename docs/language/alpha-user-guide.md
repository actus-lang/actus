# Actus Alpha User Guide

This guide describes the implemented Alpha workflow. It does not describe
planned syntax or extension-platform features.

## 1. Create the package

An Alpha package has one `Actus.toml` manifest. The language edition is
explicitly `alpha`, and hosted applications use a `main` entry verb.

To create a new project directory, use:

```sh
actus new my_program
```

Use `actus init` when the directory already exists:

```sh
actus init --no-git
```

`new` and `init` create the initial manifest and `src/main.act`; they do not
compile or publish the project. Git initialization is optional and is never
performed inside an existing parent repository.

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
This also applies to the standard library: `std::io`, `std::fs`, and
`std::path` expose typed Actus wrappers, while their raw `unsafe extern "C"`
bridges remain private implementation declarations. Applications must call the
typed facade operation rather than a runtime bridge symbol.

### 4.1 Nested module trees

For a larger package, a directory may contain child directories. Every child
directory must have its own canonical facade, and the parent facade must open
the child explicitly:

```text
src/control/control.act
src/control/runtime/runtime.act
src/control/runtime/safety.act
src/main.act
```

```act
// src/control/control.act
open runtime;

// src/control/runtime/runtime.act
open safety;
```

The application imports only the parent:

```act
import control;
verb main() -> Int {
    return control_value();
}
```

`runtime.act` is the canonical facade because its name matches its directory.
The parent controls the public boundary: declarations opened by `runtime.act`
become available through `control` only because `control.act` opens `runtime`.
Sibling implementation files share the child module's internal scope, but
private declarations do not cross the facade.

Valid application code uses `import control;` and `import std::io;`. These
forms are invalid because they bypass a parent facade or address a source file:

```act
import control::runtime;
import control::runtime::safety;
import control::runtime::runtime;
```

Use responsibility-specific child directories and keep one stable parent
facade for external consumers. The resolver, semantic analyzer, native linker,
formatter, LSP, and test runner all consume the same hierarchy.

### Package configuration

Packages may reserve `src/config/config.act` as a package-wide configuration
facade:

```text
src/
├── config/
│   ├── config.act
│   └── values.act
└── main.act
```

The facade may open configuration siblings, and normal package modules consume
its public constants with `import config;`. Configuration sources are
compile-time-only: they may expose typed constants and supporting type-level
declarations, but may not import domain/runtime modules or declare verbs.
Private constants remain private to the configuration subtree. `Actus.toml`
continues to own package, build, target, and runtime settings; source
configuration is for typed compile-time policy values only. The formatter,
LSP, semantic checker, and test runner use the same facade boundary.

## 5. Check, build, run, test, format, and watch

After installing an Actus Alpha release, the user-facing workflow is:

```sh
actus new my_program
cd my_program
actus check --strict
actus build --strict --emit exe -o target/hello
actus run
actus test --strict
actus fmt --check
actus watch --once
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

The compiler-backed editor contract is documented in
`docs/language/lsp-protocol.md`. It defines the `actus lsp` lifecycle,
versioned response metadata, result states, cancellation behavior, and the
cross-platform URI/source-location rules used by all supported clients.

### Repository contributor bootstrap

The repository currently bootstraps the compiler from Rust. Contributors who
are working from a checkout may use the equivalent form below:

```sh
cargo run --bin actus -- check examples/hello.act --strict
```

This command is development evidence, not an installation requirement for
Alpha users. Release installation and platform-specific binary artifacts are
Gate 19.6 deliverables and are not claimed by this guide until their checks
pass.

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
