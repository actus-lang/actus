# ADR-0103: Machine-level stack-limit contract

## Status

Proposed.

## Date

2026-11-05.

## Context

Ownership checking prevents invalid ownership transitions, but it does not by
itself prove that a task stack is large enough for the generated machine code.
On targets without a hardware stack guard, an overflow may corrupt adjacent
state without producing a language-level failure. A source-level estimate is
not sufficient for this safety boundary because the backend may add register
spills, callee-saved registers, alignment padding, return-value slots,
outgoing argument storage, helper calls, or target-specific prologue state.

Actus already has `meta bounded(...)` and the `max_stack_bytes` source-level
budget described by ADR-0095. That contract remains valid for source-level
resource reasoning, but it is not a machine-frame proof. `meta stack_limit`
therefore introduces a separate, stricter obligation instead of silently
changing the meaning of `bounded` or removing the existing metadata.

## Decision

Actus will define `meta stack_limit(bytes: N)` as a target- and build-specific
machine-level proof obligation. A declaration carrying this metadata is
accepted only when the selected backend can provide a complete frame report
and the compiler can close and analyze every reachable call boundary.

The compiler must fail closed. It must reject the contract when a frame,
callee, helper, indirect-call target set, recursion path, or target rule is
unknown. It must not use a source-level estimate, assume an unknown callee has
zero stack usage, insert a runtime canary, or move a value to hidden storage
just to satisfy the limit.

The initial language contract is:

```actus
meta stack_limit(bytes: 2048u32)
verb control_step(ins state: ControlState) -> Int {
    return 0;
}
```

`bytes` is a positive `u32` literal. The attribute may occur on a verb, a
`perform` implementation, a role-method declaration, or an `unsafe extern`
declaration. It is not permitted as a file-prelude attribute in the initial
contract. Duplicate attributes, unknown arguments, zero values, non-literals,
and unsupported placements are compile-time errors. `meta limitless` cannot
disable this contract.

## Contract meaning

For a checked entry `V`, let `SP0` be the stack pointer immediately before the
caller invokes `V`. The contract requires that every execution of `V` satisfy:

```text
maximum_stack_use(V) = max(SP0 - SP) <= N
```

The measured value includes the selected target's actual frame convention,
including local storage, spills, callee-saved registers, alignment, return
slots, outgoing argument storage, compiler helpers, and target prologue state.
The report must state the convention used; the implementation must not count
return-address or call-frame overhead twice.

Sequential calls reuse stack storage and therefore compose with a maximum,
not a sum. For a machine function `f`, the required recurrence is:

```text
W(f) = F(f) + max(0, max over g in succ(f) of (R + W(g)))
contract: R + W(V) <= N
```

`F(f)` is the backend-reported fixed frame of the final symbol. `R` is the
call overhead written outside the callee frame by the call instruction and is
published by the target contract: `8` on x86-64 when the return address is
pushed by the call instruction, and `0` on Thumb-2 and AArch64 when the link
register is saved inside `F`. Each target must prove `R` and `F` with a fixture
that compares the report with disassembly. All intermediate arithmetic uses
checked `u64`; overflow is a rejection.

The contract excludes runtime startup before the entry, signal handlers,
debugger activity, and interrupt or exception frames on the same stack.

## Relationship to `bounded`

`bounded(max_stack_bytes: ...)` remains a source-level estimate under
ADR-0095. It may describe a caller-visible budget, but it does not authorize
the compiler to claim machine-level stack safety.

`stack_limit` is the only metadata in this contract that makes a machine-level
stack guarantee. The two attributes may coexist and their diagnostics must
identify which proof failed. No automatic deprecation or source migration is
introduced by this ADR. Any future migration requires a separate reviewed
compatibility decision.

## Proof boundary

### Backend frame evidence

The backend is authoritative for the fixed frame size. The proof is performed
against the exact object or machine representation selected for the build:
same target, profile, optimization settings, backend, and feature set.

For LLVM, the frame report source is `TargetOptions::EmitStackSizeSection` and
function-level section identity is provided by `TargetOptions::FunctionSections`.
Actus enables these fields through its LLVM target-machine C++ shim because the
LLVM C API does not expose them.

The initial LLVM contract is limited to targets and object formats for which
Actus can extract and validate a complete frame report. The first supported
set is ELF-based hosted Linux and ELF-based `thumbv7em-none-eabi`. Thumb is a
target mode, not a separate object format.

Mach-O, COFF, and other formats remain unsupported until their own frame-report
and call-graph contracts are implemented and tested. The compiler must emit a
stable unsupported-target diagnostic rather than silently falling back to a
source estimate.

LLVM version identity is part of the proof identity. A change to the pinned
LLVM contract may change frame sizes and must trigger revalidation.

### Known LLVM frame-report pitfalls

The following behaviors are part of the initial LLVM target contract and must
be enforced by fixtures under the pinned LLVM version:

- **Push-based outgoing arguments.** On x86-64, call-frame optimization can
  pass stack arguments with pushes that are not included in the reported fixed
  frame. `stack_limit` builds must disable this optimization, or the target is
  unsupported for the contract.
- **Red zone.** A hosted x86-64 SysV leaf may use 128 bytes below the stack
  pointer while reporting a smaller frame. Hosted x86-64 therefore adds
  exactly 128 bytes to the frame of every leaf function; non-leaf functions
  cannot use that region because a call may overwrite it. Freestanding
  targets compile without a red zone.
- **Function sections.** Calls within one section may be resolved without a
  relocation. `stack_limit` builds must emit one function section per symbol
  so that every cross-function call is visible to graph construction.
- **Missing frame record.** A symbol without a frame-report entry, including
  a function with a dynamically sized stack object, is rejected.

### Call graph closure

The machine call graph must be reconciled with the semantic graph:

- direct calls must resolve to known Actus specializations, declared runtime
  bridges, or target helper-registry entries;
- generic specializations are separate graph nodes;
- imported and precompiled units must publish stack claims proven by their own
  build, together with unit identity, lockfile checksum, target, profile, and
  LLVM identity; a claim produced under a different proof identity is
  rejected;
- indirect calls require a compiler-proven closed target set;
- dynamic dispatch requires closed-world implementation metadata;
- unknown indirect calls, callbacks from native code, and unresolved symbols
  are rejected in a `stack_limit` proof closure;
- compiler-generated helpers must be registered with target-specific frame
  information and successor edges;
- linker-added veneers, PLT stubs, and equivalent target code must either be
  proven stack-neutral by the target contract or make the proof unsupported;
- tail calls are treated conservatively as ordinary calls unless a target
  contract proves an equivalent or smaller bound.

Relocations may contribute to the graph, but they are not by themselves a
complete proof for every object format or linker transformation. The target
contract must define what is visible before linking and how linker-generated
code is accounted for.

### Recursion

Any recursion reachable from a `stack_limit` entry is rejected in the initial
contract. This includes direct and mutual recursion, imported recursion,
generic specialization cycles, role/`perform` cycles, dynamic-dispatch cycles,
and machine-helper cycles.

A future bounded-recursion extension requires a separate ADR defining a
proven depth bound and a sound strongly-connected-component calculation. This
ADR does not accept such an extension.

### Native boundaries

An `unsafe extern` reachable from a `stack_limit` entry must declare its stack
claim. The claim covers the native callee's complete transitive stack use,
including every native function it calls. The claim is trusted, not
automatically proven. It is reported as `declared` unless a compatible native
object and frame report allow the compiler to verify it.

Native callbacks into Actus are rejected in the initial closure because the
native claim cannot soundly include an unknown Actus call path. Inline
assembly must declare a balanced stack effect; code that changes the stack
pointer without a verified effect is rejected.

Under `--strict`, every native claim must be `verified` or recorded as a
reviewed exception under the conformance exception model described by
ADR-0048.

### Aggregate storage

The backend frame is authoritative; these rules define which frame owns the
storage so that reports and audits are predictable:

1. A local `erg` aggregate occupies its layout size rounded to alignment in
   the declaring verb's frame.
2. An aggregate return uses the caller-owned `sret` slot (ADR-0031) and is
   charged to the caller.
3. `abs` and `ins` aggregate parameters are addresses and copy no storage.
4. A `dat` aggregate passed by value may be copied into caller storage and is
   charged to the caller; the report should suggest `ins` when such a copy is
   large on a `stack_limit` path.
5. Enum payload storage follows its lowering. Stack storage is counted by the
   frame, runtime storage is an allocation effect, and the report states which
   one applies.
6. Hosted aggregates above `LARGE_AGGREGATE_THRESHOLD` (ADR-0081) use runtime
   storage and contribute only the bridge call. On freestanding targets,
   `stack_limit` remains the guard for aggregates below that threshold.
7. Static, region, and arena storage is not stack storage; the report shows
   where each large value lives.
8. The compiler never relocates an aggregate to satisfy a limit. The fix must
   be an explicit source change.

## Target contract

Every target supporting `stack_limit` must publish:

- stack growth direction and frame-report convention;
- call-frame and return-address accounting rules;
- required stack alignment;
- red-zone policy, if applicable;
- helper registry and helper frame sizes;
- linker veneer and stub policy;
- exception/interrupt frame policy where relevant;
- supported object formats and backend versions;
- whether native claims can be upgraded from `declared` to `verified`.

For the initial Cortex-M target, task-stack proof and interrupt-stack proof
are separate domains. Interrupt exception frames are not silently included in
the task's `stack_limit`; a future interrupt/domain ADR must define that
composition explicitly.

## Diagnostics and reporting

The implementation must reserve verified diagnostic identifiers before coding.
At minimum it needs distinct diagnostics for:

- exceeded limit with required bytes and worst path;
- reachable recursion;
- missing or dynamic frame report;
- unresolved machine callee or helper;
- missing native/cross-unit claim;
- open indirect-call target set;
- unsupported backend, target, or object format;
- missing inline-assembly stack effect;
- invalid metadata syntax or placement;
- role implementation violating a declared contract.

`actus build --stack-report` will eventually emit deterministic JSON containing
the verb, target, profile, backend, compiler identity, limit, required bytes,
margin, worst path, frame source, and proof status (`proven` for Actus frames,
`verified` for native claims checked against a native frame report, or
`declared` for trusted native claims). Equal paths must use a stable
symbol-order tie-breaker. Source-level layout
details may explain a frame, but they are explanatory evidence and never
replace the backend frame proof.

## Implementation scope

Implementation is intentionally divided into independently reviewable stages:

1. **S0 — contract acceptance:** reserve this ADR number, confirm the
   `bounded` relationship, and define the supported initial target matrix.
2. **S1 — language surface:** add parser, metadata registry, formatter, LSP
   preservation, placement validation, and negative fixtures.
3. **S2 — semantic preflight:** reject recursion, unresolved dispatch, missing
   native claims, unsupported backend/target pairs, and invalid assembly
   effects before code generation.
4. **S3+ — machine proof:** add backend frame extraction, object-format
   contracts, call-graph reconciliation, max-plus analysis, reports, and
   native verification only after S0–S2 are accepted. Structural failures are
   rejected before object emission; machine-proof failures are rejected after
   object analysis and before any object or executable is published as a
   build artifact.

Cranelift remains unsupported for this metadata until it provides an equivalent
verified frame report. Its fallback is a stable diagnostic, never a source
level approximation.

## Alternatives rejected

- Treating a source-level local-size sum as machine stack safety.
- Assuming unknown native or indirect callees use zero stack.
- Adding runtime canaries or hidden stack checks as a substitute for a compile
  time proof.
- Automatically moving large values to heap or runtime storage.
- Treating a relocation-only graph as complete on every target.
- Silently falling back to Cranelift or another backend without a verified
  frame contract.

## Consequences

Positive consequences:

- stack safety is tied to generated machine evidence;
- ownership, allocation, and storage placement remain explicit;
- structural failures fail before object emission, while machine-proof
  failures prevent publication after object analysis;
- reports provide an auditable worst path rather than an unexplained number.

Costs and limitations:

- proofs are target-, profile-, backend-, object-format-, and compiler-version
  specific;
- builds need an additional frame/call-graph analysis step;
- recursion and open native boundaries are rejected initially;
- Cranelift cannot support the attribute until it has equivalent evidence;
- hardware stack-painting remains validation evidence, not a replacement for
  the compile-time proof.

## Required evidence before acceptance

- ADR number and references are conflict-free;
- S0–S2 parser, semantic, diagnostic, formatter, LSP, and `.act` fixtures
  pass under strict validation;
- LLVM target/object-format assumptions are verified with the pinned LLVM
  version, not only an older local installation;
- the target contract has explicit call-frame, red-zone, helper, and veneer
  rules;
- no existing ownership, ABI, serialization, or zero-float contract changes
  as an incidental effect.
