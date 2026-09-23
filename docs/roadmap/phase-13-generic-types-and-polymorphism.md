# Phase 13: Generic Types and Polymorphism

Generic types are a prerequisite for compiler-defined `Option[T]` and
`Result[T, E]`, reusable AST data structures, and a self-hosted compiler.
This phase must establish one coherent type system before the Arca ecosystem
is expanded.

## Generic Type Representation

- [x] Represent generic parameters and applications in the AST.
- [x] Support generic structs and enums with stable source spans.
- [x] Distinguish type parameters, concrete types, and applied types in the
  type environment.
- [x] Reject duplicate generic parameters during parsing.
- [x] Reject invalid generic arity during semantic validation.

## Constraints and Type Checking

- [ ] Define the constraint syntax and built-in constraint vocabulary.
- [ ] Validate constraints at declaration and instantiation sites.
- [x] Resolve generic field, payload, parameter, and return types.
- [ ] Produce stable diagnostics for failed constraints and type mismatches.
- [x] Reject invalid recursive generic layouts unless indirection is explicit.

## Monomorphization and Layout

- [ ] Monomorphize reachable generic instances deterministically.
- [ ] Cache instances by canonical type arguments and compiler toolchain hash.
- [x] Calculate concrete struct and tagged-union layouts after substitution.
- [ ] Lower generic ownership, borrow, move, and cleanup semantics without
  weakening `erg`, `abs`, or `dat` rules.
- [ ] Add Cranelift layout and native execution golden tests.

## Built-in Result Types

- [ ] Register `Option[T]` as a compiler-provided generic enum.
- [ ] Register `Result[T, E]` as a compiler-provided generic enum.
- [ ] Validate `Some`, `None`, `Ok`, and `Err` construction.
- [ ] Support exhaustive `case abs` and consuming `case dat` matching.
- [ ] Define null-free propagation behavior without exceptions.

## Completion Gates

- [x] Add positive and negative semantic tests for generic declarations and
  applications.
- [ ] Add deterministic monomorphization and cache invalidation tests.
- [ ] Add end-to-end executable tests for `Option` and `Result`.
- [ ] Keep all source and function limits within repository policy.
