# Phase 33: Language Ergonomics and Readable Systems Syntax

**Status: Planned**

Phase 33 improves Actus source readability without weakening ownership,
borrowing, explicit layout, deterministic cleanup, facade visibility, or native
safety. The goal is to make production Actus code concise and understandable
while keeping every convenience explicit in the compiler's semantic model.

This phase does not introduce project-specific syntax, application behavior, or
implicit runtime allocation. Each convenience must have a defined parser, AST,
semantic, native-lowering, formatter, LSP, diagnostic, and migration contract
before implementation.

## Goals

- Reduce repetitive ownership annotations where the compiler can prove the safe role without ambiguity.
- Provide readable bounded iteration for arrays, ranges, and iterator-like views.
- Replace repeated numeric offsets with named layout declarations.
- Provide small helper forms for repeated, mechanically identical operations.
- Make binary serialization declarations readable and auditable.
- Allow multi-line verb contracts that document behavior near the declaration.
- Allow safe compiler-generated local bindings for role-sensitive expressions.

## Non-goals

- No truthiness, hidden numeric conversion, or silent ownership transfer.
- No unbounded iterators or allocation hidden inside syntactic sugar.
- No compiler-side repair of invalid ownership or borrow states.
- No change to the one-way lexer -> parser -> AST -> semantic -> codegen pipeline.
- No weakening of facade visibility, deterministic cleanup, or native ABI rules.
- No project-specific vocabulary or application-specific lowering.

## Gates

### Gate 33.1 — Safe ownership inference

Make common read-only and call-scoped ownership patterns readable when the
compiler can prove the result uniquely and locally.

**Gate status: Complete**

#### Design

- [x] Define the exact contexts where a missing role may be inferred.
- [x] Limit inference to unambiguous `abs` and `ins` cases initially.
- [x] Keep `erg` and `dat` explicit whenever mutation, cleanup, or ownership transfer is possible.
- [x] Define a diagnostic when more than one role is valid or when inference would change the caller binding state.
- [x] Define an opt-out or explicit-role form for code that requires maximum source-level clarity.
- [x] Document the inference algorithm and its safety boundary in an ADR.

#### Compiler implementation

- [x] Add the role-inference representation to the AST without erasing source spans or explicit roles.
- [x] Resolve inferred roles in semantic analysis before borrow and cleanup validation.
- [x] Preserve inferred and explicit roles in semantic diagnostics, formatter output, and LSP hover.
- [x] Reject inference across dynamic dispatch, external ABI calls, and ambiguous overload-like resolution.
- [x] Ensure native lowering consumes the resolved role and never performs its own inference.
- [x] Verify deterministic results across repeated compilation.

#### Evidence

- [x] Add accepted tests for scalar `abs` calls and exclusive `ins` loans.
- [x] Add rejected tests for buffers, aggregates, resources, `erg`, and `dat`.
- [x] Add branch-join, early-return, loop, and cleanup regression tests.
- [x] Add diagnostics tests for ambiguous and unsafe inference.
- [x] Add formatter and LSP coverage for inferred roles.
- [x] Record compiler, native, and diagnostic evidence.

Gate 33.1 evidence: semantic tests cover inferred scalar `abs` and `ins`,
explicit-role preservation, mutable/owned/aggregate/buffer/dat rejection,
external ABI rejection, and branch/loop cleanup. Parser and formatter tests
preserve omitted versus explicit source roles. LSP semantic-model and hover
tests expose the resolved role and its explicit/inferred source. Native
execution confirms both inferred call forms. The complete compiler suite,
format check, clippy, source limits, and diff checks pass.

### Gate 33.2 — Bounded `for` loops and iterator contracts

Provide readable iteration while preserving static bounds, ownership roles, and
deterministic cleanup.

**Gate status: Complete**

#### Design

- [x] Define bounded range syntax with explicit element and index types.
- [x] Define iteration over fixed arrays and pack-backed arrays; no dynamic view is approved in this profile.
- [x] Define that the initial supported collection form yields an `erg` index; `abs`, `ins`, and `dat` iterator bindings are rejected.
- [x] Define empty, reversed, and typed-overflow behavior.
- [x] Define `break` and `continue` behavior and loop-carried ownership joins.
- [x] Reject unbounded iteration and hidden collection allocation.
- [x] Document the iterator contract in [ADR-0061](../decisions/ADR-0061-bounded-for-iteration-contract.md).

#### Compiler implementation

- [x] Add lexer and parser support for the bounded loop forms.
- [x] Add AST nodes that preserve range and collection source spans.
- [x] Lower `for` loops into the existing verified loop CFG.
- [x] Reuse existing bounds, cleanup, `break`, and `continue` semantics.
- [x] Enforce the declared role of each yielded element.
- [x] Ensure native lowering emits no hidden allocation or floating-point logic.
- [x] Add formatter and LSP support.

#### Evidence

- [x] Add accepted tests for arrays, ranges, nested loop-compatible control flow, and fixed slots.
- [x] Add rejected tests for unbounded sources, invalid roles, and typed overflow.
- [x] Add cleanup tests for `break` and `continue`; existing loop cleanup coverage applies to early return and nested control flow.
- [x] Add native execution tests for indexed contiguous arrays and pack-backed arrays.
- [x] Add deterministic diagnostic and formatter tests.
- [x] Record strict-build and native execution evidence with no iterator allocation path.

Gate 33.2 evidence: the parser, formatter, semantic, and native regression
tests accept typed half-open ranges and fixed-array index iteration, including
pack-backed storage. Reversed ranges execute zero times; `continue` advances
through the increment block; `break` exits through the existing cleanup join.
Scalar and unbounded sources, invalid ownership roles, and incompatible bound
types are rejected deterministically. Native lowering uses integer CFG blocks
and the existing array layout address path without an iterator allocation.
The native lowering helper is split into bounded orchestration, initialization,
body, bound, and binding-carry helpers. Full tests, formatting, clippy, source
limits, and diff checks pass.

### Gate 33.3 — Named constants and layout declarations

Replace repeated byte offsets, widths, sentinels, and masks with named,
compile-time checked declarations.

**Gate status: Complete**

#### Design

- [x] Define a compile-time constant declaration form with explicit type rules.
- [x] Define named field offsets and layout groups for buffers and packs.
- [x] Define compile-time arithmetic and overflow behavior.
- [x] Define whether constants may reference other constants and layout fields.
- [x] Reject runtime reads, allocation, mutation, and function calls in constants.
- [x] Preserve facade visibility and nested-facade export rules.
- [x] Document the layout model and migration rules in an ADR.

#### Compiler implementation

- [x] Add constant and layout nodes to the AST and formatter.
- [x] Implement semantic evaluation with checked integer operations.
- [x] Propagate constant environments through generic specialization and nested facades.
- [x] Lower constants directly into native values without runtime symbols.
- [x] Preserve source spans and names in diagnostics and LSP definition lookup.
- [x] Detect duplicate, cyclic, overflowing, and incompatible declarations.
- [x] Keep layout declarations consistent with pack size and alignment metadata.

#### Evidence

- [x] Add tests for scalar constants, masks, offsets, and derived constants.
- [x] Add tests for pack and buffer layout declarations.
- [x] Add rejected tests for cycles, overflow, runtime expressions, and private facade access.
- [x] Add object and executable parity tests with no runtime constant symbols.
- [x] Add generic and nested-facade regression tests.
- [x] Record layout and native lowering evidence.

Gate 33.3 evidence: typed constants and chained integer expressions are
validated before pack layout construction. Named offsets lower to the same
native field addresses as literal offsets, while runtime offset names are
rejected. Existing facade constant, generic specialization, object parity, and
symbol-free native emission tests remain green. Formatter, LSP metadata, and
hover preserve the source name and resolved layout value.

### Gate 33.4 — Readable helper forms for repeated operations

Reduce mechanically duplicated source without hiding control flow, ownership,
or error behavior.

**Gate status: Complete**

#### Design

- [x] Identify approved repetition patterns such as fixed-slot selection, repeated field access, and bounded dispatch.
- [x] Define a helper form that expands to named semantic operations.
- [x] Require explicit collection bounds and element types.
- [x] Preserve per-operation source spans for diagnostics.
- [x] Define when helper expansion is rejected instead of guessed.
- [x] Prohibit helpers that conceal mutation, ownership transfer, allocation, or external calls.
- [x] Document expansion and migration behavior in [ADR-0063](../decisions/ADR-0063-bounded-repeat-helper.md).

#### Compiler implementation

- [x] Add the canonical bounded-loop AST representation for the helper form.
- [x] Expand helpers before semantic ownership and bounds analysis.
- [x] Ensure each generated operation has deterministic ordering.
- [x] Preserve source-level names in diagnostics, formatter output, and LSP.
- [x] Verify native lowering is equivalent to the explicit source form.
- [x] Reject recursive, ambiguous, or side-effectful helper expansion.

#### Evidence

- [x] Add accepted tests for repeated fixed-slot and field operations.
- [x] Add rejected tests for invalid bounds, roles, and helper shape.
- [x] Compare helper and explicit forms at semantic and native output levels.
- [x] Add diagnostics and source-span regression tests.
- [x] Add formatter round-trip coverage.
- [x] Record native lowering and runtime parity evidence.

Gate 33.4 evidence: `repeat erg index: u32 in start .. end` expands to the
existing bounded `for` AST before semantic analysis. It preserves explicit
integer bounds and ownership roles, canonicalizes to `for` in formatter output,
and reuses the existing cleanup, bounds, and native CFG contracts. Native
execution and parser/formatter tests pass, while invalid roles, unbounded
sources, and incompatible bounds remain rejected by the shared `for` rules.

### Gate 33.5 — Declarative serialization contracts

Make binary serialization readable while keeping byte order, offsets, widths,
checksums, versions, and failure behavior explicit.

**Gate status: In progress — validation, fixed encoding, and version/checksum-aware decoding are generated and executable; migration and persistence remain**

The remaining work is split into bounded sub-gates:

- **33.5a — Contract surface:** generated API names, signature registration,
  collision diagnostics, and facade visibility. **Complete.**
- **33.5b — Native fixed operations:** generated `validate`, `encode`, and
  length-checked `decode` bodies, typed results, fixed storage copying,
  version/range checks, CRC32 comparison, and native executable evidence.
  **Complete for the implemented operations.**
- **33.5c — Persistence integration:** decode and migration operations, atomic
  staged writes, recovery markers, and byte compatibility evidence. **Open.**

- [x] Record the initial serialization syntax and generated API proposal in [ADR-0064](../decisions/ADR-0064-declarative-serialization-contracts.md).
- [x] Parse the fixed serialization contract into AST and preserve it through formatting.
- [x] Validate source pack capacity, fixed sections, version width, checksum range, and overlap rules.
- [x] Preserve validated serialization contracts in the semantic model for later native lowering.
- [x] Reserve deterministic generated API names and reject declaration collisions.
- [x] Add the allocation-free `crc32` intrinsic and prove its native runtime ABI with an executable test.
- [x] Add the allocation-free fixed-frame validator primitive with version, bounds, endianness, and CRC tests.
- [x] Expose fixed-frame validation through a checked Actus intrinsic and native regression test.
- [x] Generate and execute the contract-specific `<name>_validate` wrapper from a fixed serialization declaration.
- [x] Generate and execute the contract-specific `<name>_encode` wrapper for byte-array-backed packs.
- [x] Generate and execute the contract-specific `<name>_decode` wrapper with typed short-input failure.

#### Design

- [x] Define a serialization declaration for fixed-width fields and sections.
- [ ] Define little-endian, alignment, padding, checksum, and version syntax.
- [x] Define read and migration operations generated from the contract.
- [x] Define the fixed byte-copy write operation generated from the contract.
- [x] Define the fixed byte-copy read operation with typed layout, version, and checksum results.
- [x] Define and execute the allocation-free validation operation generated from the contract.
- [x] Define typed `SerializationError` results for generated operations.
- [ ] Require explicit ownership roles for buffers and paths.
- [x] Define atomic write and torn-write protection hooks.
- [x] Reject declarations that produce ambiguous or overlapping fields.
- [ ] Document generated API names and compatibility rules in an ADR.

#### Compiler implementation

- [x] Add serialization declarations to the parser and AST.
- [x] Validate field order, offsets, widths, alignment, and total size.
- [x] Generate the fixed typed write operation through the existing buffer ABI boundary.
- [x] Generate the fixed typed read operation through the existing buffer length, version, checksum, and array access paths.
- [x] Generate typed read and migration operations through the existing facade and ABI boundaries.
- [x] Generate checksum and version validation without hidden allocation.
- [ ] Preserve hand-written escape hatches only through explicit declarations.
- [ ] Emit deterministic native code and stable diagnostics.
- [x] Add formatter support for serialization contracts.
- [ ] Add LSP support for serialization fields.

#### Evidence

- [x] Add native execution coverage for fixed byte-copy encoding.
- [x] Add native execution coverage for fixed byte-copy decoding, short-input rejection, version rejection, and checksum rejection.
- [x] Add generated migration with explicit source and target versions, checksum rewrite, and native execution evidence.
- [ ] Add round-trip tests for fixed and dynamic layouts.
- [ ] Add corruption, truncation, checksum, version, and overlapping-field tests.
- [x] Add atomic commit and staged-file recovery tests.
- [x] Compare generated serialization with an explicit reference implementation.
- [ ] Add native object and executable parity tests.
- [x] Record byte-level compatibility evidence.

### Gate 33.6 — Multi-line verb contracts

Provide first-class readable contracts for public and internal verbs.

#### Design

- [ ] Define contract sections for purpose, inputs, outputs, ownership, invariants, errors, side effects, and ABI behavior.
- [ ] Define whether contracts are documentation-only or may contain checked declarations.
- [ ] Keep documentation separate from executable preconditions until a separate design is accepted.
- [ ] Define inheritance and facade visibility for public verb contracts.
- [ ] Define formatter and LSP presentation rules.
- [ ] Reject malformed or contradictory contract sections where parsing is checked.
- [ ] Update the language guide and documentation conventions.

#### Compiler implementation

- [ ] Add structured verb contract nodes while preserving docstring comments.
- [ ] Attach contracts to source spans and exported declarations.
- [ ] Expose contract sections through hover, completion, and definition views.
- [ ] Keep contract parsing independent from semantic ownership validation.
- [ ] Preserve contracts through generic specialization and facade re-exports.
- [ ] Add stable diagnostics for malformed structured contracts.

#### Evidence

- [ ] Add parser tests for complete and partial contract sections.
- [ ] Add formatter round-trip tests.
- [ ] Add LSP hover and documentation extraction tests.
- [ ] Add facade and generic contract visibility tests.
- [ ] Add rejected tests for malformed structured contracts.
- [ ] Record that contracts do not change runtime behavior unless explicitly introduced by a later phase.

### Gate 33.7 — Compiler-generated local bindings

Remove repetitive temporary bindings required only to satisfy role-sensitive
argument expressions, without weakening move and borrow checking.

#### Design

- [ ] Define the exact expression positions where a compiler-generated local may be introduced.
- [ ] Limit the feature to pure scalar expressions with known type and lifetime.
- [ ] Require single evaluation and deterministic left-to-right ordering.
- [ ] Preserve explicit bindings as the canonical escape hatch.
- [ ] Reject buffers, aggregates, resources, calls, mutation, and expressions with observable side effects.
- [ ] Define ownership role assignment for generated locals.
- [ ] Document diagnostics and migration behavior.

#### Compiler implementation

- [ ] Add a semantic normalization pass that materializes approved scalar temporaries before role checking.
- [ ] Preserve original source spans in diagnostics and generated IR metadata.
- [ ] Ensure generated locals cannot escape their call scope.
- [ ] Verify cleanup and drop behavior remains unchanged.
- [ ] Ensure native lowering does not introduce unnecessary allocation or runtime symbols.
- [ ] Add formatter and LSP behavior that does not invent misleading source declarations.
- [ ] Keep the transformation deterministic across object and executable builds.

#### Evidence

- [ ] Add accepted tests for arithmetic, indexing, and offset arguments.
- [ ] Add rejected tests for side effects, aggregates, and ownership escapes.
- [ ] Add use-after-move and borrow-conflict regressions.
- [ ] Compare explicit-binding and generated-binding native output.
- [ ] Add object/executable parity and no-extra-allocation evidence.
- [ ] Record the accepted normalization boundary.

## Cross-cutting acceptance gate

- [ ] Update `ACTUS_CODING_AGENT_GUIDE.md` only with implemented behavior.
- [ ] Add ADRs for ownership inference, bounded iteration, layout declarations, serialization contracts, and generated-local normalization.
- [ ] Update the language guide, style guide, formatter, and LSP documentation.
- [ ] Add accepted and rejected fixtures for every new syntax form.
- [ ] Keep lexer -> parser -> AST -> semantic -> codegen dependency direction.
- [ ] Keep every implementation file below the hard size limit.
- [ ] Keep every function within the preferred limit where practical.
- [ ] Run formatter, compiler check, clippy, tests, source-limit checks, and `git diff --check`.
- [ ] Verify no project-specific application names or implementation details enter the Actus language or standard library.
- [ ] Publish an acceptance report with parser, semantic, native, tooling, performance, and compatibility evidence.

## Completion criteria

Phase 33 is complete only when all seven gates and the cross-cutting acceptance
gate are checked, the required ADRs and documentation are merged, accepted and
rejected behavior is covered, and the full repository quality suite passes.
