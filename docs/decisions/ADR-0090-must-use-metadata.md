# ADR-0090: Explicit must-use results

- Status: Accepted
- Date: 2026-10-29
- Scope: `meta must_use(...)`, call-site semantic validation, diagnostics,
  formatter, LSP, and imported verb signatures

## Context

Fallible or status-producing operations can carry ownership, cleanup, or
protocol obligations. A caller that writes a call as a standalone expression
can silently discard that contract. The language needs a declaration-level
way to require an explicit call-site use without adding hidden runtime work.

## Decision

`meta must_use("reason")` is a compiler-recognized declaration attribute. The
reason is a required non-empty string. The compiler records it in the verb
signature and rejects a call whose result is discarded as a standalone
statement. A value-consuming context such as an operator, argument,
field/index receiver, return, or `?` propagation counts as an explicit use;
grouping alone does not.

Use is explicit through an assignment, return, argument, expression
composition, or other construct that consumes the call value. The attribute
does not copy values, alter ownership roles, insert cleanup, or introduce a
runtime representation.

The contract propagates through imported signatures, external declarations,
generic substitutions, and role/dynamic dispatch. Role methods may declare the
contract, and every performance implementation must preserve it exactly. It
is not permitted on declarations without a return value, and duplicate
declarations are rejected.

## Safety invariants

- Ownership remains visible at the call site.
- No implicit copy or drop is introduced to satisfy the attribute.
- The diagnostic is deterministic and points to the discarded call.
- The reason is explanatory text only and cannot alter semantics.
- Formatter and LSP preserve the declaration contract.
- Expression-context, generic, imported, external, and dispatch coverage is
  required before the contract is considered complete.

## Consequences

Callers receive an early compile-time error for an ignored marked result, while
the compiler preserves ownership and adds no runtime work. A role
implementation that adds or omits the contract is rejected with `E1123`, which
prevents dispatch from silently weakening the declaration-level guarantee.
