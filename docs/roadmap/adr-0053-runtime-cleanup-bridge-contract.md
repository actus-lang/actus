# ADR-0053: Backend-Neutral Runtime Cleanup Bridge Contract

## Status

Proposed. This ADR defines the contract required before LLVM and Cranelift
can emit and execute ownership cleanup through Actus runtime bridges.

## Context

The backend-neutral instruction model already represents cleanup actions and
requires a value-bearing `BackendCleanupOperand` for actions that can release
runtime state. The operand currently carries a value identifier, layout, and
optional field sublayout. It does not identify the runtime bridge, its ABI, or
whether the value identifier is a stored handle, an address, or an aggregate
address.

Cranelift currently obtains this information from its semantic native type and
layout registries while emitting cleanup schedules. LLVM consumes the
backend-neutral instruction model directly and therefore cannot safely infer
the same information from an action name such as `drop-binding`. Inferring a
bridge from a binding name or from aggregate size would make the two backends
diverge and could release the wrong resource.

## Decision

The semantic-to-backend cleanup lowering must produce an explicit,
backend-neutral bridge descriptor for every runtime-backed cleanup action. The
descriptor must contain:

- a stable runtime symbol identity selected from the shared runtime contract;
- the bridge argument ABI, including whether the argument is a handle value,
  an address, or a field address;
- the value type and validated layout required by the bridge;
- the optional field sublayout after moved-field validation; and
- the bridge return contract, including whether a returned status must be
  checked or discarded.

The descriptor is metadata, not a backend instruction. Each backend lowers the
same descriptor using its own call builder and target ABI rules. The LLVM
implementation must declare the symbol with the descriptor's exact parameter
and return types, emit the call, and preserve the declared alignment. The
Cranelift implementation must resolve the same stable symbol and use the same
argument mode.

Cleanup actions that do not carry a complete descriptor remain compile-time
boundaries or fail closed with a typed diagnostic. A backend must never guess a
runtime bridge from an action label, binding name, layout size, or pointer
width.

## Required implementation order

1. Extend the shared runtime contract with stable cleanup bridge identities and
   argument/return modes.
2. Extend semantic cleanup lowering so binding type, ownership state, field
   sublayout, and bridge descriptor are validated before backend emission.
3. Add shared contract tests for every supported bridge mode, invalid layouts,
   moved fields, missing symbols, and return-status handling.
4. Add Cranelift parity tests against the existing native cleanup behavior.
5. Add LLVM declarations, calls, hosted execution fixtures, and undefined
   symbol validation using the same descriptor.
6. Add cross-backend semantic comparison and only then close the runtime
   cleanup gates.

## Invariants

- Runtime cleanup remains deterministic and allocation-free on the emitted hot
  path.
- The cleanup descriptor cannot contain a raw operating-system pointer or a
  backend-specific value.
- Field offsets, sizes, and alignments must be validated against the parent
  layout before publication.
- A failed bridge declaration or call lowering must produce an explicit
  diagnostic; cleanup must not be silently omitted.
- `EndBorrow` remains a compile-time boundary and does not call a runtime
  bridge.

## Consequences

The descriptor is now present and LLVM emits the `BufferDrop` and `RegionDrop`
bridge calls for direct cleanup operands. Enum cleanup remains open because
its `usize` extent argument must be declared with the selected target's pointer
width before it can be enabled for hosted and freestanding targets. The
existing Cranelift cleanup path remains the behavioral reference for parity
fixtures until that target-aware ABI boundary is complete.

