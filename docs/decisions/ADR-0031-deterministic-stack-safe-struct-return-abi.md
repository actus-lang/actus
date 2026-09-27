# ADR-0031: Deterministic Stack-Safe Struct Return ABI

## Status

Accepted

## Context

The native backend represents an Actus struct value as an address to storage
owned by the current native frame. Returning that address from a callee without
an ABI contract lets it point into the callee's dead stack frame. The failure is
especially dangerous when a struct is returned through a generic `Result[T, E]`
constructor: the enum payload copy may read the invalid address after the call.

## Decision

Every native verb whose return type is an Actus struct uses caller-allocated
return storage (the `sret` convention):

1. The caller allocates a stack slot using the registered struct layout.
2. The caller passes that slot's address as a hidden first argument.
3. The callee writes the returned struct bytes into that address and returns no
   aggregate pointer.
4. The caller treats its own slot address as the expression value for the
   remainder of the enclosing scope.

The same signature construction is used for imported declarations so the ABI
cannot diverge between Actus verbs and external declarations. Enum values remain
pointer-backed. When an enum constructor receives a struct payload, it copies
the payload bytes from the caller-owned struct slot into the enum allocation;
the copy therefore never depends on a callee stack frame.

Case expressions whose branches return from blocks infer their result type from
the returned expressions. This keeps aggregate return values typed correctly
when cleanup or control flow is expressed with a block body.

## Invariants

- A struct-returning callee never returns a pointer to its own local aggregate.
- The hidden destination argument is present exactly once and precedes public
  parameters.
- The caller owns the destination slot and its lifetime covers the expression's
  lexical use.
- Struct payload copies use the registered size and field layout.
- Native tests must exercise functional struct-returning helpers; direct struct
  literals are not sufficient evidence of ABI correctness.

## Verification

- [x] `OpenOptions` helper verbs return through caller-allocated storage.
- [x] The native append test uses `options_new`, `options_write`, and
  `options_append` rather than an inline options literal.
- [x] `Result[File, IoError]` and struct payload construction remain native-safe.
- [ ] Add dedicated compiler-level ABI layout tests for nested aggregate returns.
