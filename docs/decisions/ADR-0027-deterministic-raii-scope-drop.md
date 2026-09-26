# ADR-0027: Deterministic RAII Scope Drop Semantics

- Status: Proposed
- Date: 2026-09-26
- Scope: lexical cleanup and owned resource destruction

## Context

Files, sockets, device handles, and other operating-system resources require
exactly one release. Actus has no garbage collector and must remain suitable
for bounded and freestanding environments. Cleanup must therefore be a
compiler-proven property of lexical ownership rather than a best-effort
runtime convention.

## Decision

When an `erg` binding owns a resource with a registered cleanup contract, the
compiler schedules its drop at the end of the binding's lexical scope. Drops
execute in strict last-in, first-out order. The same cleanup plan is used for
normal scope exit, explicit return, loop control, and error propagation.

A resource type declares its cleanup contract through an explicit performance
of the standard `Drop` role:

```actus
perform Drop for File {
    verb drop(ins self: File) { /* release the OS handle */ }
}
```

The equivalent contract may be expressed directly as `verb drop(ins self:
Type)`. In both forms, the declaration is a language-level cleanup contract;
the compiler does not infer a destructor from a field name or a host-language
implementation detail.

The cleanup action is emitted exactly once for every live owned resource.
`abs` views and `ins` loans are non-owning and never receive an owned drop.
`dat` transfers move the cleanup responsibility to the destination; the
source is removed from the caller's cleanup plan. Partially moved structures
drop only the fields that remain owned.

Scope cleanup respects access state. An owner cannot be dropped while it is
`Frozen` by a live view or `Suspended` by an active `ins` loan. The compiler
reports the conflict at the operation that would violate the state machine.

## Lowering contract

The semantic analyzer produces an ordered cleanup plan. Code generation emits
the corresponding native release calls on every exit edge. Cleanup is not a
background task, does not require a collector thread, and does not allocate a
drop record at runtime.

## Constraints and verification

- [ ] Define resource cleanup contracts for `std::fs::File` and sockets.
- [ ] Verify LIFO cleanup across nested blocks and early returns.
- [ ] Reject double drop, drop of a moved owner, and drop of a live view.
- [ ] Verify cleanup on `?`, `break`, and `continue` paths.
- [ ] Add native tests proving exactly-once release.

This ADR defines lexical resource cleanup; it does not define finalization for
non-owned values or a general destructor language feature.
