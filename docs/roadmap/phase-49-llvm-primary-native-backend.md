# Phase 49 — LLVM primary native backend

## Status

In progress. The complete backend baseline and the initial LLVM type-contract
hardening are complete; the next work is shared-contract lowering. LLVM
remains opt-in and the
default backend remains Cranelift until the shared-contract, target, runtime,
and parity work below is complete. No metadata contract is changed by this
plan.

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

- [x] Inventory every LLVM-only path and classify it as complete, partial,
      unsupported, or untested.
- [x] Inventory all shared lowering operations and map each to an LLVM emitter.
- [x] Record the supported LLVM version, installation contract, linker policy,
      target matrix, and clean-checkout setup.
- [x] Capture the focused LLVM-only baseline before changing the default.
- [x] Capture complete baseline results for LLVM-only, Cranelift-only, and
      all-features configurations before changing the default.
- [x] Add stable diagnostics for every intentionally unsupported LLVM operation.

#### 49.1 inventory evidence

The initial inventory identifies 32 LLVM implementation files under
`src/codegen/llvm/`, grouped by responsibility:

| Responsibility | LLVM-owned paths | Initial classification |
| --- | --- | --- |
| Module and emission boundary | `mod.rs`, `context.rs`, `module.rs`, `declarations.rs`, `emission.rs`, `object.rs`, `instruction.rs`, `verification.rs` | Implemented boundary; end-to-end acceptance remains open |
| Values and scalar operations | `values.rs`, `scalar.rs`, `scalar_loan.rs`, `literals.rs`, `float.rs`, `integer/mod.rs`, `integer/cast.rs`, `integer/ops.rs` | Implemented coverage; scalar integer widths `u1..=u128` are represented directly by LLVM |
| Control flow and calls | `branches.rs`, `control_flow.rs`, `guards.rs`, `failures.rs`, `calls.rs`, `returns.rs` | Implemented focused coverage; full shared-operation mapping remains open |
| Aggregate and layout lowering | `aggregates.rs`, `arrays.rs`, `arena.rs`, `enums.rs`, `memory.rs`, `packed/mod.rs`, `packed/wide.rs` | Implemented focused coverage; full ABI and target matrix remains open |
| Cleanup and ownership bridges | `cleanup.rs`, `cleanup/bridge.rs`, `cleanup/bridge/address.rs` | Implemented focused coverage; complete ownership-path audit remains open |

The compiler integration surface is feature-gated through `Cargo.toml`,
`src/codegen/mod.rs`, `src/cli/build/llvm_emission.rs`, and the LLVM-specific
test target `tests/codegen/llvm/backend.rs`. The current policy is:

- LLVM feature: `llvm-sys 231.0.0` with strict versioning;
- default feature: `cranelift`;
- LLVM-only focused backend suite: `34 passed, 0 failed`;
- LLVM-only library suite: `148 passed, 0 failed`;
- defensive invalid-type path: `src/codegen/llvm/values.rs`, which rejects
  integer widths outside `1..=128` and floating-point widths other than `32`
  or `64` with deterministic backend errors.

These results are inventory evidence, not completion evidence. The remaining
49.1 checks must run the complete backend configurations and replace
intentional incomplete paths with either full implementations or stable,
source-spanned diagnostics.

#### 49.1 configuration baseline evidence

The repaired complete local comparison produced these results:

| Configuration | Command | Result | Classification |
| --- | --- | --- | --- |
| LLVM-only | `cargo test --no-default-features --features llvm --all-targets -- --test-threads=1` | exit 0; all active suites passed | Complete baseline |
| Cranelift-only | `cargo test --no-default-features --features cranelift --all-targets -- --test-threads=1` | exit 0; all active suites passed | Compatibility baseline |
| All features | `cargo test --all-targets --all-features -- --test-threads=1` | exit 0; all active suites passed | Combined-feature baseline |

The baseline repairs were kept at the correct backend/shared boundaries:

- shared runtime-call normalization now coerces bridge arguments to their
  declared integer widths, including buffer append and i64 runtime contracts;
- LLVM checked integer casts use the native trap bridge, preserving the same
  failure contract as the shared lowering path;
- the zero-float IR policy is shared by Cranelift and LLVM before object
  emission;
- backend-specific object and symbol assumptions are scoped to the backend
  that owns them, while semantic and cross-backend coverage remains active.

The complete-baseline checkbox is closed by the three passing configurations.
The diagnostics checkbox is closed for the current LLVM inventory. User-facing
unsupported operations are rejected through the source-spanned shared
`BackendContractError::Unsupported` contract. Backend-only invariant failures,
including invalid type widths, use deterministic messages and regression tests;
they are not presented as unsupported source operations. New unsupported paths
must extend this contract before they are introduced.

#### 49.1 shared-operation mapping

The shared lowering contract currently exposes 37 `BackendInstruction`
operations. Each operation has one explicit LLVM dispatch path:

| Shared operation family | Operations | LLVM dispatch and implementation |
| --- | --- | --- |
| Constants and literals | `Constant`, `FloatConstant`, `StringLiteral` | `llvm/instruction.rs` dispatches to `llvm/scalar.rs`, `llvm/float.rs`, and `llvm/literals.rs` |
| Integer and floating operations | `IntegerBinary`, `IntegerCast`, `FloatCast`, `IntegerCompare`, `FloatCompare`, `IntegerUnary`, `ScalarSelect` | `llvm/scalar.rs` dispatches to `llvm/integer/`, `llvm/float.rs`, and LLVM select emission |
| Checked traps | `TrapIfZero`, `TrapIfSignedDivisionOverflow`, `TrapIfShiftCountOutOfRange`, `TrapIfArrayIndexOutOfBounds` | `llvm/scalar.rs` dispatches to `llvm/guards.rs` |
| Enum operations | `EnumDiscriminant`, `EnumPayload`, `EnumPayloadBinding`, `EnumConstruct` | `llvm/scalar.rs` dispatches to `llvm/enums.rs` |
| Calls and terminal control | `Call`, `Return`, `Fail`, `Trap` | `llvm/instruction.rs` dispatches to `llvm/calls.rs`, `llvm/returns.rs`, `llvm/failures.rs`, and the LLVM unreachable terminator |
| Block control flow | `Branch`, `Jump` | `llvm/instruction.rs` dispatches to `llvm/branches.rs` |
| Cleanup and memory | `Cleanup`, `MemoryCopy`, `AggregateFieldStore` | `llvm/instruction.rs` dispatches to `llvm/cleanup.rs`, `llvm/memory.rs`, and `llvm/aggregates.rs` |
| Scalar loans | `ScalarLoanLoad`, `ScalarLoanStore`, `ScalarLoanMaterialize` | `llvm/scalar.rs` dispatches to `llvm/scalar_loan.rs` |
| Arena operations | `ArenaConstruct`, `ArenaPlace` | `llvm/instruction.rs` dispatches to `llvm/arena.rs` |
| Aggregate operations | `AggregateConstruct`, `AggregateCopy`, `AggregateFieldLoad`, `PackedFieldLoad`, `PackedFieldStore`, `ArrayElementLoad`, `ArrayElementStore` | `llvm/instruction.rs` dispatches to `llvm/aggregates.rs`, with layout and packed helpers in `llvm/packed/` |

LLVM value-type mapping is centralized in `llvm/values.rs` for `Unit`, `Bool`,
all Actus integer widths, floating-point values, pointers, aggregates,
Regions, enums, and niche pointer options. Invalid widths remain defensive
backend-contract failures rather than silently being coerced to another type.

This mapping closes the inventory/documentation part of 49.1. It does not
close implementation acceptance: the remaining checks still require complete
operation coverage, target-specific validation, and stable diagnostics for
unsupported paths.

#### 49.1 toolchain and target evidence

The repository and CI configuration agree on the following setup:

- LLVM binding: `llvm-sys 231.0.0` with strict versioning;
- supported LLVM installation line: LLVM 23.x;
- discovery: `llvm-config` or `LLVM_SYS_231_PREFIX`;
- hosted Unix linker path: the configured native linker and LLVM 23 runtime;
- hosted Windows linker path: MSVC environment with LLVM `lld-link.exe`, the
  Windows SDK, and the checked static dependency set;
- freestanding ARM32 path: LLVM 23 plus `clang`, `arm-none-eabi-ld`,
  `arm-none-eabi-gcc`, `arm-none-eabi-objdump`, explicit startup/provider
  contracts, and the Thumb target profile;
- documented target matrix: hosted Linux, hosted Apple, hosted Windows,
  freestanding UEFI/object, and freestanding Cortex-M;
- clean-checkout validation: pinned GitHub Actions checkout, explicit LLVM
  installation, target toolchain verification, compilation, tests, standard
  library checks, and dependency validation.

The authoritative operational instructions remain in
`docs/guide/compiler/backends-and-support.md`; CI setup is defined in
`.github/workflows/ci.yml` and the release workflow. This evidence closes the
toolchain-policy inventory item, but does not claim that every target/backend
combination is executable or that LLVM is ready to become the default.

### 49.2 — Complete shared-contract lowering

- [x] Prove LLVM coverage for scalar values, calls, returns, branches, loops,
      traps, typed failures, and checked casts.
- [x] Complete aggregate, array, pack, enum, tagged-union, transparent,
      packed, and explicitly aligned layout lowering.
- [x] Complete ownership transfer, borrowed views, exclusive loans, cleanup,
      early return, failure, and branch-join lowering.
- [x] Complete generic specialization, imported modules, dynamic dispatch,
      native boundaries, and recursive call handling.
- [x] Verify LLVM module signatures and layouts before object emission.

#### 49.2 scalar/control-flow evidence

The LLVM backend suite contains direct coverage for the complete scalar and
control-flow contract. The relevant evidence includes integer widths and
constants, direct and external calls, scalar and unit returns, typed branches
and jump edge arguments, fixed-array iteration, checked casts, division and
shift guards, typed failures, and native trap terminators. The full
`cargo test --features llvm --test llvm_backend -- --test-threads=1` run passed
58 tests; the all-features LLVM-filtered run passed 16 focused tests, including
the width and rejection regressions. No backend-specific semantic lowering was
added to close this checkbox.

#### 49.2 aggregate/layout evidence

The same suite covers direct and indirect aggregate ABI, aggregate field load
and store, arrays, packed bit fields, `repr(C)` payload enums, transparent
representations, packed representations, explicit alignment, return slots,
pointer memory copies, and niche/regular enum failure layouts. The complete
`cargo test --features llvm --test llvm_backend -- --test-threads=1` run passed
58 tests after the integer-width, recursion, and zero-float policy additions. The fixtures exercise the shared
contract before LLVM emission; the backend does not introduce a parallel
layout or ownership model.

#### 49.2 ownership and cleanup evidence

The shared codegen integration target covers `dat` and `abs` payload bindings,
scalar and aggregate borrowed views, `ins` parameters and exclusive loans,
move roles, explicit and scope cleanup, nested-scope cleanup, early returns,
typed failures, loop unwind cleanup, and branch-merge ownership state. The
LLVM backend suite additionally verifies end-borrow boundaries, runtime cleanup
bridges, cleanup ordering parity, and return-slot failures. The combined
`cargo test --all-features --test codegen -- --test-threads=1` run passed
106 tests in the shared codegen target, while the LLVM backend suite passed 58.

#### 49.2 call-boundary evidence

Generic specialization, qualified imported calls, dynamic role dispatch, and
native/external boundaries are covered by the shared codegen, backend-parity,
native, and LLVM backend suites. Direct recursion is additionally covered by
`llvm_emits_recursive_direct_call`. The shared call resolver preserves the
currently lowered function symbol for self-calls, including hosted entry verbs,
so LLVM and Cranelift consume the same symbol contract without backend-specific
aliases. The LLVM backend suite passed 58 tests and the all-features shared
codegen target passed 106 tests after this coverage was added.

#### 49.2 module verification evidence

LLVM object emission is fail-closed at both contract boundaries. The shared
`BackendModule::validate()` check runs before LLVM lowering and validates every
external/function signature, symbol identity, block contract, instruction
layout, aggregate layout, enum layout, and cleanup descriptor. After lowering,
`LLVMVerifyModule()` validates the generated LLVM module before any object or
object-buffer emission is attempted. The LLVM object, aggregate ABI, packed,
transparent, explicit-alignment, enum, return-slot, and external-boundary
fixtures all pass in the 58-test LLVM backend suite and the full all-targets,
all-features test run.

The LLVM object path also accepts the target configuration's zero-float policy.
When `verify_no_float_ir` is enabled, LLVM IR is audited after lowering and
module verification but before object emission. The audit rejects floating-point
type and instruction tokens and known soft-float helper symbols; the
policy-disabled path remains available for hosted programs whose contract
explicitly permits floating-point operations. The soft-float-helper regression
is covered by the LLVM backend suite.

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

