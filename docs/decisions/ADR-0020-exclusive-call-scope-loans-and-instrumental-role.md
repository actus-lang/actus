# ADR-0020: Exclusive Call-Scope Loans and the Instrumental Role

- Status: Accepted
- Date: 2026-09-26
- Scope: `ins` parameters, explicit call-site loans, and access transitions

## Context

Actus distinguishes ownership transfer (`dat`), immutable inspection (`abs`),
and exclusive ownership (`erg`). Some operations need temporary exclusive
mutation without transferring ownership. Encoding that operation as a `dat`
move forces a helper to consume and return a resource even when the resource
must remain in the caller.

Actus also requires ownership effects to be visible at the call site. A reader
must identify a mutation boundary without opening the callee declaration.

## Decision

Actus introduces the `ins` (instrumental) role for an exclusive, temporary,
call-scoped loan. An `ins` loan grants the callee exclusive mutation access to
an existing owner without transferring ownership.

The declaration form is:

```actus
verb append_timestamp(ins buf: Buffer) {
    buf.append(get_time());
}
```

The call-site form is explicit and uses the existing named-argument shape:

```actus
append_timestamp(buf: ins my_buf);
```

The role before the argument expression is mandatory. The compiler verifies
that the call-site role and the declared parameter role agree. The role is not
an ordinary expression and cannot be stored as a value.

## Access-state model

Ownership and access remain separate dimensions:

```text
Ownership: Active | PartiallyMoved | Moved | Dropped
Access:    Mutable | Frozen | Suspended { loan_id }
```

An `ins` call performs this transition:

```text
Active + Mutable
        |
        | call(buffer: ins owner)
        v
Active + Suspended { loan_id }
        |
        | call returns
        v
Active + Mutable
```

`Suspended` means that the original binding cannot be read, written, moved,
dropped, or borrowed while the exclusive loan is active. The resource remains
owned by the caller throughout the transition.

Each loan receives a deterministic identifier. Diagnostics use that
identifier, the source span, and the callee parameter when reporting a
conflict.

## Loan forwarding and nested helper calls

An active `ins` loan may be forwarded to a helper call. This is sequential
reborrowing, not a second simultaneous access:

```actus
verb write_header(ins buf: Buffer) {
    buf.append(1);
}

verb parse_and_mutate(ins buf: Buffer) {
    write_header(buf: ins buf);
    buf.append(2);
}
```

During the nested call, the outer binding is suspended for the inner call.
When the inner call returns, the outer function regains exclusive access. The
compiler must not treat forwarding as ownership transfer or create two
parallel owners.

The following forms remain invalid because one call creates overlapping loans
from the same root resource:

```actus
bad_verb(target: ins buffer, second: ins buffer);
bad_verb2(target: ins buffer, view: abs buffer);
```

The rule is about simultaneous aliasing within one call, not sequential helper
composition.

## Valid loan sources and restrictions

An `ins` argument must resolve to an active `erg` or `dat` owner, or to a field
whose ownership rules explicitly permit an exclusive field loan. The following
sources are rejected:

- an `abs` binding;
- a frozen owner with a live immutable view;
- a suspended owner;
- a moved or dropped binding;
- an expression with more than one possible root owner.

`ins` is valid for verb parameters and call-site argument annotations. It is
not a local declaration role, struct-field role, `case` mode, or dynamic role.

## Compiler obligations

The semantic analyzer must:

1. validate the declared and call-site roles;
2. resolve the argument's root binding;
3. reject existing frozen, suspended, moved, or dropped access;
4. reject overlapping loans in the same call;
5. create one loan record;
6. enter `Suspended { loan_id }` for the call duration;
7. restore `Active + Mutable` after the call;
8. close the loan exactly once on every analyzed exit path.

The native backend does not infer these rules. `ins` uses the existing native
representation of the parameter type; the role is a compile-time capability,
not a runtime allocation or hidden wrapper.

## Diagnostics

Implementations must provide stable diagnostics for role mismatch, borrowing
an `abs` value as `ins`, using a suspended owner, duplicate loans from one root
resource, simultaneous `ins` and `abs` access, and use after an invalid or
completed loan. Diagnostics should identify the owner, conflicting loan,
callee, parameter, and source span when available.

## Invariants

- An `ins` loan never transfers ownership.
- An `ins` loan cannot escape its call boundary.
- A suspended owner has no readable, writable, movable, droppable, or borrowable access.
- Sequential loan forwarding is valid and restores the outer loan after the helper returns.
- One root resource cannot receive overlapping loans in one call.
- Every loan has exactly one start and one end.
- A completed call restores the owner to its prior live ownership state and mutable access.
- The backend never repairs or weakens a semantic loan decision.

## Deferred scope

This decision does not define returned `abs` views, source provenance, or
caller-scope borrow propagation. Those rules are specified separately in
ADR-0021. It also does not define shared ownership, reentrant mutation, or
escaping mutable views.

## Consequences

Actus can express composable mutating helpers without ownership transfer or
lifetime annotations. Mutation remains visible in source, and the compiler can
model it with a bounded call transition rather than an escaping borrow graph.
The additional call-site annotation requires parser and AST support, but makes
the mutation contract locally inspectable.
