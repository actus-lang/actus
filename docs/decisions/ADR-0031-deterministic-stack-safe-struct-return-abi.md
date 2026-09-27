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

### Scalar and aggregate representation contract

The `sret` rule is part of the complete native return convention:

- `u1..u8` and `i1..i8` use an `I8` storage value; `u9..u16` and `i9..i16`
  use `I16`; `u17..u32` and `i17..i32` use `I32`; and `u33..u64` and
  `i33..i64` use `I64`.
- `u65..u128` and `i65..i128` use the deterministic 16-byte wide-integer
  representation supported by the selected Cranelift target. The low/high
  word order and alignment are fixed by the registered layout, never inferred
  from a host pointer.
- `f32` and `f64` use Cranelift `F32` and `F64` values respectively.
- `Void` has no value and occupies zero bytes. It contributes no return
  register and cannot create a dangling aggregate address.
- Structs and enum payloads use registered size, alignment, and field offsets;
  only the caller owns storage whose address can outlive the callee.

The hidden `sret` destination is inserted once, before public parameters. It
is not exposed in Actus source syntax and is not allocated by the runtime.
These rules apply equally to direct returns, `Result[T, E]` payloads, `?`
propagation, and cleanup paths.

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
- Public standard-library verbs convert runtime status values into
  `Result[T, IoError]`; raw C status integers are confined to the unsafe FFI
  declarations.

## Verification

- [x] `OpenOptions` helper verbs return through caller-allocated storage.
- [x] The native append test uses `options_new`, `options_write`, and
  `options_append` rather than an inline options literal.
- [x] `Result[File, IoError]` and struct payload construction remain native-safe.
- [x] Add dedicated compiler-level ABI layout tests for nested aggregate returns.
- [x] Cover wide integers, floating-point values, and zero-sized `Void` return
  conventions in native tests.
