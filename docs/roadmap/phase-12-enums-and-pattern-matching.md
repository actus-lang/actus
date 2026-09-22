# Phase 12: Enums and Pattern Matching

Enums and exhaustive pattern matching are required before self-hosting work and the Arca package ecosystem. They provide the representation and control flow needed for `Option`, `Result`, compiler AST nodes, and structured diagnostics.

## Enum Syntax and AST

- [ ] Define enum declaration grammar and source-span rules.
- [ ] Parse unit variants without payloads.
- [ ] Parse payload variants with named and positional fields.
- [ ] Represent enum declarations and variants in dedicated AST nodes.
- [ ] Reject duplicate enum and variant names.
- [ ] Reject duplicate payload field names.
- [ ] Validate variant construction syntax.

## Tagged Union Layout

- [ ] Define discriminant representation and width rules.
- [ ] Define payload alignment, size, and byte-offset rules.
- [ ] Represent enum layout information for semantic analysis and code generation.
- [ ] Lower discriminant and payload storage through Cranelift.
- [ ] Add layout and alignment tests for unit and payload variants.
- [ ] Add deterministic native layout golden tests.

## `case` Syntax and Parser

- [ ] Define `case` expression and statement grammar.
- [ ] Parse variant patterns with and without payload bindings.
- [ ] Parse literal patterns for primitive values.
- [ ] Parse the `_` wildcard pattern.
- [ ] Reject fallthrough and standalone `break` semantics in `case` branches.
- [ ] Add valid and invalid parser fixtures.

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
