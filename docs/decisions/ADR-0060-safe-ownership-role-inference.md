# ADR-0060: Safe Ownership Role Inference

## Status

Accepted for Phase 33 Gate 33.1.

## Context

Actus currently requires a role at call sites even when the role is already
uniquely determined by the binding and the callee signature. Repeating
`abs` and `ins` in every call makes ownership-safe code harder to read without
adding information in those cases.

Role inference must not become implicit ownership transfer. A wrong inference
could change cleanup, borrowing, or caller binding state, so the compiler must
only infer a role when the binding role and the call contract identify exactly
one safe result.

## Decision

The first inference profile applies only to static calls whose resolved
parameter role is `abs` or `ins`:

1. An omitted role for an `abs` parameter is inferred only when the expression
   is a readable `abs` binding and passes the existing read-only validation.
2. An omitted role for an `ins` parameter is inferred only when the expression
   is an `ins` binding with an active mutable ownership state and passes the
   existing exclusive-loan validation.
3. The compiler never infers `erg` or `dat`.
4. The compiler never infers through dynamic dispatch, unresolved calls,
   external ABI boundaries, generic ambiguity, aggregates, buffers, resources,
   field mutation, or expressions with more than one possible ownership root.
5. An explicit role always wins and is preserved with its source span.
6. If an omitted role cannot be proven by this profile, existing argument-role
   validation rejects the call. The caller must write the explicit role.

The AST keeps the parsed role and role span. Semantic analysis creates a
resolved argument view for validation and records whether each call argument
was explicit or inferred in the semantic model. Native lowering consumes the
resolved call contract and does not perform role inference itself.

## Source and tooling contract

- The formatter preserves explicit roles and preserves omitted roles as
  omitted; it does not invent source text for compiler-inferred roles.
- Semantic model consumers and LSP hover expose the resolved role and whether
  it was explicit or inferred.
- Diagnostics retain the original argument span. An explicit role is reported
  at its role span when available; an inferred role is reported at the
  argument expression span.
- Resolution is deterministic and depends only on the static signature,
  binding role, ownership state, access state, and expression shape.

## Rejected alternatives

- Inferring from the apparent type alone: types do not determine ownership.
- Inferring `erg` or `dat`: these roles can mutate or consume the caller.
- Inferring from mutable aggregates or buffers: field and cleanup state can
  make the ownership result non-unique.
- Performing inference during native lowering: semantic ownership must be
  complete before code generation.
- Rewriting the source AST: source spans and explicit syntax would be lost.

## Acceptance evidence

- Scalar `abs` calls compile with the role omitted.
- Exclusive `ins` calls compile with the role omitted and resume the owner.
- Unsafe, ambiguous, aggregate, buffer, resource, `erg`, and `dat` cases are
  rejected or require an explicit role.
- Branch joins, early returns, loops, and cleanup behavior remain unchanged.
- Formatter output is stable, LSP hover reports the resolved role, and repeated
  object and executable builds produce deterministic results.
