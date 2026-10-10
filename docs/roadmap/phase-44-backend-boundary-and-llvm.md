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
- [x] Keep target profiles and runtime contracts outside the Cranelift module.
- [x] Remove direct Cranelift types from shared lowering and semantic APIs.
- [x] Preserve object identity, symbol names, cleanup order, ABI layout,
      diagnostics, and zero-float verification.
- [x] Run the complete existing test and benchmark suite after each cohesive
      move.
- [x] Add an architecture test that prevents new backend-specific imports from
      entering shared frontend or semantic modules.

### Gate 44.3 evidence

The existing Cranelift lowering, runtime declarations, object emission, and
linker-facing implementation are contained under `src/codegen/cranelift/`.
`tests/architecture_conformance.rs` now verifies that shared codegen modules
cannot contain Cranelift implementation imports or crate tokens; the root
`src/codegen/mod.rs` remains the explicit public facade. The architecture suite
cannot contain Cranelift implementation imports or crate tokens, and the facade
must expose neutral `BackendEmitError` and `BackendAbiError` contracts rather
than the former native error names. Symbol binding metadata is now represented
by the neutral `BackendSymbolBindings` contract and is no longer named as a
native backend type in the shared facade. The architecture suite now covers 7
tests. Cleanup lowering plans and instructions are now named as `Lowered*`
contracts, so semantic cleanup does not expose a native-backend label. The
Cranelift object-emission and native-signature entry points now live under the
explicit `codegen::cranelift` namespace instead of the shared facade. The
architecture suite also rejects re-exporting those entry points from
`src/codegen/mod.rs`.
Target and runtime source trees are also checked for Cranelift implementation
imports. The shared facade no longer re-exports Cranelift emission or native
signature validation entry points; callers use the explicit
`codegen::cranelift` namespace. Existing symbol identity tests, native cleanup
fixtures, FFI/link tests, target tests, and strict zero-float native tests pass
after the moves. The complete suite passes with zero failures. The benchmark
executables also pass: with 100 frontend iterations, lexer latency was 489 ns
per iteration and parser latency was 1.337 us per iteration; the Region
benchmark reported 23 ns open, 12 ns read, 9 ns write, 13 ns publish, 12 ns
cancel, and 17 ns remap averages over 10,000 iterations. Its 100-iteration
bulk measurements reported 1040 ns read and 924 ns write for 64 KiB, and 2051
ns read and 1858 ns write for 128 KiB. These are hosted benchmark observations,
not hardware measurements.

## Gate 44.4 — Formalize target, object, and linker boundaries

- [x] Define target profiles for hosted x86-64, hosted AArch64, freestanding
      targets, and ARM32/Thumb targets.
- [x] Separate target selection from backend selection.
- [x] Define object format, relocation, symbol visibility, entry symbol,
      linker arguments, runtime archive, and startup requirements per target.
- [x] Reject incompatible backend-target combinations with typed diagnostics.
- [x] Prove that unused backend modules are not compiled or linked.
- [x] Add deterministic object and linker-plan tests for every supported host
      profile.

### Gate 44.4 evidence

`TargetSpec::artifact_contract` now exposes the target object format, linker
flavor, entry contract, and startup contract without selecting a backend.
`LinkerPlan` materializes linker, object, runtime archive, library, and output
arguments before process execution, so the plan can be inspected and tested
deterministically. Configuration tests cover hosted Linux GNU, macOS Apple,
and Windows MSVC profiles, including the platform-specific output argument and
runtime library set. Target tests cover hosted x86-64, hosted AArch64,
freestanding x86-64, and ARM32/Thumb profiles. The full test suite passes with
zero failures. Relocation-model and symbol-visibility policy remain open for
the next Gate 44.4 block. `RelocationModel` now distinguishes static and
position-independent artifacts, while `TargetArtifactContract` records
exported entry symbols and imported runtime symbols explicitly. Configuration
and target regression tests cover both policies. Cranelift dependencies are
optional Cargo dependencies under the default `cranelift` feature, and the
architecture suite verifies that feature contract. The command
`CARGO_TARGET_DIR=/tmp/actus-target-no-cranelift cargo check
--no-default-features --lib --bin actus` passes without compiling Cranelift
crates; the no-feature CLI retains a typed unavailable-backend error path.

## Gate 44.5 — Add the LLVM backend on a hosted target

- [x] Select and document the supported LLVM integration boundary and version
      policy.
- [x] Put the LLVM integration behind an explicit Cargo feature such as
      `llvm`; the default Cranelift build must not require LLVM or llvm-sys.
- [x] Test the default feature set and the LLVM feature set as independent CI
      configurations.
- [x] Report LLVM installation, build-time, dependency, and binary-size
      requirements separately from the default compiler.
- [ ] Add LLVM IR generation for the common scalar, control-flow, call, return,
      trap, and cleanup contract.
- [ ] Add aggregate layout, memory-copy, enum discriminant, and Result/Option
      lowering using the same Actus layout rules as Cranelift.
- [x] Add runtime bridge declarations and external symbol validation.
- [ ] Emit deterministic object files and link a hosted executable.
- [ ] Compare LLVM and Cranelift semantic results for shared fixtures.
- [ ] Verify integer-only programs do not acquire floating-point instructions
      through the LLVM lowering path.
- [ ] Keep LLVM optional so the existing Cranelift build remains usable when
      LLVM is unavailable.

### Gate 44.5 evidence

The initial LLVM boundary is documented in
`docs/roadmap/adr-0052-backend-boundary-and-llvm.md`. The integration boundary
is `src/codegen/llvm/`; LLVM bindings and discovery are reserved for that
module and must never enter shared lowering, target contracts, or the default
Cranelift dependency path. The selected policy is LLVM 23.1.x through
`llvm-sys 231.0.0` with strict versioning and discovery through `llvm-config`
or `LLVM_SYS_231_PREFIX`. The manifest exposes this as an opt-in `llvm`
feature while keeping `cranelift` as the only default backend feature.
Architecture tests verify that LLVM is opt-in and that the binding is absent
from the default dependency surface. The first emitter slice now creates a
validated LLVM module with scalar/unit function signatures, basic blocks,
parameter mapping, direct scalar returns, direct calls, conditional branches,
jumps, terminating traps, and fixed-layout direct aggregate/Region values with
1, 2, 4, or 8-byte alignment. Non-direct aggregate parameters now lower to
opaque pointers with explicit parameter alignment. Aggregate return slots now
use a hidden aligned pointer with caller-side stack storage. Memory-copy
lowering, runtime-backed cleanup actions, and typed failures remain explicit
open work. Compile-time `EndBorrow` cleanup boundaries are lowered as no-op
semantic boundaries, while runtime-backed cleanup actions fail closed until
their bridge ABI and storage representation are implemented. The feature build
and this emitter slice are covered by the LLVM integration test.
The neutral contract now also validates non-zero, alignment-compatible pointer
copy extents, and the LLVM slice lowers those extents through overlap-safe
`llvm.memmove` calls. Full semantic copy propagation and aggregate field copy
integration remain open until the common body lowering carries pointer values
for those operations.
Every emitted module is also passed through LLVM's module verifier before IR
is returned, so the integration tests validate both the generated contract and
LLVM's structural/type invariants.
The neutral contract now carries explicit enum representation metadata for
typed failures: aggregate extent and alignment, discriminant offset and width,
payload offset and layout, and niche-pointer mode. Contract validation rejects
invalid ranges and mismatched payload metadata. LLVM lowers regular aggregate
enum failures with integer payloads by constructing the declared byte layout
and returning it directly or through the declared return slot; the LLVM module
verifier covers the generated IR. Niche-pointer `Option` lowering remains open
and is rejected with a stable diagnostic rather than approximated.
Neutral `BackendExternal` declarations are emitted with the same validated
symbol identity and signature contract as internal functions. The LLVM suite
also covers an external runtime-style declaration and call, including the
deterministic symbol spelling and typed argument.
The hosted LLVM emitter now accepts the shared `TargetSpec`, initializes the
native LLVM target registry, applies the target triple and data layout, and
emits an object file through `LLVMTargetMachineEmitToFile`. The object test
emits the same unit module twice, compares the complete bytes for deterministic
output, parses the result with the object reader, and verifies the exported
Actus symbol. This closes the LLVM object-emission slice for the host target;
hosted executable linking, cross-target registration, and target-specific
runtime/startup integration remain open.
CI now runs a separate Cranelift-only configuration across all targets and a
separate LLVM-only configuration that checks the compiler library and binary
surface plus the LLVM integration suite. The LLVM-only configuration does not
claim that Cranelift-specific native fixtures are supported without the
Cranelift feature.
Installation requirements, direct dependency trees, clean debug build times,
target-directory sizes, and debug executable sizes are recorded in
`docs/benchmarks/phase-44-backend-build-profile-2026-10-09.md`. The record
states the host and evidence limits explicitly.

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
