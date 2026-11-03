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

Actus currently has checked aggregate layouts and unsafe native boundaries, but
it does not yet have a source-level operation that names a volatile memory
access. Adding `meta volatile` as a marker without such an operation would
create the appearance of safety while leaving the backend free to emit ordinary
loads and stores. That would be an unsound contract.

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
`meta volatile` contract.
It does not make every `repr(C)` value automatically volatile and does not
replace the typed address contract above.

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
- shared backend instruction validation;
- LLVM and Cranelift behavior, or a stable pre-emission rejection for a
  backend without the required primitive;
- positive and negative `.act` fixtures for width, alignment, ordering,
  ownership, deterministic/realtime interaction, and target availability;
- native IR/object evidence proving that volatile operations remain volatile;
- documentation of the exact hardware and memory-ordering guarantees.

