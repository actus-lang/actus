# ADR-0026: Arbitrary Bit-Width Integers and Deterministic Packed Register Layouts

- Status: Proposed
- Date: 2026-09-26
- Scope: integer widths, packed fields, and embedded register layouts

## Context

Actus must represent hardware registers, protocol frames, and compact wire
formats without forcing programmers to reconstruct bitfields with masks and
shifts. That approach is verbose, obscures the width contract, and moves
range errors into runtime code. Bare-metal targets and embedded microcontrollers
need a layout that is known before linking and does not depend on an allocator
or host runtime.

Hexadecimal notation is useful for writing numeric literals, but it does not
describe the width or layout of a value. A literal format must therefore stay
separate from the type system.

## Decision

Actus will provide first-class unsigned and signed integer types from one to
128 bits: `u1` through `u128` and `i1` through `i128`. `u1` is the canonical
one-bit type. The `0x` prefix remains a literal format only; it does not
select a packed or register type.

Every conversion, literal, assignment, and arithmetic operation is checked
against the declared range at compile time whenever the operands are known.
Overflow that cannot be proven safe is rejected rather than silently
truncated. Runtime arithmetic follows the target's explicitly selected
overflow profile.

Actus will add a `pack` declaration for deterministic bitfields:

```actus
pack Control: u32 {
    enabled: u1,
    mode: u3,
    channel: u5,
    _reserved: u23,
}
```

The backing integer is mandatory. Field widths must fit within it, fields may
not overlap, and the compiler assigns offsets deterministically in declaration
order. Every remaining bit must either be represented by an explicit
`_reserved` field, as above, or be covered by a declared zero-filled reserved
region. Unspecified padding is not silently inserted. Endianness is explicit
in the pack contract and is never inferred from the host compiler. A packed
value has no hidden allocation.

Packed fields use the existing ownership model: `erg` permits an exclusive
update of a register value, while `abs` permits read-only inspection. A field
access cannot create an alias that outlives its permitted lexical scope.

## Code generation

The compiler lowers packed access to deterministic Cranelift integer
operations: shift by the field offset, mask to the field width, and combine
with the backing value for writes. Widths wider than a native machine word
are lowered as a defined sequence of words. The layout record, masks, shifts,
and endianness are available to diagnostics and generated ABI metadata.

## Constraints and verification

- [ ] Reject widths outside `u1..u128` and `i1..i128`.
- [ ] Reject packed fields whose total width exceeds the backing integer.
- [ ] Require explicit reserved coverage or a declared zero-filled remainder.
- [ ] Reject ambiguous or overlapping field layouts.
- [ ] Test literal range and compile-time overflow diagnostics.
- [ ] Test deterministic layout on hosted and freestanding targets.
- [ ] Test native MMIO-style reads and writes without allocation.

This ADR does not define volatile access, memory ordering, or a device
register discovery mechanism. Those contracts belong to the target HAL.
