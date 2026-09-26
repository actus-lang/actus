# ADR-0030: Lexical Scoped Arenas for Zero-Cost Cyclic Allocations

- Status: Proposed
- Date: 2026-09-26
- Scope: bounded arena allocation and lexical bulk release

## Context

Graphs, trees, and other cyclic structures cannot always be represented as a
simple ownership tree. A per-object ownership protocol would make these
structures verbose and can introduce reference-counting or interior-mutability
overhead. Actus still needs deterministic reclamation and must not require a
garbage collector.

## Decision

Actus will provide `Arena[N]` as an owned `erg` resource with a compile-time
capacity or target-defined bounded storage contract. Objects allocated from
the arena are addressed by arena-derived references. `ins` permits exclusive
construction or mutation during a call, while `abs` permits read-only graph
traversal. References cannot outlive the arena's lexical scope and cannot be
stored in an owner with a longer lifetime.

At scope exit, the arena releases its backing storage in one deterministic
bulk operation. Individual objects do not receive independent drop actions
unless their type explicitly declares a required external cleanup contract.
The common case therefore has constant-time arena teardown and no per-object
deallocation walk.

## Allocation and safety contract

Arena capacity exhaustion is a checked operation and returns a typed failure
or is rejected when the capacity is statically known to be insufficient.
Arena references carry provenance tied to one arena root. Combining references
from different arenas into one view is rejected. Moving or dropping the arena
while a derived view is live is rejected by the ownership state machine.

The representation and alignment of arena objects are target contracts. The
arena API does not define a general-purpose global allocator and does not
permit hidden process-wide storage.

## Constraints and verification

- [ ] Define `Arena[N]` layout and target alignment rules.
- [ ] Track arena provenance in semantic analysis.
- [ ] Enforce non-escaping `ins` and `abs` references.
- [ ] Lower teardown to one deterministic bulk release operation.
- [ ] Test cyclic graphs, exhaustion, nested scopes, and cross-arena rejection.
- [ ] Verify bare-metal operation without a host allocator.
