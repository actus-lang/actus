# ADR-0003: Architecture Vocabulary and Compilation Units

- Status: Accepted
- Date: 2026-09-22
- Scope: Actus repository and Actus ecosystem

## Context

Actus needs terminology for its compiler components, source namespaces,
published projects, and standard libraries. Rust's `crate` is an
implementation-specific term and is not part of Actus's user-facing model.

The repository is currently bootstrapped as one Rust package. Splitting the
compiler, standard library, and target support into independently compiled
components is a future step that will be considered after Actus becomes
self-hosting.

## Decision

Actus uses the following canonical vocabulary:

| Term | Meaning |
| --- | --- |
| `package` | A complete Actus project managed and published by Actus. |
| `module` | An importable Actus source namespace. |
| `unit` | An independently compiled Actus component; the closest Actus concept to a Rust crate. |
| `stdlib` | The official standard library distribution. |
| `target` | A compilation platform profile and its ABI, linker, and runtime requirements. |

The term `crate` must not appear in Actus language, Actus, or architecture
documentation unless it is specifically describing Rust bootstrap internals.

## Repository Structure

### Current alpha structure

Until self-hosting, the repository remains a single Rust package with a
modular compiler implementation:

```text
actus/
├── src/
│   ├── ast/
│   ├── cli/
│   ├── codegen/
│   ├── diagnostics/
│   ├── ffi/
│   ├── lexer/
│   ├── parser/
│   ├── runtime/
│   └── semantic/
├── tests/
├── examples/
├── docs/
├── benches/
└── fuzz/
```

This is an implementation boundary, not a promise that every directory is
already an independently compiled `unit`.

### Future self-hosted structure

After Actus can compile and maintain its own compiler, the repository may be
split into independently compiled units:

```text
actus/
├── compiler/
│   └── units/
│       ├── lexer/
│       ├── parser/
│       ├── semantic/
│       └── codegen/
├── stdlib/
│   ├── core/
│   ├── alloc/
│   └── std/
└── targets/
    ├── hosted/
    ├── cortex-m/
    └── riscv/
```

This future layout is descriptive, not an immediate implementation task.

## Naming Rules

- Use `package` for Actus-managed distribution units.
- Use `module` for language-level source imports and namespaces.
- Use `unit` for compiler-level independently compiled components.
- Use `stdlib` for the official library family and `core`, `alloc`, and `std`
  for its layers.
- Use `target` for compilation platform configuration and runtime contracts.
- Keep platform-specific implementation outside the language frontend.

## Consequences

This vocabulary prevents the concepts of published packages, source modules,
and compiler units from being conflated. It also leaves the current alpha
implementation small while providing a stable migration vocabulary for the
self-hosted architecture.
