# ADR-0101: Explicit volatile memory-access contract

- Status: Accepted
- Date: 2026-11-03
- Scope: metadata, native memory access, MMIO safety, backend lowering, and
  target contracts

## Context

Hardware registers and other externally observable memory cannot be accessed
through ordinary loads and stores. An optimizer may remove, merge, reorder, or
cache an ordinary access when the language contract says that no ordinary
memory observer can see the difference. MMIO and device memory are different:
each access may have an observable hardware effect.

Actus has checked aggregate layouts and unsafe native boundaries, but a type
marker alone cannot name a volatile memory access. Adding `meta volatile`
without an explicit operation would create the appearance of safety while
leaving the backend/runtime boundary free to perform ordinary loads and
stores. That would be an unsound contract.

## Proposed decision

Add `meta volatile` only as part of a complete explicit memory-access contract.
The final contract must contain all of the following:

1. `meta volatile` is a closed compiler attribute, not a user-defined label.
2. It applies only to a concrete `repr(C)` value declaration whose layout is
   fully checked before code generation.
3. It does not create a pointer, grant arbitrary address arithmetic, weaken
   ownership, or make ordinary copies volatile.
4. Volatile access is performed only by a dedicated unsafe native-memory
   primitive whose access width, alignment, address provenance, and ownership
   contract are explicit at the call site.
5. The primitive lowers to a backend volatile load/store operation. Every
   supported backend must either preserve that operation's volatile semantics
   or reject the target before object emission; silent fallback to an ordinary
   load/store is forbidden.
6. The target contract declares supported widths, alignment rules, ordering
   guarantees, and whether the target supports the requested operation.
7. Volatile operations are excluded from `meta deterministic` and `meta
   realtime` unless a later contract explicitly defines that combination.
8. Volatile values cannot enter serialized state, implicit cleanup, or hidden
   allocation paths. A volatile access is an externally observable operation,
   not a persistence or ownership mechanism.

The accepted source contract is a checked `repr(C)` struct declaration:

```actus
meta repr(C)
meta volatile
open struct UartRegisters {
    status: u32,
    data: u32,
    control: u32,
}
```

`meta volatile` is a type/layout contract only. It never turns ordinary field
access into a volatile access. Hardware access must use an explicit volatile
primitive and an opaque `VolatileAddress[T]` supplied by a trusted native
provider. Integer-to-address conversion, arbitrary address arithmetic, and
raw pointer values in Actus state remain forbidden.

The initial ordering contract is `device`, selected explicitly by the volatile
primitive. Additional ordering modes require a target capability and backend
proof; they are not inferred from the metadata.

## Current implementation boundary

The first primitive now exists as a bounded `std::region` operation:
`region_volatile_read_range` and `region_volatile_write_range`. The runtime
uses checked Region descriptors and caller-owned buffers, and applies
`read_volatile` or `write_volatile` only to the resident source or destination
bytes. The operations carry the compiler-recognized `volatile` effect, so the
effect propagates to callers and is rejected from deterministic and realtime
contexts.

The bounded Region primitive is separate from the standalone type-level
`meta volatile` contract. The typed read surface now supplies
`VolatileAddress[T]` and fixed-width `u8`/`u16`/`u32`/`u64` reads through the
same explicit `volatile` effect. The provider owns the descriptor and storage;
the runtime validates its marker, target layout, bounds, offset arithmetic, and
alignment before calling `read_volatile`. Typed writes use the following
separate contract.

### Exclusive write contract

Typed writes must not reuse `VolatileAddress[T]`. The write surface will use a
distinct opaque `VolatileWriteAddress[T]` supplied by a trusted native provider.
This prevents a read-only descriptor from being reinterpreted as writable and
makes the provider's permission boundary visible in the type system.

The public write verbs will have these properties:

1. The address parameter is an `ins VolatileWriteAddress[T]` loan. The caller
   keeps ownership of the descriptor, but no aliased `abs` view may coexist
   during the write call.
2. The write width is fixed by the verb (`u8`, `u16`, `u32`, or `u64`) and the
   field offset is a compile-time `Usize` parameter. Runtime input cannot
   select an unchecked width or perform address arithmetic.
3. The descriptor carries writable provenance, target size/alignment, and
   bounded storage extent. The runtime validates all of them, including
   checked offset arithmetic and final access alignment, before one
   `write_volatile` operation.
4. The caller must declare both `meta effect("volatile")` and the closed
   `meta capability("volatile.write")`. The capability is authorization, not
   a replacement for ownership or descriptor validation.
5. The initial ordering guarantee remains `device`. Stronger ordering modes
   require a future target-specific capability and backend proof; they are not
   implied by a write declaration.
6. A target or backend without a proven writable volatile primitive must reject
   the declaration before object emission. No check-then-store or ordinary
   store fallback is permitted.

The write contract will return a typed `Result[Void, VolatileError]`. It will
not publish the descriptor, retain a pointer, serialize device state, or hide
an allocation/cleanup boundary. Read and write descriptors remain distinct
even when they refer to the same hardware register block.

## Alternatives rejected

- **Marker-only metadata:** rejected because it provides no enforceable access
  semantics and could mislead users about generated code.
- **Make every access to a volatile type volatile:** rejected because it makes
  ordinary local copies and aggregate construction hardware-observable without
  an explicit access site.
- **Use raw pointers in Actus values:** rejected because raw addresses would
  violate ownership, provenance, and serialized-state boundaries.
- **Lower only in LLVM:** rejected as a language-wide contract; a backend that
  cannot preserve the operation must fail closed rather than silently change
  semantics.

## Required implementation evidence

The implementation must contain:

- parser, formatter, LSP, and placement diagnostics for `meta volatile`;
- semantic rejection of incomplete layouts, unsupported fields, invalid roles,
  and unsupported target contracts;
- a dedicated unsafe volatile access primitive with explicit ownership and
  address contracts;
- shared backend validation of the opaque pointer and typed native call;
- LLVM and Cranelift object lowering for the typed read call, with the native
  runtime bridge performing the actual volatile load;
- positive and negative `.act` fixtures for width, alignment, ordering,
  ownership, deterministic/realtime interaction, and target availability;
- native runtime and object evidence proving that the typed read reaches the
  volatile runtime bridge and cannot silently become an ordinary Actus field
  access;
- exclusive-write evidence proving that read-only descriptors cannot reach the
  write bridge, aliased `abs` views are rejected during an `ins` write loan,
  and both native backends preserve the typed `write_volatile` operation;
- documentation of the exact hardware and memory-ordering guarantees.

