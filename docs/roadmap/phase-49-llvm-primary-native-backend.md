# Phase 49 — LLVM primary native backend

## Status

Planned. Work begins from the clean `main` checkpoint after the checked
representation metadata work. The existing metadata backlog remains paused
until this roadmap is complete. No metadata contract is changed by this plan.

## Objective

Complete LLVM native emission and make LLVM the default Actus backend while
retaining explicit Cranelift support during the compatibility period. The
result must preserve Actus source semantics, ownership, cleanup, ABI, target,
runtime, serialization, and zero-float contracts.

## Governing decision

- [ ] Accept ADR-0098 after implementation scope and compatibility policy are
      reviewed.
- [ ] Keep backend selection explicit and reject unavailable backend/target
      pairs without fallback.
- [ ] Keep semantic, ownership, target, runtime, and shared-lowering modules
      free of backend-specific handles and logic.

## Work sequence

### 49.1 — LLVM implementation inventory and baseline

- [ ] Inventory every LLVM-only path and classify it as complete, partial,
      unsupported, or untested.
- [ ] Inventory all shared lowering operations and map each to an LLVM emitter.
- [ ] Record the supported LLVM version, installation contract, linker policy,
      target matrix, and clean-checkout setup.
- [ ] Capture baseline results for LLVM-only, Cranelift-only, and all-features
      configurations before changing the default.
- [ ] Add stable diagnostics for every intentionally unsupported LLVM operation.

### 49.2 — Complete shared-contract lowering

- [ ] Prove LLVM coverage for scalar values, calls, returns, branches, loops,
      traps, typed failures, and checked casts.
- [ ] Complete aggregate, array, pack, enum, tagged-union, transparent,
      packed, and explicitly aligned layout lowering.
- [ ] Complete ownership transfer, borrowed views, exclusive loans, cleanup,
      early return, failure, and branch-join lowering.
- [ ] Complete generic specialization, imported modules, dynamic dispatch,
      native boundaries, and recursive call handling.
- [ ] Verify LLVM module signatures and layouts before object emission.

### 49.3 — Runtime, FFI, and target contracts

- [ ] Verify all runtime bridge signatures and failure statuses through LLVM.
- [ ] Verify external C declarations, checked representations, aggregate
      passing, aggregate returns, and calling-convention contracts.
- [ ] Complete hosted target object/link/execute coverage.
- [ ] Complete freestanding object and link-contract coverage where startup,
      provider, linker, and target evidence exist.
- [ ] Keep filesystem, clock, allocation, and other hosted services explicit;
      never inherit them into freestanding targets.

### 49.4 — Backend parity and regression hardening

- [ ] Run the shared semantic and negative fixture matrix through both
      backends wherever both target contracts support the fixture.
- [ ] Compare deterministic outputs, object symbols, ABI layouts, exit status,
      cleanup behavior, and typed failures.
- [ ] Add regressions for every LLVM failure found during the inventory.
- [ ] Verify that unsupported operations fail before invalid object emission.
- [ ] Run source limits, formatter, documentation, dependency, and security
      checks in every supported CI configuration.

### 49.5 — Default selection migration

- [ ] Change the default Cargo feature and backend selection only after all
      prior acceptance work passes.
- [ ] Update backend support documentation, configuration examples, help text,
      artifact metadata, and clean-checkout instructions.
- [ ] Verify explicit Cranelift selection and rollback from a clean checkout.
- [ ] Measure and record compile time, artifact size, cache behavior, and
      relevant native execution evidence.
- [ ] Run the complete default-LLVM suite before declaring the migration ready.

## Non-negotiable invariants

- No ownership role is weakened or inferred differently by LLVM.
- No cleanup action is dropped, duplicated, or hidden behind a backend wrapper.
- No serialized layout or public standard-library contract changes as an
  incidental backend migration.
- No silent backend fallback is introduced.
- No floating-point state or operation is introduced into integer-only paths.
- No synchronous filesystem or other blocking I/O is added to hot paths.
- No target support claim is accepted without target-appropriate evidence.

## Acceptance evidence

The roadmap can be closed only when the repository contains:

1. an accepted ADR-0098;
2. passing default-LLVM, LLVM-only, explicit-Cranelift, and parity suites;
3. complete target/backend availability and diagnostic coverage;
4. documented runtime, FFI, ABI, ownership, cleanup, and serialization
   evidence;
5. reproducible toolchain and linker instructions;
6. measured compatibility and performance-impact evidence; and
7. a tested rollback path that does not modify Actus source programs.

## Known non-goals

- Removing Cranelift during this migration.
- Adding new metadata attributes.
- Changing Actus ownership syntax or source semantics.
- Claiming hardware execution from a host build or linker result alone.
- Adding a new target without a separate target contract and evidence.

