# ADR-0059: Ownership-Safe Value Reuse and Fixed-Slot Selection

- **Status:** Accepted for Gate 32.1
- **Date:** 2026-10-04
- **Decision owners:** Actus language and compiler maintainers

## Context

Actus makes ownership roles explicit at every binding and call boundary. This
is correct for aggregates and mutable state, but small scalar values become
awkward when the same read-only value must be passed to several calls. A
caller currently has to manufacture expression-level copies, often with
identity arithmetic such as `value + 0u32`, when a callee accepts an owning
`erg` parameter. That obscures intent and produces repetitive source code.

The same problem appears when a fixed-layout type exposes several named slots.
Code that evaluates all slots must manually duplicate the same operation for
each field. A loop or a small accessor should express the iteration, while the
compiler must preserve bounds, ownership, layout, and native address rules.

The language needs a deliberate solution that keeps ownership explicit and
does not silently turn every scalar into an implicit copyable value.

## Decision

Actus will add an explicit, compiler-checked value reuse contract for eligible
small scalar types and a first-class bounded fixed-slot access pattern.

The implementation must provide:

1. an explicit copy or clone operation for values whose type is declared safe
   to duplicate;
2. read-only scalar parameters that can be declared with `abs` without
   requiring an artificial ownership transfer at the call site;
3. a bounded accessor or equivalent compiler-supported lowering for selecting
   one field from a fixed slot set by an integer index;
4. native lowering that preserves the declared aggregate layout and emits
   bounds checks according to the selected safety profile;
5. diagnostics when a value or aggregate is not eligible for the requested
   reuse operation.

The compiler must not infer a deep copy for an owning aggregate, a resource
handle, a buffer, or any type with a cleanup obligation. A value reuse
operation is valid only when the type contract proves that duplication is
safe and deterministic.

## Ownership contract

- `erg` remains an owning mutable binding and a call using `erg` may move it.
- `abs` remains a read-only view and does not consume the caller binding.
- `dat` remains an ownership transfer and never becomes copyable by default.
- `ins` remains a scoped exclusive loan and does not create a second owner.
- Explicit scalar reuse produces a new value only for types approved by the
  compiler's copy contract.
- Any unsupported reuse attempt is rejected during semantic analysis.

## Fixed-slot access contract

The compiler may lower a fixed-slot accessor to direct field address
selection, but it must retain:

- the declared slot count;
- the element type and ownership role;
- the packed layout and alignment contract;
- an explicit out-of-range behavior;
- deterministic native code generation.

The accessor must not expose raw operating-system pointers or allow a caller
to bypass the ownership role of the selected field.

## Alternatives considered

### Identity arithmetic as copying

Rejected. It is type-specific, visually misleading, and can fail for values
where arithmetic is not defined. It also hides the ownership decision from
the reader.

### Implicit copy for every scalar

Rejected. It would weaken the language's explicit ownership model and make
future resource-like scalar types unsafe by default.

### Manual source unrolling

Rejected as a language-facing solution. Generated or compiler-lowered
unrolling may be an implementation detail, but users should not have to
duplicate every fixed slot operation by hand.

### Raw pointer based indexing

Rejected. It would bypass layout safety, bounds checks, and the position
independence guarantees of Actus aggregates.

## Acceptance evidence

The implementation is accepted only when all roadmap gates pass, including
accepted and rejected semantic fixtures, native executable coverage, layout
and bounds checks, ownership diagnostics, and a source-level example that
uses explicit reuse without identity arithmetic or manual repeated calls.

## Consequences

The feature adds a small amount of type metadata and semantic checking. In
exchange, Actus source can express repeated read-only scalar use and bounded
slot traversal directly, while aggregate ownership and cleanup remain
fail-closed.

## Gate 32.1 implementation evidence

The first accepted form is:

```act
erg repeated: u32 = copy(value: abs value);
```

The compiler accepts `Int`, `Bool`, and fixed-width integer values. It rejects
calls without an explicit `abs` role and rejects cleanup-bearing aggregates.
Semantic coverage is in `tests/semantic_intrinsics.rs`; native executable
coverage is in `tests/arrays_cli.rs`.
