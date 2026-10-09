# Phase 44 — Backend Boundary and LLVM Target Support

## Status

Planned. This phase defines the architecture for multiple native code
generation backends. It starts from the checkpoint
`checkpoint/pre-backend-refactor` and must preserve the current Cranelift
behavior throughout the migration.

This phase is about the Actus compiler only. It does not define an application,
device operating system, or project-specific integration.

## Objective

Separate Actus semantic lowering from backend-specific code generation so the
compiler can support more than one native backend without duplicating language
semantics or weakening ownership, ABI, target, and runtime contracts.

The first backend remains Cranelift. LLVM is added as a second backend for
targets that require ARM32/Thumb support, including Cortex-M profiles, after
the backend-neutral boundary is stable.

## Required architecture

```text
Actus source
    -> lexer -> parser -> AST -> semantic model
    -> backend-neutral Actus lowering contract
        -> Cranelift backend
        -> LLVM backend
    -> target-specific object and linker pipeline
    -> explicit runtime provider
```

The semantic analyzer must not depend on Cranelift or LLVM types. Backend
implementations may consume the common lowering contract, target profile, ABI
description, runtime imports, and linker plan. Backend-specific types must not
leak into AST, parser, semantic, or shared target contracts.

## Execution bottlenecks and mandatory risk controls

### LLVM dependency isolation

LLVM integration must be optional for users who only need the Cranelift
backend. The implementation must use an explicit Cargo feature, such as
`llvm`, and must keep LLVM-specific crates, build scripts, linker discovery,
and native bindings outside the default dependency path.

- The default Actus build must not require a system LLVM installation.
- The LLVM feature must declare its supported LLVM version policy explicitly.
- Missing or incompatible LLVM installations must produce a clear setup error.
- CI must test the default Cranelift-only configuration and the LLVM feature
  configuration separately.
- The dependency, compile-time, binary-size, and cache impact of enabling LLVM
  must be measured and recorded.

### Cortex-M ABI and calling convention

ARM32/Thumb output must follow the selected ARM AAPCS contract exactly. The
backend must define and test argument registers, callee-saved registers, stack
alignment, return values, aggregate passing, aggregate returns, and external
symbol conventions before any C library integration is accepted.

- The selected Cortex-M ABI must be encoded in the target profile.
- The required 8-byte stack alignment must be preserved at every call boundary.
- Aggregate values must have one explicit register/stack representation shared
  by Actus, LLVM, the linker, and external C declarations.
- FFI fixtures must prove scalar and aggregate calls in both directions.
- A mismatch must fail during target or ABI validation rather than producing a
  linkable but unsafe image.
- Hardware acceptance must include a reproducible fault-free call trace; a
  successful host link is not ABI evidence.

### Minimal backend-neutral IR

The common IR must represent only the Actus semantics needed by the backends.
It must not become a second general-purpose compiler IR or duplicate LLVM's
optimization and instruction-selection layers.

- Ownership roles, Region capabilities, cleanup actions, typed failures,
  aggregate layouts, calls, returns, and traps must remain explicit.
- Target-independent IR nodes must not contain Cranelift or LLVM handles.
- Target-specific instruction selection, register allocation, optimization,
  and object emission belong entirely to the selected backend.
- The lowering from common IR to LLVM IR should be mechanical wherever the
  Actus contract permits it.
- Every common IR node must have a documented ownership, layout, failure, and
  backend support contract.
- Unsupported semantics must be rejected at the common boundary with a stable
  diagnostic instead of being approximated by a backend.

## Non-negotiable invariants

- Existing Cranelift output and public compiler behavior remain unchanged
  unless a separately accepted ADR changes the contract.
- Ownership, cleanup, enum layout, aggregate layout, Region descriptors,
  serialization formats, and runtime bridge symbols remain backend-neutral.
- Target profiles select pointer width, byte order, alignment, calling
  convention, object format, linker flavor, and runtime capabilities explicitly.
- No backend may introduce hidden allocation, floating-point state into an
  integer-only contract, synchronous hot-path I/O, or raw pointers in
  serialized Actus state.
- Every backend must reject unsupported types, intrinsics, runtime services,
  and target capabilities before emitting an invalid object.
- The same semantic and negative test cases must be usable by every backend
  where the target contract permits them.
- Native output claims require object, link, execution, and target-specific
  evidence at the appropriate boundary.

## Gate 44.1 — Freeze the current backend contract

- [x] Record the checkpoint commit and tag as the migration baseline.
- [x] Inventory every Cranelift import outside the backend implementation
      boundary.
- [x] Inventory the current native ABI, runtime bridge, linker, object-format,
      target-profile, and zero-float contracts.
- [x] Add a migration ADR describing the accepted boundary and compatibility
      rules.
- [x] Map each shared codegen responsibility to a backend-neutral contract or a
      backend-specific implementation.
- [x] Enforce the filesystem boundary: Cranelift-only files under
      `src/codegen/cranelift/`, LLVM-only files under `src/codegen/llvm/`, and
      backend-independent contracts in shared `src/codegen/` modules.
- [x] Add baseline tests that prove current Cranelift behavior before any
      implementation move.

### Gate 44.1 evidence

The baseline tag is `checkpoint/pre-backend-refactor`. The current dependency
and source inventory, proposed backend boundary, optional LLVM policy, target
ABI controls, and migration order are recorded in
`docs/roadmap/adr-0052-backend-boundary-and-llvm.md`.

**Gate status: Complete** — the inventory, responsibility mapping, filesystem
boundary, and baseline regression evidence are recorded. The baseline command
`cargo test --test codegen --all-features -- --format terse` passes with 66
tests and 0 failures. After the neutral contract extension, the complete
`cargo test --all-features -- --format terse` suite also passes with no
failures, including native Region, Wire, standard-library, semantic, and
zero-float checks.

## Gate 44.2 — Define the backend-neutral lowering contract

- [x] Define a backend-neutral representation for functions, blocks,
      instructions, values, layouts, calls, returns, traps, cleanup, and
      external runtime symbols.
- [x] Define how ownership cleanup plans and aggregate return slots are passed
      to a backend without exposing Cranelift or LLVM types.
- [x] Define target-neutral ABI metadata for scalar, aggregate, borrowed,
      exclusive, and runtime bridge parameters.
- [x] Define deterministic symbol identity independently of backend mangling.
- [x] Define error reporting for unsupported backend operations with source
      spans and target context.
- [x] Keep the representation minimal and Actus-oriented; do not reproduce a
      general optimization, instruction-selection, or register-allocation IR.
- [x] Prove that roles, cleanup, Region capabilities, typed failures, and
      aggregate layout survive lowering without backend-specific handles.
- [x] Add semantic and codegen tests for the contract without selecting a
      concrete machine backend.

### Gate 44.2 evidence

The backend-independent contract suite is in `tests/codegen_contract.rs` and
does not select Cranelift or LLVM. It covers role-aware passing, aggregate
layouts and return slots, deterministic unsupported-operation diagnostics,
ownership capability passing, cleanup actions, block identity, and neutral
control-flow targets, and typed failure discriminants. The command
`cargo test --test codegen_contract --all-features -- --format terse` passes
with 22 tests and 0 failures. The suite now also exercises an explicit
backend-neutral `Region` value with borrowed, exclusive-loan, and ownership
transfer capability modes. `lower_semantic_cleanup` validates semantic cleanup
plans before converting them into backend-neutral cleanup actions; full
semantic function and signature lowering now uses `lower_verb_signature` with
an injected neutral layout provider. Backend-specific layout registries remain
outside this adapter. Return ownership is preserved as `erg` for `Owned` and
`abs` for immutable returns. `lower_verb_body` now lowers direct calls, returns,
and block-based `if` control flow through an injected value and symbol provider.
Continuation blocks are threaded through sequential and nested control flow,
and `lower_verb` validates the combined signature and body as one neutral
function. Body constructs without a neutral contract are rejected with a
source-spanned unsupported-operation diagnostic rather than being approximated.
`BackendModule` now validates functions and external runtime symbols together,
including symbol uniqueness and external signature contracts, without exposing
backend-specific handles.
Typed failure resolution is delegated to the neutral body provider and lowers
to a validated `BackendInstruction::Fail` with an enum symbol, discriminant,
and optional payload value. Cleanup plans can now be attached to the neutral
function contract through `lower_verb_with_cleanup`, preserving their declared
depth, source span, action order, and ownership cleanup operations.
Together these tests prove that the semantic roles, Region capability modes,
aggregate return slots, typed failure discriminants, and cleanup actions are
represented by neutral values and metadata only; no Cranelift or LLVM handle
is required by the contract.

## Gate 44.3 — Isolate the existing Cranelift backend

- [x] Move Cranelift-specific lowering, instruction construction, runtime
      declarations, object emission, and linker integration under a dedicated
      backend boundary without changing behavior.
- [ ] Keep target profiles and runtime contracts outside the Cranelift module.
- [ ] Remove direct Cranelift types from shared lowering and semantic APIs.
- [ ] Preserve object identity, symbol names, cleanup order, ABI layout,
      diagnostics, and zero-float verification.
- [ ] Run the complete existing test and benchmark suite after each cohesive
      move.
- [x] Add an architecture test that prevents new backend-specific imports from
      entering shared frontend or semantic modules.

### Gate 44.3 evidence

The existing Cranelift lowering, runtime declarations, object emission, and
linker-facing implementation are contained under `src/codegen/cranelift/`.
`tests/architecture_conformance.rs` now verifies that shared codegen modules
cannot contain Cranelift implementation imports or crate tokens; the root
`src/codegen/mod.rs` remains the explicit public facade. The architecture suite
passes with 5 tests and 0 failures. Remaining Gate 44.3 work covers target
profiles, runtime contract extraction, and removal of backend-specific types
from the shared public API.

## Gate 44.4 — Formalize target, object, and linker boundaries

- [ ] Define target profiles for hosted x86-64, hosted AArch64, freestanding
      targets, and ARM32/Thumb targets.
- [ ] Separate target selection from backend selection.
- [ ] Define object format, relocation, symbol visibility, entry symbol,
      linker arguments, runtime archive, and startup requirements per target.
- [ ] Reject incompatible backend-target combinations with typed diagnostics.
- [ ] Prove that unused backend modules are not compiled or linked.
- [ ] Add deterministic object and linker-plan tests for every supported host
      profile.

## Gate 44.5 — Add the LLVM backend on a hosted target

- [ ] Select and document the supported LLVM integration boundary and version
      policy.
- [ ] Put the LLVM integration behind an explicit Cargo feature such as
      `llvm`; the default Cranelift build must not require LLVM or llvm-sys.
- [ ] Test the default feature set and the LLVM feature set as independent CI
      configurations.
- [ ] Report LLVM installation, build-time, dependency, and binary-size
      requirements separately from the default compiler.
- [ ] Add LLVM IR generation for the common scalar, control-flow, call, return,
      trap, and cleanup contract.
- [ ] Add aggregate layout, memory-copy, enum discriminant, and Result/Option
      lowering using the same Actus layout rules as Cranelift.
- [ ] Add runtime bridge declarations and external symbol validation.
- [ ] Emit deterministic object files and link a hosted executable.
- [ ] Compare LLVM and Cranelift semantic results for shared fixtures.
- [ ] Verify integer-only programs do not acquire floating-point instructions
      through the LLVM lowering path.
- [ ] Keep LLVM optional so the existing Cranelift build remains usable when
      LLVM is unavailable.

## Gate 44.6 — Add ARM32/Thumb target support

- [ ] Define the supported ARM architecture profile and Thumb instruction mode.
- [ ] Define the Cortex-M ABI, integer widths, alignment, argument and
      callee-saved registers, stack contract, entry point, runtime provider,
      and linker script interface.
- [ ] Preserve the required 8-byte stack alignment at every call boundary.
- [ ] Define register and stack representations for scalar and aggregate
      arguments and return values before external C interoperability.
- [ ] Add ABI fixtures for scalar and aggregate calls in both directions.
- [ ] Generate ARM32/Thumb objects through the LLVM backend.
- [ ] Link a minimal freestanding image with the explicit runtime provider.
- [ ] Add object-level checks for relocations, symbols, sections, alignment,
      and forbidden hosted imports.
- [ ] Add emulator or target execution evidence for the minimal image.
- [ ] Record the exact LLVM, assembler, linker, target triple, and linker-script
      versions used by the acceptance run.

## Gate 44.7 — External C ABI and runtime interoperability

- [ ] Define the stable Actus-to-C ABI boundary for external libraries and
      runtime providers.
- [ ] Define ownership, lifetime, error, calling convention, alignment, and
      symbol visibility rules for imported C functions.
- [ ] Add a minimal external-library fixture that links through the LLVM ARM
      path without putting C or foreign code into Actus source modules.
- [ ] Prove that external runtime calls remain explicit and that serialized
      Actus state contains no OS pointer.
- [ ] Document how precompiled or separately compiled C libraries may be linked
      without implying that Actus can compile arbitrary C source itself.

## Gate 44.8 — Cross-backend acceptance and regression protection

- [ ] Run semantic, ownership, cleanup, layout, ABI, serialization, and
      negative tests against every applicable backend.
- [ ] Compare deterministic results and failure classes between Cranelift and
      LLVM for shared target profiles.
- [ ] Verify standard-library facades and runtime bridges on hosted and
      freestanding profiles.
- [ ] Verify no hidden allocation, filesystem I/O, reverse pipeline dependency,
      or backend type leak was introduced.
- [ ] Run formatter, source-limit, compiler, native, documentation, coverage,
      dependency, and license checks.
- [ ] Record benchmark conditions and distinguish host measurements from
      embedded measurements.

## Gate 44.9 — Documentation and migration completion

- [ ] Update the language handbook with backend selection and target profiles.
- [ ] Document Cranelift and LLVM installation, version, and availability
      requirements separately.
- [ ] Document supported targets, unsupported combinations, runtime providers,
      object formats, linkers, and rollback behavior.
- [ ] Add troubleshooting guidance for missing LLVM, linker, sysroot, runtime,
      or target capability failures.
- [ ] Record the final architecture decision and migration impact.
- [ ] Close this phase only when every claim has reproducible evidence and the
      original Cranelift checkpoint remains recoverable.

## Out of scope

- Rewriting the Actus semantic analyzer for a particular backend.
- Adding application-specific protocol or operating-system functionality.
- Claiming STM32 execution from a host-only LLVM build.
- Replacing Cranelift before LLVM reaches equivalent shared-contract coverage.
- Adding dynamic allocation, hidden runtime state, or unbounded collections to
  make backend integration easier.
