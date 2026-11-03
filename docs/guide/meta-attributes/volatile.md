# `meta volatile`

`meta volatile` is reserved for an explicit hardware-observable memory-access
contract. It is not currently enabled as a standalone attribute.

This distinction is intentional. A metadata marker by itself cannot force a
backend to emit volatile loads and stores. If the compiler accepted the marker
without a dedicated access primitive, code could look hardware-safe while the
generated object still used ordinary memory operations.

## Planned contract

The completed feature will require:

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

Actus currently has checked native ABI layouts, but it does not yet expose the
explicit volatile memory-access primitive required to make this contract
meaningful. The implementation must therefore begin with that primitive and
its shared backend contract. Enabling the metadata first would be a misleading
and unsafe partial implementation.

The accepted design and required evidence are recorded in
[ADR-0101](../../decisions/ADR-0101-volatile-memory-contract.md).

