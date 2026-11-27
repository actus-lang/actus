# ADR-0027: Deterministic RAII Scope Drop Semantics

- Status: Accepted
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

## Exit lists and value cleanup

[ADR-0125](ADR-0125-static-cleanup-at-every-exit.md) specifies this decision
in detail: every exit (scope end, `return`, `break`, `continue`, `?`) releases
exactly the owners live there, as one list the analyzer computes; `erg` and
`dat` parameters are owners of the verb's scope; and the cleanup of a value
runs its own `Drop` first, then its owned fields in reverse declaration order,
an enum by its active variant and an array last element first.

## Lowering contract

The semantic analyzer produces an ordered cleanup plan for every exit. Code
generation emits the corresponding native release calls on that exit edge and
derives nothing itself (ADR-0125). Cleanup is not a
background task, does not require a collector thread, and does not allocate a
drop record at runtime.

## Constraints and verification

- [ ] Define resource cleanup contracts for `std::fs::File` and sockets.
- [x] Verify LIFO cleanup across nested blocks and early returns.
- [x] Reject double drop, drop of a moved owner, and drop of a live view.
- [x] Verify cleanup on `?`, `break`, and `continue` paths.
- [x] Add native tests proving exactly-once release.

This ADR defines lexical resource cleanup; it does not define finalization for
non-owned values or a general destructor language feature.
