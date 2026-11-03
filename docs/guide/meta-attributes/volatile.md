# `meta volatile`

`meta volatile` is a closed type/layout attribute for an explicit
hardware-observable memory-access contract. It is accepted only on a
non-generic `repr(C)` struct whose fields satisfy the checked native-layout
rules:

```actus
meta repr(C)
meta volatile
open struct UartRegisters {
    status: u32,
    data: u32,
}
```

The attribute does not make ordinary reads, writes, copies, comparisons, or
serialization volatile. Access must go through an explicit volatile primitive
using a trusted opaque `VolatileAddress[T]`. Raw integer addresses and pointer
arithmetic are not part of the language contract.

This distinction is intentional. A metadata marker by itself cannot force a
backend to emit volatile loads and stores. If the compiler accepted the marker
without a dedicated access primitive, code could look hardware-safe while the
generated object still used ordinary memory operations.

## Contract

The completed standalone metadata feature will require:

- a concrete, fully checked `meta repr(C)` value layout;
- an explicit unsafe native-memory operation at the access site;
- declared access width and alignment;
- target-specific availability and ordering rules;
- ownership and address-provenance checks;
- backend validation that rejects unsupported volatile lowering before object
  emission.

The initial ordering mode is `device`. A target or backend that cannot prove
the requested width, alignment, provenance, or ordering must reject the
program before object emission.

`meta volatile` will not create raw pointers, permit unchecked address
arithmetic, make every copy of a value volatile, or change ownership and
cleanup semantics. It will also not make a value serializable or suitable for
the persistence layer.

## Current implementation boundary

Actus now exposes explicit bounded Region operations:
`region_volatile_read_range` and `region_volatile_write_range`. They use
checked Region descriptors and caller-owned buffers, perform volatile access
only on the resident source or destination bytes, and carry the compiler's
`volatile` effect. They do not expose raw addresses or unchecked arithmetic.

The typed `VolatileAddress[T]` read surface is now available through
`std::volatile`. A trusted native provider returns the opaque descriptor; the
Actus caller can only borrow it and select a fixed-width read at an explicit
compile-time byte offset. The runtime validates the descriptor marker, target
size, target alignment, offset arithmetic, bounds, and access alignment before
performing one volatile load. Supported read widths are `u8`, `u16`, `u32`,
and `u64`. No integer-to-address conversion or write operation is provided.

The provider remains responsible for the lifetime and provenance of the
descriptor and addressed object. A descriptor that does not describe live
storage is outside the contract and must be rejected by the native provider;
the Actus runtime still rejects malformed metadata and every checked access
that it can prove invalid before loading.

The accepted design and required evidence are recorded in
[ADR-0101](../../decisions/ADR-0101-volatile-memory-contract.md).

