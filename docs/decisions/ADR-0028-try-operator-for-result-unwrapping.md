# ADR-0028: Ergonomic Early Return with the Try Operator (?)

- Status: Proposed
- Date: 2026-09-26
- Scope: `Result[T, E]` propagation and I/O control flow

## Context

I/O code frequently transforms one `Result[T, E]` into another. Repeating a
case expression for every operation makes the success path difficult to read
and makes it easier to omit cleanup on an error path.

## Decision

Actus will provide `?` as a postfix operator on a `Result[T, E]` expression.
On `Ok(value)`, the operator will yield `value` with type `T`. On `Err(error)`,
it will return `Err(error)` from the enclosing verb without executing later
statements.

The enclosing verb must return a compatible `Result[_, E]`. The compiler
rejects use of `?` on non-Result values, incompatible error types, and verbs
whose return contract cannot propagate the error.

The implementation will execute the lexical cleanup plan for every owned
binding in the active scopes before an early return. `abs` views will be
released as borrow records, `ins` loans will be restored at their call
boundary, and owned resources will be dropped in LIFO order. The operator
will add no hidden exception mechanism.

## Lowering contract

The planned Cranelift lowering reads the Result discriminant, branches to an
error block or a success block, and carries the payload through the existing
enum layout. The error block performs planned cleanup and returns the original
`Err` payload. The success block unwraps the value without allocation or
wrapper objects.

## Verification

- [ ] Parse postfix `?` expressions.
- [ ] Infer `Ok` and `Err` constructors from the expected Result type.
- [ ] Validate compatible error propagation.
- [ ] Lower success and early-return paths natively.
- [ ] Preserve scope cleanup and loan restoration on early return.
- [ ] Extend integration coverage to filesystem and network streams.
