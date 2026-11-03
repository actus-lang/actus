# `meta volatile`

`meta volatile` is reserved for an explicit hardware-observable memory-access
contract. The first explicit primitive is now available through the bounded
`std::region` API; the standalone type metadata remains disabled until the
full type-level contract is implemented.

This distinction is intentional. A metadata marker by itself cannot force a
backend to emit volatile loads and stores. If the compiler accepted the marker
without a dedicated access primitive, code could look hardware-safe while the
generated object still used ordinary memory operations.

## Planned contract

The completed standalone metadata feature will require:

- a concrete, fully checked `meta repr(C)` value layout;
- an explicit unsafe native-memory operation at the access site;
- declared access width and alignment;
- target-specific availability and ordering rules;
- ownership and address-provenance checks;
- backend validation that rejects unsupported volatile lowering before object
  emission.

`meta volatile` will not create raw pointers, permit unchecked address
arithmetic, make every copy of a value volatile, or change ownership and
cleanup semantics. It will also not make a value serializable or suitable for
the persistence layer.

## Why it is not enabled yet

Actus now exposes explicit bounded Region operations:
`region_volatile_read_range` and `region_volatile_write_range`. They use
checked Region descriptors and caller-owned buffers, perform volatile access
only on the resident source or destination bytes, and carry the compiler's
`volatile` effect. They do not expose raw addresses or unchecked arithmetic.

The standalone type metadata is still disabled because its remaining contract
must connect a declared `repr(C)` type to an explicit volatile storage place;
enabling that metadata before the storage/provenance model is complete would be
a misleading and unsafe partial implementation.

The accepted design and required evidence are recorded in
[ADR-0101](../../decisions/ADR-0101-volatile-memory-contract.md).

