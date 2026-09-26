# ADR-0021: Single-Origin Views and Caller-Scope Borrow Propagation

- Status: Accepted
- Date: 2026-09-26
- Scope: `abs` returns, view provenance, and caller-owned borrow lifetimes

## Implementation status

Gates 0 through 5 are implemented. Gate 6 is implemented for the current
eligible view representation, `abs Buffer`: its native value is the existing
`BufferHandle` pointer, so returning a view is a pointer return with no copy
and no owned cleanup. The handle points to the runtime's `ActusBuffer`, which
contains the data pointer, length, and capacity. A future dedicated `Slice`
type may define a separate pointer-and-length ABI; it is not part of this
implementation.

Gate 7 is complete for the implemented `abs Buffer` contract. Native
execution, semantic rejection, cleanup planning, and source-limit checks are
covered by the repository quality gates.

## Context

Actus needs zero-copy views such as slices without introducing lifetime
annotations. A view returned from a verb cannot be represented as an ordinary
borrow owned by the callee's lexical scope: that scope ends before the caller
uses the result.

The compiler therefore needs a rule that identifies the source owner and moves
the resulting borrow record into the caller's scope. The rule must remain
simple enough to be checked locally and must reject ambiguous origins.

## Decision

An `abs` return is a non-owning view whose source is exactly one `abs` input
parameter. The return syntax is:

```actus
verb sub_slice(
    abs input: Buffer,
    start: Usize,
    len: Usize
) -> abs Slice {
    return input.raw_slice(start, len);
}
```

The single-origin rule applies to ordinary verbs and methods. A method receiver
declared as `abs self` counts as the one `abs` source parameter:

```actus
verb sub_slice(abs self: Buffer, start: Usize, len: Usize) -> abs Slice {
    return self.raw_slice(start, len);
}
```

The rule is based on the declared access role, not on the parameter name.

## Single Origin Invariant

An `abs` return is valid only when all of the following hold:

1. The verb has exactly one `abs` input parameter, including `abs self`.
2. The returned expression is that parameter or a supported derivation from it.
3. The derivation has one statically known source origin.
4. The source owner remains live for the returned view's caller scope.
5. The returned value is never treated as an owned resource.

Scalar parameters do not count as view origins. Thus a verb with one `abs`
buffer and scalar indexes remains valid.

Two or more `abs` parameters make an unqualified `abs` return invalid:

```actus
verb choose(abs left: Buffer, abs right: Buffer) -> abs Slice {
    return left.raw_slice(0, 1);
}
```

Selecting an origin from multiple inputs requires a future explicit origin
contract and is outside this decision.

## Syntactic provenance

The Alpha implementation uses a bounded provenance model rather than a general
dataflow graph:

```text
Origin:
  None
  AbsParameter(parameter_index)
  Derived(AbsParameter(parameter_index))
  Binding(binding_index)
  Unknown
  Multiple(origins)
```

The following forms are valid when they preserve the one origin:

```actus
return input;
return input.raw_slice(start, len);
return input.field;
```

The following form is rejected:

```actus
verb invalid(abs input: Buffer) -> abs Slice {
    erg temporary = Buffer[100];
    return temporary.raw_slice(0, 10);
}
```

An unknown call, a value derived from a local owner, or a combination of two
different view sources produces `Unknown` or multiple origins and cannot be
returned as `abs`.

The set of origin-preserving operations must be explicit in the semantic
model. A method is not origin-preserving merely because its receiver has an
`abs` role; its declaration or intrinsic contract must say that it returns a
view derived from that receiver.

## Caller-scope propagation

For a call such as:

```actus
verb process() {
    erg buffer = Buffer[1024];
    {
        abs view = sub_slice(
            input: abs buffer,
            start: 0,
            len: 10,
        );
        inspect(view);
    }
}
```

the semantic state is:

```text
before call:  buffer = Active + Mutable
during view:  buffer = Active + Frozen
               view = non-owning, origin = buffer
after scope:  buffer = Active + Mutable
```

The callee reports an origin contract. It does not leave a borrow record in
the callee's scope. The caller creates the borrow record for the returned view,
and the caller's scope owns the corresponding end-borrow action.

In the current implementation, the callee's `abs` return contract is stored in
its signature. At the call site, the argument origin is resolved to the
caller's owner binding. That `Binding` origin creates the caller borrow record
and freezes the binding until the record is ended.

## Ownership and cleanup rules

An `abs` view:

- does not transfer ownership;
- does not receive an owned drop action;
- cannot be passed to `dat`;
- cannot be passed to `ins`;
- prevents moving or dropping its source owner while it is live;
- ends its borrow at the enclosing caller scope;
- thaws the source owner only after all derived views from that source end.

The source owner cannot be destroyed while a returned view is live:

```actus
abs view = sub_slice(input: abs buffer, start: 0, len: 10);
drop(buffer); // rejected: the view still refers to buffer
```

Nested views must preserve the same root origin. A view derived from a view is
not a new independent owner and does not create a second cleanup path.

## Type and AST obligations

The type representation must distinguish an access-qualified return from an
ordinary type:

```text
ReturnType:
  access = Owned | Abs
  type   = TypeName
  span   = source span
```

The semantic model must retain view origin metadata sufficient to connect a
caller binding, its borrow record, and the returned view. The native type
layout of a view must be specified independently from ownership semantics. A
view may use a pointer-and-length representation, but its exact ABI is a
separate implementation contract.

## Diagnostics

Implementations must provide stable diagnostics for at least:

- an `abs` return with zero `abs` source parameters;
- an `abs` return with multiple possible source parameters;
- a view derived from a local owner;
- an unknown-origin return expression;
- moving or dropping a source owner while a derived view is live;
- returning an owned value where an `abs` view is declared;
- using a returned view after its caller scope ends.

## Invariants

- Every valid `abs` return has exactly one statically known origin.
- The origin is one declared `abs` parameter, including `abs self`.
- A returned view never owns its source and never receives owned cleanup.
- The borrow record for a returned view belongs to the caller's scope.
- A source owner cannot move or drop while a derived view is live.
- All nested derived views retain the same root origin.
- An unknown or multiple origin is rejected at semantic analysis time.
- View cleanup ends the borrow exactly once and restores source access only when safe.

## Deferred scope

This decision does not define multiple-origin return syntax, explicit origin
annotations, mutable returned views, shared ownership, or escaping views whose
source is not a lexical caller owner. Those features require separate semantic
contracts.

## Consequences

Actus can return zero-copy views without lifetime spelling while retaining a
small, deterministic proof obligation. The compiler tracks one origin and
places the resulting borrow in the caller's lexical scope. The restriction on
multiple origins intentionally favors a decidable rule over general view
composition.
