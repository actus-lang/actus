# ADR-0090: Explicit must-use results

- Status: Proposed
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
signature and rejects a direct call expression whose result is discarded as a
standalone statement.

Use is explicit through an assignment, return, argument, expression
composition, or other construct that consumes the call value. The attribute
does not copy values, alter ownership roles, insert cleanup, or introduce a
runtime representation.

The contract propagates through imported signatures and external declarations.
It is not permitted on declarations without a return value, and duplicate
declarations are rejected.

## Safety invariants

- Ownership remains visible at the call site.
- No implicit copy or drop is introduced to satisfy the attribute.
- The diagnostic is deterministic and points to the discarded call.
- The reason is explanatory text only and cannot alter semantics.
- Formatter and LSP preserve the declaration contract.
- Generic and dispatch-specific coverage must be added before expanding the
  rule beyond direct call expressions.

## Consequences

Callers receive an early compile-time error for an ignored marked result. The
initial implementation is deliberately conservative: it does not claim that
every dispatch form is covered until dedicated semantic fixtures establish the
same ownership behavior for those forms.
