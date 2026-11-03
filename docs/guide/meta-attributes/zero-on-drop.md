# `meta zero_on_drop`

`meta zero_on_drop` marks a concrete `repr(C)` struct whose owned storage must
be overwritten with zero bytes when its ownership lifetime ends.

```actus
meta repr(C)
meta zero_on_drop
struct SessionKey {
    key_a: u64,
    key_b: u64,
    key_c: u64,
    key_d: u64,
}
```

The declaration is deliberately explicit. The attribute does not make every
copy or borrow volatile, does not grant access to hardware, and does not
change the ownership role of a binding. An `abs` view observes the value and
does not clear it; the owner is responsible for the eventual cleanup.

The first contract accepts only concrete `repr(C)` structs. Generic structs,
`abs`/`ins` fields, and runtime-resource fields such as `Buffer`, `Region`, and
`String` are rejected until their nested cleanup order has its own contract.
This prevents a secret-clearing promise from being attached to storage whose
layout or ownership is not fully known.

The compiler-generated zeroing operation applies to normal scope cleanup,
explicit `drop`, early return, loop exits, and other ownership cleanup paths.
It runs after any supported nested cleanup and before the owned storage is
released or becomes unreachable. The operation uses the native runtime's
volatile byte-store bridge so an optimizer cannot remove it as a dead ordinary
store.

`zero_on_drop` is not a substitute for key rotation, process isolation, or
hardware-backed key storage. It protects the Actus-owned storage lifetime and
does not promise that copies made outside the contract or prior allocator
backing storage have been erased.
