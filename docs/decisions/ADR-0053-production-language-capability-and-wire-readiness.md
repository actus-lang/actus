# ADR-0053: Production Language Capability and Wire Readiness

- Status: Proposed / Architectural Specification
- Date: 2026-10-01
- Scope: Actus language core, compiler pipeline, standard-library contracts,
  and readiness evidence for target-neutral binary codecs

## Context

Actus already provides typed integers, explicit casts, relational and bitwise
operators, bounded arrays, buffers, packs, ownership roles, typed results,
deterministic cleanup, compile-time constants, facade-aware modules, native
lowering, and compiler-backed tooling.

Those features are individually useful, but a production codec exercises them
as one system. A codec must perform bounded byte access, fixed-width arithmetic,
bit manipulation, checks, branching, error propagation, and zero-copy mutation
without handwritten duplication, hidden conversions, C bridges, or target-
specific compiler exceptions.

The previous prerequisite work proved several of these contracts separately.
It did not yet establish a durable acceptance boundary saying that the whole
language is capable of expressing and executing a production-grade binary
algorithm. Starting `std::wire` before that evidence exists risks discovering
language gaps in the middle of a library implementation and hiding them behind
special cases.

The compiler audit identifies a second production boundary: user-controlled
source, package metadata, runtime input, and LSP/JSON-RPC messages must never
terminate the compiler through an avoidable panic. The documentation audit
identifies the corresponding product boundary: public claims, roadmaps,
standard-library contracts, examples, and architecture documents must describe
implemented behavior rather than aspirational behavior.

## Decision

Actus introduces Phase 21, **Production Language Capability and Wire
Readiness**, as a prerequisite phase for `std::wire` implementation.

The phase validates the language through orthogonal, compiler-owned contracts:

1. structured control flow and deterministic cleanup;
2. fixed-width integer arithmetic and checked conversion;
3. bounded contiguous memory and zero-copy ownership;
4. typed failure propagation and public module boundaries;
5. compile-time protocol constants and reusable abstractions;
6. synchronized parser, semantic, codegen, formatter, LSP, and native
   acceptance evidence.
7. panic-free handling of user-controlled compiler, module, runtime, and LSP
   input;
8. documentation and release metadata that remain truthful and synchronized
   with the implementation.

No gate may be marked complete from documentation alone. Every check must have
an executable positive or negative test where the contract crosses the relevant
compiler phase or runtime boundary.

## Capability contracts

### 1. Control flow and cleanup

The compiler must lower nested `if`, `case`, `loop`, `break`, `continue`, and
`return` paths deterministically. Branch-local ownership changes must join
without leaks, double cleanup, or use-after-move. Loop-carried values must be
passed through every control-flow edge, including edges originating inside case
branches.

### 2. Fixed-width numeric behavior

Integer operations must preserve declared signedness and width. Unsuffixed
integer literals may be directed by a known integer operand only when the value
fits that type. Overflow and underflow must be rejected statically when
provable and trapped deterministically when only runtime information is
available. Explicit casts remain the only general numeric conversion syntax.

### 3. Bounded memory and ownership

`Array[T, N]`, `Buffer`, and `pack` must support dynamic indexing, bounds
checking, field access, and in-place mutation through `ins` without hidden
allocation or whole-container copying. `abs` views must remain non-owning and
`dat` transfers must remain explicit.

### 4. Typed failure boundaries

Public operations must expose `Result[T, E]` or another documented typed
failure contract. Raw ABI statuses must be translated before reaching public
Actus code. Failure paths must preserve ownership and cleanup guarantees.

### 5. Compile-time protocol data

Named constants and fixed-width types must be sufficient for protocol values,
limits, masks, and layout declarations. The compiler must reject cycles,
runtime dependencies, invalid widths, and out-of-range constants without
emitting runtime storage for compile-time values.

### 6. Tooling parity

Parser, semantic analysis, codegen, formatter, LSP, diagnostics, native
execution, and source-conformance checks must agree on the same language
contract. A feature is not production-ready if the compiler accepts it while
the formatter, LSP, or diagnostics corrupt or misrepresent it.

### 7. Production failure containment

Production code must not use `unwrap`, `expect`, `unreachable`, or equivalent
panic paths for states reachable from source text, package files, module
resolution, runtime input, or LSP/JSON-RPC messages. Internal invariants must
be represented by typed errors or validated private construction boundaries.

The compiler and language server must return deterministic diagnostics for
malformed or incomplete source, recovered ASTs, invalid modules and manifests,
invalid numeric widths and bounds, ownership failures, malformed JSON-RPC,
missing documents, stale versions, invalid ranges, unavailable targets, and
invalid runtime handles. Each production panic fix must include a regression
test that verifies both the diagnostic and process liveness.

Panic scanning is a discovery tool, not an acceptance criterion by itself.
Every occurrence must be classified as test-only, private-invariant-only, or
replaced with an explicit error boundary.

### 8. Documentation and release truthfulness

Documentation is part of the public product contract. README, MANIFESTO,
language guides, roadmaps, ADR status fields, release notes, compiler
architecture, standard-library references, examples, package metadata, and
target claims must not describe planned behavior as implemented.

Every public standard-library declaration must have an implementation-matched
API reference covering ownership roles, `Result` errors, side effects,
allocation behavior, and ABI boundaries. Every example must be executable by
a documented command or explicitly marked as design material. Documentation
checks must detect stale phase references, duplicate ADR identifiers, broken
links, unsupported feature claims, and undocumented public APIs.

## Explicit scope boundary

Phase 21 is a Wire-readiness and production-correctness phase. It is not a
claim that every enterprise compiler capability is already implemented or that
every future language feature is required before Wire can begin.

The following tracks are therefore recorded as separate enterprise-platform
work, with their own acceptance criteria in
[Phase 22](../roadmap/phase-22-enterprise-compiler-platform.md):

- parser multi-error recovery and diagnostic aggregation;
- proactive decomposition of files above the repository warning threshold;
- source-level Rust API documentation coverage;
- incremental compilation and cache invalidation;
- parallel compilation and build scheduling;
- DWARF/debug-info generation and debugger integration;
- additional targets such as WASM, with target-specific runtime contracts;
- an interactive REPL;
- borrow-checking complexity and benchmark-driven optimization;
- expanded compiler, backend, LSP, and runtime benchmarks;
- closures, lambdas, async execution, and macros, each only after a separate
  language and ownership decision.

These tracks must not be silently treated as complete because the Wire
readiness gates pass. Conversely, they must not block Wire unless a concrete
Wire contract depends on them. Their status remains visible and independently
evidenced.

## Evidence policy

Each phase gate requires all applicable evidence below:

- accepted syntax and semantic tests;
- rejected syntax, type, ownership, bounds, and failure tests;
- native execution tests for code-generation behavior;
- deterministic repeated-build or repeated-diagnostic tests;
- freestanding/object tests where hosted services could leak;
- formatter and LSP tests for the public source contract;
- repository quality gates and source-size checks;
- documentation that names the exact test files or commands providing proof.

A passing compile is not evidence of runtime behavior. A passing unit test is
not evidence of native execution. A passing LSP response is not evidence that
the compiler and editor understand different semantics. The evidence must be
matched to the layer being claimed.

## Non-goals

- This ADR does not add implicit numeric conversions.
- This ADR does not remove ownership roles or bounds checks.
- This ADR does not introduce a C bridge to make a codec appear complete.
- This ADR does not define the Wire or Ustari protocol format.
- This ADR does not require every possible language feature before useful
  application development.
- This ADR does not mark any Phase 21 gate complete in advance.
- This ADR does not classify every test-only `expect` as a production bug;
  production reachability and invariant ownership must be reviewed first.
- This ADR does not permit documentation-only closure of an implementation
  gate.

## Consequences

Positive consequences:

- language gaps are found through small, reproducible capability tests;
- Wire implementation can consume stable primitives instead of compiler
  exceptions;
- production claims are tied to executable evidence;
- frontend, backend, runtime, and tooling drift becomes visible early;
- embedded and freestanding targets share the same language contracts.
- malformed user input cannot silently terminate the compiler or LSP;
- release and documentation claims become reviewable, testable artifacts.

Costs and risks:

- the phase adds acceptance work before the Wire library itself;
- some capabilities may require compiler changes across multiple pipeline
  layers;
- native and freestanding evidence can take longer than parser-only tests;
- a gate may remain open when a platform-specific environment is unavailable;
- over-broad requirements must be rejected rather than solved with exceptions.
- panic removal may require API changes through several compiler layers;
- documentation synchronization becomes ongoing release work.

## Exit criteria

Phase 21 may close only when every roadmap gate is either completed with direct
evidence or explicitly deferred with an owner, reason, and unblock condition.
The final readiness gate must demonstrate a small, production-shaped codec
fixture using only public Actus capabilities and must pass the complete
repository quality suite. Only then may the `std::wire` Gate 1 implementation
begin. No audit report, checklist, or passing compilation may substitute for
the required executable evidence.
