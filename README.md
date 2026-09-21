# Actus

Actus is an experimental low-level systems programming language focused on explicit semantic roles, deterministic ownership, and a small compile-time safety core.

The project is in an early alpha stage. The current architecture is being developed around three roles:

- `erg` — an exclusive owned binding that may read, mutate, borrow, move, or be dropped;
- `abs` — a temporary shared read-only borrow;
- `dat` — a linear ownership transfer into a callee.

Actus Alpha supports lexical, non-escaping borrows only. Borrowed values cannot be returned or stored in longer-lived structures. The compiler tracks borrow records, derives the `Frozen` owner state from active borrows, and inserts deterministic cleanup at scope boundaries.

## Project status

The language specification and compiler architecture are under active development. The compiler is being bootstrapped in Rust and currently emits native object files through a Rust-native Cranelift backend. The native alpha slice supports integer parameters and locals, arithmetic expressions, lexical scopes, loop control flow, and integer returns.

## Build and run

Build the compiler and run the introductory example through the frontend:

```sh
cargo run -- check examples/hello.act
```

Emit a native object file:

```sh
cargo run -- build examples/hello.act --emit obj -o examples/hello.o
```

Build a native executable directly through the system linker:

```sh
cargo run -- build examples/hello.act --emit exe -o examples/hello
./examples/hello
echo $?
```

Build and execute a temporary native binary without keeping the artifact:

```sh
cargo run -- run examples/loop_ssa.act
```

The command returns the Actus program's exit code and removes its temporary executable.

The example returns `42`. Object files and native executables are local build artifacts and are excluded from version control.

### Toolchain configuration

Basic project build settings are read from the root `Arca.toml` manifest. The linker can be overridden without changing source code by setting `ACTUS_LINKER`:

```toml
[package]
name = "my_program"
version = "0.1.0"
entry = "main"
```

```sh
ACTUS_LINKER=clang cargo run -- build examples/hello.act --emit exe -o examples/hello
```

The default linker is `cc`. Hosted executables currently require the configured entry verb to be `main`; custom entry symbols will be supported with a future freestanding/linker-target configuration. The current manifest supports package identity and native backend settings. Full Arca project commands, dependency resolution, and publishing are planned separately. Language semantics, ownership rules, and borrow safety are not configurable project options.

Foreign C declarations must use an explicit unsafe boundary:

```act
unsafe extern "C" verb rand() -> Int;
```

The initial C boundary supports `Int` values and `Buffer` pointers. `abs` and
`dat` are restricted to opaque resource pointers; foreign pointer returns are
owned by the Actus caller. Unsupported layouts are rejected before linking.

Run the complete test suite with:

```sh
cargo test
```

## Design

See [MANIFESTO.md](MANIFESTO.md) for the ownership model, borrow rules, cleanup semantics, syntax direction, and compiler roadmap.

## Open source

Actus is open-source software released under the MIT License. Contributions, design discussions, and implementation feedback are welcome as the language evolves.

## License

Copyright (c) 2026 Actus contributors

Licensed under the MIT License. See [LICENSE](LICENSE) for the full text.
