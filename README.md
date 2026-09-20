# Actus

Actus is an experimental low-level systems programming language focused on explicit semantic roles, deterministic ownership, and a small compile-time safety core.

The project is in an early alpha stage. The current architecture is being developed around three roles:

- `erg` — an exclusive owned binding that may read, mutate, borrow, move, or be dropped;
- `abs` — a temporary shared read-only borrow;
- `dat` — a linear ownership transfer into a callee.

Actus Alpha supports lexical, non-escaping borrows only. Borrowed values cannot be returned or stored in longer-lived structures. The compiler tracks borrow records, derives the `Frozen` owner state from active borrows, and inserts deterministic cleanup at scope boundaries.

## Project status

The language specification and compiler architecture are under active development. The initial compiler is being bootstrapped in Rust, with a planned C99 emission backend. The current repository contains the project foundation and manifesto; language implementation work is the next milestone.

## Build and run

```sh
cargo run
```

Run the test suite with:

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
