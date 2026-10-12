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
does not select Cranelift or LLVM. Backend-specific tests live under
`tests/codegen/cranelift/` and `tests/codegen/llvm/`, each with an explicit
Cargo test target. The shared contract suite covers role-aware passing, aggregate
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
- [x] Add LLVM IR generation for the common scalar, control-flow, call, return,
      trap, and cleanup contract.
- [x] Add indirect aggregate layout and bounded memory-copy lowering using the
      same Actus layout rules as Cranelift.
- [x] Add regular enum discriminant and integer-payload lowering through the
      declared aggregate `ReturnSlot` contract.
- [x] Define and validate a target-aware direct aggregate ABI descriptor for
      register fragments, stack fragments, alignment, and complete layout
      coverage.
- [x] Validate niche-pointer `Option` representation boundaries in the
      backend-neutral enum contract without claiming native pointer lowering.
- [x] Lower and execute the niche-pointer `Option::None` direct pointer return
      through both native backends.
- [x] Add direct aggregate ABI and niche-pointer `Option` lowering using the
      same Actus layout rules as Cranelift.
- [x] Add runtime bridge declarations and external symbol validation.
- [x] Emit deterministic object files and link a hosted executable.
- [x] Compare LLVM and Cranelift semantic results for the supported direct
      scalar/unit and control-flow contract slices.
- [x] Compare LLVM and Cranelift semantic results for the supported indirect
      aggregate and memory-copy contract slice.
- [x] Compare LLVM and Cranelift semantic results for the supported regular
      enum typed-failure `ReturnSlot` fixture.
- [x] Extend cross-backend semantic comparison to direct aggregate and
      niche-pointer `Option` fixtures.
- [x] Extend cross-backend semantic comparison to ownership and runtime-cleanup
      fixtures.
- [x] Require explicit value-bearing neutral cleanup operands before backend
      lowering.
- [x] Define bounds-checked field sublayouts for ownership cleanup operands.
- [x] Emit and execute runtime bridge calls for ownership cleanup.
- [x] Define a backend-neutral cleanup bridge descriptor with stable symbol,
      argument mode, layout, and return-status metadata.
- [x] Lower semantic binding types and moved-field sublayouts into complete
      cleanup bridge descriptors before backend emission.
- [x] Execute identical descriptor-driven cleanup fixtures through Cranelift
      and LLVM and compare status, call arguments, and cleanup ordering.
- [x] Verify integer-only programs do not acquire floating-point instructions
      through the LLVM lowering path.
- [x] Keep LLVM optional so the existing Cranelift build remains usable when
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
lowering remains available through the existing neutral memory-copy contract;
descriptor-driven Buffer, Region, and target-width Enum cleanup calls are now
emitted. Shared lowering now carries the complete semantic cleanup schedule,
including return and loop unwind plans, with target-neutral builtin cleanup
descriptors for opaque `Region`, `Buffer`, `String`, and `Arena` layouts. Native
runtime emission now materializes resolved builtin parameter drops and
function-scope local builtin drops as neutral `Cleanup` instructions before
return and typed-failure terminators. Nested block cleanup placement and
loop `break`/`continue` unwind cleanup are now covered by neutral lowering;
aggregate field cleanup emission remains open.
Regular enum typed failures with integer payloads now lower through the
declared aggregate `ReturnSlot`; compile-time `EndBorrow` cleanup boundaries are lowered as no-op
semantic boundaries, while incomplete semantic cleanup descriptors fail closed
until their storage representation is implemented. The feature build
and this emitter slice are covered by the LLVM integration test. LLVM regression
coverage verifies descriptor-driven Region and target-width Enum cleanup calls.
The neutral contract now lowers bounded Buffer and Region cleanup bridges through
Cranelift as well as LLVM. The hosted parity fixture
`compares_llvm_and_cranelift_on_runtime_cleanup_order_fixture` links both
objects against the same runtime shim and verifies identical BufferDrop then
RegionDrop execution order.
ADR-0053 defines the remaining semantic bridge and parity work.
The neutral contract now also validates non-zero, alignment-compatible pointer
copy extents, and the LLVM slice lowers those extents through overlap-safe
`llvm.memmove` calls. Full semantic copy propagation and aggregate field copy
integration remain open until the common body lowering carries pointer values
for those operations.
Cranelift's authoritative layout registry now resolves nested moved-field paths
to checked offset, size, and alignment metadata, and native partial-binding
cleanup rejects an unknown or malformed path before emitting any runtime drop.
This is a fail-closed native validation step; the shared semantic cleanup
descriptor still needs to carry the resolved metadata before the remaining
cleanup gate can close.
The neutral contract now has a separate resolved cleanup descriptor that binds
the bridge ABI to a complete parent layout and optional field sublayout. Its
validation rejects missing field metadata, misalignment, and out-of-bounds
field ranges before a backend can consume the descriptor. Provider-based
semantic population of this descriptor is now available through an explicit
cleanup provider interface. The concrete Cranelift layout adapter and backend
layout regression now exercise the native registry for nested Buffer and Region
fields. Cranelift production emission invokes this provider after native
layouts are built and rejects invalid semantic cleanup metadata before object
emission. Non-runtime semantic fields are excluded from the runtime descriptor
set. The descriptor-driven hosted parity fixture now materializes both backend
operands from the same resolved descriptors and links both objects against the
same runtime shim. The shim verifies the exact Buffer and Region pointer
arguments, the Region status, and Buffer-before-Region cleanup ordering; the
fixture passes for both backends. Production Cranelift emission now attaches
resolved plans to the native cleanup schedule and uses the descriptor for
direct Buffer and Region binding drops, while preserving the existing path for
partial moved-field cleanup. Full moved-field descriptor emission remains a
separate follow-up.
Every emitted module is also passed through LLVM's module verifier before IR
is returned, so the integration tests validate both the generated contract and
LLVM's structural/type invariants.
The neutral contract now carries explicit enum representation metadata for
typed failures: aggregate extent and alignment, discriminant offset and width,
payload offset and layout, and niche-pointer mode. Contract validation rejects
invalid ranges and mismatched payload metadata. LLVM lowers regular aggregate
enum failures with integer payloads by constructing the declared byte layout
and returning it directly or through the declared return slot; the LLVM module
verifier covers the generated IR. Generic pointer-`Option` construction and
case lowering now use the shared contract and backend emitters; nested `case`
expressions inside arithmetic remain open and continue to return a stable
shared-lowering diagnostic.
Direct aggregate ABI lowering remains open until a target/backend ABI
descriptor defines register fragments, stack placement, padding, alignment,
caller/callee lowering, and the matching external C declaration. Aggregate
size alone is not sufficient to infer this contract, and no direct aggregate
shape is silently rewritten as an indirect pointer.
The neutral contract now provides `BackendDirectAggregateAbi` with separate
parameter and return fragment descriptions. Validation rejects gaps,
misaligned ranges, duplicate register indices, overlapping stack ranges, and
descriptors that do not cover the complete aggregate layout. The descriptor is
now carried explicitly by `DirectAggregate` parameter and return modes, and a
scalar cannot use that mode. LLVM hosted lowering accepts the explicit mode
and preserves the aggregate value type without applying pointer-only alignment
attributes. Cranelift now lowers the hosted contiguous-register slice by
mapping each declared fragment to one native integer parameter or return value.
Stack fragments and non-contiguous register assignments return stable
unsupported diagnostics; no implicit pointer rewrite is used.
Niche-pointer enum metadata is also restricted to pointer-sized,
zero-offset layouts with no inline payload. Native pointer-sentinel lowering
for `Option::None` and pointer payloads for `Option::Some` are covered by the
LLVM/Cranelift parity fixtures; the emitted sentinel remains private backend
ABI state.
Neutral `BackendExternal` declarations are emitted with the same validated
symbol identity and signature contract as internal functions. The LLVM suite
also covers an external runtime-style declaration and call, including the
deterministic symbol spelling and typed argument.
The hosted LLVM emitter now accepts the shared `TargetSpec`, initializes the
native LLVM target registry, applies the target triple and data layout, and
emits an object file through `LLVMTargetMachineEmitToFile`. The object test
emits the same unit module twice, compares the complete bytes for deterministic
output, parses the result with the object reader, and verifies the exported
Actus symbol. A Linux hosted integration fixture then combines that LLVM object
with a C entry shim through the shared `LinkerPlan`, links an executable, and
executes it successfully. This closes the host object-to-executable slice;
cross-target registration, CLI backend selection, and target-specific
runtime/startup integration remain open.
LLVM lowering also rejects floating-point parameter and return types before
constructing an LLVM module. The negative fixture verifies the stable
integer-only diagnostic; the supported neutral value model emits only integer,
pointer, aggregate, and unit types. Instruction-level float auditing remains
covered by the existing Cranelift audit and will be extended when shared body
lowering feeds both backends.
The optional boundary is verified independently with
`CARGO_TARGET_DIR=/tmp/actus-no-llvm cargo check --no-default-features --lib
--bin actus` and `cargo check --features cranelift --lib --bin actus`; both
complete without LLVM. The architecture suite checks the manifest feature
contract, and CI runs separate Cranelift-only and LLVM-only jobs. No fallback
from a requested LLVM build to Cranelift is introduced.
CI now runs a separate Cranelift-only configuration across all targets and a
separate LLVM-only configuration that checks the compiler library and binary
surface plus the LLVM integration suite. The LLVM-only configuration does not
claim that Cranelift-specific native fixtures are supported without the
Cranelift feature.
The Cranelift backend now consumes the neutral `BackendModule` contract for a
bounded slice: direct scalar and unit signatures, direct calls and returns,
conditional branches, jumps, trap terminators, and compile-time `EndBorrow`
cleanup boundaries. The shared parity fixture lowers the same identity and
control-flow functions through LLVM and Cranelift, links each object with the
same hosted C entry shim through `LinkerPlan`, and executes both programs with
the same exit status. The focused command
`LLVM_SYS_231_PREFIX=/usr cargo test --features llvm --test llvm_backend`
passes with 18 tests and 0 failures. This evidence covers only the stated
scalar/unit and control-flow slices; direct aggregate, remaining niche-pointer
`Option` payload cases, ownership, and runtime-cleanup parity remain open.
The same parity suite now also covers an indirect fixed-layout aggregate with
16-byte extent and 8-byte alignment. LLVM and Cranelift lower the neutral
`MemoryCopy` instruction to their respective overlap-safe native copy paths,
link against the same C entry shim, and both executables verify the copied
bytes. Regular enum typed failures with an integer payload are also compared
through a 16-byte aggregate return slot; both executables verify the
discriminant and payload offsets. The focused suite contains 19 passing tests.
The additional niche-pointer `Option::None` fixture links LLVM and Cranelift
against the same C entry shim and verifies the private null-pointer sentinel
from both executables. This sentinel is backend ABI state only: Actus source
has no `null` literal, and `Option::None` remains the source-level absence
variant; the semantic suite rejects `null` as an undeclared identifier. The
corresponding `Option::Some` fixture passes a pointer payload through both
native backends, links both objects against the same C entry shim, and verifies
that the returned pointer is unchanged. The neutral contract accepts this as a
niche payload without declaring inline enum storage, while both emitters reject
non-pointer niche payloads. Direct aggregate register ABI, ownership
transitions, and runtime cleanup remain outside this evidence.
The shared lowering path now also recognizes generic `Option[abs T]` and
`Option[ins T]` constructors, field loads, variant tests, and payload bindings;
the regression lives in `tests/codegen/shared_lowering.rs` and both LLVM and
Cranelift emitters consume the same niche-pointer enum instruction contract.
`examples/arena_tree.act` still cannot be accepted by LLVM because its return
expression nests `case` expressions inside arithmetic; the current shared body
lowering supports case control flow only at statement, owner, and top-level
return boundaries. The exact LLVM diagnostic remains the follow-up boundary,
not an Option representation failure.
The LLVM parameter-value mapper skips the hidden return-slot parameter when
assigning neutral `BackendValueId`s, so typed-failure payloads cannot alias the
slot pointer. This is covered by the hosted parity fixture.
The direct-register aggregate parity fixture now routes an aggregate value
through a caller into an aggregate callee and returns the callee result. LLVM
and Cranelift both lower and execute this cross-function path with the same
two-register descriptor. The Cranelift implementation expands logical
aggregate arguments into the declared contiguous integer fragments and records
multi-result calls as one logical aggregate value; stack fragments and
non-contiguous registers remain explicit unsupported cases.
Installation requirements, direct dependency trees, clean debug build times,
target-directory sizes, and debug executable sizes are recorded in
`docs/benchmarks/phase-44-backend-build-profile-2026-10-09.md`. The record
states the host and evidence limits explicitly.

## Gate 44.6 — Add ARM32/Thumb target support

- [x] Define the supported ARM architecture profile and Thumb instruction mode.
- [x] Define the Cortex-M ABI, integer widths, alignment, argument and
      callee-saved registers, stack contract, entry point, runtime provider,
      and linker script interface.
- [x] Preserve the required 8-byte stack alignment at every call boundary.
- [x] Define register and stack representations for scalar and aggregate
      arguments and return values before external C interoperability.
- [x] Add ABI fixtures for scalar and aggregate calls in both directions.
- [x] Generate ARM32/Thumb objects through the LLVM backend.
- [x] Link a minimal freestanding image with the explicit runtime provider.
- [x] Add object-level checks for relocations, symbols, sections, alignment,
      and forbidden hosted imports.
- [x] Add emulator or target execution evidence for the minimal image.
- [x] Record the exact LLVM, assembler, linker, target triple, and linker-script
      versions used by the acceptance run.

### Gate 44.6 progress

The first ARM target profile is `thumbv7em-none-eabihf`. The compiler records
this as the Cortex-M v7E-M profile with Thumb-2 instruction mode through the
typed `TargetSpec` contract. The typed ARM contract defines 32-bit words and
pointers, argument and return registers, callee-saved registers, 8-byte stack
alignment, explicit entry, runtime-provider, and linker-script requirements.
The emitted call-boundary proof remains a separate check.

The profile now also exposes the base AAPCS contract: 32-bit words and
pointers, `r0-r3` argument registers, `r0-r1` return registers, `r4-r11`
callee-saved registers, and 8-byte stack alignment. Aggregate register/stack
classification, runtime-provider wiring, and linker-script acceptance remain
separate checks below.

The target contract now explicitly identifies the Cortex-M freestanding link
boundary: a configured entry symbol is required, a linker script is required,
the runtime provider must be explicit, and the stack contract remains
8-byte-aligned. This is a typed target requirement only; it does not claim
that the provider, startup object, or linker script has been supplied yet.

The build manifest now accepts `linker_script = "memory.x"`. The path is
resolved relative to the project manifest and is emitted as `-T <path>` only
for the GNU linker flavor. Configuration tests verify the resolved path and
the deterministic linker-plan argument. This provides the configuration
boundary; it does not claim that the script, startup object, or runtime
provider links successfully.

The LLVM integration suite now links a minimal freestanding ARM ELF image.
The fixture combines an Actus-generated `thumbv7em-none-eabihf` object, an
explicit Thumb startup object, an explicit `__aeabi_unwind_cpp_pr0` provider,
and a pinned test-local `memory.x` script. The parsed image contains the
vector table, `.text`, `Reset_Handler`, and the Actus entry symbol, with no
undefined symbols. The recorded toolchain is LLVM/Clang 23.1.1 for assembly
and object generation, rust-lld LLD 22.1.8 for linking, and target triple
`thumbv7em-none-eabihf`; the linker script content is pinned directly in the
fixture. This is link evidence only and does not claim emulator or hardware
execution.

The representation contract now covers one-word scalar values in `r0` and
stack-spilled aggregates with explicit byte size, alignment, and offset. An
aggregate return uses an explicit indirect return address in `r0`. The ARM
call fixture supplements this neutral contract with emitted Thumb disassembly
evidence: both generated callers contain `bl` and preserve an 8-byte frame
with paired `push {r7, lr}` and `pop {r7, pc}` instructions.

The LLVM backend now initializes the full target registry and emits a real
ARM ELF object for `thumbv7em-none-eabihf`. The fixture verifies the ARM object
architecture, `.text` section, and public Actus symbol. This is host-side
object evidence only; it does not claim linker-script, runtime-provider, or
hardware execution support.

The ARM call fixture now lowers scalar and two-word aggregate caller/callee
pairs through the same backend-neutral `Call` and `Return` contract before
emitting the ARM object. It verifies that both argument flow and return flow
survive LLVM ARM object emission and that all four public symbols are present
in the parsed object. This proves fixture coverage through object generation;
it does not yet prove exact register allocation, freestanding linking, or
target execution.

The linked-image fixture now performs strict object-level acceptance checks:
the vector table is word-aligned and points at the linker-defined stack, the
reset vector selects Thumb state and targets `Reset_Handler`, code sections
are halfword-aligned and inside the declared flash range, the final ELF has
no relocations or undefined symbols, and forbidden hosted imports are
rejected. The linker script therefore has executable acceptance coverage;
instruction-level runtime observation remains limited to the recorded target
programming and reset evidence.

Target execution evidence was obtained on the connected STM32H753 target.
`probe-rs list` found the ST-LINK V3 probe. The generated
`thumbv7em-none-eabihf` ELF was programmed with `probe-rs download` at 50 kHz
SWD using `--verify --reset`, and the command returned exit code 0. A separate
`probe-rs reset` also returned exit code 0. The run used `probe-rs 0.32.0`,
chip `STM32H753ZITx`, probe serial
`002800463335510C33383531`, and image fixture
`/tmp/actus-llvm-arm-image-2124394/image.elf`. This is target programming and
reset evidence; instruction-level runtime observation remains unavailable
because the installed `arm-none-eabi-gdb` cannot load its Python module.

## Gate 44.7 — External C ABI and runtime interoperability

- [x] Define the stable Actus-to-C ABI boundary for external libraries and
      runtime providers.
- [x] Define ownership, lifetime, error, calling convention, alignment, and
      symbol visibility rules for imported C functions.
- [x] Add a minimal external-library fixture that links through the LLVM ARM
      path without putting C or foreign code into Actus source modules.
- [x] Prove that external runtime calls remain explicit and that serialized
      Actus state contains no OS pointer.
- [x] Document how precompiled or separately compiled C libraries may be linked
      without implying that Actus can compile arbitrary C source itself.

The completed portion is implemented by the typed FFI contract in `src/ffi`,
the neutral `BackendExternal` validation, the LLVM external-symbol IR fixture,
and the Linux ARM acceptance fixture
`links_an_explicit_external_c_provider_through_the_llvm_arm_path`. The fixture
compiles `provider.c` separately with Clang, links its object with an Actus
LLVM object, and checks the final ELF for explicit symbols, no relocations, and
no undefined symbols. Aggregate foreign ABI paths, runtime cleanup calls, and
serialized-state pointer exclusion is covered by the semantic pack regression
`rejects_pointer_bearing_or_dynamic_pack_fields_from_serialized_state`, while
external-call explicitness is covered by the LLVM IR and linked-ELF fixtures.

## Gate 44.7.1 — Complete backend selection and semantic-to-backend pipeline

This gate is required before claiming that LLVM is a selectable application
backend. It closes the current boundary where the CLI build pipeline lowers
semantic programs directly through Cranelift while LLVM accepts only manually
constructed backend-neutral contracts.

- [x] Define one public backend selector with explicit `cranelift` and `llvm`
      variants and a deterministic default.
- [x] Add manifest configuration for backend selection with strict parsing,
      validation, inheritance, and a clear error for an unavailable feature.
- [x] Add an explicit build-level override only if it has the same validated
      configuration path as the manifest; do not create a second backend
      selection implementation.
- [x] Route the resolved backend selector through `CompilerConfiguration`,
      build planning, object emission, executable emission, and run commands.
- [ ] Move semantic-to-`BackendModule` lowering behind a shared lowering
      boundary so LLVM and Cranelift consume the same roles, layouts, cleanup
      plans, calls, returns, failures, and external symbols.
- [x] Define neutral block parameters and typed edge arguments for values that
      cross branch and loop back-edges, including source spans and ownership
      transitions.
- [x] Validate block-parameter arity, value types, and predecessor arguments at
      the neutral contract before either backend emits control flow.
- [x] Lower the neutral block-parameter contract to Cranelift block parameters
      and LLVM PHI values without changing the existing ABI or cleanup model.

Evidence: `BackendBlockParameter` and `BackendEdgeArgument` are validated in
the neutral contract before emission. Cranelift appends typed block parameters
and passes typed edge values on `brif` and `jump`; LLVM creates one PHI per
non-entry block parameter and records incoming values at each predecessor
terminator. `codegen_contract` covers neutral arity and type rejection, while
`llvm_backend::emits_block_parameter_phi_from_typed_edge_arguments` exercises
the same fixture through LLVM and Cranelift object emission.

- [x] Route bounded range iteration through the shared loop contract, including
      carried owner values, typed loop block parameters, typed back-edge
      arguments, and hosted CLI fixtures for both backends.
- [x] Route fixed-array iteration through the shared loop contract and add its
      hosted CLI fixtures for both backends.
- [x] Preserve the complete current Cranelift lowering behavior while adding
      the shared path; no backend-specific semantic rewrite or duplicate
      lowering implementation is allowed.
- [ ] Implement the LLVM CLI emission path for the supported shared contract,
      including module units, generic specializations, runtime bridges, entry
      selection, object merging, and linker integration.
- [ ] Make unsupported operations fail with the typed backend diagnostic that
      names the operation, source span, selected backend, and target; never
      silently fall back to the other backend.
- [ ] Add configuration tests for default selection, manifest selection,
      feature-disabled selection, target/backend incompatibility, and invalid
      backend names.
- [ ] Add hosted and freestanding CLI fixtures that build the same source with
      both backends and compare exit status, exported symbols, failure class,
      ownership behavior, runtime bridge resolution, and zero-float evidence.
- [x] Run the standard-library Region and aggregate fixtures through LLVM as
      well as Cranelift, including hosted execution and freestanding object
      acceptance where the target contract permits it.
- [ ] Verify that the selected backend is included in build metadata, artifact
      invalidation, diagnostics, and reproducibility records.
- [x] Verify architecture boundaries: shared semantic/codegen modules contain
      no Cranelift or LLVM imports, and backend implementations remain inside
      their canonical directories.
- [ ] Update the handbook, ADR, and troubleshooting documentation with the
      final selection syntax, feature requirements, supported combinations,
      failure behavior, and rollback procedure.
- [ ] Record exact compiler, LLVM, linker, target, and fixture conditions for
      every completed acceptance test before marking this gate complete.

### Current shared-lowering evidence

The shared semantic lowering boundary now covers scalar owner declarations,
same-scope scalar reassignment, and same-scope scalar compound assignment as
distinct value bindings, including the safe bitwise operators `&`, `|`, and
`^`. The neutral contract rejects non-binding assignment targets and
assignments that would require an outer-scope mutation without an explicit
branch-merge value contract. The regression coverage is
`lowers_scalar_local_assignment_into_a_new_value` and
`lowers_scalar_compound_assignment_into_integer_binary_ir`, and
`lowers_scalar_parameter_assignment_in_the_function_scope`; top-level `erg`
parameters use the same value-rebinding model while nested branch mutation
remains rejected until a merge contract exists. The negative regression
`rejects_outer_scalar_assignment_without_a_branch_merge_contract` verifies that
boundary explicitly.

Generic AST specialization is now owned by the backend-neutral
`src/codegen/generic/` module. Generic verb expansion, type substitution,
transitive instance discovery, specialized naming, and specialized declaration
materialization no longer live under the Cranelift implementation directory.
Both CLI backend paths use the same specialization functions, and LLVM-only
library compilation no longer requires the Cranelift feature for this stage.
Machine-dependent generic layout calculation remains backend-owned. This closes
the generic specialization ownership slice but does not close Gate 44.7.1:
complete shared semantic lowering, LLVM CLI emission coverage, typed unsupported
operation diagnostics, and full cross-backend acceptance remain open.

`llvm_backend_builds_and_runs_shared_scalar_control_flow` builds and executes a
program that declares, reassigns, branches on, and returns a scalar owner.
The LLVM CLI regression
`llvm_backend_builds_and_runs_scalar_assignment_through_shared_lowering` also
builds and executes same-scope compound assignment through the shared scalar
contract. This confirms the assignment slice at the real LLVM CLI boundary;
branch-merge mutation and aggregate assignment remain outside this evidence.
The shared CLI candidate boundary now also admits a bounded non-generic Result
slice: primitive success payloads, unit-only user error enums, explicit Ok/Err
constructors, and expression-bodied case returns without guards or block
branches. The LLVM and Cranelift CLI regressions
`llvm_backend_builds_and_runs_a_scalar_result_program` and
`cranelift_backend_builds_and_runs_a_scalar_result_through_shared_lowering`
build the same Actus source and verify exit status 42 from both executables.
This does not claim Region, aggregate assignment, generic Result, or block-
branch CLI parity.
The shared scalar contract now also lowers integer division and remainder with
an explicit `TrapIfZero` guard before the arithmetic instruction. Both
Cranelift and LLVM lower that guard to a terminating zero-divisor path; the
LLVM CLI regression `llvm_backend_builds_and_runs_checked_scalar_division_and_remainder`
executes `20 / 5 + 17 % 6` and verifies the result. The neutral regression
`lowers_scalar_division_and_remainder_with_zero_guards` verifies the ordering
of each guard and its arithmetic operation. The shared scalar contract now
also emits an explicit `TrapIfSignedDivisionOverflow` guard for signed
division; `emits_signed_division_overflow_guard_for_runtime_operands` verifies
that LLVM preserves the guard for runtime operands. Short-circuit logical
operators remain separate contract work and are not implied by this evidence.
The shared scalar contract now also lowers signed
left and right shifts with an explicit `TrapIfShiftCountOutOfRange` guard;
`lowers_scalar_shifts_with_width_guards` checks the neutral ordering and
`llvm_backend_builds_and_runs_checked_scalar_shifts` executes the LLVM path.
This is incremental shared-pipeline evidence; it does not close the remaining
aggregate, cleanup, runtime-bridge, generic, target, or cross-backend items in
this gate.

The shared capability classifier now admits bounded `Buffer` and `Region`
values, including `ins` and `dat` parameter roles and bounded Buffer literals.
The classifier regression `accepts_bounded_buffer_and_region_signatures` and
the native aggregate Region fixture both pass. This expands the candidate
boundary for the next shared-lowering slices; it does not claim complete
Cranelift migration or full backend parity.

The classifier also admits block-bodied `Result` cases whose terminal paths
return supported shared values. The regression
`accepts_block_bodied_result_cases_in_the_shared_contract` covers this shape,
and the aggregate Region suite remains green after the expansion. Nested
ownership cleanup and generic runtime calls still require backend execution
evidence before this boundary can be considered complete.

The hosted parity fixture
`aggregate_region_lifecycle_has_matching_hosted_backend_results` now builds
the same `Region[Array[u8,16]]` source with explicit `cranelift` and `llvm`
selection and verifies exit status 0, empty stdout, and empty stderr from both
executables. This is hosted result parity evidence for the covered lifecycle;
it does not yet prove identical generated symbols, freestanding acceptance, or
complete shared lowering for every ownership path.

The freestanding fixture
`freestanding_region_objects_have_compatible_backend_runtime_symbols` builds
the same standard-library Region source as x86_64 UEFI objects with both
backends. After excluding optional cleanup bridges and the LLVM target helper
`__chkstk`, both objects expose the same Region generic-open and Region-drop
runtime symbols, with no unexpected non-Actus imports. Explicit `drop`
statements are now represented as semantic facts and lowered through the same
validated cleanup bridge as automatic ownership cleanup. The hosted parity
fixture `aggregate_region_lifecycle_has_matching_hosted_backend_results`
exercises an explicit `Region[Array[u8, 16]]` drop on both selected backends,
while the semantic regression `records_explicit_drop_for_backend_lowering`
verifies that the explicit action is removed from the later scope cleanup
plan. Scalar drops remain valid no-op cleanup operations; custom user-defined
Drop performance lowering remains outside the shared builtin bridge contract.

The first neutral aggregate-layout slice is now program-backed: the shared
signature contract derives non-generic user-defined struct size and alignment
from Actus declarations, recursively handles nested structs, treats `abs` and
`ins` fields as pointer-width references, and rejects recursive, malformed, or
overflowing layouts by withholding the unsupported layout. The regression
`derives_nested_struct_layouts_in_the_neutral_signature_contract` verifies the
same deterministic layout metadata and indirect aggregate passing mode without
importing Cranelift or LLVM types. This is layout groundwork only; user-defined
`Drop` performance body lowering and backend emission remain open.

This layout slice now also registers named `pack` declarations as their
validated storage layout, including fixed arrays whose elements are packs. The
neutral regression `derives_array_layout_from_pack_storage_in_the_neutral_contract`
verifies the deterministic size and alignment for `Array[Register, 2]`. Pack
storage construction is therefore represented in the shared layout contract;
packed bit-field reads and writes remain a separate open instruction-contract
slice and are not treated as ordinary byte-offset aggregate fields.
The next contract slice now exposes validated packed-field metadata
(`bit_offset`, `width`, storage layout, and endianness) through the neutral
layout provider. The regression
`exposes_pack_bit_field_metadata_to_the_neutral_contract` verifies the metadata
for a little-endian `u1` field. Backend bit extraction and update instructions now consume this metadata
through the shared `PackedFieldLoad` and `PackedFieldStore` contract. The
regression `both_backends_consume_shared_array_pack_bit_fields` verifies LLVM
IR emission and Cranelift object emission for an array-backed little-endian
`u1` field. A separate hosted fixture also builds and executes the same source
with LLVM and returns the updated field value. Unit-return calls are lowered as
statement-level calls with no fabricated result; a focused hosted LLVM fixture
verifies that contract. Arena construction and bounded placement are now also
represented by neutral `ArenaConstruct` and `ArenaPlace` instructions. Both
backend emitters allocate bounded arena storage, maintain the in-arena offset,
check aligned capacity before placement, and copy fixed-size aggregate values
into the selected location. The neutral regression
`lowers_arena_constructor_and_placement_into_neutral_instructions` and the
hosted LLVM regression
`llvm_backend_builds_and_runs_phase18_arrays_and_packs_example` cover this
slice together with arrays, indexed mutation, and packed fields. Big-endian
packed emission remains a separate open slice.

The neutral cleanup provider now uses the same program-backed layout source for
builtin cleanup bridges and nested owned `Buffer` or `Region` field descriptors.
Field addresses carry the complete root aggregate layout plus checked offset,
size, and alignment metadata, and traversal stops at borrowed or exclusive
reference fields. Backend-specific layout registries are no longer required to
describe this cleanup operand contract; user-defined `Drop` body lowering and
emission remain open.

User-defined `Drop` is now detected from the program declarations during
neutral cleanup resolution. The shared descriptor carries a validated
      `PerformanceDrop` bridge and deterministic `SymbolIdentity` for the declared
performance. The declared performance method is also lowered into the shared
`BackendModule` as a typed `SymbolKind::Performance` function, and both LLVM
and Cranelift resolve cleanup calls to that local function symbol. The
regressions `records_user_defined_drop_with_a_neutral_performance_bridge_symbol`
and `both_backends_emit_user_defined_drop_performance_calls` cover the neutral
function contract and hosted backend emission. This closes the user-defined
`Drop` body and local-call slice only; generic performance dispatch, complete
CLI candidate admission, freestanding runtime integration, and the complete
Gate 44.7.1 remain open.

Build metadata now has an integration regression,
`backend_change_invalidates_generated_artifacts`, proving that an artifact
written under the Cranelift configuration is invalidated when the selected
backend changes to LLVM. Backend metadata and target metadata are preserved in
the artifact contract; reproducibility and diagnostic record coverage remain
open.

An exploratory LLVM CLI build of the existing aggregate Region lifecycle
fixture reached the shared signature boundary and was rejected with the typed
diagnostic `backend llvm does not support semantic-signature ... no neutral
layout is registered for Result`. The fixture was not accepted as evidence:
supporting `Region[Array[...]]` through LLVM requires a complete neutral generic
`Result` enum contract, including its layout, constructors, case/failure
lowering, return ABI, and runtime bridge compatibility. A partial layout entry
must not be used as a workaround.

The first neutral Result slice is now implemented without closing the gate:
`BackendValueType::Enum` carries validated representation metadata, the shared
lowering path resolves scalar `Result[T, E]` layouts, and normalized `Ok` and
`Err` constructors become explicit `EnumConstruct` instructions. LLVM emits
the discriminant and payload into the declared aggregate representation and
return slot. Focused regressions cover neutral construction and LLVM IR
emission. Result case matching, aggregate and Region payloads, Cranelift
parity, and complete CLI execution remain open until their own implementation
and evidence are present.

The neutral case slice now also lowers `Result.Ok` and `Result.Err` variant
tests into discriminant extraction, integer comparison, explicit branch blocks,
and scalar return paths. Scalar payload bindings are represented by the
explicit `EnumPayloadBinding` instruction, which records `plain`, `abs`, or
`dat` ownership mode; LLVM consumes the same payload extraction contract.
Wildcard payload patterns are accepted without creating a binding. Result
payload metadata is keyed by both the subject value and discriminant, so an
`Ok` payload and an `Err` payload may have different types. Direct Region
payload construction, a mixed `Result[Region[u32], Int]` case binding, and a
call result carrying that mixed payload shape are covered by integration
regressions. The shared regression
`lowers_plain_result_case_into_neutral_branches`, the integration regressions
`lowers_dat_and_abs_result_payload_bindings_with_explicit_modes`,
`lowers_result_constructor_with_region_payload_layout`, and
`preserves_distinct_result_payload_types_for_case_bindings`,
`preserves_result_payload_types_for_call_results`, and the LLVM
regressions `emits_plain_result_case_from_shared_lowering` and
`emits_region_payload_result_constructor_from_shared_lowering` cover this
slice. Cranelift now consumes discriminant extraction, scalar payload bindings
from indirect enum storage, and scalar `Result` construction into an enum
return slot; the backend regressions
`cranelift_consumes_scalar_result_case_from_shared_lowering` and
`cranelift_emits_scalar_result_constructor_to_return_slot` cover this
boundary. Shared case lowering keeps payload extraction inside the target
branch block before its terminator. Cranelift also emits Region payload
construction by bounded memory copy into the enum return slot and exposes
Region case payloads as bounded payload pointers. Return-slot calls now use
the same explicit ABI as LLVM: the caller allocates a validated bounded stack
slot, passes its pointer as the hidden first argument, and retains that pointer
as the neutral aggregate or enum value. A hosted parity regression executes a
callee that constructs a Result, a caller that extracts its discriminant, and
the same C entry shim through both backends. The cleanup parity fixture now
also covers the EnumDrop bridge, checking the enum pointer, byte size, and
Buffer-Region-Enum ordering through both generated executables. The remaining
open work is full aggregate runtime behavior beyond the covered contract and
complete CLI execution.

Explicit `drop` statements are now admitted by the shared CLI candidate
boundary. The neutral cleanup contract handles builtin `Buffer` and `Region`
drops through validated runtime bridges, while scalar drops lower to no
backend instruction after semantic ownership validation. The external
regression `lowers_explicit_scalar_drop_without_backend_cleanup` covers the
scalar no-op contract, and the existing hosted Region parity fixtures cover
explicit Region cleanup through both native backends. User-defined `Drop`
performances remain outside this candidate until neutral aggregate layout,
performance declarations, and performance-call lowering are available.

The Region acceptance slice is now closed by the hosted LLVM/Cranelift parity
fixture, the LLVM CLI aggregate lifecycle fixture, and the freestanding object
fixture that compares the selected backend runtime symbols. The same validation
run also passed the architecture conformance suite, including the checks that
shared codegen contains no backend-specific imports and that backend
implementations remain inside their canonical directories. These results close
only the two corresponding checklist items; complete backend selection and
semantic-to-backend migration remain open.

The neutral lowering boundary now materializes compile-time integer casts as
typed constants after validating the target range. It also lowers runtime
integer casts as a checked `IntegerCast` instruction carrying both source and
target integer shapes. Cranelift and LLVM widen the source, validate signed or
unsigned bounds, branch to a native integer-overflow trap on failure, and only
then narrow to the target width. The neutral regression
`lowers_runtime_integer_casts_with_checked_neutral_instruction`, the existing
Cranelift execution regression `executes_checked_integer_casts`, and the LLVM
regression `emits_checked_runtime_integer_cast_from_shared_contract` cover this
contract. The LLVM const-generic regression still verifies `N as u32` after
specialization. This is incremental shared-pipeline evidence; the complete
backend-selection and CLI integration gate remains open.

The regression `shared_lowered_module_is_consumed_by_llvm_and_cranelift` now
parses one scalar Actus source, lowers it once into `BackendModule`, and sends
that same neutral module through both LLVM IR emission and Cranelift object
emission. This proves shared-contract consumption for the covered scalar slice;
it does not yet prove complete CLI routing for all language features.

The Cranelift CLI now uses the same shared route for the explicitly eligible
non-generic scalar slice across the root unit and imported module units. The
slice includes straight-line scalar expressions, direct-return branch-local
`if/else` control flow, same-scope scalar reassignment and compound assignment,
and calls to declared scalar verbs or external verbs;
imported verb bindings resolve to their namespaced
symbols, and local declarations retain precedence. Target-annotated
declarations are filtered before eligibility and lowering. Programs outside
that contract continue through the established Cranelift lowering until their
constructs receive shared lowering support; this staged route preserves the
existing behavior while the remaining semantic surface is migrated. The
hosted regressions `cranelift_backend_builds_and_runs_scalar_imported_module_through_shared_lowering`
and `cranelift_backend_builds_and_runs_scalar_if_through_shared_lowering` build
and execute imported-call and branch-local scalar programs.

The LLVM CLI now follows the existing generic-instance discovery and
specialization pipeline before neutral lowering, applies target filtering to
each object unit, and preserves the object-plan generic type support needed by
specialized declarations. The hosted regression
`llvm_backend_builds_and_runs_a_scalar_const_generic` builds and executes a
scalar const-generic call through LLVM. This evidence covers specialization
of the declaration and call symbol; generic compile-time expression lowering,
aggregate specialization, ownership-bearing generic values, cleanup bridges,
and cross-backend parity remain open work.

The hosted regression
`llvm_backend_builds_and_runs_a_const_generic_from_an_imported_module` extends
that evidence to a generic declaration owned by an imported module. The CLI
collects the instance from the module implementation unit, specializes the
module declaration, emits its namespaced specialized symbol, and links the
root call through the LLVM object set. This covers the imported-module generic
specialization slice; aggregate specialization, ownership-bearing generic
values, cleanup bridges, and cross-backend parity remain open work.

The Cranelift CLI now sends the same bounded scalar const-generic slice through
the shared neutral route after specialization. The regression
`cranelift_backend_builds_and_runs_a_scalar_const_generic_through_shared_lowering`
builds and executes `read_capacity[4]()` and verifies exit status 4. This
extends shared generic coverage to both native backends for scalar values. It
does not claim generic aggregate or Region support, runtime bridge parity, or
complete standard-library CLI execution.

The LLVM implementation keeps backend responsibilities behind the canonical
facade and now separates failure-value lowering into `failures.rs` and memory
copy lowering into `memory.rs`, and LLVM IR printing/module verification into
`verification.rs`; `control_flow.rs` coordinates block emission without owning
those data-layout or module-validation operations. Branch/jump target
resolution now lives in `branches.rs`, while the explicit unsupported cleanup
boundary lives in `cleanup.rs`; module construction and function assembly live
in `module.rs`. This is a structural
maintainability step and does not claim LLVM and Cranelift feature parity.
Neutral cleanup plans now also require one bridge descriptor per cleanup action;
the contract rejects misaligned action and descriptor sequences before backend
emission. This prevents a backend from silently dropping or reordering a
runtime cleanup operation.

The shared CLI lowering boundary now accepts bounded `Buffer[N]` expressions by
lowering their length to the declared unsigned ABI width and emitting the
neutral runtime allocation call. Typed integer literal suffixes are preserved
through this path, so `u64` Region bridge arguments retain their native width.
Block-bodied `Result` case branches now lower through the same neutral body
pipeline, with outer-scope cleanup distributed to nested terminal blocks before
backend emission. LLVM enum construction and payload extraction now preserve
the indirect pointer representation required by aggregate and `Region` payloads;
aggregate payloads are copied into the declared inline storage with the
validated size and alignment. The hosted `std::region` lifecycle fixture,
aggregate Region suite, and the complete
`cargo test --all-targets --all-features -- --test-threads=1` suite pass,
including LLVM zero-float verification and the 128 KiB bounded bulk fixture.
Source limits and formatting pass. This is implementation evidence for the
Region and aggregate portion of the shared pipeline; Gate 44.7.1 remains open
until the remaining backend-selection, unsupported-diagnostic, freestanding
parity, metadata, and documentation requirements are completed.

The LLVM CLI path now collects generic instances from module implementation
units as well as the root caller, emits the corresponding specialized module
symbols, and resolves the standard-library Region runtime bridge without a
missing specialization. Runtime externals that return composite values now use
an explicit pointer result ABI: LLVM loads the declared aggregate from the
returned storage and releases that storage through the typed runtime cleanup
bridge. The regression
`aggregate_region_lifecycle_builds_and_runs_through_llvm_cli` builds and runs
`Region[Array[u8,16]]` through the real LLVM CLI path. The focused evidence is
9 aggregate Region tests, 12 standard-library Region tests, and 41 LLVM
backend tests with zero failures; this closes the tested aggregate Region ABI
slice while the complete gate remains open.

The real LLVM CLI path now also builds and runs a source unit that declares a
user-defined `Drop` performance alongside the selected entry verb. The
regression `llvm_backend_builds_and_runs_a_user_defined_drop_performance`
verifies manifest backend selection, executable emission, linking, and exit
status 42. The fixture now constructs a scalar-field `Counter` owner, so its
scope cleanup emits and executes the declared performance bridge. The neutral
regression `lowers_aggregate_owner_construction_into_scoped_drop_cleanup` and
the backend regression `both_backends_emit_user_defined_drop_performance_calls`
cover the same construction and cleanup contract for the neutral module,
LLVM IR, and Cranelift object paths. This closes only the non-generic
scalar-field aggregate construction slice; nested aggregate fields, aggregate
field access and assignment, generic aggregate values, and full ownership
mutation parity remain open.

The aggregate field-read slice adds scalar and pointer field reads from a neutral
aggregate value. `AggregateFieldLoad` carries the complete root layout, checked
field offset, field layout, and value type; the contract rejects out-of-bounds,
misaligned, and aggregate-valued loads until a dedicated aggregate-copy result
contract exists. Both Cranelift and LLVM consume the same instruction, and
hosted CLI fixtures build and execute the same `Counter.value` program with
exit status 42 through each backend. The neutral regression and malformed
layout regression cover instruction construction and rejection. Aggregate
field mutation, nested aggregate field reads, and generic aggregate values
remain open.

The following aggregate mutation slice adds scalar and pointer field assignment
through `AggregateFieldStore`. The store carries the root aggregate layout as
well as the checked field layout, offset, and value type, so the neutral
contract rejects out-of-bounds, misaligned, and aggregate-valued stores before
either backend emits code. Cranelift and LLVM execute the same `Counter.value`
assignment fixture and return 42 through the CLI. Compound field assignment,
nested aggregate field reads, and generic aggregate values remain open.

The compound field-assignment slice now reuses the same neutral contract as
ordinary field assignment: `AggregateFieldLoad`, one checked integer binary
operation, and `AggregateFieldStore`. The shared regression verifies the
load-operation-store order, while the Cranelift and LLVM CLI fixtures execute
`counter.value += 1` and return 42. Compound assignment remains limited to
scalar and pointer fields; nested aggregate values, generic aggregate values,
and branch-merge mutation remain open.

The Cranelift shared candidate now also admits checked scalar division,
remainder, and left/right shift expressions. These operations already emit
the neutral trap contracts for zero divisors, signed division overflow, and
out-of-range shift counts; the CLI regression executes the shared Cranelift
path and returns 42. The LLVM CLI suite continues to cover the corresponding
checked operations. This expands shared scalar parity without changing the
legacy fallback path or closing the broader backend-pipeline gate.

The shared candidate now admits loop bodies as well. Loop-scoped owner
assignment updates an enclosing owner only while the body is inside an active
loop context; ordinary branch assignment still requires the not-yet-available
branch-merge contract and remains rejected. The Cranelift CLI fixture executes
a loop with an explicit `break` and returns 42, while the existing shared
lowering regression continues to reject unsupported outer branch mutation.

The shared scalar contract now preserves declared integer widths for owner
initializers and binary operations. Return lowering inserts a checked integer
cast when a narrow scalar expression is returned through a wider declared
integer result, while aggregate and enum return values retain their existing
neutral representations. The Cranelift regression for an unsuffixed `u8`
initializer and an out-of-range shift now traps as required. The complete
all-targets/all-features suite passed with zero failures, including 137 library
tests, 49 CLI tests, 92 codegen tests, 44 LLVM backend tests, and the existing
standard-library, runtime, target, and semantic suites.

The neutral contract now lowers bounded range loops through typed loop block
parameters and edge arguments. Loop-carried owner values are preserved across
the header, body, increment, and continuation blocks; the codegen contract
tests validate both side-effect-free and carried-owner loops. Hosted CLI
fixtures build and execute the same carried-owner range program through both
Cranelift and LLVM and return the expected result.

The CLI shared-candidate classifier now admits the bounded range-loop slice
when the bounds and binding are integer expressions and the body uses only the
already-supported scalar loop operations. The classifier regression
`accepts_bounded_range_iteration_in_the_shared_contract` and the hosted
Cranelift and LLVM fixtures `cranelift_backend_builds_and_runs_a_shared_range_loop`
and `llvm_backend_builds_and_runs_a_shared_range_loop` verify that this source
is accepted by the shared route and executes with the same result. Branch
mutation remains rejected by the classifier until a branch-merge value
contract exists.

Fixed-array iteration now uses the same neutral loop contract. Shared lowering
preserves the declared `Array[T, N]` nominal type, validates the concrete
capacity and recursive element layout, lowers `Array[T, N]()` to a bounded
zero-initialized aggregate, and emits typed element loads for the loop body.
The hosted CLI fixtures
`cranelift_backend_builds_and_runs_a_shared_fixed_array_loop` and
`llvm_backend_builds_and_runs_a_shared_fixed_array_loop` compile the identical
source through the selected backend and execute it with exit status `0`.
The focused acceptance evidence is `cargo test --test cli --all-features
fixed_array_loop -- --test-threads=1`; the neutral and backend suites also
pass with 44 and 46 tests respectively. The broader backend-pipeline gate
remains open until its remaining semantic lowering, unsupported-operation,
configuration, reproducibility, and documentation requirements are complete.

The Cranelift shared-candidate classifier now admits the fixed-array slice only
when the source has a concrete non-zero `Array[T, N]` constructor, an integer
array-loop binding, and scalar indexed reads. The positive classifier regression
`accepts_fixed_array_iteration_in_the_shared_contract` and the hosted
Cranelift fixture prove that this source reaches the neutral `BackendModule`
route; unsupported constructs continue through the established legacy
Cranelift path. The complete all-targets/all-features regression suite remains
green, including the existing array, aggregate, generic, ownership, runtime,
and freestanding Cranelift fixtures. No Cranelift-specific semantic lowering
was added to shared modules and no legacy behavior was removed.

Direct fixed-array literal-index reads now use the same guarded shared route.
The CLI classifier admits an indexed expression only when its base is a
declared fixed `Array[T, N]` binding or a concrete fixed-array constructor,
the index is an integer literal, and the expression is a value read rather
than an indexed loan argument. Neutral `ArrayElementLoad` lowering then
validates the nominal element type, concrete capacity, recursive layout, and
integer index before either backend emits code. The classifier regression
`accepts_direct_fixed_array_index_reads_in_the_shared_contract` and the hosted
fixtures `cranelift_backend_builds_and_runs_a_shared_fixed_array_index_read`
and `llvm_backend_builds_and_runs_a_shared_fixed_array_index_read` compile and
execute the identical source through both selected backends. This evidence
covers literal-index reads only; array stores, dynamic indexing,
indexed loan arguments, and the remaining
semantic-to-backend migration stay outside this slice.

Fixed-array literal-index reads from `Array[T, N]` parameters now use the same
shared candidate contract as local fixed-array reads. The classifier regression
`accepts_fixed_array_parameter_reads_in_the_shared_contract` and the hosted
fixtures `cranelift_backend_builds_and_runs_a_shared_fixed_array_parameter_read`
and `llvm_backend_builds_and_runs_a_shared_fixed_array_parameter_read` compile
and execute the identical parameter-passing source through both selected
backends. This validates the current nominal aggregate parameter ABI slice;
array mutation, dynamic indexing, indexed loan arguments, and the remaining
semantic-to-backend migration stay outside this slice.

Scalar-field-only value structs are now admitted at aggregate call boundaries.
The classifier regression
`accepts_scalar_struct_parameter_reads_in_the_shared_contract` and the hosted
fixtures `cranelift_backend_builds_and_runs_a_shared_scalar_struct_parameter_read`
and `llvm_backend_builds_and_runs_a_shared_scalar_struct_parameter_read` compile
and execute the same `Counter` parameter/read source through both selected
backends. The slice covers read-only scalar fields in non-generic value
structs; ownership-bearing fields, mutable indexed stores, generic structs, and
the remaining semantic-to-backend migration stay outside this slice.

Exclusive mutable loans of scalar-field-only value structs are now covered at
the same call boundary. The classifier regression
`accepts_scalar_struct_parameter_mutation_in_the_shared_contract` and the
hosted fixtures
`cranelift_backend_builds_and_runs_a_shared_scalar_struct_parameter_mutation`
and `llvm_backend_builds_and_runs_a_shared_scalar_struct_parameter_mutation`
compile and execute an `ins Counter` mutation through both selected backends.
The fixture validates field compound assignment, aggregate loan passing, and
the caller's post-call read; nested ownership fields, indexed stores, generic
structs, and the remaining semantic-to-backend migration stay outside this
slice.

Fixed-array literal-index stores now use the shared aggregate contract. The
classifier regression
`accepts_fixed_array_element_stores_in_the_shared_contract` and the hosted
fixtures
`cranelift_backend_builds_and_runs_a_shared_fixed_array_element_store` and
`llvm_backend_builds_and_runs_a_shared_fixed_array_element_store` compile and
execute the same `Array[u32, 4]` store/read source through both selected
backends. The neutral contract validates element layout, capacity, scalar
value type, and integer index before emission; dynamic indexing, indexed
compound assignment, aggregate element stores, and the remaining
semantic-to-backend migration stay outside this slice.

Fixed-array literal-index compound stores now use the same shared contract.
The classifier regression
`accepts_fixed_array_element_compound_stores_in_the_shared_contract` and the
hosted fixtures
`cranelift_backend_builds_and_runs_a_shared_fixed_array_element_compound_store`
and
`llvm_backend_builds_and_runs_a_shared_fixed_array_element_compound_store`
compile and execute the same `Array[u32, 4]` compound update through both
selected backends. Shared lowering emits the element load, typed integer
operation, and element store as one validated sequence. Indexed compound
assignment on parameters, dynamic indexing, and aggregate element values stay
outside this slice.

Dynamic integer indices for local fixed arrays now pass through the shared
contract with an explicit `TrapIfArrayIndexOutOfBounds` instruction emitted
before every element load or store. The classifier regression
`accepts_dynamic_fixed_array_indices_in_the_shared_contract` and the hosted
Cranelift/LLVM fixtures
`cranelift_backend_builds_and_runs_a_shared_dynamic_fixed_array_index` and
`llvm_backend_builds_and_runs_a_shared_dynamic_fixed_array_index` verify the
valid path. The corresponding out-of-bounds fixtures verify a native trap on
both backends. This slice covers integer literals and simple integer bindings
for local arrays; indexed parameter mutation, compound index expressions, and
aggregate element values remain outside it.

Shared conditional lowering now preserves a fallthrough continuation block
when both branches reach that block through an explicit jump. The neutral
regression `preserves_fallthrough_continuation_block_when_both_if_branches_jump_to_it`
validates the block identity before backend emission. This removes the prior
`unknown block` failure from the LLVM CLI path for
`examples/phase19_scalar_ergonomics.act`; that example now reaches its runtime
link integration boundary. The remaining `region` example failure is a
separate scalar-contract diagnostic and is not marked as resolved by this
control-flow slice.

The shared CLI contract now lowers the `print` intrinsic to the typed hosted
runtime bridges `actus_print_int`, `actus_print_string`, and
`actus_print_buffer_stdout`, with corresponding neutral external declarations.
The LLVM CLI regression `llvm_backend_builds_and_runs_shared_print_runtime_bridge`
builds and executes a program containing `print(42)` and verifies exit status
42. This establishes the hosted integer print bridge; the remaining standard
library console intrinsics and freestanding provider coverage stay open.

No checkbox in Gate 44.8 may be closed from the existing contract-level LLVM
fixtures until this gate proves that a real CLI build can select and execute
the corresponding backend through the shared semantic pipeline.

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

The neutral aggregate layout now records whether a struct field is stored
indirectly for `abs` and `ins` fields. Shared field lowering emits a pointer
value for those fields and preserves the field's nominal type separately, so
nested arena references can be resolved without treating an address as an
inline aggregate. Arena placement likewise records its result as a pointer
while retaining the placed aggregate's nominal type. The LLVM-only and
all-feature `arena_graph` fixtures both pass, as do the 47 LLVM backend
contract tests, the 40 Cranelift application tests, the 95 Cranelift codegen
tests, architecture conformance, strict Clippy, and source-limit checks.

The full LLVM-only integration matrix is still open. The current run reports
23 failing integration targets, including nested case/body lowering, generic
aggregate layout, Buffer and IO module linking, array and pack lowering,
Option and enum representation, return-slot and void-call ABI behavior, and
runtime failure paths. These failures remain recorded as implementation work;
they do not close Gate 44.7.1 and no related checkbox is marked complete.

## Out of scope

- Rewriting the Actus semantic analyzer for a particular backend.
- Adding application-specific protocol or operating-system functionality.
- Claiming STM32 execution from a host-only LLVM build.
- Replacing Cranelift before LLVM reaches equivalent shared-contract coverage.
- Adding dynamic allocation, hidden runtime state, or unbounded collections to
  make backend integration easier.
