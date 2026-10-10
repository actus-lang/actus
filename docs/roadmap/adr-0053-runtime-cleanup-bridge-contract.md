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

The descriptor is now present and LLVM emits `BufferDrop`, `RegionDrop`, and
target-width `EnumDrop` bridge calls for direct cleanup operands. Cranelift's
layout registry resolves nested moved-field paths and rejects an unknown path
before native cleanup emission. Semantic enum layout discovery and full
cleanup schedule integration remain open; the shared descriptor still needs
to carry the resolved binding and moved-field metadata before those semantic
paths are complete. The existing Cranelift cleanup path remains the behavioral
reference for parity fixtures until that integration is complete.
The neutral contract also defines a resolved descriptor that validates the
parent layout and optional field sublayout as one unit. Semantic providers have
now populated it from binding types and each moved-field path through an
explicit provider interface. The concrete Cranelift layout adapter and backend
emission adapter resolve nested Buffer and Region fields from the native layout
registry with a regression test. Cranelift production emission invokes this
provider after native layouts are built and fails before object emission when
semantic cleanup metadata is invalid. Non-runtime fields are excluded from the
runtime descriptor set. The semantic descriptor-lowering requirement is
complete. A hosted parity fixture now materializes both backend operands from
the same resolved descriptors and verifies exact pointer arguments, Region
status, and Buffer-before-Region ordering through a shared runtime shim. The
established Cranelift cleanup schedule remains the emission path until resolved
descriptors replace it in the full semantic emission path. Direct Buffer and
Region binding drops now consume the resolved descriptor in Cranelift native
emission; partial moved-field cleanup still uses the established layout path
until field-address descriptor emission is integrated.

