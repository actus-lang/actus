# Phase 12: Enums and Pattern Matching

Enums and exhaustive pattern matching are required before self-hosting work and the Arca package ecosystem. They provide the representation and control flow needed for `Option`, `Result`, compiler AST nodes, and structured diagnostics.

## Enum Syntax and AST

- [x] Add `enum` and `case` lexer keywords.
- [x] Define enum declaration grammar and source-span rules.
- [x] Parse unit variants without payloads.
- [x] Parse payload variants with named and positional fields.
- [x] Represent enum declarations and variants in dedicated AST nodes.
- [x] Reject duplicate enum and variant names.
- [x] Reject duplicate payload field names.
- [x] Validate variant construction syntax.

## Enum Semantic Validation

- [x] Register enum types in the semantic type environment.
- [x] Validate payload fields against built-in, struct, and enum types.
- [x] Reject direct recursive enum payloads without indirection.
- [x] Validate unit and payload variant construction.
- [x] Validate positional and named payload arguments.
- [x] Emit stable E10xx diagnostics for invalid variant construction.
- [x] Add positive and negative semantic tests for enum validation.

## Tagged Union Layout

- [x] Define discriminant representation and width rules.
- [x] Define payload alignment, size, and byte-offset rules.
- [x] Represent enum layout information for semantic analysis and code generation.
- [x] Lower discriminant and payload storage through Cranelift.
- [x] Add layout and alignment tests for unit and payload variants.
- [x] Add deterministic native layout golden tests.

## `case` Syntax and Parser

- [ ] Define `case` expression and statement grammar.
- [ ] Parse variant patterns with and without payload bindings.
- [ ] Parse literal patterns for primitive values.
- [ ] Parse the `_` wildcard pattern.
- [ ] Reject fallthrough and standalone `break` semantics in `case` branches.
- [ ] Add valid and invalid `case` parser fixtures.
- [x] Add enum lexer and parser unit tests.
- [x] Add duplicate enum and payload-name diagnostics tests.

## Exhaustiveness and Pattern Semantics

- [ ] Implement exhaustive coverage checks for enum variants.
- [ ] Require a wildcard when a pattern set is intentionally open-ended.
- [ ] Reject unreachable and duplicate patterns.
- [ ] Check pattern binding types against variant payload types.
- [ ] Define deterministic evaluation order for guards and branch bodies.
- [ ] Produce stable diagnostics for non-exhaustive and unreachable matches.

## Role-Based Deconstruction

- [ ] Define `case abs` inspection without consuming the matched value.
- [ ] Define `case dat` deconstruction as an ownership-consuming operation.
- [ ] Reject mutation through an `abs` pattern binding.
- [ ] Track moved payload fields after `dat` deconstruction.
- [ ] Apply deterministic cleanup to unmatched and consumed payloads.
- [ ] Add ownership, borrowing, and cleanup regression tests.

## Standard Result Types

- [ ] Define `Option[T]` as a standard enum.
- [ ] Define `Result[T, E]` as a standard enum.
- [ ] Specify null-free propagation behavior for `Option` and `Result`.
- [ ] Add compiler-facing AST and diagnostic examples using enum matching.
- [ ] Add Cranelift execution tests for construction and matching.
