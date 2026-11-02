# Phase 49 — LLVM primary native backend

## Status

In progress. The complete backend baseline, shared-contract lowering, target
contracts, runtime contracts, parity evidence, and default selection change
are complete through the current migration checks. LLVM is now the default
native backend while Cranelift remains explicitly available during the
compatibility period. No Actus source, ownership, runtime, or serialized-data
contract is changed by this plan.

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
- default feature: `llvm`;
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

- [x] Verify all runtime bridge signatures and failure statuses through LLVM.
- [x] Verify external C declarations, checked representations, aggregate
      passing, aggregate returns, and calling-convention contracts.
- [x] Complete hosted target object/link/execute coverage.
- [x] Complete freestanding object and link-contract coverage where startup,
      provider, linker, and target evidence exist.
- [x] Keep filesystem, clock, allocation, and other hosted services explicit;
      never inherit them into freestanding targets.

#### 49.3 current LLVM ABI evidence

The current audit compares every `unsafe extern "C" verb` declared by the
standard library with the native runtime's exported C providers, including
parameter arity. The only permitted arity difference is the one caller-owned
return slot required by the established aggregate `sret` ABI. It also lowers
all hosted standard-library facades through strict LLVM object emission with
no-float verification enabled. The executable runtime-status matrix covers
the deterministic success, boundary, and failure contracts for process
arguments, UTF-8 string access, buffer capacity and clearing, cursor bounds,
and restricted blocking contexts. The existing filesystem, path, directory,
region, stream, clock, and locking fixtures remain part of the same LLVM
bridge suite, so status values are checked at the native boundary rather than
only in Rust unit tests. A hosted C-provider fixture links and executes both
a `repr(C)` aggregate parameter and an aggregate return through the
LLVM-produced object, proving the pointer and `sret` ABIs at a real C boundary
rather than only in generated IR text. A hosted executable fixture also
constructs, mutates, and observes packed, transparent, explicitly aligned,
and payload-enum values through LLVM, so these checked representations are
exercised at runtime.
The CI matrix now invokes the LLVM bridge suite explicitly on Ubuntu, macOS,
and Windows; target-specific native-provider fixtures remain guarded by their
actual host contract instead of being treated as portable evidence.

The LLVM runtime-bridge suite now verifies `repr(C)` aggregate boundaries in
addition to scalar, pointer, buffer, filesystem, clock, process, locking,
directory, and region bridges. An aggregate parameter is emitted as an
aligned pointer according to the existing caller/callee ABI contract. An
aggregate returned by an unsafe external declaration uses the existing
caller-provided return slot: the external declaration returns `void`, writes
into the aligned slot, and the caller copies the declared aggregate layout
back into its destination. These tests do not introduce a second FFI layout
or silently reinterpret a C declaration.

Hosted target coverage is also complete for the current CI matrix. The LLVM
host-object suite emits and inspects a host object, while the portable CLI
fixture builds and executes `examples/hello.act` with the explicit LLVM
backend. The fixture uses the target runner's executable suffix and is enabled
on hosted Linux, macOS, and Windows jobs; no Unix-only linker assumption is
made by this coverage.

Freestanding coverage is complete for the currently supported evidence
boundary. LLVM emits a freestanding host-independent object without implicit
runtime imports, emits the Cortex-M Thumb object for the declared target
profile, and links the ARM image only with explicit startup, provider, and
linker inputs. The external-provider fixture also verifies that the provider
symbol is resolved and that the final image contains no unresolved symbols or
relocations. Unsupported startup or target combinations remain explicit
configuration errors rather than implicit hosted fallback.

Hosted-service isolation is covered by target-aware module resolution and
native rejection fixtures: hosted `std::fs`, `std::time`, and process modules
are rejected before native emission for freestanding targets, while allocation
and Region operations use an explicit target runtime contract. Effect metadata
is preserved during target filtering and is never used as an implicit service
permission.

### 49.4 — Backend parity and regression hardening

- [x] Run the shared semantic and negative fixture matrix through both
      backends wherever both target contracts support the fixture.
- [x] Compare deterministic outputs, object symbols, ABI layouts, exit status,
      cleanup behavior, and typed failures.
- [x] Add regressions for every LLVM failure found during the inventory.
- [x] Verify that unsupported operations fail before invalid object emission.
- [x] Run source limits, formatter, documentation, dependency, and security
      checks in every supported CI configuration.

#### 49.4 current parity evidence

The backend-parity suite currently passes 20/20 tests on Unix hosts with
both backends enabled. It compares hosted exit status and buffer ownership,
directory cleanup and deterministic order, qualified imports, generic calls,
aggregate and enum construction, named array constants, typed failures, and
hosted float behavior. It also verifies deterministic object bytes, compares
source-declared external aggregate provider symbols and relocation sizes, and
checks the external provider contract for `repr(C, packed)`,
`repr(transparent)`, and `repr(C, align(...))` aggregates.
The same checked representations are also exercised through external return
boundaries, so parameter and return relocation contracts must agree across
both backends.
It also verifies that the same semantic or target rejection occurs before
either backend emits an object. Freestanding fixtures compare exported symbols,
zero-float evidence, large-aggregate rejection, hosted-service rejection for
`std::fs`, `std::time`, and `std::process`, and shared target diagnostics. This
is stronger regression evidence. The full
`cargo test --all-targets --all-features` suite also passes, including the
shared lowering, FFI, LLVM object, target filtering, source-limit, and
standard-library suites. The unsupported-operation contract is therefore
closed for the covered semantic and target fixtures: both backends reject
before object emission and leave no invalid artifact behind.

The remaining 49.4 work is limited to LLVM-specific regression inventory and
the supported CI configuration checks; the cross-backend output, symbol,
cleanup, typed-failure, and checked ABI boundary comparison is covered.

The current semantic-contract matrix additionally covers `must_use`, effect,
capability, deterministic, realtime, and bounded-contract failures, plus
parser, unknown-type, and unknown-verb failures. Each failure is compared
across both backends, checks the stable diagnostic code, and asserts that no
object is emitted. This closes the shared semantic and negative fixture matrix
for the currently supported hosted and freestanding target contracts.

The LLVM inventory regression suite passes 98/98 tests together with the
shared 106-test codegen suite. It includes stable regressions for non-native
integer widths, recursive calls, enum equality, zero-float rejection,
runtime status boundaries, checked representations, aggregate return slots,
cleanup ordering, and target-specific object/link contracts.

The final quality-check contract is also complete. The source-limit,
formatter, documentation, standard-library, architecture, secrets, and
dependency/security checks pass locally; `cargo check --all-targets
--all-features` and strict clippy pass as well. The explicit CI backend
configurations pass with Cranelift enabled and LLVM enabled, including the
LLVM backend suite. The CI workflow applies the corresponding Rust, backend,
standard-library, source-limit, and security checks across its hosted Linux,
macOS, and Windows matrix, with target-specific LLVM jobs kept explicit.
No unsupported platform result is inferred from a different host, and no
later default-backend migration work is included in this acceptance check.

The fail-closed target matrix separately verifies floating-point policy,
hosted-service imports, and unsupported large aggregates on freestanding
targets. Each case is rejected by both backends before an object is emitted;
the hosted semantic-contract matrix applies the same no-artifact rule to
frontend contract failures.

### 49.5 — Default selection migration

- [x] Change the default Cargo feature and backend selection only after all
      prior acceptance work passes.
- [x] Update backend support documentation, configuration examples, help text,
      artifact metadata, and clean-checkout instructions.
- [ ] Verify explicit Cranelift selection and rollback from a clean checkout.
- [ ] Measure and record compile time, artifact size, cache behavior, and
      relevant native execution evidence.
- [ ] Run the complete default-LLVM suite before declaring the migration ready.

#### 49.5 default selection evidence

The default Cargo feature now selects LLVM, and the backend configuration
selects LLVM whenever the LLVM feature is available. The explicit
Cranelift-only feature configuration retains Cranelift as its default, so the
compatibility backend remains available without changing Actus source code.
Configuration regression tests cover both feature configurations. CLI help,
support documentation, and clean-checkout rollback instructions are covered
by this documentation check; compatibility measurements and release-artifact
work remain intentionally open under the subsequent 49.5 checks.

The backend support guide, native-build guide, workflow reference, manifest
reference, README, and CLI help now describe LLVM as the default and Cranelift
as an explicit compatibility choice. They document feature requirements,
manifest and command-line precedence, target validation, unavailable-backend
errors, artifact metadata invalidation, clean-checkout verification, and
source-preserving rollback. The existing artifact metadata contract already
records the selected backend together with target, profile, runtime,
source-limit, compiler, and runtime identity fields; the documentation now
describes that contract rather than introducing a second cache mechanism.

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

