# Phase 55: Integers Wider Than 64 Bits

## Status: Planned

The language accepts every integer width from 1 to 128: `u65`..`u128` and
`i65`..`i128` are type names, and a literal may carry a suffix up to `u128`.
The front end checks programs that use them, but code generation supports
integers only up to 64 bits. A program that uses a wider integer passes
`actus check` and then fails in `actus build` with an internal back-end error
instead of a diagnostic:

- a literal that does not fit in 64 bits, such as
  `18446744073709551616u128`, fails with `invalid scalar integer literal`;
- an `as` conversion to or from a wide type fails with
  `cast source is not a supported scalar integer` or
  `cast target is not a supported scalar integer`;
- the Cranelift back end rejects a wide local with
  `Cranelift neutral contract slice supports scalar values only`;
- `print(int:)` rejects a wide value with `E1139`, and `format` has no wide
  argument.

A local that is only declared and computed without being used, such as
`erg b = a * a` with `a: u128`, builds on LLVM. That is the only case that works.

Counters, checksums, fixed-point products of two 64-bit values and identifiers
need these widths, and today a user only finds out at build time, through a
message that names the compiler's internals.

## Goals

1. **Values.** Wide integers work wherever a 64-bit integer works: locals,
   parameters, return values, struct and pack fields, arrays, constants, range
   loops and `serialize`, on the LLVM and Cranelift back ends. Literals of any
   width up to 128 bits are parsed, range-checked and emitted without going
   through 64 bits.
2. **Arithmetic and checks.** Every integer operator has the same meaning at
   widths 65 to 128 as at 64 and below: the same-type rule of ADR-0119 and
   ADR-0120, overflow, division by zero, `MIN / -1`, shift range and
   out-of-range conversions trap with the ADR-0117 report (ADR-0115). Odd
   widths such as `u100` behave like the odd widths below 64.
3. **Conversions.** `as` between every pair of integer types, wide included,
   with the existing literal and range rules.
4. **Output.** `print(int:)`, `eprint(int:)` and `format` accept every integer
   width; `E1139` is removed or narrowed to what is still unsupported.
5. **No internal errors.** Until a back end supports a construct, `actus check`
   rejects it with a diagnostic that has a code and names the construct. No
   width that the front end accepts may reach a back-end `does not support`
   error.

## Exit criteria

- Each goal has semantic tests and CLI tests that build and run on both back
  ends, including the boundaries `u65`, `u127`, `u128`, `i65`, `i128`, the
  values `MIN` and `MAX` of each, and literals just above `u64::MAX`; every new
  test has been seen failing.
- Freestanding ARM targets build programs with wide integers (LLVM lowers
  them to library calls or instruction pairs; the bundled runtime provides
  any helper it needs).
- The type reference, the operator reference and `docs/language/diagnostics.md`
  describe wide integers without exceptions.
- An ADR records the decisions, including how `print` and `format` render
  128-bit values.
