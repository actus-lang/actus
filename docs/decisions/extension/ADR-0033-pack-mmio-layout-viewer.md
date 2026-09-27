# ADR-0033: Pack and MMIO Layout Viewer

- Status: Proposed
- Date: 2026-09-27
- Scope: `pack` inspection in the Actus LSP and VS Code extension

## Context

Actus `pack` declarations provide deterministic backing storage, explicit
endianness, fixed field widths, offsets, ownership roles, reserved bits, and
compile-time overlap checks. Embedded developers need to inspect this layout
without repeatedly translating source into a datasheet-style bit diagram.

The viewer must reflect compiler layout math exactly. TextMate scopes and
client-side parsing are insufficient because they cannot prove capacity,
overlap, masks, or target-specific representation.

## Decision

The compiler will expose resolved pack layout metadata through the existing
semantic/LSP snapshot and a versioned `actus/packLayout` request. The VS Code
extension will provide a read-only Pack/MMIO Layout view, opened from a pack
declaration, a field hover, CodeLens, or completion result.

## Contract

Each layout response includes:

- pack name and declaration location;
- fixed backing integer (`u8`, `u16`, `u32`, `u64`, or `u128`);
- capacity in bits and bytes;
- explicit `little` or `big` layout;
- target data model and pointer width where relevant;
- ordered fields with name, role, type, offset, width, inclusive/exclusive
  bit range, mask, default value, and declaration span;
- reserved-field status and uncovered-bit diagnostics;
- density and padding summary; and
- layout schema/compiler version.

The mask is computed as `((1 << width) - 1) << offset` in the resolved
backing-width domain. For a 128-bit backing type the response carries a
structured high/low representation rather than a lossy JSON number.

## Visual behavior

The bit strip displays bit zero and byte order unambiguously. Little-endian
and big-endian are shown as separate labels and byte lanes; the UI must never
imply that source bit numbering changes with byte order. Fields are colored by
access role, with reserved bits visually distinct. Hover and keyboard focus
show exact offset, width, mask, type, role, and source location.

The first release is read-only. A value inspector may accept a numeric value
locally and show the resulting encoded word, but it must not modify source,
write memory, or access hardware. “Copy as Actus literal” may copy a
deterministic literal only after the compiler validates the value.

## MMIO boundary

A `pack` layout is not itself an MMIO mapping. A future target HAL must provide
an explicit address and access capability. The viewer labels host-side packed
values separately from mapped hardware registers and refuses to claim live
hardware state without a configured debugger/target provider.

## Failure and compatibility behavior

Invalid or incomplete declarations show compiler diagnostics and an incomplete
layout state. The extension must not fabricate offsets or masks. Unknown schema
versions produce a visible compatibility warning and preserve source
navigation.

## Verification

- [ ] Golden responses cover `u8`, `u16`, `u32`, `u64`, and `u128` layouts.
- [ ] Tests cover little/big endianness, reserved fields, zero-width rejection,
      overlap, capacity overflow, and uncovered-bit diagnostics.
- [ ] LSP tests verify field definition, hover, and mask formatting.
- [ ] UI tests verify keyboard navigation, light/dark themes, and 128-bit
      display without numeric truncation.
- [ ] Freestanding tests prove the metadata path does not require a host
      allocator or runtime.

## Non-goals

This ADR does not define a new `pack` syntax, perform register writes, replace
hardware documentation, or expose arbitrary process memory to a webview.
