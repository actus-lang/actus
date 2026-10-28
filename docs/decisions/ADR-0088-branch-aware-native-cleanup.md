# ADR-0088: Branch-aware native cleanup lowering

## Status

Accepted. Gate 3.1 introduces branch-local cleanup value tracking for native
lowering. Dominance validation, backend parity, and native leak coverage remain
follow-up acceptance work in the same gate.

## Context

Actus cleanup plans are defined by lexical ownership scopes, but native code is
lowered into basic blocks with branch-local SSA values. Resolving a cleanup
operand only from the latest mutable binding name can select a value produced
in a different branch. Cranelift then rejects the generated call because its
operand does not dominate the cleanup block.

This is especially visible when an owned value is bound by a `case` payload and
then crosses a loop, an early return, or a continuation edge. The source-level
ownership contract remains valid; the lowering layer must preserve the active
value for each block.

## Decision

The neutral native body lowerer records the active binding-to-SSA mapping when
each block is entered. Distributed scope cleanup uses the snapshot belonging to
the block receiving the cleanup. Case-scope exit cleanup uses the mapping
captured immediately after payload binding. A structured transfer into a nested
control-flow region does not run the enclosing scope cleanup before the nested
region; cleanup is emitted on the appropriate return, failure, or continuation
path instead.

The implementation does not change ownership roles, public resource APIs,
serialized layouts, or runtime behavior. It also does not replace cleanup with
an untyped wrapper or suppress a backend verifier failure.

## Verification

- The neutral lowering regression covers a user-defined `Drop` performance
  across a `case` branch and loop control flow.
- A second regression covers case fallthrough and verifies that lowering
  remains valid.
- The standard-library directory fixture covers implicit-close early return,
  repeated EOF, closed-handle reuse, and explicit close.
- Strict Actus checking and the compiler library test suite must pass before
  the remaining Gate 3.1 checkboxes are closed.

## Deferred acceptance work

- Add explicit dominance validation before backend emission.
- Verify equivalent behavior across every supported native backend.
- Add native leak, double-close, use-after-close, and early-return evidence.
