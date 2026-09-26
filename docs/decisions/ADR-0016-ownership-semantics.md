# ADR-0016: Ownership and Access Semantics

## Status

Accepted. Phase 14 now implements and verifies the ownership/access state
model, case branch isolation, branch joins, nested partial moves, pattern
guards, and native guard branching. Later ownership extensions are specified
by ADR-0020 and ADR-0021.

## Context

Actus uses `erg`, `abs`, `dat`, and `ins` as its ownership vocabulary. Existing
ownership checking also needs to model temporary read-only access, partial
moves, pattern matching, control-flow exits, and deterministic cleanup.

A single state enum is not sufficient: `Frozen` describes an access/aliasing
condition, while `Moved` and `PartiallyMoved` describe ownership. These are
independent dimensions and must remain independent in the semantic model.

## Decision

### Ownership and access dimensions

Every tracked resource has two states:

```text
Ownership: Active | Moved | PartiallyMoved | Dropped
Access:    Mutable | Frozen
```

`Frozen` never replaces an ownership state. An active owner may temporarily
become frozen while an `abs` view is live, then return to mutable access when
the lexical borrow scope ends. A moved, partially moved, or dropped value
cannot be made mutable by changing its access state.

### Role behavior

- `erg` establishes or retains the active owner and may mutate it.
- `abs` creates a temporary read-only view without transferring ownership.
- `dat` transfers ownership to exactly one destination; the source becomes
  `Moved` and receives no cleanup for the transferred resource.
- `ins` creates an exclusive call-scope loan. The owner becomes `Suspended`
  during the call and returns to `Active + Mutable` afterward.

An `erg` owner or a received `dat` value may create a temporary `abs` view:

```text
Active + Mutable -> Active + Frozen -> Active + Mutable
```

This remains the rule for temporary `abs` access. Exclusive mutable borrowing
is specified separately by ADR-0020; advanced reborrowing chains and lifetime
polymorphism remain deferred.

### `case abs` lifecycle

`case abs subject` performs a lexical inspection without consuming `subject`.
The subject must be readable and is temporarily frozen for the case scope.
Variant payload bindings are read-only `abs` bindings. Every branch starts
from the same subject snapshot, so a branch-local ownership or access change
cannot leak into another alternative. At the case boundary, temporary access
is released and an active owner returns to mutable access.

### `case dat` lifecycle

`case dat subject` consumes the subject before branch evaluation. Moving the
complete value marks it `Moved`; moving nested payload fields records precise
paths such as `inner.first` and marks the root `PartiallyMoved`. Overlapping
paths are rejected. Every branch starts from the same consumed snapshot and a
join is valid only when all reachable paths have compatible ownership and
access states.

Unmoved owned payloads are cleaned up in deterministic reverse declaration
order. Nested struct cleanup preserves the hierarchy and skips exactly the
paths already transferred, preventing leaks and double drops.

### Pattern guards

A guard has the form `Pattern if condition => body`. The condition must have
type `Bool` and may inspect bindings only through temporary read-only access.
Calls or expressions that could transfer ownership are rejected. Guard
evaluation is isolated from branch body state; a failed guard rolls back its
temporary access and continues with the next pattern without consuming a
resource.

Pattern guards do not silently create ambiguous ownership. They cannot move or
mutate resources while deciding whether a branch is applicable.

### Exit and failure behavior

Every return, break, continue, branch join, and error-propagation path must
resolve its ownership state before leaving a scope. Cleanup is emitted exactly
once for each remaining owned resource.

For future structured tasks, `act(process, dat payload)` follows this rule:

- successful spawn transfers ownership to the task handle;
- a spawn failure before acceptance leaves ownership with the caller;
- a runtime that accepts ownership must also accept responsibility for cleanup.

The selected behavior must be represented by the task API result rather than
left as an implicit runtime convention.

## Invariants

After the ownership gate is complete, the compiler must prove:

1. Every owner has exactly one ownership path.
2. Every move invalidates the source binding appropriately.
3. Partial moves produce a deterministic cleanup state.
4. Every control-flow exit resolves ownership.
5. An `abs` view cannot outlive its owner.
6. Frozen access does not permanently consume an active owner.
7. A `dat` transfer has exactly one destination owner.
8. Cleanup occurs exactly once.
9. Failed operations do not create ambiguous ownership.
10. Unsafe code cannot bypass ownership without crossing an explicit `unsafe`
    boundary.

## Verification gate

The implementation gate is:

```text
Prerequisites -> Implemented Scope -> Verification -> Invariant -> Deferral
```

Verification must include transition-matrix tests, negative semantic tests,
nested pattern tests, partial-move tests, early-exit cleanup tests, temporary
`abs` tests, and task-transfer failure tests when task support is implemented.

The following remain explicitly deferred: mutable borrowing, reborrowing
chains, advanced lifetime polymorphism, and shared mutable ownership.

## Consequences

The model keeps Actus simpler than a general lifetime calculus while making
ownership and access states independently checkable. It supports deterministic
cleanup and embedded-oriented owner-centric mutation. APIs that require a
mutable view into a subrange must first demonstrate semantic pressure before
Actus adds another borrowing mechanism.
